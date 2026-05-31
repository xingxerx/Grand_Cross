# Grand_Cross

Grand_Cross is an ultra-thin crossplay relay bridging the worlds of Roblox, Minecraft, and Hytale into a unified gameplay experience.

## Overview
This project serves as a zero-state relay that allows true crossplay capabilities. It takes events from individual platforms via platform-specific adapters (WebSocket for Roblox, TCP for Minecraft and Hytale), unifies them using the `CrpPacket` format, and routes them to all other peers in a shared session.

## Architecture
- **Adapters**: Bridges translating platform-specific data into the unified CRP envelope.
- **Router**: Lock-free concurrent router built on `dashmap` handling sessions.
- **Voice Pipeline**: Fast processing block supporting PCM gating, muting, and VAD.

## Setup
Build the project using Cargo:
```bash
cargo build --release
```

Run the relay server:
```bash
cargo run --release
```
