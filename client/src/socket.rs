use std::{sync::Arc, time::Duration};

use anyhow::{Error, Result};
use bytes::Bytes;
use log::warn;
use reqwest::{Client as HttpClient, Response};
use tinyjson::JsonValue;
use tokio::{
    sync::{mpsc, oneshot},
    time::sleep,
};

use crate::webrtc::{
    data_channel::internal::data_channel::DataChannel,
    peer_connection::{sdp::session_description::RTCSessionDescription, RTCPeerConnection},
};

use super::addr_cell::AddrCell;

const MESSAGE_SIZE: usize = 1500;

pub struct Socket {
    addr_cell: AddrCell,
    to_server_receiver: mpsc::UnboundedReceiver<Box<[u8]>>,
    to_server_disconnect_receiver: mpsc::Receiver<()>,
    to_client_sender: mpsc::UnboundedSender<Box<[u8]>>,
    to_client_id_sender: oneshot::Sender<Result<String, u16>>,
}

pub struct SocketIo {
    pub addr_cell: AddrCell,
    pub to_server_sender: mpsc::UnboundedSender<Box<[u8]>>,
    pub to_server_disconnect_sender: mpsc::Sender<()>,
    pub to_client_receiver: mpsc::UnboundedReceiver<Box<[u8]>>,
    pub to_client_id_receiver: oneshot::Receiver<Result<String, u16>>,
}

impl Socket {
    pub fn new() -> (Self, SocketIo) {
        let addr_cell = AddrCell::default();
        let (to_server_sender, to_server_receiver) = mpsc::unbounded_channel();
        let (to_server_disconnect_sender, to_server_disconnect_receiver) = mpsc::channel(1);
        let (to_client_sender, to_client_receiver) = mpsc::unbounded_channel();
        let (to_client_id_sender, to_client_id_receiver) = oneshot::channel();

        (
            Self {
                addr_cell: addr_cell.clone(),
                to_server_receiver,
                to_server_disconnect_receiver,
                to_client_sender,
                to_client_id_sender,
            },
            SocketIo {
                addr_cell,
                to_server_sender,
                to_server_disconnect_sender,
                to_client_receiver,
                to_client_id_receiver,
            },
        )
    }

    pub async fn connect(
        self,
        server_url: &str,
        auth_bytes_opt: Option<Vec<u8>>,
        auth_headers_opt: Option<Vec<(String, String)>>,
    ) {
        let Self {
            addr_cell,
            to_server_receiver,
            to_server_disconnect_receiver,
            to_client_sender,
            to_client_id_sender,
        } = self;

        // create a new RTCPeerConnection
        let peer_connection = RTCPeerConnection::new().await;

        let label = "data";
        let protocol = "";

        // create a datachannel with label 'data'
        let data_channel = peer_connection
            .create_data_channel(label, protocol)
            .await
            .expect("cannot create data channel");

        // datachannel on_error callback
        data_channel
            .on_error(Box::new(move |error| {
                println!("data channel error: {:?}", error);
                Box::pin(async {})
            }))
            .await;

        // datachannel on_open callback
        let peer_connection_ref = Arc::clone(&peer_connection);
        let data_channel_ref = Arc::clone(&data_channel);
        data_channel
            .on_open(Box::new(move || {
                let peer_connection_ref_2 = Arc::clone(&peer_connection_ref);
                let data_channel_ref_2 = Arc::clone(&data_channel_ref);
                Box::pin(async move {
                    let detached_data_channel = data_channel_ref_2
                        .detach()
                        .await
                        .expect("data channel detach got error");

                    // Handle reading from the data channel
                    let peer_connection_ref_3 = Arc::clone(&peer_connection_ref_2);
                    let peer_connection_ref_4 = Arc::clone(&peer_connection_ref_2);

                    let detached_data_channel_1 = Arc::clone(&detached_data_channel);
                    let detached_data_channel_2 = Arc::clone(&detached_data_channel);
                    tokio::spawn(async move {
                        let _loop_result =
                            read_loop(detached_data_channel_1, to_client_sender).await;

                        // do nothing with result, just close thread
                        peer_connection_ref_3.internal.close().await;
                    });

                    // Handle writing to the data channel
                    tokio::spawn(async move {
                        let detached_data_channel_3 = Arc::clone(&detached_data_channel_2);
                        let _loop_result = write_loop(
                            detached_data_channel_3,
                            to_server_receiver,
                            to_server_disconnect_receiver,
                        )
                        .await;

                        // do nothing with result, just close thread
                        detached_data_channel_2.close().await;

                        peer_connection_ref_4.internal.close().await;
                    });
                })
            }))
            .await;

        // create an offer to send to the server
        let offer = peer_connection
            .create_offer()
            .await
            .expect("cannot create offer");

        // sets the LocalDescription, and starts our UDP listeners
        peer_connection
            .set_local_description(offer)
            .await
            .expect("cannot set local description");

        // send a request to server to initiate connection (signaling, essentially)
        let http_client = HttpClient::new();

        let sdp = peer_connection.local_description().await.unwrap().sdp;

        let sdp_len = sdp.len();

        // wait to receive a response from server
        let response: Response = loop {
            let mut request = http_client
                .post(server_url)
                .header("Content-Length", sdp_len)
                .body(sdp.clone());
            if let Some(auth_bytes) = auth_bytes_opt.clone() {
                let base64_encoded = base64::encode(auth_bytes);
                request = request.header("Authorization", &base64_encoded);
            }
            if let Some(auth_headers) = auth_headers_opt.clone() {
                for (key, value) in auth_headers {
                    request = request.header(key, value);
                }
            }

            match request.send().await {
                Ok(resp) => {
                    break resp;
                }
                Err(err) => {
                    warn!("Could not send request, original error: {:?}", err);
                    sleep(Duration::from_secs(1)).await;
                }
            };
        };

        if !response.status().is_success() {
            let status_code = response.status().as_u16();
            to_client_id_sender.send(Err(status_code)).unwrap();
            return;
        }

        // get the body of the response as a string
        let response_string = match response.text().await {
            Ok(response_string) => response_string,
            Err(_err) => {
                // error reading response?
                to_client_id_sender.send(Err(500)).unwrap();
                return;
            }
        };

        // parse session from server response
        let session_response_result = get_session_response(response_string.as_str());
        let session_response = match session_response_result {
            Ok(session_response) => session_response,
            Err(_err) => {
                // parsing error?
                to_client_id_sender.send(Err(500)).unwrap();
                return;
            }
        };

        // send the id token to the client
        // info!("Sending id token to client: {:?}", auth_header);
        if let Err(err) = to_client_id_sender.send(Ok(session_response.id_token)) {
            warn!("Could not send id token to client: {:?}. Did the IdentityReceiver returned from Socket::connect() de-allocate?", err);
            return;
        }

        // apply the server's response as the remote description
        let session_description =
            RTCSessionDescription::answer(session_response.answer.sdp).unwrap();

        peer_connection
            .set_remote_description(session_description)
            .await
            .expect("cannot set remote description");

        addr_cell
            .receive_candidate(session_response.candidate.candidate.as_str())
            .await;

        // add ice candidate to connection
        if let Err(error) = peer_connection
            .add_ice_candidate(session_response.candidate.candidate)
            .await
        {
            panic!("Error during add_ice_candidate: {:?}", error);
        }
    }
}

