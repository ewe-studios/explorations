---
source: /home/darkvoid/Boxxed/@dev/repo-expolorations/spacetimedb/
explored_at: 2026-03-22
revised_at: 2026-03-22
workspace: spacetimedb-workspace
---

# Rust Revision: SpacetimeDB Sub-Projects

## Overview

This document consolidates the SpacetimeDB sub-project explorations into implementation guidance for building applications with SpacetimeDB. The revision covers authentication patterns, multiplayer game development, educational workshop materials, and core database architecture.

## Sub-Projects Covered

### 1. JWKS Authentication
**Source:** `jwks-exploration.md`
**Implementation:** JSON Web Key Set client for JWT validation

### 2. SpacetimeDB Minecraft
**Source:** `spacetimedb-minecraft-exploration.md`
**Implementation:** Full Minecraft server on SpacetimeDB

### 3. Hophacks Workshop
**Source:** `hophacks-workshop-exploration.md`
**Implementation:** Educational chat application

### 4. Core Crates
**Source:** `spacetimedb-core-crates-exploration.md`
**Implementation:** Database internals reference

### 5. Cookbook Patterns
**Source:** `spacetimedb-cookbook-exploration.md`
**Implementation:** Reusable recipe collection

### 6. Blackholio MMO
**Source:** `blackholio-exploration.md`
**Implementation:** Multiplayer game reference

### 7. OmniPaxos Consensus
**Source:** `omnipaxos-exploration.md`
**Implementation:** Distributed consensus protocol

## Workspace Structure

```
spacetimedb-workspace/
├── auth/
│   ├── jwks-client/          # JWKS fetching and caching
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── client.rs     # JWKS HTTP client
│   │       ├── cache.rs      # Key caching
│   │       ├── key.rs        # JWK representation
│   │       ├── validator.rs  # JWT validation
│   │       └── error.rs      # Error types
│   └── stdb-auth-module/     # SpacetimeDB auth module
│       ├── Cargo.toml
│       └── src/lib.rs
├── games/
│   ├── minecraft-module/     # SpacetimeDB Minecraft module
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── blackholio-module/    # Blackholio game module
│   │   ├── Cargo.toml
│   │   └── src/
│   └── chat-workshop/        # Educational chat app
│       ├── server-rs/
│       └── client-ts/
├── patterns/
│   ├── cookbook-examples/    # Cookbook recipes
│   └── omnipaxos-integration/# Consensus integration
└── core-reference/           # Core crate references
    └── architecture-notes.md
```

## JWKS Authentication Implementation

### JWKS Client Crate

```rust
// auth/jwks-client/src/client.rs
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct Jwks {
    pub keys: Vec<Jwk>,
}

#[derive(Debug, Deserialize)]
pub struct Jwk {
    pub kty: String,
    pub kid: Option<String>,
    pub use_: Option<String>,
    pub alg: Option<String>,
    pub n: Option<String>,
    pub e: Option<String>,
    pub crv: Option<String>,
    pub x: Option<String>,
    pub y: Option<String>,
}

pub struct JwksClient {
    http: Client,
    issuer_url: String,
    cache: JwksCache,
}

impl JwksClient {
    pub fn new(issuer_url: String) -> Self {
        Self {
            http: Client::new(),
            issuer_url,
            cache: JwksCache::new(Duration::from_secs(3600)),
        }
    }

    pub async fn get_keys(&self) -> Result<&Jwks, JwksError> {
        if let Some(keys) = self.cache.get() {
            return Ok(keys);
        }

        let jwks_url = format!("{}/.well-known/jwks.json", self.issuer_url);
        let response = self.http.get(&jwks_url).send().await?;
        let jwks: Jwks = response.json().await?;

        self.cache.set(jwks);
        Ok(self.cache.get().unwrap())
    }

    pub async fn get_key(&self, kid: &str) -> Result<&Jwk, JwksError> {
        let jwks = self.get_keys().await?;
        jwks.keys
            .iter()
            .find(|k| k.kid.as_deref() == Some(kid))
            .ok_or(JwksError::KeyNotFound(kid.to_string()))
    }
}
```

### SpacetimeDB Auth Module

```rust
// auth/stdb-auth-module/src/lib.rs
use spacetimedb::{Identity, ReducerContext, Table, Timestamp};
use jwks_client::{JwksClient, JwtValidator};

#[spacetimedb(table)]
pub struct AuthenticatedUser {
    #[primarykey]
    pub id: Identity,
    pub jwt_sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub authenticated_at: Timestamp,
}

#[spacetimedb(reducer)]
pub async fn authenticate(
    ctx: ReducerContext,
    token: String,
) -> Result<(), AuthError> {
    let validator = get_validator();
    let claims = validator.validate(&token)
        .await
        .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

    if claims.exp < current_timestamp() {
        return Err(AuthError::TokenExpired);
    }

    ctx.authenticated_user.insert(AuthenticatedUser {
        id: ctx.sender,
        jwt_sub: claims.sub,
        email: claims.email,
        name: claims.name,
        authenticated_at: Timestamp::now(),
    });

    Ok(())
}

#[spacetimedb(reducer)]
pub fn logout(ctx: ReducerContext) -> Result<(), AuthError> {
    ctx.authenticated_user.delete(ctx.sender);
    Ok(())
}
```

