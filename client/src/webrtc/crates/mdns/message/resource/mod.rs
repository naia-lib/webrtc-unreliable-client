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