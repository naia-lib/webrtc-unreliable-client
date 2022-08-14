pub(crate) mod a;
pub(crate) mod aaaa;
pub(crate) mod cname;
pub(crate) mod mx;
pub(crate) mod ns;
pub(crate) mod opt;
pub(crate) mod ptr;
pub(crate) mod soa;
pub(crate) mod srv;
pub(crate) mod txt;

use super::name::*;
use super::packer::*;
use super::*;
use crate::webrtc::mdns::error::*;

use std::collections::HashMap;
use std::fmt;

// EDNS(0) wire constants.

// A Resource is a DNS resource record.
#[derive(Default, Debug)]
pub(crate) struct Resource {
    pub(crate) header: ResourceHeader,
    pub(crate) body: Option<Box<dyn ResourceBody>>,
}

impl fmt::Display for Resource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "dnsmessage.Resource{{Header: {}, Body: {}}}",
            self.header,
            if let Some(body) = &self.body {
                body.to_string()
            } else {
                "None".to_owned()
            }
        )
    }
}

impl Resource {
    // pack appends the wire format of the Resource to msg.
    pub(crate) fn pack(
        &mut self,
        msg: Vec<u8>,
        compression: &mut Option<HashMap<String, usize>>,
        compression_off: usize,
    ) -> Result<Vec<u8>> {
        if let Some(body) = &self.body {
            self.header.typ = body.real_type();
        } else {
            return Err(Error::ErrNilResourceBody);
        }
        let (mut msg, len_off) = self.header.pack(msg, compression, compression_off)?;
        let pre_len = msg.len();
        if let Some(body) = &self.body {
            msg = body.pack(msg, compression, compression_off)?;
            self.header.fix_len(&mut msg, len_off, pre_len)?;
        }
        Ok(msg)
    }
}

// A ResourceHeader is the header of a DNS resource record. There are
// many types of DNS resource records, but they all share the same header.
#[derive(Clone, Default, PartialEq, Debug)]
pub(crate) struct ResourceHeader {
    // Name is the domain name for which this resource record pertains.
    pub(crate) name: Name,

    // Type is the type of DNS resource record.
    //
    // This field will be set automatically during packing.
    pub(crate) typ: DnsType,

    // Class is the class of network to which this DNS resource record
    // pertains.
    pub(crate) class: DnsClass,

    // TTL is the length of time (measured in seconds) which this resource
    // record is valid for (time to live). All Resources in a set should
    // have the same TTL (RFC 2181 Section 5.2).
    pub(crate) ttl: u32,

    // Length is the length of data in the resource record after the header.
    //
    // This field will be set automatically during packing.
    pub(crate) length: u16,
}

impl fmt::Display for ResourceHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "dnsmessage.ResourceHeader{{Name: {}, Type: {}, Class: {}, TTL: {}, Length: {}}}",
            self.name, self.typ, self.class, self.ttl, self.length,
        )
    }
}

impl ResourceHeader {
    // pack appends the wire format of the ResourceHeader to oldMsg.
    //
    // lenOff is the offset in msg where the Length field was packed.
    pub(crate) fn pack(
        &self,
        mut msg: Vec<u8>,
        compression: &mut Option<HashMap<String, usize>>,
        compression_off: usize,
    ) -> Result<(Vec<u8>, usize)> {
        msg = self.name.pack(msg, compression, compression_off)?;
        msg = self.typ.pack(msg);
        msg = self.class.pack(msg);
        msg = pack_uint32(msg, self.ttl);
        let len_off = msg.len();
        msg = pack_uint16(msg, self.length);
        Ok((msg, len_off))
    }

    pub(crate) fn unpack(&mut self, msg: &[u8], off: usize, _length: usize) -> Result<usize> {
        let mut new_off = off;
        new_off = self.name.unpack(msg, new_off)?;
        new_off = self.typ.unpack(msg, new_off)?;
        new_off = self.class.unpack(msg, new_off)?;
        let (ttl, new_off) = unpack_uint32(msg, new_off)?;
        self.ttl = ttl;
        let (l, new_off) = unpack_uint16(msg, new_off)?;
        self.length = l;

        Ok(new_off)
    }

    // fixLen updates a packed ResourceHeader to include the length of the
    // ResourceBody.
    //
    // lenOff is the offset of the ResourceHeader.Length field in msg.
    //
    // preLen is the length that msg was before the ResourceBody was packed.
    pub(crate) fn fix_len(&mut self, msg: &mut [u8], len_off: usize, pre_len: usize) -> Result<()> {
        if msg.len() < pre_len || msg.len() > pre_len + u16::MAX as usize {
            return Err(Error::ErrResTooLong);
        }

        let con_len = msg.len() - pre_len;

        // Fill in the length now that we know how long the content is.
        msg[len_off] = ((con_len >> 8) & 0xFF) as u8;
        msg[len_off + 1] = (con_len & 0xFF) as u8;
        self.length = con_len as u16;

        Ok(())
    }
}

// A ResourceBody is a DNS resource record minus the header.
pub(crate) trait ResourceBody: fmt::Display + fmt::Debug {
    // real_type returns the actual type of the Resource. This is used to
    // fill in the header Type field.
    fn real_type(&self) -> DnsType;

    // pack packs a Resource except for its header.
    fn pack(
        &self,
        msg: Vec<u8>,
        compression: &mut Option<HashMap<String, usize>>,
        compression_off: usize,
    ) -> Result<Vec<u8>>;

    fn unpack(&mut self, msg: &[u8], off: usize, length: usize) -> Result<usize>;
}