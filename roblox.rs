//! Roblox Adapter — WebSocket server + full outbound path.
//!
//! Inbound:  Roblox LuaU → JSON  → CrpPacket → inbound channel → dispatch
//! Outbound: router bytes → ConnectionRegistry → per-conn pump → JSON → Roblox
//!
//! Per-connection lifecycle:
//!   1. TCP accept → WebSocket upgrade
//!   2. Read first message — must be a Join with player_id
//!   3. Register player_id → conn_tx in ConnectionRegistry
//!   4. Spawn two tasks: reader (WS → inbound) + writer (conn_rx → WS)
//!   5. Either task ending cleans up the registry entry

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use crate::{CrpPacket, CrpEventType, CRP_VERSION, Platform};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, info, warn};

use crate::{ConnectionRegistry, ConnTx, CubeAdapter};

pub struct RobloxAdapter {
    pub port: u16,
}

impl RobloxAdapter {
    pub fn new(port: u16) -> Self { Self { port } }
}

#[async_trait]
impl CubeAdapter for RobloxAdapter {
    fn name(&self) -> &str { "Roblox" }

    async fn start(
        &self,
        inbound: mpsc::Sender<CrpPacket>,
    ) -> Result<ConnectionRegistry> {
        let addr     = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        let registry = ConnectionRegistry::new();

        info!("Roblox adapter listening on ws://{}", addr);

        let registry_clone = registry.clone();
        tokio::spawn(async move {
            while let Ok((stream, peer)) = listener.accept().await {
                info!("Roblox: new connection from {}", peer);
                let tx       = inbound.clone();
                let reg      = registry_clone.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, tx, reg).await {
                        warn!("Roblox: connection {} error: {}", peer, e);
                    }
                });
            }
        });

        Ok(registry)
    }

    async fn stop(&self) -> Result<()> {
        info!("Roblox adapter stopping");
        Ok(())
    }
}

async fn handle_connection(
    stream:   tokio::net::TcpStream,
    inbound:  mpsc::Sender<CrpPacket>,
    registry: ConnectionRegistry,
) -> Result<()> {
    let ws             = accept_async(stream).await?;
    let (sink, mut ws_stream) = ws.split();
    let sink = std::sync::Arc::new(tokio::sync::Mutex::new(sink));

    // ── Step 1: read the first packet — must be Join ─────────────────────
    let first = ws_stream.next().await
        .ok_or_else(|| anyhow::anyhow!("connection closed before Join"))??;

    let join_pkt = match first {
        Message::Text(ref t) => parse_roblox_json(t),
        _ => None,
    };

    let join_pkt = match join_pkt {
        Some(p) if p.event_type == CrpEventType::Join => p,
        _ => {
            warn!("Roblox: first message was not a Join — closing");
            return Ok(());
        }
    };

    let player_id = extract_player_id(&join_pkt.payload)
        .unwrap_or_else(|| format!("roblox_{}", join_pkt.seq));

    // ── Step 2: register in ConnectionRegistry ────────────────────────────
    let (conn_tx, mut conn_rx) = mpsc::unbounded_channel::<Bytes>();
    registry.register(player_id.clone(), conn_tx);
    info!("Roblox: player '{}' registered", player_id);

    // Forward Join into inbound
    let _ = inbound.send(join_pkt).await;

    // ── Step 3: writer task — router bytes → JSON → WS ───────────────────
    let player_id_w = player_id.clone();
    let registry_w  = registry.clone();
    let sink_w      = sink.clone();
    tokio::spawn(async move {
        while let Some(bytes) = conn_rx.recv().await {
            // bytes is raw CRP — decode and re-encode as JSON for Roblox
            if let Some(pkt) = CrpPacket::decode(&bytes) {
                if let Some(json) = crp_to_roblox_json(&pkt) {
                    let mut s = sink_w.lock().await;
                    if s.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
            }
        }
        registry_w.deregister(&player_id_w);
        debug!("Roblox: writer task ended for '{}'", player_id_w);
    });

    // ── Step 4: reader loop — WS → inbound ───────────────────────────────
    while let Some(msg) = ws_stream.next().await {
        match msg? {
            Message::Text(text) => {
                match parse_roblox_json(&text) {
                    Some(pkt) => { if inbound.send(pkt).await.is_err() { break; } }
                    None => warn!("Roblox: malformed JSON ({}B)", text.len()),
                }
            }
            Message::Binary(bin) => {
                if let Some(pkt) = CrpPacket::decode(&bin) {
                    if inbound.send(pkt).await.is_err() { break; }
                }
            }
            Message::Close(_) => break,
            Message::Ping(d)  => { let _ = sink.lock().await.send(Message::Pong(d)).await; }
            _ => {}
        }
    }

    registry.deregister(&player_id);
    info!("Roblox: player '{}' disconnected", player_id);
    Ok(())
}

// ── JSON codec ────────────────────────────────────────────────────────────────

fn parse_roblox_json(text: &str) -> Option<CrpPacket> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    let session_id = v["session_id"].as_u64()?;
    let seq        = v["seq"].as_u64()? as u32;
    let event_type = match v["type"].as_str()? {
        "move"    => CrpEventType::Move,
        "chat"    => CrpEventType::Chat,
        "voice"   => CrpEventType::Voice,
        "join"    => CrpEventType::Join,
        "leave"   => CrpEventType::Leave,
        "ping"    => CrpEventType::Ping,
        "action"  => CrpEventType::Action,
        _         => return None,
    };
    let payload = serde_json::to_vec(&v["data"]).ok()?;
    Some(CrpPacket { version: CRP_VERSION, platform: Platform::Roblox,
                     session_id, seq, event_type, payload })
}

/// Encode a CrpPacket as Roblox-friendly JSON.
fn crp_to_roblox_json(pkt: &CrpPacket) -> Option<String> {
    let type_str = match pkt.event_type {
        CrpEventType::Move         => "move",
        CrpEventType::Chat         => "chat",
        CrpEventType::Voice        => "voice",
        CrpEventType::Join         => "join",
        CrpEventType::Leave        => "leave",
        CrpEventType::Ping         => "ping",
        CrpEventType::Pong         => "pong",
        CrpEventType::Action       => "action",
        CrpEventType::SessionState => "session_state",
    };
    let data: serde_json::Value = serde_json::from_slice(&pkt.payload)
        .unwrap_or(serde_json::Value::Null);
    let json = serde_json::json!({
        "session_id": pkt.session_id,
        "seq":        pkt.seq,
        "platform":   pkt.platform.to_string(),
        "type":       type_str,
        "data":       data,
    });
    serde_json::to_string(&json).ok()
}

fn extract_player_id(payload: &[u8]) -> Option<String> {
    let v: serde_json::Value = serde_json::from_slice(payload).ok()?;
    Some(v["player_id"].as_str()?.to_string())
}
