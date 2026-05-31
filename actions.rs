//! CRP Action Payload types.
//!
//! Every CrpEventType::Action packet's payload deserializes into a
//! CubeAction. Adapters match on the variant and emit the platform-
//! specific equivalent. Unknown variants are silently dropped.
//!
//! Serialization: JSON, max 1024 bytes, UTF-8.

use serde::{Deserialize, Serialize};

/// Top-level action envelope — deserialize the CRP payload into this first.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionEnvelope {
    pub player_id: String,
    pub ts:        i64,
    #[serde(flatten)]
    pub action:    CubeAction,
}

/// All supported cross-platform game actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum CubeAction {
    // ── Movement ──────────────────────────────────────────────────────────
    Move(MoveData),

    // ── Terrain ───────────────────────────────────────────────────────────
    Mine(BlockCoord),
    Build(BuildData),

    // ── Shard (FRACTURE RAID win condition) ───────────────────────────────
    ShardPick(ShardPickData),
    ShardDrop(ShardDropData),
    ShardDeliver(ShardDeliverData),

    // ── Combat ────────────────────────────────────────────────────────────
    Damage(DamageData),
    Death(DeathData),
    Respawn(RespawnData),

    // ── Platform Powers ───────────────────────────────────────────────────
    PowerUse(PowerUseData),

    // ── Shard Intel ───────────────────────────────────────────────────────
    ClueBroadcast(ClueData),
}

// ── Movement ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveData {
    pub x:        f64,
    pub y:        f64,
    pub z:        f64,
    pub yaw:      f32,
    pub pitch:    f32,
    pub speed:    f32,
    pub grounded: bool,
}

// ── Terrain ───────────────────────────────────────────────────────────────────

/// Shared block coordinate + metadata used by Mine and Build.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockCoord {
    pub x:        i32,
    pub y:        i32,
    pub z:        i32,
    pub block_id: BlockId,
    pub tool:     ToolType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildData {
    pub x:        i32,
    pub y:        i32,
    pub z:        i32,
    pub block_id: BlockId,
    pub face:     BlockFace,
}

/// Platform-agnostic block registry.
/// Adapters map these to their native block type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockId {
    Stone,
    Dirt,
    Wood,
    Glass,
    OreIron,
    OreRift,    // The Shard ore — glows blue (CGX #00BFFF)
    RiftShard,  // Placed Shard entity / beacon
    #[serde(other)]
    Unknown,    // Unmapped block → substitute Stone, log DEBUG
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolType {
    Hand,
    Pickaxe,
    Shovel,
    Axe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockFace {
    Top, Bottom, North, South, East, West,
}

// ── Shard ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardPickData {
    pub shard_id:   String,
    pub carrier_id: String,
    pub x: f64, pub y: f64, pub z: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardDropData {
    pub shard_id:   String,
    pub carrier_id: String,
    pub x: f64, pub y: f64, pub z: f64,
    pub reason:     DropReason,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DropReason {
    Voluntary,
    Killed,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardDeliverData {
    pub shard_id:       String,
    pub carrier_id:     String,
    pub team_id:        String,
    pub convergence_x:  f64,
    pub convergence_y:  f64,
    pub convergence_z:  f64,
}

// ── Combat ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageData {
    pub target_id:    String,
    pub source_id:    Option<String>,
    /// Normalized 0.0–1.0. Adapters map to platform HP scale.
    pub amount:       f32,
    pub damage_type:  DamageType,
    pub knockback_x:  Option<f32>,
    pub knockback_z:  Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DamageType {
    Melee,
    Ranged,
    Fall,
    Environment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeathData {
    pub player_id: String,
    pub killer_id: Option<String>,
    pub x: f64, pub y: f64, pub z: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RespawnData {
    pub player_id: String,
    pub x: f64, pub y: f64, pub z: f64,
}

// ── Platform Powers ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerUseData {
    pub player_id: String,
    pub x: f64, pub y: f64, pub z: f64,
    pub radius:    f32,
    pub power:     PlatformPower,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "power", rename_all = "snake_case")]
pub enum PlatformPower {
    /// Roblox — instant structure snap placement
    BuilderSnap {
        structure_id: String,
        rotation:     f32,
    },
    /// Minecraft — reveals directional Shard clue within radius
    MinerScan {
        revealed_hint: CardinalDir,
    },
    /// Hytale — reveals hidden paths or manipulates mob state
    ShaperPulse {
        effect:      ShaperEffect,
        duration_ms: i32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShaperEffect {
    RevealPath,
    CalmMobs,
    AgitateMobs,
}

// ── Shard Intel / Clues ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClueData {
    pub platform:   CluePlatform,
    pub clue_type:  ClueType,
    pub direction:  CardinalDir,
    /// 0.0–1.0. Distance-based — closer to Shard = higher confidence.
    pub confidence: f32,
    pub origin_x:   f64,
    pub origin_z:   f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CluePlatform {
    Roblox,
    Minecraft,
    Hytale,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClueType {
    StructuralAnomaly, // Roblox — glowing structure above ground
    VeinPattern,       // Minecraft — ore vein direction
    MobBehavior,       // Hytale — creature agitation
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardinalDir {
    N, Ne, E, Se, S, Sw, W, Nw, Up, Down,
}

// ── Serialization helpers ────────────────────────────────────────────────────

impl ActionEnvelope {
    /// Deserialize from CRP payload bytes. Returns None on any error.
    pub fn from_payload(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > 1024 {
            tracing::warn!("ActionEnvelope: payload {} bytes exceeds 1024 limit", bytes.len());
            return None;
        }
        serde_json::from_slice(bytes).ok()
    }

    /// Serialize to CRP payload bytes.
    pub fn to_payload(&self) -> Option<Vec<u8>> {
        serde_json::to_vec(self).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shard_pick_roundtrip() {
        let env = ActionEnvelope {
            player_id: "xingxerx".into(),
            ts: 1_700_000_000_000,
            action: CubeAction::ShardPick(ShardPickData {
                shard_id:   "shard-001".into(),
                carrier_id: "xingxerx".into(),
                x: 120.5, y: 64.0, z: -33.2,
            }),
        };
        let bytes = env.to_payload().unwrap();
        let decoded = ActionEnvelope::from_payload(&bytes).unwrap();
        assert!(matches!(decoded.action, CubeAction::ShardPick(_)));
    }

    #[test]
    fn rejects_oversized_payload() {
        let big = vec![0u8; 1025];
        assert!(ActionEnvelope::from_payload(&big).is_none());
    }

    #[test]
    fn clue_broadcast_roundtrip() {
        let env = ActionEnvelope {
            player_id: "miner_player".into(),
            ts: 0,
            action: CubeAction::ClueBroadcast(ClueData {
                platform:   CluePlatform::Minecraft,
                clue_type:  ClueType::VeinPattern,
                direction:  CardinalDir::Ne,
                confidence: 0.82,
                origin_x:   200.0,
                origin_z:   -150.0,
            }),
        };
        let bytes = env.to_payload().unwrap();
        let decoded = ActionEnvelope::from_payload(&bytes).unwrap();
        assert!(matches!(decoded.action, CubeAction::ClueBroadcast(_)));
    }
}
