//! Session manager — sits between adapters and the router.
//!
//! Responsibilities:
//!   - Generate session IDs
//!   - Handle CrpEventType::Join handshake
//!   - Map player_id → session_id
//!   - Enforce session limits
//!
//! Flow:
//!   1. Adapter receives Join packet from platform client
//!   2. Adapter calls SessionManager::on_join()
//!   3. SessionManager returns session_id (new or existing)
//!   4. Adapter calls Router::join(session_id, platform)
//!   5. Adapter pumps inbound packets into Router::route()

use std::sync::Arc;
use dashmap::DashMap;
use tracing::{info, warn};

/// Max players per session across all platforms.
const MAX_SESSION_PLAYERS: usize = 9; // 3v3v3

/// Max concurrent sessions on this relay instance.
const MAX_SESSIONS: usize = 1000;

#[derive(Clone)]
pub struct SessionManager {
    /// player_id → session_id
    player_sessions: Arc<DashMap<String, u64>>,
    /// session_id → player count
    session_counts:  Arc<DashMap<u64, usize>>,
    /// Monotonic session ID counter
    next_id:         Arc<std::sync::atomic::AtomicU64>,
}

pub enum JoinResult {
    /// Player joined an existing or new session. Here's the session_id.
    Joined(u64),
    /// Player is already in this session — idempotent, return same ID.
    AlreadyJoined(u64),
    /// Session is full.
    SessionFull,
    /// Relay is at capacity.
    RelaySaturated,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            player_sessions: Arc::new(DashMap::new()),
            session_counts:  Arc::new(DashMap::new()),
            next_id:         Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    /// Called when a Join packet arrives.
    ///
    /// If `invite_session_id` is Some, join that session.
    /// If None, create a new session.
    pub fn on_join(
        &self,
        player_id: &str,
        invite_session_id: Option<u64>,
    ) -> JoinResult {
        // Already in a session?
        if let Some(existing) = self.player_sessions.get(player_id) {
            return JoinResult::AlreadyJoined(*existing);
        }

        let session_id = match invite_session_id {
            Some(id) => {
                // Validate session exists and has room
                let count = self.session_counts
                    .get(&id)
                    .map(|c| *c)
                    .unwrap_or(0);

                if count >= MAX_SESSION_PLAYERS {
                    warn!("SessionManager: session {} is full ({} players)", id, count);
                    return JoinResult::SessionFull;
                }
                id
            }
            None => {
                // Create new session
                if self.session_counts.len() >= MAX_SESSIONS {
                    warn!("SessionManager: relay at capacity ({} sessions)", MAX_SESSIONS);
                    return JoinResult::RelaySaturated;
                }
                let id = self.next_id
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                self.session_counts.insert(id, 0);
                info!("SessionManager: created session {}", id);
                id
            }
        };

        self.player_sessions.insert(player_id.to_string(), session_id);
        self.session_counts
            .entry(session_id)
            .and_modify(|c| *c += 1)
            .or_insert(1);

        info!("SessionManager: player '{}' joined session {}", player_id, session_id);
        JoinResult::Joined(session_id)
    }

    /// Called when a Leave packet arrives or connection drops.
    pub fn on_leave(&self, player_id: &str) {
        if let Some((_, session_id)) = self.player_sessions.remove(player_id) {
            if let Some(mut count) = self.session_counts.get_mut(&session_id) {
                *count = count.saturating_sub(1);
            }
            // Remove empty sessions
            self.session_counts.retain(|_, c| *c > 0);
            info!("SessionManager: player '{}' left session {}", player_id, session_id);
        }
    }

    pub fn session_id_for(&self, player_id: &str) -> Option<u64> {
        self.player_sessions.get(player_id).map(|v| *v)
    }

    pub fn active_sessions(&self) -> usize { self.session_counts.len() }
    pub fn active_players(&self)  -> usize { self.player_sessions.len() }
}

impl Default for SessionManager {
    fn default() -> Self { Self::new() }
}
