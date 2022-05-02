use crate::webrtc::turn::auth::*;
use crate::webrtc::turn::error::*;
use crate::webrtc::turn::relay::*;

use crate::webrtc::util::Conn;

use std::sync::Arc;
use tokio::time::Duration;

// ConnConfig is used for UDP listeners
pub struct ConnConfig {
    pub conn: Arc<dyn Conn + Send + Sync>,

    // When an allocation is generated the RelayAddressGenerator
    // creates the net.PacketConn and returns the IP/Port it is available at
    pub relay_addr_generator: Box<dyn RelayAddressGenerator + Send + Sync>,
}

// ServerConfig configures the Pion TURN Server
pub struct ServerConfig {
    // conn_configs are a list of all the turn listeners
    // Each listener can have custom behavior around the creation of Relays
    pub conn_configs: Vec<ConnConfig>,

    // realm sets the realm for this server
    pub realm: String,

    // auth_handler is a callback used to handle incoming auth requests, allowing users to customize Pion TURN with custom behavior
    pub auth_handler: Arc<dyn AuthHandler + Send + Sync>,

    // channel_bind_timeout sets the lifetime of channel binding. Defaults to 10 minutes.
    pub channel_bind_timeout: Duration,
}