---
name: SpacetimeDB Cookbook
description: Recipe collection and examples for building applications with SpacetimeDB
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.SpacetimeDB/spacetimedb-cookbook/
---

# SpacetimeDB Cookbook - Recipes and Examples

## Overview

The SpacetimeDB Cookbook is a **collection of recipes and example implementations** demonstrating common patterns and use cases for building applications with SpacetimeDB. It provides practical, copy-pasteable code samples for developers looking to leverage SpacetimeDB's unique architecture.

Key features:
- **Ready-to-use recipes** - Copy-paste examples
- **Common patterns** - Auth, chat, multiplayer, IoT
- **Best practices** - Performance, security, scaling
- **Full examples** - Complete working applications
- **Language variants** - Rust, TypeScript, C# examples

## Directory Structure

```
spacetimedb-cookbook/
├── recipes/
│   ├── authentication/         # User auth patterns
│   ├── chat-application/       # Real-time chat
│   ├── multiplayer-games/      # Game development
│   ├── iot-telemetry/          # IoT data streaming
│   ├── collaborative-editing/  # CRDT-based collaboration
│   ├── real-time-analytics/    # Live dashboards
│   └── data-sync/              # Offline-first sync
├── examples/
│   ├── chat/                   # Complete chat app
│   ├── multiplayer/            # Multiplayer game
│   ├── dashboard/              # Real-time dashboard
│   └── iot-sensor/             # IoT example
├── Cargo.toml
└── README.md
```

## Authentication Patterns

### Simple Token Auth

```rust
use spacetimedb::{ReducerContext, Table, Identity};

#[spacetimedb(table)]
pub struct User {
    #[primarykey]
    pub id: Identity,
    pub username: String,
    pub email: String,
    pub created_at: Timestamp,
}

#[spacetimedb(table)]
pub struct Session {
    #[primarykey]
    pub token: u64,
    pub user_id: Identity,
    pub expires_at: Timestamp,
}

#[spacetimedb(reducer)]
pub fn register(
    ctx: ReducerContext,
    username: String,
    email: String,
) -> Result<(), AuthError> {
    // Check if username exists
    if ctx.user().filter(|u| u.username == username).is_some() {
        return Err(AuthError::UsernameTaken);
    }

    // Create user
    ctx.user.insert(User {
        id: ctx.sender,
        username,
        email,
        created_at: Timestamp::now(),
    });

    Ok(())
}

#[spacetimedb(reducer)]
pub fn create_session(ctx: ReducerContext) -> Result<u64, AuthError> {
    let token = random_token();

    ctx.session.insert(Session {
        token,
        user_id: ctx.sender,
        expires_at: Timestamp::now() + Duration::days(30),
    });

    Ok(token)
}

fn random_token() -> u64 {
    use rand::Rng;
    rand::thread_rng().gen()
}
```

### JWT-style Auth

```rust
use jsonwebtoken::{encode, decode, Header, Algorithm, Validation};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: Identity,
    exp: usize,
    iat: usize,
}

#[spacetimedb(reducer)]
pub fn login_with_token(
    ctx: ReducerContext,
    token: String,
) -> Result<UserInfo, AuthError> {
    // Decode and validate JWT
    let claims = decode::<Claims>(
        &token,
        &get_secret_key(),
        &Validation::new(Algorithm::HS256)
    )?;

    // Get user from database
    let user = ctx.user()
        .find(|u| u.id == claims.claims.sub)
        .ok_or(AuthError::UserNotFound)?;

    Ok(UserInfo {
        id: user.id,
        username: user.username,
    })
}

#[spacetimedb(reducer)]
pub fn generate_jwt(ctx: ReducerContext) -> Result<String, AuthError> {
    let user = ctx.user()
        .find(|u| u.id == ctx.sender)
        .ok_or(AuthError::UserNotFound)?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as usize;

    let claims = Claims {
        sub: user.id,
        exp: now + (86400 * 30), // 30 days
        iat: now,
    };

    encode(&Header::default(), &claims, &get_secret_key())
        .map_err(|_| AuthError::TokenGenerationFailed)
}
```

## Chat Application

### Core Tables

