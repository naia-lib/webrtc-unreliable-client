//! Hermetic loopback interop test: webrtc-unreliable-client <-> webrtc-unreliable
//! (via naia-server-socket), exercising the full signaling + DTLS + SCTP path
//! in-process on 127.0.0.1. Mirrors the demos/{server,client} pair, but bounded,
//! self-cleaning, and assertion-driven.
//!
//! Ports are distinct from the demos (24191/24192) so both can coexist.
//! Any WARN/ERROR logs emitted during the run are captured and reported at the
//! end as unresolved evidence — never filtered, but only connection-path
//! failures fail the test.

use std::{
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Mutex, OnceLock,
    },
    thread,
    time::Duration,
};

use naia_server_socket::{
    shared::SocketConfig, PacketReceiver, PacketSender, ServerAddrs, Socket as ServerSocket,
};
use webrtc_unreliable_client::{ServerAddr, Socket as ClientSocket};

const SIGNAL_ADDR: &str = "127.0.0.1:24191";
const DATA_ADDR: &str = "127.0.0.1:24192";
const AUTH_TOKEN: &str = "12345";
const ROUND_TRIPS: u32 = 5;
const WHOLE_TEST_TIMEOUT: Duration = Duration::from_secs(30);
const PER_PONG_TIMEOUT: Duration = Duration::from_secs(5);

/// Captures WARN/ERROR records from the whole process so teardown artifacts
/// (e.g. the known SCTP heartbeat parse warning) are surfaced, not filtered.
struct CapturingLogger {
    warnings: Mutex<Vec<String>>,
}

static LOGGER: OnceLock<CapturingLogger> = OnceLock::new();

impl log::Log for &'static CapturingLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Warn
    }
    fn log(&self, record: &log::Record) {
        if record.level() <= log::Level::Warn {
            self.warnings
                .lock()
                .unwrap()
                .push(format!("{}: {}", record.target(), record.args()));
        }
    }
    fn flush(&self) {}
}

fn install_logger() -> &'static CapturingLogger {
    let logger = LOGGER.get_or_init(|| CapturingLogger {
        warnings: Mutex::new(Vec::new()),
    });
    let _ = log::set_logger(Box::leak(Box::new(logger)));
    log::set_max_level(log::LevelFilter::Warn);
    logger
}

/// Server half: auth-accept + PING->PONG echo, polled on a thread until `stop`.
fn run_server(stop: Arc<AtomicBool>, pongs_sent: Arc<AtomicU32>) {
    let server_addrs = ServerAddrs::new(
        SIGNAL_ADDR.parse().expect("bad session addr"),
        DATA_ADDR.parse().expect("bad data addr"),
        &format!("http://{}", DATA_ADDR),
    );
    let (auth_sender, mut auth_receiver, mut packet_sender, mut packet_receiver) =
        ServerSocket::listen_with_auth(&server_addrs, &SocketConfig::new(None, None));

    while !stop.load(Ordering::Relaxed) {
        let mut idle = true;

        if let Ok(Some((address, payload))) = auth_receiver.receive() {
            idle = false;
            if String::from_utf8_lossy(payload) == AUTH_TOKEN {
                auth_sender
                    .accept(&address, &"id".to_string())
                    .expect("server failed to accept auth");
            } else {
                let _ = auth_sender.reject(&address);
            }
        }

        if let Ok(Some((address, payload))) = packet_receiver.receive() {
            idle = false;
            if String::from_utf8_lossy(payload) == "PING" {
                packet_sender
                    .send(&address, "PONG".as_bytes())
                    .expect("server failed to send PONG");
                pongs_sent.fetch_add(1, Ordering::Relaxed);
            }
        }

        if idle {
            thread::sleep(Duration::from_millis(1));
        }
    }
}

async fn client_ping_pong(pongs_received: Arc<AtomicU32>) {
    let (socket, mut io) = ClientSocket::new();
    socket
        .connect(
            &format!("http://{}/rtc_session", SIGNAL_ADDR),
            Some(AUTH_TOKEN.as_bytes().to_vec()),
            None,
        )
        .await;

    // Connection is established once the id token arrives and the server addr
    // is resolved.
    let id = tokio::time::timeout(Duration::from_secs(10), io.to_client_id_receiver)
        .await
        .expect("timed out waiting for id token (signaling/DTLS handshake)")
        .expect("id channel dropped")
        .expect("server rejected auth");
    assert!(!id.is_empty(), "empty id token");
    assert!(
        matches!(io.addr_cell.get(), ServerAddr::Found(_)),
        "server data addr not resolved after handshake"
    );

    for round in 1..=ROUND_TRIPS {
        io.to_server_sender
            .send("PING".as_bytes().into())
            .expect("client send channel closed");
        let reply = tokio::time::timeout(PER_PONG_TIMEOUT, io.to_client_receiver.recv())
            .await
            .unwrap_or_else(|_| panic!("round {round}: no PONG within {PER_PONG_TIMEOUT:?}"))
            .unwrap_or_else(|| panic!("round {round}: client receive channel closed"));
        assert_eq!(
            String::from_utf8_lossy(&reply),
            "PONG",
            "round {round}: unexpected payload"
        );
        pongs_received.fetch_add(1, Ordering::Relaxed);
    }

    // Deterministic client-side shutdown before the server stops.
    let _ = io.to_server_disconnect_sender.send(()).await;
    tokio::time::sleep(Duration::from_millis(250)).await;
}

#[test]
fn loopback_ping_pong_round_trips() {
    let logger = install_logger();
    let stop = Arc::new(AtomicBool::new(false));
    let pongs_sent = Arc::new(AtomicU32::new(0));
    let pongs_received = Arc::new(AtomicU32::new(0));

    let server_thread = {
        let stop = stop.clone();
        let pongs_sent = pongs_sent.clone();
        thread::spawn(move || run_server(stop, pongs_sent))
    };

    let started = std::time::Instant::now();
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    let client_result = runtime.block_on(async {
        tokio::time::timeout(WHOLE_TEST_TIMEOUT, client_ping_pong(pongs_received.clone())).await
    });
    // Stop the server and tear down the client runtime regardless of outcome,
    // so no listener or thread outlives the test.
    stop.store(true, Ordering::Relaxed);
    runtime.shutdown_timeout(Duration::from_secs(5));
    server_thread.join().expect("server thread panicked");
    let elapsed = started.elapsed();

    client_result.expect("whole-test timeout exceeded");

    assert_eq!(pongs_received.load(Ordering::Relaxed), ROUND_TRIPS);
    assert_eq!(
        pongs_sent.load(Ordering::Relaxed),
        ROUND_TRIPS,
        "server-side echo count mismatch"
    );

    let warnings = logger.warnings.lock().unwrap();
    println!(
        "interop baseline: {ROUND_TRIPS} bidirectional round trips in {elapsed:?}; \
         {} WARN+ log records captured (unresolved evidence, not filtered):",
        warnings.len()
    );
    for w in warnings.iter() {
        println!("  UNRESOLVED-EVIDENCE {w}");
    }
}
