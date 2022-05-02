
use crate::webrtc::ice_transport::ice_candidate_type::RTCIceCandidateType;
use ice::agent::agent_config::InterfaceFilterFn;
use ice::mdns::MulticastDnsMode;
use ice::network_type::NetworkType;

use std::sync::Arc;

#[derive(Default, Clone)]
pub struct Detach {
    pub data_channels: bool,
}

#[derive(Default, Clone)]
pub struct Candidates {
    pub ice_lite: bool,
    pub ice_network_types: Vec<NetworkType>,
    pub interface_filter: Arc<Option<InterfaceFilterFn>>,
    pub nat_1to1_ips: Vec<String>,
    pub nat_1to1_ip_candidate_type: RTCIceCandidateType,
    pub multicast_dns_mode: MulticastDnsMode,
    pub multicast_dns_host_name: String,
    pub username_fragment: String,
    pub password: String,
}

/// SettingEngine allows influencing behavior in ways that are not
/// supported by the WebRTC API. This allows us to support additional
/// use-cases without deviating from the WebRTC API elsewhere.
#[derive(Default, Clone)]
pub struct SettingEngine {
    pub(crate) candidates: Candidates,
}

impl SettingEngine {
    pub fn new() -> Self {
        let setting_engine = Self::default();
        setting_engine
    }
}
