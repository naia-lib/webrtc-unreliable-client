#[cfg(test)]
pub mod peer_connection_test;

pub mod certificate;
pub mod configuration;
pub mod offer_answer_options;
pub mod operation;
mod peer_connection_internal;
pub mod peer_connection_state;
pub mod policy;
pub mod sdp;
pub mod signaling_state;

use crate::webrtc::api::setting_engine::SettingEngine;
use crate::webrtc::api::API;
use crate::webrtc::data_channel::data_channel_state::RTCDataChannelState;
use crate::webrtc::data_channel::RTCDataChannel;
use crate::webrtc::dtls_transport::dtls_fingerprint::RTCDtlsFingerprint;
use crate::webrtc::dtls_transport::dtls_parameters::DTLSParameters;
use crate::webrtc::dtls_transport::dtls_role::{
    DTLSRole, DEFAULT_DTLS_ROLE_OFFER,
};
use crate::webrtc::dtls_transport::dtls_transport_state::RTCDtlsTransportState;
use crate::webrtc::dtls_transport::RTCDtlsTransport;
use crate::webrtc::error::{Error, Result};
use crate::webrtc::ice_transport::ice_candidate::RTCIceCandidate;
use crate::webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use crate::webrtc::ice_transport::ice_gatherer::RTCIceGatherOptions;
use crate::webrtc::ice_transport::ice_gatherer::RTCIceGatherer;
use crate::webrtc::ice_transport::ice_gatherer_state::RTCIceGathererState;
use crate::webrtc::ice_transport::ice_gathering_state::RTCIceGatheringState;
use crate::webrtc::ice_transport::ice_parameters::RTCIceParameters;
use crate::webrtc::ice_transport::ice_role::RTCIceRole;
use crate::webrtc::ice_transport::ice_transport_state::RTCIceTransportState;
use crate::webrtc::ice_transport::RTCIceTransport;
use crate::webrtc::peer_connection::certificate::RTCCertificate;
use crate::webrtc::peer_connection::configuration::RTCConfiguration;
use crate::webrtc::peer_connection::operation::{Operation, Operations};
use crate::webrtc::peer_connection::peer_connection_state::{
    NegotiationNeededState, RTCPeerConnectionState,
};
use crate::webrtc::peer_connection::policy::sdp_semantics::RTCSdpSemantics;
use crate::webrtc::peer_connection::sdp::sdp_type::RTCSdpType;
use crate::webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use crate::webrtc::peer_connection::sdp::*;
use crate::webrtc::peer_connection::signaling_state::{
    check_next_signaling_state, RTCSignalingState, StateChangeOp,
};
use crate::webrtc::sctp_transport::sctp_transport_capabilities::SCTPTransportCapabilities;
use crate::webrtc::sctp_transport::sctp_transport_state::RTCSctpTransportState;
use crate::webrtc::sctp_transport::RTCSctpTransport;

use crate::webrtc::ice::candidate::candidate_base::unmarshal_candidate;
use crate::webrtc::ice::candidate::Candidate;
use crate::webrtc::sdp::description::session::*;
use crate::webrtc::sdp::util::ConnectionRole;
use peer_connection_internal::*;
use rand::{thread_rng, Rng};
use rcgen::KeyPair;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use crate::webrtc::sdp::description::session::ATTR_KEY_ICELITE;
use tokio::sync::Mutex;

pub const MEDIA_SECTION_APPLICATION: &str = "application";

const RUNES_ALPHA: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// math_rand_alpha generates a mathmatical random alphabet sequence of the requested length.
pub fn math_rand_alpha(n: usize) -> String {
    let mut rng = thread_rng();

    let rand_string: String = (0..n)
        .map(|_| {
            let idx = rng.gen_range(0..RUNES_ALPHA.len());
            RUNES_ALPHA[idx] as char
        })
        .collect();

    rand_string
}

pub type OnSignalingStateChangeHdlrFn = Box<
    dyn (FnMut(RTCSignalingState) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>)
        + Send
        + Sync,
>;

pub type OnICEConnectionStateChangeHdlrFn = Box<
    dyn (FnMut(RTCIceConnectionState) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>)
        + Send
        + Sync,
>;

pub type OnPeerConnectionStateChangeHdlrFn = Box<
    dyn (FnMut(RTCPeerConnectionState) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>)
        + Send
        + Sync,
>;

pub type OnDataChannelHdlrFn = Box<
    dyn (FnMut(Arc<RTCDataChannel>) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>)
        + Send
        + Sync,
>;

pub type OnNegotiationNeededHdlrFn =
    Box<dyn (FnMut() -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>) + Send + Sync>;

