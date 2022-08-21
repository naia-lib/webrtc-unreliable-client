
use crate::webrtc::util::error::*;
use crate::webrtc::util::vnet::chunk::Chunk;
use crate::webrtc::util::vnet::net::UDP_STR;

use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;

// EndpointDependencyType defines a type of behavioral dependendency on the
// remote endpoint's IP address or port number. This is used for the two
// kinds of behaviors:
//  - Port Mapping behavior
//  - Filtering behavior
// See: https://tools.ietf.org/html/rfc4787
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum EndpointDependencyType {
    // EndpointIndependent means the behavior is independent of the endpoint's address or port
    EndpointIndependent,
}

impl Default for EndpointDependencyType {
    fn default() -> Self {
        EndpointDependencyType::EndpointIndependent
    }
}

// NATMode defines basic behavior of the NAT
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum NatMode {
    // NATModeNormal means the NAT behaves as a standard NAPT (RFC 2663).
    Normal,
    // NATModeNAT1To1 exhibits 1:1 DNAT where the external IP address is statically mapped to
    // a specific local IP address with port number is preserved always between them.
    // When this mode is selected, mapping_behavior, filtering_behavior, port_preservation and
    // mapping_life_time of NATType are ignored.
    Nat1To1,
}

impl Default for NatMode {
    fn default() -> Self {
        NatMode::Normal
    }
}

// NATType has a set of parameters that define the behavior of NAT.
#[derive(Default, Debug, Copy, Clone)]
pub(crate) struct NatType {
    pub(crate) mode: NatMode,
    pub(crate) filtering_behavior: EndpointDependencyType,
}

#[derive(Debug, Clone)]
pub(crate) struct Mapping {
    proto: String,                        // "udp" or "tcp"
    local: String,                        // "<local-ip>:<local-port>"
    mapped: String,                       // "<mapped-ip>:<mapped-port>"
    bound: String,                        // key: "[<remote-ip>[:<remote-port>]]"
    filters: Arc<Mutex<HashSet<String>>>, // key: "[<remote-ip>[:<remote-port>]]"
    expires: Arc<Mutex<SystemTime>>,      // time to expire
}

impl Default for Mapping {
    fn default() -> Self {
        Mapping {
            proto: String::new(),                             // "udp" or "tcp"
            local: String::new(),                             // "<local-ip>:<local-port>"
            mapped: String::new(),                            // "<mapped-ip>:<mapped-port>"
            bound: String::new(), // key: "[<remote-ip>[:<remote-port>]]"
            filters: Arc::new(Mutex::new(HashSet::new())), // key: "[<remote-ip>[:<remote-port>]]"
            expires: Arc::new(Mutex::new(SystemTime::now())), // time to expire
        }
    }
}

#[derive(Default, Debug, Clone)]
pub(crate) struct NetworkAddressTranslator {
    pub(crate) name: String,
    pub(crate) nat_type: NatType,
    pub(crate) mapped_ips: Vec<IpAddr>, // mapped IPv4
    pub(crate) local_ips: Vec<IpAddr>,  // local IPv4, required only when the mode is NATModeNAT1To1
    pub(crate) outbound_map: Arc<Mutex<HashMap<String, Arc<Mapping>>>>, // key: "<proto>:<local-ip>:<local-port>[:remote-ip[:remote-port]]
    pub(crate) inbound_map: Arc<Mutex<HashMap<String, Arc<Mapping>>>>, // key: "<proto>:<mapped-ip>:<mapped-port>"
}

impl NetworkAddressTranslator {

    pub(crate) fn get_paired_local_ip(&self, mapped_ip: &IpAddr) -> Option<&IpAddr> {
        for (i, ip) in self.mapped_ips.iter().enumerate() {
            if ip == mapped_ip {
                return self.local_ips.get(i);
            }
        }
        None
    }

    pub(crate) async fn translate_inbound(
        &self,
        from: &(dyn Chunk + Send + Sync),
    ) -> Result<Option<Box<dyn Chunk + Send + Sync>>> {
        let mut to = from.clone_to();

        if from.network() == UDP_STR {
            if self.nat_type.mode == NatMode::Nat1To1 {
                // 1:1 NAT behavior
                let dst_addr = from.destination_addr();
                if let Some(dst_ip) = self.get_paired_local_ip(&dst_addr.ip()) {
                    let dst_port = from.destination_addr().port();
                    to.set_destination_addr(&format!("{}:{}", dst_ip, dst_port))?;
                } else {
                    return Err(Error::Other(format!(
                        "drop {} as {:?}",
                        from,
                        Error::ErrNoAssociatedLocalAddress
                    )));
                }
            } else {
                // Normal (NAPT) behavior
                let filter_key = match self.nat_type.filtering_behavior {
                    EndpointDependencyType::EndpointIndependent => "".to_owned(),
                };

                let i_key = format!("udp:{}", from.destination_addr());
                if let Some(m) = self.find_inbound_mapping(&i_key).await {
                    {
                        let filters = m.filters.lock().await;
                        if !filters.contains(&filter_key) {
                            return Err(Error::Other(format!(
                                "drop {} as the remote {} {:?}",
                                from,
                                filter_key,
                                Error::ErrHasNoPermission
                            )));
                        }
                    }

                    // See RFC 4847 Section 4.3.  Mapping Refresh
                    // a) Inbound refresh may be useful for applications with no outgoing
                    //   UDP traffic.  However, allowing inbound refresh may allow an
                    //   external attacker or misbehaving application to keep a Mapping
                    //   alive indefinitely.  This may be a security risk.  Also, if the
                    //   process is repeated with different ports, over time, it could
                    //   use up all the ports on the NAT.

                    to.set_destination_addr(&m.local)?;
                } else {
                    return Err(Error::Other(format!(
                        "drop {} as {:?}",
                        from,
                        Error::ErrNoNatBindingFound
                    )));
                }
            }

            log::debug!(
                "[{}] translate inbound chunk from {} to {}",
                self.name,
                from,
                to
            );

            return Ok(Some(to));
        }

        Err(Error::ErrNonUdpTranslationNotSupported)
    }

    // caller must hold the mutex
    pub(crate) async fn find_inbound_mapping(&self, i_key: &str) -> Option<Arc<Mapping>> {
        let mut expired = false;
        let (in_key, out_key) = {
            let inbound_map = self.inbound_map.lock().await;
            if let Some(m) = inbound_map.get(i_key) {
                let now = SystemTime::now();

                {
                    let expires = m.expires.lock().await;
                    // check if this Mapping is expired
                    if now.duration_since(*expires).is_ok() {
                        expired = true;
                    }
                }
                (
                    NetworkAddressTranslator::get_inbound_map_key(m),
                    NetworkAddressTranslator::get_outbound_map_key(m),
                )
            } else {
                (String::new(), String::new())
            }
        };

        if expired {
            {
                let mut inbound_map = self.inbound_map.lock().await;
                inbound_map.remove(&in_key);
            }
            {
                let mut outbound_map = self.outbound_map.lock().await;
                outbound_map.remove(&out_key);
            }
        }

        let inbound_map = self.inbound_map.lock().await;
        inbound_map.get(i_key).map(Arc::clone)
    }

    // caller must hold the mutex
    fn get_outbound_map_key(m: &Mapping) -> String {
        format!("{}:{}:{}", m.proto, m.local, m.bound)
    }

    fn get_inbound_map_key(m: &Mapping) -> String {
        format!("{}:{}", m.proto, m.mapped)
    }
}
