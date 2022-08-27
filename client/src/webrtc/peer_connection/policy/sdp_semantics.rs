use serde::{Deserialize, Serialize};
use std::fmt;

/// SDPSemantics determines which style of SDP offers and answers
/// can be used
#[derive(Debug, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub(crate) enum RTCSdpSemantics {
    Unspecified = 0,

    /// UnifiedPlan uses unified-plan offers and answers
    /// (the default in Chrome since M72)
    /// <https://tools.ietf.org/html/draft-roach-mmusic-unified-plan-00>
    #[serde(rename = "unified-plan")]
    UnifiedPlan = 1,
}

impl Default for RTCSdpSemantics {
    fn default() -> Self {
        RTCSdpSemantics::UnifiedPlan
    }
}

const SDP_SEMANTICS_UNIFIED_PLAN: &str = "unified-plan";

impl From<&str> for RTCSdpSemantics {
    fn from(raw: &str) -> Self {
        match raw {
            SDP_SEMANTICS_UNIFIED_PLAN => RTCSdpSemantics::UnifiedPlan,
            _ => RTCSdpSemantics::Unspecified,
        }
    }
}

impl fmt::Display for RTCSdpSemantics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match *self {
            RTCSdpSemantics::UnifiedPlan => SDP_SEMANTICS_UNIFIED_PLAN,
            RTCSdpSemantics::Unspecified => crate::webrtc::UNSPECIFIED_STR,
        };
        write!(f, "{}", s)
    }
}
