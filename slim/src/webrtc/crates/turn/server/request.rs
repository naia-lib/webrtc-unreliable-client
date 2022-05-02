#[cfg(test)]
mod request_test;

use crate::webrtc::turn::allocation::allocation_manager::*;
use crate::webrtc::turn::allocation::channel_bind::ChannelBind;
use crate::webrtc::turn::allocation::five_tuple::*;
use crate::webrtc::turn::allocation::permission::Permission;
use crate::webrtc::turn::auth::*;
use crate::webrtc::turn::error::*;
use crate::webrtc::turn::proto::chandata::ChannelData;
use crate::webrtc::turn::proto::channum::ChannelNumber;
use crate::webrtc::turn::proto::data::Data;
use crate::webrtc::turn::proto::evenport::EvenPort;
use crate::webrtc::turn::proto::lifetime::*;
use crate::webrtc::turn::proto::peeraddr::PeerAddress;
use crate::webrtc::turn::proto::relayaddr::RelayedAddress;
use crate::webrtc::turn::proto::reqtrans::RequestedTransport;
use crate::webrtc::turn::proto::rsrvtoken::ReservationToken;
use crate::webrtc::turn::proto::*;

use stun::agent::*;
use stun::attributes::*;
use stun::error_code::*;
use stun::fingerprint::*;
use stun::integrity::*;
use stun::message::*;
use stun::textattrs::*;
use stun::uattrs::*;
use stun::xoraddr::*;

use crate::webrtc::util::Conn;

use std::collections::HashMap;
use std::marker::{Send, Sync};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

use md5::{Digest, Md5};

pub(crate) const MAXIMUM_ALLOCATION_LIFETIME: Duration = Duration::from_secs(3600); // https://tools.ietf.org/html/rfc5766#section-6.2 defines 3600 seconds recommendation
pub(crate) const NONCE_LIFETIME: Duration = Duration::from_secs(3600); // https://tools.ietf.org/html/rfc5766#section-4

// Request contains all the state needed to process a single incoming datagram
pub struct Request {
    // Current Request State
    pub conn: Arc<dyn Conn + Send + Sync>,
    pub src_addr: SocketAddr,
    pub buff: Vec<u8>,

    // Server State
    pub allocation_manager: Arc<Manager>,
    pub nonces: Arc<Mutex<HashMap<String, Instant>>>,

    // User Configuration
    pub auth_handler: Arc<dyn AuthHandler + Send + Sync>,
    pub realm: String,
    pub channel_bind_timeout: Duration,
}
