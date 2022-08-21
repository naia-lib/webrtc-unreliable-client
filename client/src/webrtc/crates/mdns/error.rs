use thiserror::Error;

use std::io;
use std::net;
use std::string::FromUtf8Error;

pub(crate) type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error, PartialEq)]
#[non_exhaustive]
pub(crate) enum Error {
    #[error("insufficient data for base length type")]
    ErrBaseLen,
    #[error("insufficient data for calculated length type")]
    ErrCalcLen,
    #[error("segment prefix is reserved")]
    ErrReserved,
    #[error("too many pointers (>10)")]
    ErrTooManyPtr,
    #[error("invalid pointer")]
    ErrInvalidPtr,
    #[error("segment length too long")]
    ErrSegTooLong,
    #[error("zero length segment")]
    ErrZeroSegLen,
    #[error("name is not in canonical format (it must end with a .)")]
    ErrNonCanonicalName,
    #[error("character string exceeds maximum length (255)")]
    ErrStringTooLong,
    #[error("compressed name in SRV resource data")]
    ErrCompressedSrv,
    #[error("{0}")]
    Io(#[source] IoError),
    #[error("utf-8 error: {0}")]
    Utf8(#[from] FromUtf8Error),
    #[error("parse addr: {0}")]
    ParseIp(#[from] net::AddrParseError),
}

#[derive(Debug, Error)]
#[error("io error: {0}")]
pub(crate) struct IoError(#[from] pub(crate) io::Error);

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
