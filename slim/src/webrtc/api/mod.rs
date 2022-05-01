pub mod media_engine;
pub mod setting_engine;

use crate::webrtc::dtls_transport::RTCDtlsTransport;
use crate::webrtc::ice_transport::ice_gatherer::RTCIceGatherOptions;
use crate::webrtc::ice_transport::ice_gatherer::RTCIceGatherer;
use crate::webrtc::ice_transport::RTCIceTransport;
use crate::webrtc::peer_connection::certificate::RTCCertificate;

use media_engine::*;
use setting_engine::*;

use crate::webrtc::error::{Error, Result};
use crate::webrtc::sctp_transport::RTCSctpTransport;
use interceptor::registry::Registry;

use rcgen::KeyPair;
use std::sync::Arc;
use std::time::SystemTime;

/// API bundles the global functions of the WebRTC and ORTC API.
/// Some of these functions are also exported globally using the
/// defaultAPI object. Note that the global version of the API
/// may be phased out in the future.
pub struct API {
    pub(crate) setting_engine: Arc<SettingEngine>,
    pub(crate) media_engine: Arc<MediaEngine>,
    pub(crate) interceptor_registry: Registry,
}

impl API {

    /// new_ice_gatherer creates a new ice gatherer.
    /// This constructor is part of the ORTC API. It is not
    /// meant to be used together with the basic WebRTC API.
    pub fn new_ice_gatherer(&self, opts: RTCIceGatherOptions) -> Result<RTCIceGatherer> {
        let mut validated_servers = vec![];
        if !opts.ice_servers.is_empty() {
            for server in &opts.ice_servers {
                let url = server.urls()?;
                validated_servers.extend(url);
            }
        }

        Ok(RTCIceGatherer::new(
            validated_servers,
            opts.ice_gather_policy,
            Arc::clone(&self.setting_engine),
        ))
    }

    /// new_ice_transport creates a new ice transport.
    /// This constructor is part of the ORTC API. It is not
    /// meant to be used together with the basic WebRTC API.
    pub fn new_ice_transport(&self, gatherer: Arc<RTCIceGatherer>) -> RTCIceTransport {
        RTCIceTransport::new(gatherer)
    }

    /// new_dtls_transport creates a new dtls_transport transport.
    /// This constructor is part of the ORTC API. It is not
    /// meant to be used together with the basic WebRTC API.
    pub fn new_dtls_transport(
        &self,
        ice_transport: Arc<RTCIceTransport>,
        mut certificates: Vec<RTCCertificate>,
    ) -> Result<RTCDtlsTransport> {
        if !certificates.is_empty() {
            let now = SystemTime::now();
            for cert in &certificates {
                cert.expires()
                    .duration_since(now)
                    .map_err(|_| Error::ErrCertificateExpired)?;
            }
        } else {
            let kp = KeyPair::generate(&rcgen::PKCS_ECDSA_P256_SHA256)?;
            let cert = RTCCertificate::from_key_pair(kp)?;
            certificates = vec![cert];
        };

        Ok(RTCDtlsTransport::new(
            ice_transport,
            certificates,
            Arc::clone(&self.setting_engine),
        ))
    }

    /// new_sctp_transport creates a new SCTPTransport.
    /// This constructor is part of the ORTC API. It is not
    /// meant to be used together with the basic WebRTC API.
    pub fn new_sctp_transport(
        &self,
        dtls_transport: Arc<RTCDtlsTransport>,
    ) -> Result<RTCSctpTransport> {
        Ok(RTCSctpTransport::new(
            dtls_transport,
            Arc::clone(&self.setting_engine),
        ))
    }
}
