use serde::{Deserialize, Serialize};
use std::fmt;

/// RTCPMuxPolicy affects what ICE candidates are gathered to support
/// non-multiplexed RTCP.
#[derive(Debug, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub(crate) enum RTCRtcpMuxPolicy {
    Unspecified = 0,

    /// RTCPMuxPolicyNegotiate indicates to gather ICE candidates for both
    /// RTP and RTCP candidates. If the remote-endpoint is capable of
    /// multiplexing RTCP, multiplex RTCP on the RTP candidates. If it is not,
    /// use both the RTP and RTCP candidates separately.
    #[serde(rename = "negotiate")]
    Negotiate = 1,

    /// RTCPMuxPolicyRequire indicates to gather ICE candidates only for
    /// RTP and multiplex RTCP on the RTP candidates. If the remote endpoint is
    /// not capable of rtcp-mux, session negotiation will fail.
    #[serde(rename = "require")]
    Require = 2,
}

impl Default for RTCRtcpMuxPolicy {
    fn default() -> Self {
        RTCRtcpMuxPolicy::Negotiate
    }
}

const RTCP_MUX_POLICY_NEGOTIATE_STR: &str = "negotiate";
const RTCP_MUX_POLICY_REQUIRE_STR: &str = "require";

impl From<&str> for RTCRtcpMuxPolicy {
    fn from(raw: &str) -> Self {
        match raw {
            RTCP_MUX_POLICY_NEGOTIATE_STR => RTCRtcpMuxPolicy::Negotiate,
            RTCP_MUX_POLICY_REQUIRE_STR => RTCRtcpMuxPolicy::Require,
            _ => RTCRtcpMuxPolicy::Unspecified,
        }
    }
}

impl fmt::Display for RTCRtcpMuxPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match *self {
            RTCRtcpMuxPolicy::Negotiate => RTCP_MUX_POLICY_NEGOTIATE_STR,
            RTCRtcpMuxPolicy::Require => RTCP_MUX_POLICY_REQUIRE_STR,
            RTCRtcpMuxPolicy::Unspecified => crate::webrtc::UNSPECIFIED_STR,
        };
        write!(f, "{}", s)
    }
}
