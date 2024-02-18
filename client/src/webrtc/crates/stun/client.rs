
use crate::webrtc::stun::error::*;

use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{self, Duration};

/// Collector calls function f with constant rate.
///
/// The simple Collector is ticker which calls function on each tick.
pub(crate) trait Collector {
    fn start(
        &mut self,
        rate: Duration,
        client_agent_tx: Arc<mpsc::Sender<()>>,
    ) -> Result<()>;
    fn close(&mut self) -> Result<()>;
}

#[derive(Default)]
struct TickerCollector {
    close_tx: Option<mpsc::Sender<()>>,
}

impl Collector for TickerCollector {
    fn start(
        &mut self,
        rate: Duration,
        client_agent_tx: Arc<mpsc::Sender<()>>,
    ) -> Result<()> {
        let (close_tx, mut close_rx) = mpsc::channel(1);
        self.close_tx = Some(close_tx);

        tokio::spawn(async move {
            let mut interval = time::interval(rate);

            loop {
                tokio::select! {
                    _ = close_rx.recv() => break,
                    _ = interval.tick() => {
                        if client_agent_tx.send(()).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(())
    }

    fn close(&mut self) -> Result<()> {
        if self.close_tx.is_none() {
            return Err(Error::ErrCollectorClosed);
        }
        self.close_tx.take();
        Ok(())
    }
}
