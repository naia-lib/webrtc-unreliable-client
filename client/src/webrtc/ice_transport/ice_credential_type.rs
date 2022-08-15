use std::fmt;

/// ICECredentialType indicates the type of credentials used to connect to
/// an ICE server.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum RTCIceCredentialType {
    Unspecified,

    /// ICECredential::Password describes username and password based
    /// credentials as described in <https://tools.ietf.org/html/rfc5389>.
    Password,

    /// ICECredential::Oauth describes token based credential as described
    /// in <https://tools.ietf.org/html/rfc7635>.
    /// Not supported in WebRTC 1.0 spec
    Oauth,
}

impl Default for RTCIceCredentialType {
    fn default() -> Self {
        RTCIceCredentialType::Password
    }
}

const ICE_CREDENTIAL_TYPE_PASSWORD_STR: &str = "password";
const ICE_CREDENTIAL_TYPE_OAUTH_STR: &str = "oauth";

impl From<&str> for RTCIceCredentialType {
    fn from(raw: &str) -> Self {
        match raw {
            ICE_CREDENTIAL_TYPE_PASSWORD_STR => RTCIceCredentialType::Password,
            ICE_CREDENTIAL_TYPE_OAUTH_STR => RTCIceCredentialType::Oauth,
            _ => RTCIceCredentialType::Unspecified,
        }
    }
}

impl fmt::Display for RTCIceCredentialType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            RTCIceCredentialType::Password => write!(f, "{}", ICE_CREDENTIAL_TYPE_PASSWORD_STR),
            RTCIceCredentialType::Oauth => write!(f, "{}", ICE_CREDENTIAL_TYPE_OAUTH_STR),
            _ => write!(f, "{}", crate::webrtc::UNSPECIFIED_STR),
        }
    }
}
