mod crates;

// re-export sub-crates
pub use crates::data;
pub use crates::sctp;

pub mod api;
pub mod data_channel;
pub mod dtls_transport;
pub mod error;
pub mod ice_transport;
pub mod mux;
pub mod peer_connection;
pub mod sctp_transport;

pub use error::Error;

pub(crate) const UNSPECIFIED_STR: &str = "Unspecified";

/// Equal to UDP MTU
pub(crate) const RECEIVE_MTU: usize = 1460;
