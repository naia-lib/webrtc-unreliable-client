use crate::webrtc::sctp::chunk::chunk_payload_data::ChunkPayloadData;

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::sync::Mutex;

/// pendingBaseQueue
pub(crate) type PendingBaseQueue = VecDeque<ChunkPayloadData>;

// TODO: benchmark performance between multiple Atomic+Mutex vs one Mutex<PendingQueueInternal>

/// pendingQueue
#[derive(Debug, Default)]
pub(crate) struct PendingQueue {
    unordered_queue: Mutex<PendingBaseQueue>,
    queue_len: AtomicUsize,
    n_bytes: AtomicUsize,
    selected: AtomicBool,
}

impl PendingQueue {
    pub(crate) fn new() -> Self {
        PendingQueue::default()
    }

    pub(crate) async fn push(&self, c: ChunkPayloadData) {
        self.n_bytes.fetch_add(c.user_data.len(), Ordering::SeqCst);
        let mut unordered_queue = self.unordered_queue.lock().await;
        unordered_queue.push_back(c);
        self.queue_len.fetch_add(1, Ordering::SeqCst);
    }

    pub(crate) async fn peek(&self) -> Option<ChunkPayloadData> {
        if self.selected.load(Ordering::SeqCst) {
            let unordered_queue = self.unordered_queue.lock().await;
            return unordered_queue.get(0).cloned();
        }

        let c = {
            let unordered_queue = self.unordered_queue.lock().await;
            unordered_queue.get(0).cloned()
        };

        if c.is_some() {
            return c;
        }

        None
    }

    pub(crate) async fn pop(
        &self,
        beginning_fragment: bool,
    ) -> Option<ChunkPayloadData> {
        let popped = if self.selected.load(Ordering::SeqCst) {
            let popped = {
                let mut unordered_queue = self.unordered_queue.lock().await;
                unordered_queue.pop_front()
            };

            if let Some(p) = &popped {
                if p.ending_fragment {
                    self.selected.store(false, Ordering::SeqCst);
                }
            }
            popped
        } else {
            if !beginning_fragment {
                return None;
            }
            let popped = {
                let mut unordered_queue = self.unordered_queue.lock().await;
                unordered_queue.pop_front()
            };
            if let Some(p) = &popped {
                if !p.ending_fragment {
                    self.selected.store(true, Ordering::SeqCst);
                }
            }
            popped
        };

        if let Some(p) = &popped {
            self.n_bytes.fetch_sub(p.user_data.len(), Ordering::SeqCst);
            self.queue_len.fetch_sub(1, Ordering::SeqCst);
        }

        popped
    }

    pub(crate) fn len(&self) -> usize {
        self.queue_len.load(Ordering::SeqCst)
    }
}
