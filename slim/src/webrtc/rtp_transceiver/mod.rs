
use crate::webrtc::error::{Error, Result};
use crate::webrtc::rtp_transceiver::rtp_codec::*;
use crate::webrtc::rtp_transceiver::rtp_transceiver_direction::RTCRtpTransceiverDirection;

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};
use tokio::sync::Mutex;

pub(crate) mod fmtp;
pub mod rtp_codec;
pub mod rtp_transceiver_direction;

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

/// RTPTransceiver represents a combination of an RTPSender and an RTPReceiver that share a common mid.
pub struct RTCRtpTransceiver {
    // Removing the below value crashes compilation..
    #[allow(dead_code)]
    codecs: bool,
    #[allow(dead_code)]
    receiver: bool,

    mid: Mutex<String>,                           //atomic.Value
    sender: bool,     //atomic.Value
    direction: AtomicU8,                          //RTPTransceiverDirection, //atomic.Value
    pub(crate) stopped: bool,
    pub(crate) kind: bool,
}

impl RTCRtpTransceiver {

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
}