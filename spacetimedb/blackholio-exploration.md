---
name: Blackholio MMORPG
description: SpacetimeDB's reference MMORPG implementation demonstrating real-time multiplayer game architecture
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.SpacetimeDB/Blackholio/
---

# Blackholio - SpacetimeDB MMORPG Reference Implementation

## Overview

Blackholio is a **massively multiplayer online (MMO) agar.io-style game** built with SpacetimeDB. It serves as the primary reference implementation demonstrating:

- Real-time multiplayer game architecture
- Entity Component System (ECS) pattern in SpacetimeDB
- Client-side prediction and server reconciliation
- Efficient network replication for hundreds of concurrent players
- Physics simulation with collision detection
- Leaderboard systems

The project is structured as a **full-stack application** with:
- **Server-side**: SpacetimeDB module (Rust) defining game logic and state
- **Client-side**: Unity (C#) and web (TypeScript/JavaScript) clients
- **Shared**: Protocol definitions and types

## Directory Structure

```
Blackholio/
├── .cargo/                          # Rust cargo configuration
├── docs/                            # Documentation
│   └── ...
├── examples/                        # Example implementations
│   └── ...
├── .github/                         # GitHub Actions CI/CD
├── omnipaxos/                       # Consensus protocol (shared)
├── omnipaxos_macros/                # Procedural macros for Omnipaxos
├── omnipaxos_storage/               # Storage layer for Omnipaxos
├── omnipaxos_ui/                    # UI components for Omnipaxos
├── Cargo.toml                       # Root workspace manifest
├── check.sh                         # CI check script
├── crates-checklist.md              # Crate documentation checklist
├── README.md                        # Project overview
└── overview.png                     # Architecture diagram
```

## Core Architecture

### Game State Model

Blackholio uses a **deterministic entity-based state model**:

```rust
// Core game entities stored in SpacetimeDB tables
#[spacetimedb(table)]
pub struct Player {
    pub id: PlayerId,
    pub x: f32,
    pub y: f32,
    pub mass: f32,
    pub color: Color,
    pub name: String,
    pub velocity_x: f32,
    pub velocity_y: f32,
}

#[spacetimedb(table)]
pub struct Food {
    pub id: FoodId,
    pub x: f32,
    pub y: f32,
    pub mass: f32,
    pub color: Color,
}

#[spacetimedb(table)]
pub struct Blob {
    pub id: BlobId,
    pub player_id: PlayerId,
    pub x: f32,
    pub y: f32,
    pub mass: f32,
}
```

### Game Loop Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Client (Unity/Web)                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ Input       │  │ Render      │  │ Client-side Prediction  │  │
│  │ Handling    │  │ (Canvas/    │  │ - Interpolation         │  │
│  │             │  │  WebGL)     │  │ - Dead reckoning        │  │
│  └──────┬──────┘  └──────┬──────┘  └───────────┬─────────────┘  │
│         │                │                      │                │
│         └────────────────┼──────────────────────┘                │
│                          │                                       │
│                  WebSocket Connection                            │
│                          │                                       │
└──────────────────────────┼───────────────────────────────────────┘
                           │
┌──────────────────────────┼───────────────────────────────────────┐
│                  SpacetimeDB Server                             │
│                          │                                       │
│  ┌───────────────────────▼────────────────────────────────┐     │
│  │              Reducer Queue (Transaction Log)            │     │
│  │  - move_player(x, y)                                    │     │
│  │  - spawn_food()                                         │     │
│  │  - eject_mass()                                         │     │
│  │  - split_blob()                                         │     │
│  └───────────────────────┬────────────────────────────────┘     │
│                          │                                       │
│  ┌───────────────────────▼────────────────────────────────┐     │
│  │              Game State Tables                         │     │
│  │  - Players[]                                           │     │
│  │  - Blobs[]                                             │     │
│  │  - Food[]                                              │     │
│  │  - Leaderboard[]                                       │     │
│  └────────────────────────────────────────────────────────┘     │
│                                                                  │
│  ┌────────────────────────────────────────────────────────┐     │
│  │              Subscription Sets                         │     │
│  │  - Per-client filtered views                           │     │
│  │  - Automatic delta compression                         │     │
│  └────────────────────────────────────────────────────────┘     │
└──────────────────────────────────────────────────────────────────┘
```

### Key SpacetimeDB Features Demonstrated

#### 1. Reducers (Server Actions)

Reducers are transactional functions that modify database state:

```rust
#[spacetimedb(reducer)]
pub fn move_player(ctx: ReducerContext, x: f32, y: f32) {
    let player = ctx.player(ctx.sender)?;

    // Update player position
    ctx.player.update(Player {
        id: player.id,
        x,
        y,
        ..player
    });

    // Check food collisions
    check_food_collisions(ctx, player.id, x, y);

    // Check player collisions
    check_player_collisions(ctx, player.id);
}

#[spacetimedb(reducer)]
pub fn spawn_food(ctx: ReducerContext, x: f32, y: f32, mass: f32) {
    ctx.food.insert(Food {
        id: generate_id(),
        x,
        y,
        mass,
        color: random_color(),
    });
}
```

#### 2. Subscription Queries

Clients subscribe to filtered views of the database:

```typescript
// Client-side subscription
db.subscription({
    players: db.player.subscribe({
        onSelect: (player) => updatePlayerUI(player),
        onDelete: (player) => removePlayerUI(player),
    }),
    food: db.food.subscribe({
        onSelect: (food) => renderFood(food),
        onDelete: (food) => removeFood(food),
    }),
    blobs: db.blob.subscribe({
        onSelect: (blob) => updateBlob(blob),
    }),
});
```

#### 3. Client-Side Prediction

```typescript
class GameClient {
    predictMove(deltaX: number, deltaY: number) {
        // Optimistically update local state
        this.localPlayer.x += deltaX;
        this.localPlayer.y += deltaY;

        // Send to server
        this.db.rpc.move_player(deltaX, deltaY);

        // Store for reconciliation
        this.pendingMoves.push({
            deltaX,
            deltaY,
            timestamp: Date.now(),
        });
    }

    reconcile(serverState: PlayerState) {
        // Compare predicted vs actual
        const diff = this.compareStates(
            this.localPlayer,
            serverState
        );

        if (diff > THRESHOLD) {
            // Smooth correction
            this.smoothReposition(serverState);
        }

        // Clear acknowledged moves
        this.clearAcknowledgedMoves();
    }
}
```

## Client Implementations

### Unity Client (C#)

Located in `client-unity/`:

```csharp
// PlayerController.cs
public class PlayerController : MonoBehaviour {
    private SpacetimeDBClient client;
    private Player localPlayer;

    void Update() {
        // Get input
        float x = Input.GetAxis("Horizontal");
        float y = Input.GetAxis("Vertical");

        // Send to server
        client.Reducers.MovePlayer(x, y);

        // Local prediction
        predictedPosition += new Vector3(x, y) * speed * Time.deltaTime;
    }

    void OnStateUpdate(Player player) {
        if (player.Id == localPlayer.Id) {
            // Reconcile with server state
            Reconcile(player);
        } else {
            // Interpolate other players
            InterpolatePlayer(player);
        }
    }
}
```

### Web Client (TypeScript)

```typescript
// GameRenderer.ts
class GameRenderer {
    private canvas: HTMLCanvasElement;
    private ctx: CanvasRenderingContext2D;

    render(state: GameState) {
        // Clear canvas
        this.ctx.clearRect(0, 0, width, height);

        // Render food
        for (const food of state.food) {
            this.drawCircle(food.x, food.y, food.mass, food.color);
        }

        // Render players
        for (const player of state.players) {
            this.drawPlayer(player);
        }

        // Render blobs
        for (const blob of state.blobs) {
            this.drawBlob(blob);
        }
    }
}
```

## Game Mechanics Implementation

### Movement Physics

```rust
fn update_player_physics(ctx: &mut ReducerContext, player_id: PlayerId) {
    let player = ctx.player(player_id).unwrap();
    let blobs: Vec<_> = ctx.blob().filter(|b| b.player_id == player_id).collect();

    // Calculate center of mass
    let (center_x, center_y) = calculate_center_of_mass(&blobs);

    // Apply movement to all blobs
    for mut blob in blobs {
        let dx = blob.x - center_x;
        let dy = blob.y - center_y;

        // Spring force to center
        let force_x = dx * SPRING_CONSTANT;
        let force_y = dy * SPRING_CONSTANT;

        blob.x += force_x / blob.mass;
        blob.y += force_y / blob.mass;

        ctx.blob.update(blob);
    }
}
```

### Collision Detection

```rust
fn check_food_collisions(ctx: &mut ReducerContext, player_id: PlayerId, x: f32, y: f32) {
    let player = ctx.player(player_id).unwrap();
    let eat_radius = calculate_eat_radius(player.mass);

    // Query food in radius using spatial index
    let food_in_range: Vec<_> = ctx
        .food()
        .filter(|f| {
            let dx = f.x - x;
            let dy = f.y - y;
            let dist = (dx * dx + dy * dy).sqrt();
            dist < eat_radius
        })
        .collect();

    for food in food_in_range {
        // Remove food
        ctx.food.delete(food.id);

        // Increase player mass
        let new_mass = player.mass + food.mass;
        ctx.player.update(Player {
            mass: new_mass,
            ..player
        });
    }
}
```

### Splitting Mechanic

```rust
#[spacetimedb(reducer)]
pub fn split_blob(ctx: ReducerContext, direction_x: f32, direction_y: f32) {
    let player = ctx.player(ctx.sender).unwrap();

    // Find largest blob
    let mut blobs: Vec<_> = ctx.blob()
        .filter(|b| b.player_id == player.id)
        .collect();
    blobs.sort_by(|a, b| b.mass.partial_cmp(&a.mass).unwrap());

    if blobs.is_empty() || blobs[0].mass < MIN_SPLIT_MASS {
        return;
    }

    let parent = &mut blobs[0];
    let new_mass = parent.mass / 2;
    parent.mass = new_mass;

    // Create new blob
    let new_blob = Blob {
        id: generate_id(),
        player_id: player.id,
        x: parent.x + direction_x * SPLIT_DISTANCE,
        y: parent.y + direction_y * SPLIT_DISTANCE,
        mass: new_mass,
        velocity_x: direction_x * SPLIT_VELOCITY,
        velocity_y: direction_y * SPLIT_VELOCITY,
    };

    ctx.blob.insert(new_blob);
    ctx.blob.update(*parent);
}
```

## Networking Architecture

### Message Types

```typescript
// Messages from client to server
type ClientMessage =
    | { type: 'move', x: number, y: number }
    | { type: 'split', direction: { x: number, y: number } }
    | { type: 'eject', angle: number };

// Messages from server to client
type ServerMessage =
    | { type: 'snapshot', state: GameState }
    | { type: 'delta', changes: StateDelta }
    | { type: 'reconcile', ackedMoves: number[] };
```

### Delta Compression

SpacetimeDB automatically handles delta compression:

```rust
// SpacetimeDB tracks row-level changes
struct StateDelta {
    players_inserted: Vec<Player>,
    players_deleted: Vec<PlayerId>,
    players_updated: Vec<Player>,
    food_inserted: Vec<Food>,
    food_deleted: Vec<FoodId>,
    // ...
}
```

### Latency Compensation

```typescript
class LatencyCompensator {
    private pendingActions: Map<number, Action>;
    private lastAckedSequence = 0;

    sendAction(action: Action) {
        const seq = this.nextSequence();
        this.pendingActions.set(seq, {
            ...action,
            timestamp: Date.now(),
            sequence: seq,
        });

        this.sendToServer({ seq, action });
    }

    onAck(ackedSeq: number) {
        // Remove all acknowledged actions
        for (const seq of this.pendingActions.keys()) {
            if (seq <= ackedSeq) {
                this.pendingActions.delete(seq);
            }
        }
        this.lastAckedSequence = ackedSeq;
    }

    onMismatch(serverState: GameState) {
        // Rollback unacknowledged actions
        const baseState = this.applyActions(
            serverState,
            Array.from(this.pendingActions.values())
        );
        this.render(baseState);
    }
}
```

## Performance Optimizations

### Spatial Partitioning

```rust
// Quadtree for efficient collision queries
struct SpatialHash {
    cell_size: f32,
    cells: HashMap<(i32, i32), Vec<EntityId>>,
}

impl SpatialHash {
    fn query_radius(&self, x: f32, y: f32, radius: f32) -> Vec<EntityId> {
        let min_cell = self.world_to_cell(x - radius, y - radius);
        let max_cell = self.world_to_cell(x + radius, y + radius);

        let mut entities = Vec::new();
        for cx in min_cell.0..=max_cell.0 {
            for cy in min_cell.1..=max_cell.1 {
                if let Some(cell) = self.cells.get(&(cx, cy)) {
                    entities.extend(cell);
                }
            }
        }
        entities
    }
}
```

### Interest Management

```rust
// Subscription queries filter by proximity
fn get_interest_query(player_x: f32, player_y: f32, view_distance: f32) -> Query {
    Query::new()
        .players(|p| {
            p.x.gt(player_x - view_distance)
                .lt(player_x + view_distance)
                .and(|x| {
                    x.y.gt(player_y - view_distance)
                        .lt(player_y + view_distance)
                })
        })
        .food(|f| {
            // Same filter for food
            f.x.gt(player_x - view_distance)
                .lt(player_x + view_distance)
                .and(|x| {
                    x.y.gt(player_y - view_distance)
                        .lt(player_y + view_distance)
                })
        })
}
```

### Batch Updates

```rust
// Batch food spawning for performance
#[spacetimedb(reducer)]
pub fn spawn_food_batch(ctx: ReducerContext, count: u32) {
    let mut food_batch = Vec::with_capacity(count as usize);

    for _ in 0..count {
        food_batch.push(Food {
            id: generate_id(),
            x: random_x(),
            y: random_y(),
            mass: BASE_FOOD_MASS,
            color: random_color(),
        });
    }

    // Single transaction for all inserts
    ctx.food.insert_batch(food_batch);
}
```

## Lessons from Blackholio

### 1. Deterministic Simulation

The game logic must be deterministic to enable:
- Client-side prediction
- Replay systems
- Debugging and testing

```rust
// Use fixed-point math for deterministic physics
fn deterministic_move(x: f32, y: f32, vx: f32, vy: f32, dt: u32) -> (f32, f32) {
    // Fixed timestep
    let steps = dt / FIXED_TIMESTEP;
    let mut px = x;
    let mut py = y;

    for _ in 0..steps {
        px += vx * FIXED_TIMESTEP as f32;
        py += vy * FIXED_TIMESTEP as f32;
    }

    (px, py)
}
```

### 2. State vs Events

Blackholio demonstrates when to use:
- **State**: Current positions, scores, mass (queries via subscriptions)
- **Events**: One-time actions like split, eject (sent via reducers)

### 3. Scaling Strategies

```
┌────────────────────────────────────────────────────────┐
│              Horizontal Scaling                        │
│                                                        │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐               │
│  │ Module  │  │ Module  │  │ Module  │               │
│  │  Inst 1 │  │  Inst 2 │  │  Inst N │               │
│  └────┬────┘  └────┬────┘  └────┬────┘               │
│       │           │           │                       │
│       └───────────┼───────────┘                       │
│                   │                                   │
│          ┌────────▼────────┐                         │
│          │  Load Balancer  │                         │
│          │  (Consistent    │                         │
│          │   Hashing)      │                         │
│          └────────┬────────┘                         │
│                   │                                   │
│          ┌────────┴────────┐                         │
│          │  Redis/Spacetime│                         │
│          │  Global State   │                         │
│          └─────────────────┘                         │
└────────────────────────────────────────────────────────┘
```

## Comparison with Traditional Game Servers

| Aspect | Traditional (UDP) | SpacetimeDB |
|--------|-------------------|-------------|
| State sync | Manual serialization | Automatic via subscriptions |
| Authority | Custom logic | Database constraints |
| Scaling | Sharding complexity | Module instances |
| Persistence | Separate DB | Built-in |
| Client prediction | Framework-specific | Built-in reconciliation |

## Related Projects

- **[OmniPaxos](./omnipaxos-exploration.md)**: Consensus protocol used for distributed coordination
- **[SpacetimeDB Core](./spacetimedb-core-exploration.md)**: Main database engine
- **[SpacetimeDB Cookbook](./spacetimedb-cookbook-exploration.md)**: Additional examples

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.SpacetimeDB/Blackholio/`
- SpacetimeDB Documentation: https://spacetimedb.com/docs
- Blackholio Demo: https://github.com/clockworklabs/SpacetimeDB/tree/master/blackholio
