
use crate::webrtc::dtls_transport::RTCDtlsTransport;
use crate::webrtc::error::{flatten_errs, Error, Result};
use crate::webrtc::peer_connection::sdp::TrackDetails;
use crate::webrtc::rtp_transceiver::rtp_codec::{
    RTCRtpCodecCapability, RTCRtpCodecParameters,
    RTPCodecType,
};
use crate::webrtc::rtp_transceiver::{
    create_stream_info, RTCRtpDecodingParameters, RTCRtpReceiveParameters, SSRC,
};
use crate::webrtc::track::track_remote::TrackRemote;
use crate::webrtc::track::{TrackStream, TrackStreams};

use interceptor::{Attributes, Interceptor};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, Notify, RwLock};
use crate::webrtc::RECEIVE_MTU;

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

    pub fn kind(&self) -> RTPCodecType {
        self.kind
    }

    pub(crate) async fn set_transceiver_codecs(
        &self,
        codecs: Option<Arc<Mutex<Vec<RTCRtpCodecParameters>>>>,
    ) {
        let mut transceiver_codecs = self.internal.transceiver_codecs.lock().await;
        *transceiver_codecs = codecs;
    }

    /// track returns the RtpTransceiver TrackRemote
    pub async fn track(&self) -> Option<Arc<TrackRemote>> {
        let tracks = self.internal.tracks.read().await;
        if tracks.len() != 1 {
            None
        } else {
            tracks.first().map(|t| Arc::clone(&t.track))
        }
    }

    /// tracks returns the RtpTransceiver traclockks
    /// A RTPReceiver to support Simulcast may now have multiple tracks
    pub async fn tracks(&self) -> Vec<Arc<TrackRemote>> {
        let tracks = self.internal.tracks.read().await;
        tracks.iter().map(|t| Arc::clone(&t.track)).collect()
    }

    pub(crate) async fn have_received(&self) -> bool {
        let received_tx = self.received_tx.lock().await;
        received_tx.is_none()
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

    /// receiveForRtx starts a routine that processes the repair stream
    /// These packets aren't exposed to the user yet, but we need to process them for
    /// TWCC
    pub(crate) async fn receive_for_rtx(
        &self,
        ssrc: SSRC,
        rsid: String,
        repair_stream: TrackStream,
    ) -> Result<()> {
        let mut tracks = self.internal.tracks.write().await;
        let l = tracks.len();
        for t in &mut *tracks {
            if (ssrc != 0 && l == 1) || t.track.rid() == rsid {
                t.repair_stream = repair_stream;

                let track = t.clone();
                tokio::spawn(async move {
                    let a = Attributes::new();
                    let mut b = vec![0u8; RECEIVE_MTU];
                    while let Some(repair_rtp_interceptor) = &track.repair_stream.rtp_interceptor {
                        //TODO: cancel repair_rtp_interceptor.read gracefully
                        //println!("repair_rtp_interceptor read begin with ssrc={}", ssrc);
                        if repair_rtp_interceptor.read(&mut b, &a).await.is_err() {
                            break;
                        }
                    }
                });

                return Ok(());
            }
        }

        Err(Error::ErrRTPReceiverForRIDTrackStreamNotFound)
    }
}
