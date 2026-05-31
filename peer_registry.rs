//! Peer registry — tracks which players are active on this adapter.
//!
//! No longer stores PeerHandles (the outbound pump owns those now).
//! Just tracks player_id → session_id for Join dedup and Leave routing.

use std::sync::Arc;
use dashmap::DashMap;

#[derive(Clone, Default)]
pub struct PeerRegistry {
    /// player_id → session_id
    inner: Arc<DashMap<String, u64>>,
}

impl PeerRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn insert_id(&self, player_id: String, session_id: u64) {
        self.inner.insert(player_id, session_id);
    }

    pub fn remove(&self, player_id: &str) {
        self.inner.remove(player_id);
    }

    pub fn contains(&self, player_id: &str) -> bool {
        self.inner.contains_key(player_id)
    }

    pub fn session_id_for(&self, player_id: &str) -> Option<u64> {
        self.inner.get(player_id).map(|v| *v)
    }
}