// read_loop shows how to read from the datachannel directly
async fn read_loop(
    data_channel: Arc<DataChannel>,
    to_client_sender: mpsc::UnboundedSender<Box<[u8]>>,
) -> Result<()> {
    let mut buffer = vec![0u8; MESSAGE_SIZE];
    loop {
        let message_length = match data_channel.read(&mut buffer).await {
            Ok(length) => length,
            Err(_err) => {
                //println!("Datachannel closed; Exit the read_loop: {}", err);
                return Ok(());
            }
        };

        match to_client_sender.send(buffer[..message_length].into()) {
            Ok(_) => {}
            Err(e) => {
                return Err(Error::new(e));
            }
        }
    }
}

// write_loop shows how to write to the datachannel directly
async fn write_loop(
    data_channel: Arc<DataChannel>,
    mut to_server_receiver: mpsc::UnboundedReceiver<Box<[u8]>>,
    mut to_server_disconnect_receiver: mpsc::Receiver<()>,
) -> Result<()> {
    loop {
        tokio::select! {
            _ = to_server_disconnect_receiver.recv() => {
                return Ok(());
            }
            result = to_server_receiver.recv() => {
                if let Some(mut write_message) = result {
                    let taken_message = std::mem::take(&mut write_message);
                    let message_bytes = Bytes::from(taken_message);
                    if let Err(e) = data_channel.write(&message_bytes).await {
                        return Err(Error::new(e));
                    }
                } else {
                    return Ok(());
                }
            }
        }
    }
}

#[derive(Clone)]
pub(crate) struct SessionAnswer {
    pub(crate) sdp: String,
}

pub(crate) struct SessionCandidate {
    pub(crate) candidate: String,
}

pub(crate) struct JsSessionResponse {
    pub(crate) id_token: String,
    pub(crate) answer: SessionAnswer,
    pub(crate) candidate: SessionCandidate,
}

fn get_session_response(input: &str) -> Result<JsSessionResponse, String> {
    // info!("{}", input);
    let Ok(json_obj): Result<JsonValue, _> = input.parse() else {
        return Err("Could not parse response JSON".to_string());
    };

    let sdp_opt: Option<&String> = json_obj["sdp"]["answer"]["sdp"].get();
    let sdp: String = sdp_opt.unwrap().clone();

    let candidate_opt: Option<&String> = json_obj["sdp"]["candidate"]["candidate"].get();
    let candidate: String = candidate_opt.unwrap().clone();

    let id_token_opt: Option<&String> = json_obj["id"].get();
    let id_token: String = id_token_opt.unwrap().clone();

    Ok(JsSessionResponse {
        id_token,
        answer: SessionAnswer { sdp },
        candidate: SessionCandidate { candidate },
    })
}
