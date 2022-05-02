
use crate::webrtc::stun::error::*;
use crate::webrtc::stun::message::*;

use rand::Rng;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::Instant;

/// Handler handles state changes of transaction.
/// Handler is called on transaction state change.
/// Usage of e is valid only during call, user must
/// copy needed fields explicitly.
pub type Handler = Option<Arc<mpsc::UnboundedSender<Event>>>;

#[derive(Debug, Clone)]
pub enum EventType {
    Callback(TransactionId),
}

impl Default for EventType {
    fn default() -> Self {
        EventType::Callback(TransactionId::default())
    }
}

/// Event is passed to Handler describing the transaction event.
/// Do not reuse outside Handler.
#[derive(Debug)] //Clone
pub struct Event {
    pub event_type: EventType,
    pub event_body: Result<Message>,
}

impl Default for Event {
    fn default() -> Self {
        Event {
            event_type: EventType::default(),
            event_body: Ok(Message::default()),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Copy, Clone, Default, Debug)]
pub struct TransactionId(pub [u8; TRANSACTION_ID_SIZE]);

impl TransactionId {
    /// new returns new random transaction ID using crypto/rand
    /// as source.
    pub fn new() -> Self {
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
pub enum ClientAgent {
    Collect(Instant),
}