use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use crate::webrtc::ice::candidate::Candidate;
use crate::webrtc::ice::state::ConnectionState;
use tokio::sync::{mpsc, Mutex};
use crate::webrtc::util::Conn;

use ice_candidate::RTCIceCandidate;
use ice_candidate_pair::RTCIceCandidatePair;
use ice_gatherer::RTCIceGatherer;
use ice_role::RTCIceRole;

use crate::webrtc::error::{Error, Result};
use crate::webrtc::ice_transport::ice_parameters::RTCIceParameters;
use crate::webrtc::ice_transport::ice_transport_state::RTCIceTransportState;
use crate::webrtc::mux::endpoint::Endpoint;
use crate::webrtc::mux::mux_func::MatchFunc;
use crate::webrtc::mux::{Config, Mux};

pub mod ice_candidate;
pub mod ice_candidate_pair;
pub mod ice_candidate_type;
pub mod ice_connection_state;
pub mod ice_credential_type;
pub mod ice_gatherer;
pub mod ice_gatherer_state;
pub mod ice_gathering_state;
pub mod ice_parameters;
pub mod ice_protocol;
pub mod ice_role;
pub mod ice_server;
pub mod ice_transport_state;

pub type OnConnectionStateChangeHdlrFn = Box<
    dyn (FnMut(RTCIceTransportState) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>)
        + Send
        + Sync,
>;

pub type OnSelectedCandidatePairChangeHdlrFn = Box<
    dyn (FnMut(RTCIceCandidatePair) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>)
        + Send
        + Sync,
>;

#[derive(Default)]
struct ICETransportInternal {
    role: RTCIceRole,
    conn: Option<Arc<dyn Conn + Send + Sync>>, //AgentConn
    mux: Option<Mux>,
    cancel_tx: Option<mpsc::Sender<()>>,
}

/// ICETransport allows an application access to information about the ICE
/// transport over which packets are sent and received.
#[derive(Default)]
pub struct RTCIceTransport {
    gatherer: Arc<RTCIceGatherer>,
    on_connection_state_change_handler: Arc<Mutex<Option<OnConnectionStateChangeHdlrFn>>>,
    on_selected_candidate_pair_change_handler:
        Arc<Mutex<Option<OnSelectedCandidatePairChangeHdlrFn>>>,
    state: Arc<AtomicU8>, // ICETransportState
    internal: Mutex<ICETransportInternal>,
}

impl RTCIceTransport {
    /// creates a new new_icetransport.
    pub fn new(gatherer: Arc<RTCIceGatherer>) -> Self {
        RTCIceTransport {
            state: Arc::new(AtomicU8::new(RTCIceTransportState::New as u8)),
            gatherer,
            ..Default::default()
        }
    }

    /// Start incoming connectivity checks based on its configured role.
    pub async fn start(&self, params: &RTCIceParameters, role: Option<RTCIceRole>) -> Result<()> {
        if self.state() != RTCIceTransportState::New {
            return Err(Error::ErrICETransportNotInNew);
        }

        self.ensure_gatherer().await?;

        if let Some(agent) = self.gatherer.get_agent().await {
            let state = Arc::clone(&self.state);

            let on_connection_state_change_handler =
                Arc::clone(&self.on_connection_state_change_handler);
            agent
                .on_connection_state_change(Box::new(move |ice_state: ConnectionState| {
                    let s = RTCIceTransportState::from(ice_state);
                    let on_connection_state_change_handler_clone =
                        Arc::clone(&on_connection_state_change_handler);
                    state.store(s as u8, Ordering::SeqCst);
                    Box::pin(async move {
                        let mut handler = on_connection_state_change_handler_clone.lock().await;
                        if let Some(f) = &mut *handler {
                            f(s).await;
                        }
                    })
                }))
                .await;

            let on_selected_candidate_pair_change_handler =
                Arc::clone(&self.on_selected_candidate_pair_change_handler);
            agent
                .on_selected_candidate_pair_change(Box::new(
                    move |local: &Arc<dyn Candidate + Send + Sync>,
                          remote: &Arc<dyn Candidate + Send + Sync>| {
                        let on_selected_candidate_pair_change_handler_clone =
                            Arc::clone(&on_selected_candidate_pair_change_handler);
                        let local = RTCIceCandidate::from(local);
                        let remote = RTCIceCandidate::from(remote);
                        Box::pin(async move {
                            let mut handler =
                                on_selected_candidate_pair_change_handler_clone.lock().await;
                            if let Some(f) = &mut *handler {
                                f(RTCIceCandidatePair::new(local, remote)).await;
                            }
                        })
                    },
                ))
                .await;

            let role = if let Some(role) = role {
                role
            } else {
                RTCIceRole::Controlled
            };

            let (cancel_tx, cancel_rx) = mpsc::channel(1);
            {
                let mut internal = self.internal.lock().await;
                internal.role = role;
                internal.cancel_tx = Some(cancel_tx);
            }

            let conn: Arc<dyn Conn + Send + Sync> = match role {
                RTCIceRole::Controlling => {
                    agent
                        .dial(
                            cancel_rx,
                            params.username_fragment.clone(),
                            params.password.clone(),
                        )
                        .await?
                }

                RTCIceRole::Controlled => {
                    agent
                        .accept(
                            cancel_rx,
                            params.username_fragment.clone(),
                            params.password.clone(),
                        )
                        .await?
                }

                _ => return Err(Error::ErrICERoleUnknown),
            };

            let config = Config {
                conn: Arc::clone(&conn),
            };

            {
                let mut internal = self.internal.lock().await;
                internal.conn = Some(conn);
                internal.mux = Some(Mux::new(config));
            }

            Ok(())
        } else {
            Err(Error::ErrICEAgentNotExist)
        }
    }

