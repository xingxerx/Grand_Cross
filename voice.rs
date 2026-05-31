use serde::{Deserialize, Serialize};

/// Single Opus-encoded voice frame with filter metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceFrame {
    pub player_id:   String,
    pub opus_bytes:  Vec<u8>,
    pub duration_ms: u8,   // standard: 20ms
    pub muted:       bool, // filter replaced with silence
    pub vad_active:  bool, // voice activity detected
    pub seq:         u32,  // jitter buffer ordering
}

/// Filter decision for a voice frame.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterDecision {
    Pass, // forward unchanged
    Mute, // replace with silence (bleep)
    Drop, // below noise gate — discard
}
