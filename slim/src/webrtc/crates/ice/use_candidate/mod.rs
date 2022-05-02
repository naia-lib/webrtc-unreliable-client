
use stun::attributes::ATTR_USE_CANDIDATE;
use stun::message::*;

/// Represents USE-CANDIDATE attribute.
#[derive(Default)]
pub struct UseCandidateAttr;

impl Setter for UseCandidateAttr {
    /// Adds USE-CANDIDATE attribute to message.
    fn add_to(&self, m: &mut Message) -> Result<(), stun::Error> {
        m.add(ATTR_USE_CANDIDATE, &[]);
        Ok(())
    }
}
