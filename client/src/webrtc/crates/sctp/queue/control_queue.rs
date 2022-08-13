use crate::webrtc::sctp::packet::Packet;

use std::collections::VecDeque;

/// control queue
pub type ControlQueue = VecDeque<Packet>;
