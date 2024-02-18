use super::*;
use crate::webrtc::util::Buffer;

use std::collections::HashMap;

use tokio::sync::Mutex;

/// UdpConn augments a connection-oriented connection over a UdpSocket
pub(crate) struct UdpConn {
    pconn: Arc<dyn Conn + Send + Sync>,
    conns: Arc<Mutex<HashMap<String, Arc<UdpConn>>>>,
    raddr: SocketAddr,
    buffer: Buffer,
}

#[async_trait]
impl Conn for UdpConn {
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
