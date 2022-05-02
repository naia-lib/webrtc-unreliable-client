use crate::webrtc::stun::agent::*;
use crate::webrtc::stun::error::*;

use crate::webrtc::util::Conn;

use std::marker::{Send, Sync};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{self, Duration, Instant};

const DEFAULT_TIMEOUT_RATE: Duration = Duration::from_millis(5);
const DEFAULT_RTO: Duration = Duration::from_millis(300);
const DEFAULT_MAX_ATTEMPTS: u32 = 7;
const DEFAULT_MAX_BUFFER_SIZE: usize = 8;

/// Collector calls function f with constant rate.
///
/// The simple Collector is ticker which calls function on each tick.
pub trait Collector {
    fn start(
        &mut self,
        rate: Duration,
        client_agent_tx: Arc<mpsc::Sender<ClientAgent>>,
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
        client_agent_tx: Arc<mpsc::Sender<ClientAgent>>,
    ) -> Result<()> {
        let (close_tx, mut close_rx) = mpsc::channel(1);
        self.close_tx = Some(close_tx);

        tokio::spawn(async move {
            let mut interval = time::interval(rate);

            loop {
                tokio::select! {
                    _ = close_rx.recv() => break,
                    _ = interval.tick() => {
                        if client_agent_tx.send(ClientAgent::Collect(Instant::now())).await.is_err() {
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

/// ClientTransaction represents transaction in progress.
/// If transaction is succeed or failed, f will be called
/// provided by event.
/// Concurrent access is invalid.
#[derive(Debug, Clone)]
pub struct ClientTransaction {
    id: TransactionId,
    attempt: u32,
    calls: u32,
    handler: Handler,
    start: Instant,
    rto: Duration,
    raw: Vec<u8>,
}

struct ClientSettings {
    buffer_size: usize,
    rto: Duration,
    rto_rate: Duration,
    max_attempts: u32,
    closed: bool,
    //handler: Handler,
    collector: Option<Box<dyn Collector + Send>>,
    c: Option<Arc<dyn Conn + Send + Sync>>,
}

impl Default for ClientSettings {
    fn default() -> Self {
        ClientSettings {
            buffer_size: DEFAULT_MAX_BUFFER_SIZE,
            rto: DEFAULT_RTO,
            rto_rate: DEFAULT_TIMEOUT_RATE,
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            closed: false,
            //handler: None,
            collector: None,
            c: None,
        }
    }
}

#[derive(Default)]
pub struct ClientBuilder {
    settings: ClientSettings,
}

/// Client simulates "connection" to STUN server.
#[derive(Default)]
pub struct Client {
    settings: ClientSettings,
    close_tx: Option<mpsc::Sender<()>>,
    client_agent_tx: Option<Arc<mpsc::Sender<ClientAgent>>>,
    handler_tx: Option<Arc<mpsc::UnboundedSender<Event>>>,
}
