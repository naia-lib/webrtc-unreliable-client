use super::track_local_static_rtp::TrackLocalStaticRTP;
use super::*;

use crate::webrtc::track::RTP_OUTBOUND_MTU;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
struct TrackLocalStaticSampleInternal {
    packetizer: Option<Box<dyn rtp::packetizer::Packetizer + Send + Sync>>,
    sequencer: Option<Box<dyn rtp::sequence::Sequencer + Send + Sync>>,
    clock_rate: f64,
}

/// TrackLocalStaticSample is a TrackLocal that has a pre-set codec and accepts Samples.
/// If you wish to send a RTP Packet use TrackLocalStaticRTP
#[derive(Debug)]
pub struct TrackLocalStaticSample {
    rtp_track: TrackLocalStaticRTP,
    internal: Mutex<TrackLocalStaticSampleInternal>,
}

impl TrackLocalStaticSample {
    /// returns a TrackLocalStaticSample
    pub fn new(codec: RTCRtpCodecCapability, id: String, stream_id: String) -> Self {
        let rtp_track = TrackLocalStaticRTP::new(codec, id, stream_id);

        TrackLocalStaticSample {
            rtp_track,
            internal: Mutex::new(TrackLocalStaticSampleInternal {
                packetizer: None,
                sequencer: None,
                clock_rate: 0.0f64,
            }),
        }
    }
}

#[async_trait]
impl TrackLocal for TrackLocalStaticSample {
    /// Bind is called by the PeerConnection after negotiation is complete
    /// This asserts that the code requested is supported by the remote peer.
    /// If so it setups all the state (SSRC and PayloadType) to have a call
    async fn bind(&self, t: &TrackLocalContext) -> Result<RTCRtpCodecParameters> {
        let codec = self.rtp_track.bind(t).await?;

        let mut internal = self.internal.lock().await;

        // We only need one packetizer
        if internal.packetizer.is_some() {
            return Ok(codec);
        }

        let payloader = codec.capability.payloader_for_codec()?;
        let sequencer: Box<dyn rtp::sequence::Sequencer + Send + Sync> =
            Box::new(rtp::sequence::new_random_sequencer());
        internal.packetizer = Some(Box::new(rtp::packetizer::new_packetizer(
            RTP_OUTBOUND_MTU,
            0, // Value is handled when writing
            0, // Value is handled when writing
            payloader,
            sequencer.clone(),
            codec.capability.clock_rate,
        )));
        internal.sequencer = Some(sequencer);
        internal.clock_rate = codec.capability.clock_rate as f64;

        Ok(codec)
    }

    /// unbind implements the teardown logic when the track is no longer needed. This happens
    /// because a track has been stopped.
    async fn unbind(&self, t: &TrackLocalContext) -> Result<()> {
        self.rtp_track.unbind(t).await
    }

    /// id is the unique identifier for this Track. This should be unique for the
    /// stream, but doesn't have to globally unique. A common example would be 'audio' or 'video'
    /// and StreamID would be 'desktop' or 'webcam'
    fn id(&self) -> &str {
        self.rtp_track.id()
    }

    /// stream_id is the group this track belongs too. This must be unique
    fn stream_id(&self) -> &str {
        self.rtp_track.stream_id()
    }

    /// kind controls if this TrackLocal is audio or video
    fn kind(&self) -> RTPCodecType {
        self.rtp_track.kind()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
