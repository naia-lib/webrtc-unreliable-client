use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

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
    // The candidate line looks like
    // "candidate:<foundation> <component> udp <priority> <ip> <port> typ host ...";
    // take the first adjacent (ip, port) token pair.
    let tokens: Vec<&str> = candidate_str.split_whitespace().collect();
    for w in tokens.windows(2) {
        if let Ok(ip_addr) = w[0].parse::<IpAddr>() {
            if let Ok(port) = w[1].parse::<u16>() {
                return ServerAddr::Found(SocketAddr::new(ip_addr, port));
            }
        }
    }
    panic!("failed to find a SocketAddr in the candidate string");
}
