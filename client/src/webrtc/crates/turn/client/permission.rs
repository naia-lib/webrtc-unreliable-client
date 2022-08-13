use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum PermState {
    Idle = 0,
    Permitted = 1,
}

impl Default for PermState {
    fn default() -> Self {
        PermState::Idle
    }
}

impl From<u8> for PermState {
    fn from(v: u8) -> Self {
        match v {
            0 => PermState::Idle,
            _ => PermState::Permitted,
        }
    }
}

#[derive(Default)]
pub struct Permission {
    st: AtomicU8, //PermState,
}

impl Permission {
    pub fn set_state(&self, state: PermState) {
        self.st.store(state as u8, Ordering::SeqCst);
    }

    pub fn state(&self) -> PermState {
        self.st.load(Ordering::SeqCst).into()
    }
}

// Thread-safe Permission map
#[derive(Default)]
pub struct PermissionMap {
    perm_map: HashMap<String, Arc<Permission>>,
}

impl PermissionMap {
    pub fn new() -> PermissionMap {
        PermissionMap {
            perm_map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, addr: &SocketAddr, p: Arc<Permission>) {
        self.perm_map.insert(addr.ip().to_string(), p);
    }

    pub fn find(&self, addr: &SocketAddr) -> Option<&Arc<Permission>> {
        self.perm_map.get(&addr.ip().to_string())
    }

    pub fn delete(&mut self, addr: &SocketAddr) {
        self.perm_map.remove(&addr.ip().to_string());
    }

    pub fn addrs(&self) -> Vec<SocketAddr> {
        let mut a = vec![];
        for k in self.perm_map.keys() {
            if let Ok(ip) = k.parse() {
                a.push(SocketAddr::new(ip, 0));
            }
        }
        a
    }
}
