
use anyhow::{Error, Result};
use tokio::{sync::mpsc, time::Duration};

use webrtc_unreliable_client::{AddrCell, ServerAddr, Socket};

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init()
        .expect("A logger was already initialized");

    log::info!("Client Demo started");

    let server_address = "127.0.0.1";
    let server_url = format!("http://{}:14191/rtc_session", server_address);

    let (addr_cell, to_server_sender, to_client_receiver) =
        Socket::connect(server_url.as_str()).await;

    let addr_cell_1 = addr_cell.clone();
    let addr_cell_2 = addr_cell.clone();
    tokio::spawn(async move {
        read_loop(addr_cell_1, to_client_receiver)
            .await
            .expect("error in read_loop!");
    });

    // Handle writing to the data channel
    tokio::spawn(async move {
        write_loop(addr_cell_2, to_server_sender)
            .await
            .expect("error in write_loop!");
    });

    loop {}
}

async fn read_loop(
    addr_cell: AddrCell,
    mut to_client_receiver: mpsc::Receiver<Box<[u8]>>,
) -> Result<()> {
    loop {
        let message = match to_client_receiver.recv().await {
            Some(message) => message,
            None => {
                return Ok(());
            }
        };

        let addr = match addr_cell.get() {
            ServerAddr::Found(addr) => addr.to_string(),
            ServerAddr::Finding => "".to_string(),
        };
        log::info!(
            "Client recv <- {}: {}",
            addr,
            String::from_utf8_lossy(&message),
        );
    }
}

async fn write_loop(addr_cell: AddrCell, to_server_sender: mpsc::Sender<Box<[u8]>>) -> Result<()> {
    let mut count = 0;

    loop {
        let timeout = tokio::time::sleep(Duration::from_secs(1));
        tokio::pin!(timeout);

        tokio::select! {
            _ = timeout.as_mut() =>{
                if count < 10 {
                    count += 1;

                    let addr = match addr_cell.get() {
                        ServerAddr::Found(addr) => addr.to_string(),
                        ServerAddr::Finding => "".to_string(),
                    };
                    let message = "PING".to_string();
                    log::info!("Client send -> {}: {}", addr, message);
                    match to_server_sender.send(message.as_bytes().into()).await {
                        Ok(_) => {},
                        Err(e) => {
                            return Err(Error::new(e));
                        }
                    }
                }
            }
        };
    }
}
