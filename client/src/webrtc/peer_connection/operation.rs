use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::webrtc::error::Result;

/// Operation is a function
pub(crate) struct Operation(
    pub Box<dyn (FnMut() -> Pin<Box<dyn Future<Output = bool> + Send + 'static>>) + Send + Sync>,
);

impl fmt::Debug for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Operation").finish()
    }
}

/// Operations is a task executor.
#[derive(Default)]
pub(crate) struct Operations {
    length: Arc<AtomicUsize>,
    ops_tx: Option<Arc<mpsc::UnboundedSender<Operation>>>,
    // Removing this causes exceptions
    #[allow(dead_code)]
    close_tx: Option<mpsc::Sender<()>>,
}

impl Operations {
    pub(crate) fn new() -> Self {
        let length = Arc::new(AtomicUsize::new(0));
        let (ops_tx, ops_rx) = mpsc::unbounded_channel();
        let (close_tx, close_rx) = mpsc::channel(1);
        let l = Arc::clone(&length);
        let ops_tx = Arc::new(ops_tx);
        let ops_tx2 = Arc::clone(&ops_tx);
        tokio::spawn(async move {
            Operations::start(l, ops_tx, ops_rx, close_rx).await;
        });

        Operations {
            length,
            ops_tx: Some(ops_tx2),
            close_tx: Some(close_tx),
        }
    }

    /// enqueue adds a new action to be executed. If there are no actions scheduled,
    /// the execution will start immediately in a new goroutine.
    pub(crate) async fn enqueue(&self, op: Operation) -> Result<()> {
        if let Some(ops_tx) = &self.ops_tx {
            return Operations::enqueue_inner(op, ops_tx, &self.length);
        }

        Ok(())
    }

    fn enqueue_inner(
        op: Operation,
        ops_tx: &Arc<mpsc::UnboundedSender<Operation>>,
        length: &Arc<AtomicUsize>,
    ) -> Result<()> {
        length.fetch_add(1, Ordering::SeqCst);
        let _ = ops_tx.send(op)?;

        Ok(())
    }

    pub(crate) async fn start(
        length: Arc<AtomicUsize>,
        ops_tx: Arc<mpsc::UnboundedSender<Operation>>,
        mut ops_rx: mpsc::UnboundedReceiver<Operation>,
        mut close_rx: mpsc::Receiver<()>,
    ) {
        loop {
            tokio::select! {
                _ = close_rx.recv() => {
                    break;
                }
                result = ops_rx.recv() => {
                    if let Some(mut f) = result {
                        length.fetch_sub(1, Ordering::SeqCst);
                        if f.0().await {
                            // Requeue this operation
                            let _ = Operations::enqueue_inner(f, &ops_tx, &length);
                        }
                    }
                }
            }
        }
    }
}
