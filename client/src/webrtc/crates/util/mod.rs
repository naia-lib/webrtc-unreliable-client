use async_trait::async_trait;
use thiserror::Error;
use std::io;

pub(crate) mod fixed_big_int;
pub(crate) mod replay_detector;

/// KeyingMaterialExporter to extract keying material.
///
/// This trait sits here to avoid getting a direct dependency between
/// the dtls and srtp crates.
#[async_trait]
pub(crate) trait KeyingMaterialExporter {
    async fn export_keying_material(
        &self,
        label: &str,
        context: &[u8],
        length: usize,
    ) -> std::result::Result<Vec<u8>, KeyingMaterialExporterError>;
}

/// Possible errors while exporting keying material.
///
/// These errors might have been more logically kept in the dtls
/// crate, but that would have required a direct depdency between
/// srtp and dtls.
#[derive(Debug, Error, PartialEq)]
#[non_exhaustive]
pub(crate) enum KeyingMaterialExporterError {
    #[error("tls handshake is in progress")]
    HandshakeInProgress,
    #[error("context is not supported for export_keying_material")]
    ContextUnsupported,
    #[error("export_keying_material can not be used with a reserved label")]
    ReservedExportKeyingMaterial,
    #[error("no cipher suite for export_keying_material")]
    CipherSuiteUnset,
    #[error("export_keying_material io: {0}")]
    Io(#[source] error::IoError),
    #[error("export_keying_material hash: {0}")]
    Hash(String),
}

impl From<io::Error> for KeyingMaterialExporterError {
    fn from(e: io::Error) -> Self {
        KeyingMaterialExporterError::Io(error::IoError(e))
    }
}

pub(crate) mod buffer;
pub(crate) mod conn;
pub(crate) mod ifaces;
pub(crate) mod vnet;
pub(crate) mod marshal;
pub(crate) use crate::webrtc::util::buffer::Buffer;
pub(crate) use crate::webrtc::util::conn::Conn;

mod error;
pub(crate) use error::Error;
