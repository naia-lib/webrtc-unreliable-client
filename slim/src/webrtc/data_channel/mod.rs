#[cfg(test)]
mod data_channel_test;

pub mod data_channel_init;
pub mod data_channel_message;
pub mod data_channel_parameters;
pub mod data_channel_state;

use data_channel_message::*;
use data_channel_parameters::*;

use bytes::Bytes;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Weak};

use crate::webrtc::data::message::message_channel_open::ChannelType;
use crate::webrtc::sctp::stream::OnBufferedAmountLowFn;
use tokio::sync::{Mutex, Notify};

use data_channel_state::RTCDataChannelState;
use crate::webrtc::dtls_transport::dtls_role::DTLSRole;

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
    // these fields, when removed, seem to crash compilation...
    #[allow(dead_code)]
    pub(crate) setting_engine: bool,

    pub(crate) label: String,
    pub(crate) ordered: bool,
    pub(crate) max_packet_lifetime: Option<u16>,
    pub(crate) max_retransmits: Option<u16>,
    pub(crate) protocol: String,
    pub(crate) negotiated: bool,
    pub(crate) has_id: AtomicBool,
    pub(crate) id: AtomicU16,
    pub(crate) ready_state: Arc<AtomicU8>, // DataChannelState
    pub(crate) buffered_amount_low_threshold: AtomicUsize,
    pub(crate) detach_called: Arc<AtomicBool>,

    // The binaryType represents attribute MUST, on getting, return the value to
    // which it was last set. On setting, if the new value is either the string
    // "blob" or the string "arraybuffer", then set the IDL attribute to this
    // new value. Otherwise, throw a SyntaxError. When an DataChannel object
    // is created, the binaryType attribute MUST be initialized to the string
    // "blob". This attribute controls how binary data is exposed to scripts.
    // binaryType                 string
    pub(crate) on_message_handler: Arc<Mutex<Option<OnMessageHdlrFn>>>,
    pub(crate) on_open_handler: Arc<Mutex<Option<OnOpenHdlrFn>>>,
    pub(crate) on_close_handler: Arc<Mutex<Option<OnCloseHdlrFn>>>,
    pub(crate) on_error_handler: Arc<Mutex<Option<OnErrorHdlrFn>>>,

    pub(crate) on_buffered_amount_low: Mutex<Option<OnBufferedAmountLowFn>>,

    pub(crate) sctp_transport: Mutex<Option<Weak<RTCSctpTransport>>>,
    pub(crate) data_channel: Mutex<Option<Arc<crate::webrtc::data::data_channel::DataChannel>>>,

    pub(crate) notify_tx: Arc<Notify>,

}

impl RTCDataChannel {
    // create the DataChannel object before the networking is set up.
    pub(crate) fn new(params: DataChannelParameters) -> Self {
        let id = match params.id {
            Some(inner) => inner,
            None => 0,
        };

        RTCDataChannel {
            label: params.label,
            protocol: params.protocol,
            negotiated: params.negotiated,
            has_id: AtomicBool::new(params.id.is_some()),
            id: AtomicU16::new(id),
            ordered: params.ordered,
            max_packet_lifetime: params.max_packet_life_time,
            max_retransmits: params.max_retransmits,
            ready_state: Arc::new(AtomicU8::new(RTCDataChannelState::Connecting as u8)),
            detach_called: Arc::new(AtomicBool::new(false)),

            notify_tx: Arc::new(Notify::new()),

            setting_engine: true,
            ..Default::default()
        }
    }

