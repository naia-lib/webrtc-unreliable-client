use thiserror::Error;

use std::io;
use std::net;
use std::num::ParseIntError;
use std::time::SystemTimeError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error, PartialEq)]
#[non_exhaustive]
pub enum Error {
    #[error("turn: RelayAddressGenerator has invalid ListeningAddress")]
    ErrListeningAddressInvalid,
    #[error("turn: max retries exceeded")]
    ErrMaxRetriesExceeded,
    #[error("turn: MaxPort must be not 0")]
    ErrMaxPortNotZero,
    #[error("turn: MaxPort must be not 0")]
    ErrMinPortNotZero,
    #[error("turn: MaxPort less than MinPort")]
    ErrMaxPortLessThanMinPort,
    #[error("all retransmissions failed")]
    ErrAllRetransmissionsFailed,
    #[error("no binding found for channel")]
    ErrChannelBindNotFound,
    #[error("only one Allocate() caller is allowed")]
    ErrOneAllocateOnly,
    #[error("non-STUN message from STUN server")]
    ErrNonStunmessage,
    #[error("unexpected STUN request message")]
    ErrUnexpectedStunrequestMessage,
    #[error("channel number not in [0x4000, 0x7FFF]")]
    ErrInvalidChannelNumber,
    #[error("channelData length != len(Data)")]
    ErrBadChannelDataLength,
    #[error("unexpected EOF")]
    ErrUnexpectedEof,
    #[error("try again")]
    ErrTryAgain,
    #[error("already closed")]
    ErrAlreadyClosed,
    #[error("transaction closed")]
    ErrTransactionClosed,
    #[error("wait_for_result called on non-result transaction")]
    ErrWaitForResultOnNonResultTransaction,
    #[error("too short buffer")]
    ErrShortBuffer,
    #[error("unexpected response type")]
    ErrUnexpectedResponse,
    #[error("parse int: {0}")]
    ParseInt(#[from] ParseIntError),
    #[error("parse addr: {0}")]
    ParseIp(#[from] net::AddrParseError),
    #[error("{0}")]
    Io(#[source] IoError),
    #[error("{0}")]
    Util(#[from] crate::webrtc::util::Error),
    #[error("{0}")]
    Stun(#[from] crate::webrtc::stun::Error),
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Error)]
#[error("io error: {0}")]
pub struct IoError(#[from] pub io::Error);

// Workaround for wanting PartialEq for io::Error.
impl PartialEq for IoError {
    fn eq(&self, other: &Self) -> bool {
        self.0.kind() == other.0.kind()
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(IoError(e))
    }
}

impl From<SystemTimeError> for Error {
    fn from(e: SystemTimeError) -> Self {
        Error::Other(e.to_string())
    }
}