#[derive(Clone)]
struct CheckNegotiationNeededParams {
    sctp_transport: Arc<RTCSctpTransport>,
    current_local_description: Arc<Mutex<Option<RTCSessionDescription>>>,
}

#[derive(Clone)]
struct NegotiationNeededParams {
    on_negotiation_needed_handler: Arc<Mutex<Option<OnNegotiationNeededHdlrFn>>>,
    is_closed: Arc<AtomicBool>,
    ops: Arc<Operations>,
    negotiation_needed_state: Arc<AtomicU8>,
    is_negotiation_needed: Arc<AtomicBool>,
    signaling_state: Arc<AtomicU8>,
    check_negotiation_needed_params: CheckNegotiationNeededParams,
}

/// PeerConnection represents a WebRTC connection that establishes a
/// peer-to-peer communications with another PeerConnection instance in a
/// browser, or to another endpoint implementing the required protocols.
pub struct RTCPeerConnection {

    idp_login_url: Option<String>,

    configuration: RTCConfiguration,

    pub internal: Arc<PeerConnectionInternal>,
}

impl RTCPeerConnection {
    /// creates a PeerConnection with the default codecs and
    /// interceptors.  See register_default_codecs and register_default_interceptors.
    ///
    /// If you wish to customize the set of available codecs or the set of
    /// active interceptors, create a MediaEngine and call api.new_peer_connection
    /// instead of this function.
    pub async fn new() -> Arc<RTCPeerConnection> {
        let api = API {
            setting_engine: Arc::new(SettingEngine::new()),
        };

        let mut configuration = RTCConfiguration::default();

        RTCPeerConnection::init_configuration(&mut configuration).expect("can't create configuration");

        let (internal, configuration) =
            PeerConnectionInternal::new(&api, configuration).await.expect("can't create peer connection");

        // <https://w3c.github.io/webrtc-pc/#constructor> (Step #2)
        // Some variables defined explicitly despite their implicit zero values to
        // allow better readability to understand what is happening.
        Arc::new(RTCPeerConnection {
            internal,
            configuration,
            idp_login_url: None,
        })
    }

    /// init_configuration defines validation of the specified Configuration and
    /// its assignment to the internal configuration variable. This function differs
    /// from its set_configuration counterpart because most of the checks do not
    /// include verification statements related to the existing state. Thus the
    /// function describes only minor verification of some the struct variables.
    fn init_configuration(configuration: &mut RTCConfiguration) -> Result<()> {
        let sanitized_ice_servers = configuration.get_ice_servers();
        if !sanitized_ice_servers.is_empty() {
            for server in &sanitized_ice_servers {
                server.validate()?;
            }
        }

        // <https://www.w3.org/TR/webrtc/#constructor> (step #3)
        if !configuration.certificates.is_empty() {
            let now = SystemTime::now();
            for cert in &configuration.certificates {
                cert.expires()
                    .duration_since(now)
                    .map_err(|_| Error::ErrCertificateExpired)?;
            }
        } else {
            let kp = KeyPair::generate(&rcgen::PKCS_ECDSA_P256_SHA256)?;
            let cert = RTCCertificate::from_key_pair(kp)?;
            configuration.certificates = vec![cert];
        };

        Ok(())
    }

    async fn do_signaling_state_change(&self, new_state: RTCSignalingState) {
        log::info!("signaling state changed to {}", new_state);
        let mut handler = self.internal.on_signaling_state_change_handler.lock().await;
        if let Some(f) = &mut *handler {
            f(new_state).await;
        }
    }

    fn do_negotiation_needed_inner(params: &NegotiationNeededParams) -> bool {
        // https://w3c.github.io/webrtc-pc/#updating-the-negotiation-needed-flag
        // non-canon step 1
        let state: NegotiationNeededState = params
            .negotiation_needed_state
            .load(Ordering::SeqCst)
            .into();
        if state == NegotiationNeededState::Run {
            params
                .negotiation_needed_state
                .store(NegotiationNeededState::Queue as u8, Ordering::SeqCst);
            false
        } else if state == NegotiationNeededState::Queue {
            false
        } else {
            params
                .negotiation_needed_state
                .store(NegotiationNeededState::Run as u8, Ordering::SeqCst);
            true
        }
    }

    /// do_negotiation_needed enqueues negotiation_needed_op if necessary
    /// caller of this method should hold `pc.mu` lock
    async fn do_negotiation_needed(params: NegotiationNeededParams) {
        if !RTCPeerConnection::do_negotiation_needed_inner(&params) {
            return;
        }

        let params2 = params.clone();
        let _ = params
            .ops
            .enqueue(Operation(Box::new(move || {
                let params3 = params2.clone();
                Box::pin(async move { RTCPeerConnection::negotiation_needed_op(params3).await })
            })))
            .await;
    }

