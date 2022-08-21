
use tokio::sync::{mpsc, Mutex};

use async_trait::async_trait;

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum TimerIdRefresh {
    Alloc,
}

impl Default for TimerIdRefresh {
    fn default() -> Self {
        TimerIdRefresh::Alloc
    }
}

// PeriodicTimerTimeoutHandler is a handler called on timeout
#[async_trait]
pub(crate) trait PeriodicTimerTimeoutHandler {
    async fn on_timeout(&mut self, id: TimerIdRefresh);
}

// PeriodicTimer is a periodic timer
#[derive(Default)]
pub(crate) struct PeriodicTimer {
    close_tx: Mutex<Option<mpsc::Sender<()>>>,
}

impl PeriodicTimer {

    // Stop stops the timer.
    pub(crate) async fn stop(&self) {
        let mut close_tx = self.close_tx.lock().await;
        close_tx.take();
    }
}
