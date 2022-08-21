
pub(crate) mod binding;
pub(crate) mod periodic_timer;
pub(crate) mod permission;
pub(crate) mod relay_conn;
pub(crate) mod transaction;

use crate::webrtc::turn::error::*;
use relay_conn::*;
use transaction::*;

use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use crate::webrtc::stun::message::*;
use crate::webrtc::stun::textattrs::*;
use tokio::sync::{mpsc, Mutex};
use crate::webrtc::util::conn::*;

use async_trait::async_trait;

//              interval [msec]
// 0: 0 ms      +500
// 1: 500 ms	+1000
// 2: 1500 ms   +2000
// 3: 3500 ms   +4000
// 4: 7500 ms   +8000
// 5: 15500 ms  +16000
// 6: 31500 ms  +32000
// -: 63500 ms  failed

struct ClientInternal {
    conn: Arc<dyn Conn + Send + Sync>,
    turn_serv_addr: String,
    username: Username,
    realm: Realm,
    tr_map: Arc<Mutex<TransactionMap>>,
    rto_in_ms: u16,
    read_ch_tx: Arc<Mutex<Option<mpsc::Sender<InboundData>>>>,
}

#[async_trait]
impl RelayConnObserver for ClientInternal {
    // turn_server_addr return the TURN server address
    fn turn_server_addr(&self) -> String {
        self.turn_serv_addr.clone()
    }

    // username returns username
    fn username(&self) -> Username {
        self.username.clone()
    }

    // realm return realm
    fn realm(&self) -> Realm {
        self.realm.clone()
    }

    // WriteTo sends data to the specified destination using the base socket.
    async fn write_to(&self, data: &[u8], to: &str) -> std::result::Result<usize, crate::webrtc::util::Error> {
        let n = self.conn.send_to(data, SocketAddr::from_str(to)?).await?;
        Ok(n)
    }

    // PerformTransaction performs STUN transaction
    async fn perform_transaction(
        &mut self,
        msg: &Message,
        to: &str,
        ignore_result: bool,
    ) -> Result<TransactionResult> {
        let tr_key = base64::encode(&msg.transaction_id.0);

        let mut tr = Transaction::new(TransactionConfig {
            key: tr_key.clone(),
            raw: msg.raw.clone(),
            to: to.to_string(),
            interval: self.rto_in_ms,
            ignore_result,
        });
        let result_ch_rx = tr.get_result_channel();

        log::trace!("start {} transaction {} to {}", msg.typ, tr_key, tr.to);
        {
            let mut tm = self.tr_map.lock().await;
            tm.insert(tr_key.clone(), tr);
        }

        self.conn
            .send_to(&msg.raw, SocketAddr::from_str(to)?)
            .await?;

        let conn2 = Arc::clone(&self.conn);
        let tr_map2 = Arc::clone(&self.tr_map);
        {
            let mut tm = self.tr_map.lock().await;
            if let Some(tr) = tm.get(&tr_key) {
                tr.start_rtx_timer(conn2, tr_map2).await;
            }
        }

        // If dontWait is true, get the transaction going and return immediately
        if ignore_result {
            return Ok(TransactionResult::default());
        }

        // wait_for_result waits for the transaction result
        if let Some(mut result_ch_rx) = result_ch_rx {
            match result_ch_rx.recv().await {
                Some(tr) => Ok(tr),
                None => Err(Error::ErrTransactionClosed),
            }
        } else {
            Err(Error::ErrWaitForResultOnNonResultTransaction)
        }
    }
}

impl ClientInternal {

    // Close closes this client
    async fn close(&mut self) {
        {
            let mut read_ch_tx = self.read_ch_tx.lock().await;
            read_ch_tx.take();
        }
        {
            let mut tm = self.tr_map.lock().await;
            tm.close_and_delete_all();
        }
    }
}

// Client is a STUN server client
#[derive(Clone)]
pub(crate) struct Client {
    client_internal: Arc<Mutex<ClientInternal>>,
}

impl Client {

    pub(crate) async fn close(&self) -> Result<()> {
        let mut ci = self.client_internal.lock().await;
        ci.close().await;
        Ok(())
    }
}
