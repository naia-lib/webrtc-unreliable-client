use crate::webrtc::interceptor::error::Result;
use crate::webrtc::interceptor::nack::UINT16SIZE_HALF;
use crate::webrtc::interceptor::{Attributes, RTPWriter};

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

struct ResponderStreamInternal {
    packets: Vec<Option<rtp::packet::Packet>>,
    size: u16,
    last_added: u16,
    started: bool,
}

impl ResponderStreamInternal {
    fn new(log2_size: u8) -> Self {
        ResponderStreamInternal {
            packets: vec![None; 1 << log2_size],
            size: 1 << log2_size,
            last_added: 0,
            started: false,
        }
    }

    fn add(&mut self, packet: &rtp::packet::Packet) {
        let seq = packet.header.sequence_number;
        if !self.started {
            self.packets[(seq % self.size) as usize] = Some(packet.clone());
            self.last_added = seq;
            self.started = true;
            return;
        }

        let diff = seq.wrapping_sub(self.last_added);
        if diff == 0 {
            return;
        } else if diff < UINT16SIZE_HALF {
            let mut i = self.last_added.wrapping_add(1);
            while i != seq {
                self.packets[(i % self.size) as usize] = None;
                i = i.wrapping_add(1);
            }
        }

        self.packets[(seq % self.size) as usize] = Some(packet.clone());
        self.last_added = seq;
    }

    fn get(&self, seq: u16) -> Option<&rtp::packet::Packet> {
        let diff = self.last_added.wrapping_sub(seq);
        if diff >= UINT16SIZE_HALF {
            return None;
        }

        if diff >= self.size {
            return None;
        }

        self.packets[(seq % self.size) as usize].as_ref()
    }
}

pub(super) struct ResponderStream {
    internal: Mutex<ResponderStreamInternal>,
    pub(super) next_rtp_writer: Arc<dyn RTPWriter + Send + Sync>,
}

impl ResponderStream {
    pub(super) fn new(log2_size: u8, writer: Arc<dyn RTPWriter + Send + Sync>) -> Self {
        ResponderStream {
            internal: Mutex::new(ResponderStreamInternal::new(log2_size)),
            next_rtp_writer: writer,
        }
    }

    async fn add(&self, pkt: &rtp::packet::Packet) {
        let mut internal = self.internal.lock().await;
        internal.add(pkt);
    }

    pub(super) async fn get(&self, seq: u16) -> Option<rtp::packet::Packet> {
        let internal = self.internal.lock().await;
        internal.get(seq).cloned()
    }
}

/// RTPWriter is used by Interceptor.bind_local_stream.
#[async_trait]
impl RTPWriter for ResponderStream {
    /// write a rtp packet
    async fn write(&self, pkt: &rtp::packet::Packet, a: &Attributes) -> Result<usize> {
        self.add(pkt).await;

        self.next_rtp_writer.write(pkt, a).await
    }
}