    async fn after_negotiation_needed_op(params: NegotiationNeededParams) -> bool {
        let old_negotiation_needed_state = params.negotiation_needed_state.load(Ordering::SeqCst);

        params
            .negotiation_needed_state
            .store(NegotiationNeededState::Empty as u8, Ordering::SeqCst);

        if old_negotiation_needed_state == NegotiationNeededState::Queue as u8 {
            RTCPeerConnection::do_negotiation_needed_inner(&params)
        } else {
            false
        }
    }

    async fn negotiation_needed_op(params: NegotiationNeededParams) -> bool {
        // Don't run NegotiatedNeeded checks if on_negotiation_needed is not set
        {
            let handler = params.on_negotiation_needed_handler.lock().await;
            if handler.is_none() {
                return false;
            }
        }

        // https://www.w3.org/TR/webrtc/#updating-the-negotiation-needed-flag
        // Step 2.1
        if params.is_closed.load(Ordering::SeqCst) {
            return false;
        }
        // non-canon step 2.2
        if !params.ops.is_empty().await {
            //enqueue negotiation_needed_op again by return true
            return true;
        }

        // non-canon, run again if there was a request
        // starting defer(after_do_negotiation_needed(params).await);

        // Step 2.3
        if params.signaling_state.load(Ordering::SeqCst) != RTCSignalingState::Stable as u8 {
            return RTCPeerConnection::after_negotiation_needed_op(params).await;
        }

        // Step 2.4
        if !RTCPeerConnection::check_negotiation_needed(&params.check_negotiation_needed_params)
            .await
        {
            params.is_negotiation_needed.store(false, Ordering::SeqCst);
            return RTCPeerConnection::after_negotiation_needed_op(params).await;
        }

        // Step 2.5
        if params.is_negotiation_needed.load(Ordering::SeqCst) {
            return RTCPeerConnection::after_negotiation_needed_op(params).await;
        }

        // Step 2.6
        params.is_negotiation_needed.store(true, Ordering::SeqCst);

        // Step 2.7
        {
            let mut handler = params.on_negotiation_needed_handler.lock().await;
            if let Some(f) = &mut *handler {
                f().await;
            }
        }

        RTCPeerConnection::after_negotiation_needed_op(params).await
    }

    async fn check_negotiation_needed(params: &CheckNegotiationNeededParams) -> bool {
        // To check if negotiation is needed for connection, perform the following checks:
        // Skip 1, 2 steps
        // Step 3
        let current_local_description = {
            let current_local_description = params.current_local_description.lock().await;
            current_local_description.clone()
        };

        if let Some(local_desc) = &current_local_description {
            let len_data_channel = {
                let data_channels = params.sctp_transport.data_channels.lock().await;
                data_channels.len()
            };

            if len_data_channel != 0 && have_data_channel(local_desc).is_none() {
                return true;
            }

            false
        } else {
            true
        }
    }

    async fn do_ice_connection_state_change(
        on_ice_connection_state_change_handler: &Arc<
            Mutex<Option<OnICEConnectionStateChangeHdlrFn>>,
        >,
        ice_connection_state: &Arc<AtomicU8>,
        cs: RTCIceConnectionState,
    ) {
        ice_connection_state.store(cs as u8, Ordering::SeqCst);

        log::info!("ICE connection state changed: {}", cs);
        let mut handler = on_ice_connection_state_change_handler.lock().await;
        if let Some(f) = &mut *handler {
            f(cs).await;
        }
    }

    async fn do_peer_connection_state_change(
        on_peer_connection_state_change_handler: &Arc<
            Mutex<Option<OnPeerConnectionStateChangeHdlrFn>>,
        >,
        cs: RTCPeerConnectionState,
    ) {
        let mut handler = on_peer_connection_state_change_handler.lock().await;
        if let Some(f) = &mut *handler {
            f(cs).await;
        }
    }

