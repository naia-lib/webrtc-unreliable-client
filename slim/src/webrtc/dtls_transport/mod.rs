
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use bytes::Bytes;
use dtls::config::ClientAuthType;
use dtls::conn::DTLSConn;
use tokio::sync::Mutex;
use util::Conn;

use dtls_role::*;

use crate::webrtc::api::setting_engine::SettingEngine;
use crate::webrtc::dtls_transport::dtls_parameters::DTLSParameters;
use crate::webrtc::dtls_transport::dtls_transport_state::RTCDtlsTransportState;
use crate::webrtc::error::{flatten_errs, Error, Result};
use crate::webrtc::ice_transport::ice_role::RTCIceRole;
use crate::webrtc::ice_transport::ice_transport_state::RTCIceTransportState;
use crate::webrtc::ice_transport::RTCIceTransport;
use crate::webrtc::mux::mux_func::match_dtls;
use crate::webrtc::peer_connection::certificate::RTCCertificate;

pub mod dtls_fingerprint;
pub mod dtls_parameters;
pub mod dtls_role;
pub mod dtls_transport_state;

pub type OnDTLSTransportStateChangeHdlrFn = Box<
    dyn (FnMut(RTCDtlsTransportState) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>)
        + Send
        + Sync,
>;

/// DTLSTransport allows an application access to information about the DTLS
/// transport over which RTP and RTCP packets are sent and received by
/// RTPSender and RTPReceiver, as well other data such as SCTP packets sent
/// and received by data channels.
#[derive(Default)]
pub struct RTCDtlsTransport {
    pub(crate) ice_transport: Arc<RTCIceTransport>,
    pub(crate) certificates: Vec<RTCCertificate>,
    pub(crate) setting_engine: Arc<SettingEngine>,

    pub(crate) remote_parameters: Mutex<DTLSParameters>,
    pub(crate) remote_certificate: Mutex<Bytes>,
    pub(crate) state: AtomicU8, //DTLSTransportState,
    pub(crate) on_state_change_handler: Arc<Mutex<Option<OnDTLSTransportStateChangeHdlrFn>>>,
    pub(crate) conn: Mutex<Option<Arc<DTLSConn>>>,
}

impl RTCDtlsTransport {
    pub(crate) fn new(
        ice_transport: Arc<RTCIceTransport>,
        certificates: Vec<RTCCertificate>,
        setting_engine: Arc<SettingEngine>,
    ) -> Self {
        RTCDtlsTransport {
            ice_transport,
            certificates,
            setting_engine,
            state: AtomicU8::new(RTCDtlsTransportState::New as u8),
            ..Default::default()
        }
    }

    pub(crate) async fn conn(&self) -> Option<Arc<DTLSConn>> {
        let conn = self.conn.lock().await;
        conn.clone()
    }

    /// returns the currently-configured ICETransport or None
    /// if one has not been configured
    pub fn ice_transport(&self) -> &RTCIceTransport {
        &self.ice_transport
    }

    /// state_change requires the caller holds the lock
    async fn state_change(&self, state: RTCDtlsTransportState) {
        self.state.store(state as u8, Ordering::SeqCst);
        let mut handler = self.on_state_change_handler.lock().await;
        if let Some(f) = &mut *handler {
            f(state).await;
        }
    }

    /// on_state_change sets a handler that is fired when the DTLS
    /// connection state changes.
    pub async fn on_state_change(&self, f: OnDTLSTransportStateChangeHdlrFn) {
        let mut on_state_change_handler = self.on_state_change_handler.lock().await;
        *on_state_change_handler = Some(f);
    }

    /// state returns the current dtls_transport transport state.
    pub fn state(&self) -> RTCDtlsTransportState {
        self.state.load(Ordering::SeqCst).into()
    }

    /// get_local_parameters returns the DTLS parameters of the local DTLSTransport upon construction.
    pub fn get_local_parameters(&self) -> Result<DTLSParameters> {
        let mut fingerprints = vec![];

        for c in &self.certificates {
            fingerprints.extend(c.get_fingerprints()?);
        }

        Ok(DTLSParameters {
            role: DTLSRole::Auto, // always returns the default role
            fingerprints,
        })
    }

