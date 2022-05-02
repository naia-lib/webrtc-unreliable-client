use crate::webrtc::mdns::error::*;
use crate::webrtc::mdns::message::header::{Header, HeaderInternal, Section};
use crate::webrtc::mdns::message::name::Name;
use crate::webrtc::mdns::message::question::Question;
use crate::webrtc::mdns::message::resource::ResourceHeader;
use crate::webrtc::mdns::message::{DnsClass, DnsType};

// A Parser allows incrementally parsing a DNS message.
//
// When parsing is started, the Header is parsed. Next, each question can be
// either parsed or skipped. Alternatively, all Questions can be skipped at
// once. When all Questions have been parsed, attempting to parse Questions
// will return (nil, nil) and attempting to skip Questions will return
// (true, nil). After all Questions have been either parsed or skipped, all
// Answers, Authorities and Additionals can be either parsed or skipped in the
// same way, and each type of Resource must be fully parsed or skipped before
// proceeding to the next type of Resource.
//
// Note that there is no requirement to fully skip or parse the message.
#[derive(Default)]
pub struct Parser<'a> {
    pub msg: &'a [u8],
    pub header: HeaderInternal,

    pub section: Section,
    pub off: usize,
    pub index: usize,
    pub res_header_valid: bool,
    pub res_header: ResourceHeader,
}

impl<'a> Parser<'a> {
    // start parses the header and enables the parsing of Questions.
    pub fn start(&mut self, msg: &'a [u8]) -> Result<Header> {
        *self = Parser {
            msg,
            ..Default::default()
        };
        self.off = self.header.unpack(msg, 0)?;
        self.section = Section::Questions;
        Ok(self.header.header())
    }

    fn check_advance(&mut self, sec: Section) -> Result<()> {
        if self.section < sec {
            return Err(Error::ErrNotStarted);
        }
        if self.section > sec {
            return Err(Error::ErrSectionDone);
        }
        self.res_header_valid = false;
        if self.index == self.header.count(sec) as usize {
            self.index = 0;
            self.section = Section::from(1 + self.section as u8);
            return Err(Error::ErrSectionDone);
        }
        Ok(())
    }

    fn resource_header(&mut self, sec: Section) -> Result<ResourceHeader> {
        if self.res_header_valid {
            return Ok(self.res_header.clone());
        }
        self.check_advance(sec)?;
        let mut hdr = ResourceHeader::default();
        let off = hdr.unpack(self.msg, self.off, 0)?;

        self.res_header_valid = true;
        self.res_header = hdr.clone();
        self.off = off;
        Ok(hdr)
    }

    // question parses a single question.
    pub fn question(&mut self) -> Result<Question> {
        self.check_advance(Section::Questions)?;
        let mut name = Name::new("")?;
        let mut off = name.unpack(self.msg, self.off)?;
        let mut typ = DnsType::Unsupported;
        off = typ.unpack(self.msg, off)?;
        let mut class = DnsClass::default();
        off = class.unpack(self.msg, off)?;
        self.off = off;
        self.index += 1;
        Ok(Question { name, typ, class })
    }

    // answer_header parses a single answer ResourceHeader.
    pub fn answer_header(&mut self) -> Result<ResourceHeader> {
        self.resource_header(Section::Answers)
    }
}
