
use std::convert::From;
use std::fmt;

/// The type of server used in the ice.URL structure.
#[derive(PartialEq, Debug, Copy, Clone)]
pub(crate) enum SchemeType {
    /// The URL represents a STUN server.
    Stun,

    /// The URL represents a STUNS (secure) server.
    Stuns,

    /// The URL represents a TURN server.
    Turn,

    /// The URL represents a TURNS (secure) server.
    Turns,

    /// Default public constant to use for "enum" like struct comparisons when no value was defined.
    Unknown,
}

impl Default for SchemeType {
    fn default() -> Self {
        Self::Unknown
    }
}

impl From<&str> for SchemeType {
    /// Defines a procedure for creating a new `SchemeType` from a raw
    /// string naming the scheme type.
    fn from(raw: &str) -> Self {
        match raw {
            "stun" => Self::Stun,
            "stuns" => Self::Stuns,
            "turn" => Self::Turn,
            "turns" => Self::Turns,
            _ => Self::Unknown,
        }
    }
}

impl fmt::Display for SchemeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match *self {
            SchemeType::Stun => "stun",
            SchemeType::Stuns => "stuns",
            SchemeType::Turn => "turn",
            SchemeType::Turns => "turns",
            SchemeType::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

/// The transport protocol type that is used in the `ice::url::Url` structure.
#[derive(PartialEq, Debug, Copy, Clone)]
pub(crate) enum ProtoType {
    /// The URL uses a UDP transport.
    Udp,

    /// The URL uses a TCP transport.
    Tcp,

    Unknown,
}

impl Default for ProtoType {
    fn default() -> Self {
        Self::Udp
    }
}

// defines a procedure for creating a new ProtoType from a raw
// string naming the transport protocol type.
impl From<&str> for ProtoType {
    // NewSchemeType defines a procedure for creating a new SchemeType from a raw
    // string naming the scheme type.
    fn from(raw: &str) -> Self {
        match raw {
            "udp" => Self::Udp,
            "tcp" => Self::Tcp,
            _ => Self::Unknown,
        }
    }
}

impl fmt::Display for ProtoType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match *self {
            Self::Udp => "udp",
            Self::Tcp => "tcp",
            Self::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

/// Represents a STUN (rfc7064) or TURN (rfc7065) URL.
#[derive(Debug, Clone, Default)]
pub(crate) struct Url {
    pub(crate) scheme: SchemeType,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) username: String,
    pub(crate) password: String,
    pub(crate) proto: ProtoType,
}

impl fmt::Display for Url {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let host = if self.host.contains("::") {
            "[".to_owned() + self.host.as_str() + "]"
        } else {
            self.host.clone()
        };
        if self.scheme == SchemeType::Turn || self.scheme == SchemeType::Turns {
            write!(
                f,
                "{}:{}:{}?transport={}",
                self.scheme, host, self.port, self.proto
            )
        } else {
            write!(f, "{}:{}:{}", self.scheme, host, self.port)
        }
    }
}
