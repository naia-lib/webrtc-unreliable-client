#![recursion_limit = "256"]

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_derive;

mod addr_cell;
mod socket;

pub use addr_cell::{AddrCell, ServerAddr};
pub use socket::Socket;

mod webrtc;