```rust
use spacetimedb::{ReducerContext, Identity, Timestamp};

#[spacetimedb(table)]
pub struct Channel {
    #[primarykey]
    pub id: u64,
    pub name: String,
    pub created_by: Identity,
    pub created_at: Timestamp,
}

#[spacetimedb(table)]
pub struct Message {
    #[primarykey]
    pub id: u64,
    pub channel_id: u64,
    pub author: Identity,
    pub content: String,
    pub created_at: Timestamp,
    pub edited_at: Option<Timestamp>,
}

#[spacetimedb(table)]
pub struct ChannelMember {
    pub channel_id: u64,
    pub user_id: Identity,
    pub joined_at: Timestamp,
    #[primarykey(channel_id, user_id)]
}

#[spacetimedb(table)]
pub struct TypingIndicator {
    pub channel_id: u64,
    pub user_id: Identity,
    pub timestamp: Timestamp,
    #[primarykey(channel_id, user_id)]
}
```

### Chat Reducers

```rust
#[spacetimedb(reducer)]
pub fn create_channel(
    ctx: ReducerContext,
    name: String,
) -> Result<u64, ChatError> {
    let id = generate_id();

    ctx.channel.insert(Channel {
        id,
        name,
        created_by: ctx.sender,
        created_at: Timestamp::now(),
    });

    // Creator joins automatically
    ctx.channel_member.insert(ChannelMember {
        channel_id: id,
        user_id: ctx.sender,
        joined_at: Timestamp::now(),
    });

    Ok(id)
}

#[spacetimedb(reducer)]
pub fn send_message(
    ctx: ReducerContext,
    channel_id: u64,
    content: String,
) -> Result<(), ChatError> {
    // Verify membership
    if ctx.channel_member()
        .filter(|m| m.channel_id == channel_id && m.user_id == ctx.sender)
        .is_none()
    {
        return Err(ChatError::NotMember);
    }

    let id = generate_id();

    ctx.message.insert(Message {
        id,
        channel_id,
        author: ctx.sender,
        content,
        created_at: Timestamp::now(),
        edited_at: None,
    });

    Ok(())
}

#[spacetimedb(reducer)]
pub fn set_typing(
    ctx: ReducerContext,
    channel_id: u64,
    is_typing: bool,
) -> Result<(), ChatError> {
    if is_typing {
        ctx.typing_indicator.insert(TypingIndicator {
            channel_id,
            user_id: ctx.sender,
            timestamp: Timestamp::now(),
        });
    } else {
        ctx.typing_indicator.delete(channel_id, ctx.sender);
    }

    Ok(())
}

#[spacetimedb(reducer)]
pub fn join_channel(
    ctx: ReducerContext,
    channel_id: u64,
) -> Result<(), ChatError> {
    ctx.channel_member.insert(ChannelMember {
        channel_id,
        user_id: ctx.sender,
        joined_at: Timestamp::now(),
    });

    Ok(())
}
```

## Multiplayer Game Patterns

### Player State

```rust
#[spacetimedb(table)]
pub struct Player {
    #[primarykey]
    pub id: Identity,
    pub x: f32,
    pub y: f32,
    pub health: u32,
    pub score: u32,
    pub last_action: Timestamp,
}

#[spacetimedb(table)]
pub struct GameState {
    #[primarykey]
    pub id: u8,  // Single row for game state
    pub status: GameStatus,
    pub started_at: Timestamp,
    pub ended_at: Option<Timestamp>,
}

pub enum GameStatus {
    Waiting,
    Playing,
    Ended,
}
```

### Game Logic

