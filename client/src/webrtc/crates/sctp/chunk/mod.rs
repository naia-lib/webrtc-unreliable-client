#[cfg(test)]
mod chunk_test;

pub mod chunk_abort;
pub mod chunk_cookie_ack;
pub mod chunk_cookie_echo;
pub mod chunk_error;
pub mod chunk_forward_tsn;
pub mod chunk_header;
pub mod chunk_heartbeat;
pub mod chunk_heartbeat_ack;
pub mod chunk_init;
pub mod chunk_payload_data;
pub mod chunk_reconfig;
pub mod chunk_selective_ack;
pub mod chunk_shutdown;
pub mod chunk_shutdown_ack;
pub mod chunk_shutdown_complete;
pub mod chunk_type;

use crate::webrtc::sctp::error::{Error, Result};
use chunk_header::*;

use bytes::{Bytes, BytesMut};
use std::marker::Sized;
use std::{any::Any, fmt};

pub trait Chunk: fmt::Display + fmt::Debug {
    fn header(&self) -> ChunkHeader;
    fn unmarshal(raw: &Bytes) -> Result<Self>
    where
        Self: Sized;
    fn marshal_to(&self, buf: &mut BytesMut) -> Result<usize>;
    fn check(&self) -> Result<()>;
    fn value_length(&self) -> usize;
    fn as_any(&self) -> &(dyn Any + Send + Sync);

    fn marshal(&self) -> Result<Bytes> {
        let capacity = CHUNK_HEADER_SIZE + self.value_length();
        let mut buf = BytesMut::with_capacity(capacity);
        self.marshal_to(&mut buf)?;
        Ok(buf.freeze())
    }
}
