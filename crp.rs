//! CubeRelay Packet (CRP) — unified cross-platform envelope.
//!
//! Wire format (little-endian):
//!   [0..2]   magic:       0xCB 0x01
//!   [2]      version:     u8
//!   [3]      platform:    u8  (Platform enum)
//!   [4..12]  session_id:  u64
//!   [12..16] seq:         u32
//!   [16]     event_type:  u8  (CrpEventType)
//!   [17..19] payload_len: u16
//!   [19..]   payload:     [u8; payload_len]

use serde::{Deserialize, Serialize};
use crate::Platform;

pub const CRP_MAGIC: [u8; 2]  = [0xCB, 0x01];
pub const CRP_VERSION: u8      = 1;
pub const CRP_HEADER_LEN: usize = 19;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum CrpEventType {
    Move         = 0x01, // position + rotation
    Chat         = 0x02, // text message
    Voice        = 0x03, // Opus audio frame
    Join         = 0x04, // player joined session
    Leave        = 0x05, // player left session
    Ping         = 0x06, // keepalive
    Pong         = 0x07, // keepalive ack
    Action       = 0x08, // platform-specific game action
    SessionState = 0x09, // full roster broadcast
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrpPacket {
    pub version:    u8,
    pub platform:   Platform,
    pub session_id: u64,
    pub seq:        u32,
    pub event_type: CrpEventType,
    pub payload:    Vec<u8>,
}

impl CrpPacket {
    pub fn encode(&self) -> Vec<u8> {
        let payload_len = self.payload.len() as u16;
        let mut buf = Vec::with_capacity(CRP_HEADER_LEN + self.payload.len());
        buf.extend_from_slice(&CRP_MAGIC);
        buf.push(self.version);
        buf.push(self.platform as u8);
        buf.extend_from_slice(&self.session_id.to_le_bytes());
        buf.extend_from_slice(&self.seq.to_le_bytes());
        buf.push(self.event_type as u8);
        buf.extend_from_slice(&payload_len.to_le_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    pub fn decode(buf: &[u8]) -> Option<Self> {
        if buf.len() < CRP_HEADER_LEN  { return None; }
        if buf[0..2] != CRP_MAGIC      { return None; }

        let version    = buf[2];
        let platform   = match buf[3] {
            0x01 => Platform::Roblox,
            0x02 => Platform::Minecraft,
            0x03 => Platform::Hytale,
            _    => return None,
        };
        let session_id = u64::from_le_bytes(buf[4..12].try_into().ok()?);
        let seq        = u32::from_le_bytes(buf[12..16].try_into().ok()?);
        let event_type = match buf[16] {
            0x01 => CrpEventType::Move,
            0x02 => CrpEventType::Chat,
            0x03 => CrpEventType::Voice,
            0x04 => CrpEventType::Join,
            0x05 => CrpEventType::Leave,
            0x06 => CrpEventType::Ping,
            0x07 => CrpEventType::Pong,
            0x08 => CrpEventType::Action,
            0x09 => CrpEventType::SessionState,
            _    => return None,
        };
        let payload_len = u16::from_le_bytes(buf[17..19].try_into().ok()?) as usize;
        if buf.len() < CRP_HEADER_LEN + payload_len { return None; }
        let payload = buf[CRP_HEADER_LEN..CRP_HEADER_LEN + payload_len].to_vec();

        Some(Self { version, platform, session_id, seq, event_type, payload })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_chat() {
        let pkt = CrpPacket {
            version:    CRP_VERSION,
            platform:   Platform::Minecraft,
            session_id: 0xDEADBEEFCAFEBABE,
            seq:        42,
            event_type: CrpEventType::Chat,
            payload:    b"hello crossplay".to_vec(),
        };
        let encoded = pkt.encode();
        let decoded  = CrpPacket::decode(&encoded).expect("decode failed");
        assert_eq!(decoded.platform,   Platform::Minecraft);
        assert_eq!(decoded.session_id, 0xDEADBEEFCAFEBABE);
        assert_eq!(decoded.seq,        42);
        assert_eq!(decoded.payload,    b"hello crossplay");
    }

    #[test]
    fn rejects_bad_magic() {
        let mut buf = vec![0x00u8; CRP_HEADER_LEN + 4];
        assert!(CrpPacket::decode(&buf).is_none());
        buf[0] = 0xCB; buf[1] = 0x01;
        // still none — zero platform byte 0x00 is invalid
        assert!(CrpPacket::decode(&buf).is_none());
    }
}
