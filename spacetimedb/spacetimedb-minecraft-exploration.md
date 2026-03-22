---
name: SpacetimeDB Minecraft
description: Minecraft Beta 1.7.3 server implementation running on SpacetimeDB with real-time multiplayer synchronization
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.SpacetimeDB/spacetimedb-minecraft/
---

# SpacetimeDB Minecraft - Minecraft Server on SpacetimeDB

## Overview

SpacetimeDB Minecraft is a **Minecraft Beta 1.7.3 server implementation** that runs entirely on SpacetimeDB. This project demonstrates how a complex, stateful game server can be built using SpacetimeDB's in-memory database with real-time synchronization. The entire game logic - chunk generation, entity tracking, block updates, player interactions - runs as a SpacetimeDB module.

Key features:
- **Full Minecraft server** - Beta 1.7.3 protocol implementation
- **Real-time synchronization** - All players see updates instantly
- **Chunk generation** - Procedural world generation in SpacetimeDB
- **Entity system** - Mobs, animals, and entity AI
- **Block physics** - Redstone, water, lava, gravity blocks
- **Multiplayer** - Thousands of concurrent players supported
- **Proxy architecture** - Lightweight proxy server for Minecraft protocol

## Directory Structure

```
spacetimedb-minecraft/
├── crates/
│   ├── module/                 # SpacetimeDB module (game logic)
│   │   └── Cargo.toml
│   ├── mc173-module/           # Minecraft protocol implementation
│   │   └── src/
│   │       ├── block/          # Block types and behaviors
│   │       ├── entity/         # Entity system and AI
│   │       ├── chunk/          # Chunk management
│   │       ├── gen/            # World generation
│   │       ├── item/           # Items and crafting
│   │       ├── world/          # World interaction
│   │       └── stdb/           # SpacetimeDB integration
│   └── mc173-server/           # Proxy server for Minecraft clients
│       └── src/
│           ├── autogen/        # Auto-generated code
│           └── main.rs
├── public/                     # Web client (optional)
├── src/                        # TypeScript/React frontend
├── Cargo.toml                  # Workspace configuration
└── README.md
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Minecraft Client                             │
│                    (Vanilla 1.7.3)                              │
└─────────────────────────────────────────────────────────────────┘
                            │
                            │ Minecraft Protocol (TCP)
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                  Minecraft Proxy Server                         │
│                  (mc173-server)                                 │
│  - Protocol handling                                            │
│  - Packet encoding/decoding                                     │
│  - Client state management                                      │
└─────────────────────────────────────────────────────────────────┘
                            │
                            │ SpacetimeDB Client SDK
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                    SpacetimeDB Module                           │
│                    (mc173-module)                               │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Block System        Entity System     World Generation │   │
│  │  - Block types       - Mobs (AI)       - Chunk gen      │   │
│  │  - Physics           - Players         - Biomes         │   │
│  │  - Redstone          - Collision       - Structures     │   │
│  │  - Fluids            - Spawning        - Caves          │   │
│  └─────────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              SpacetimeDB Tables (In-Memory)             │   │
│  │  - Chunks    - Entities    - Players    - Blocks       │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Core Components

### SpacetimeDB Module

```rust
// crates/mc173-module/src/lib.rs
use spacetimedb::{spacetimedb, ReducerContext, Identity, Table, Timestamp};

/// Player position and state
#[spacetimedb(table)]
pub struct Player {
    #[primarykey]
    pub identity: Identity,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub health: f32,
    pub dimension: i8,
    pub online: bool,
}

/// Chunk data (16x16x256 block section)
#[spacetimedb(table)]
pub struct Chunk {
    #[primarykey]
    pub x: i32,
    pub z: i32,
    pub blocks: Vec<u8>,
    pub block_light: Vec<u8>,
    pub sky_light: Vec<u8>,
    pub biome: Vec<u8>,
    pub generated: bool,
    pub populated: bool,
}

/// Entity (mobs, animals, drops, etc.)
#[spacetimedb(table)]
pub struct Entity {
    #[primarykey]
    pub id: i32,
    pub entity_type: EntityType,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub velocity_x: f64,
    pub velocity_y: f64,
    pub velocity_z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub health: f32,
    pub ai_state: AIState,
}

/// Block update event
#[spacetimedb(table)]
pub struct BlockUpdate {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub block_id: u8,
    pub metadata: u8,
    pub timestamp: Timestamp,
}
```

### World Generation

```rust
// crates/mc173-module/src/gen/mod.rs
use crate::chunk::Chunk;
use crate::noise::PerlinNoise;

