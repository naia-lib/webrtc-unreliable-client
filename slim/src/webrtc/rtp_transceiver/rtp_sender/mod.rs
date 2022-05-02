
use crate::webrtc::rtp_transceiver::srtp_writer_future::SrtpWriterFuture;
use crate::webrtc::rtp_transceiver::RTCRtpTransceiver;
use crate::webrtc::track::track_local::{
    TrackLocal, TrackLocalContext,
};

use interceptor::stream_info::StreamInfo;
use interceptor::Interceptor;
use std::sync::atomic::AtomicBool;
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

    pub(crate) interceptor: Arc<dyn Interceptor + Send + Sync>,

    rtp_transceiver: Mutex<Option<Weak<RTCRtpTransceiver>>>,

    send_called_tx: Mutex<Option<mpsc::Sender<()>>>,
    stop_called_tx: Arc<Notify>,
    stop_called_signal: Arc<AtomicBool>,
}
