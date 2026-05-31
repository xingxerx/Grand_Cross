//! Dispatch loop — routes inbound packets to the Router.
//! Handles Join/Leave lifecycle, spawns outbound pumps.

use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::{CrpPacket, CrpEventType, Platform, adapters::ConnectionRegistry};

use crate::router::Router;
use crate::session_manager::{SessionManager, JoinResult};
use crate::peer_registry::PeerRegistry;
use crate::outbound_pump::spawn_outbound_pump;

pub fn spawn_dispatch_loop(
    mut inbound: mpsc::Receiver<CrpPacket>,
    platform:    Platform,
    router:      Router,
    sessions:    SessionManager,
    registry:    ConnectionRegistry,
) {
    let peers = PeerRegistry::new();

    tokio::spawn(async move {
        info!("Dispatch loop started for {:?}", platform);

        while let Some(pkt) = inbound.recv().await {
            match pkt.event_type {
                CrpEventType::Join  => handle_join(&pkt, platform, &router, &sessions, &peers, &registry),
                CrpEventType::Leave => handle_leave(&pkt, &sessions, &peers),
                CrpEventType::Ping  => debug!("Ping from {:?} session {}", platform, pkt.session_id),
                _                   => router.route(&pkt),
            }
        }

        info!("Dispatch loop ended for {:?}", platform);
    });
}

fn handle_join(
    pkt:      &CrpPacket,
    platform: Platform,
    router:   &Router,
    sessions: &SessionManager,
    peers:    &PeerRegistry,
    registry: &ConnectionRegistry,
) {
    let (player_id, invite_session_id) = parse_join_payload(&pkt.payload)
        .unwrap_or_else(|| {
            warn!("Dispatch: malformed Join from {:?}", platform);
            (format!("unknown_{}", pkt.seq), None)
        });

    if peers.contains(&player_id) {
        debug!("Dispatch: '{}' already joined", player_id);
        return;
    }

    match sessions.on_join(&player_id, invite_session_id) {
        JoinResult::Joined(sid) | JoinResult::AlreadyJoined(sid) => {
            let handle = router.join(sid, platform);
            // Spawn pump — owns the handle, bridges router → connection
            spawn_outbound_pump(handle, registry.clone(), player_id.clone());
            // NOTE: PeerRegistry no longer stores the handle (pump owns it).
            // We still track player_id for Leave dedup.
            peers.insert_id(player_id.clone(), sid);
            info!("Dispatch: {:?} '{}' → session {}", platform, player_id, sid);
        }
        JoinResult::SessionFull     => warn!("Dispatch: '{}' — session full", player_id),
        JoinResult::RelaySaturated  => warn!("Dispatch: '{}' — relay saturated", player_id),
    }
}

fn handle_leave(
    pkt:      &CrpPacket,
    sessions: &SessionManager,
    peers:    &PeerRegistry,
) {
    let player_id = parse_leave_payload(&pkt.payload)
        .unwrap_or_else(|| format!("unknown_{}", pkt.seq));

    peers.remove(&player_id);
    sessions.on_leave(&player_id);
    // PeerHandle in the pump task drops when pump ends → Router::leave fires
}

fn parse_join_payload(bytes: &[u8]) -> Option<(String, Option<u64>)> {
    let v: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let player_id = v["player_id"].as_str()?.to_string();
    let invite    = v["invite_session_id"].as_u64();
    Some((player_id, invite))
}

fn parse_leave_payload(bytes: &[u8]) -> Option<String> {
    let v: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    Some(v["player_id"].as_str()?.to_string())
}
