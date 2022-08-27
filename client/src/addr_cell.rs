use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use regex::Regex;
use tokio::sync::Mutex;

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
            Ok(addr) => {
                addr.0
            }
            Err(_) => {
                ServerAddr::Finding
            }
        }
    }
}

/// The server's socket address, if it has been found
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ServerAddr {
    /// Client has found the server's socket address
    Found(SocketAddr),
    /// Client is still finding the server's socket address
    Finding,
}

pub(crate) fn candidate_to_addr(candidate_str: &str) -> ServerAddr {
    let pattern =
        Regex::new(r"\b(?P<ip_addr>(?:[0-9]{1,3}\.){3}[0-9]{1,3}) (?P<port>[0-9]{1,5})\b")
            .expect("failed to compile regex pattern");

    let captures = pattern
        .captures(candidate_str)
        .expect("regex failed to find SocketAddr string");

    let ip_addr = captures["ip_addr"]
        .parse::<Ipv4Addr>()
        .expect("not a valid ip address..");
    let port = &captures["port"].parse::<u16>().expect("not a valid port..");

    ServerAddr::Found(SocketAddr::new(IpAddr::V4(ip_addr), *port))
}
