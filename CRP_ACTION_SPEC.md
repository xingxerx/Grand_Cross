# CRP Action Payload Specification
# CGX_Cube — v0.1
#
# Every CrpEventType::Action packet carries a JSON payload.
# This file defines the schema for each action subtype.
#
# All adapters MUST implement encode/decode for every action defined here.
# Unknown action types MUST be silently dropped, never crash.
#
# Payload is always UTF-8 JSON. Max size: 1024 bytes.
# Field names are snake_case. All fields required unless marked optional?.

# ─────────────────────────────────────────────────────────────────────────────
# TOP-LEVEL ENVELOPE (wraps every action payload)
# ─────────────────────────────────────────────────────────────────────────────

{
  "action": "<action_type>",   # string — identifies the action schema below
  "player_id": "<string>",     # originating player's cross-platform ID
  "ts": <i64>,                 # unix ms timestamp at origin
  "data": { ... }              # action-specific payload (schema below)
}

# ─────────────────────────────────────────────────────────────────────────────
# MOVEMENT
# ─────────────────────────────────────────────────────────────────────────────

action: "move"
{
  "x": <f64>,           # world X coordinate
  "y": <f64>,           # world Y (vertical)
  "z": <f64>,           # world Z coordinate
  "yaw": <f32>,         # horizontal rotation degrees 0–360
  "pitch": <f32>,       # vertical rotation degrees -90–90
  "speed": <f32>,       # units/sec (used for animation blending)
  "grounded": <bool>    # is player on ground
}

# Platform translations:
#   Roblox    → RemoteEvent("CubeMove", {x,y,z,yaw,pitch})
#   Minecraft → teleport or EntityMovePacket to proxy entity
#   Hytale    → EntityTransform update

# ─────────────────────────────────────────────────────────────────────────────
# TERRAIN — MINE
# ─────────────────────────────────────────────────────────────────────────────

action: "mine"
{
  "x": <i32>,           # block coordinate X
  "y": <i32>,           # block coordinate Y
  "z": <i32>,           # block coordinate Z
  "block_id": <string>, # platform-agnostic block type (see Block Registry below)
  "tool": <string>      # "hand" | "pickaxe" | "shovel" | "axe"
}

# Platform translations:
#   Roblox    → visual crack animation at (x,y,z), then part removal VFX
#   Minecraft → block break event (Paper: BlockBreakEvent mirror)
#   Hytale    → terrain deform at (x,y,z)

# ─────────────────────────────────────────────────────────────────────────────
# TERRAIN — BUILD / PLACE
# ─────────────────────────────────────────────────────────────────────────────

action: "build"
{
  "x": <i32>,
  "y": <i32>,
  "z": <i32>,
  "block_id": <string>, # platform-agnostic block type
  "face": <string>      # "top"|"bottom"|"north"|"south"|"east"|"west"
}

# Platform translations:
#   Roblox    → Part placed at (x,y,z) with CGX material map
#   Minecraft → block place event mirror
#   Hytale    → block placed in terrain

# ─────────────────────────────────────────────────────────────────────────────
# SHARD — PICKUP
# ─────────────────────────────────────────────────────────────────────────────

action: "shard_pick"
{
  "shard_id": <string>,     # UUID of the specific Rift Shard
  "carrier_id": <string>,   # player_id of the carrier
  "x": <f64>,               # pickup location
  "y": <f64>,
  "z": <f64>
}

# Platform translations:
#   Roblox    → BillboardGui glow on carrier model + particle VFX at location
#   Minecraft → beacon beam + glowing effect on carrier entity + sound
#   Hytale    → creature agitation radius around carrier + ambient VFX

# ─────────────────────────────────────────────────────────────────────────────
# SHARD — DROP
# ─────────────────────────────────────────────────────────────────────────────

action: "shard_drop"
{
  "shard_id": <string>,
  "carrier_id": <string>,   # player who dropped it
  "x": <f64>,               # drop location
  "y": <f64>,
  "z": <f64>,
  "reason": <string>        # "voluntary" | "killed" | "timeout"
}

# Platform translations:
#   Roblox    → remove carrier VFX, spawn shard prop at (x,y,z)
#   Minecraft → remove beacon + glowing, place beacon block at location
#   Hytale    → calm mobs, spawn shard entity

# ─────────────────────────────────────────────────────────────────────────────
# SHARD — DELIVERED (win condition trigger)
# ─────────────────────────────────────────────────────────────────────────────

action: "shard_deliver"
{
  "shard_id": <string>,
  "carrier_id": <string>,
  "team_id": <string>,      # winning team
  "convergence_x": <f64>,   # Convergence Point coordinates
  "convergence_y": <f64>,
  "convergence_z": <f64>
}

# Platform translations:
#   All platforms → win screen + fireworks/particle event + score update

