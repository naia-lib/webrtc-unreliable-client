#[cfg(test)]
mod chunk_queue_test;

use super::chunk::*;

use std::collections::VecDeque;
use tokio::sync::RwLock;

#[derive(Default)]
pub(crate) struct ChunkQueue {
    chunks: RwLock<VecDeque<Box<dyn Chunk + Send + Sync>>>,
    max_size: usize, // 0 or negative value: unlimited
}

impl ChunkQueue {
    pub(crate) async fn push(&self, c: Box<dyn Chunk + Send + Sync>) -> bool {
        let mut chunks = self.chunks.write().await;

        if self.max_size > 0 && chunks.len() >= self.max_size {
            false // dropped
        } else {
            chunks.push_back(c);
            true
        }
    }
}
