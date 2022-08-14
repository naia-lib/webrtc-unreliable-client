
use super::direction::*;
use crate::webrtc::sdp::description::common::*;

use std::fmt;
use url::Url;

/// ExtMap represents the activation of a single RTP header extension
#[derive(Debug, Clone, Default)]
pub struct ExtMap {
    pub value: isize,
    pub direction: Direction,
    pub uri: Option<Url>,
    pub ext_attr: Option<String>,
}

impl fmt::Display for ExtMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut output = format!("{}", self.value);
        if self.direction != Direction::Unspecified {
            output += format!("/{}", self.direction).as_str();
        }

        if let Some(uri) = &self.uri {
            output += format!(" {}", uri).as_str();
        }

        if let Some(ext_attr) = &self.ext_attr {
            output += format!(" {}", ext_attr).as_str();
        }

        write!(f, "{}", output)
    }
}
