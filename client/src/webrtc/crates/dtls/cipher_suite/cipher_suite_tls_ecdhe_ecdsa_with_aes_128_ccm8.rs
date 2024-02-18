use super::*;
use crate::webrtc::dtls::cipher_suite::cipher_suite_aes_128_ccm::CipherSuiteAes128Ccm;
use crate::webrtc::dtls::crypto::crypto_ccm::CryptoCcmTagLen;

pub(crate) fn new_cipher_suite_tls_ecdhe_ecdsa_with_aes_128_ccm8() -> CipherSuiteAes128Ccm {
    CipherSuiteAes128Ccm::new(
        CipherSuiteId::Tls_Ecdhe_Ecdsa_With_Aes_128_Ccm_8,
        false,
        CryptoCcmTagLen::CryptoCcm8TagLength,
    )
}
