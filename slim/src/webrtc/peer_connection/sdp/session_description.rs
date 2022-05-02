use crate::webrtc::error::Result;

use super::sdp_type::RTCSdpType;

use crate::webrtc::sdp::description::session::SessionDescription;
use serde::{Deserialize, Serialize};
use std::io::Cursor;

/// SessionDescription is used to expose local and remote session descriptions.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RTCSessionDescription {
    #[serde(rename = "type")]
    pub sdp_type: RTCSdpType,

    pub sdp: String,

    /// This will never be initialized by callers, internal use only
    #[serde(skip)]
    pub(crate) parsed: Option<SessionDescription>,
}

impl RTCSessionDescription {
    /// Given SDP representing an answer, wrap it in an RTCSessionDescription
    /// that can be given to an RTCPeerConnection.
    pub fn answer(sdp: String) -> Result<RTCSessionDescription> {
        let mut desc = RTCSessionDescription {
            sdp,
            sdp_type: RTCSdpType::Answer,
            parsed: None,
        };

        let parsed = desc.unmarshal()?;
        desc.parsed = Some(parsed);

        Ok(desc)
    }

    /// Unmarshal is a helper to deserialize the sdp
    pub(crate) fn unmarshal(&self) -> Result<SessionDescription> {
        let mut reader = Cursor::new(self.sdp.as_bytes());
        let parsed = SessionDescription::unmarshal(&mut reader)?;
        Ok(parsed)
    }
}