pub fn generate_chunk(chunk_x: i32, chunk_z: i32, seed: i64) -> Chunk {
    let noise = PerlinNoise::new(seed);
    let mut chunk = Chunk::new(chunk_x, chunk_z);

    // Generate terrain heightmap
    for x in 0..16 {
        for z in 0..16 {
            let world_x = chunk_x * 16 + x;
            let world_z = chunk_z * 16 + z;

            // Generate height using noise
            let height = (noise.sample(world_x as f64, world_z as f64) * 32.0 + 64.0) as i32;

            // Fill blocks from bottom to height
            for y in 0..256 {
                let block = get_block_for_height(y, height, noise, world_x, world_z);
                chunk.set_block(x, y, z, block);
            }
        }
    }

    // Generate structures (dungeons, caves, etc.)
    generate_dungeons(&mut chunk, chunk_x, chunk_z, seed);
    generate_caves(&mut chunk, chunk_x, chunk_z, seed);

    chunk.generated = true;
    chunk
}

fn get_block_for_height(
    y: i32,
    surface_height: i32,
    noise: PerlinNoise,
    x: i32,
    z: i32
) -> Block {
    if y < 10 {
        Block::Bedrock
    } else if y < surface_height - 4 {
        Block::Stone
    } else if y < surface_height {
        Block::Dirt
    } else if y == surface_height {
        // Surface biome-based block
        get_surface_block(noise, x, z)
    } else {
        Block::Air
    }
}
```

### Entity System and AI

```rust
// crates/mc173-module/src/entity/mod.rs
#[spacetimedb(table)]
pub struct EntityAI {
    #[primarykey]
    pub entity_id: i32,
    pub ai_state: AIState,
    pub target_x: Option<f64>,
    pub target_y: Option<f64>,
    pub target_z: Option<f64>,
    pub tick_count: u64,
}

pub enum AIState {
    Idle,
    Wandering,
    Following(Identity),  // Following a player
    Attacking(Identity),  // Attacking a player
    Fleeing(Identity),    // Fleeing from a player
}

#[spacetimedb(reducer)]
pub fn tick_entities(ctx: &ReducerContext) {
    // Called every game tick (50ms)
    for mut entity in ctx.db.entity_ai().iter() {
        match entity.ai_state {
            AIState::Wandering => {
                // Random movement
                tick_wandering(&mut entity);
            }
            AIState::Following(target) => {
                // Follow player
                tick_following(&mut entity, target);
            }
            AIState::Attacking(target) => {
                // Attack player
                tick_attacking(&mut entity, target);
            }
            _ => {}
        }

        // Update entity position
        update_entity_position(entity);
    }
}
```

### Block Physics System

```rust
// crates/mc173-module/src/world/tick.rs
#[spacetimedb(reducer)]
pub fn tick_blocks(ctx: &ReducerContext) {
    // Process block ticks (redstone, fluids, gravity)
    for tick in ctx.db.scheduled_tick().iter() {
        let block = ctx.db.block().at(tick.x, tick.y, tick.z);

        match block.block_type {
            BlockType::Sand | BlockType::Gravel => {
                tick_gravity_block(ctx, tick.x, tick.y, tick.z);
            }
            BlockType::Water | BlockType::Lava => {
                tick_fluid(ctx, tick.x, tick.y, tick.z);
            }
            BlockType::RedstoneWire => {
                tick_redstone(ctx, tick.x, tick.y, tick.z);
            }
            _ => {}
        }
    }
}

fn tick_gravity_block(
    ctx: &ReducerContext,
    x: i32,
    y: i32,
    z: i32
) {
    // Check if block can fall
    let below = ctx.db.block().at(x, y - 1, z);

    if below.block_type == BlockType::Air {
        // Remove from current position
        ctx.db.block().delete(x, y, z);

        // Find landing position
        let mut landing_y = y - 1;
        while ctx.db.block().at(x, landing_y - 1, z).block_type == BlockType::Air {
            landing_y -= 1;
        }

        // Place at landing position
        ctx.db.block().set(x, landing_y, z, below);

        // Schedule update for clients
        schedule_block_update(ctx, x, landing_y, z);
    }
}
```

### Proxy Server

```rust
// crates/mc173-server/src/main.rs
use spacetimedb_client::SpacetimeDBClient;
use minecraft_protocol::{Client, Packet, ServerPacket};

struct MinecraftProxy {
    stdb_client: SpacetimeDBClient,
    minecraft_client: Client,
    player_identity: Option<Identity>,
}

impl MinecraftProxy {
    async fn connect(
        stdb_uri: String,
        module_name: String,
    ) -> Result<Self, ProxyError> {
        let stdb_client = SpacetimeDBClient::connect(&stdb_uri, &module_name).await?;

        Ok(Self {
            stdb_client,
            minecraft_client: Client::new(),
            player_identity: None,
        })
    }