    /// open opens the datachannel over the sctp transport
    pub(crate) async fn open(&self, sctp_transport: Arc<RTCSctpTransport>) -> Result<()> {
        if let Some(association) = sctp_transport.association().await {
            {
                let mut st = self.sctp_transport.lock().await;
                if st.is_none() {
                    *st = Some(Arc::downgrade(&sctp_transport));
                } else {
                    return Ok(());
                }
            }

            let channel_type;
            let reliability_parameter: Option<u16>;

            if self.max_packet_lifetime.is_none() && self.max_retransmits.is_none() {
                reliability_parameter = None;
                if self.ordered {
                    channel_type = ChannelType::Reliable;
                } else {
                    channel_type = ChannelType::ReliableUnordered;
                }
            } else if self.max_retransmits.is_some() {
                reliability_parameter = self.max_retransmits;
                if self.ordered {
                    channel_type = ChannelType::PartialReliableRexmit;
                } else {
                    channel_type = ChannelType::PartialReliableRexmitUnordered;
                }
            } else {
                reliability_parameter = self.max_packet_lifetime;
                if self.ordered {
                    channel_type = ChannelType::PartialReliableTimed;
                } else {
                    channel_type = ChannelType::PartialReliableTimedUnordered;
                }
            }

            let cfg = crate::webrtc::data::data_channel::Config {
                channel_type,
                priority: crate::webrtc::data::message::message_channel_open::CHANNEL_PRIORITY_NORMAL,
                reliability_parameter,
                label: self.label.clone(),
                protocol: self.protocol.clone(),
                negotiated: self.negotiated,
            };

            if !self.has_id.load(Ordering::SeqCst) {
                self.has_id.store(true, Ordering::SeqCst);
                self.id.store(
                    sctp_transport
                        .generate_and_set_data_channel_id(
                            DTLSRole::Client
                        )
                        .await?,
                    Ordering::SeqCst,
                );
            }

            let dc = crate::webrtc::data::data_channel::DataChannel::dial(&association, self.id(), cfg).await?;

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

    /// transport returns the SCTPTransport instance the DataChannel is sending over.
    pub async fn transport(&self) -> Option<Weak<RTCSctpTransport>> {
        let sctp_transport = self.sctp_transport.lock().await;
        sctp_transport.clone()
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

    /// on_close sets an event handler which is invoked when
    /// the underlying data transport has been closed.
    pub async fn on_close(&self, f: OnCloseHdlrFn) {
        let mut handler = self.on_close_handler.lock().await;
        *handler = Some(f);
    }

    /// on_message sets an event handler which is invoked on a binary
    /// message arrival over the sctp transport from a remote peer.
    /// OnMessage can currently receive messages up to 16384 bytes
    /// in size. Check out the detach API if you want to use larger
    /// message sizes. Note that browser support for larger messages
    /// is also limited.
    pub async fn on_message(&self, f: OnMessageHdlrFn) {
        let mut handler = self.on_message_handler.lock().await;
        *handler = Some(f);
    }

    pub(crate) async fn handle_open(&self, dc: Arc<crate::webrtc::data::data_channel::DataChannel>) {
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

    /// send sends the binary message to the DataChannel peer
    pub async fn send(&self, data: &Bytes) -> Result<usize> {
        self.ensure_open()?;

        let data_channel = self.data_channel.lock().await;
        if let Some(dc) = &*data_channel {
            Ok(dc.write_data_channel(data, false).await?)
        } else {
            Err(Error::ErrClosedPipe)
        }
    }

    /// send_text sends the text message to the DataChannel peer
    pub async fn send_text(&self, s: String) -> Result<usize> {
        self.ensure_open()?;

        let data_channel = self.data_channel.lock().await;
        if let Some(dc) = &*data_channel {
            Ok(dc.write_data_channel(&Bytes::from(s), true).await?)
        } else {
            Err(Error::ErrClosedPipe)
        }
    }

    fn ensure_open(&self) -> Result<()> {
        if self.ready_state() != RTCDataChannelState::Open {
            Err(Error::ErrClosedPipe)
        } else {
            Ok(())
        }
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

    /// Close Closes the DataChannel. It may be called regardless of whether
    /// the DataChannel object was created by this peer or the remote peer.
    pub async fn close(&self) -> Result<()> {
        if self.ready_state() == RTCDataChannelState::Closed {
            return Ok(());
        }

        self.set_ready_state(RTCDataChannelState::Closing);
        self.notify_tx.notify_waiters();

        let data_channel = self.data_channel.lock().await;
        if let Some(dc) = &*data_channel {
            Ok(dc.close().await?)
        } else {
            Ok(())
        }
    }

    /// label represents a label that can be used to distinguish this
    /// DataChannel object from other DataChannel objects. Scripts are
    /// allowed to create multiple DataChannel objects with the same label.
    pub fn label(&self) -> &str {
        self.label.as_str()
    }

    /// Ordered returns true if the DataChannel is ordered, and false if
    /// out-of-order delivery is allowed.
    pub fn ordered(&self) -> bool {
        self.ordered
    }

    /// max_packet_lifetime represents the length of the time window (msec) during
    /// which transmissions and retransmissions may occur in unreliable mode.
    pub fn max_packet_lifetime(&self) -> Option<u16> {
        self.max_packet_lifetime
    }

    /// max_retransmits represents the maximum number of retransmissions that are
    /// attempted in unreliable mode.
    pub fn max_retransmits(&self) -> Option<u16> {
        self.max_retransmits
    }

    /// protocol represents the name of the sub-protocol used with this
    /// DataChannel.
    pub fn protocol(&self) -> &str {
        self.protocol.as_str()
    }

    /// negotiated represents whether this DataChannel was negotiated by the
    /// application (true), or not (false).
    pub fn negotiated(&self) -> bool {
        self.negotiated
    }

    /// ID represents the ID for this DataChannel. The value is initially
    /// null, which is what will be returned if the ID was not provided at
    /// channel creation time, and the DTLS role of the SCTP transport has not
    /// yet been negotiated. Otherwise, it will return the ID that was either
    /// selected by the script or generated. After the ID is set to a non-null
    /// value, it will not change.
    pub fn id(&self) -> u16 {
        self.id.load(Ordering::SeqCst)
    }

    /// ready_state represents the state of the DataChannel object.
    pub fn ready_state(&self) -> RTCDataChannelState {
        self.ready_state.load(Ordering::SeqCst).into()
    }

    /// buffered_amount represents the number of bytes of application data
    /// (UTF-8 text and binary data) that have been queued using send(). Even
    /// though the data transmission can occur in parallel, the returned value
    /// MUST NOT be decreased before the current task yielded back to the event
    /// loop to prevent race conditions. The value does not include framing
    /// overhead incurred by the protocol, or buffering done by the operating
    /// system or network hardware. The value of buffered_amount slot will only
    /// increase with each call to the send() method as long as the ready_state is
    /// open; however, buffered_amount does not reset to zero once the channel
    /// closes.
    pub async fn buffered_amount(&self) -> usize {
        let data_channel = self.data_channel.lock().await;
        if let Some(dc) = &*data_channel {
            dc.buffered_amount()
        } else {
            0
        }
    }

    /// buffered_amount_low_threshold represents the threshold at which the
    /// bufferedAmount is considered to be low. When the bufferedAmount decreases
    /// from above this threshold to equal or below it, the bufferedamountlow
    /// event fires. buffered_amount_low_threshold is initially zero on each new
    /// DataChannel, but the application may change its value at any time.
    /// The threshold is set to 0 by default.
    pub async fn buffered_amount_low_threshold(&self) -> usize {
        let data_channel = self.data_channel.lock().await;
        if let Some(dc) = &*data_channel {
            dc.buffered_amount_low_threshold()
        } else {
            self.buffered_amount_low_threshold.load(Ordering::SeqCst)
        }
    }

    /// set_buffered_amount_low_threshold is used to update the threshold.
    /// See buffered_amount_low_threshold().
    pub async fn set_buffered_amount_low_threshold(&self, th: usize) {
        self.buffered_amount_low_threshold
            .store(th, Ordering::SeqCst);
        let data_channel = self.data_channel.lock().await;
        if let Some(dc) = &*data_channel {
            dc.set_buffered_amount_low_threshold(th);
        }
    }

    /// on_buffered_amount_low sets an event handler which is invoked when
    /// the number of bytes of outgoing data becomes lower than the
    /// buffered_amount_low_threshold.
    pub async fn on_buffered_amount_low(&self, f: OnBufferedAmountLowFn) {
        let data_channel = self.data_channel.lock().await;
        if let Some(dc) = &*data_channel {
            dc.on_buffered_amount_low(f).await;
        } else {
            let mut on_buffered_amount_low = self.on_buffered_amount_low.lock().await;
            *on_buffered_amount_low = Some(f);
        }
    }

    pub(crate) fn set_ready_state(&self, r: RTCDataChannelState) {
        self.ready_state.store(r as u8, Ordering::SeqCst);
    }
}
