#[cfg(test)]
mod binding_test;

use std::collections::HashMap;
use std::net::SocketAddr;

use tokio::time::Instant;

//  Chanel number:
//    0x4000 through 0x7FFF: These values are the allowed channel
//    numbers (16,383 possible values).
const MIN_CHANNEL_NUMBER: u16 = 0x4000;
const MAX_CHANNEL_NUMBER: u16 = 0x7fff;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum BindingState {
    Idle,
    Request,
    Ready,
    Refresh,
    Failed,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Binding {
    pub number: u16,
    pub st: BindingState,
    pub addr: SocketAddr,
    pub refreshed_at: Instant,
}

impl Binding {
    pub fn set_state(&mut self, state: BindingState) {
        //atomic.StoreInt32((*int32)(&b.st), int32(state))
        self.st = state;
    }

    pub fn state(&self) -> BindingState {
        //return BindingState(atomic.LoadInt32((*int32)(&b.st)))
        self.st
    }

    pub fn set_refreshed_at(&mut self, at: Instant) {
        self.refreshed_at = at;
    }

    pub fn refreshed_at(&self) -> Instant {
        self.refreshed_at
    }
}
// Thread-safe Binding map
#[derive(Default)]
pub struct BindingManager {
    chan_map: HashMap<u16, String>,
    addr_map: HashMap<String, Binding>,
    next: u16,
}

impl BindingManager {
    pub fn new() -> Self {
        BindingManager {
            chan_map: HashMap::new(),
            addr_map: HashMap::new(),
            next: MIN_CHANNEL_NUMBER,
        }
    }

    pub fn assign_channel_number(&mut self) -> u16 {
        let n = self.next;
        if self.next == MAX_CHANNEL_NUMBER {
            self.next = MIN_CHANNEL_NUMBER;
        } else {
            self.next += 1;
        }
        n
    }

    pub fn create(&mut self, addr: SocketAddr) -> Option<&Binding> {
        let b = Binding {
            number: self.assign_channel_number(),
            st: BindingState::Idle,
            addr,
            refreshed_at: Instant::now(),
        };

        self.chan_map.insert(b.number, b.addr.to_string());
        self.addr_map.insert(b.addr.to_string(), b);
        self.addr_map.get(&addr.to_string())
    }

    pub fn find_by_addr(&self, addr: &SocketAddr) -> Option<&Binding> {
        self.addr_map.get(&addr.to_string())
    }

    pub fn get_by_addr(&mut self, addr: &SocketAddr) -> Option<&mut Binding> {
        self.addr_map.get_mut(&addr.to_string())
    }

    pub fn find_by_number(&self, number: u16) -> Option<&Binding> {
        if let Some(s) = self.chan_map.get(&number) {
            self.addr_map.get(s)
        } else {
            None
        }
    }

    pub fn delete_by_addr(&mut self, addr: &SocketAddr) -> bool {
        if let Some(b) = self.addr_map.remove(&addr.to_string()) {
            self.chan_map.remove(&b.number);
            true
        } else {
            false
        }
    }
}
