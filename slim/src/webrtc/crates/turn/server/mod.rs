#[cfg(test)]
mod server_test;

pub mod config;
pub mod request;

use crate::webrtc::turn::allocation::allocation_manager::*;
use crate::webrtc::turn::auth::AuthHandler;
use crate::webrtc::turn::error::*;
use config::*;
use request::*;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{watch, Mutex};
use tokio::time::{Duration, Instant};
use crate::webrtc::util::Conn;

const INBOUND_MTU: usize = 1500;

/// Server is an instance of the TURN Server
pub struct Server {
    auth_handler: Arc<dyn AuthHandler + Send + Sync>,
    realm: String,
    channel_bind_timeout: Duration,
    pub(crate) nonces: Arc<Mutex<HashMap<String, Instant>>>,
    shutdown_tx: Mutex<Option<watch::Sender<bool>>>,
}

impl Server {

    /// Close stops the TURN Server. It cleans up any associated state and closes all connections it is managing
    pub async fn close(&self) -> Result<()> {
        let mut shutdown_tx = self.shutdown_tx.lock().await;
        if let Some(tx) = shutdown_tx.take() {
            // errors if there are no receivers, but that's irrelevant.
            let _ = tx.send(true);
            // wait for all receivers to drop/close.
            let _ = tx.closed().await;
        }

        Ok(())
    }
}
