
#[derive(Default, Clone)]
pub(crate) struct Candidates {
    pub(crate) multicast_dns_host_name: String,
    pub(crate) username_fragment: String,
    pub(crate) password: String,
}

/// SettingEngine allows influencing behavior in ways that are not
/// supported by the WebRTC API. This allows us to support additional
/// use-cases without deviating from the WebRTC API elsewhere.
#[derive(Default, Clone)]
pub(crate) struct SettingEngine {
    pub(crate) candidates: Candidates,
}

impl SettingEngine {
    pub(crate) fn new() -> Self {
        Self {
            candidates: Candidates {
                multicast_dns_host_name: "".to_string(),
                username_fragment: "".to_string(),
                password: "".to_string()
            }
        }
    }
}