## SpacetimeDB Minecraft Implementation

### Module Structure

```rust
// games/minecraft-module/src/lib.rs
use spacetimedb::{spacetimedb, Identity, Table, Timestamp, ReducerContext};

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

#[spacetimedb(table)]
pub struct Chunk {
    #[primarykey]
    pub x: i32,
    pub z: i32,
    pub blocks: Vec<u8>,
    pub generated: bool,
    pub populated: bool,
}

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
}

#[spacetimedb(reducer)]
pub fn update_player_position(
    ctx: &ReducerContext,
    x: f64,
    y: f64,
    z: f64,
    yaw: f32,
    pitch: f32,
) {
    if let Some(mut player) = ctx.db.player().identity().find(ctx.sender) {
        player.x = x;
        player.y = y;
        player.z = z;
        player.yaw = yaw;
        player.pitch = pitch;
        ctx.db.player().identity().update(player);
    }
}

#[spacetimedb(reducer)]
pub fn place_block(
    ctx: &ReducerContext,
    x: i32,
    y: i32,
    z: i32,
    block_id: u8,
) {
    // Validate placement
    // Update chunk data
    // Notify nearby players
}

#[spacetimedb(reducer)]
pub fn break_block(
    ctx: &ReducerContext,
    x: i32,
    y: i32,
    z: i32,
) {
    // Validate break
    // Drop items
    // Update chunk
    // Notify players
}
```

### Proxy Server

```rust
// games/minecraft-module/proxy/src/main.rs
use spacetimedb_client::SpacetimeDBClient;
use minecraft_protocol::{Client, Packet, ServerPacket};

struct MinecraftProxy {
    stdb_client: SpacetimeDBClient,
    minecraft_client: Client,
}

impl MinecraftProxy {
    async fn handle_packet(&mut self, packet: Packet) -> Result<(), ProxyError> {
        match packet {
            Packet::PlayerPosition { x, y, z, .. } => {
                self.stdb_client.call("update_player_position", (x, y, z)).await?;
            }
            Packet::PlayerBlockPlacement { x, y, z, block } => {
                self.stdb_client.call("place_block", (x, y, z, block)).await?;
            }
            Packet::PlayerDigging { status, x, y, z } => {
                if status == DiggingStatus::Start {
                    self.stdb_client.call("start_break_block", (x, y, z)).await?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn receive_updates(&mut self) -> Result<(), ProxyError> {
        while let Some(update) = self.stdb_client.next_update().await {
            for row in update.rows {
                self.handle_database_update(row).await?;
            }
        }
        Ok(())
    }
}
```

## Chat Workshop Implementation

### Server Module

```rust
// games/chat-workshop/server-rs/src/lib.rs
use spacetimedb::{Identity, ReducerContext, Table, Timestamp};

#[spacetimedb::table(name = user, public)]
pub struct User {
    #[primary_key]
    pub identity: Identity,
    pub name: Option<String>,
    pub online: bool,
}

#[spacetimedb::table(name = message, public)]
pub struct Message {
    pub sender: Identity,
    pub sent: Timestamp,
    pub text: String,
}

#[spacetimedb::reducer]
pub fn set_name(ctx: &ReducerContext, name: String) -> Result<(), String> {
    if name.is_empty() {
        return Err("Names must not be empty".to_string());
    }

    if let Some(user) = ctx.db.user().identity().find(ctx.sender) {
        ctx.db.user().identity().update(User {
            name: Some(name),
            ..user
        });
        Ok(())
    } else {
        Err("Cannot set name for unknown user".to_string())
    }
}

#[spacetimedb::reducer]
pub fn send_message(ctx: &ReducerContext, text: String) -> Result<(), String> {
    if text.is_empty() {
        return Err("Messages must not be empty".to_string());
    }

    ctx.db.message().insert(Message {
        sender: ctx.sender,
        text,
        sent: ctx.timestamp,
    });
    Ok(())
}

#[spacetimedb::reducer(client_connected)]
pub fn identity_connected(ctx: &ReducerContext) {
    if let Some(user) = ctx.db.user().identity().find(ctx.sender) {
        ctx.db.user().identity().update(User { online: true, ..user });
    } else {
        ctx.db.user().insert(User {
            name: None,
            identity: ctx.sender,
            online: true,
        });
    }
}

#[spacetimedb::reducer(client_disconnected)]
pub fn identity_disconnected(ctx: &ReducerContext) {
    if let Some(user) = ctx.db.user().identity().find(ctx.sender) {
        ctx.db.user().identity().update(User { online: false, ..user });
    }
}
```

### TypeScript Client

