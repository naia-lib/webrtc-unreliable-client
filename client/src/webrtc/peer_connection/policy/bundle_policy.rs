use serde::{Deserialize, Serialize};
use std::fmt;

/// BundlePolicy affects which media tracks are negotiated if the remote
/// endpoint is not bundle-aware, and what ICE candidates are gathered. If the
/// remote endpoint is bundle-aware, all media tracks and data channels are
/// bundled onto the same transport.
#[derive(Debug, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub(crate) enum RTCBundlePolicy {
    Unspecified = 0,

    /// BundlePolicyBalanced indicates to gather ICE candidates for each
    /// media type in use (audio, video, and data). If the remote endpoint is
    /// not bundle-aware, negotiate only one audio and video track on separate
    /// transports.
    #[serde(rename = "balanced")]
    Balanced = 1,

    /// BundlePolicyMaxCompat indicates to gather ICE candidates for each
    /// track. If the remote endpoint is not bundle-aware, negotiate all media
    /// tracks on separate transports.
    #[serde(rename = "max-compat")]
    MaxCompat = 2,

    /// BundlePolicyMaxBundle indicates to gather ICE candidates for only
    /// one track. If the remote endpoint is not bundle-aware, negotiate only
    /// one media track.
    #[serde(rename = "max-bundle")]
    MaxBundle = 3,
}

impl Default for RTCBundlePolicy {
    fn default() -> Self {
        RTCBundlePolicy::Unspecified
    }
}

/// This is done this way because of a linter.
const BUNDLE_POLICY_BALANCED_STR: &str = "balanced";
const BUNDLE_POLICY_MAX_COMPAT_STR: &str = "max-compat";
const BUNDLE_POLICY_MAX_BUNDLE_STR: &str = "max-bundle";

impl From<&str> for RTCBundlePolicy {
    /// NewSchemeType defines a procedure for creating a new SchemeType from a raw
    /// string naming the scheme type.
    fn from(raw: &str) -> Self {
        match raw {
            BUNDLE_POLICY_BALANCED_STR => RTCBundlePolicy::Balanced,
            BUNDLE_POLICY_MAX_COMPAT_STR => RTCBundlePolicy::MaxCompat,
            BUNDLE_POLICY_MAX_BUNDLE_STR => RTCBundlePolicy::MaxBundle,
            _ => RTCBundlePolicy::Unspecified,
        }
    }
}

impl fmt::Display for RTCBundlePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            RTCBundlePolicy::Balanced => write!(f, "{}", BUNDLE_POLICY_BALANCED_STR),
            RTCBundlePolicy::MaxCompat => write!(f, "{}", BUNDLE_POLICY_MAX_COMPAT_STR),
            RTCBundlePolicy::MaxBundle => write!(f, "{}", BUNDLE_POLICY_MAX_BUNDLE_STR),
            _ => write!(f, "{}", crate::webrtc::UNSPECIFIED_STR),
        }
    }
}
