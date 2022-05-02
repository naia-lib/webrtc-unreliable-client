
use crate::webrtc::error::{Error, Result};
use crate::webrtc::rtp_transceiver::rtp_codec::*;
use crate::webrtc::rtp_transceiver::rtp_receiver::RTCRtpReceiver;
use crate::webrtc::rtp_transceiver::rtp_sender::RTCRtpSender;
use crate::webrtc::rtp_transceiver::rtp_transceiver_direction::RTCRtpTransceiverDirection;

use interceptor::{
    stream_info::{RTPHeaderExtension, StreamInfo},
    Attributes,
};

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

pub(crate) mod fmtp;
pub mod rtp_codec;
pub mod rtp_receiver;
pub mod rtp_sender;
pub mod rtp_transceiver_direction;
pub(crate) mod srtp_writer_future;

/// SSRC represents a synchronization source
/// A synchronization source is a randomly chosen
/// value meant to be globally unique within a particular
/// RTP session. Used to identify a single stream of media.
/// <https://tools.ietf.org/html/rfc3550#section-3>
#[allow(clippy::upper_case_acronyms)]
pub type SSRC = u32;

/// PayloadType identifies the format of the RTP payload and determines
/// its interpretation by the application. Each codec in a RTP Session
/// will have a different PayloadType
/// <https://tools.ietf.org/html/rfc3550#section-3>
pub type PayloadType = u8;

/// rtcpfeedback signals the connection to use additional RTCP packet types.
/// <https://draft.ortc.org/#dom-rtcrtcpfeedback>
#[derive(Default, Debug, Clone, PartialEq)]
pub struct RTCPFeedback {
    /// Type is the type of feedback.
    /// see: <https://draft.ortc.org/#dom-rtcrtcpfeedback>
    /// valid: ack, ccm, nack, goog-remb, transport-cc
    pub typ: String,

    /// The parameter value depends on the type.
    /// For example, type="nack" parameter="pli" will send Picture Loss Indicator packets.
    pub parameter: String,
}

/// RTPCapabilities represents the capabilities of a transceiver
/// <https://w3c.github.io/webrtc-pc/#rtcrtpcapabilities>
#[derive(Default, Debug, Clone)]
pub struct RTCRtpCapabilities {
    pub codecs: Vec<RTCRtpCodecCapability>,
    pub header_extensions: Vec<RTCRtpHeaderExtensionCapability>,
}

/// RTPRtxParameters dictionary contains information relating to retransmission (RTX) settings.
/// <https://draft.ortc.org/#dom-rtcrtprtxparameters>
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RTCRtpRtxParameters {
    pub ssrc: SSRC,
}

/// RTPCodingParameters provides information relating to both encoding and decoding.
/// This is a subset of the RFC since Pion WebRTC doesn't implement encoding/decoding itself
/// <http://draft.ortc.org/#dom-rtcrtpcodingparameters>
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RTCRtpCodingParameters {
    pub rid: String,
    pub ssrc: SSRC,
    pub payload_type: PayloadType,
    pub rtx: RTCRtpRtxParameters,
}

/// RTPDecodingParameters provides information relating to both encoding and decoding.
/// This is a subset of the RFC since Pion WebRTC doesn't implement decoding itself
/// <http://draft.ortc.org/#dom-rtcrtpdecodingparameters>
pub type RTCRtpDecodingParameters = RTCRtpCodingParameters;

/// RTPEncodingParameters provides information relating to both encoding and decoding.
/// This is a subset of the RFC since Pion WebRTC doesn't implement encoding itself
/// <http://draft.ortc.org/#dom-rtcrtpencodingparameters>
pub type RTCRtpEncodingParameters = RTCRtpCodingParameters;

/// RTPReceiveParameters contains the RTP stack settings used by receivers
#[derive(Debug)]
pub struct RTCRtpReceiveParameters {
    pub encodings: Vec<RTCRtpDecodingParameters>,
}

/// RTPSendParameters contains the RTP stack settings used by receivers
#[derive(Debug)]
pub struct RTCRtpSendParameters {
    pub encodings: Vec<RTCRtpEncodingParameters>,
}

pub(crate) fn create_stream_info(
    id: String,
    ssrc: SSRC,
    payload_type: PayloadType,
    codec: RTCRtpCodecCapability,
    webrtc_header_extensions: &[RTCRtpHeaderExtensionParameters],
) -> StreamInfo {
    let mut header_extensions = vec![];
    for h in webrtc_header_extensions {
        header_extensions.push(RTPHeaderExtension {
            id: h.id,
            uri: h.uri.clone(),
        });
    }

    let mut feedbacks = vec![];
    for f in &codec.rtcp_feedback {
        feedbacks.push(interceptor::stream_info::RTCPFeedback {
            typ: f.typ.clone(),
            parameter: f.parameter.clone(),
        });
    }

    StreamInfo {
        id,
        attributes: Attributes::new(),
        ssrc,
        payload_type,
        rtp_header_extensions: header_extensions,
        mime_type: codec.mime_type,
        clock_rate: codec.clock_rate,
        channels: codec.channels,
        sdp_fmtp_line: codec.sdp_fmtp_line,
        rtcp_feedback: feedbacks,
    }
}

/// RTPTransceiver represents a combination of an RTPSender and an RTPReceiver that share a common mid.
pub struct RTCRtpTransceiver {
    mid: Mutex<String>,                           //atomic.Value
    sender: Mutex<Option<Arc<RTCRtpSender>>>,     //atomic.Value
    receiver: Mutex<Option<Arc<RTCRtpReceiver>>>, //atomic.Value
    direction: AtomicU8,                          //RTPTransceiverDirection, //atomic.Value

