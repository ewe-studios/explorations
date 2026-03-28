---
title: "Stateroom Deep Dive"
subtitle: "Server integration with Stateroom and StateProgram"
prerequisites: [03-sync-protocol-deep-dive.md](03-sync-protocol-deep-dive.md)
next: [05-yew-frontend-deep-dive.md](05-yew-frontend-deep-dive.md)
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.driftingspace/aper/aper-stateroom/
---

# Stateroom Deep Dive

This document explores how Aper integrates with Stateroom for WebSocket server infrastructure, including the StateProgram trait and timed transitions.

## Table of Contents

1. [What is Stateroom?](#1-what-is-stateroom)
2. [StateProgram Trait](#2-stateprogram-trait)
3. [TransitionEvent: Timestamped Transitions](#3-transitionevent-timestamped-transitions)
4. [StateMachineContainerProgram](#4-statemachinecontainerprogram)
5. [AperStateroomService](#5-aperstateroomservice)
6. [Timer-Based Transitions](#6-timer-based-transitions)
7. [StateProgramClient](#7-stateprogramclient)
8. [Complete Example: Drop Four Game](#8-complete-example-drop-four-game)

---

## 1. What is Stateroom?

**Stateroom** is a Rust library for building real-time multiplayer game servers. It provides:

- WebSocket server infrastructure
- Room/connection management
- Client identification
- Binary/JSON message handling
- Timer support

### Stateroom Integration

Aper builds on Stateroom to provide:

```
┌─────────────────────────────────────────┐
│   AperStateroomService<P: StateProgram> │
│                                         │
│  - Manages state machine state          │
│  - Handles client connections           │
│  - Processes transitions                │
│  - Broadcasts updates                   │
│  - Supports timer-based transitions     │
└─────────────────────────────────────────┘
```

### Stateroom Traits

```rust
// Core Stateroom service trait
pub trait SimpleStateroomService {
    fn new(name: &str, ctx: &impl StateroomContext) -> Self;
    fn connect(&mut self, client_id: ClientId, ctx: &impl StateroomContext);
    fn disconnect(&mut self, user: ClientId, ctx: &impl StateroomContext);
    fn message(&mut self, client_id: ClientId, message: &str, ctx: &impl StateroomContext);
    fn binary(&mut self, client_id: ClientId, message: &[u8], ctx: &impl StateroomContext);
    fn timer(&mut self, ctx: &impl StateroomContext);
}
```

Aper implements this trait for `AperStateroomService`.

---

## 2. StateProgram Trait

`StateProgram` extends `StateMachine` with additional capabilities for server operation.

### Trait Definition

```rust
pub trait StateProgram: StateMachine<Transition = TransitionEvent<Self::T>>
where
    <Self as StateProgram>::T: Unpin + Send + Sync,
{
    type T: Debug + Serialize + DeserializeOwned + Clone + PartialEq;

    /// Return a suspended event that should fire at a specific time
    fn suspended_event(&self) -> Option<TransitionEvent<Self::T>> {
        None
    }

    fn new() -> Self;
}
```

### Key Differences from StateMachine

| StateMachine | StateProgram |
|--------------|--------------|
| Generic transitions | TransitionEvent-wrapped transitions |
| No timing support | Supports timer-based transitions |
| Client or server | Server-side only |
| No Send/Sync requirements | Must be Send + Sync + Unpin |

### Why TransitionEvent?

`TransitionEvent` wraps transitions with metadata:

```rust
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TransitionEvent<T>
where
    T: Unpin + Send + Sync + 'static + Clone,
{
    pub timestamp: Timestamp,      // When this happened
    pub client: Option<ClientId>,  // Who did this (None for timer events)
    pub transition: T,             // The actual transition
}
```

This enables:
- Audit trails (who did what when)
- Time-based operations (suspended events)
- Server-side validation

---

## 3. TransitionEvent: Timestamped Transitions

### Structure

```rust
pub type Timestamp = DateTime<Utc>;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TransitionEvent<T> {
    #[serde(with = "ts_milliseconds")]
    pub timestamp: Timestamp,
    pub client: Option<ClientId>,
    pub transition: T,
}
```

### Constructor

```rust
impl<T> TransitionEvent<T>
where
    T: Unpin + Send + Sync + 'static + Clone,
{
    pub fn new(
        player: Option<ClientId>,
        timestamp: Timestamp,
        transition: T,
    ) -> TransitionEvent<T> {
        TransitionEvent {
            timestamp,
            client: player,
            transition,
        }
    }
}
```

### Usage Patterns

**Client-Initiated Transition**:
```rust
let event = TransitionEvent::new(
    Some(client_id),  // User initiated
    Utc::now(),
    GameTransition::Drop(3),
);
```

**Timer-Initiated Transition**:
```rust
let event = TransitionEvent::new(
    None,  // System/timer initiated
    scheduled_time,
    GameTransition::Tick,
);
```

### Timestamp Serialization

```rust
// Uses millisecond timestamps for compact JSON
#[serde(with = "ts_milliseconds")]
pub timestamp: Timestamp,

// Example JSON:
// {"timestamp": 1711555200000, "client": 1, "transition": {"Drop": 3}}
```

---

## 4. StateMachineContainerProgram

`StateMachineContainerProgram` wraps any `StateMachine` to make it a `StateProgram`.

### Definition

```rust
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(bound = "")]
pub struct StateMachineContainerProgram<SM: StateMachine>(pub SM)
where
    <SM as StateMachine>::Transition: Send;
```

### Implementation

```rust
impl<SM: StateMachine> StateMachine for StateMachineContainerProgram<SM>
where
    <SM as StateMachine>::Transition: Send + Unpin + Sync,
{
    type Transition = TransitionEvent<SM::Transition>;
    type Conflict = SM::Conflict;

    fn apply(&self, transition: &Self::Transition) -> Result<Self, Self::Conflict> {
        Ok(StateMachineContainerProgram(
            self.0.apply(&transition.transition)?,
        ))
    }
}

impl<SM: StateMachine + Default> StateProgram for StateMachineContainerProgram<SM>
where
    <SM as StateMachine>::Transition: Send + Unpin + Sync,
{
    type T = SM::Transition;

    fn new() -> Self {
        Self::default()
    }
}
```

### Why Use Container Program?

**Use when**:
- You have an existing `StateMachine`
- You don't need timer-based transitions
- You want minimal boilerplate

**Don't use when**:
- You need custom `suspended_event()` logic
- You want to add metadata to transitions
- You need custom `StateProgram` behavior

### Example: Counter

```rust
// Your existing StateMachine
#[derive(StateMachine)]
pub struct Counter { value: i64 }

// Wrap it for Stateroom
type CounterProgram = StateMachineContainerProgram<Counter>;

// Use in server
type CounterService = AperStateroomService<CounterProgram>;
```

---

## 5. AperStateroomService

`AperStateroomService` is the main server component that handles WebSocket connections.

### Structure

```rust
pub struct AperStateroomService<P: StateProgram> {
    state: StateServer<P>,                          // State machine state
    suspended_event: Option<TransitionEvent<P::T>>, // Pending timer event
}
```

### Connection Handling

```rust
fn connect(&mut self, client_id: ClientId, ctx: &impl StateroomContext) {
    // Send initial state to connecting client
    let response = StateProgramMessage::InitialState {
        timestamp: Utc::now(),
        client_id,
        state: self.state.state().clone(),
        version: self.state.version,
    };

    ctx.send_message(
        MessageRecipient::Client(client_id),
        serde_json::to_string(&response).unwrap().as_str(),
    );
}
```

### Message Handling

```rust
fn message(&mut self, client_id: ClientId, message: &str, ctx: &impl StateroomContext) {
    // Deserialize incoming message
    let message: MessageToServer<P> = serde_json::from_str(message).unwrap();
    self.process_message(message, Some(client_id), ctx);
}

fn binary(&mut self, client_id: ClientId, message: &[u8], ctx: &impl StateroomContext) {
    // Binary (bincode) deserialization
    let message: MessageToServer<P> = bincode::deserialize(message).unwrap();
    self.process_message(message, Some(client_id), ctx);
}
```

### Process Message

```rust
fn process_message(
    &mut self,
    message: MessageToServer<P>,
    client_id: Option<ClientId>,
    ctx: &impl StateroomContext,
) {
    // Validate client ID for transitions
    if let MessageToServer::DoTransition { transition, .. } = &message {
        if transition.client != client_id {
            log::warn!("Invalid player ID in transition");
            return;
        }
    }

    let timestamp = Utc::now();
    let StateServerMessageResponse {
        reply_message,
        broadcast_message,
    } = self.state.receive_message(message);

    // Send reply to sender
    let reply_message = StateProgramMessage::Message {
        message: reply_message,
        timestamp,
    };

    if let Some(client_id) = client_id {
        ctx.send_message(
            MessageRecipient::Client(client_id),
            serde_json::to_string(&reply_message).unwrap().as_str(),
        );
    }

    // Broadcast to others
    if let Some(broadcast_message) = broadcast_message {
        let broadcast_message = StateProgramMessage::Message {
            message: broadcast_message,
            timestamp,
        };

        let recipient = if let Some(client_id) = client_id {
            MessageRecipient::EveryoneExcept(client_id)
        } else {
            MessageRecipient::Broadcast
        };

        ctx.send_message(
            recipient,
            serde_json::to_string(&broadcast_message).unwrap().as_str(),
        );
    }

    // Update timer if needed
    self.update_suspended_event(ctx);
}
```

### Timer Management

```rust
fn update_suspended_event(&mut self, ctx: &impl StateroomContext) {
    let susp = self.state.state().suspended_event();
    if susp == self.suspended_event {
        return; // No change
    }

    // Set timer for new suspended event
    if let Some(ev) = &susp {
        if let Ok(dur) = ev.timestamp.signed_duration_since(Utc::now()).to_std() {
            ctx.set_timer(dur.as_millis() as u32);
        }
    }

    self.suspended_event = susp;
}
```

---

## 6. Timer-Based Transitions

StateProgram supports transitions that fire at specific times.

### Suspended Events

```rust
/// Return a suspended event that should fire at a specific time in the future.
///
/// This is useful for:
/// - Game ticks
/// - Timed power-ups
/// - Auto-reset after inactivity
/// - Scheduled state changes
fn suspended_event(&self) -> Option<TransitionEvent<Self::T>> {
    // Return Some(event) to schedule, None to cancel
}
```

### How It Works

```
1. State machine returns Some(TransitionEvent) from suspended_event()
2. Server sets a timer for the event's timestamp
3. When timer fires, server calls timer()
4. Server processes the suspended transition
5. State updates and broadcasts to all clients
```

### Example: Game Timer

```rust
#[derive(StateMachine)]
pub struct Game {
    time_remaining: Counter,
    // ...
}

impl StateProgram for Game {
    type T = GameTransition;

    fn suspended_event(&self) -> Option<TransitionEvent<GameTransition>> {
        // Return next tick event
        let next_tick = Utc::now() + chrono::Duration::seconds(1);
        Some(TransitionEvent::new(
            None, // System event, not user-initiated
            next_tick,
            GameTransition::Tick,
        ))
    }
}

// Handle the tick
impl Game {
    fn apply(&self, event: &TransitionEvent<GameTransition>) -> Result<Self, NeverConflict> {
        match event.transition {
            GameTransition::Tick => {
                // Decrement timer
                let new_time = self.time_remaining.value() - 1;
                Ok(Game {
                    time_remaining: Counter::new(new_time),
                    // ...
                })
            }
            // ...
        }
    }
}
```

### Timer Callback

```rust
fn timer(&mut self, ctx: &impl StateroomContext) {
    // Process the suspended event
    if let Some(event) = self.suspended_event.take() {
        self.process_message(
            MessageToServer::DoTransition {
                transition_number: ClientTransitionNumber::default(),
                transition: event,
            },
            None, // No client - system event
            ctx,
        );
    }
}
```

---

## 7. StateProgramClient

`StateProgramClient` wraps `StateClient` for Stateroom integration.

### Structure

```rust
pub struct StateProgramClient<S: StateProgram> {
    inner_state: Option<InnerState<S>>,
}

struct InnerState<S: StateProgram> {
    client: StateClient<S>,
    pub client_id: ClientId,
    pub server_time_delta: Duration,  // For clock sync
}
```

### Server Time Sync

```rust
fn receive_message_from_server(
    &mut self,
    message: StateProgramMessage<S>,
) -> Option<MessageToServer<S>> {
    match (message, &mut self.inner_state) {
        (
            StateProgramMessage::InitialState {
                timestamp,
                client_id,
                state,
                version,
            },
            None,
        ) => {
            let client = StateClient::new(state, version);
            // Calculate clock offset
            let server_time_delta = Utc::now().signed_duration_since(timestamp);
            self.inner_state.replace(InnerState {
                client,
                client_id,
                server_time_delta,
            });
            None
        }
        // ...
    }
}
```

### Current Server Time

```rust
impl<S: StateProgram> InnerState<S> {
    fn current_server_time(&self) -> DateTime<Utc> {
        Utc::now()
            .checked_sub_signed(self.server_time_delta)
            .unwrap()
    }
}
```

### Wrapping Transitions

```rust
fn wrap_transition(&self, transition: S::T) -> TransitionEvent<S::T> {
    let timestamp = self.current_server_time();

    TransitionEvent {
        client: Some(self.client_id),
        timestamp,
        transition,
    }
}
```

---

## 8. Complete Example: Drop Four Game

The drop-four example demonstrates a complete StateProgram implementation.

### Game State

```rust
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct DropFourGame(PlayState);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PlayState {
    Waiting {
        waiting_player: Option<ClientId>,
    },
    Playing {
        next_player: PlayerColor,
        board: Board,
        player_map: PlayerMap,
        winner: Option<PlayerColor>,
    },
}
```

### Transitions

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum GameTransition {
    Join,
    Drop(usize),
    Reset,
}
```

### StateMachine Implementation

```rust
impl StateMachine for DropFourGame {
    type Transition = TransitionEvent<GameTransition>;
    type Conflict = NeverConflict;

    fn apply(&self, event: &Self::Transition) -> Result<Self, NeverConflict> {
        let mut new_self = self.clone();
        match event.transition {
            GameTransition::Join => {
                if let PlayState::Waiting { waiting_player: Some(wp) } = new_self.0 {
                    // Second player joined - start game
                    let player_map = PlayerMap {
                        teal_player: wp,
                        brown_player: event.client.unwrap(),
                    };
                    new_self.0 = PlayState::Playing {
                        next_player: PlayerColor::Teal,
                        board: Default::default(),
                        player_map,
                        winner: None,
                    };
                } else if let PlayState::Waiting { .. } = self.0 {
                    // First player - wait for second
                    new_self.0 = PlayState::Waiting {
                        waiting_player: event.client,
                    };
                }
            }
            GameTransition::Drop(col) => {
                if let PlayState::Playing {
                    board,
                    next_player,
                    player_map,
                    winner,
                } = &mut new_self.0
                {
                    if winner.is_some() {
                        return Ok(new_self); // Game over
                    }
                    if player_map.id_of_color(*next_player) != event.client.unwrap() {
                        return Ok(new_self); // Not your turn
                    }

                    // Drop piece
                    if let Some(row) = board.lowest_open_row(col) {
                        board.0[row][col] = Some(*next_player);
                        *winner = board.check_winner_at(row as i32, col as i32);
                        *next_player = next_player.other();
                    }
                }
            }
            GameTransition::Reset => {
                if let PlayState::Playing { winner: Some(w), player_map, .. } = new_self.0 {
                    // Start new game, winner goes second
                    new_self.0 = PlayState::Playing {
                        next_player: w.other(),
                        board: Default::default(),
                        player_map,
                        winner: None,
                    };
                }
            }
        }
        Ok(new_self)
    }
}
```

### StateProgram Implementation

```rust
impl StateProgram for DropFourGame {
    type T = GameTransition;

    fn new() -> Self {
        Default::default()
    }

    // No timer-based transitions for this game
    fn suspended_event(&self) -> Option<TransitionEvent<GameTransition>> {
        None
    }
}
```

### Server Setup

```rust
// In service/src/lib.rs
use aper_stateroom::AperStateroomService;
use stateroom_wasm::prelude::stateroom_wasm;

#[stateroom_wasm]
type DropFourService = AperStateroomService<StateMachineContainerProgram<DropFourGame>>;
```

---

## Summary

| Component | Purpose |
|-----------|---------|
| StateProgram | StateMachine + timestamps + timers |
| TransitionEvent | Timestamped, client-tagged transitions |
| StateMachineContainerProgram | Wrapper for any StateMachine |
| AperStateroomService | WebSocket server implementation |
| suspended_event() | Timer-based transition scheduling |
| StateProgramClient | Client with server time sync |

---

## Next Steps

Continue to [05-yew-frontend-deep-dive.md](05-yew-frontend-deep-dive.md) to learn about the WebAssembly frontend.
