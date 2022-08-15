
use crate::webrtc::ice_transport::ice_candidate_type::RTCIceCandidateType;
use crate::webrtc::ice::agent::agent_config::InterfaceFilterFn;
use crate::webrtc::ice::mdns::MulticastDnsMode;

use std::sync::Arc;

#[derive(Default, Clone)]
pub(crate) struct Candidates {
    pub(crate) interface_filter: Arc<Option<InterfaceFilterFn>>,
    pub(crate) nat_1to1_ips: Vec<String>,
    pub(crate) nat_1to1_ip_candidate_type: RTCIceCandidateType,
    pub(crate) multicast_dns_mode: MulticastDnsMode,
    pub(crate) multicast_dns_host_name: String,
    pub(crate) username_fragment: String,
    pub(crate) password: String,
}

/// SettingEngine allows influencing behavior in ways that are not
/// supported by the WebRTC API. This allows us to support additional
/// use-cases without deviating from the WebRTC API elsewhere.
#[derive(Default, Clone)]
pub(crate) struct SettingEngine {
    pub(crate) candidates: Candidates,
}

impl SettingEngine {
    pub(crate) fn new() -> Self {
        Self {
            candidates: Candidates {
                interface_filter: Arc::new(None),
                nat_1to1_ips: vec![],
                nat_1to1_ip_candidate_type: Default::default(),
                multicast_dns_mode: Default::default(),
                multicast_dns_host_name: "".to_string(),
                username_fragment: "".to_string(),
                password: "".to_string()
            }
        }
    }
}
