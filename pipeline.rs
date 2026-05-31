//! Voice pipeline — orchestrates filter + jitter buffer per player.

use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::info;
use anyhow::Result;

use crate::VoiceFrame;
use crate::filter::VoiceFilter;

pub struct VoicePipeline {
    /// Per-player filter state
    filters: HashMap<String, VoiceFilter>,
    /// Processed frames ready to broadcast
    pub output_tx: mpsc::Sender<VoiceFrame>,
}

impl VoicePipeline {
    pub fn new(output_tx: mpsc::Sender<VoiceFrame>) -> Self {
        Self {
            filters: HashMap::new(),
            output_tx,
        }
    }

    /// Process an incoming voice frame through the filter.
    /// PCM decode from Opus happens upstream — this receives raw PCM.
    pub async fn ingest(&mut self, frame: VoiceFrame, pcm: &[f32]) -> Result<()> {
        let filter = self.filters
            .entry(frame.player_id.clone())
            .or_insert_with(VoiceFilter::new);

        let decision = filter.process_pcm(frame.seq, pcm);

        let out = VoiceFrame {
            muted:      decision == crate::FilterDecision::Mute,
            vad_active: decision != crate::FilterDecision::Drop,
            ..frame
        };

        // Drop silent frames — don't forward
        if decision == crate::FilterDecision::Drop {
            return Ok(());
        }

        let _ = self.output_tx.send(out).await;
        Ok(())
    }

    /// Register a mute window for a player (e.g. triggered by keyword detection).
    pub fn mute_player_window(&mut self, player_id: &str, start_seq: u32, end_seq: u32) {
        let filter = self.filters
            .entry(player_id.to_string())
            .or_insert_with(VoiceFilter::new);
        filter.mute_window(start_seq, end_seq);
        info!("Voice: mute window [{}-{}] registered for {}", start_seq, end_seq, player_id);
    }
}
