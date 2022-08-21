
use std::fmt;
use crate::webrtc::stun::attributes::*;
use crate::webrtc::stun::checks::*;
use crate::webrtc::stun::message::*;

// 16 bits of uint + 16 bits of RFFU = 0.
const CHANNEL_NUMBER_SIZE: usize = 4;

// ChannelNumber represents CHANNEL-NUMBER attribute.
//
// The CHANNEL-NUMBER attribute contains the number of the channel.
//
// RFC 5766 Section 14.1
// encoded as uint16
#[derive(Default, Eq, PartialEq, Debug, Copy, Clone, Hash)]
pub(crate) struct ChannelNumber(pub(crate) u16);

impl fmt::Display for ChannelNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Setter for ChannelNumber {
    // AddTo adds CHANNEL-NUMBER to message.
    fn add_to(&self, m: &mut Message) -> Result<(), crate::webrtc::stun::Error> {
        let mut v = vec![0; CHANNEL_NUMBER_SIZE];
        v[..2].copy_from_slice(&self.0.to_be_bytes());
        // v[2:4] are zeroes (RFFU = 0)
        m.add(ATTR_CHANNEL_NUMBER, &v);
        Ok(())
    }
}

impl Getter for ChannelNumber {
    // GetFrom decodes CHANNEL-NUMBER from message.
    fn get_from(&mut self, m: &Message) -> Result<(), crate::webrtc::stun::Error> {
        let v = m.get(ATTR_CHANNEL_NUMBER)?;

        check_size(ATTR_CHANNEL_NUMBER, v.len(), CHANNEL_NUMBER_SIZE)?;

        //_ = v[CHANNEL_NUMBER_SIZE-1] // asserting length
        self.0 = u16::from_be_bytes([v[0], v[1]]);
        // v[2:4] is RFFU and equals to 0.
        Ok(())
    }
}
