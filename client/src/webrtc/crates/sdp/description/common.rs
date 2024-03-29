use std::fmt;

/// Information describes the "i=" field which provides textual information
/// about the session.
pub(crate) type Information = String;

/// ConnectionInformation defines the representation for the "c=" field
/// containing connection data.
#[derive(Debug, Default, Clone)]
pub(crate) struct ConnectionInformation {
    pub(crate) network_type: String,
    pub(crate) address_type: String,
    pub(crate) address: Option<Address>,
}

impl fmt::Display for ConnectionInformation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(address) = &self.address {
            write!(f, "{} {} {}", self.network_type, self.address_type, address,)
        } else {
            write!(f, "{} {}", self.network_type, self.address_type,)
        }
    }
}

/// Address describes a structured address token from within the "c=" field.
#[derive(Debug, Default, Clone)]
pub(crate) struct Address {
    pub(crate) address: String,
    pub(crate) ttl: Option<isize>,
    pub(crate) range: Option<isize>,
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = vec![self.address.to_owned()];
        if let Some(t) = &self.ttl {
            parts.push(t.to_string());
        }
        if let Some(r) = &self.range {
            parts.push(r.to_string());
        }
        write!(f, "{}", parts.join("/"))
    }
}

/// Bandwidth describes an optional field which denotes the proposed bandwidth
/// to be used by the session or media.
#[derive(Debug, Default, Clone)]
pub(crate) struct Bandwidth {
    pub(crate) experimental: bool,
    pub(crate) bandwidth_type: String,
    pub(crate) bandwidth: u64,
}

impl fmt::Display for Bandwidth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let output = if self.experimental { "X-" } else { "" };
        write!(f, "{}{}:{}", output, self.bandwidth_type, self.bandwidth)
    }
}

/// EncryptionKey describes the "k=" which conveys encryption key information.
pub(crate) type EncryptionKey = String;

/// Attribute describes the "a=" field which represents the primary means for
/// extending SDP.
#[derive(Debug, Default, Clone)]
pub(crate) struct Attribute {
    pub(crate) key: String,
    pub(crate) value: Option<String>,
}

impl fmt::Display for Attribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(value) = &self.value {
            write!(f, "{}:{}", self.key, value)
        } else {
            write!(f, "{}", self.key)
        }
    }
}

impl Attribute {
    /// new constructs a new attribute
    pub(crate) fn new(key: String, value: Option<String>) -> Self {
        Attribute { key, value }
    }

    /// is_ice_candidate returns true if the attribute key equals "candidate".
    pub(crate) fn is_ice_candidate(&self) -> bool {
        self.key.as_str() == "candidate"
    }
}
