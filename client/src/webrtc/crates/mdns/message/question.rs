use super::name::*;
use super::*;

use std::fmt;

// A question is a DNS query.
#[derive(Default, Debug, PartialEq, Clone)]
pub(crate) struct Question {
    pub(crate) name: Name,
    pub(crate) typ: DnsType,
    pub(crate) class: DnsClass,
}

impl fmt::Display for Question {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "dnsmessage.question{{Name: {}, Type: {}, Class: {}}}",
            self.name, self.typ, self.class
        )
    }
}