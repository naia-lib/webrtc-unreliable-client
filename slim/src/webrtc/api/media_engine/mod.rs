#[cfg(test)]
mod media_engine_test;

use crate::webrtc::error::Result;
use crate::webrtc::rtp_transceiver::rtp_codec::{
    RTCRtpCodecParameters,
    RTCRtpParameters,
    RTPCodecType,
};
use crate::webrtc::rtp_transceiver::rtp_transceiver_direction::RTCRtpTransceiverDirection;

use sdp::description::session::SessionDescription;

#[derive(Default, Clone)]
pub(crate) struct MediaEngineHeaderExtension;

/// A MediaEngine defines the codecs supported by a PeerConnection, and the
/// configuration of those codecs. A MediaEngine must not be shared between
/// PeerConnections.
#[derive(Default)]
pub struct MediaEngine {
    // If we have attempted to negotiate a codec type yet.
    pub(crate) video_codecs: Vec<RTCRtpCodecParameters>,
    pub(crate) audio_codecs: Vec<RTCRtpCodecParameters>,

    pub(crate) header_extensions: Vec<MediaEngineHeaderExtension>,
}

impl MediaEngine {

    /// clone_to copies any user modifiable state of the MediaEngine
    /// all internal state is reset
    pub(crate) fn clone_to(&self) -> Self {
        MediaEngine {
            video_codecs: self.video_codecs.clone(),
            audio_codecs: self.audio_codecs.clone(),
            header_extensions: self.header_extensions.clone(),
            ..Default::default()
        }
    }

    /// Update the MediaEngine from a remote description
    pub(crate) async fn update_from_remote_description(
        &self,
        _desc: &SessionDescription,
    ) -> Result<()> {
        // for media in &desc.media_descriptions {
        //     let typ = if !self.negotiated_audio.load(Ordering::SeqCst)
        //         && media.media_name.media.to_lowercase() == "audio"
        //     {
        //         self.negotiated_audio.store(true, Ordering::SeqCst);
        //         RTPCodecType::Audio
        //     } else if !self.negotiated_video.load(Ordering::SeqCst)
        //         && media.media_name.media.to_lowercase() == "video"
        //     {
        //         self.negotiated_video.store(true, Ordering::SeqCst);
        //         RTPCodecType::Video
        //     } else {
        //         continue;
        //     };
        //
        //     let codecs = codecs_from_media_description(media)?;
        //
        //     let mut exact_matches = vec![]; //make([]RTPCodecParameters, 0, len(codecs))
        //     let mut partial_matches = vec![]; //make([]RTPCodecParameters, 0, len(codecs))
        //
        //     for codec in codecs {
        //         let match_type =
        //             self.match_remote_codec(&codec, typ, &exact_matches, &partial_matches)?;
        //
        //         if match_type == CodecMatch::Exact {
        //             exact_matches.push(codec);
        //         } else if match_type == CodecMatch::Partial {
        //             partial_matches.push(codec);
        //         }
        //     }
        //
        //     // use exact matches when they exist, otherwise fall back to partial
        //     if !exact_matches.is_empty() {
        //         self.push_codecs(exact_matches, typ).await;
        //     } else if !partial_matches.is_empty() {
        //         self.push_codecs(partial_matches, typ).await;
        //     } else {
        //         // no match, not negotiated
        //         continue;
        //     }
        //
        //     let extensions = rtp_extensions_from_media_description(media)?;
        //
        //     for (extension, id) in extensions {
        //         self.update_header_extension(id, &extension, typ).await?;
        //     }
        // }

        Ok(())
    }

    pub(crate) async fn get_codecs_by_kind(&self, _typ: RTPCodecType) -> Vec<RTCRtpCodecParameters> {
        // if typ == RTPCodecType::Video {
        //     if self.negotiated_video.load(Ordering::SeqCst) {
        //         let negotiated_video_codecs = self.negotiated_video_codecs.lock().await;
        //         negotiated_video_codecs.clone()
        //     } else {
        //         self.video_codecs.clone()
        //     }
        // } else if typ == RTPCodecType::Audio {
        //     if self.negotiated_audio.load(Ordering::SeqCst) {
        //         let negotiated_audio_codecs = self.negotiated_audio_codecs.lock().await;
        //         negotiated_audio_codecs.clone()
        //     } else {
        //         self.audio_codecs.clone()
        //     }
        // } else {
            vec![]
        // }
    }

    pub(crate) async fn get_rtp_parameters_by_kind(
        &self,
        _typ: RTPCodecType,
        _directions: &[RTCRtpTransceiverDirection],
    ) -> RTCRtpParameters {

        RTCRtpParameters {
            header_extensions: vec![],
            codecs: vec![],
        }
    }
}
