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

    /// PlanB uses plan-b offers and answers
    /// NB: This format should be considered deprecated
    /// <https://tools.ietf.org/html/draft-uberti-rtcweb-plan-00>
    #[serde(rename = "plan-b")]
    PlanB = 2,

    /// UnifiedPlanWithFallback prefers unified-plan
    /// offers and answers, but will respond to a plan-b offer
    /// with a plan-b answer
    #[serde(rename = "unified-plan-with-fallback")]
    UnifiedPlanWithFallback = 3,
}

impl Default for RTCSdpSemantics {
    fn default() -> Self {
        RTCSdpSemantics::UnifiedPlan
    }
}

const SDP_SEMANTICS_UNIFIED_PLAN_WITH_FALLBACK: &str = "unified-plan-with-fallback";
const SDP_SEMANTICS_UNIFIED_PLAN: &str = "unified-plan";
const SDP_SEMANTICS_PLAN_B: &str = "plan-b";

impl From<&str> for RTCSdpSemantics {
    fn from(raw: &str) -> Self {
        match raw {
            SDP_SEMANTICS_UNIFIED_PLAN_WITH_FALLBACK => RTCSdpSemantics::UnifiedPlanWithFallback,
            SDP_SEMANTICS_UNIFIED_PLAN => RTCSdpSemantics::UnifiedPlan,
            SDP_SEMANTICS_PLAN_B => RTCSdpSemantics::PlanB,
            _ => RTCSdpSemantics::Unspecified,
        }
    }
}

impl fmt::Display for RTCSdpSemantics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match *self {
            RTCSdpSemantics::UnifiedPlanWithFallback => SDP_SEMANTICS_UNIFIED_PLAN_WITH_FALLBACK,
            RTCSdpSemantics::UnifiedPlan => SDP_SEMANTICS_UNIFIED_PLAN,
            RTCSdpSemantics::PlanB => SDP_SEMANTICS_PLAN_B,
            RTCSdpSemantics::Unspecified => crate::webrtc::UNSPECIFIED_STR,
        };
        write!(f, "{}", s)
    }
}