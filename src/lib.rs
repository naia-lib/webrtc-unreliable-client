#![recursion_limit = "256"]

#[macro_use]
extern crate lazy_static;

mod addr_cell;
mod socket;

pub use addr_cell::{AddrCell, ServerAddr};
pub use socket::{Socket, SocketIo, MAX_MESSAGE_SIZE};

mod webrtc;
