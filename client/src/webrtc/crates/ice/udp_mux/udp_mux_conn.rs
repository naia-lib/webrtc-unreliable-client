use std::convert::TryInto;
use std::{collections::HashSet, io, net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use tokio::sync::watch;

use crate::webrtc::util::{sync::Mutex, Buffer, Conn, Error};

use super::socket_addr_ext::{SocketAddrExt, MAX_ADDR_SIZE};
use super::{normalize_socket_addr, UDPMuxDefault, RECEIVE_MTU};

#[inline(always)]
/// Create a buffer of appropriate size to fit both a packet with max RECEIVE_MTU and the
/// additional metadata used for muxing.
fn make_buffer() -> Vec<u8> {
    // The 4 extra bytes are used to encode the length of the data and address respectively.
    // See [`write_packet`] for details.
    vec![0u8; RECEIVE_MTU + MAX_ADDR_SIZE + 2 + 2]
}

pub(crate) struct UDPMuxConnParams {
    pub(crate) local_addr: SocketAddr,

    pub(crate) key: String,

    // NOTE: This Arc exists in both directions which is liable to cause a retain cycle. This is
    // accounted for in [`UDPMuxDefault::close`], which makes sure to drop all Arcs referencing any
    // `UDPMuxConn`.
    pub(crate) udp_mux: Arc<UDPMuxDefault>,
}

struct UDPMuxConnInner {
    pub(crate) params: UDPMuxConnParams,

    /// Close Sender. We'll send a value on this channel when we close
    closed_watch_tx: Mutex<Option<watch::Sender<bool>>>,

    /// Remote addresses we've seen on this connection.
    addresses: Mutex<HashSet<SocketAddr>>,

    buffer: Buffer,
}

impl UDPMuxConnInner {
    // Sending/Recieving
    async fn recv_from(&self, buf: &mut [u8]) -> ConnResult<(usize, SocketAddr)> {
        // NOTE: Pion/ice uses Sync.Pool to optimise this.
        let mut buffer = make_buffer();
        let mut offset = 0;

        let len = self.buffer.read(&mut buffer, None).await?;
        // We always have at least.
        //
        // * 2 bytes for data len
        // * 2 bytes for addr len
        // * 7 bytes for an Ipv4 addr
        if len < 11 {
            return Err(Error::ErrBufferShort);
        }

        let data_len: usize = buffer[..2]
            .try_into()
            .map(u16::from_le_bytes)
            .map(From::from)
            .unwrap();
        offset += 2;

        let total = 2 + data_len + 2 + 7;
        if data_len > buf.len() || total > len {
            return Err(Error::ErrBufferShort);
        }

        buf[..data_len].copy_from_slice(&buffer[offset..offset + data_len]);
        offset += data_len;

        let address_len: usize = buffer[offset..offset + 2]
            .try_into()
            .map(u16::from_le_bytes)
            .map(From::from)
            .unwrap();
        offset += 2;

        let addr = SocketAddr::decode(&buffer[offset..offset + address_len])?;

        Ok((data_len, addr))
    }

    async fn send_to(&self, buf: &[u8], target: &SocketAddr) -> ConnResult<usize> {
        self.params.udp_mux.send_to(buf, target).await
    }

    fn close(self: &Arc<Self>) {
        let mut closed_tx = self.closed_watch_tx.lock();

        if let Some(tx) = closed_tx.take() {
            let _ = tx.send(true);
            drop(closed_tx);

            let cloned_self = Arc::clone(self);

            {
                let mut addresses = self.addresses.lock();
                *addresses = Default::default();
            }

            // NOTE: Alternatively we could wait on the buffer closing here so that
            // our caller can wait for things to fully settle down
            tokio::spawn(async move {
                cloned_self.buffer.close().await;
            });
        }
    }

    fn local_addr(&self) -> SocketAddr {
        self.params.local_addr
    }

    // Address related methods
    pub(crate) fn get_addresses(&self) -> Vec<SocketAddr> {
        let addresses = self.addresses.lock();

        addresses.iter().cloned().collect()
    }

    pub(crate) fn add_address(self: &Arc<Self>, addr: SocketAddr) {
        {
            let mut addresses = self.addresses.lock();
            addresses.insert(addr);
        }
    }

    pub(crate) fn remove_address(&self, addr: &SocketAddr) {
        {
            let mut addresses = self.addresses.lock();
            addresses.remove(addr);
        }
    }

    pub(crate) fn contains_address(&self, addr: &SocketAddr) -> bool {
        let addresses = self.addresses.lock();

        addresses.contains(addr)
    }
}

#[derive(Clone)]
pub(crate) struct UDPMuxConn {
    /// Close Receiver. A copy of this can be obtained via [`close_tx`].
    closed_watch_rx: watch::Receiver<bool>,

    inner: Arc<UDPMuxConnInner>,
}

impl UDPMuxConn {
    pub(crate) fn new(params: UDPMuxConnParams) -> Self {
        let (closed_watch_tx, closed_watch_rx) = watch::channel(false);

        Self {
            closed_watch_rx,
            inner: Arc::new(UDPMuxConnInner {
                params,
                closed_watch_tx: Mutex::new(Some(closed_watch_tx)),
                addresses: Default::default(),
                buffer: Buffer::new(0, 0),
            }),
        }
    }

    pub(crate) fn key(&self) -> &str {
        &self.inner.params.key
    }

    /// Get a copy of the close [`tokio::sync::watch::Receiver`] that fires when this
    /// connection is closed.
    pub(crate) fn close_rx(&self) -> watch::Receiver<bool> {
        self.closed_watch_rx.clone()
    }

    /// Close this connection
    pub(crate) fn close(&self) {
        self.inner.close();
    }

    pub(crate) fn get_addresses(&self) -> Vec<SocketAddr> {
        self.inner.get_addresses()
    }

    pub(crate) async fn add_address(&self, addr: SocketAddr) {
        self.inner.add_address(addr);
        self.inner
            .params
            .udp_mux
            .register_conn_for_address(self, addr)
            .await;
    }

    pub(crate) fn remove_address(&self, addr: &SocketAddr) {
        self.inner.remove_address(addr)
    }

    pub(crate) fn contains_address(&self, addr: &SocketAddr) -> bool {
        self.inner.contains_address(addr)
    }
}

type ConnResult<T> = Result<T, crate::webrtc::util::Error>;

#[async_trait]
impl Conn for UDPMuxConn {
    async fn connect(&self, _addr: SocketAddr) -> ConnResult<()> {
        Err(io::Error::new(io::ErrorKind::Other, "Not applicable").into())
    }

    async fn recv(&self, _buf: &mut [u8]) -> ConnResult<usize> {
        Err(io::Error::new(io::ErrorKind::Other, "Not applicable").into())
    }

    async fn recv_from(&self, buf: &mut [u8]) -> ConnResult<(usize, SocketAddr)> {
        self.inner.recv_from(buf).await
    }

    async fn send(&self, _buf: &[u8]) -> ConnResult<usize> {
        Err(io::Error::new(io::ErrorKind::Other, "Not applicable").into())
    }

    async fn send_to(&self, buf: &[u8], target: SocketAddr) -> ConnResult<usize> {
        let normalized_target = normalize_socket_addr(&target, &self.inner.params.local_addr);

        if !self.contains_address(&normalized_target) {
            self.add_address(normalized_target).await;
        }

        self.inner.send_to(buf, &normalized_target).await
    }

    async fn local_addr(&self) -> ConnResult<SocketAddr> {
        Ok(self.inner.local_addr())
    }

    async fn remote_addr(&self) -> Option<SocketAddr> {
        None
    }
    async fn close(&self) -> ConnResult<()> {
        self.inner.close();

        Ok(())
    }
}
