use crate::webrtc::error::{Error, Result};
use crate::webrtc::ice_transport::ice_credential_type::RTCIceCredentialType;

/// ICEServer describes a single STUN and TURN server that can be used by
/// the ICEAgent to establish a connection with a peer.
#[derive(Default, Debug, Clone)]
pub(crate) struct RTCIceServer {
    pub(crate) urls: Vec<String>,
    pub(crate) username: String,
    pub(crate) credential: String,
    pub(crate) credential_type: RTCIceCredentialType,
}

impl RTCIceServer {
    pub(crate) fn parse_url(&self, url_str: &str) -> Result<crate::webrtc::ice::url::Url> {
        Ok(crate::webrtc::ice::url::Url::parse_url(url_str)?)
    }

    pub(crate) fn validate(&self) -> Result<()> {
        self.urls()?;
        Ok(())
    }

    pub(crate) fn urls(&self) -> Result<Vec<crate::webrtc::ice::url::Url>> {
        let mut urls = vec![];

        for url_str in &self.urls {
            let mut url = self.parse_url(url_str)?;
            if url.scheme == crate::webrtc::ice::url::SchemeType::Turn || url.scheme == crate::webrtc::ice::url::SchemeType::Turns
            {
                // https://www.w3.org/TR/webrtc/#set-the-configuration (step #11.3.2)
                if self.username.is_empty() || self.credential.is_empty() {
                    return Err(Error::ErrNoTurnCredentials);
                }
                url.username = self.username.clone();

                match self.credential_type {
                    RTCIceCredentialType::Password => {
                        // https://www.w3.org/TR/webrtc/#set-the-configuration (step #11.3.3)
                        url.password = self.credential.clone();
                    }
                    RTCIceCredentialType::Oauth => {
                        // https://www.w3.org/TR/webrtc/#set-the-configuration (step #11.3.4)
                        /*if _, ok: = s.Credential.(OAuthCredential); !ok {
                                return nil,
                                &rtcerr.InvalidAccessError{Err: ErrTurnCredentials
                            }
                        }*/
                    }
                    _ => return Err(Error::ErrTurnCredentials),
                };
            }

            urls.push(url);
        }

        Ok(urls)
    }
}