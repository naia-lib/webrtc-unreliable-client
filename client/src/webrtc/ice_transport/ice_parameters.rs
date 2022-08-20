use serde::{Deserialize, Serialize};

/// ICEParameters includes the ICE username fragment
/// and password and other ICE-related parameters.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct RTCIceParameters {
    pub(crate) username_fragment: String,
    pub(crate) password: String,
}
