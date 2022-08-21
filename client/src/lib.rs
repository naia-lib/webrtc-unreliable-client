#![recursion_limit = "256"]

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_derive;

mod socket;
mod addr_cell;

pub use socket::Socket;
pub use addr_cell::{AddrCell, ServerAddr};

mod webrtc;