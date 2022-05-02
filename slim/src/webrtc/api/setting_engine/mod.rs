#[cfg(test)]
mod setting_engine_test;

use crate::webrtc::dtls_transport::dtls_role::DTLSRole;
use crate::webrtc::ice_transport::ice_candidate_type::RTCIceCandidateType;
use dtls::extension::extension_use_srtp::SrtpProtectionProfile;
use ice::agent::agent_config::InterfaceFilterFn;
use ice::mdns::MulticastDnsMode;
use ice::network_type::NetworkType;
use ice::udp_network::UDPNetwork;

use std::sync::Arc;
use tokio::time::Duration;
use util::vnet::net::*;

#[derive(Default, Clone)]
pub struct Detach {
    pub data_channels: bool,
}

#[derive(Default, Clone)]
pub struct Timeout {
    pub ice_disconnected_timeout: Option<Duration>,
    pub ice_failed_timeout: Option<Duration>,
    pub ice_keepalive_interval: Option<Duration>,
    pub ice_host_acceptance_min_wait: Option<Duration>,
    pub ice_srflx_acceptance_min_wait: Option<Duration>,
    pub ice_prflx_acceptance_min_wait: Option<Duration>,
    pub ice_relay_acceptance_min_wait: Option<Duration>,
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

#[derive(Default, Clone)]
pub struct ReplayProtection {
    pub dtls: usize,
    pub srtp: usize,
    pub srtcp: usize,
}

/// SettingEngine allows influencing behavior in ways that are not
/// supported by the WebRTC API. This allows us to support additional
/// use-cases without deviating from the WebRTC API elsewhere.
#[derive(Default, Clone)]
pub struct SettingEngine {
    pub(crate) timeout: Timeout,
    pub(crate) candidates: Candidates,
    pub(crate) replay_protection: ReplayProtection,
    pub(crate) sdp_media_level_fingerprints: bool,
    pub(crate) answering_dtls_role: DTLSRole,
    pub(crate) disable_certificate_fingerprint_verification: bool,
    pub(crate) disable_srtp_replay_protection: bool,
    pub(crate) disable_srtcp_replay_protection: bool,
    pub(crate) vnet: Option<Arc<Net>>,
    //BufferFactory                             :func(packetType packetio.BufferPacketType, ssrc uint32) io.ReadWriteCloser,
    //iceTCPMux                                 :ice.TCPMux,?
    //iceProxyDialer                            :proxy.Dialer,?
    pub(crate) udp_network: UDPNetwork,
    pub(crate) srtp_protection_profiles: Vec<SrtpProtectionProfile>,
}

impl SettingEngine {
    pub fn new() -> Self {
        let mut setting_engine = Self::default();
        setting_engine.answering_dtls_role = DTLSRole::Client;
        setting_engine
    }
}
