//! CGX_Cube Relay — entry point
//!
//! Boot sequence:
//!   1. Parse config
//!   2. Start SessionManager + Router
//!   3. Start platform adapters → get ConnectionRegistry per adapter
//!   4. Spawn dispatch loop per adapter (wired to registry for outbound pump)
//!   5. Spawn stats reporter
//!   6. Block on Ctrl-C → graceful shutdown


use anyhow::Result;
use clap::Parser;
use tokio::sync::mpsc;
use tracing::info;
use tracing_subscriber::EnvFilter;

use grand_cross::{
    roblox::RobloxAdapter,
    minecraft::MinecraftAdapter,
    hytale::HytaleAdapter,
    adapters::CubeAdapter,
};
use grand_cross::Platform;

use grand_cross::dispatch::spawn_dispatch_loop;
use grand_cross::router::Router;
use grand_cross::session_manager::SessionManager;

const INBOUND_CHANNEL_DEPTH: usize = 1024;

#[derive(Parser, Debug)]
#[command(name = "cube-relay", about = "CGX_Cube crossplay relay — FRACTURE RAID")]
struct Args {
    #[arg(long, default_value = "9001", env = "CUBE_ROBLOX_PORT")]
    roblox_port: u16,

    #[arg(long, default_value = "9002", env = "CUBE_MC_PORT")]
    mc_port: u16,

    #[arg(long, default_value = "9003", env = "CUBE_HYTALE_PORT")]
    hytale_port: u16,

    #[arg(long, default_value = "9004", env = "CUBE_VOICE_PORT")]
    voice_port: u16,

    #[arg(long, default_value = "30", env = "CUBE_STATS_INTERVAL")]
    stats_interval: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("grand_cross=info".parse()?),
        )
        .init();

    let args     = Args::parse();
    let router   = Router::new();
    let sessions = SessionManager::new();

    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("  CGX_Cube Relay  —  FRACTURE RAID");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("  Roblox  WS  :{}", args.roblox_port);
    info!("  Minecraft   :{}", args.mc_port);
    info!("  Hytale      :{}", args.hytale_port);
    info!("  Voice  UDP  :{}", args.voice_port);
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // ── Roblox ───────────────────────────────────────────────────────────────
    let (roblox_tx, roblox_rx) = mpsc::channel(INBOUND_CHANNEL_DEPTH);
    let roblox_reg = RobloxAdapter::new(args.roblox_port)
        .start(roblox_tx).await?;
    spawn_dispatch_loop(roblox_rx, Platform::Roblox,
                        router.clone(), sessions.clone(), roblox_reg);

    // ── Minecraft ────────────────────────────────────────────────────────────
    let (mc_tx, mc_rx) = mpsc::channel(INBOUND_CHANNEL_DEPTH);
    let mc_reg = MinecraftAdapter::new(args.mc_port)
        .start(mc_tx).await?;
    spawn_dispatch_loop(mc_rx, Platform::Minecraft,
                        router.clone(), sessions.clone(), mc_reg);

    // ── Hytale ───────────────────────────────────────────────────────────────
    let (hytale_tx, hytale_rx) = mpsc::channel(INBOUND_CHANNEL_DEPTH);
    let hytale_reg = HytaleAdapter::new(args.hytale_port)
        .start(hytale_tx).await?;
    spawn_dispatch_loop(hytale_rx, Platform::Hytale,
                        router.clone(), sessions.clone(), hytale_reg);

    // ── Stats ────────────────────────────────────────────────────────────────
    if args.stats_interval > 0 {
        let r = router.clone();
        let s = sessions.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(
                std::time::Duration::from_secs(args.stats_interval)
            );
            loop {
                tick.tick().await;
                info!("Stats — sessions: {}  players: {}  peers: {}",
                    s.active_sessions(), s.active_players(), r.peer_count());
            }
        });
    }

    tokio::signal::ctrl_c().await?;
    info!("CGX_Cube Relay shutting down");
    Ok(())
}
