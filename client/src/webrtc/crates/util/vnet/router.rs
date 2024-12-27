use std::sync::{atomic::AtomicU64, Arc};

use tokio::sync::mpsc;

use crate::webrtc::util::vnet::{chunk::Chunk, chunk_queue::ChunkQueue};

lazy_static! {
    pub(crate) static ref ROUTER_ID_CTR: AtomicU64 = AtomicU64::new(0);
}

// Router ...
#[derive(Default)]
pub(crate) struct Router {
    name: String,                      // read-only
    queue: Arc<ChunkQueue>,            // read-only
    done: Option<mpsc::Sender<()>>,    // requires mutex [x]
    push_ch: Option<mpsc::Sender<()>>, // writer requires mutex
}

impl Router {
    pub(crate) async fn push(&self, mut c: Box<dyn Chunk + Send + Sync>) {
        log::debug!("[{}] route {}", self.name, c);
        if self.done.is_some() {
            c.set_timestamp();

            if self.queue.push(c).await {
                if let Some(push_ch) = &self.push_ch {
                    let _ = push_ch.try_send(());
                }
            } else {
                log::warn!("[{}] queue was full. dropped a chunk", self.name);
            }
        } else {
            log::warn!("router is done");
        }
    }
}
