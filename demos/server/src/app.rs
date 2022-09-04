use std::env;

use naia_server_socket::{PacketReceiver, PacketSender, ServerAddrs, Socket};
use naia_socket_shared::SocketConfig;

pub(crate) struct App {
    packet_sender: PacketSender,
    packet_receiver: PacketReceiver,
}

impl App {
    pub(crate) fn new() -> Self {
        info!("Naia Server Socket Demo started");

        let listen_address = env::var("LISTEN_ADDRESS").unwrap_or("127.0.0.1".to_string());

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

        let mut socket = Socket::new(&SocketConfig::new(None, None));
        socket.listen(&server_address);

        App {
            packet_sender: socket.packet_sender(),
            packet_receiver: socket.packet_receiver(),
        }
    }

    pub(crate) fn update(&mut self) {
        match self.packet_receiver.receive() {
            Ok(Some((address, payload))) => {
                let message_from_client = String::from_utf8_lossy(payload);
                info!("Server recv <- {}: {}", address, message_from_client);

                if message_from_client.eq("PING") {
                    let message_to_client: String = "PONG".to_string();
                    info!("Server send -> {}: {}", address, message_to_client);
                    self.packet_sender
                        .send(&address, message_to_client.as_bytes());
                }
            }
            Ok(None) => {}
            Err(error) => {
                info!("Server Error: {}", error);
            }
        }
    }
}
