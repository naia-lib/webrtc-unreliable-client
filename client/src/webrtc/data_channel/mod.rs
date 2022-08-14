#[cfg(test)]
mod data_channel_test;

pub mod data_channel_message;
pub mod data_channel_state;

use data_channel_message::*;

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Weak};

use crate::webrtc::data::message::message_channel_open::ChannelType;
use crate::webrtc::sctp::stream::OnBufferedAmountLowFn;
use tokio::sync::{Mutex, Notify};

use data_channel_state::RTCDataChannelState;

use crate::webrtc::error::{Error, OnErrorHdlrFn, Result};
use crate::webrtc::sctp_transport::RTCSctpTransport;

pub type OnMessageHdlrFn = Box<
    dyn (FnMut(DataChannelMessage) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>)
        + Send
        + Sync,
>;

pub type OnOpenHdlrFn =
    Box<dyn (FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>) + Send + Sync>;

pub type OnCloseHdlrFn =
    Box<dyn (FnMut() -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>) + Send + Sync>;

/// DataChannel represents a WebRTC DataChannel
/// The DataChannel interface represents a network channel
/// which can be used for bidirectional peer-to-peer transfers of arbitrary data
#[derive(Default)]
pub struct RTCDataChannel {

    pub ready_state: Arc<AtomicU8>, // DataChannelState
    pub buffered_amount_low_threshold: AtomicUsize,
    pub detach_called: Arc<AtomicBool>,

    // The binaryType represents attribute MUST, on getting, return the value to
    // which it was last set. On setting, if the new value is either the string
    // "blob" or the string "arraybuffer", then set the IDL attribute to this
    // new value. Otherwise, throw a SyntaxError. When an DataChannel object
    // is created, the binaryType attribute MUST be initialized to the string
    // "blob". This attribute controls how binary data is exposed to scripts.
    // binaryType                 string
    pub on_message_handler: Arc<Mutex<Option<OnMessageHdlrFn>>>,
    pub on_open_handler: Arc<Mutex<Option<OnOpenHdlrFn>>>,
    pub on_close_handler: Arc<Mutex<Option<OnCloseHdlrFn>>>,
    pub on_error_handler: Arc<Mutex<Option<OnErrorHdlrFn>>>,

    pub on_buffered_amount_low: Mutex<Option<OnBufferedAmountLowFn>>,

    pub sctp_transport: Mutex<Option<Weak<RTCSctpTransport>>>,
    pub data_channel: Mutex<Option<Arc<crate::webrtc::data::data_channel::DataChannel>>>,

    pub notify_tx: Arc<Notify>,

}

impl RTCDataChannel {
    // create the DataChannel object before the networking is set up.
    pub fn new() -> Self {

        RTCDataChannel {
            ready_state: Arc::new(AtomicU8::new(RTCDataChannelState::Connecting as u8)),
            detach_called: Arc::new(AtomicBool::new(false)),

            notify_tx: Arc::new(Notify::new()),
            ..Default::default()
        }
    }

    /// open opens the datachannel over the sctp transport
    pub async fn open(&self, sctp_transport: Arc<RTCSctpTransport>) -> Result<()> {
        if let Some(association) = sctp_transport.association().await {
            {
                let mut st = self.sctp_transport.lock().await;
                if st.is_none() {
                    *st = Some(Arc::downgrade(&sctp_transport));
                } else {
                    return Ok(());
                }
            }

            // Do next Connor
            let cfg = crate::webrtc::data::data_channel::Config {
                channel_type: ChannelType::PartialReliableRexmitUnordered,
                priority: crate::webrtc::data::message::message_channel_open::CHANNEL_PRIORITY_NORMAL,
                reliability_parameter: Some(0),
                label: "data".to_string(),
                protocol: "".to_string(),
                negotiated: false,
            };

            let dc = crate::webrtc::data::data_channel::DataChannel::dial(&association, 0, cfg).await?;

            // buffered_amount_low_threshold and on_buffered_amount_low might be set earlier
            dc.set_buffered_amount_low_threshold(
                self.buffered_amount_low_threshold.load(Ordering::SeqCst),
            );
            {
                let mut on_buffered_amount_low = self.on_buffered_amount_low.lock().await;
                if let Some(f) = on_buffered_amount_low.take() {
                    dc.on_buffered_amount_low(f).await;
                }
            }

            self.handle_open(Arc::new(dc)).await;

            Ok(())
        } else {
            Err(Error::ErrSCTPNotEstablished)
        }
    }

    /// on_open sets an event handler which is invoked when
    /// the underlying data transport has been established (or re-established).
    pub async fn on_open(&self, f: OnOpenHdlrFn) {
        {
            let mut handler = self.on_open_handler.lock().await;
            *handler = Some(f);
        }

        if self.ready_state() == RTCDataChannelState::Open {
            self.do_open().await;
        }
    }

    async fn do_open(&self) {
        let on_open_handler = Arc::clone(&self.on_open_handler);
        let detach_data_channels = true;
        let detach_called = Arc::clone(&self.detach_called);
        tokio::spawn(async move {
            let mut handler = on_open_handler.lock().await;
            if let Some(f) = handler.take() {
                f().await;

                // self.check_detach_after_open();
                // After onOpen is complete check that the user called detach
                // and provide an error message if the call was missed
                if detach_data_channels && !detach_called.load(Ordering::SeqCst) {
                    log::warn!(
                        "webrtc.DetachDataChannels() enabled but didn't Detach, call Detach from OnOpen"
                    );
                }
            }
        });
    }

    pub async fn handle_open(&self, dc: Arc<crate::webrtc::data::data_channel::DataChannel>) {
        {
            let mut data_channel = self.data_channel.lock().await;
            *data_channel = Some(Arc::clone(&dc));
        }
        self.set_ready_state(RTCDataChannelState::Open);

        self.do_open().await;
    }

    /// on_error sets an event handler which is invoked when
    /// the underlying data transport cannot be read.
    pub async fn on_error(&self, f: OnErrorHdlrFn) {
        let mut handler = self.on_error_handler.lock().await;
        *handler = Some(f);
    }

    /// detach allows you to detach the underlying datachannel. This provides
    /// an idiomatic API to work with, however it disables the OnMessage callback.
    /// Before calling Detach you have to enable this behavior by calling
    /// webrtc.DetachDataChannels(). Combining detached and normal data channels
    /// is not supported.
    /// Please refer to the data-channels-detach example and the
    /// pion/datachannel documentation for the correct way to handle the
    /// resulting DataChannel object.
    pub async fn detach(&self) -> Result<Arc<crate::webrtc::data::data_channel::DataChannel>> {

        let data_channel = self.data_channel.lock().await;
        if let Some(dc) = &*data_channel {
            self.detach_called.store(true, Ordering::SeqCst);

            Ok(Arc::clone(dc))
        } else {
            Err(Error::ErrDetachBeforeOpened)
        }
    }

    /// ready_state represents the state of the DataChannel object.
    pub fn ready_state(&self) -> RTCDataChannelState {
        self.ready_state.load(Ordering::SeqCst).into()
    }

    pub fn set_ready_state(&self, r: RTCDataChannelState) {
        self.ready_state.store(r as u8, Ordering::SeqCst);
    }
}
