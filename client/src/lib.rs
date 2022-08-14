#![recursion_limit = "256"]

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_derive;

mod socket;
mod addr_cell;

pub use socket::Socket;
pub use addr_cell::{AddrCell, ServerAddr};

mod webrtc;

pub(crate) mod peer_connection {
    pub(crate) mod sdp {
        pub(crate) mod session_description {
            pub(crate) use crate::webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
        }
    }
    pub(crate) use crate::webrtc::peer_connection::RTCPeerConnection;
}
pub(crate) mod data {
    pub(crate) mod data_channel {
        pub(crate) use crate::webrtc::data::data_channel::DataChannel;
    }
}