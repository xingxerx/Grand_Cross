//! Minimal jitter buffer — reorders out-of-sequence Opus frames.
//!
//! Holds up to BUFFER_DEPTH frames, releases in seq order.
//! If a frame arrives too late (gap > MAX_GAP), it is dropped.

use std::collections::BTreeMap;
use crate::VoiceFrame;

const BUFFER_DEPTH: usize = 4;  // frames
const MAX_GAP: u32        = 10; // seq units

pub struct JitterBuffer {
    buf:      BTreeMap<u32, VoiceFrame>,
    next_seq: u32,
}

impl JitterBuffer {
    pub fn new() -> Self {
        Self { buf: BTreeMap::new(), next_seq: 0 }
    }

    /// Insert a frame. Returns any frames that are now ready to play.
    pub fn push(&mut self, frame: VoiceFrame) -> Vec<VoiceFrame> {
        let seq = frame.seq;

        // Drop frames that arrived too late
        if seq + MAX_GAP < self.next_seq {
            return vec![];
        }

        self.buf.insert(seq, frame);

        // Trim buffer to depth
        while self.buf.len() > BUFFER_DEPTH {
            self.buf.pop_first();
        }

        self.drain_ready()
    }

    fn drain_ready(&mut self) -> Vec<VoiceFrame> {
        let mut ready = vec![];
        while let Some((&seq, _)) = self.buf.first_key_value() {
            if seq == self.next_seq || self.buf.len() >= BUFFER_DEPTH {
                let frame = self.buf.remove(&seq).unwrap();
                self.next_seq = frame.seq + 1;
                ready.push(frame);
            } else {
                break;
            }
        }
        ready
    }
}

impl Default for JitterBuffer {
    fn default() -> Self { Self::new() }
}
