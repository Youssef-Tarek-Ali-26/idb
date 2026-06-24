use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const WIRE_MAGIC: u16 = 0x4944; // 'ID'
pub const HEADER_BYTES: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl ProtocolVersion {
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionHello {
    pub client_name: String,
    pub min_version: ProtocolVersion,
    pub max_version: ProtocolVersion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionWelcome {
    pub negotiated_version: ProtocolVersion,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WireError {
    #[error(
        "incompatible protocol versions: client {client_min:?}-{client_max:?}, server {server_min:?}-{server_max:?}"
    )]
    IncompatibleVersion {
        client_min: ProtocolVersion,
        client_max: ProtocolVersion,
        server_min: ProtocolVersion,
        server_max: ProtocolVersion,
    },
    #[error("invalid protocol range: min {min:?} is greater than max {max:?}")]
    InvalidRange {
        min: ProtocolVersion,
        max: ProtocolVersion,
    },
    #[error("invalid wire magic: expected 0x{expected:04X}, got 0x{actual:04X}")]
    InvalidMagic { expected: u16, actual: u16 },
    #[error("truncated frame: expected at least {expected} bytes, got {actual}")]
    TruncatedFrame { expected: usize, actual: usize },
    #[error("frame payload length mismatch: header {expected}, actual {actual}")]
    PayloadLengthMismatch { expected: usize, actual: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolRange {
    pub min: ProtocolVersion,
    pub max: ProtocolVersion,
}

impl ProtocolRange {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.min > self.max {
            return Err(WireError::InvalidRange {
                min: self.min,
                max: self.max,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireFrameHeader {
    pub version: ProtocolVersion,
    pub opcode: u16,
    pub request_id: u64,
    pub payload_len: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireFrame {
    pub header: WireFrameHeader,
    pub payload: Vec<u8>,
}

impl WireFrame {
    pub fn new(version: ProtocolVersion, opcode: u16, request_id: u64, payload: Vec<u8>) -> Self {
        Self {
            header: WireFrameHeader {
                version,
                opcode,
                request_id,
                payload_len: payload.len() as u32,
            },
            payload,
        }
    }
}

pub fn negotiate_protocol(
    client: ProtocolRange,
    server: ProtocolRange,
) -> Result<ProtocolVersion, WireError> {
    client.validate()?;
    server.validate()?;

    let floor = client.min.max(server.min);
    let ceiling = client.max.min(server.max);

    if floor > ceiling {
        return Err(WireError::IncompatibleVersion {
            client_min: client.min,
            client_max: client.max,
            server_min: server.min,
            server_max: server.max,
        });
    }

    Ok(ceiling)
}

pub fn negotiate_handshake(
    hello: &SessionHello,
    server_supported: ProtocolRange,
) -> Result<SessionWelcome, WireError> {
    let client_range = ProtocolRange {
        min: hello.min_version,
        max: hello.max_version,
    };
    let negotiated_version = negotiate_protocol(client_range, server_supported)?;
    Ok(SessionWelcome { negotiated_version })
}

pub fn encode_frame(frame: &WireFrame) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_BYTES + frame.payload.len());
    out.extend_from_slice(&WIRE_MAGIC.to_be_bytes());
    out.extend_from_slice(&frame.header.version.major.to_be_bytes());
    out.extend_from_slice(&frame.header.version.minor.to_be_bytes());
    out.extend_from_slice(&frame.header.opcode.to_be_bytes());
    out.extend_from_slice(&frame.header.request_id.to_be_bytes());
    out.extend_from_slice(&frame.header.payload_len.to_be_bytes());
    out.extend_from_slice(&frame.payload);
    out
}

pub fn decode_frame(buffer: &[u8]) -> Result<WireFrame, WireError> {
    if buffer.len() < HEADER_BYTES {
        return Err(WireError::TruncatedFrame {
            expected: HEADER_BYTES,
            actual: buffer.len(),
        });
    }

    let magic = u16::from_be_bytes([buffer[0], buffer[1]]);
    if magic != WIRE_MAGIC {
        return Err(WireError::InvalidMagic {
            expected: WIRE_MAGIC,
            actual: magic,
        });
    }

    let version = ProtocolVersion {
        major: u16::from_be_bytes([buffer[2], buffer[3]]),
        minor: u16::from_be_bytes([buffer[4], buffer[5]]),
    };
    let opcode = u16::from_be_bytes([buffer[6], buffer[7]]);
    let request_id = u64::from_be_bytes([
        buffer[8], buffer[9], buffer[10], buffer[11], buffer[12], buffer[13], buffer[14],
        buffer[15],
    ]);
    let payload_len = u32::from_be_bytes([buffer[16], buffer[17], buffer[18], buffer[19]]);

    let expected_total = HEADER_BYTES + payload_len as usize;
    if buffer.len() < expected_total {
        return Err(WireError::TruncatedFrame {
            expected: expected_total,
            actual: buffer.len(),
        });
    }
    if buffer.len() > expected_total {
        return Err(WireError::PayloadLengthMismatch {
            expected: expected_total,
            actual: buffer.len(),
        });
    }

    let payload = buffer[HEADER_BYTES..].to_vec();
    Ok(WireFrame {
        header: WireFrameHeader {
            version,
            opcode,
            request_id,
            payload_len,
        },
        payload,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        decode_frame, encode_frame, negotiate_handshake, negotiate_protocol, ProtocolRange,
        ProtocolVersion, SessionHello, WireError, WireFrame, WireFrameHeader, HEADER_BYTES,
    };

    #[test]
    fn negotiation_picks_highest_common_version() {
        let client = ProtocolRange {
            min: ProtocolVersion::new(1, 0),
            max: ProtocolVersion::new(2, 3),
        };
        let server = ProtocolRange {
            min: ProtocolVersion::new(2, 0),
            max: ProtocolVersion::new(3, 0),
        };

        let negotiated = negotiate_protocol(client, server).expect("negotiate");
        assert_eq!(negotiated, ProtocolVersion::new(2, 3));
    }

    #[test]
    fn negotiation_rejects_non_overlapping_ranges() {
        let client = ProtocolRange {
            min: ProtocolVersion::new(1, 0),
            max: ProtocolVersion::new(1, 9),
        };
        let server = ProtocolRange {
            min: ProtocolVersion::new(2, 0),
            max: ProtocolVersion::new(2, 4),
        };

        let err = negotiate_protocol(client, server).expect_err("must fail");
        assert!(matches!(err, WireError::IncompatibleVersion { .. }));
    }

    #[test]
    fn negotiation_rejects_invalid_ranges() {
        let client = ProtocolRange {
            min: ProtocolVersion::new(2, 2),
            max: ProtocolVersion::new(1, 5),
        };
        let server = ProtocolRange {
            min: ProtocolVersion::new(1, 0),
            max: ProtocolVersion::new(3, 0),
        };

        let err = negotiate_protocol(client, server).expect_err("must fail");
        assert!(matches!(err, WireError::InvalidRange { .. }));
    }

    #[test]
    fn handshake_returns_welcome_with_negotiated_version() {
        let hello = SessionHello {
            client_name: "sdk-rust".to_string(),
            min_version: ProtocolVersion::new(1, 0),
            max_version: ProtocolVersion::new(1, 7),
        };
        let server_supported = ProtocolRange {
            min: ProtocolVersion::new(1, 5),
            max: ProtocolVersion::new(2, 0),
        };

        let welcome = negotiate_handshake(&hello, server_supported).expect("handshake");
        assert_eq!(welcome.negotiated_version, ProtocolVersion::new(1, 7));
    }

    #[test]
    fn frame_encode_decode_roundtrip() {
        let frame = WireFrame {
            header: WireFrameHeader {
                version: ProtocolVersion::new(1, 1),
                opcode: 0x04,
                request_id: 42,
                payload_len: 3,
            },
            payload: vec![10, 20, 30],
        };

        let encoded = encode_frame(&frame);
        assert_eq!(encoded.len(), HEADER_BYTES + 3);

        let decoded = decode_frame(&encoded).expect("decode");
        assert_eq!(decoded, frame);
    }

    #[test]
    fn frame_decode_rejects_invalid_magic() {
        let mut bytes = vec![0x00, 0x00];
        bytes.extend_from_slice(&1u16.to_be_bytes());
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(&1u16.to_be_bytes());
        bytes.extend_from_slice(&1u64.to_be_bytes());
        bytes.extend_from_slice(&0u32.to_be_bytes());

        let err = decode_frame(&bytes).expect_err("decode must fail");
        assert!(matches!(err, WireError::InvalidMagic { .. }));
    }

    #[test]
    fn frame_decode_rejects_truncated_payload() {
        let frame = WireFrame::new(ProtocolVersion::new(1, 0), 7, 99, vec![1, 2, 3, 4]);
        let mut encoded = encode_frame(&frame);
        encoded.pop();

        let err = decode_frame(&encoded).expect_err("decode must fail");
        assert!(matches!(err, WireError::TruncatedFrame { .. }));
    }

    #[test]
    fn frame_decode_rejects_extra_bytes() {
        let frame = WireFrame::new(ProtocolVersion::new(1, 0), 7, 99, vec![1, 2, 3, 4]);
        let mut encoded = encode_frame(&frame);
        encoded.extend_from_slice(&[55, 66]);

        let err = decode_frame(&encoded).expect_err("decode must fail");
        assert!(matches!(err, WireError::PayloadLengthMismatch { .. }));
    }
}
