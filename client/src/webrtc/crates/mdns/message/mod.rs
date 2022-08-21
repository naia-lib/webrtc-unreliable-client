
pub(crate) mod header;
pub(crate) mod name;
mod packer;
pub(crate) mod question;
pub(crate) mod resource;

use header::*;
use question::*;
use resource::*;

use std::fmt;

// Message formats

// A Type is a type of DNS request and response.
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum DnsType {
    // ResourceHeader.Type and question.Type
    A = 1,
    Ns = 2,
    Cname = 5,
    Soa = 6,
    Ptr = 12,
    Mx = 15,
    Txt = 16,
    Aaaa = 28,
    Srv = 33,
    Opt = 41,

    // question.Type
    Wks = 11,
    Hinfo = 13,
    Minfo = 14,
    Axfr = 252,
    All = 255,

    Unsupported = 0,
}

impl Default for DnsType {
    fn default() -> Self {
        DnsType::Unsupported
    }
}

impl From<u16> for DnsType {
    fn from(v: u16) -> Self {
        match v {
            1 => DnsType::A,
            2 => DnsType::Ns,
            5 => DnsType::Cname,
            6 => DnsType::Soa,
            12 => DnsType::Ptr,
            15 => DnsType::Mx,
            16 => DnsType::Txt,
            28 => DnsType::Aaaa,
            33 => DnsType::Srv,
            41 => DnsType::Opt,

            // question.Type
            11 => DnsType::Wks,
            13 => DnsType::Hinfo,
            14 => DnsType::Minfo,
            252 => DnsType::Axfr,
            255 => DnsType::All,

            _ => DnsType::Unsupported,
        }
    }
}

impl fmt::Display for DnsType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match *self {
            DnsType::A => "A",
            DnsType::Ns => "NS",
            DnsType::Cname => "CNAME",
            DnsType::Soa => "SOA",
            DnsType::Ptr => "PTR",
            DnsType::Mx => "MX",
            DnsType::Txt => "TXT",
            DnsType::Aaaa => "AAAA",
            DnsType::Srv => "SRV",
            DnsType::Opt => "OPT",
            DnsType::Wks => "WKS",
            DnsType::Hinfo => "HINFO",
            DnsType::Minfo => "MINFO",
            DnsType::Axfr => "AXFR",
            DnsType::All => "ALL",
            _ => "Unsupported",
        };
        write!(f, "{}", s)
    }
}

// A Class is a type of network.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct DnsClass(pub(crate) u16);

// ResourceHeader.Class and question.Class
pub(crate) const DNSCLASS_INET: DnsClass = DnsClass(1);
pub(crate) const DNSCLASS_CSNET: DnsClass = DnsClass(2);
pub(crate) const DNSCLASS_CHAOS: DnsClass = DnsClass(3);
pub(crate) const DNSCLASS_HESIOD: DnsClass = DnsClass(4);
// question.Class
pub(crate) const DNSCLASS_ANY: DnsClass = DnsClass(255);

impl fmt::Display for DnsClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let other = format!("{}", self.0);
        let s = match *self {
            DNSCLASS_INET => "ClassINET",
            DNSCLASS_CSNET => "ClassCSNET",
            DNSCLASS_CHAOS => "ClassCHAOS",
            DNSCLASS_HESIOD => "ClassHESIOD",
            DNSCLASS_ANY => "ClassANY",
            _ => other.as_str(),
        };
        write!(f, "{}", s)
    }
}

// An OpCode is a DNS operation code.
pub(crate) type OpCode = u16;

// An RCode is a DNS response status code.
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum RCode {
    // Message.Rcode
    Success = 0,
    FormatError = 1,
    ServerFailure = 2,
    NameError = 3,
    NotImplemented = 4,
    Refused = 5,
    Unsupported,
}

impl Default for RCode {
    fn default() -> Self {
        RCode::Success
    }
}

impl From<u8> for RCode {
    fn from(v: u8) -> Self {
        match v {
            0 => RCode::Success,
            1 => RCode::FormatError,
            2 => RCode::ServerFailure,
            3 => RCode::NameError,
            4 => RCode::NotImplemented,
            5 => RCode::Refused,
            _ => RCode::Unsupported,
        }
    }
}

impl fmt::Display for RCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match *self {
            RCode::Success => "RCodeSuccess",
            RCode::FormatError => "RCodeFormatError",
            RCode::ServerFailure => "RCodeServerFailure",
            RCode::NameError => "RCodeNameError",
            RCode::NotImplemented => "RCodeNotImplemented",
            RCode::Refused => "RCodeRefused",
            RCode::Unsupported => "RCodeUnsupported",
        };
        write!(f, "{}", s)
    }
}

// Internal constants.

// UINT16LEN is the length (in bytes) of a uint16.
const UINT16LEN: usize = 2;

// UINT32LEN is the length (in bytes) of a uint32.
const UINT32LEN: usize = 4;

// Message is a representation of a DNS message.
#[derive(Default, Debug)]
pub(crate) struct Message {
    pub(crate) header: Header,
    pub(crate) questions: Vec<Question>,
    pub(crate) answers: Vec<Resource>,
    pub(crate) authorities: Vec<Resource>,
    pub(crate) additionals: Vec<Resource>,
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = "dnsmessage.Message{Header: ".to_owned();
        s += self.header.to_string().as_str();

        s += ", Questions: ";
        let v: Vec<String> = self.questions.iter().map(|q| q.to_string()).collect();
        s += &v.join(", ");

        s += ", Answers: ";
        let v: Vec<String> = self.answers.iter().map(|q| q.to_string()).collect();
        s += &v.join(", ");

        s += ", Authorities: ";
        let v: Vec<String> = self.authorities.iter().map(|q| q.to_string()).collect();
        s += &v.join(", ");

        s += ", Additionals: ";
        let v: Vec<String> = self.additionals.iter().map(|q| q.to_string()).collect();
        s += &v.join(", ");

        write!(f, "{}", s)
    }
}