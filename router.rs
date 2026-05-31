//! CGX_Cube Router — zero-state packet router.
//!
//! Responsibilities:
//!   - Maintain session_id → [(Platform, tx)] registry
//!   - Route CRP packets to all peers except sender
//!   - Handle Join/Leave lifecycle
//!   - Broadcast SessionState on roster change
//!
//! What the router does NOT do:
//!   - Game logic
//!   - Packet translation
//!   - Authentication
//!
//! All hot-path operations are O(1) DashMap lookups.

use std::sync::Arc;
use bytes::Bytes;
use dashmap::DashMap;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::{CrpPacket, CrpEventType, Platform};

pub type PeerTx = mpsc::UnboundedSender<Bytes>;

/// Handle returned to an adapter when it registers a peer.
/// Drop this to signal disconnect.
pub struct PeerHandle {
    pub session_id: u64,
    pub platform:   Platform,
    pub outbound:   mpsc::UnboundedReceiver<Bytes>,
    router:         Arc<RouterInner>,
}

impl Drop for PeerHandle {
    fn drop(&mut self) {
        self.router.leave(self.session_id, self.platform);
        info!("Router: {:?} left session {}", self.platform, self.session_id);
    }
}

struct RouterInner {
    /// session_id → Vec<(Platform, tx)>
    sessions: DashMap<u64, Vec<(Platform, PeerTx)>>,
}

impl RouterInner {
    fn leave(&self, session_id: u64, platform: Platform) {
        if let Some(mut peers) = self.sessions.get_mut(&session_id) {
            peers.retain(|(p, _)| *p != platform);
        }
        self.sessions.retain(|_, peers| !peers.is_empty());
    }
}

#[derive(Clone)]
pub struct Router {
    inner: Arc<RouterInner>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RouterInner {
                sessions: DashMap::new(),
            }),
        }
    }

    /// Register a peer for a session.
    /// Called when a Join packet arrives from an adapter.
    /// Returns a PeerHandle — drop it to deregister.
    pub fn join(&self, session_id: u64, platform: Platform) -> PeerHandle {
        let (tx, rx) = mpsc::unbounded_channel();
        self.inner.sessions
            .entry(session_id)
            .or_default()
            .push((platform, tx));

        info!("Router: {:?} joined session {}", platform, session_id);
        self.broadcast_session_state(session_id);

        PeerHandle {
            session_id,
            platform,
            outbound: rx,
            router: self.inner.clone(),
        }
    }

    /// Route a CRP packet to all peers in the session except the sender.
    /// Fire-and-forget — dead channels are pruned lazily on next write.
    pub fn route(&self, pkt: &CrpPacket) {
        let encoded = Bytes::from(pkt.encode());

        let Some(peers) = self.inner.sessions.get(&pkt.session_id) else {
            warn!(
                "Router: packet for unknown session {} from {:?}",
                pkt.session_id, pkt.platform
            );
            return;
        };

        let mut dead_indices = vec![];
        for (i, (platform, tx)) in peers.iter().enumerate() {
            if *platform == pkt.platform { continue; } // no echo

            if tx.send(encoded.clone()).is_err() {
                dead_indices.push(i);
                warn!("Router: dead peer {:?} in session {}", platform, pkt.session_id);
            } else {
                debug!(
                    "Router: {:?} → {:?} | {:?} | {} bytes",
                    pkt.platform, platform, pkt.event_type, encoded.len()
                );
            }
        }

        // Lazy dead peer cleanup
        if !dead_indices.is_empty() {
            drop(peers); // release read lock before write
            if let Some(mut peers_mut) = self.inner.sessions.get_mut(&pkt.session_id) {
                let mut i = 0usize;
                peers_mut.retain(|_| {
                    let keep = !dead_indices.contains(&i);
                    i += 1;
                    keep
                });
            }
        }
    }

    /// Broadcast to ALL peers in a session including the sender.
    /// Used for SessionState updates so everyone has the full roster.
    pub fn broadcast(&self, session_id: u64, data: Bytes) {
        if let Some(peers) = self.inner.sessions.get(&session_id) {
            for (_, tx) in peers.iter() {
                let _ = tx.send(data.clone());
            }
        }
    }

    /// Build and broadcast a SessionState packet whenever roster changes.
    fn broadcast_session_state(&self, session_id: u64) {
        let Some(peers) = self.inner.sessions.get(&session_id) else { return };

        // Roster: Vec<{player_platform}> — adapters will enrich with player_id
        let roster: Vec<String> = peers.iter()
            .map(|(p, _)| p.to_string())
            .collect();

        let payload = serde_json::json!({
            "session_id": session_id,
            "roster": roster,
        });

        let Ok(payload_bytes) = serde_json::to_vec(&payload) else { return };

        let pkt = CrpPacket {
            version:    crate::CRP_VERSION,
            platform:   Platform::Roblox, // sender field unused for broadcasts
            session_id,
            seq:        0,
            event_type: CrpEventType::SessionState,
            payload:    payload_bytes,
        };
        let encoded = Bytes::from(pkt.encode());
        drop(peers);

        self.broadcast(session_id, encoded);
    }

    pub fn session_count(&self) -> usize { self.inner.sessions.len() }
    pub fn peer_count(&self)    -> usize {
        self.inner.sessions.iter().map(|e| e.value().len()).sum()
    }
}

impl Default for Router {
    fn default() -> Self { Self::new() }
}
