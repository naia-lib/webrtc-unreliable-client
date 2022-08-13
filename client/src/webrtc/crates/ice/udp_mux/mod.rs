use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use crate::webrtc::util::{sync::RwLock, Conn, Error};

use async_trait::async_trait;

use tokio::sync::{watch, Mutex};

mod udp_mux_conn;
use udp_mux_conn::{UDPMuxConn, UDPMuxConnParams};

mod socket_addr_ext;

use crate::webrtc::ice::candidate::RECEIVE_MTU;

/// Normalize a target socket addr for sending over a given local socket addr. This is useful when
/// a dual stack socket is used, in which case an IPv4 target needs to be mapped to an IPv6
/// address.
fn normalize_socket_addr(target: &SocketAddr, socket_addr: &SocketAddr) -> SocketAddr {
    match (target, socket_addr) {
        (SocketAddr::V4(target_ipv4), SocketAddr::V6(_)) => {
            let ipv6_mapped = target_ipv4.ip().to_ipv6_mapped();

            SocketAddr::new(std::net::IpAddr::V6(ipv6_mapped), target_ipv4.port())
        }
        // This will fail later if target is IPv6 and socket is IPv4, we ignore it here
        (_, _) => *target,
    }
}

#[async_trait]
pub trait UDPMux {
    /// Close the muxing.
    async fn close(&self) -> Result<(), Error>;

    /// Get the underlying connection for a given ufrag.
    async fn get_conn(self: Arc<Self>, ufrag: &str) -> Result<Arc<dyn Conn + Send + Sync>, Error>;

    /// Remove the underlying connection for a given ufrag.
    async fn remove_conn_by_ufrag(&self, ufrag: &str);
}

pub struct UDPMuxParams {
    conn: Box<dyn Conn + Send + Sync>,
}

pub struct UDPMuxDefault {
    /// The params this instance is configured with.
    /// Contains the underlying UDP socket in use
    params: UDPMuxParams,

    /// Maps from ufrag to the underlying connection.
    conns: Mutex<HashMap<String, UDPMuxConn>>,

    /// Maps from ip address to the underlying connection.
    address_map: RwLock<HashMap<SocketAddr, UDPMuxConn>>,

    // Close sender
    closed_watch_tx: Mutex<Option<watch::Sender<()>>>,
}

impl UDPMuxDefault {

    pub async fn is_closed(&self) -> bool {
        self.closed_watch_tx.lock().await.is_none()
    }

    async fn send_to(&self, buf: &[u8], target: &SocketAddr) -> Result<usize, Error> {
        self.params
            .conn
            .send_to(buf, *target)
            .await
            .map_err(Into::into)
    }

    /// Create a muxed connection for a given ufrag.
    async fn create_muxed_conn(self: &Arc<Self>, ufrag: &str) -> Result<UDPMuxConn, Error> {
        let local_addr = self.params.conn.local_addr().await?;

        let params = UDPMuxConnParams {
            local_addr,
            key: ufrag.into(),
            udp_mux: Arc::clone(self),
        };

        Ok(UDPMuxConn::new(params))
    }

    async fn register_conn_for_address(&self, conn: &UDPMuxConn, addr: SocketAddr) {
        if self.is_closed().await {
            return;
        }

        let key = conn.key();
        {
            let mut addresses = self.address_map.write();

            addresses
                .entry(addr)
                .and_modify(|e| {
                    if e.key() != key {
                        e.remove_address(&addr);
                        *e = conn.clone()
                    }
                })
                .or_insert_with(|| conn.clone());
        }

        log::debug!("Registered {} for {}", addr, key);
    }
}

#[async_trait]
impl UDPMux for UDPMuxDefault {
    async fn close(&self) -> Result<(), Error> {
        if self.is_closed().await {
            return Err(Error::ErrAlreadyClosed);
        }

        let mut closed_tx = self.closed_watch_tx.lock().await;

        if let Some(tx) = closed_tx.take() {
            let _ = tx.send(());
            drop(closed_tx);

            let old_conns = {
                let mut conns = self.conns.lock().await;

                std::mem::take(&mut (*conns))
            };

            // NOTE: We don't wait for these closure to complete
            for (_, conn) in old_conns.into_iter() {
                conn.close();
            }

            {
                let mut address_map = self.address_map.write();

                // NOTE: This is important, we need to drop all instances of `UDPMuxConn` to
                // avoid a retain cycle due to the use of [`std::sync::Arc`] on both sides.
                let _ = std::mem::take(&mut (*address_map));
            }
        }

        Ok(())
    }

    async fn get_conn(self: Arc<Self>, ufrag: &str) -> Result<Arc<dyn Conn + Send + Sync>, Error> {
        if self.is_closed().await {
            return Err(Error::ErrUseClosedNetworkConn);
        }

        {
            let mut conns = self.conns.lock().await;
            if let Some(conn) = conns.get(ufrag) {
                // UDPMuxConn uses `Arc` internally so it's cheap to clone, but because
                // we implement `Conn` we need to further wrap it in an `Arc` here.
                return Ok(Arc::new(conn.clone()) as Arc<dyn Conn + Send + Sync>);
            }

            let muxed_conn = self.create_muxed_conn(ufrag).await?;
            let mut close_rx = muxed_conn.close_rx();
            let cloned_self = Arc::clone(&self);
            let cloned_ufrag = ufrag.to_string();
            tokio::spawn(async move {
                let _ = close_rx.changed().await;

                // Arc needed
                cloned_self.remove_conn_by_ufrag(&cloned_ufrag).await;
            });

            conns.insert(ufrag.into(), muxed_conn.clone());

            Ok(Arc::new(muxed_conn) as Arc<dyn Conn + Send + Sync>)
        }
    }

    async fn remove_conn_by_ufrag(&self, ufrag: &str) {
        // Pion's ice implementation has both `RemoveConnByFrag` and `RemoveConn`, but since `conns`
        // is keyed on `ufrag` their implementation is equivalent.

        let removed_conn = {
            let mut conns = self.conns.lock().await;
            conns.remove(ufrag)
        };

        if let Some(conn) = removed_conn {
            let mut address_map = self.address_map.write();

            for address in conn.get_addresses() {
                address_map.remove(&address);
            }
        }
    }
}
