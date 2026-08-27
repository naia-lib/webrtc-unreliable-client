//! Hermetic loopback interop tests: webrtc-unreliable-client <-> webrtc-unreliable
//! (via naia-server-socket), exercising the full signaling + DTLS + SCTP path
//! in-process on 127.0.0.1. Mirrors the demos/{server,client} pair, but bounded,
//! self-cleaning, and assertion-driven.
//!
//! Each test uses its own port pair (distinct from the demos) so tests can run
//! concurrently and coexist with the demo binaries.
//! Any WARN/ERROR logs emitted during the run are captured and reported at the
//! end as unresolved evidence — never filtered, but only connection-path
//! failures fail the test.
//!
//! Beyond the happy-path baseline, the variants below deliberately broaden the
//! exercised code envelope (for coverage-guided slimming of the vendored
//! webrtc tree): large payloads (SCTP fragmentation/reassembly), auth
//! rejection, and abrupt client teardown.

use std::{
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Mutex, OnceLock,
    },
    thread,
    time::Duration,
};

use naia_server_socket::{ServerAddrs, Socket as ServerSocket, shared::SocketConfig};
use webrtc_unreliable_client::{ServerAddr, Socket as ClientSocket};

const AUTH_TOKEN: &str = "12345";
const WHOLE_TEST_TIMEOUT: Duration = Duration::from_secs(30);
const PER_REPLY_TIMEOUT: Duration = Duration::from_secs(5);

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

/// Server half: auth-accept (token match) + echo, polled on a thread until `stop`.
/// Echoes every received payload back verbatim and counts echoes.
fn run_server(signal_addr: &str, data_addr: &str, stop: Arc<AtomicBool>, echoes: Arc<AtomicU32>) {
    let server_addrs = ServerAddrs::new(
        signal_addr.parse().expect("bad session addr"),
        data_addr.parse().expect("bad data addr"),
        &format!("http://{}", data_addr),
    );
    let (auth_sender, mut auth_receiver, packet_sender, mut packet_receiver) =
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
            let payload = payload.to_vec();
            packet_sender
                .send(&address, &payload)
                .expect("server failed to echo");
            echoes.fetch_add(1, Ordering::Relaxed);
        }

        if idle {
            thread::sleep(Duration::from_millis(1));
        }
    }
}

struct ServerHandle {
    stop: Arc<AtomicBool>,
    echoes: Arc<AtomicU32>,
    thread: thread::JoinHandle<()>,
}

fn spawn_server(signal_addr: &'static str, data_addr: &'static str) -> ServerHandle {
    let stop = Arc::new(AtomicBool::new(false));
    let echoes = Arc::new(AtomicU32::new(0));
    let thread = {
        let stop = stop.clone();
        let echoes = echoes.clone();
        thread::spawn(move || run_server(signal_addr, data_addr, stop, echoes))
    };
    ServerHandle {
        stop,
        echoes,
        thread,
    }
}

impl ServerHandle {
    fn shutdown(self) -> u32 {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.join().expect("server thread panicked");
        self.echoes.load(Ordering::Relaxed)
    }
}

/// Connects a client and waits for the accepted id token; asserts the data
/// addr resolved. Returns the io halves for the caller to drive.
async fn connect_client(
    signal_addr: &str,
    token: &str,
) -> Result<webrtc_unreliable_client::SocketIo, ()> {
    let (socket, mut io) = ClientSocket::new();
    socket
        .connect(
            &format!("http://{}/rtc_session", signal_addr),
            Some(token.as_bytes().to_vec()),
            None,
        )
        .await;

    let id_result = tokio::time::timeout(Duration::from_secs(10), &mut io.to_client_id_receiver)
        .await
        .expect("timed out waiting for auth outcome (signaling/DTLS handshake)")
        .expect("id channel dropped");
    match id_result {
        Ok(id) => {
            assert!(!id.is_empty(), "empty id token");
            assert!(
                matches!(io.addr_cell.get(), ServerAddr::Found(_)),
                "server data addr not resolved after handshake"
            );
            Ok(io)
        }
        Err(_) => Err(()),
    }
}

/// Sends each payload and asserts it is echoed back verbatim.
async fn echo_rounds(io: &mut webrtc_unreliable_client::SocketIo, payloads: &[Vec<u8>]) {
    for (i, payload) in payloads.iter().enumerate() {
        io.to_server_sender
            .send(payload.clone().into_boxed_slice())
            .expect("client send channel closed");
        let reply = tokio::time::timeout(PER_REPLY_TIMEOUT, io.to_client_receiver.recv())
            .await
            .unwrap_or_else(|_| panic!("round {i}: no echo within {PER_REPLY_TIMEOUT:?}"))
            .unwrap_or_else(|| panic!("round {i}: client receive channel closed"));
        assert_eq!(
            reply.as_ref(),
            payload.as_slice(),
            "round {i}: echoed payload mismatch (len sent {} vs received {})",
            payload.len(),
            reply.len()
        );
    }
}

fn report_warnings(logger: &CapturingLogger, label: &str, elapsed: Duration) {
    let warnings = logger.warnings.lock().unwrap();
    println!(
        "{label}: done in {elapsed:?}; {} WARN+ log records captured so far \
         (unresolved evidence, cumulative across tests, not filtered):",
        warnings.len()
    );
    for w in warnings.iter() {
        println!("  UNRESOLVED-EVIDENCE {w}");
    }
}

