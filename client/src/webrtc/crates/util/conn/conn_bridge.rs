use super::*;

use bytes::Bytes;
use std::collections::VecDeque;
use std::io::{Error, ErrorKind};
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::Duration;

const TICK_WAIT: Duration = Duration::from_micros(10);

/// BridgeConn is a Conn that represents an endpoint of the bridge.
struct BridgeConn {
    br: Arc<Bridge>,
    id: usize,
    rd_rx: Mutex<mpsc::Receiver<Bytes>>,
    loss_chance: u8,
}

#[async_trait]
impl Conn for BridgeConn {
    async fn connect(&self, _addr: SocketAddr) -> Result<()> {
        Err(Error::new(ErrorKind::Other, "Not applicable").into())
    }

    async fn recv(&self, b: &mut [u8]) -> Result<usize> {
        let mut rd_rx = self.rd_rx.lock().await;
        let v = match rd_rx.recv().await {
            Some(v) => v,
            None => return Err(Error::new(ErrorKind::UnexpectedEof, "Unexpected EOF").into()),
        };
        let l = std::cmp::min(v.len(), b.len());
        b[..l].copy_from_slice(&v[..l]);
        Ok(l)
    }

    async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        let n = self.recv(buf).await?;
        Ok((n, SocketAddr::from_str("0.0.0.0:0")?))
    }

    async fn send(&self, b: &[u8]) -> Result<usize> {
        if rand::random::<u8>() % 100 < self.loss_chance {
            return Ok(b.len());
        }

        self.br.push(b, self.id).await
    }

    async fn send_to(&self, _buf: &[u8], _target: SocketAddr) -> Result<usize> {
        Err(Error::new(ErrorKind::Other, "Not applicable").into())
    }

    async fn local_addr(&self) -> Result<SocketAddr> {
        Err(Error::new(ErrorKind::AddrNotAvailable, "Addr Not Available").into())
    }

    async fn remote_addr(&self) -> Option<SocketAddr> {
        None
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }
}

pub(crate) type FilterCbFn = Box<dyn Fn(&Bytes) -> bool + Send + Sync>;

/// Bridge represents a network between the two endpoints.
#[derive(Default)]
pub(crate) struct Bridge {
    drop_nwrites: [AtomicUsize; 2],
    reorder_nwrites: [AtomicUsize; 2],

    stack: [Mutex<VecDeque<Bytes>>; 2],
    queue: [Mutex<VecDeque<Bytes>>; 2],

    filter_cb: [Option<FilterCbFn>; 2],
}

impl Bridge {

    pub(crate) async fn push(&self, b: &[u8], id: usize) -> Result<usize> {
        // Push rate should be limited as same as Tick rate.
        // Otherwise, queue grows too fast on free running Write.
        tokio::time::sleep(TICK_WAIT).await;

        let d = Bytes::from(b.to_vec());
        if self.drop_nwrites[id].load(Ordering::SeqCst) > 0 {
            self.drop_nwrites[id].fetch_sub(1, Ordering::SeqCst);
        } else if self.reorder_nwrites[id].load(Ordering::SeqCst) > 0 {
            let mut stack = self.stack[id].lock().await;
            stack.push_back(d);
            if self.reorder_nwrites[id].fetch_sub(1, Ordering::SeqCst) == 1 {
                let ok = inverse(&mut stack);
                if ok {
                    let mut queue = self.queue[id].lock().await;
                    queue.append(&mut stack);
                }
            }
        } else if let Some(filter_cb) = &self.filter_cb[id] {
            if filter_cb(&d) {
                let mut queue = self.queue[id].lock().await;
                queue.push_back(d);
            }
        } else {
            //log::debug!("queue [{}] enter lock", id);
            let mut queue = self.queue[id].lock().await;
            queue.push_back(d);
            //log::debug!("queue [{}] exit lock", id);
        }

        Ok(b.len())
    }
}

pub(crate) fn inverse(s: &mut VecDeque<Bytes>) -> bool {
    if s.len() < 2 {
        return false;
    }

    let (mut i, mut j) = (0, s.len() - 1);
    while i < j {
        s.swap(i, j);
        i += 1;
        j -= 1;
    }

    true
}
