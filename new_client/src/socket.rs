use std::sync::Arc;

use anyhow::{Error, Result};
use bytes::Bytes;
use reqwest::Client as HttpClient;
use tinyjson::JsonValue;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};

use webrtc::{
    api::API,
    ice_transport::ice_candidate::RTCIceCandidateInit,
    peer_connection::{
        sdp::{sdp_type::RTCSdpType, session_description::RTCSessionDescription},
    },
};

use super::addr_cell::AddrCell;

const MESSAGE_SIZE: usize = 1500;
const CLIENT_CHANNEL_SIZE: usize = 8;

pub struct Socket;

impl Socket {
    pub async fn connect(server_url: &str) -> (AddrCell, mpsc::Sender<Box<[u8]>>, mpsc::Receiver<Box<[u8]>>) {

        let (to_server_sender, to_server_receiver) = mpsc::channel::<Box<[u8]>>(CLIENT_CHANNEL_SIZE);
        let (to_client_sender, to_client_receiver) = mpsc::channel::<Box<[u8]>>(CLIENT_CHANNEL_SIZE);

        let addr_cell = AddrCell::default();

        // create a new RTCPeerConnection
        let peer_connection = API::new_peer_connection().await;

        // create a config for our new datachannel


        // create a datachannel with label 'data'
        let data_channel = peer_connection
            .create_data_channel()
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
        let data_channel_ref = Arc::clone(&data_channel);
        data_channel
            .on_open(Box::new(move || {
                let data_channel_ref_2 = Arc::clone(&data_channel_ref);
                Box::pin(async move {
                    let detached_data_channel = data_channel_ref_2
                        .detach()
                        .await
                        .expect("data channel detach got error");

                    // Handle reading from the data channel
                    let detached_data_channel_1 = Arc::clone(&detached_data_channel);
                    let detached_data_channel_2 = Arc::clone(&detached_data_channel);
                    tokio::spawn(async move {
                        read_loop(detached_data_channel_1, to_client_sender)
                            .await
                            .expect("error in read_loop!");
                    });

                    // Handle writing to the data channel
                    tokio::spawn(async move {
                        write_loop(detached_data_channel_2, to_server_receiver)
                            .await
                            .expect("error in write_loop!");
                    });
                })
            }))
            .await;

        // create an offer to send to the server
        let offer = peer_connection.create_offer().await.expect("cannot create offer");

        // sets the LocalDescription, and starts our UDP listeners
        peer_connection.set_local_description(offer).await.expect("cannot set local description");

        // send a request to server to initiate connection (signaling, essentially)
        let http_client = HttpClient::new();

        let sdp = peer_connection.local_description().await.unwrap().sdp;

        let request = http_client
            .post(server_url)
            .header("Content-Length", sdp.len())
            .body(sdp);

        // wait to receive a response from server
        let response = match request.send().await {
            Ok(resp) => resp,
            Err(err) => {
                panic!("Could not send request, original error: {:?}", err);
            }
        };
        let response_string = response.text().await.unwrap();

        // parse session from server response
        let session_response: JsSessionResponse = get_session_response(response_string.as_str());

        // apply the server's response as the remote description
        let mut session_description = RTCSessionDescription::default();
        session_description.sdp_type = RTCSdpType::Answer;
        session_description.sdp = session_response.answer.sdp;
        peer_connection
            .set_remote_description(session_description)
            .await
            .expect("cannot set remote description");

        addr_cell
            .receive_candidate(session_response.candidate.candidate.as_str())
            .await;

        // create ice candidate
        let ice_candidate = RTCIceCandidateInit {
            candidate: session_response.candidate.candidate,
            sdp_mid: Some(session_response.candidate.sdp_mid),
            sdp_mline_index: Some(session_response.candidate.sdp_m_line_index),
            ..Default::default()
        };
        // add ice candidate to connection
        if let Err(error) = peer_connection.add_ice_candidate(ice_candidate).await {
            panic!("Error during add_ice_candidate: {:?}", error);
        }

        (addr_cell, to_server_sender, to_client_receiver)
    }
}

// read_loop shows how to read from the datachannel directly
async fn read_loop(
    data_channel: Arc<webrtc::data::data_channel::DataChannel>,
    to_client_sender: Sender<Box<[u8]>>,
) -> Result<()> {
    let mut buffer = vec![0u8; MESSAGE_SIZE];
    loop {
        let message_length = match data_channel.read(&mut buffer).await {
            Ok(length) => length,
            Err(err) => {
                println!("Datachannel closed; Exit the read_loop: {}", err);
                return Ok(());
            }
        };

        match to_client_sender.send(buffer[..message_length].into()).await {
            Ok(_) => {},
            Err(e) => {
                return Err(Error::new(e));
            }
        }
    }
}

// write_loop shows how to write to the datachannel directly
async fn write_loop(
    data_channel: Arc<webrtc::data::data_channel::DataChannel>,
    mut to_server_receiver: Receiver<Box<[u8]>>,
) -> Result<()> {
    loop {
        if let Some(write_message) = to_server_receiver.recv().await {
            match data_channel.write(&Bytes::from(write_message)).await {
                Ok(_) => {},
                Err(e) => {
                    return Err(Error::new(e));
                }
            }
        } else {
            return Ok(());
        }
    }
}

#[derive(Clone)]
pub struct SessionAnswer {
    pub sdp: String,
    pub type_str: String,
}

pub struct SessionCandidate {
    pub candidate: String,
    pub sdp_m_line_index: u16,
    pub sdp_mid: String,
}

pub struct JsSessionResponse {
    pub answer: SessionAnswer,
    pub candidate: SessionCandidate,
}

fn get_session_response(input: &str) -> JsSessionResponse {
    let json_obj: JsonValue = input.parse().unwrap();

    let sdp_opt: Option<&String> = json_obj["answer"]["sdp"].get();
    let sdp: String = sdp_opt.unwrap().clone();

    let type_str_opt: Option<&String> = json_obj["answer"]["type"].get();
    let type_str: String = type_str_opt.unwrap().clone();

    let candidate_opt: Option<&String> = json_obj["candidate"]["candidate"].get();
    let candidate: String = candidate_opt.unwrap().clone();

    let sdp_m_line_index_opt: Option<&f64> = json_obj["candidate"]["sdpMLineIndex"].get();
    let sdp_m_line_index: u16 = *(sdp_m_line_index_opt.unwrap()) as u16;

    let sdp_mid_opt: Option<&String> = json_obj["candidate"]["sdpMid"].get();
    let sdp_mid: String = sdp_mid_opt.unwrap().clone();

    JsSessionResponse {
        answer: SessionAnswer { sdp, type_str },
        candidate: SessionCandidate {
            candidate,
            sdp_m_line_index,
            sdp_mid,
        },
    }
}
