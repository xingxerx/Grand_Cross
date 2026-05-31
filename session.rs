use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::Platform;

/// A live crossplay session — N players across up to 3 platforms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CubeSession {
    pub id:         u64,
    pub created_at: i64,
    /// player_id → platform
    pub roster:     HashMap<String, Platform>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionEvent {
    PlayerJoined  { session_id: u64, player_id: String, platform: Platform },
    PlayerLeft    { session_id: u64, player_id: String },
    SessionClosed { session_id: u64 },
}
