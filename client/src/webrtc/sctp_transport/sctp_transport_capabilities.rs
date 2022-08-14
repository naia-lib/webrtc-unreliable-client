use serde::{Deserialize, Serialize};

/// SCTPTransportCapabilities indicates the capabilities of the SCTPTransport.
#[derive(Debug, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub(crate) struct SCTPTransportCapabilities {
    pub(crate) max_message_size: u32,
}
