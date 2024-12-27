use std::{
    fmt,
    net::{IpAddr, SocketAddr},
    ops::{BitAnd, BitOr},
    sync::atomic::{AtomicU64, Ordering},
    time::SystemTime,
};

use super::net::UDP_STR;

lazy_static! {
    static ref TAG_CTR: AtomicU64 = AtomicU64::new(0);
}

/// Encodes a u64 value to a lowercase base 36 string.
pub(crate) fn base36(value: impl Into<u64>) -> String {
    let mut digits: Vec<u8> = vec![];

    let mut value = value.into();
    while value > 0 {
        let digit = (value % 36) as usize;
        value /= 36;

        digits.push(b"0123456789abcdefghijklmnopqrstuvwxyz"[digit]);
    }

    digits.reverse();
    format!("{:0>8}", String::from_utf8(digits).unwrap())
}

// Generate a base36-encoded unique tag
// See: https://play.golang.org/p/0ZaAID1q-HN
fn assign_chunk_tag() -> String {
    let n = TAG_CTR.fetch_add(1, Ordering::SeqCst);
    base36(n)
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) struct TcpFlag(pub(crate) u8);

pub(crate) const TCP_FLAG_ZERO: TcpFlag = TcpFlag(0x00);
pub(crate) const TCP_FLAG_FIN: TcpFlag = TcpFlag(0x01);
pub(crate) const TCP_FLAG_SYN: TcpFlag = TcpFlag(0x02);
pub(crate) const TCP_FLAG_RST: TcpFlag = TcpFlag(0x04);
pub(crate) const TCP_FLAG_PSH: TcpFlag = TcpFlag(0x08);
pub(crate) const TCP_FLAG_ACK: TcpFlag = TcpFlag(0x10);

impl BitOr for TcpFlag {
    type Output = Self;

    // rhs is the "right-hand side" of the expression `a | b`
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitAnd for TcpFlag {
    type Output = Self;

    // rhs is the "right-hand side" of the expression `a & b`
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl fmt::Display for TcpFlag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut sa = vec![];
        if *self & TCP_FLAG_FIN != TCP_FLAG_ZERO {
            sa.push("FIN");
        }
        if *self & TCP_FLAG_SYN != TCP_FLAG_ZERO {
            sa.push("SYN");
        }
        if *self & TCP_FLAG_RST != TCP_FLAG_ZERO {
            sa.push("RST");
        }
        if *self & TCP_FLAG_PSH != TCP_FLAG_ZERO {
            sa.push("PSH");
        }
        if *self & TCP_FLAG_ACK != TCP_FLAG_ZERO {
            sa.push("ACK");
        }

        write!(f, "{}", sa.join("-"))
    }
}

// Chunk represents a packet passed around in the vnet
pub(crate) trait Chunk: fmt::Display + fmt::Debug {
    fn set_timestamp(&mut self) -> SystemTime; // used by router
    fn get_destination_ip(&self) -> IpAddr; // used by router

    fn source_addr(&self) -> SocketAddr;
    fn destination_addr(&self) -> SocketAddr;
    fn user_data(&self) -> Vec<u8>;
    fn tag(&self) -> String;
    fn network(&self) -> String; // returns "udp" or "tcp"
}

#[derive(PartialEq, Debug)]
pub(crate) struct ChunkIp {
    pub(crate) timestamp: SystemTime,
    pub(crate) source_ip: IpAddr,
    pub(crate) destination_ip: IpAddr,
    pub(crate) tag: String,
}

impl ChunkIp {
    fn set_timestamp(&mut self) -> SystemTime {
        self.timestamp = SystemTime::now();
        self.timestamp
    }

    fn get_destination_ip(&self) -> IpAddr {
        self.destination_ip
    }

    fn tag(&self) -> String {
        self.tag.clone()
    }
}

#[derive(PartialEq, Debug)]
pub(crate) struct ChunkUdp {
    pub(crate) chunk_ip: ChunkIp,
    pub(crate) source_port: u16,
    pub(crate) destination_port: u16,
    pub(crate) user_data: Vec<u8>,
}

impl fmt::Display for ChunkUdp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} chunk {} {} => {}",
            self.network(),
            self.tag(),
            self.source_addr(),
            self.destination_addr(),
        )
    }
}

impl Chunk for ChunkUdp {
    fn set_timestamp(&mut self) -> SystemTime {
        self.chunk_ip.set_timestamp()
    }

    fn get_destination_ip(&self) -> IpAddr {
        self.chunk_ip.get_destination_ip()
    }

    fn tag(&self) -> String {
        self.chunk_ip.tag()
    }

    fn source_addr(&self) -> SocketAddr {
        SocketAddr::new(self.chunk_ip.source_ip, self.source_port)
    }

    fn destination_addr(&self) -> SocketAddr {
        SocketAddr::new(self.chunk_ip.destination_ip, self.destination_port)
    }

    fn user_data(&self) -> Vec<u8> {
        self.user_data.clone()
    }

    fn network(&self) -> String {
        UDP_STR.to_owned()
    }
}

impl ChunkUdp {
    pub(crate) fn new(src_addr: SocketAddr, dst_addr: SocketAddr) -> Self {
        ChunkUdp {
            chunk_ip: ChunkIp {
                timestamp: SystemTime::now(),
                source_ip: src_addr.ip(),
                destination_ip: dst_addr.ip(),
                tag: assign_chunk_tag(),
            },
            source_port: src_addr.port(),
            destination_port: dst_addr.port(),
            user_data: vec![],
        }
    }
}