/// Baseline: 5 bidirectional PING/PONG round trips, deterministic teardown.
#[test]
fn loopback_ping_pong_round_trips() {
    let logger = install_logger();
    let server = spawn_server("127.0.0.1:24191", "127.0.0.1:24192");
    let started = std::time::Instant::now();

    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    let result = runtime.block_on(async {
        tokio::time::timeout(WHOLE_TEST_TIMEOUT, async {
            let mut io = connect_client("127.0.0.1:24191", AUTH_TOKEN)
                .await
                .expect("auth unexpectedly rejected");
            let payloads: Vec<Vec<u8>> = (0..5).map(|_| b"PING".to_vec()).collect();
            echo_rounds(&mut io, &payloads).await;
            // Deterministic client-side shutdown before the server stops.
            let _ = io.to_server_disconnect_sender.send(()).await;
            tokio::time::sleep(Duration::from_millis(250)).await;
        })
        .await
    });
    runtime.shutdown_timeout(Duration::from_secs(5));
    let echoes = server.shutdown();

    result.expect("whole-test timeout exceeded");
    assert_eq!(echoes, 5, "server-side echo count mismatch");
    report_warnings(logger, "baseline", started.elapsed());
}

/// Large payloads, up to the system's supported envelope: webrtc-unreliable
/// (the only peer) drops fragmented SCTP packets by design (client.rs:614 in
/// that repo), so a message must fit in a single SCTP packet under the
/// client's MTU (~1200). Probed empirically: 1150 round-trips, 4000 does not.
/// Multi-fragment sizes are deliberately NOT tested: they are outside the
/// protocol envelope, which is itself slimming evidence.
#[test]
fn loopback_large_payloads_round_trip() {
    let logger = install_logger();
    let server = spawn_server("127.0.0.1:24201", "127.0.0.1:24202");
    let started = std::time::Instant::now();

    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    let result = runtime.block_on(async {
        tokio::time::timeout(WHOLE_TEST_TIMEOUT, async {
            let mut io = connect_client("127.0.0.1:24201", AUTH_TOKEN)
                .await
                .expect("auth unexpectedly rejected");
            let payloads: Vec<Vec<u8>> = [512usize, 1000, 1150]
                .iter()
                .map(|&n| (0..n).map(|i| (i % 251) as u8).collect())
                .collect();
            let count = payloads.len() as u32;
            echo_rounds(&mut io, &payloads).await;
            let _ = io.to_server_disconnect_sender.send(()).await;
            tokio::time::sleep(Duration::from_millis(250)).await;
            count
        })
        .await
    });
    runtime.shutdown_timeout(Duration::from_secs(5));
    let echoes = server.shutdown();

    let count = result.expect("whole-test timeout exceeded");
    assert_eq!(echoes, count, "server-side echo count mismatch");
    report_warnings(logger, "large-payloads", started.elapsed());
}

/// Wrong token: the server must reject, and the client must observe it.
#[test]
fn loopback_auth_reject() {
    let logger = install_logger();
    let server = spawn_server("127.0.0.1:24211", "127.0.0.1:24212");
    let started = std::time::Instant::now();

    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    let result = runtime.block_on(async {
        tokio::time::timeout(WHOLE_TEST_TIMEOUT, async {
            connect_client("127.0.0.1:24211", "wrong-token").await
        })
        .await
    });
    runtime.shutdown_timeout(Duration::from_secs(5));
    let echoes = server.shutdown();

    let outcome = result.expect("whole-test timeout exceeded");
    assert!(outcome.is_err(), "server accepted a wrong auth token");
    assert_eq!(echoes, 0, "no data should flow on a rejected session");
    report_warnings(logger, "auth-reject", started.elapsed());
}

/// Abrupt teardown: client vanishes mid-session (no disconnect message).
/// The server must stay healthy and remain usable; exercises client-loss
/// paths on the server and non-graceful shutdown paths in the client.
#[test]
fn loopback_abrupt_client_teardown() {
    let logger = install_logger();
    let server = spawn_server("127.0.0.1:24221", "127.0.0.1:24222");
    let started = std::time::Instant::now();

    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    let result = runtime.block_on(async {
        tokio::time::timeout(WHOLE_TEST_TIMEOUT, async {
            let mut io = connect_client("127.0.0.1:24221", AUTH_TOKEN)
                .await
                .expect("auth unexpectedly rejected");
            echo_rounds(&mut io, &[b"PING".to_vec()]).await;
            // Drop io without sending disconnect: abrupt vanish.
            drop(io);
        })
        .await
    });
    // Kill the entire client runtime abruptly too.
    runtime.shutdown_timeout(Duration::from_millis(100));
    result.expect("whole-test timeout exceeded");

    // Server must still be alive and polling after the client vanished.
    thread::sleep(Duration::from_millis(500));
    assert!(
        !server.thread.is_finished(),
        "server thread died after abrupt client teardown"
    );
    let echoes = server.shutdown();
    assert_eq!(echoes, 1);
    report_warnings(logger, "abrupt-teardown", started.elapsed());
}
