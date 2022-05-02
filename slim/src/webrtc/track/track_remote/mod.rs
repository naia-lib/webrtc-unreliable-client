use crate::webrtc::error::{Error, Result};
use crate::webrtc::rtp_transceiver::rtp_codec::{RTCRtpCodecParameters, RTCRtpParameters, RTPCodecType};
use crate::webrtc::rtp_transceiver::{PayloadType, SSRC};

use crate::webrtc::rtp_transceiver::rtp_receiver::RTPReceiverInternal;

use bytes::Bytes;
use interceptor::Attributes;
use std::sync::atomic::{AtomicU32, AtomicU8, AtomicUsize, Ordering};
use std::sync::Weak;
use tokio::sync::Mutex;
use util::Unmarshal;

lazy_static! {
    static ref TRACK_REMOTE_UNIQUE_ID: AtomicUsize = AtomicUsize::new(0);
}

#[derive(Default)]
struct TrackRemoteInternal {
    peeked: Option<Bytes>,
    peeked_attributes: Option<Attributes>,
}

/// TrackRemote represents a single inbound source of media
pub struct TrackRemote {
    tid: usize,

    id: Mutex<String>,
    stream_id: Mutex<String>,

    receive_mtu: usize,
    payload_type: AtomicU8, //PayloadType,
    kind: AtomicU8,         //RTPCodecType,
    ssrc: AtomicU32,        //SSRC,
    codec: Mutex<RTCRtpCodecParameters>,
    pub(crate) params: Mutex<RTCRtpParameters>,
    rid: String,

    receiver: Option<Weak<RTPReceiverInternal>>,
    internal: Mutex<TrackRemoteInternal>,
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
        receive_mtu: usize,
        kind: RTPCodecType,
        ssrc: SSRC,
        rid: String,
        receiver: Weak<RTPReceiverInternal>,
    ) -> Self {
        TrackRemote {
            tid: TRACK_REMOTE_UNIQUE_ID.fetch_add(1, Ordering::SeqCst),
            id: Default::default(),
            stream_id: Default::default(),
            receive_mtu,
            payload_type: Default::default(),
            kind: AtomicU8::new(kind as u8),
            ssrc: AtomicU32::new(ssrc),
            codec: Default::default(),
            params: Default::default(),
            rid,
            receiver: Some(receiver),

            internal: Default::default(),
        }
    }

    pub fn tid(&self) -> usize {
        self.tid
    }

    /// id is the unique identifier for this Track. This should be unique for the
    /// stream, but doesn't have to globally unique. A common example would be 'audio' or 'video'
    /// and StreamID would be 'desktop' or 'webcam'
    pub async fn id(&self) -> String {
        let id = self.id.lock().await;
        id.clone()
    }

    pub async fn set_id(&self, s: String) {
        let mut id = self.id.lock().await;
        *id = s;
    }

    /// stream_id is the group this track belongs too. This must be unique
    pub async fn stream_id(&self) -> String {
        let stream_id = self.stream_id.lock().await;
        stream_id.clone()
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

    /// payload_type gets the PayloadType of the track
    pub fn payload_type(&self) -> PayloadType {
        self.payload_type.load(Ordering::SeqCst)
    }

    pub fn set_payload_type(&self, payload_type: PayloadType) {
        self.payload_type.store(payload_type, Ordering::SeqCst);
    }

    /// kind gets the Kind of the track
    pub fn kind(&self) -> RTPCodecType {
        self.kind.load(Ordering::SeqCst).into()
    }

    pub fn set_kind(&self, kind: RTPCodecType) {
        self.kind.store(kind as u8, Ordering::SeqCst);
    }

    /// ssrc gets the SSRC of the track
    pub fn ssrc(&self) -> SSRC {
        self.ssrc.load(Ordering::SeqCst)
    }

    pub fn set_ssrc(&self, ssrc: SSRC) {
        self.ssrc.store(ssrc, Ordering::SeqCst);
    }

    /// msid gets the Msid of the track
    pub async fn msid(&self) -> String {
        self.stream_id().await + " " + self.id().await.as_str()
    }

    /// codec gets the Codec of the track
    pub async fn codec(&self) -> RTCRtpCodecParameters {
        let codec = self.codec.lock().await;
        codec.clone()
    }

    pub async fn set_codec(&self, codec: RTCRtpCodecParameters) {
        let mut c = self.codec.lock().await;
        *c = codec;
    }

    pub async fn params(&self) -> RTCRtpParameters {
        let p = self.params.lock().await;
        p.clone()
    }

    pub async fn set_params(&self, params: RTCRtpParameters) {
        let mut p = self.params.lock().await;
        *p = params;
    }

    /// Read reads data from the track.
    pub async fn read(&self, b: &mut [u8]) -> Result<(usize, Attributes)> {
        let (peeked, peeked_attributes) = {
            let mut internal = self.internal.lock().await;
            (internal.peeked.take(), internal.peeked_attributes.take())
        };

        if let (Some(data), Some(attributes)) = (peeked, peeked_attributes) {
            // someone else may have stolen our packet when we
            // released the lock.  Deal with it.
            let n = std::cmp::min(b.len(), data.len());
            b[..n].copy_from_slice(&data[..n]);
            Ok((n, attributes))
        } else {
            let (n, attributes) = {
                if let Some(receiver) = &self.receiver {
                    if let Some(receiver) = receiver.upgrade() {
                        receiver.read_rtp(b, self.tid).await?
                    } else {
                        return Err(Error::ErrRTPReceiverNil);
                    }
                } else {
                    return Err(Error::ErrRTPReceiverNil);
                }
            };
            Ok((n, attributes))
        }
    }

    /// read_rtp is a convenience method that wraps Read and unmarshals for you.
    pub async fn read_rtp(&self) -> Result<(rtp::packet::Packet, Attributes)> {
        let mut b = vec![0u8; self.receive_mtu];
        let (n, attributes) = self.read(&mut b).await?;

        let mut buf = &b[..n];
        let r = rtp::packet::Packet::unmarshal(&mut buf)?;
        Ok((r, attributes))
    }
}
