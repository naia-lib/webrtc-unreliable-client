use crate::webrtc::sctp::chunk::chunk_payload_data::{ChunkPayloadData, PayloadProtocolIdentifier};
use crate::webrtc::sctp::util::*;

use crate::webrtc::sctp::error::{Error, Result};

use std::cmp::Ordering;

fn sort_chunks_by_tsn(c: &mut Vec<ChunkPayloadData>) {
    c.sort_by(|a, b| {
        if sna32lt(a.tsn, b.tsn) {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    });
}

/// chunkSet is a set of chunks that share the same SSN
#[derive(Debug, Clone)]
pub(crate) struct ChunkSet {
    pub(crate) ppi: PayloadProtocolIdentifier,
    pub(crate) chunks: Vec<ChunkPayloadData>,
}

impl ChunkSet {
    pub(crate) fn new(ppi: PayloadProtocolIdentifier) -> Self {
        Self {
            ppi,
            chunks: vec![],
        }
    }
}

#[derive(Default, Debug)]
pub(crate) struct ReassemblyQueue {
    pub(crate) si: u16,
    pub(crate) unordered: Vec<ChunkSet>,
    pub(crate) unordered_chunks: Vec<ChunkPayloadData>,
    pub(crate) n_bytes: usize,
}

impl ReassemblyQueue {
    /// From RFC 4960 Sec 6.5:
    ///   The Stream Sequence Number in all the streams MUST start from 0 when
    ///   the association is Established.  Also, when the Stream Sequence
    ///   Number reaches the value 65535 the next Stream Sequence Number MUST
    ///   be set to 0.
    pub(crate) fn new(si: u16) -> Self {
        Self {
            si,
            unordered: vec![],
            unordered_chunks: vec![],
            n_bytes: 0,
        }
    }

    pub(crate) fn push(&mut self, chunk: ChunkPayloadData) -> bool {
        if chunk.stream_identifier != self.si {
            return false;
        }

        // First, insert into unordered_chunks array
        //atomic.AddUint64(&r.n_bytes, uint64(len(chunk.userData)))
        self.n_bytes += chunk.user_data.len();
        self.unordered_chunks.push(chunk);
        sort_chunks_by_tsn(&mut self.unordered_chunks);

        // Scan unordered_chunks that are contiguous (in TSN)
        // If found, append the complete set to the unordered array
        if let Some(cset) = self.find_complete_unordered_chunk_set() {
            self.unordered.push(cset);
            return true;
        }

        false
    }

    pub(crate) fn find_complete_unordered_chunk_set(&mut self) -> Option<ChunkSet> {
        let mut start_idx = -1isize;
        let mut n_chunks = 0usize;
        let mut last_tsn = 0u32;
        let mut found = false;

        for (i, c) in self.unordered_chunks.iter().enumerate() {
            // seek beigining
            if c.beginning_fragment {
                start_idx = i as isize;
                n_chunks = 1;
                last_tsn = c.tsn;

                if c.ending_fragment {
                    found = true;
                    break;
                }
                continue;
            }

            if start_idx < 0 {
                continue;
            }

            // Check if contiguous in TSN
            if c.tsn != last_tsn + 1 {
                start_idx = -1;
                continue;
            }

            last_tsn = c.tsn;
            n_chunks += 1;

            if c.ending_fragment {
                found = true;
                break;
            }
        }

        if !found {
            return None;
        }

        // Extract the range of chunks
        let chunks: Vec<ChunkPayloadData> = self
            .unordered_chunks
            .drain(start_idx as usize..(start_idx as usize) + n_chunks)
            .collect();

        let mut chunk_set = ChunkSet::new(chunks[0].payload_type);
        chunk_set.chunks = chunks;

        Some(chunk_set)
    }

    pub(crate) fn is_readable(&self) -> bool {
        // Check unordered first
        if !self.unordered.is_empty() {
            // The chunk sets in r.unordered should all be complete.
            return true;
        }

        false
    }

    pub(crate) fn read(&mut self, buf: &mut [u8]) -> Result<(usize, PayloadProtocolIdentifier)> {
        // Check unordered first
        let cset = if !self.unordered.is_empty() {
            self.unordered.remove(0)
        } else {
            return Err(Error::ErrTryAgain);
        };

        // Concat all fragments into the buffer
        let mut n_written = 0;
        let mut err = None;
        for c in &cset.chunks {
            let to_copy = c.user_data.len();
            self.subtract_num_bytes(to_copy);
            if err.is_none() {
                let n = std::cmp::min(to_copy, buf.len() - n_written);
                buf[n_written..n_written + n].copy_from_slice(&c.user_data[..n]);
                n_written += n;
                if n < to_copy {
                    err = Some(Error::ErrShortBuffer);
                }
            }
        }

        if let Some(err) = err {
            Err(err)
        } else {
            Ok((n_written, cset.ppi))
        }
    }

    /// Remove all fragments in the unordered sets that contains chunks
    /// equal to or older than `new_cumulative_tsn`.
    /// We know all sets in the r.unordered are complete ones.
    /// Just remove chunks that are equal to or older than new_cumulative_tsn
    /// from the unordered_chunks
    pub(crate) fn forward_tsn_for_unordered(&mut self, new_cumulative_tsn: u32) {
        let mut last_idx: isize = -1;
        for (i, c) in self.unordered_chunks.iter().enumerate() {
            if sna32gt(c.tsn, new_cumulative_tsn) {
                break;
            }
            last_idx = i as isize;
        }
        if last_idx >= 0 {
            for i in 0..(last_idx + 1) as usize {
                self.subtract_num_bytes(self.unordered_chunks[i].user_data.len());
            }
            self.unordered_chunks.drain(..(last_idx + 1) as usize);
        }
    }

    pub(crate) fn subtract_num_bytes(&mut self, n_bytes: usize) {
        if self.n_bytes >= n_bytes {
            self.n_bytes -= n_bytes;
        } else {
            self.n_bytes = 0;
        }
    }

    pub(crate) fn get_num_bytes(&self) -> usize {
        self.n_bytes
    }
}