    codecs: Arc<Mutex<Vec<RTCRtpCodecParameters>>>, // User provided codecs via set_codec_preferences

    pub(crate) stopped: AtomicBool,
    pub(crate) kind: RTPCodecType,
}

impl RTCRtpTransceiver {
    pub(crate) async fn new(
        receiver: Option<Arc<RTCRtpReceiver>>,
        sender: Option<Arc<RTCRtpSender>>,
        direction: RTCRtpTransceiverDirection,
        kind: RTPCodecType,
        codecs: Vec<RTCRtpCodecParameters>,
    ) -> Arc<Self> {
        let t = Arc::new(RTCRtpTransceiver {
            mid: Mutex::new(String::new()),
            sender: Mutex::new(None),
            receiver: Mutex::new(None),
            direction: AtomicU8::new(direction as u8),
            codecs: Arc::new(Mutex::new(codecs)),
            stopped: AtomicBool::new(false),
            kind,
        });

        t.set_receiver(receiver).await;
        t.set_sender(sender).await;

        t
    }

    /// sender returns the RTPTransceiver's RTPSender if it has one
    pub async fn sender(&self) -> Option<Arc<RTCRtpSender>> {
        let sender = self.sender.lock().await;
        sender.clone()
    }

    pub async fn set_sender(self: &Arc<Self>, s: Option<Arc<RTCRtpSender>>) {
        if let Some(sender) = &s {
            sender.set_rtp_transceiver(Some(Arc::downgrade(self))).await;
        }

        if let Some(prev_sender) = self.sender().await {
            prev_sender.set_rtp_transceiver(None).await;
        }

        {
            let mut sender = self.sender.lock().await;
            *sender = s;
        }
    }

    pub(crate) async fn set_receiver(&self, r: Option<Arc<RTCRtpReceiver>>) {
        if let Some(receiver) = &r {
            receiver
                .set_transceiver_codecs(Some(Arc::clone(&self.codecs)))
                .await;
        }

        {
            let mut receiver = self.receiver.lock().await;
            if let Some(prev_receiver) = &*receiver {
                prev_receiver.set_transceiver_codecs(None).await;
            }

            *receiver = r;
        }
    }

    /// set_mid sets the RTPTransceiver's mid. If it was already set, will return an error.
    pub(crate) async fn set_mid(&self, mid: String) -> Result<()> {
        let mut m = self.mid.lock().await;
        if !m.is_empty() {
            return Err(Error::ErrRTPTransceiverCannotChangeMid);
        }
        *m = mid;

        Ok(())
    }

    /// mid gets the Transceiver's mid value. When not already set, this value will be set in CreateOffer or create_answer.
    pub async fn mid(&self) -> String {
        let mid = self.mid.lock().await;
        mid.clone()
    }

    /// direction returns the RTPTransceiver's current direction
    pub fn direction(&self) -> RTCRtpTransceiverDirection {
        self.direction.load(Ordering::SeqCst).into()
    }

    pub(crate) fn set_direction(&self, d: RTCRtpTransceiverDirection) {
        self.direction.store(d as u8, Ordering::SeqCst);
    }

    /// stop irreversibly stops the RTPTransceiver
    pub async fn stop(&self) -> Result<()> {
        if self.stopped.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.stopped.store(true, Ordering::SeqCst);

        {
            let s = self.sender.lock().await;
            if let Some(sender) = &*s {
                sender.stop().await?;
            }
        }
        {
            let r = self.receiver.lock().await;
            if let Some(receiver) = &*r {
                receiver.stop().await?;
            }
        }

        self.set_direction(RTCRtpTransceiverDirection::Inactive);

        Ok(())
    }
}

pub(crate) async fn find_by_mid(
    mid: &str,
    local_transceivers: &mut Vec<Arc<RTCRtpTransceiver>>,
) -> Option<Arc<RTCRtpTransceiver>> {
    for (i, t) in local_transceivers.iter().enumerate() {
        if t.mid().await == mid {
            return Some(local_transceivers.remove(i));
        }
    }

    None
}

/// Given a direction+type pluck a transceiver from the passed list
/// if no entry satisfies the requested type+direction return a inactive Transceiver
pub(crate) async fn satisfy_type_and_direction(
    remote_kind: RTPCodecType,
    remote_direction: RTCRtpTransceiverDirection,
    local_transceivers: &mut Vec<Arc<RTCRtpTransceiver>>,
) -> Option<Arc<RTCRtpTransceiver>> {
    // Get direction order from most preferred to least
    let get_preferred_directions = || -> Vec<RTCRtpTransceiverDirection> {
        match remote_direction {
            RTCRtpTransceiverDirection::Sendrecv => vec![
                RTCRtpTransceiverDirection::Recvonly,
                RTCRtpTransceiverDirection::Sendrecv,
            ],
            RTCRtpTransceiverDirection::Sendonly => vec![RTCRtpTransceiverDirection::Recvonly],
            RTCRtpTransceiverDirection::Recvonly => vec![
                RTCRtpTransceiverDirection::Sendonly,
                RTCRtpTransceiverDirection::Sendrecv,
            ],
            _ => vec![],
        }
    };

    for possible_direction in get_preferred_directions() {
        for (i, t) in local_transceivers.iter().enumerate() {
            if t.mid().await.is_empty()
                && t.kind == remote_kind
                && possible_direction == t.direction()
            {
                return Some(local_transceivers.remove(i));
            }
        }
    }

    None
}
