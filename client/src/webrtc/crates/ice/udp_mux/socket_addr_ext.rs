use std::array::TryFromSliceError;
use std::convert::TryInto;
use std::net::SocketAddr;

use crate::webrtc::util::Error;

pub(crate) trait SocketAddrExt {
    ///Encode a representation of `self` into the buffer and return the length of this encoded
    ///version.
    ///
    /// The buffer needs to be at least 27 bytes in length.
    fn encode(&self, buffer: &mut [u8]) -> Result<usize, Error>;

    /// Decode a `SocketAddr` from a buffer. The encoding should have previously been done with
    /// [`SocketAddrExt::encode`].
    fn decode(buffer: &[u8]) -> Result<SocketAddr, Error>;
}

const IPV4_MARKER: u8 = 4;
const IPV4_ADDRESS_SIZE: usize = 7;
const IPV6_MARKER: u8 = 6;
const IPV6_ADDRESS_SIZE: usize = 27;

pub(crate) const MAX_ADDR_SIZE: usize = IPV6_ADDRESS_SIZE;

impl SocketAddrExt for SocketAddr {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize, Error> {
        use std::net::SocketAddr::*;

        if buffer.len() < MAX_ADDR_SIZE {
            return Err(Error::ErrBufferShort);
        }

        match self {
            V4(addr) => {
                let marker = IPV4_MARKER;
                let ip: [u8; 4] = addr.ip().octets();
                let port: u16 = addr.port();

                buffer[0] = marker;
                buffer[1..5].copy_from_slice(&ip);
                buffer[5..7].copy_from_slice(&port.to_le_bytes());

                Ok(7)
            }
            V6(addr) => {
                let marker = IPV6_MARKER;
                let ip: [u8; 16] = addr.ip().octets();
                let port: u16 = addr.port();
                let flowinfo = addr.flowinfo();
                let scope_id = addr.scope_id();

                buffer[0] = marker;
                buffer[1..17].copy_from_slice(&ip);
                buffer[17..19].copy_from_slice(&port.to_le_bytes());
                buffer[19..23].copy_from_slice(&flowinfo.to_le_bytes());
                buffer[23..27].copy_from_slice(&scope_id.to_le_bytes());

                Ok(MAX_ADDR_SIZE)
            }
        }
    }

    fn decode(buffer: &[u8]) -> Result<SocketAddr, Error> {
        use std::net::*;

        match buffer[0] {
            IPV4_MARKER => {
                if buffer.len() < IPV4_ADDRESS_SIZE {
                    return Err(Error::ErrBufferShort);
                }

                let ip_parts = &buffer[1..5];
                let port = match &buffer[5..7].try_into() {
                    Err(_) => return Err(Error::ErrFailedToParseIpaddr),
                    Ok(input) => u16::from_le_bytes(*input),
                };

                let ip = Ipv4Addr::new(ip_parts[0], ip_parts[1], ip_parts[2], ip_parts[3]);

                Ok(SocketAddr::V4(SocketAddrV4::new(ip, port)))
            }
            IPV6_MARKER => {
                if buffer.len() < IPV6_ADDRESS_SIZE {
                    return Err(Error::ErrBufferShort);
                }

                // Just to help the type system infer correctly
                fn helper(b: &[u8]) -> Result<&[u8; 16], TryFromSliceError> {
                    b.try_into()
                }

                let ip = match helper(&buffer[1..17]) {
                    Err(_) => return Err(Error::ErrFailedToParseIpaddr),
                    Ok(input) => Ipv6Addr::from(*input),
                };
                let port = match &buffer[17..19].try_into() {
                    Err(_) => return Err(Error::ErrFailedToParseIpaddr),
                    Ok(input) => u16::from_le_bytes(*input),
                };

                let flowinfo = match &buffer[19..23].try_into() {
                    Err(_) => return Err(Error::ErrFailedToParseIpaddr),
                    Ok(input) => u32::from_le_bytes(*input),
                };

                let scope_id = match &buffer[23..27].try_into() {
                    Err(_) => return Err(Error::ErrFailedToParseIpaddr),
                    Ok(input) => u32::from_le_bytes(*input),
                };

                Ok(SocketAddr::V6(SocketAddrV6::new(
                    ip, port, flowinfo, scope_id,
                )))
            }
            _ => Err(Error::ErrFailedToParseIpaddr),
        }
    }
}