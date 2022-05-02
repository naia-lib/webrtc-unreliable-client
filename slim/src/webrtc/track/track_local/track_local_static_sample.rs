use super::track_local_static_rtp::TrackLocalStaticRTP;
use super::*;

#[derive(Debug, Clone)]
struct TrackLocalStaticSampleInternal;

/// TrackLocalStaticSample is a TrackLocal that has a pre-set codec and accepts Samples.
/// If you wish to send a RTP Packet use TrackLocalStaticRTP
#[derive(Debug)]
pub struct TrackLocalStaticSample {
    rtp_track: TrackLocalStaticRTP,
}

#[async_trait]
impl TrackLocal for TrackLocalStaticSample {

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