    async fn handle_packet(&mut self, packet: Packet) -> Result<(), ProxyError> {
        match packet {
            Packet::PlayerPosition { x, y, z, .. } => {
                // Update player position in SpacetimeDB
                self.stdb_client.call("update_player_position", (x, y, z)).await?;
            }
            Packet::PlayerLook { yaw, pitch } => {
                // Update player rotation
                self.stdb_client.call("update_player_rotation", (yaw, pitch)).await?;
            }
            Packet::PlayerBlockPlacement { x, y, z, direction, block } => {
                // Place block via SpacetimeDB reducer
                self.stdb_client.call("place_block", (x, y, z, direction, block)).await?;
            }
            Packet::PlayerDigging { status, x, y, z } => {
                // Break block
                if status == DiggingStatus::Start {
                    self.stdb_client.call("start_break_block", (x, y, z)).await?;
                } else if status == DiggingStatus::Stop {
                    self.stdb_client.call("stop_break_block", (x, y, z)).await?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    async fn receive_updates(&mut self) -> Result<(), ProxyError> {
        // Receive database updates from SpacetimeDB
        while let Some(update) = self.stdb_client.next_update().await {
            for row in update.rows {
                self.handle_database_update(row).await?;
            }
        }

        Ok(())
    }

    async fn handle_database_update(
        &mut self,
        row: DatabaseRow
    ) -> Result<(), ProxyError> {
        match row {
            DatabaseRow::Chunk(chunk) => {
                // Send chunk data to Minecraft client
                self.minecraft_client.send(
                    ServerPacket::MapChunk {
                        x: chunk.x,
                        z: chunk.z,
                        data: chunk.blocks,
                    }
                ).await?;
            }
            DatabaseRow::Entity(entity) => {
                // Send entity spawn/movement
                self.minecraft_client.send(
                    ServerPacket::EntityVelocity {
                        entity_id: entity.id,
                        velocity_x: entity.velocity_x,
                        velocity_y: entity.velocity_y,
                        velocity_z: entity.velocity_z,
                    }
                ).await?;
            }
            DatabaseRow::Player(player) => {
                // Send player position update
                self.minecraft_client.send(
                    ServerPacket::EntityTeleport {
                        entity_id: player.entity_id,
                        x: player.x,
                        y: player.y,
                        z: player.z,
                        yaw: player.yaw,
                        pitch: player.pitch,
                    }
                ).await?;
            }
            _ => {}
        }

        Ok(())
    }
}
```

## Deployment

### Publishing the Module

```bash
# Deploy to SpacetimeDB testnet
spacetime publish -s testnet minecraft-server

# Or run locally
spacetime start
spacetime publish minecraft-server
```

### Running the Proxy Server

```bash
# Run proxy server
cargo run --release -p mc173-server -- \
    -m minecraft-server \
    -s "https://testnet.spacetimedb.com"

# Local deployment
cargo run --release -p mc173-server -- \
    -m minecraft-server \
    -s "http://localhost:3000"
```

### Connecting with Minecraft Client

1. Use Minecraft Beta 1.7.3 client
2. Add server: `localhost` (or proxy server IP)
3. Connect and play!

## Performance Considerations

### Chunk Loading Optimization

```rust
// Only load chunks near players
#[spacetimedb(reducer)]
fn update_loaded_chunks(ctx: &ReducerContext) {
    for player in ctx.db.player().iter() {
        let player_chunk_x = (player.x / 16.0) as i32;
        let player_chunk_z = (player.z / 16.0) as i32;

        // Load chunks in render distance
        for dx in -RENDER_DISTANCE..=RENDER_DISTANCE {
            for dz in -RENDER_DISTANCE..=RENDER_DISTANCE {
                let chunk_x = player_chunk_x + dx;
                let chunk_z = player_chunk_z + dz;

                // Ensure chunk exists and is loaded
                ensure_chunk_loaded(ctx, chunk_x, chunk_z);
            }
        }
    }
}
```

### Entity Culling

```rust
// Only send entity updates to players who can see them
fn send_entity_updates(
    ctx: &ReducerContext,
    entity: &Entity,
) {
    let visible_players = ctx.db.player()
        .iter()
        .filter(|p| {
            let distance = distance_squared(
                (entity.x, entity.y, entity.z),
                (p.x, p.y, p.z)
            );
            distance < VIEW_DISTANCE_SQUARED
        });

    for player in visible_players {
        send_entity_update_packet(player.identity, entity);
    }
}
```

## Related Documents

- [SpacetimeDB Cookbook](./spacetimedb-cookbook-exploration.md) - Game patterns
- [Blackholio](./blackholio-exploration.md) - Another multiplayer game example
- [OmniPaxos](./omnipaxos-exploration.md) - Consensus protocol

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.SpacetimeDB/spacetimedb-minecraft/`
- GitHub: https://github.com/clockworklabs/spacetimedb-minecraft
- SpacetimeDB: https://spacetimedb.com/docs