```rust
#[spacetimedb(reducer)]
pub fn join_game(ctx: ReducerContext) -> Result<(), GameError> {
    let game = ctx.game_state().find(|g| g.id == 0).unwrap();

    if game.status != GameStatus::Waiting {
        return Err(GameError::GameInProgress);
    }

    ctx.player.insert(Player {
        id: ctx.sender,
        x: 0.0,
        y: 0.0,
        health: 100,
        score: 0,
        last_action: Timestamp::now(),
    });

    Ok(())
}

#[spacetimedb(reducer)]
pub fn move_player(
    ctx: ReducerContext,
    dx: f32,
    dy: f32,
) -> Result<(), GameError> {
    let mut player = ctx.player()
        .find(|p| p.id == ctx.sender)
        .ok_or(GameError::NotJoined)?;

    player.x = (player.x + dx).clamp(0.0, 100.0);
    player.y = (player.y + dy).clamp(0.0, 100.0);
    player.last_action = Timestamp::now();

    ctx.player.update(player);

    Ok(())
}

#[spacetimedb(reducer)]
pub fn start_game(ctx: ReducerContext) -> Result<(), GameError> {
    // Only host can start
    if !is_host(ctx.sender) {
        return Err(GameError::NotHost);
    }

    let player_count = ctx.player().count();
    if player_count < 2 {
        return Err(GameError::NotEnoughPlayers);
    }

    let mut game = ctx.game_state().find(|g| g.id == 0).unwrap();
    game.status = GameStatus::Playing;
    game.started_at = Timestamp::now();
    ctx.game_state.update(game);

    Ok(())
}
```

## IoT Telemetry Pattern

### Sensor Data

```rust
#[spacetimedb(table)]
pub struct SensorReading {
    #[primarykey]
    pub id: u128,
    pub device_id: String,
    pub sensor_type: String,
    pub value: f64,
    pub unit: String,
    pub timestamp: Timestamp,
}

#[spacetimedb(table)]
pub struct Device {
    #[primarykey]
    pub id: String,
    pub name: String,
    pub location: String,
    pub last_seen: Timestamp,
    pub status: DeviceStatus,
}

pub enum DeviceStatus {
    Online,
    Offline,
    Error,
}
```

### Telemetry Ingestion

```rust
#[spacetimedb(reducer)]
pub fn submit_reading(
    ctx: ReducerContext,
    device_id: String,
    sensor_type: String,
    value: f64,
    unit: String,
) -> Result<(), TelemetryError> {
    // Verify device exists
    let mut device = ctx.device()
        .find(|d| d.id == device_id)
        .ok_or(TelemetryError::UnknownDevice)?;

    // Update device status
    device.last_seen = Timestamp::now();
    device.status = DeviceStatus::Online;
    ctx.device.update(device);

    // Insert reading
    let id = generate_ulid();
    ctx.sensor_reading.insert(SensorReading {
        id,
        device_id,
        sensor_type,
        value,
        unit,
        timestamp: Timestamp::now(),
    });

    Ok(())
}

#[spacetimedb(reducer)]
pub fn register_device(
    ctx: ReducerContext,
    device_id: String,
    name: String,
    location: String,
) -> Result<(), TelemetryError> {
    ctx.device.insert(Device {
        id: device_id,
        name,
        location,
        last_seen: Timestamp::now(),
        status: DeviceStatus::Online,
    });

    Ok(())
}
```

### Aggregation Queries

```rust
// Client-side subscription for aggregated data
fn subscribe_to_averages(device_id: &str) {
    db.sensor_reading.subscribe(
        Query::new()
            .filter(|r| r.device_id == device_id)
            .filter(|r| r.timestamp > Timestamp::now() - Duration::hours(1))
    );
}

// Compute averages in client
fn compute_averages(readings: &[SensorReading]) -> HashMap<String, f64> {
    let mut averages = HashMap::new();
    let mut sums: HashMap<String, f64> = HashMap::new();
    let mut counts: HashMap<String, usize> = HashMap::new();

    for reading in readings {
        *sums.entry(reading.sensor_type.clone()).or_insert(0.0) += reading.value;
        *counts.entry(reading.sensor_type.clone()).or_insert(0) += 1;
    }

    for (sensor_type, sum) in sums {
        averages.insert(sensor_type, sum / counts[&sensor_type] as f64);
    }

    averages
}
```

## Related Documents

- [SpacetimeDB Core](./core-architecture-deep-dive.md) - Core database
- [Blackholio](./blackholio-exploration.md) - MMO example
- [Omnipaxos](./omnipaxos-exploration.md) - Consensus

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.SpacetimeDB/spacetimedb-cookbook/`
- SpacetimeDB Documentation: https://spacetimedb.com/docs
