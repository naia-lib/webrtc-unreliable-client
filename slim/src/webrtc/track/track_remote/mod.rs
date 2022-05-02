use crate::webrtc::rtp_transceiver::rtp_codec::{RTCRtpCodecParameters, RTCRtpParameters, RTPCodecType};
use crate::webrtc::rtp_transceiver::SSRC;

use std::sync::atomic::{AtomicU32, AtomicU8, AtomicUsize, Ordering};
use tokio::sync::Mutex;

lazy_static! {
    static ref TRACK_REMOTE_UNIQUE_ID: AtomicUsize = AtomicUsize::new(0);
}

#[derive(Default)]
struct TrackRemoteInternal;

/// TrackRemote represents a single inbound source of media
pub struct TrackRemote {

    id: Mutex<String>,
    stream_id: Mutex<String>,

    payload_type: AtomicU8, //PayloadType,
    kind: AtomicU8,         //RTPCodecType,
    ssrc: AtomicU32,        //SSRC,
    codec: Mutex<RTCRtpCodecParameters>,
    pub(crate) params: Mutex<RTCRtpParameters>,
    rid: String,
}

impl std::fmt::Debug for TrackRemote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrackRemote")
            .field("id", &self.id)
            .field("stream_id", &self.stream_id)
            .field("payload_type", &self.payload_type)
            .field("kind", &self.kind)
            .field("ssrc", &self.ssrc)
            .field("codec", &self.codec)
            .field("params", &self.params)
            .field("rid", &self.rid)
            .finish()
    }
}

impl TrackRemote {
    pub(crate) fn new(
        kind: RTPCodecType,
        ssrc: SSRC,
        rid: String,
    ) -> Self {
        TrackRemote {
            id: Default::default(),
            stream_id: Default::default(),
            payload_type: Default::default(),
            kind: AtomicU8::new(kind as u8),
            ssrc: AtomicU32::new(ssrc),
            codec: Default::default(),
            params: Default::default(),
            rid,
        }
    }

    pub async fn set_id(&self, s: String) {
        let mut id = self.id.lock().await;
        *id = s;
    }

    pub async fn set_stream_id(&self, s: String) {
        let mut stream_id = self.stream_id.lock().await;
        *stream_id = s;
    }

    /// rid gets the RTP Stream ID of this Track
    /// With Simulcast you will have multiple tracks with the same ID, but different RID values.
    /// In many cases a TrackRemote will not have an RID, so it is important to assert it is non-zero
    pub fn rid(&self) -> &str {
        self.rid.as_str()
    }

    /// ssrc gets the SSRC of the track
    pub fn ssrc(&self) -> SSRC {
        self.ssrc.load(Ordering::SeqCst)
    }
}
