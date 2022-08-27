use std::fmt;
use url::Url;

/// ExtMap represents the activation of a single RTP header extension
#[derive(Debug, Clone, Default)]
pub(crate) struct ExtMap {
    pub(crate) value: isize,
    pub(crate) uri: Option<Url>,
    pub(crate) ext_attr: Option<String>,
}

impl fmt::Display for ExtMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut output = format!("{}", self.value);

        if let Some(uri) = &self.uri {
            output += format!(" {}", uri).as_str();
        }

        if let Some(ext_attr) = &self.ext_attr {
            output += format!(" {}", ext_attr).as_str();
        }

        write!(f, "{}", output)
    }
}
