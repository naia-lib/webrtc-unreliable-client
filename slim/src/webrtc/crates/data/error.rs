use std::string::FromUtf8Error;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error, PartialEq)]
#[non_exhaustive]
pub enum Error {
    #[error(
        "DataChannel message is not long enough to determine type: (expected: {expected}, actual: {actual})"
    )]
    UnexpectedEndOfBuffer { expected: usize, actual: usize },
    #[error("Unknown MessageType {0}")]
    InvalidMessageType(u8),
    #[error("Unknown ChannelType {0}")]
    InvalidChannelType(u8),
    #[error("Unknown PayloadProtocolIdentifier {0}")]
    InvalidPayloadProtocolIdentifier(u8),
    #[error("Stream closed")]
    ErrStreamClosed,

    #[error("{0}")]
    Util(#[from] util::Error),
    #[error("{0}")]
    Sctp(#[from] crate::webrtc::sctp::Error),
    #[error("utf-8 error: {0}")]
    Utf8(#[from] FromUtf8Error),

    #[allow(non_camel_case_types)]
    #[error("{0}")]
    new(String),
}

impl From<Error> for util::Error {
    fn from(e: Error) -> Self {
        util::Error::from_std(e)
    }
}

impl PartialEq<util::Error> for Error {
    fn eq(&self, other: &util::Error) -> bool {
        if let Some(down) = other.downcast_ref::<Error>() {
            return self == down;
        }
        false
    }
}

impl PartialEq<crate::webrtc::sctp::Error> for Error {
    fn eq(&self, other: &crate::webrtc::sctp::Error) -> bool {
        if let Error::Sctp(e) = self {
            return e == other;
        }
        false
    }
}
