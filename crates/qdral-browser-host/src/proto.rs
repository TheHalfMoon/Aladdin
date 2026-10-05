//! SG-000074 private local framed IPC vocabulary.
//!
//! Deskal and the host binary exchange length-prefixed JSON frames over
//! anonymous pipes only. There is no TCP, no HTTP, and no listener. The
//! vocabulary is closed: hello, ping, shutdown, and their replies. Unknown
//! frames are rejected without action. Capability frames do not exist in
//! this grain.

use crate::error::{HostError, UnavailableReason};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// Maximum frame body in bytes. Anything larger fails closed.
pub const MAX_FRAME_BYTES: usize = 64 * 1024;

/// Protocol generation bound into hello so mismatched peers refuse.
pub const PROTOCOL_GENERATION: u32 = 1;

/// Closed host request vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "frame", rename_all = "snake_case", deny_unknown_fields)]
pub enum HostRequest {
    Hello { generation: u32 },
    Ping { nonce: String },
    Shutdown,
}

/// Closed host reply vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "frame", rename_all = "snake_case", deny_unknown_fields)]
pub enum HostReply {
    Hello { generation: u32, host: String },
    Pong { nonce: String },
    Bye,
    Error { code: String },
}

/// Encode one frame: u32 little-endian length followed by JSON bytes.
pub fn encode_frame<T: Serialize>(value: &T) -> Result<Vec<u8>, HostError> {
    let body = serde_json::to_vec(value)
        .map_err(|err| HostError::Invalid(format!("frame encode: {err}")))?;
    if body.len() > MAX_FRAME_BYTES {
        return Err(HostError::Invalid("frame exceeds the size bound".into()));
    }
    let mut frame = Vec::with_capacity(4 + body.len());
    frame.extend_from_slice(&(body.len() as u32).to_le_bytes());
    frame.extend_from_slice(&body);
    Ok(frame)
}

/// Decode one frame from a reader, enforcing the size bound before
/// allocating the body.
pub fn decode_frame<T: serde::de::DeserializeOwned, R: Read>(
    reader: &mut R,
) -> Result<T, HostError> {
    let mut prefix = [0u8; 4];
    reader.read_exact(&mut prefix).map_err(|err| {
        if err.kind() == std::io::ErrorKind::UnexpectedEof {
            HostError::Unavailable(UnavailableReason::ProtocolViolation)
        } else {
            HostError::from(err)
        }
    })?;
    let len = u32::from_le_bytes(prefix) as usize;
    if len == 0 || len > MAX_FRAME_BYTES {
        return Err(HostError::Unavailable(UnavailableReason::ProtocolViolation));
    }
    let mut body = vec![0u8; len];
    reader
        .read_exact(&mut body)
        .map_err(|_| HostError::Unavailable(UnavailableReason::ProtocolViolation))?;
    serde_json::from_slice(&body)
        .map_err(|_| HostError::Unavailable(UnavailableReason::ProtocolViolation))
}

/// Write one frame to a writer.
pub fn write_frame<T: Serialize, W: Write>(writer: &mut W, value: &T) -> Result<(), HostError> {
    writer
        .write_all(&encode_frame(value)?)
        .map_err(HostError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn frames_round_trip_and_reject_unknown() {
        let request = HostRequest::Ping {
            nonce: "n-1".into(),
        };
        let bytes = encode_frame(&request).unwrap();
        let back: HostRequest = decode_frame(&mut Cursor::new(bytes)).unwrap();
        assert_eq!(back, request);
        let reply = HostReply::Pong {
            nonce: "n-1".into(),
        };
        let bytes = encode_frame(&reply).unwrap();
        let back: HostReply = decode_frame(&mut Cursor::new(bytes)).unwrap();
        assert_eq!(back, reply);
        // Unknown vocabulary never decodes.
        let raw = br#"{"frame":"navigate","url":"https://example.com"}"#;
        let mut framed = (raw.len() as u32).to_le_bytes().to_vec();
        framed.extend_from_slice(raw);
        assert!(decode_frame::<HostRequest, _>(&mut Cursor::new(framed)).is_err());
        // Oversized prefix fails before allocation.
        let mut big = (MAX_FRAME_BYTES as u32 + 1).to_le_bytes().to_vec();
        big.extend_from_slice(b"{}");
        assert!(decode_frame::<HostRequest, _>(&mut Cursor::new(big)).is_err());
        // Empty body fails.
        assert!(
            decode_frame::<HostRequest, _>(&mut Cursor::new(0u32.to_le_bytes().to_vec())).is_err()
        );
    }
}
