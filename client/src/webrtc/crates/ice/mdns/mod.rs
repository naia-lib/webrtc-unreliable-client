use uuid::Uuid;

/// Represents the different Multicast modes that ICE can run.
#[derive(PartialEq, Debug, Copy, Clone)]
pub(crate) enum MulticastDnsMode {
    Unspecified,

    /// Means remote mDNS candidates will be discarded, and local host candidates will use IPs.
    Disabled,

    /// Means remote mDNS candidates will be accepted, and local host candidates will use IPs.
    QueryOnly,

    /// Means remote mDNS candidates will be accepted, and local host candidates will use mDNS.
    QueryAndGather,
}

impl Default for MulticastDnsMode {
    fn default() -> Self {
        MulticastDnsMode::Unspecified
    }
}

pub(crate) fn generate_multicast_dns_name() -> String {
    // https://tools.ietf.org/id/draft-ietf-rtcweb-mdns-ice-candidates-02.html#gathering
    // The unique name MUST consist of a version 4 UUID as defined in [RFC4122], followed by “.local”.
    let u = Uuid::new_v4();
    format!("{}.local", u)
}
