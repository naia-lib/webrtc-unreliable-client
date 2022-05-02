
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
