pub(crate) mod addr;
pub(crate) mod chandata;
pub(crate) mod channum;
pub(crate) mod data;
pub(crate) mod dontfrag;
pub(crate) mod evenport;
pub(crate) mod lifetime;
pub(crate) mod peeraddr;
pub(crate) mod relayaddr;
pub(crate) mod reqfamily;
pub(crate) mod reqtrans;
pub(crate) mod rsrvtoken;

use std::fmt;

// proto implements RFC 5766 Traversal Using Relays around NAT.

// protocol is IANA assigned protocol number.
#[derive(PartialEq, Eq, Default, Debug, Clone, Copy)]
pub(crate) struct Protocol(pub(crate) u8);

// PROTO_UDP is IANA assigned protocol number for UDP.
pub(crate) const PROTO_TCP: Protocol = Protocol(6);
pub(crate) const PROTO_UDP: Protocol = Protocol(17);

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