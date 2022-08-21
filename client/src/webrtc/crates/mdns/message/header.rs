use super::*;

// Header is a representation of a DNS message header.
#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub(crate) struct Header {
    pub(crate) id: u16,
    pub(crate) response: bool,
    pub(crate) op_code: OpCode,
    pub(crate) authoritative: bool,
    pub(crate) truncated: bool,
    pub(crate) recursion_desired: bool,
    pub(crate) recursion_available: bool,
    pub(crate) rcode: RCode,
}

impl fmt::Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "dnsmessage.Header{{id: {}, response: {}, op_code: {}, authoritative: {}, truncated: {}, recursion_desired: {}, recursion_available: {}, rcode: {} }}",
            self.id,
            self.response,
            self.op_code,
            self.authoritative,
            self.truncated,
            self.recursion_desired,
            self.recursion_available,
            self.rcode
        )
    }
}

#[derive(Copy, Clone, PartialOrd, PartialEq)]
pub(crate) enum Section {
    NotStarted = 0,
    Header = 1,
    Questions = 2,
    Answers = 3,
    Authorities = 4,
    Additionals = 5,
    Done = 6,
}

impl Default for Section {
    fn default() -> Self {
        Section::NotStarted
    }
}

impl From<u8> for Section {
    fn from(v: u8) -> Self {
        match v {
            0 => Section::NotStarted,
            1 => Section::Header,
            2 => Section::Questions,
            3 => Section::Answers,
            4 => Section::Authorities,
            5 => Section::Additionals,
            _ => Section::Done,
        }
    }
}

impl fmt::Display for Section {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match *self {
            Section::NotStarted => "NotStarted",
            Section::Header => "Header",
            Section::Questions => "question",
            Section::Answers => "answer",
            Section::Authorities => "authority",
            Section::Additionals => "additional",
            Section::Done => "Done",
        };
        write!(f, "{}", s)
    }
}