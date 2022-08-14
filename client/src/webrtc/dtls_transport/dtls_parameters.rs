use super::dtls_fingerprint::*;
use super::dtls_role::*;

use serde::{Deserialize, Serialize};

/// DTLSParameters holds information relating to DTLS configuration.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DTLSParameters {
    pub(crate) role: DTLSRole,
    pub(crate) fingerprints: Vec<RTCDtlsFingerprint>,
}
