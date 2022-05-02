use super::*;
use crate::webrtc::error::{Error, Result};

use std::fmt;

/// RTPCodecType determines the type of a codec
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum RTPCodecType {
    Unspecified = 0,
}

impl Default for RTPCodecType {
    fn default() -> Self {
        RTPCodecType::Unspecified
    }
}

impl From<&str> for RTPCodecType {
    fn from(raw: &str) -> Self {
        match raw {
            _ => RTPCodecType::Unspecified,
        }
    }
}

impl From<u8> for RTPCodecType {
    fn from(v: u8) -> Self {
        match v {
            _ => RTPCodecType::Unspecified,
        }
    }
}

impl fmt::Display for RTPCodecType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match *self {
            RTPCodecType::Unspecified => crate::webrtc::UNSPECIFIED_STR,
        };
        write!(f, "{}", s)
    }
}

/// RTPCodecCapability provides information about codec capabilities.
/// <https://w3c.github.io/webrtc-pc/#dictionary-rtcrtpcodeccapability-members>
#[derive(Default, Debug, Clone, PartialEq)]
pub struct RTCRtpCodecCapability {
    pub mime_type: String,
    pub clock_rate: u32,
    pub channels: u16,
    pub sdp_fmtp_line: String,
    pub rtcp_feedback: Vec<RTCPFeedback>,
}

impl RTCRtpCodecCapability {
    pub(crate) fn payloader_for_codec(
        &self,
    ) -> Result<Box<dyn rtp::packetizer::Payloader + Send + Sync>> {
        Err(Error::ErrNoPayloaderForCodec)
    }
}

/// RTPHeaderExtensionCapability is used to define a RFC5285 RTP header extension supported by the codec.
/// <https://w3c.github.io/webrtc-pc/#dom-rtcrtpcapabilities-headerextensions>
#[derive(Default, Debug, Clone)]
pub struct RTCRtpHeaderExtensionCapability {
    pub uri: String,
}

/// RTPHeaderExtensionParameter represents a negotiated RFC5285 RTP header extension.
/// <https://w3c.github.io/webrtc-pc/#dictionary-rtcrtpheaderextensionparameters-members>
#[derive(Default, Debug, Clone, PartialEq)]
pub struct RTCRtpHeaderExtensionParameters {
    pub uri: String,
    pub id: isize,
}

/// RTPCodecParameters is a sequence containing the media codecs that an RtpSender
/// will choose from, as well as entries for RTX, RED and FEC mechanisms. This also
/// includes the PayloadType that has been negotiated
/// <https://w3c.github.io/webrtc-pc/#rtcrtpcodecparameters>
#[derive(Default, Debug, Clone, PartialEq)]
pub struct RTCRtpCodecParameters {
    pub capability: RTCRtpCodecCapability,
    pub payload_type: PayloadType,
    pub stats_id: String,
}

/// RTPParameters is a list of negotiated codecs and header extensions
/// <https://w3c.github.io/webrtc-pc/#dictionary-rtcrtpparameters-members>
#[derive(Default, Debug, Clone)]
pub struct RTCRtpParameters {
    pub header_extensions: Vec<RTCRtpHeaderExtensionParameters>,
    pub codecs: Vec<RTCRtpCodecParameters>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum CodecMatch {
    None = 0,
}

impl Default for CodecMatch {
    fn default() -> Self {
        CodecMatch::None
    }
}