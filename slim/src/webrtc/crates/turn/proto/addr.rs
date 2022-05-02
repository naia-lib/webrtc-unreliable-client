#[cfg(test)]
mod addr_test;

use super::*;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

// Addr is ip:port.
#[derive(PartialEq, Eq, Debug)]
pub struct Addr {
    ip: IpAddr,
    port: u16,
}

impl Default for Addr {
    fn default() -> Self {
        Addr {
            ip: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            port: 0,
        }
    }
}

impl fmt::Display for Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.ip, self.port)
    }
}

// FiveTuple represents 5-TUPLE value.
#[derive(PartialEq, Eq, Default)]
pub struct FiveTuple {
    pub client: Addr,
    pub server: Addr,
    pub proto: Protocol,
}

impl fmt::Display for FiveTuple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}->{} ({})", self.client, self.server, self.proto)
    }
}