    /// create_offer starts the PeerConnection and generates the localDescription
    /// <https://w3c.github.io/webrtc-pc/#dom-rtcpeerconnection-createoffer>
    pub async fn create_offer(
        &self,
    ) -> Result<RTCSessionDescription> {
        let use_identity = self.idp_login_url.is_some();
        if use_identity {
            return Err(Error::ErrIdentityProviderNotImplemented);
        } else if self.internal.is_closed.load(Ordering::SeqCst) {
            return Err(Error::ErrConnectionClosed);
        }

        // This may be necessary to recompute if, for example, createOffer was called when only an
        // audio RTCRtpTransceiver was added to connection, but while performing the in-parallel
        // steps to create an offer, a video RTCRtpTransceiver was added, requiring additional
        // inspection of video system resources.
        let offer;

        loop {

            // in-parallel steps to create an offer
            // https://w3c.github.io/webrtc-pc/#dfn-in-parallel-steps-to-create-an-offer
            let is_plan_b = {
                let current_remote_description =
                    self.internal.current_remote_description.lock().await;
                if current_remote_description.is_some() {
                    description_is_plan_b(current_remote_description.as_ref())?
                } else {
                    self.configuration.sdp_semantics == RTCSdpSemantics::PlanB
                }
            };

            // include unmatched local transceivers
            if !is_plan_b {
                // update the greater mid if the remote description provides a greater one
                {
                    let current_remote_description =
                        self.internal.current_remote_description.lock().await;
                    if let Some(d) = &*current_remote_description {
                        if let Some(parsed) = &d.parsed {
                            for media in &parsed.media_descriptions {
                                if let Some(mid) = get_mid_value(media) {
                                    if mid.is_empty() {
                                        continue;
                                    }
                                    let numeric_mid = match mid.parse::<isize>() {
                                        Ok(n) => n,
                                        Err(_) => continue,
                                    };
                                    if numeric_mid
                                        > self.internal.greater_mid.load(Ordering::SeqCst)
                                    {
                                        self.internal
                                            .greater_mid
                                            .store(numeric_mid, Ordering::SeqCst);
                                    }
                                }
                            }
                        }
                    }
                }

            }

            let current_remote_description_is_none = {
                let current_remote_description =
                    self.internal.current_remote_description.lock().await;
                current_remote_description.is_none()
            };

            let mut d = if current_remote_description_is_none {
                self.internal
                    .generate_unmatched_sdp(
                        use_identity,
                        self.configuration.sdp_semantics,
                    )
                    .await?
            } else {
                self.internal
                    .generate_matched_sdp(
                        use_identity,
                        true, /*includeUnmatched */
                        DEFAULT_DTLS_ROLE_OFFER.to_connection_role(),
                    )
                    .await?
            };

            {
                let mut sdp_origin = self.internal.sdp_origin.lock().await;
                update_sdp_origin(&mut sdp_origin, &mut d);
            }
            let sdp = d.marshal();

            offer = RTCSessionDescription {
                sdp_type: RTCSdpType::Offer,
                sdp,
                parsed: Some(d),
            };

            break;
        }

        {
            let mut last_offer = self.internal.last_offer.lock().await;
            *last_offer = offer.sdp.clone();
        }
        Ok(offer)
    }

    /// Update the PeerConnectionState given the state of relevant transports
    /// <https://www.w3.org/TR/webrtc/#rtcpeerconnectionstate-enum>
    async fn update_connection_state(
        on_peer_connection_state_change_handler: &Arc<
            Mutex<Option<OnPeerConnectionStateChangeHdlrFn>>,
        >,
        is_closed: &Arc<AtomicBool>,
        peer_connection_state: &Arc<AtomicU8>,
        ice_connection_state: RTCIceConnectionState,
        dtls_transport_state: RTCDtlsTransportState,
    ) {
        let  connection_state =
        // The RTCPeerConnection object's [[IsClosed]] slot is true.
        if is_closed.load(Ordering::SeqCst) {
             RTCPeerConnectionState::Closed
        }else if ice_connection_state == RTCIceConnectionState::Failed || dtls_transport_state == RTCDtlsTransportState::Failed {
            // Any of the RTCIceTransports or RTCDtlsTransports are in a "failed" state.
             RTCPeerConnectionState::Failed
        }else if ice_connection_state == RTCIceConnectionState::Disconnected {
            // Any of the RTCIceTransports or RTCDtlsTransports are in the "disconnected"
            // state and none of them are in the "failed" or "connecting" or "checking" state.
            RTCPeerConnectionState::Disconnected
        }else if ice_connection_state == RTCIceConnectionState::Connected && dtls_transport_state == RTCDtlsTransportState::Connected {
            // All RTCIceTransports and RTCDtlsTransports are in the "connected", "completed" or "closed"
            // state and at least one of them is in the "connected" or "completed" state.
            RTCPeerConnectionState::Connected
        }else if ice_connection_state == RTCIceConnectionState::Checking && dtls_transport_state == RTCDtlsTransportState::Connecting{
        //  Any of the RTCIceTransports or RTCDtlsTransports are in the "connecting" or
        // "checking" state and none of them is in the "failed" state.
             RTCPeerConnectionState::Connecting
        }else{
            RTCPeerConnectionState::New
        };

        if peer_connection_state.load(Ordering::SeqCst) == connection_state as u8 {
            return;
        }

        log::info!("peer connection state changed: {}", connection_state);
        peer_connection_state.store(connection_state as u8, Ordering::SeqCst);

        RTCPeerConnection::do_peer_connection_state_change(
            on_peer_connection_state_change_handler,
            connection_state,
        )
        .await;
    }

