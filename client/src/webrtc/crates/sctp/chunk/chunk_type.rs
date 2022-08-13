use std::fmt;

// chunkType is an enum for SCTP Chunk Type field
// This field identifies the type of information contained in the
// Chunk Value field.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct ChunkType(pub u8);

pub const CT_PAYLOAD_DATA: ChunkType = ChunkType(0);
pub const CT_INIT: ChunkType = ChunkType(1);
pub const CT_INIT_ACK: ChunkType = ChunkType(2);
pub const CT_SACK: ChunkType = ChunkType(3);
pub const CT_HEARTBEAT: ChunkType = ChunkType(4);
pub const CT_HEARTBEAT_ACK: ChunkType = ChunkType(5);
pub const CT_ABORT: ChunkType = ChunkType(6);
pub const CT_SHUTDOWN: ChunkType = ChunkType(7);
pub const CT_SHUTDOWN_ACK: ChunkType = ChunkType(8);
pub const CT_ERROR: ChunkType = ChunkType(9);
pub const CT_COOKIE_ECHO: ChunkType = ChunkType(10);
pub const CT_COOKIE_ACK: ChunkType = ChunkType(11);
pub const CT_CWR: ChunkType = ChunkType(13);
pub const CT_SHUTDOWN_COMPLETE: ChunkType = ChunkType(14);
pub const CT_RECONFIG: ChunkType = ChunkType(130);
pub const CT_FORWARD_TSN: ChunkType = ChunkType(192);

impl fmt::Display for ChunkType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let others = format!("Unknown ChunkType: {}", self.0);
        let s = match *self {
            CT_PAYLOAD_DATA => "DATA",
            CT_INIT => "INIT",
            CT_INIT_ACK => "INIT-ACK",
            CT_SACK => "SACK",
            CT_HEARTBEAT => "HEARTBEAT",
            CT_HEARTBEAT_ACK => "HEARTBEAT-ACK",
            CT_ABORT => "ABORT",
            CT_SHUTDOWN => "SHUTDOWN",
            CT_SHUTDOWN_ACK => "SHUTDOWN-ACK",
            CT_ERROR => "ERROR",
            CT_COOKIE_ECHO => "COOKIE-ECHO",
            CT_COOKIE_ACK => "COOKIE-ACK",
            CT_CWR => "ECNE", // Explicit Congestion Notification Echo
            CT_SHUTDOWN_COMPLETE => "SHUTDOWN-COMPLETE",
            CT_RECONFIG => "RECONFIG", // Re-configuration
            CT_FORWARD_TSN => "FORWARD-TSN",
            _ => others.as_str(),
        };
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_chunk_type_string() {
        let tests = vec![
            (CT_PAYLOAD_DATA, "DATA"),
            (CT_INIT, "INIT"),
            (CT_INIT_ACK, "INIT-ACK"),
            (CT_SACK, "SACK"),
            (CT_HEARTBEAT, "HEARTBEAT"),
            (CT_HEARTBEAT_ACK, "HEARTBEAT-ACK"),
            (CT_ABORT, "ABORT"),
            (CT_SHUTDOWN, "SHUTDOWN"),
            (CT_SHUTDOWN_ACK, "SHUTDOWN-ACK"),
            (CT_ERROR, "ERROR"),
            (CT_COOKIE_ECHO, "COOKIE-ECHO"),
            (CT_COOKIE_ACK, "COOKIE-ACK"),
            (CT_CWR, "ECNE"),
            (CT_SHUTDOWN_COMPLETE, "SHUTDOWN-COMPLETE"),
            (CT_RECONFIG, "RECONFIG"),
            (CT_FORWARD_TSN, "FORWARD-TSN"),
            (ChunkType(255), "Unknown ChunkType: 255"),
        ];

        for (ct, expected) in tests {
            assert_eq!(
                ct.to_string(),
                expected,
                "failed to stringify chunkType {}, expected {}",
                ct,
                expected
            );
        }
    }
}
