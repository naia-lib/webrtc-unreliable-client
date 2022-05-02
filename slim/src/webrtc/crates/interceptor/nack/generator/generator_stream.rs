use super::*;
use crate::webrtc::interceptor::nack::UINT16SIZE_HALF;

use util::Unmarshal;

struct GeneratorStreamInternal {
    packets: Vec<u64>,
    size: u16,
    end: u16,
    started: bool,
    last_consecutive: u16,
}

impl GeneratorStreamInternal {
    fn new(log2_size_minus_6: u8) -> Self {
        GeneratorStreamInternal {
            packets: vec![0u64; 1 << log2_size_minus_6],
            size: 1 << (log2_size_minus_6 + 6),
            end: 0,
            started: false,
            last_consecutive: 0,
        }
    }

    fn add(&mut self, seq: u16) {
        if !self.started {
            self.set_received(seq);
            self.end = seq;
            self.started = true;
            self.last_consecutive = seq;
            return;
        }

        let last_consecutive_plus1 = self.last_consecutive.wrapping_add(1);
        let diff = seq.wrapping_sub(self.end);
        if diff == 0 {
            return;
        } else if diff < UINT16SIZE_HALF {
            // this means a positive diff, in other words seq > end (with counting for rollovers)
            let mut i = self.end.wrapping_add(1);
            while i != seq {
                // clear packets between end and seq (these may contain packets from a "size" ago)
                self.del_received(i);
                i = i.wrapping_add(1);
            }
            self.end = seq;

            let seq_sub_last_consecutive = seq.wrapping_sub(self.last_consecutive);
            if last_consecutive_plus1 == seq {
                self.last_consecutive = seq;
            } else if seq_sub_last_consecutive > self.size {
                let diff = seq.wrapping_sub(self.size);
                self.last_consecutive = diff;
                self.fix_last_consecutive(); // there might be valid packets at the beginning of the buffer now
            }
        } else if last_consecutive_plus1 == seq {
            // negative diff, seq < end (with counting for rollovers)
            self.last_consecutive = seq;
            self.fix_last_consecutive(); // there might be other valid packets after seq
        }

        self.set_received(seq);
    }

    fn get(&self, seq: u16) -> bool {
        let diff = self.end.wrapping_sub(seq);
        if diff >= UINT16SIZE_HALF {
            return false;
        }

        if diff >= self.size {
            return false;
        }

        self.get_received(seq)
    }

    fn missing_seq_numbers(&self, skip_last_n: u16) -> Vec<u16> {
        let until = self.end.wrapping_sub(skip_last_n);
        let diff = until.wrapping_sub(self.last_consecutive);
        if diff >= UINT16SIZE_HALF {
            // until < s.last_consecutive (counting for rollover)
            return vec![];
        }

        let mut missing_packet_seq_nums = vec![];
        let mut i = self.last_consecutive.wrapping_add(1);
        let util_plus1 = until.wrapping_add(1);
        while i != util_plus1 {
            if !self.get_received(i) {
                missing_packet_seq_nums.push(i);
            }
            i = i.wrapping_add(1);
        }

        missing_packet_seq_nums
    }

    fn set_received(&mut self, seq: u16) {
        let pos = (seq % self.size) as usize;
        self.packets[pos / 64] |= 1u64 << (pos % 64);
    }

    fn del_received(&mut self, seq: u16) {
        let pos = (seq % self.size) as usize;
        self.packets[pos / 64] &= u64::MAX ^ (1u64 << (pos % 64));
    }

    fn get_received(&self, seq: u16) -> bool {
        let pos = (seq % self.size) as usize;
        (self.packets[pos / 64] & (1u64 << (pos % 64))) != 0
    }

    fn fix_last_consecutive(&mut self) {
        let mut i = self.last_consecutive + 1;
        while i != self.end + 1 && self.get_received(i) {
            // find all consecutive packets
            i += 1;
        }
        self.last_consecutive = i - 1;
    }
}

pub(super) struct GeneratorStream {
    parent_rtp_reader: Arc<dyn RTPReader + Send + Sync>,

    internal: Mutex<GeneratorStreamInternal>,
}

impl GeneratorStream {
    pub(super) fn new(log2_size_minus_6: u8, reader: Arc<dyn RTPReader + Send + Sync>) -> Self {
        GeneratorStream {
            parent_rtp_reader: reader,
            internal: Mutex::new(GeneratorStreamInternal::new(log2_size_minus_6)),
        }
    }

    pub(super) async fn missing_seq_numbers(&self, skip_last_n: u16) -> Vec<u16> {
        let internal = self.internal.lock().await;
        internal.missing_seq_numbers(skip_last_n)
    }

    pub(super) async fn add(&self, seq: u16) {
        let mut internal = self.internal.lock().await;
        internal.add(seq);
    }
}

/// RTPReader is used by Interceptor.bind_remote_stream.
#[async_trait]
impl RTPReader for GeneratorStream {
    /// read a rtp packet
    async fn read(&self, buf: &mut [u8], a: &Attributes) -> Result<(usize, Attributes)> {
        let (n, attr) = self.parent_rtp_reader.read(buf, a).await?;

        let mut b = &buf[..n];
        let pkt = rtp::packet::Packet::unmarshal(&mut b)?;
        self.add(pkt.header.sequence_number).await;

        Ok((n, attr))
    }
}