    /// on_selected_candidate_pair_change sets a handler that is invoked when a new
    /// ICE candidate pair is selected
    pub async fn on_selected_candidate_pair_change(&self, f: OnSelectedCandidatePairChangeHdlrFn) {
        let mut on_selected_candidate_pair_change_handler =
            self.on_selected_candidate_pair_change_handler.lock().await;
        *on_selected_candidate_pair_change_handler = Some(f);
    }

    /// on_connection_state_change sets a handler that is fired when the ICE
    /// connection state changes.
    pub async fn on_connection_state_change(&self, f: OnConnectionStateChangeHdlrFn) {
        let mut on_connection_state_change_handler =
            self.on_connection_state_change_handler.lock().await;
        *on_connection_state_change_handler = Some(f);
    }

    /// Role indicates the current role of the ICE transport.
    pub async fn role(&self) -> RTCIceRole {
        let internal = self.internal.lock().await;
        internal.role
    }

    /// set_remote_candidates sets the sequence of candidates associated with the remote ICETransport.
    pub async fn set_remote_candidates(&self, remote_candidates: &[RTCIceCandidate]) -> Result<()> {
        self.ensure_gatherer().await?;

        if let Some(agent) = self.gatherer.get_agent().await {
            for rc in remote_candidates {
                let c: Arc<dyn Candidate + Send + Sync> = Arc::new(rc.to_ice().await?);
                agent.add_remote_candidate(&c).await?;
            }
            Ok(())
        } else {
            Err(Error::ErrICEAgentNotExist)
        }
    }

    /// adds a candidate associated with the remote ICETransport.
    pub async fn add_remote_candidate(
        &self,
        remote_candidate: Option<RTCIceCandidate>,
    ) -> Result<()> {
        self.ensure_gatherer().await?;

        if let Some(agent) = self.gatherer.get_agent().await {
            if let Some(r) = remote_candidate {
                let c: Arc<dyn Candidate + Send + Sync> = Arc::new(r.to_ice().await?);
                agent.add_remote_candidate(&c).await?;
            }

            Ok(())
        } else {
            Err(Error::ErrICEAgentNotExist)
        }
    }

    /// State returns the current ice transport state.
    pub fn state(&self) -> RTCIceTransportState {
        RTCIceTransportState::from(self.state.load(Ordering::SeqCst))
    }

    pub fn set_state(&self, s: RTCIceTransportState) {
        self.state.store(s as u8, Ordering::SeqCst)
    }

    pub async fn new_endpoint(&self, f: MatchFunc) -> Option<Arc<Endpoint>> {
        let internal = self.internal.lock().await;
        if let Some(mux) = &internal.mux {
            Some(mux.new_endpoint(f).await)
        } else {
            None
        }
    }

    pub async fn ensure_gatherer(&self) -> Result<()> {
        if self.gatherer.get_agent().await.is_none() {
            self.gatherer.create_agent().await
        } else {
            Ok(())
        }
    }
}
