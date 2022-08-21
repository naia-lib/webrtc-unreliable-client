use serde::{Deserialize, Serialize};
use std::fmt;

/// ICETransportPolicy defines the ICE candidate policy surface the
/// permitted candidates. Only these candidates are used for connectivity checks.
#[derive(Debug, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub(crate) enum RTCIceTransportPolicy {
    Unspecified = 0,

    /// ICETransportPolicyAll indicates any type of candidate is used.
    #[serde(rename = "all")]
    All = 1,
}

impl Default for RTCIceTransportPolicy {
    fn default() -> Self {
        RTCIceTransportPolicy::Unspecified
    }
}

const ICE_TRANSPORT_POLICY_ALL_STR: &str = "all";

/// takes a string and converts it to ICETransportPolicy
impl From<&str> for RTCIceTransportPolicy {
    fn from(raw: &str) -> Self {
        match raw {
            ICE_TRANSPORT_POLICY_ALL_STR => RTCIceTransportPolicy::All,
            _ => RTCIceTransportPolicy::Unspecified,
        }
    }
}

impl fmt::Display for RTCIceTransportPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match *self {
            RTCIceTransportPolicy::All => ICE_TRANSPORT_POLICY_ALL_STR,
            RTCIceTransportPolicy::Unspecified => crate::webrtc::UNSPECIFIED_STR,
        };
        write!(f, "{}", s)
    }
}