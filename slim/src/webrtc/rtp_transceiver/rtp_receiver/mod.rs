
use crate::webrtc::dtls_transport::RTCDtlsTransport;
use crate::webrtc::error::{flatten_errs, Result};
use crate::webrtc::rtp_transceiver::rtp_codec::{
    RTCRtpCodecParameters,
    RTPCodecType,
};
use crate::webrtc::track::TrackStreams;

use interceptor::Interceptor;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, Notify, RwLock};

pub struct RTPReceiverInternal {
    // removing these seems to cause a compiler panic
    #[allow(dead_code)]
    kind: bool,
    #[allow(dead_code)]
    transport: bool,
    #[allow(dead_code)]
    media_engine: bool,
    #[allow(dead_code)]
    received_rx: bool,
    #[allow(dead_code)]
    closed_rx: bool,

    tracks: RwLock<Vec<TrackStreams>>,
    transceiver_codecs: Mutex<Option<Arc<Mutex<Vec<RTCRtpCodecParameters>>>>>,
    interceptor: Arc<dyn Interceptor + Send + Sync>,
}

/// RTPReceiver allows an application to inspect the receipt of a TrackRemote
pub struct RTCRtpReceiver {
    kind: RTPCodecType,
    transport: Arc<RTCDtlsTransport>,
    closed_tx: Arc<Notify>,
    received_tx: Mutex<Option<mpsc::Sender<()>>>,

    pub internal: Arc<RTPReceiverInternal>,
}

impl std::fmt::Debug for RTCRtpReceiver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RTCRtpReceiver")
            .field("kind", &self.kind)
            .finish()
    }
}

impl RTCRtpReceiver {
    pub fn new(
        kind: RTPCodecType,
        transport: Arc<RTCDtlsTransport>,
        interceptor: Arc<dyn Interceptor + Send + Sync>,
    ) -> Self {
        let closed_tx = Arc::new(Notify::new());
        let (received_tx, _) = mpsc::channel(1);

        RTCRtpReceiver {
            kind,
            transport: Arc::clone(&transport),
            closed_tx,
            received_tx: Mutex::new(Some(received_tx)),

            internal: Arc::new(RTPReceiverInternal {
                kind: true,
                transport: true,
                media_engine: true,
                received_rx: true,

                tracks: RwLock::new(vec![]),
                interceptor,
                closed_rx: true,
                transceiver_codecs: Mutex::new(None),
            }),
        }
    }

    pub(crate) async fn set_transceiver_codecs(
        &self,
        codecs: Option<Arc<Mutex<Vec<RTCRtpCodecParameters>>>>,
    ) {
        let mut transceiver_codecs = self.internal.transceiver_codecs.lock().await;
        *transceiver_codecs = codecs;
    }

    /// Stop irreversibly stops the RTPReceiver
    pub async fn stop(&self) -> Result<()> {
        self.closed_tx.notify_waiters();

        let received_tx_is_none = {
            let received_tx = self.received_tx.lock().await;
            received_tx.is_none()
        };

        let mut errs = vec![];
        if received_tx_is_none {
            let tracks = self.internal.tracks.write().await;
            for t in &*tracks {
                if let Some(rtcp_read_stream) = &t.stream.rtcp_read_stream {
                    if let Err(err) = rtcp_read_stream.close().await {
                        errs.push(err);
                    }
                }

                if let Some(rtp_read_stream) = &t.stream.rtp_read_stream {
                    if let Err(err) = rtp_read_stream.close().await {
                        errs.push(err);
                    }
                }

                if let Some(repair_rtcp_read_stream) = &t.repair_stream.rtcp_read_stream {
                    if let Err(err) = repair_rtcp_read_stream.close().await {
                        errs.push(err);
                    }
                }

                if let Some(repair_rtp_read_stream) = &t.repair_stream.rtp_read_stream {
                    if let Err(err) = repair_rtp_read_stream.close().await {
                        errs.push(err);
                    }
                }

                if let Some(stream_info) = &t.stream.stream_info {
                    self.internal
                        .interceptor
                        .unbind_remote_stream(stream_info)
                        .await;
                }

                if let Some(repair_stream_info) = &t.repair_stream.stream_info {
                    self.internal
                        .interceptor
                        .unbind_remote_stream(repair_stream_info)
                        .await;
                }
            }
        }

        flatten_errs(errs)
    }
}
