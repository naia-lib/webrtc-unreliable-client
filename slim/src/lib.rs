#[macro_use]
extern crate lazy_static;

mod webrtc;

pub mod api {
    pub use crate::webrtc::api::API;
}
pub mod ice_transport {
    pub mod ice_candidate {
        pub use crate::webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
    }
}
pub mod peer_connection {
    pub mod sdp {
        pub mod sdp_type {
            pub use crate::webrtc::peer_connection::sdp::sdp_type::RTCSdpType;
        }
        pub mod session_description {
            pub use crate::webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
        }
    }
}
pub mod data {
    pub mod data_channel {
        pub use crate::webrtc::data::data_channel::DataChannel;
    }
}