# ─────────────────────────────────────────────────────────────────────────────
# PLAYER — DAMAGE
# ─────────────────────────────────────────────────────────────────────────────

action: "damage"
{
  "target_id": <string>,    # player receiving damage
  "source_id": <string>,    # player or entity dealing damage (optional?)
  "amount": <f32>,          # normalized 0.0–1.0 (adapters map to platform HP)
  "type": <string>,         # "melee" | "ranged" | "fall" | "environment"
  "knockback_x": <f32>,     # optional? knockback vector
  "knockback_z": <f32>
}

# Platform translations:
#   Roblox    → Humanoid.Health delta + knockback BodyVelocity
#   Minecraft → damage packet to proxy entity
#   Hytale    → damage event on entity

# ─────────────────────────────────────────────────────────────────────────────
# PLAYER — DEATH
# ─────────────────────────────────────────────────────────────────────────────

action: "death"
{
  "player_id": <string>,
  "killer_id": <string>,    # optional? — null if environmental
  "x": <f64>,               # death location (for respawn logic)
  "y": <f64>,
  "z": <f64>
}

# ─────────────────────────────────────────────────────────────────────────────
# PLAYER — RESPAWN
# ─────────────────────────────────────────────────────────────────────────────

action: "respawn"
{
  "player_id": <string>,
  "x": <f64>,
  "y": <f64>,
  "z": <f64>
}

# ─────────────────────────────────────────────────────────────────────────────
# PLATFORM POWER — ACTIVATE
# Each platform's unique power emits this when used
# ─────────────────────────────────────────────────────────────────────────────

action: "power_use"
{
  "player_id": <string>,
  "power": <string>,        # "builder_snap" | "miner_scan" | "shaper_pulse"
  "x": <f64>,               # origin
  "y": <f64>,
  "z": <f64>,
  "radius": <f32>,          # effect radius
  "data": { ... }           # power-specific payload (see below)
}

# Power-specific data schemas:

# builder_snap (Roblox) — instant structure placement
{
  "structure_id": <string>, # predefined structure template ID
  "rotation": <f32>         # yaw rotation of placed structure
}

# miner_scan (Minecraft) — reveals Shard clue in radius
{
  "revealed_hint": <string> # "north" | "south" | "east" | "west" | "down" | "up"
}

# shaper_pulse (Hytale) — reveals hidden paths, calms/agitates mobs
{
  "effect": <string>,       # "reveal_path" | "calm_mobs" | "agitate_mobs"
  "duration_ms": <i32>
}

# ─────────────────────────────────────────────────────────────────────────────
# CLUE — SHARD INTEL BROADCAST
# Emitted when a player discovers a directional clue
# ─────────────────────────────────────────────────────────────────────────────

action: "clue_broadcast"
{
  "player_id": <string>,
  "platform": <string>,     # "roblox" | "minecraft" | "hytale"
  "clue_type": <string>,    # "structural_anomaly" | "vein_pattern" | "mob_behavior"
  "direction": <string>,    # cardinal + vertical: "N"|"NE"|"E"|"SE"|"S"|"SW"|"W"|"NW"|"up"|"down"
  "confidence": <f32>,      # 0.0–1.0 (distance-based — closer = higher confidence)
  "origin_x": <f64>,        # where the clue was observed
  "origin_z": <f64>
}

# Platform translations:
#   All platforms → minimap ping + chat notification
#   "Player [X] spotted a [clue_type] to the [direction]"

# ─────────────────────────────────────────────────────────────────────────────
# BLOCK REGISTRY — Platform-Agnostic Block IDs
# Maps CGX block_id → platform native block
# ─────────────────────────────────────────────────────────────────────────────

# CGX ID          Roblox Material     Minecraft Block      Hytale Block
# ─────────────────────────────────────────────────────────────────────
# "stone"         SmoothPlastic gray  minecraft:stone      stone
# "dirt"          SmoothPlastic brown minecraft:dirt       dirt
# "wood"          Wood                minecraft:oak_log    oak_log
# "glass"         Glass               minecraft:glass      glass
# "ore_iron"      Neon orange         minecraft:iron_ore   iron_ore
# "ore_rift"      Neon blue (CGX)     minecraft:amethyst   rift_crystal  ← Shard ore
# "rift_shard"    Special part        minecraft:beacon     rift_shard_entity

# ─────────────────────────────────────────────────────────────────────────────
# ERROR HANDLING RULES (all adapters must follow)
# ─────────────────────────────────────────────────────────────────────────────

# 1. Unknown "action" field → drop silently, log at DEBUG level
# 2. Missing required field → drop silently, log at WARN level
# 3. Payload > 1024 bytes   → drop, log at WARN, increment metric
# 4. Invalid coordinate     → clamp to world bounds, log at DEBUG
# 5. Unknown player_id      → drop, log at WARN (player not in session)
# 6. Unknown block_id       → substitute "stone", log at DEBUG
