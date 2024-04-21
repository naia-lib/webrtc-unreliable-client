pub(crate) mod conn_bridge;
pub(crate) mod conn_disconnected_packet;
pub(crate) mod conn_pipe;
pub(crate) mod conn_udp;
pub(crate) mod conn_udp_listener;

use async_trait::async_trait;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::webrtc::util::error::Result;

#[async_trait]
pub(crate) trait Conn {
    async fn recv(&self, buf: &mut [u8]) -> Result<usize>;
    async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)>;
    async fn send(&self, buf: &[u8]) -> Result<usize>;
    async fn send_to(&self, buf: &[u8], target: SocketAddr) -> Result<usize>;
    async fn local_addr(&self) -> Result<SocketAddr>;
    async fn remote_addr(&self) -> Option<SocketAddr>;
    async fn close(&self) -> Result<()>;
}
