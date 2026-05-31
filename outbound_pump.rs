//! Outbound pump — drains PeerHandle.outbound → ConnectionRegistry.
//!
//! One pump task runs per player, spawned at Join time.
//! It bridges the gap between the router's per-player channel
//! and the adapter's per-connection send path.
//!
//! Lifecycle:
//!   Join  → dispatch → spawn_outbound_pump(handle, registry, player_id)
//!   Leave → PeerHandle dropped → handle.outbound closes → pump task ends

use bytes::Bytes;
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::ConnectionRegistry;
use crate::router::PeerHandle;

/// Spawn an outbound pump for one player.
/// Consumes the PeerHandle — owns it until the player leaves.
pub fn spawn_outbound_pump(
    mut handle:    PeerHandle,
    registry:  ConnectionRegistry,
    player_id: String,
) {
    tokio::spawn(async move {
        // PeerHandle.outbound is UnboundedReceiver<Bytes>
        // It closes when the Router drops the sender (peer left / session closed)
        debug!("Outbound pump started for '{}'", player_id);

        while let Some(bytes) = handle.outbound.recv().await {
            if !registry.send_to(&player_id, bytes) {
                // Connection is gone — player disconnected at the transport layer
                // before a clean Leave packet arrived. That's fine — RAII handles cleanup.
                break;
            }
        }

        info!("Outbound pump ended for '{}'", player_id);
        // PeerHandle drops here → Router::leave fires via RAII
    });
}
