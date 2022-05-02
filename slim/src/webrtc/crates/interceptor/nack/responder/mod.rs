mod responder_stream;

use crate::webrtc::interceptor::stream_info::StreamInfo;
use crate::webrtc::interceptor::{
    Attributes, Interceptor, InterceptorBuilder, RTCPReader, RTCPWriter, RTPReader, RTPWriter,
};
use responder_stream::ResponderStream;

use crate::webrtc::interceptor::error::Result;
use crate::webrtc::interceptor::nack::stream_support_nack;

use async_trait::async_trait;
use rtcp::transport_feedbacks::transport_layer_nack::TransportLayerNack;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

/// GeneratorBuilder can be used to configure Responder Interceptor
#[derive(Default)]
pub struct ResponderBuilder {
    log2_size: Option<u8>,
}

impl InterceptorBuilder for ResponderBuilder {
    fn build(&self, _id: &str) -> Result<Arc<dyn Interceptor + Send + Sync>> {
        Ok(Arc::new(Responder {
            internal: Arc::new(ResponderInternal {
                log2_size: if let Some(log2_size) = self.log2_size {
                    log2_size
                } else {
                    13 // 8192 = 1 << 13
                },
                streams: Arc::new(Mutex::new(HashMap::new())),
            }),
        }))
    }
}

pub struct ResponderInternal {
    log2_size: u8,
    streams: Arc<Mutex<HashMap<u32, Arc<ResponderStream>>>>,
}

impl ResponderInternal {
    async fn resend_packets(
        streams: Arc<Mutex<HashMap<u32, Arc<ResponderStream>>>>,
        nack: TransportLayerNack,
    ) {
        let stream = {
            let m = streams.lock().await;
            if let Some(stream) = m.get(&nack.media_ssrc) {
                stream.clone()
            } else {
                return;
            }
        };

        for n in &nack.nacks {
            let stream2 = Arc::clone(&stream);
            n.range(Box::new(
                move |seq: u16| -> Pin<Box<dyn Future<Output = bool> + Send + 'static>> {
                    let stream3 = Arc::clone(&stream2);
                    Box::pin(async move {
                        if let Some(p) = stream3.get(seq).await {
                            let a = Attributes::new();
                            if let Err(err) = stream3.next_rtp_writer.write(&p, &a).await {
                                log::warn!("failed resending nacked packet: {}", err);
                            }
                        }

                        true
                    })
                },
            ))
            .await;
        }
    }
}

/// Responder responds to nack feedback messages
pub struct Responder {
    internal: Arc<ResponderInternal>,
}

#[async_trait]
impl Interceptor for Responder {

    /// bind_local_stream lets you modify any outgoing RTP packets. It is called once for per LocalStream. The returned method
    /// will be called once per rtp packet.
    async fn bind_local_stream(
        &self,
        info: &StreamInfo,
        writer: Arc<dyn RTPWriter + Send + Sync>,
    ) -> Arc<dyn RTPWriter + Send + Sync> {
        if !stream_support_nack(info) {
            return writer;
        }

        let stream = Arc::new(ResponderStream::new(self.internal.log2_size, writer));
        {
            let mut streams = self.internal.streams.lock().await;
            streams.insert(info.ssrc, Arc::clone(&stream));
        }

        stream
    }

    /// unbind_local_stream is called when the Stream is removed. It can be used to clean up any data related to that track.
    async fn unbind_local_stream(&self, info: &StreamInfo) {
        let mut streams = self.internal.streams.lock().await;
        streams.remove(&info.ssrc);
    }

    /// bind_remote_stream lets you modify any incoming RTP packets. It is called once for per RemoteStream. The returned method
    /// will be called once per rtp packet.
    async fn bind_remote_stream(
        &self,
        _info: &StreamInfo,
        reader: Arc<dyn RTPReader + Send + Sync>,
    ) -> Arc<dyn RTPReader + Send + Sync> {
        reader
    }

    /// unbind_remote_stream is called when the Stream is removed. It can be used to clean up any data related to that track.
    async fn unbind_remote_stream(&self, _info: &StreamInfo) {}

    /// close closes the Interceptor, cleaning up any data if necessary.
    async fn close(&self) -> Result<()> {
        Ok(())
    }
}
