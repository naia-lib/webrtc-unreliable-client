use std::{time::Duration, thread::sleep};

use naia_server_socket::{
    shared::SocketConfig, AuthReceiver, AuthSender, PacketReceiver, PacketSender, ServerAddrs,
    Socket,
};

pub(crate) struct App {
    auth_sender: Box<dyn AuthSender>,
    auth_receiver: Box<dyn AuthReceiver>,
    packet_sender: Box<dyn PacketSender>,
    packet_receiver: Box<dyn PacketReceiver>,
}

impl App {
    pub(crate) fn new() -> Self {
        info!("Naia Server Socket Demo started");

        let listen_address = "127.0.0.1";

        let server_address = ServerAddrs::new(
            format!("{}:14191", listen_address)
                .parse()
                .expect("could not parse Session address/port"),
            // IP Address to listen on for UDP WebRTC data channels
            format!("{}:14192", listen_address)
                .parse()
                .expect("could not parse WebRTC data address/port"),
            // The public WebRTC IP address to advertise
            format!("http://{}:14192", listen_address).as_str(),
        );

        let (auth_sender, auth_receiver, packet_sender, packet_receiver) =
            Socket::listen_with_auth(&server_address, &SocketConfig::new(None, None));

        Self {
            auth_sender,
            auth_receiver,
            packet_sender,
            packet_receiver,
        }
    }

    pub(crate) fn update(&mut self) {
        let mut no_auths = false;
        let mut no_packets = false;

        match self.auth_receiver.receive() {
            Ok(Some((address, payload))) => {
                let auth_from_client = String::from_utf8_lossy(payload);
                info!(
                    "Server incoming Auth <- {}: [{}]",
                    address, auth_from_client
                );

                if auth_from_client.eq("12345") {
                    let id_token = "id".to_string();
                    if let Err(error) = self.auth_sender.accept(&address, &id_token) {
                        info!("Server Accept Auth error {}", error);
                    } else {
                        info!("Server Auth accepted: {}", address);
                    }
                } else {
                    if let Err(error) = self.auth_sender.reject(&address) {
                        info!("Server Reject Auth error {}", error);
                    } else {
                        info!("Server Auth rejected: {}", address);
                    }
                }
            }
            Ok(None) => {
                no_auths = true;
            }
            Err(error) => {
                info!("Server Auth Error: {}", error);
            }
        }
        match self.packet_receiver.receive() {
            Ok(Some((address, payload))) => {
                let message_from_client = String::from_utf8_lossy(payload);
                info!("Server recv <- {}: {}", address, message_from_client);

                if message_from_client.eq("PING") {
                    let message_to_client: String = "PONG".to_string();
                    info!("Server send -> {}: {}", address, message_to_client);
                    match self
                        .packet_sender
                        .send(&address, message_to_client.as_bytes())
                    {
                        Ok(()) => {}
                        Err(error) => {
                            info!("Server Send Error {}", error);
                        }
                    }
                }
            }
            Ok(None) => {
                no_packets = true;
            }
            Err(error) => {
                info!("Server Error: {}", error);
            }
        }

        if no_auths && no_packets {
            // If we don't sleep here, app will loop at 100% CPU until a new message comes in
            sleep(Duration::from_millis(1));
        }
    }
}
