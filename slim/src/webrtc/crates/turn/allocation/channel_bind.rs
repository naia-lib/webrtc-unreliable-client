
use super::*;
use crate::webrtc::turn::proto::channum::*;

use std::sync::{atomic::AtomicBool, Arc};
use tokio::sync::Mutex;
use tokio::time::Duration;

// ChannelBind represents a TURN Channel
// https://tools.ietf.org/html/rfc5766#section-2.5
#[derive(Clone)]
pub struct ChannelBind {
    pub(crate) peer: SocketAddr,
    pub(crate) number: ChannelNumber,
    pub(crate) channel_bindings: Option<Arc<Mutex<HashMap<ChannelNumber, ChannelBind>>>>,
    reset_tx: Option<mpsc::Sender<Duration>>,
    timer_expired: Arc<AtomicBool>,
}