    // 4.4.1.6 Set the SessionDescription
    pub async fn set_description(
        &self,
        sd: &RTCSessionDescription,
        op: StateChangeOp,
    ) -> Result<()> {
        if self.internal.is_closed.load(Ordering::SeqCst) {
            return Err(Error::ErrConnectionClosed);
        } else if sd.sdp_type == RTCSdpType::Unspecified {
            return Err(Error::ErrPeerConnSDPTypeInvalidValue);
        }

        let next_state = {
            let cur = self.signaling_state();
            let new_sdpdoes_not_match_offer = Error::ErrSDPDoesNotMatchOffer;
            let new_sdpdoes_not_match_answer = Error::ErrSDPDoesNotMatchAnswer;

            match op {
                StateChangeOp::SetLocal => {
                    match sd.sdp_type {
                        // stable->SetLocal(offer)->have-local-offer
                        RTCSdpType::Offer => {
                            let check = {
                                let last_offer = self.internal.last_offer.lock().await;
                                sd.sdp != *last_offer
                            };
                            if check {
                                Err(new_sdpdoes_not_match_offer)
                            } else {
                                let next_state = check_next_signaling_state(
                                    cur,
                                    RTCSignalingState::HaveLocalOffer,
                                    StateChangeOp::SetLocal,
                                    sd.sdp_type,
                                );
                                if next_state.is_ok() {
                                    let mut pending_local_description =
                                        self.internal.pending_local_description.lock().await;
                                    *pending_local_description = Some(sd.clone());
                                }
                                next_state
                            }
                        }
                        // have-remote-offer->SetLocal(answer)->stable
                        // have-local-pranswer->SetLocal(answer)->stable
                        RTCSdpType::Answer => {
                            let check = {
                                let last_answer = self.internal.last_answer.lock().await;
                                sd.sdp != *last_answer
                            };
                            if check {
                                Err(new_sdpdoes_not_match_answer)
                            } else {
                                let next_state = check_next_signaling_state(
                                    cur,
                                    RTCSignalingState::Stable,
                                    StateChangeOp::SetLocal,
                                    sd.sdp_type,
                                );
                                if next_state.is_ok() {
                                    let pending_remote_description = {
                                        let mut pending_remote_description =
                                            self.internal.pending_remote_description.lock().await;
                                        pending_remote_description.take()
                                    };
                                    let _pending_local_description = {
                                        let mut pending_local_description =
                                            self.internal.pending_local_description.lock().await;
                                        pending_local_description.take()
                                    };

                                    {
                                        let mut current_local_description =
                                            self.internal.current_local_description.lock().await;
                                        *current_local_description = Some(sd.clone());
                                    }
                                    {
                                        let mut current_remote_description =
                                            self.internal.current_remote_description.lock().await;
                                        *current_remote_description = pending_remote_description;
                                    }
                                }
                                next_state
                            }
                        }
                        RTCSdpType::Rollback => {
                            let next_state = check_next_signaling_state(
                                cur,
                                RTCSignalingState::Stable,
                                StateChangeOp::SetLocal,
                                sd.sdp_type,
                            );
                            if next_state.is_ok() {
                                let mut pending_local_description =
                                    self.internal.pending_local_description.lock().await;
                                *pending_local_description = None;
                            }
                            next_state
                        }
                        // have-remote-offer->SetLocal(pranswer)->have-local-pranswer
                        RTCSdpType::Pranswer => {
                            let check = {
                                let last_answer = self.internal.last_answer.lock().await;
                                sd.sdp != *last_answer
                            };
                            if check {
                                Err(new_sdpdoes_not_match_answer)
                            } else {
                                let next_state = check_next_signaling_state(
                                    cur,
                                    RTCSignalingState::HaveLocalPranswer,
                                    StateChangeOp::SetLocal,
                                    sd.sdp_type,
                                );
                                if next_state.is_ok() {
                                    let mut pending_local_description =
                                        self.internal.pending_local_description.lock().await;
                                    *pending_local_description = Some(sd.clone());
                                }
                                next_state
                            }
                        }
                        _ => Err(Error::ErrPeerConnStateChangeInvalid),
                    }
                }
                StateChangeOp::SetRemote => {
                    match sd.sdp_type {
                        // stable->SetRemote(offer)->have-remote-offer
                        RTCSdpType::Offer => {
                            let next_state = check_next_signaling_state(
                                cur,
                                RTCSignalingState::HaveRemoteOffer,
                                StateChangeOp::SetRemote,
                                sd.sdp_type,
                            );
                            if next_state.is_ok() {
                                let mut pending_remote_description =
                                    self.internal.pending_remote_description.lock().await;
                                *pending_remote_description = Some(sd.clone());
                            }
                            next_state
                        }
                        // have-local-offer->SetRemote(answer)->stable
                        // have-remote-pranswer->SetRemote(answer)->stable
                        RTCSdpType::Answer => {
                            let next_state = check_next_signaling_state(
                                cur,
                                RTCSignalingState::Stable,
                                StateChangeOp::SetRemote,
                                sd.sdp_type,
                            );
                            if next_state.is_ok() {
                                let pending_local_description = {
                                    let mut pending_local_description =
                                        self.internal.pending_local_description.lock().await;
                                    pending_local_description.take()
                                };

                                let _pending_remote_description = {
                                    let mut pending_remote_description =
                                        self.internal.pending_remote_description.lock().await;
                                    pending_remote_description.take()
                                };

                                {
                                    let mut current_remote_description =
                                        self.internal.current_remote_description.lock().await;
                                    *current_remote_description = Some(sd.clone());
                                }
                                {
                                    let mut current_local_description =
                                        self.internal.current_local_description.lock().await;
                                    *current_local_description = pending_local_description;
                                }
                            }
                            next_state
                        }
                        RTCSdpType::Rollback => {
                            let next_state = check_next_signaling_state(
                                cur,
                                RTCSignalingState::Stable,
                                StateChangeOp::SetRemote,
                                sd.sdp_type,
                            );
                            if next_state.is_ok() {
                                let mut pending_remote_description =
                                    self.internal.pending_remote_description.lock().await;
                                *pending_remote_description = None;
                            }
                            next_state
                        }
                        // have-local-offer->SetRemote(pranswer)->have-remote-pranswer
                        RTCSdpType::Pranswer => {
                            let next_state = check_next_signaling_state(
                                cur,
                                RTCSignalingState::HaveRemotePranswer,
                                StateChangeOp::SetRemote,
                                sd.sdp_type,
                            );
                            if next_state.is_ok() {
                                let mut pending_remote_description =
                                    self.internal.pending_remote_description.lock().await;
                                *pending_remote_description = Some(sd.clone());
                            }
                            next_state
                        }
                        _ => Err(Error::ErrPeerConnStateChangeInvalid),
                    }
                } //_ => Err(Error::ErrPeerConnStateChangeUnhandled.into()),
            }
        };

        match next_state {
            Ok(next_state) => {
                self.internal
                    .signaling_state
                    .store(next_state as u8, Ordering::SeqCst);
                if self.signaling_state() == RTCSignalingState::Stable {
                    self.internal
                        .is_negotiation_needed
                        .store(false, Ordering::SeqCst);
                    RTCPeerConnection::do_negotiation_needed(NegotiationNeededParams {
                        on_negotiation_needed_handler: Arc::clone(
                            &self.internal.on_negotiation_needed_handler,
                        ),
                        is_closed: Arc::clone(&self.internal.is_closed),
                        ops: Arc::clone(&self.internal.ops),
                        negotiation_needed_state: Arc::clone(
                            &self.internal.negotiation_needed_state,
                        ),
                        is_negotiation_needed: Arc::clone(&self.internal.is_negotiation_needed),
                        signaling_state: Arc::clone(&self.internal.signaling_state),
                        check_negotiation_needed_params: CheckNegotiationNeededParams {
                            sctp_transport: Arc::clone(&self.internal.sctp_transport),
                            current_local_description: self
                                .internal
                                .current_local_description
                                .clone(),
                        },
                    })
                    .await;
                }
                self.do_signaling_state_change(next_state).await;
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    /// set_local_description sets the SessionDescription of the local peer
    pub async fn set_local_description(&self, mut desc: RTCSessionDescription) -> Result<()> {
        if self.internal.is_closed.load(Ordering::SeqCst) {
            return Err(Error::ErrConnectionClosed);
        }

        // JSEP 5.4
        if desc.sdp.is_empty() {
            match desc.sdp_type {
                RTCSdpType::Answer | RTCSdpType::Pranswer => {
                    let last_answer = self.internal.last_answer.lock().await;
                    desc.sdp = last_answer.clone();
                }
                RTCSdpType::Offer => {
                    let last_offer = self.internal.last_offer.lock().await;
                    desc.sdp = last_offer.clone();
                }
                _ => return Err(Error::ErrPeerConnSDPTypeInvalidValueSetLocalDescription),
            }
        }

        desc.parsed = Some(desc.unmarshal()?);
        self.set_description(&desc, StateChangeOp::SetLocal).await?;

        let we_answer = desc.sdp_type == RTCSdpType::Answer;
        let remote_description = self.remote_description().await;
        if we_answer {
            if let Some(remote_desc) = remote_description {

                let pci = Arc::clone(&self.internal);
                let remote_desc = Arc::new(remote_desc);
                self.internal
                    .ops
                    .enqueue(Operation(Box::new(move || {
                        let pc = Arc::clone(&pci);
                        let rd = Arc::clone(&remote_desc);
                        Box::pin(async move {
                            let _ = pc
                                .maybe_start_sctp(rd)
                                .await;
                            false
                        })
                    })))
                    .await?;
            }
        }

        if self.internal.ice_gatherer.state() == RTCIceGathererState::New {
            self.internal.ice_gatherer.gather().await
        } else {
            Ok(())
        }
    }

    /// local_description returns PendingLocalDescription if it is not null and
    /// otherwise it returns CurrentLocalDescription. This property is used to
    /// determine if set_local_description has already been called.
    /// <https://www.w3.org/TR/webrtc/#dom-rtcpeerconnection-localdescription>
    pub async fn local_description(&self) -> Option<RTCSessionDescription> {
        if let Some(pending_local_description) = self.pending_local_description().await {
            return Some(pending_local_description);
        }
        self.current_local_description().await
    }

    /// set_remote_description sets the SessionDescription of the remote peer
    pub async fn set_remote_description(&self, mut desc: RTCSessionDescription) -> Result<()> {
        if self.internal.is_closed.load(Ordering::SeqCst) {
            return Err(Error::ErrConnectionClosed);
        }

        desc.parsed = Some(desc.unmarshal()?);
        self.set_description(&desc, StateChangeOp::SetRemote)
            .await?;

        if let Some(parsed) = &desc.parsed {

            let we_offer = desc.sdp_type == RTCSdpType::Answer;

            let (remote_ufrag, remote_pwd, candidates) = extract_ice_details(parsed).await?;

            for candidate in candidates {
                self.internal
                    .ice_transport
                    .add_remote_candidate(Some(candidate))
                    .await?;
            }

            let mut remote_is_lite = false;
            for a in &parsed.attributes {
                if a.key.trim() == ATTR_KEY_ICELITE {
                    remote_is_lite = true;
                    break;
                }
            }

            let (fingerprint, fingerprint_hash) = extract_fingerprint(parsed)?;

            // If one of the agents is lite and the other one is not, the lite agent must be the controlling agent.
            // If both or neither agents are lite the offering agent is controlling.
            // RFC 8445 S6.1.1
            let ice_role = if (we_offer
                && remote_is_lite == self.internal.setting_engine.candidates.ice_lite)
                || (remote_is_lite && !self.internal.setting_engine.candidates.ice_lite)
            {
                RTCIceRole::Controlling
            } else {
                RTCIceRole::Controlled
            };

            let pci = Arc::clone(&self.internal);
            let dtls_role = DTLSRole::from(parsed);
            let remote_desc = Arc::new(desc);
            self.internal
                .ops
                .enqueue(Operation(Box::new(move || {
                    let pc = Arc::clone(&pci);
                    let rd = Arc::clone(&remote_desc);
                    let ru = remote_ufrag.clone();
                    let rp = remote_pwd.clone();
                    let fp = fingerprint.clone();
                    let fp_hash = fingerprint_hash.clone();
                    Box::pin(async move {
                        log::trace!(
                            "start_transports: ice_role={}, dtls_role={}",
                            ice_role,
                            dtls_role,
                        );
                        pc.start_transports(ice_role, dtls_role, ru, rp, fp, fp_hash)
                            .await;

                        if we_offer {
                            let _ = pc.maybe_start_sctp(rd).await;
                        }
                        false
                    })
                })))
                .await?;
        }

        Ok(())
    }

    /// remote_description returns pending_remote_description if it is not null and
    /// otherwise it returns current_remote_description. This property is used to
    /// determine if setRemoteDescription has already been called.
    /// <https://www.w3.org/TR/webrtc/#dom-rtcpeerconnection-remotedescription>
    pub async fn remote_description(&self) -> Option<RTCSessionDescription> {
        self.internal.remote_description().await
    }

    /// add_ice_candidate accepts an ICE candidate string and adds it
    /// to the existing set of candidates.
    pub async fn add_ice_candidate(&self, candidate_str: String) -> Result<()> {
        if self.remote_description().await.is_none() {
            return Err(Error::ErrNoRemoteDescription);
        }

        let candidate_value = match candidate_str.strip_prefix("candidate:") {
            Some(s) => s,
            None => candidate_str.as_str(),
        };

        let ice_candidate = if !candidate_value.is_empty() {
            let candidate: Arc<dyn Candidate + Send + Sync> =
                Arc::new(unmarshal_candidate(candidate_value).await?);

            Some(RTCIceCandidate::from(&candidate))
        } else {
            None
        };

        self.internal
            .ice_transport
            .add_remote_candidate(ice_candidate)
            .await
    }

    /// create_data_channel creates a new DataChannel object with the given label
    /// and optional DataChannelInit used to configure properties of the
    /// underlying channel such as data reliability.
    pub async fn create_data_channel(
        &self,
    ) -> Result<Arc<RTCDataChannel>> {

        // https://w3c.github.io/webrtc-pc/#peer-to-peer-data-api (Step #2)
        if self.internal.is_closed.load(Ordering::SeqCst) {
            return Err(Error::ErrConnectionClosed);
        }

        let d = Arc::new(RTCDataChannel::new());

        {
            let mut data_channels = self.internal.sctp_transport.data_channels.lock().await;
            data_channels.push(Arc::clone(&d));
        }
        self.internal
            .sctp_transport
            .data_channels_requested
            .fetch_add(1, Ordering::SeqCst);

        // If SCTP already connected open all the channels
        if self.internal.sctp_transport.state() == RTCSctpTransportState::Connected {
            d.open(Arc::clone(&self.internal.sctp_transport)).await?;
        }

        RTCPeerConnection::do_negotiation_needed(NegotiationNeededParams {
            on_negotiation_needed_handler: Arc::clone(&self.internal.on_negotiation_needed_handler),
            is_closed: Arc::clone(&self.internal.is_closed),
            ops: Arc::clone(&self.internal.ops),
            negotiation_needed_state: Arc::clone(&self.internal.negotiation_needed_state),
            is_negotiation_needed: Arc::clone(&self.internal.is_negotiation_needed),
            signaling_state: Arc::clone(&self.internal.signaling_state),
            check_negotiation_needed_params: CheckNegotiationNeededParams {
                sctp_transport: Arc::clone(&self.internal.sctp_transport),
                current_local_description: self.internal.current_local_description.clone(),
            },
        })
        .await;

        Ok(d)
    }

    /// CurrentLocalDescription represents the local description that was
    /// successfully negotiated the last time the PeerConnection transitioned
    /// into the stable state plus any local candidates that have been generated
    /// by the ICEAgent since the offer or answer was created.
    pub async fn current_local_description(&self) -> Option<RTCSessionDescription> {
        let local_description = {
            let current_local_description = self.internal.current_local_description.lock().await;
            current_local_description.clone()
        };
        let ice_gather = Some(&self.internal.ice_gatherer);
        let ice_gathering_state = self.ice_gathering_state();

        populate_local_candidates(local_description.as_ref(), ice_gather, ice_gathering_state).await
    }

    /// PendingLocalDescription represents a local description that is in the
    /// process of being negotiated plus any local candidates that have been
    /// generated by the ICEAgent since the offer or answer was created. If the
    /// PeerConnection is in the stable state, the value is null.
    pub async fn pending_local_description(&self) -> Option<RTCSessionDescription> {
        let local_description = {
            let pending_local_description = self.internal.pending_local_description.lock().await;
            pending_local_description.clone()
        };
        let ice_gather = Some(&self.internal.ice_gatherer);
        let ice_gathering_state = self.ice_gathering_state();

        populate_local_candidates(local_description.as_ref(), ice_gather, ice_gathering_state).await
    }

    /// signaling_state attribute returns the signaling state of the
    /// PeerConnection instance.
    pub fn signaling_state(&self) -> RTCSignalingState {
        self.internal.signaling_state.load(Ordering::SeqCst).into()
    }

    /// icegathering_state attribute returns the ICE gathering state of the
    /// PeerConnection instance.
    pub fn ice_gathering_state(&self) -> RTCIceGatheringState {
        self.internal.ice_gathering_state()
    }
}
