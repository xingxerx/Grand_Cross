//! Light voice filter — noise gate, VAD, lookahead bleep mute.
//!
//! All decisions are made on raw PCM energy + zero-crossing rate.
//! No ML, no external API calls, no latency spikes.

use crate::{VoiceFrame, FilterDecision};
use std::collections::VecDeque;

/// Energy threshold below which a frame is considered silence.
/// Tune per deployment — lower = more sensitive noise gate.
const NOISE_GATE_THRESHOLD: f32 = 0.001;

/// Zero-crossing rate threshold for VAD confirmation.
const ZCR_THRESHOLD: f32 = 0.05;

/// How many consecutive silent frames before VAD goes inactive.
const VAD_HANGOVER_FRAMES: usize = 4; // 80ms

pub struct VoiceFilter {
    /// Sliding lookahead buffer for bleep timing.
    lookahead: VecDeque<VoiceFrame>,
    /// Frames remaining in current mute window.
    mute_remaining: usize,
    /// VAD hangover counter.
    vad_hangover: usize,
    /// Configurable keyword trigger list (populated from config).
    /// In practice this is used to flag a mute window when the STT
    /// transcript (if enabled) hits a keyword. Without STT, muting
    /// is energy-pattern based only.
    pub keyword_mute_windows: Vec<(u32, u32)>, // (start_seq, end_seq)
}

impl VoiceFilter {
    pub fn new() -> Self {
        Self {
            lookahead:           VecDeque::with_capacity(4),
            mute_remaining:      0,
            vad_hangover:        0,
            keyword_mute_windows: vec![],
        }
    }

    /// Process a decoded PCM frame (f32 samples, mono).
    /// Returns the filter decision for the frame.
    pub fn process_pcm(&mut self, seq: u32, samples: &[f32]) -> FilterDecision {
        let energy = rms_energy(samples);
        let zcr    = zero_crossing_rate(samples);

        // 1. Noise gate
        if energy < NOISE_GATE_THRESHOLD {
            if self.vad_hangover > 0 {
                self.vad_hangover -= 1;
            } else {
                return FilterDecision::Drop;
            }
        } else {
            self.vad_hangover = VAD_HANGOVER_FRAMES;
        }

        // 2. Keyword mute window check
        if self.is_muted(seq) {
            return FilterDecision::Mute;
        }

        FilterDecision::Pass
    }

    /// Register a mute window by sequence range (set by transcript layer or manual).
    pub fn mute_window(&mut self, start_seq: u32, end_seq: u32) {
        self.keyword_mute_windows.push((start_seq, end_seq));
        // Prune old windows (keep last 20)
        if self.keyword_mute_windows.len() > 20 {
            self.keyword_mute_windows.remove(0);
        }
    }

    fn is_muted(&self, seq: u32) -> bool {
        self.keyword_mute_windows
            .iter()
            .any(|(start, end)| seq >= *start && seq <= *end)
    }
}

/// Root mean square energy of a PCM frame.
fn rms_energy(samples: &[f32]) -> f32 {
    if samples.is_empty() { return 0.0; }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

/// Zero-crossing rate — fraction of consecutive sample pairs with sign change.
fn zero_crossing_rate(samples: &[f32]) -> f32 {
    if samples.len() < 2 { return 0.0; }
    let crossings = samples.windows(2)
        .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
        .count();
    crossings as f32 / (samples.len() - 1) as f32
}

impl Default for VoiceFilter {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_drops() {
        let mut f = VoiceFilter::new();
        let silence = vec![0.0f32; 480]; // 10ms @ 48kHz
        assert_eq!(f.process_pcm(0, &silence), FilterDecision::Drop);
    }

    #[test]
    fn mute_window_works() {
        let mut f = VoiceFilter::new();
        f.mute_window(10, 15);
        let loud = vec![0.5f32; 480];
        assert_eq!(f.process_pcm(12, &loud), FilterDecision::Mute);
        assert_eq!(f.process_pcm(20, &loud), FilterDecision::Pass);
    }
}
