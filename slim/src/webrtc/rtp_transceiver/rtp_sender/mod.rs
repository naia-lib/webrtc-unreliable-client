
use crate::webrtc::error::{Error, Result};
use crate::webrtc::rtp_transceiver::srtp_writer_future::SrtpWriterFuture;
use crate::webrtc::rtp_transceiver::RTCRtpTransceiver;
use crate::webrtc::track::track_local::{
    TrackLocal, TrackLocalContext,
};

use interceptor::stream_info::StreamInfo;
use interceptor::Interceptor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use tokio::sync::{mpsc, Mutex, Notify};

pub(crate) struct RTPSenderInternal {
    pub(crate) stop_called_rx: Arc<Notify>,
    pub(crate) stop_called_signal: Arc<AtomicBool>,
}

/// RTPSender allows an application to control how a given Track is encoded and transmitted to a remote peer
pub struct RTCRtpSender {
    pub(crate) track: Mutex<Option<Arc<dyn TrackLocal + Send + Sync>>>,

    pub(crate) srtp_stream: Arc<SrtpWriterFuture>,
    pub(crate) stream_info: Mutex<StreamInfo>,

    pub(crate) context: Mutex<TrackLocalContext>,

    /// a transceiver sender since we can just check the
    /// transceiver negotiation status
    pub(crate) negotiated: AtomicBool,

    pub(crate) interceptor: Arc<dyn Interceptor + Send + Sync>,

    pub(crate) id: String,

    rtp_transceiver: Mutex<Option<Weak<RTCRtpTransceiver>>>,

    send_called_tx: Mutex<Option<mpsc::Sender<()>>>,
    stop_called_tx: Arc<Notify>,
    stop_called_signal: Arc<AtomicBool>,
}

impl RTCRtpSender {

    pub(crate) fn set_negotiated(&self) {
        self.negotiated.store(true, Ordering::SeqCst);
    }

    pub(crate) async fn set_rtp_transceiver(
        &self,
        rtp_transceiver: Option<Weak<RTCRtpTransceiver>>,
    ) {
        let mut tr = self.rtp_transceiver.lock().await;
        *tr = rtp_transceiver;
    }

    /// track returns the RTCRtpTransceiver track, or nil
    pub async fn track(&self) -> Option<Arc<dyn TrackLocal + Send + Sync>> {
        let track = self.track.lock().await;
        track.clone()
    }

    /// replace_track replaces the track currently being used as the sender's source with a new TrackLocal.
    /// The new track must be of the same media kind (audio, video, etc) and switching the track should not
    /// require negotiation.
    pub async fn replace_track(
        &self,
        track: Option<Arc<dyn TrackLocal + Send + Sync>>,
    ) -> Result<()> {
        if let Some(t) = &track {
            let tr = self.rtp_transceiver.lock().await;
            if let Some(r) = &*tr {
                if let Some(r) = r.upgrade() {
                    if r.kind != t.kind() {
                        return Err(Error::ErrRTPSenderNewTrackHasIncorrectKind);
                    }
                } else {
                    //TODO: what about None arc?
                }
            } else {
                //TODO: what about None tr?
            }
        }

        if self.has_sent().await {
            let t = {
                let t = self.track.lock().await;
                t.clone()
            };
            if let Some(t) = t {
                let context = self.context.lock().await;
                t.unbind(&*context).await?;
            }
        }

        if !self.has_sent().await || track.is_none() {
            let mut t = self.track.lock().await;
            *t = track;
            return Ok(());
        }

        Ok(())
    }

    /// stop irreversibly stops the RTPSender
    pub async fn stop(&self) -> Result<()> {
        if self.stop_called_signal.load(Ordering::SeqCst) {
            return Ok(());
        }
        self.stop_called_signal.store(true, Ordering::SeqCst);
        self.stop_called_tx.notify_waiters();

        if !self.has_sent().await {
            return Ok(());
        }

        self.replace_track(None).await?;

        {
            let stream_info = self.stream_info.lock().await;
            self.interceptor.unbind_local_stream(&*stream_info).await;
        }

        self.srtp_stream.close().await
    }

    /// has_sent tells if data has been ever sent for this instance
    pub(crate) async fn has_sent(&self) -> bool {
        let send_called_tx = self.send_called_tx.lock().await;
        send_called_tx.is_none()
    }
}