    /// get_remote_certificate returns the certificate chain in use by the remote side
    /// returns an empty list prior to selection of the remote certificate
    pub async fn get_remote_certificate(&self) -> Bytes {
        let remote_certificate = self.remote_certificate.lock().await;
        remote_certificate.clone()
    }

    pub(crate) async fn role(&self) -> DTLSRole {
        // If remote has an explicit role use the inverse
        {
            let remote_parameters = self.remote_parameters.lock().await;
            match remote_parameters.role {
                DTLSRole::Client => return DTLSRole::Server,
                DTLSRole::Server => return DTLSRole::Client,
                _ => {}
            };
        }

        // If SettingEngine has an explicit role
        match self.setting_engine.answering_dtls_role {
            DTLSRole::Server => return DTLSRole::Server,
            DTLSRole::Client => return DTLSRole::Client,
            _ => {}
        };

        // Remote was auto and no explicit role was configured via SettingEngine
        if self.ice_transport.role().await == RTCIceRole::Controlling {
            return DTLSRole::Server;
        }

        DEFAULT_DTLS_ROLE_ANSWER
    }

    async fn prepare_transport(
        &self,
        remote_parameters: DTLSParameters,
    ) -> Result<(DTLSRole, dtls::config::Config)> {
        self.ensure_ice_conn()?;

        if self.state() != RTCDtlsTransportState::New {
            return Err(Error::ErrInvalidDTLSStart);
        }

        {
            let mut rp = self.remote_parameters.lock().await;
            *rp = remote_parameters;
        }

        let certificate = if let Some(cert) = self.certificates.first() {
            cert.certificate.clone()
        } else {
            return Err(Error::ErrNonCertificate);
        };
        self.state_change(RTCDtlsTransportState::Connecting).await;

        Ok((
            self.role().await,
            dtls::config::Config {
                certificates: vec![certificate],
                srtp_protection_profiles: vec![],
                client_auth: ClientAuthType::RequireAnyClientCert,
                insecure_skip_verify: true,
                ..Default::default()
            },
        ))
    }

    /// start DTLS transport negotiation with the parameters of the remote DTLS transport
    pub async fn start(&self, remote_parameters: DTLSParameters) -> Result<()> {
        let dtls_conn_result = if let Some(dtls_endpoint) =
            self.ice_transport.new_endpoint(Box::new(match_dtls)).await
        {
            let (_, mut dtls_config) = self.prepare_transport(remote_parameters).await?;
            if self.setting_engine.replay_protection.dtls != 0 {
                dtls_config.replay_protection_window = self.setting_engine.replay_protection.dtls;
            }

            // Connect as DTLS Client/Server, function is blocking and we
            // must not hold the DTLSTransport lock
            dtls::conn::DTLSConn::new(
                dtls_endpoint as Arc<dyn Conn + Send + Sync>,
                dtls_config,
                true,
                None,
            )
                .await
        } else {
            Err(dtls::Error::Other(
                "ice_transport.new_endpoint failed".to_owned(),
            ))
        };

        let dtls_conn = match dtls_conn_result {
            Ok(dtls_conn) => dtls_conn,
            Err(err) => {
                self.state_change(RTCDtlsTransportState::Failed).await;
                return Err(err.into());
            }
        };

        {
            let mut conn = self.conn.lock().await;
            *conn = Some(Arc::new(dtls_conn));
        }
        self.state_change(RTCDtlsTransportState::Connected).await;

        Ok(())
    }

    /// stops and closes the DTLSTransport object.
    pub async fn stop(&self) -> Result<()> {
        // Try closing everything and collect the errors
        let mut close_errs: Vec<Error> = vec![];

        if let Some(conn) = self.conn().await {
            // dtls_transport connection may be closed on sctp close.
            match conn.close().await {
                Ok(_) => {}
                Err(err) => {
                    if err.to_string() != dtls::Error::ErrConnClosed.to_string() {
                        close_errs.push(err.into());
                    }
                }
            }
        }

        self.state_change(RTCDtlsTransportState::Closed).await;

        flatten_errs(close_errs)
    }

    pub(crate) fn ensure_ice_conn(&self) -> Result<()> {
        if self.ice_transport.state() == RTCIceTransportState::New {
            Err(Error::ErrICEConnectionNotStarted)
        } else {
            Ok(())
        }
    }
}
