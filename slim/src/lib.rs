#[macro_use]
extern crate lazy_static;

mod webrtc;

pub mod peer_connection {
    pub mod sdp {
        pub mod session_description {
            pub use crate::webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
        }
    }
    pub use crate::webrtc::peer_connection::RTCPeerConnection;
}
pub mod data {
    pub mod data_channel {
        pub use crate::webrtc::data::data_channel::DataChannel;
    }
}