
// EndpointDependencyType defines a type of behavioral dependency on the
// remote endpoint's IP address or port number. This is used for the two
// kinds of behaviors:
//  - Port Mapping behavior
//  - Filtering behavior
// See: https://tools.ietf.org/html/rfc4787
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum EndpointDependencyType {
    // EndpointIndependent means the behavior is independent of the endpoint's address or port
    EndpointIndependent,
}

impl Default for EndpointDependencyType {
    fn default() -> Self {
        EndpointDependencyType::EndpointIndependent
    }
}

// NATMode defines basic behavior of the NAT
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum NatMode {
    // NATModeNormal means the NAT behaves as a standard NAPT (RFC 2663).
    Normal,
}

impl Default for NatMode {
    fn default() -> Self {
        NatMode::Normal
    }
}