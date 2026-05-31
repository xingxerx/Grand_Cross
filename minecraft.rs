//! Minecraft Adapter — TCP bridge to Paper plugin, full outbound path.
//!
//! Wire format: length-prefixed CRP
//!   [u16 LE payload_len][CRP bytes]
//!
//! Inbound:  Paper plugin → TCP → CRP → inbound channel → dispatch
//! Outbound: router bytes → ConnectionRegistry → per-conn pump → TCP → Paper

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use crate::{CrpPacket, CrpEventType, CRP_VERSION, Platform};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::{ConnectionRegistry, CubeAdapter};

pub struct MinecraftAdapter {
    pub port: u16,
}

impl MinecraftAdapter {
    pub fn new(port: u16) -> Self { Self { port } }
}

#[async_trait]
impl CubeAdapter for MinecraftAdapter {
    fn name(&self) -> &str { "Minecraft" }

    async fn start(
        &self,
        inbound: mpsc::Sender<CrpPacket>,
    ) -> Result<ConnectionRegistry> {
        let addr     = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        let registry = ConnectionRegistry::new();

        info!("Minecraft adapter listening on tcp://{}", addr);

        let registry_clone = registry.clone();
        tokio::spawn(async move {
            while let Ok((stream, peer)) = listener.accept().await {
                info!("Minecraft: Paper plugin connected from {}", peer);
                let tx  = inbound.clone();
                let reg = registry_clone.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, tx, reg).await {
                        warn!("Minecraft: connection {} error: {}", peer, e);
                    }
                });
            }
        });

        Ok(registry)
    }

    async fn stop(&self) -> Result<()> {
        info!("Minecraft adapter stopping");
        Ok(())
    }
}

async fn handle_connection(
    mut stream:  tokio::net::TcpStream,
    inbound:     mpsc::Sender<CrpPacket>,
    registry:    ConnectionRegistry,
) -> Result<()> {
    // ── Step 1: read first packet — must be Join ──────────────────────────
    let join_pkt = read_crp_packet(&mut stream).await?;

    if join_pkt.event_type != CrpEventType::Join {
        warn!("Minecraft: first packet was not Join — closing");
        return Ok(());
    }

    let player_id = extract_player_id(&join_pkt.payload)
        .unwrap_or_else(|| format!("mc_{}", join_pkt.seq));

    // ── Step 2: split stream, register writer ─────────────────────────────
    let (mut reader, mut writer) = stream.into_split();
    let (conn_tx, mut conn_rx)   = mpsc::unbounded_channel::<Bytes>();
    registry.register(player_id.clone(), conn_tx);
    info!("Minecraft: player '{}' registered", player_id);

    let _ = inbound.send(join_pkt).await;

    // ── Step 3: writer task — router bytes → TCP ──────────────────────────
    let player_id_w = player_id.clone();
    let registry_w  = registry.clone();
    tokio::spawn(async move {
        while let Some(bytes) = conn_rx.recv().await {
            let len = bytes.len() as u16;
            if writer.write_all(&len.to_le_bytes()).await.is_err() { break; }
            if writer.write_all(&bytes).await.is_err()             { break; }
        }
        registry_w.deregister(&player_id_w);
    });

    // ── Step 4: reader loop — TCP → inbound ──────────────────────────────
    loop {
        let pkt = match read_crp_packet(&mut reader).await {
            Ok(p)  => p,
            Err(_) => break,
        };
        if inbound.send(pkt).await.is_err() { break; }
    }

    registry.deregister(&player_id);
    info!("Minecraft: player '{}' disconnected", player_id);
    Ok(())
}

/// Read one length-prefixed CRP packet from any AsyncRead.
async fn read_crp_packet<R: AsyncReadExt + Unpin>(reader: &mut R) -> Result<CrpPacket> {
    let mut len_buf = [0u8; 2];
    reader.read_exact(&mut len_buf).await?;
    let len = u16::from_le_bytes(len_buf) as usize;

    if len == 0 || len > 65_000 {
        anyhow::bail!("invalid packet length: {}", len);
    }

    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).await?;
    CrpPacket::decode(&buf).ok_or_else(|| anyhow::anyhow!("malformed CRP"))
}

fn extract_player_id(payload: &[u8]) -> Option<String> {
    let v: serde_json::Value = serde_json::from_slice(payload).ok()?;
    Some(v["player_id"].as_str()?.to_string())
}
