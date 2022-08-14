use crate::webrtc::util::error::*;
use crate::webrtc::util::vnet::chunk::*;
use crate::webrtc::util::vnet::chunk_queue::*;
use crate::webrtc::util::vnet::interface::*;
use crate::webrtc::util::vnet::nat::*;
use crate::webrtc::util::vnet::resolver::*;

use async_trait::async_trait;
use ipnet::*;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::Duration;

lazy_static! {
    pub(crate) static ref ROUTER_ID_CTR: AtomicU64 = AtomicU64::new(0);
}

// NIC is a network interface controller that interfaces Router
#[async_trait]
pub(crate) trait Nic {
    async fn get_interface(&self, ifc_name: &str) -> Option<Interface>;
    async fn add_addrs_to_interface(&mut self, ifc_name: &str, addrs: &[IpNet]) -> Result<()>;
    async fn on_inbound_chunk(&self, c: Box<dyn Chunk + Send + Sync>);
    async fn get_static_ips(&self) -> Vec<IpAddr>;
    async fn set_router(&self, r: Arc<Mutex<Router>>) -> Result<()>;
}

#[derive(Default)]
pub(crate) struct RouterInternal {
    pub(crate) nat_type: Option<NatType>,          // read-only
    pub(crate) parent: Option<Arc<Mutex<Router>>>, // read-only
    pub(crate) nat: NetworkAddressTranslator,      // read-only
}

// Router ...
#[derive(Default)]
pub(crate) struct Router {
    name: String,                              // read-only
    queue: Arc<ChunkQueue>,                    // read-only
    interfaces: Vec<Interface>,                // read-only
    static_ips: Vec<IpAddr>,                   // read-only
    static_local_ips: HashMap<String, IpAddr>, // read-only,
    done: Option<mpsc::Sender<()>>,            // requires mutex [x]
    pub(crate) resolver: Arc<Mutex<Resolver>>, // read-only
    push_ch: Option<mpsc::Sender<()>>,         // writer requires mutex
    router_internal: Arc<Mutex<RouterInternal>>,
}

#[async_trait]
impl Nic for Router {
    async fn get_interface(&self, ifc_name: &str) -> Option<Interface> {
        for ifc in &self.interfaces {
            if ifc.name == ifc_name {
                return Some(ifc.clone());
            }
        }
        None
    }

    async fn add_addrs_to_interface(&mut self, ifc_name: &str, addrs: &[IpNet]) -> Result<()> {
        for ifc in &mut self.interfaces {
            if ifc.name == ifc_name {
                for addr in addrs {
                    ifc.add_addr(*addr);
                }
                return Ok(());
            }
        }

        Err(Error::ErrNotFound)
    }

    async fn on_inbound_chunk(&self, c: Box<dyn Chunk + Send + Sync>) {
        let from_parent: Box<dyn Chunk + Send + Sync> = {
            let router_internal = self.router_internal.lock().await;
            match router_internal.nat.translate_inbound(&*c).await {
                Ok(from) => {
                    if let Some(from) = from {
                        from
                    } else {
                        return;
                    }
                }
                Err(err) => {
                    log::warn!("[{}] {}", self.name, err);
                    return;
                }
            }
        };

        self.push(from_parent).await;
    }

    async fn get_static_ips(&self) -> Vec<IpAddr> {
        self.static_ips.clone()
    }

    // caller must hold the mutex
    async fn set_router(&self, parent: Arc<Mutex<Router>>) -> Result<()> {
        {
            let mut router_internal = self.router_internal.lock().await;
            router_internal.parent = Some(Arc::clone(&parent));
        }

        let parent_resolver = {
            let p = parent.lock().await;
            Arc::clone(&p.resolver)
        };
        {
            let mut resolver = self.resolver.lock().await;
            resolver.set_parent(parent_resolver);
        }

        let mut mapped_ips = vec![];
        let mut local_ips = vec![];

        // when this method is called, one or more IP address has already been assigned by
        // the parent router.
        if let Some(ifc) = self.get_interface("eth0").await {
            for ifc_addr in ifc.addrs() {
                let ip = ifc_addr.addr();
                mapped_ips.push(ip);

                if let Some(loc_ip) = self.static_local_ips.get(&ip.to_string()) {
                    local_ips.push(*loc_ip);
                }
            }
        } else {
            return Err(Error::ErrNoIpaddrEth0);
        }

        // Set up NAT here
        {
            let mut router_internal = self.router_internal.lock().await;
            if router_internal.nat_type.is_none() {
                router_internal.nat_type = Some(NatType {
                    mapping_behavior: EndpointDependencyType::EndpointIndependent,
                    filtering_behavior: EndpointDependencyType::EndpointAddrPortDependent,
                    port_preservation: false,
                    mapping_life_time: Duration::from_secs(30),
                    ..Default::default()
                });
            }

            router_internal.nat = NetworkAddressTranslator::new(NatConfig {
                name: self.name.clone(),
                nat_type: router_internal.nat_type.unwrap(),
                mapped_ips,
                local_ips,
            })?;
        }

        Ok(())
    }
}

impl Router {

    pub(crate) async fn push(&self, mut c: Box<dyn Chunk + Send + Sync>) {
        log::debug!("[{}] route {}", self.name, c);
        if self.done.is_some() {
            c.set_timestamp();

            if self.queue.push(c).await {
                if let Some(push_ch) = &self.push_ch {
                    let _ = push_ch.try_send(());
                }
            } else {
                log::warn!("[{}] queue was full. dropped a chunk", self.name);
            }
        } else {
            log::warn!("router is done");
        }
    }
}
