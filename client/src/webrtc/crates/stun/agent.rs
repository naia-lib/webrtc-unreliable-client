
use crate::webrtc::stun::error::*;
use crate::webrtc::stun::message::*;

use rand::Rng;
use tokio::time::Instant;

#[derive(Debug, Clone)]
pub(crate) enum EventType {
    Callback(TransactionId),
}

impl Default for EventType {
    fn default() -> Self {
        EventType::Callback(TransactionId::default())
    }
}

#[derive(PartialEq, Eq, Hash, Copy, Clone, Default, Debug)]
pub(crate) struct TransactionId(pub(crate) [u8; TRANSACTION_ID_SIZE]);

impl TransactionId {
    /// new returns new random transaction ID using crypto/rand
    /// as source.
    pub(crate) fn new() -> Self {
        let mut b = TransactionId([0u8; TRANSACTION_ID_SIZE]);
        rand::thread_rng().fill(&mut b.0);
        b
    }
}

impl Setter for TransactionId {
    fn add_to(&self, m: &mut Message) -> Result<()> {
        m.transaction_id = *self;
        m.write_transaction_id();
        Ok(())
    }
}

/// ClientAgent is Agent implementation that is used by Client to
/// process transactions.
#[derive(Debug)]
pub(crate) enum ClientAgent {
    Collect(Instant),
}