use super::cipher_suite::*;
use super::conn::*;
use super::curve::named_curve::*;
use super::extension::extension_use_srtp::SrtpProtectionProfile;
use super::handshake::handshake_random::*;
use super::prf::*;

use async_trait::async_trait;
use std::io::BufWriter;
use std::marker::{Send, Sync};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::webrtc::util::KeyingMaterialExporter;
use crate::webrtc::util::KeyingMaterialExporterError;

// State holds the dtls connection state and implements both encoding.BinaryMarshaler and encoding.BinaryUnmarshaler
pub struct State {
    pub(crate) local_epoch: Arc<AtomicU16>,
    pub(crate) remote_epoch: Arc<AtomicU16>,
    pub(crate) local_sequence_number: Arc<Mutex<Vec<u64>>>, // uint48
    pub(crate) local_random: HandshakeRandom,
    pub(crate) remote_random: HandshakeRandom,
    pub(crate) master_secret: Vec<u8>,
    pub(crate) cipher_suite: Arc<Mutex<Option<Box<dyn CipherSuite + Send + Sync>>>>, // nil if a cipher_suite hasn't been chosen

    pub(crate) srtp_protection_profile: SrtpProtectionProfile, // Negotiated srtp_protection_profile
    pub peer_certificates: Vec<Vec<u8>>,
    pub identity_hint: Vec<u8>,

    pub(crate) is_client: bool,

    pub(crate) pre_master_secret: Vec<u8>,
    pub(crate) extended_master_secret: bool,

    pub(crate) named_curve: NamedCurve,
    pub(crate) local_keypair: Option<NamedCurveKeypair>,
    pub(crate) cookie: Vec<u8>,
    pub(crate) handshake_send_sequence: isize,
    pub(crate) handshake_recv_sequence: isize,
    pub(crate) server_name: String,
    pub(crate) remote_requested_certificate: bool, // Did we get a CertificateRequest
    pub(crate) local_certificates_verify: Vec<u8>, // cache CertificateVerify
    pub(crate) local_verify_data: Vec<u8>,         // cached VerifyData
    pub(crate) local_key_signature: Vec<u8>,       // cached keySignature
    pub(crate) peer_certificates_verified: bool,
    //pub(crate) replay_detector: Vec<Box<dyn ReplayDetector + Send + Sync>>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct SerializedState {
    local_epoch: u16,
    remote_epoch: u16,
    local_random: [u8; HANDSHAKE_RANDOM_LENGTH],
    remote_random: [u8; HANDSHAKE_RANDOM_LENGTH],
    cipher_suite_id: u16,
    master_secret: Vec<u8>,
    sequence_number: u64,
    srtp_protection_profile: u16,
    peer_certificates: Vec<Vec<u8>>,
    identity_hint: Vec<u8>,
    is_client: bool,
}

impl Default for State {
    fn default() -> Self {
        State {
            local_epoch: Arc::new(AtomicU16::new(0)),
            remote_epoch: Arc::new(AtomicU16::new(0)),
            local_sequence_number: Arc::new(Mutex::new(vec![])),
            local_random: HandshakeRandom::default(),
            remote_random: HandshakeRandom::default(),
            master_secret: vec![],
            cipher_suite: Arc::new(Mutex::new(None)), // nil if a cipher_suite hasn't been chosen

            srtp_protection_profile: SrtpProtectionProfile::Unsupported, // Negotiated srtp_protection_profile
            peer_certificates: vec![],
            identity_hint: vec![],

            is_client: false,

            pre_master_secret: vec![],
            extended_master_secret: false,

            named_curve: NamedCurve::Unsupported,
            local_keypair: None,
            cookie: vec![],
            handshake_send_sequence: 0,
            handshake_recv_sequence: 0,
            server_name: "".to_string(),
            remote_requested_certificate: false, // Did we get a CertificateRequest
            local_certificates_verify: vec![],   // cache CertificateVerify
            local_verify_data: vec![],           // cached VerifyData
            local_key_signature: vec![],         // cached keySignature
            peer_certificates_verified: false,
            //replay_detector: vec![],
        }
    }
}

#[async_trait]
impl KeyingMaterialExporter for State {
    /// export_keying_material returns length bytes of exported key material in a new
    /// slice as defined in RFC 5705.
    /// This allows protocols to use DTLS for key establishment, but
    /// then use some of the keying material for their own purposes
    async fn export_keying_material(
        &self,
        label: &str,
        context: &[u8],
        length: usize,
    ) -> std::result::Result<Vec<u8>, KeyingMaterialExporterError> {
        use KeyingMaterialExporterError::*;

        if self.local_epoch.load(Ordering::SeqCst) == 0 {
            return Err(HandshakeInProgress);
        } else if !context.is_empty() {
            return Err(ContextUnsupported);
        } else if INVALID_KEYING_LABELS.contains_key(label) {
            return Err(ReservedExportKeyingMaterial);
        }

        let mut local_random = vec![];
        {
            let mut writer = BufWriter::<&mut Vec<u8>>::new(local_random.as_mut());
            self.local_random.marshal(&mut writer)?;
        }
        let mut remote_random = vec![];
        {
            let mut writer = BufWriter::<&mut Vec<u8>>::new(remote_random.as_mut());
            self.remote_random.marshal(&mut writer)?;
        }

        let mut seed = label.as_bytes().to_vec();
        if self.is_client {
            seed.extend_from_slice(&local_random);
            seed.extend_from_slice(&remote_random);
        } else {
            seed.extend_from_slice(&remote_random);
            seed.extend_from_slice(&local_random);
        }

        let cipher_suite = self.cipher_suite.lock().await;
        if let Some(cipher_suite) = &*cipher_suite {
            match prf_p_hash(&self.master_secret, &seed, length, cipher_suite.hash_func()) {
                Ok(v) => Ok(v),
                Err(err) => Err(Hash(err.to_string())),
            }
        } else {
            Err(CipherSuiteUnset)
        }
    }
}