```typescript
// games/chat-workshop/client-ts/src/main.ts
import { SpacetimeDB } from 'spacetimedb';
import { User, Message } from './types/generated';

async function main() {
  const db = await SpacetimeDB.connect('http://localhost:3000', 'quickstart-chat');

  db.user.subscribe({
    onInsert: (user) => console.log('User joined:', user.name),
    onUpdate: (_, newUser) => {
      if (newUser.online) {
        console.log('User online:', newUser.name);
      }
    },
  });

  db.message.subscribe({
    onInsert: (msg) => {
      const user = db.user.identity.get(msg.sender);
      console.log(`${user?.name || 'Anonymous'}: ${msg.text}`);
    },
  });

  await db.reducers.set_name('Alice');
  await db.reducers.send_message('Hello, world!');
}

main();
```

## Core Architecture Reference

### Table Storage Pattern

```rust
// Reference implementation based on core crates
pub trait TableStorage: Send + Sync {
    type Row;
    type Pointer;

    fn insert(&mut self, row: Self::Row) -> Self::Pointer;
    fn delete(&mut self, ptr: Self::Pointer) -> Self::Row;
    fn get(&self, ptr: Self::Pointer) -> &Self::Row;
    fn iter(&self) -> impl Iterator<Item = (Self::Pointer, &Self::Row)>;
}

pub trait Index: Send + Sync {
    type Key: Eq + std::hash::Hash;
    type Pointer;

    fn insert(&mut self, key: Self::Key, ptr: Self::Pointer);
    fn remove(&mut self, key: &Self::Key, ptr: Self::Pointer);
    fn lookup(&self, key: &Self::Key) -> Vec<Self::Pointer>;
}
```

### Subscription Pattern

```rust
pub struct SubscriptionManager {
    subscriptions: HashMap<Identity, Vec<Subscription>>,
}

impl SubscriptionManager {
    pub fn add(
        &mut self,
        client_id: Identity,
        queries: Vec<Query>,
        sender: tokio::sync::mpsc::Sender<DatabaseUpdate>,
    ) -> Result<SubscriptionId, SubscriptionError> {
        let id = SubscriptionId::generate();
        self.subscriptions
            .entry(client_id)
            .or_default()
            .push(Subscription { id, queries, sender });
        Ok(id)
    }

    pub fn evaluate_after_tx(&self, tx: &Transaction) {
        for subscriptions in self.subscriptions.values() {
            for sub in subscriptions {
                for query in &sub.queries {
                    let delta = evaluate_query_delta(query, tx);
                    if !delta.is_empty() {
                        let _ = sub.sender.try_send(DatabaseUpdate {
                            subscription_id: sub.id,
                            delta,
                        });
                    }
                }
            }
        }
    }
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use spacetimedb::testing::TestContext;
    use crate::*;

    #[test]
    fn test_set_name() {
        let ctx = TestContext::new();

        // Simulate client connection
        ctx.connect_client();

        // Call reducer
        let result = set_name(&ctx, "Alice".to_string());

        assert!(result.is_ok());

        // Verify state
        let user = ctx.db.user().identity().find(ctx.sender).unwrap();
        assert_eq!(user.name, Some("Alice".to_string()));
    }

    #[test]
    fn test_send_message() {
        let ctx = TestContext::new();

        // Set name first
        set_name(&ctx, "Alice".to_string()).unwrap();

        // Send message
        send_message(&ctx, "Hello!".to_string()).unwrap();

        // Verify message inserted
        let messages: Vec<_> = ctx.db.message().iter().collect();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text, "Hello!");
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration {
    use spacetimedb::testing::{TestRunner, ClientHandle};

    #[test]
    fn test_multiplayer_chat() {
        let mut runner = TestRunner::new();

        // Start two clients
        let mut alice = runner.add_client();
        let mut bob = runner.add_client();

        // Both set names
        alice.call("set_name", ("Alice",));
        bob.call("set_name", ("Bob",));

        // Alice sends message
        alice.call("send_message", ("Hello Bob!",));

        // Bob should receive message
        let updates = bob.wait_for_updates();
        assert!(updates.messages.iter().any(|m| m.text == "Hello Bob!"));
    }
}
```

## Deployment Guide

### Local Development

```bash
# Start SpacetimeDB
spacetime start

# Publish module
cd games/chat-workshop/server-rs
spacetime publish quickstart-chat

# Run client
cd ../client-ts
npm install
npm run dev
```

### Cloud Deployment

```bash
# Deploy to SpacetimeDB cloud
spacetime publish --server https://spacetimedb.com my-chat-app

# Check status
spacetime logs my-chat-app
```

## Deep Dive Documents

### Storage Internals
- [Storage Internals Deep Dive](./storage-internals-deep-dive.md) - BSATN/BFLATN formats, page manager, commit log, transaction model, indexes, blob store

### Consensus and Replication
- [Consensus and Replication Deep Dive](./consensus-and-replication-deep-dive.md) - Leader-based replication, reducer execution, subscriptions, durability guarantees, efficiency comparisons

## Related Documents

- [Blackholio](./blackholio-exploration.md) - Full MMO reference
- [JWKS](./jwks-exploration.md) - Authentication details
- [OmniPaxos](./omnipaxos-exploration.md) - Consensus protocol

## Sources

- SpacetimeDB Documentation: https://spacetimedb.com/docs
- GitHub: https://github.com/clockworklabs/SpacetimeDB
