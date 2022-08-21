use super::*;
use crate::webrtc::util::error::Error;
use crate::webrtc::util::Buffer;

use core::sync::atomic::Ordering;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use tokio::sync::{mpsc, watch, Mutex};

type AcceptDoneCh = (mpsc::Receiver<Arc<UdpConn>>, watch::Receiver<()>);

/// listener is used in the [DTLS](https://github.com/webrtc-rs/dtls) and
/// [SCTP](https://github.com/webrtc-rs/sctp) transport to provide a connection-oriented
/// listener over a UDP.
struct ListenerImpl {
    pconn: Arc<dyn Conn + Send + Sync>,
    accepting: Arc<AtomicBool>,
    accept_ch_tx: Arc<Mutex<Option<mpsc::Sender<Arc<UdpConn>>>>>,
    done_ch_tx: Arc<Mutex<Option<watch::Sender<()>>>>,
    ch_rx: Arc<Mutex<AcceptDoneCh>>,
}

#[async_trait]
impl Listener for ListenerImpl {
    /// accept waits for and returns the next connection to the listener.
    async fn accept(&self) -> Result<(Arc<dyn Conn + Send + Sync>, SocketAddr)> {
        let (accept_ch_rx, done_ch_rx) = &mut *self.ch_rx.lock().await;

        tokio::select! {
            c = accept_ch_rx.recv() =>{
                if let Some(c) = c{
                    let raddr = c.raddr;
                    Ok((c, raddr))
                }else{
                    Err(Error::ErrClosedListenerAcceptCh)
                }
            }
            _ = done_ch_rx.changed() =>  Err(Error::ErrClosedListener),
        }
    }

    /// close closes the listener.
    /// Any blocked Accept operations will be unblocked and return errors.
    async fn close(&self) -> Result<()> {
        if self.accepting.load(Ordering::SeqCst) {
            self.accepting.store(false, Ordering::SeqCst);
            {
                let mut done_ch = self.done_ch_tx.lock().await;
                done_ch.take();
            }
            {
                let mut accept_ch = self.accept_ch_tx.lock().await;
                accept_ch.take();
            }
        }

        Ok(())
    }

    /// Addr returns the listener's network address.
    async fn addr(&self) -> Result<SocketAddr> {
        self.pconn.local_addr().await
    }
}

/// UdpConn augments a connection-oriented connection over a UdpSocket
pub(crate) struct UdpConn {
    pconn: Arc<dyn Conn + Send + Sync>,
    conns: Arc<Mutex<HashMap<String, Arc<UdpConn>>>>,
    raddr: SocketAddr,
    buffer: Buffer,
}

#[async_trait]
impl Conn for UdpConn {
    async fn connect(&self, addr: SocketAddr) -> Result<()> {
        self.pconn.connect(addr).await
    }

    async fn recv(&self, buf: &mut [u8]) -> Result<usize> {
        Ok(self.buffer.read(buf, None).await?)
    }

    async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        let n = self.buffer.read(buf, None).await?;
        Ok((n, self.raddr))
    }

    async fn send(&self, buf: &[u8]) -> Result<usize> {
        self.pconn.send_to(buf, self.raddr).await
    }

    async fn send_to(&self, buf: &[u8], target: SocketAddr) -> Result<usize> {
        self.pconn.send_to(buf, target).await
    }

    async fn local_addr(&self) -> Result<SocketAddr> {
        self.pconn.local_addr().await
    }

    async fn remote_addr(&self) -> Option<SocketAddr> {
        Some(self.raddr)
    }

    async fn close(&self) -> Result<()> {
        let mut conns = self.conns.lock().await;
        conns.remove(self.raddr.to_string().as_str());
        Ok(())
    }
}
