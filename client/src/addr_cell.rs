use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::Arc,
};

use regex::Regex;
use tokio::sync::Mutex;

/// The server's socket address, if it has been found
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ServerAddr {
    /// Client has found the server's socket address
    Found(SocketAddr),
    /// Client is still finding the server's socket address
    Finding,
}

// MaybeAddr
struct MaybeAddr(pub(crate) ServerAddr);

// AddrCell
#[derive(Clone)]
pub struct AddrCell {
    cell: Arc<Mutex<MaybeAddr>>,
}

impl Default for AddrCell {
    fn default() -> Self {
        AddrCell {
            cell: Arc::new(Mutex::new(MaybeAddr(ServerAddr::Finding))),
        }
    }
}

impl AddrCell {
    pub async fn receive_candidate(&self, candidate_str: &str) {
        let mut cell = self.cell.lock().await;
        cell.0 = candidate_to_addr(candidate_str);
    }

    pub fn get(&self) -> ServerAddr {
        match self.cell.try_lock() {
            Ok(addr) => addr.0,
            Err(_) => ServerAddr::Finding,
        }
    }
}

pub(crate) fn candidate_to_addr(candidate_str: &str) -> ServerAddr {
    let pattern = Regex::new(r"\b(?P<ip_addr>(?:[0-9]{1,3}\.){3}[0-9]{1,3}|(([0-9a-fA-F]{1,4}:){7,7}[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,7}:|([0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){1,2}|([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){1,3}|([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){1,4}|([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){1,5}|[0-9a-fA-F]{1,4}:((:[0-9a-fA-F]{1,4}){1,6})|:((:[0-9a-fA-F]{1,4}){1,7}|:)|fe80:(:[0-9a-fA-F]{0,4}){0,4}%[0-9a-zA-Z]{1,}|::(ffff(:0{1,4}){0,1}:){0,1}((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])|([0-9a-fA-F]{1,4}:){1,4}:((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9]))) (?P<port>[0-9]{1,5})\b")
        .expect("failed to compile regex pattern");

    let captures = pattern
        .captures(candidate_str)
        .expect("regex failed to find SocketAddr string");

    let port = &captures["port"].parse::<u16>().expect("not a valid port..");
    if let Ok(ip_addr) = captures["ip_addr"].parse::<Ipv6Addr>() {
        ServerAddr::Found(SocketAddr::new(IpAddr::V6(ip_addr), *port))
    } else {
        let ip_addr = captures["ip_addr"]
            .parse::<Ipv4Addr>()
            .expect("not a valid ip address..");

        ServerAddr::Found(SocketAddr::new(IpAddr::V4(ip_addr), *port))
    }
}
