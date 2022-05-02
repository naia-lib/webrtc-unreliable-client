#[derive(Default, Clone)]
pub(crate) struct MediaEngineHeaderExtension;

/// A MediaEngine defines the codecs supported by a PeerConnection, and the
/// configuration of those codecs. A MediaEngine must not be shared between
/// PeerConnections.
#[derive(Default)]
pub struct MediaEngine;