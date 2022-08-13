pub mod addr;
pub mod chandata;
pub mod channum;
pub mod data;
pub mod dontfrag;
pub mod evenport;
pub mod lifetime;
pub mod peeraddr;
pub mod relayaddr;
pub mod reqfamily;
pub mod reqtrans;
pub mod rsrvtoken;

use std::fmt;

// proto implements RFC 5766 Traversal Using Relays around NAT.

// protocol is IANA assigned protocol number.
#[derive(PartialEq, Eq, Default, Debug, Clone, Copy)]
pub struct Protocol(pub u8);

// PROTO_UDP is IANA assigned protocol number for UDP.
pub const PROTO_TCP: Protocol = Protocol(6);
pub const PROTO_UDP: Protocol = Protocol(17);

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let others = format!("{}", self.0);
        let s = match *self {
            PROTO_UDP => "UDP",
            PROTO_TCP => "TCP",
            _ => others.as_str(),
        };

        write!(f, "{}", s)
    }
}