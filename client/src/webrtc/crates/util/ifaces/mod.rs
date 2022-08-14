pub(crate) mod ffi;
pub(crate) use ffi::ifaces;

#[derive(PartialEq, Eq, Debug, Clone)]
pub(crate) enum NextHop {
    Broadcast(::std::net::SocketAddr),
    Destination(::std::net::SocketAddr),
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub(crate) enum Kind {
    Packet,
    Link,
    Ipv4,
    Ipv6,
    Unknow(i32),
}

#[derive(Debug, Clone)]
pub(crate) struct Interface {
    pub(crate) name: String,
    pub(crate) kind: Kind,
    pub(crate) addr: Option<::std::net::SocketAddr>,
    pub(crate) mask: Option<::std::net::SocketAddr>,
    pub(crate) hop: Option<NextHop>,
}
