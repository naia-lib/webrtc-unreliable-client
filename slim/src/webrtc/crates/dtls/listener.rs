use crate::webrtc::dtls::config::*;
use crate::webrtc::dtls::conn::DTLSConn;

use async_trait::async_trait;
use std::net::SocketAddr;
use std::sync::Arc;
use crate::webrtc::util::conn::*;

/// DTLSListener represents a DTLS listener
pub struct DTLSListener {
    parent: Arc<dyn Listener + Send + Sync>,
    config: Config,
}

type UtilResult<T> = std::result::Result<T, crate::webrtc::util::Error>;

#[async_trait]
impl Listener for DTLSListener {
    /// Accept waits for and returns the next connection to the listener.
    /// You have to either close or read on all connection that are created.
    /// Connection handshake will timeout using ConnectContextMaker in the Config.
    /// If you want to specify the timeout duration, set ConnectContextMaker.
    async fn accept(&self) -> UtilResult<(Arc<dyn Conn + Send + Sync>, SocketAddr)> {
        let (conn, raddr) = self.parent.accept().await?;
        let dtls_conn = DTLSConn::new(conn, self.config.clone(), false, None)
            .await
            .map_err(crate::webrtc::util::Error::from_std)?;
        Ok((Arc::new(dtls_conn), raddr))
    }

    /// Close closes the listener.
    /// Any blocked Accept operations will be unblocked and return errors.
    /// Already Accepted connections are not closed.
    async fn close(&self) -> UtilResult<()> {
        self.parent.close().await
    }

    /// Addr returns the listener's network address.
    async fn addr(&self) -> UtilResult<SocketAddr> {
        self.parent.addr().await
    }
}
