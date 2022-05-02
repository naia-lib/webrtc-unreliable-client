use std::sync::Arc;

use super::udp_mux::UDPMux;

#[derive(Default, Clone)]
pub struct EphemeralUDP {
    port_min: u16,
    port_max: u16,
}

impl EphemeralUDP {

    pub fn port_min(&self) -> u16 {
        self.port_min
    }

    pub fn port_max(&self) -> u16 {
        self.port_max
    }
}

/// Configuration for the underlying UDP network stack.
/// There are two ways to configure this Ephemeral and Muxed.
///
/// **Ephemeral mode**
///
/// In Ephemeral mode sockets are created and bound to random ports during ICE
/// gathering. The ports to use can be restricted by setting [`EphemeralUDP::port_min`] and
/// [`EphemeralEphemeralUDP::port_max`] in which case only ports in this range will be used.
///
/// **Muxed**
///
/// In muxed mode a single UDP socket is used and all connections are muxed over this single socket.
///
#[derive(Clone)]
pub enum UDPNetwork {
    Ephemeral(EphemeralUDP),
    Muxed(Arc<dyn UDPMux + Send + Sync>),
}

impl Default for UDPNetwork {
    fn default() -> Self {
        Self::Ephemeral(Default::default())
    }
}