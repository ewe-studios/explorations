---
title: "Sync Protocol Deep Dive"
subtitle: "StateClient, StateServer, and message handling"
prerequisites: [02-data-structures-deep-dive.md](02-data-structures-deep-dive.md)
next: [04-stateroom-deep-dive.md](04-stateroom-deep-dive.md)
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.driftingspace/aper/aper/src/sync/
---

# Sync Protocol Deep Dive

This document explores how Aper synchronizes state between clients and servers using a message-based protocol.

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Message Types](#2-message-types)
3. [StateClient: Client-Side Synchronization](#3-stateclient-client-side-synchronization)
4. [StateServer: Server-Side Authority](#4-stateserver-server-side-authority)
5. [Version Tracking](#5-version-tracking)
6. [Optimistic State Updates](#6-optimistic-state-updates)
7. [Conflict and Recovery](#7-conflict-and-recovery)
8. [Complete Flow Example](#8-complete-flow-example)

---

## 1. Architecture Overview

Aper uses a **centralized client-server architecture** with optimistic updates:

```
┌─────────────┐                      ┌─────────────┐
│   Client    │                      │   Server    │
│             │                      │             │
│ ┌─────────┐ │  MessageToServer     │ ┌─────────┐ │
│ │StateClt │ │ ───────────────────► │ │StateSvr │ │
│ │         │ │                      │ │         │ │
│ │  Opt.   │ │ ◄──────────────────  │ │  Auth.  │ │
│ │  State  │ │  MessageToClient     │ │  State  │ │
│ └─────────┘ │                      │ └─────────┘ │
└─────────────┘                      └─────────────┘
```

### Key Concepts

| Concept | Description |
|---------|-------------|
| Optimistic State | Client updates immediately, before server confirmation |
| Authoritative State | Server's state is the source of truth |
| Transitions | Changes flow from client to server |
| Broadcast | Server broadcasts changes to all clients |
| Version Numbers | Track state evolution for consistency |

### Flow Summary

1. Client loads initial state from server
2. User triggers a transition locally
3. Client applies transition optimistically
4. Client sends transition to server
5. Server applies transition (or rejects)
6. Server broadcasts result to all clients
7. Clients update their state

---

## 2. Message Types

### Messages to Server

```rust
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum MessageToServer<S: StateMachine> {
    /// Client wants to apply a transition
    DoTransition {
        transition_number: ClientTransitionNumber,
        transition: S::Transition,
    },
    /// Client requests full state (after error/conflict)
    RequestState,
}
```

**DoTransition**: Client wants to modify state
- `transition_number`: Unique identifier for this transition
- `transition`: The actual state change

**RequestState**: Client needs full state reset
- Sent after conflict or detection of inconsistency

### Messages to Client

```rust
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum MessageToClient<S>
where
    S: StateMachine,
{
    /// Set local state (initial load or reset)
    SetState {
        state: S,
        version: StateVersionNumber,
    },

    /// Apply a transition made by a peer
    PeerTransition {
        transition: S::Transition,
        version: StateVersionNumber,
    },

    /// Acknowledge a transition made by this client
    ConfirmTransition {
        transition_number: ClientTransitionNumber,
        version: StateVersionNumber,
    },

    /// Reject a transition due to conflict
    Conflict {
        transition_number: ClientTransitionNumber,
        conflict: S::Conflict,
    },
}
```

### Version Numbers

```rust
#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Copy, Clone)]
pub struct StateVersionNumber(pub u32);

#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Copy, Clone)]
pub struct ClientTransitionNumber(pub u32);
```

- `StateVersionNumber`: Increments on server for each accepted transition
- `ClientTransitionNumber`: Unique per client for tracking their transitions

---

## 3. StateClient: Client-Side Synchronization

`StateClient` manages the client-side state and message handling.

### Structure

```rust
pub struct StateClient<S: StateMachine> {
    golden_state: Rc<S>,                          // Last confirmed state
    transitions: VecDeque<(ClientTransitionNumber, S::Transition)>, // Pending transitions
    optimistic_state: Rc<S>,                       // Current (including pending)
    version: StateVersionNumber,                   // Last confirmed version
    next_transition: ClientTransitionNumber,       // Next transition ID
}
```

### Key Methods

#### push_transition()

Creates a local transition and queues it for sending:

```rust
pub fn push_transition(
    &mut self,
    transition: S::Transition,
) -> Result<MessageToServer<S>, S::Conflict> {
    // Apply transition to optimistic state immediately
    let current_state = self.state();
    self.optimistic_state = Rc::new(current_state.apply(&transition)?);

    // Queue transition for sending to server
    let transition_number = self.next_transition();
    self.transitions.push_back((transition_number, transition.clone()));

    // Return message to send
    Ok(MessageToServer::DoTransition {
        transition_number,
        transition,
    })
}
```

**Key insight**: Optimistic state is updated BEFORE server confirmation.

#### receive_message_from_server()

Handles all server messages:

```rust
pub fn receive_message_from_server(
    &mut self,
    message: MessageToClient<S>,
) -> Option<MessageToServer<S>> {
    match message {
        MessageToClient::SetState { state, version } => {
            // Full state reset
            let state = Rc::new(state);
            self.golden_state = state.clone();
            self.optimistic_state = state;
            self.transitions = VecDeque::new();
            self.version = version;
            None
        }

        MessageToClient::ConfirmTransition { transition_number, version } => {
            // Server accepted our transition
            self.process_confirmation(transition_number, version)
        }

        MessageToClient::PeerTransition { transition, version } => {
            // Another client made a change
            self.process_peer_transition(transition, version)
        }

        MessageToClient::Conflict { transition_number, conflict } => {
            // Server rejected our transition
            self.process_conflict(transition_number, conflict)
        }
    }
}
```

#### state()

Returns current optimistic state:

```rust
pub fn state(&self) -> Rc<S> {
    self.optimistic_state.clone()
}
```

### Confirmation Handling

```rust
fn process_confirmation(
    &mut self,
    transition_number: ClientTransitionNumber,
    version: StateVersionNumber,
) -> Option<MessageToServer<S>> {
    // Pop the confirmed transition
    if let Some((optimistic_transition_number, transition)) = self.transitions.pop_front() {
        if optimistic_transition_number != transition_number {
            // Out of order - something is wrong
            return Some(MessageToServer::RequestState);
        }

        // Apply to golden state
        if let Ok(state) = self.golden_state.apply(&transition) {
            self.golden_state = Rc::new(state);
        } else {
            // Should not happen - server confirmed something that fails locally
            return Some(MessageToServer::RequestState);
        }
        self.version = version;
        None
    } else {
        // Server confirmed something we don't have
        Some(MessageToServer::RequestState)
    }
}
```

### Peer Transition Handling

```rust
fn process_peer_transition(
    &mut self,
    transition: S::Transition,
    version: StateVersionNumber,
) -> Option<MessageToServer<S>> {
    // Check version ordering
    if self.version != version.prior_version() {
        return Some(MessageToServer::RequestState);
    }

    // Apply to golden state
    let state = if let Ok(state) = self.golden_state.apply(&transition) {
        state
    } else {
        // Peer transition caused conflict locally
        return Some(MessageToServer::RequestState);
    };

    self.golden_state = Rc::new(state);
    self.version = version;

    // Replay our pending transitions on top of new golden state
    let mut state = self.golden_state.clone();
    for (_, transition) in &self.transitions {
        if let Ok(st) = state.apply(transition) {
            state = Rc::new(st);
        };
    }
    self.optimistic_state = state;

    None
}
```

---

## 4. StateServer: Server-Side Authority

`StateServer` is the authoritative source of truth.

### Structure

```rust
pub struct StateServer<S: StateMachine> {
    pub version: StateVersionNumber,
    state: S,
}
```

### Key Methods

#### receive_message()

Processes client messages:

```rust
pub fn receive_message(
    &mut self,
    message: MessageToServer<S>,
) -> StateServerMessageResponse<S> {
    match message {
        MessageToServer::DoTransition { transition_number, transition } => {
            match self.state.apply(&transition) {
                Ok(state) => {
                    self.state = state;
                    self.version.0 += 1;

                    StateServerMessageResponse {
                        // Confirm to the sender
                        reply_message: MessageToClient::ConfirmTransition {
                            transition_number,
                            version: self.version,
                        },
                        // Broadcast to everyone else
                        broadcast_message: Some(MessageToClient::PeerTransition {
                            transition,
                            version: self.version,
                        }),
                    }
                }
                Err(conflict) => {
                    // TODO: Handle conflict
                    todo!()
                }
            }
        }

        MessageToServer::RequestState => {
            StateServerMessageResponse {
                reply_message: MessageToClient::SetState {
                    state: self.state.clone(),
                    version: self.version,
                },
                broadcast_message: None,
            }
        }
    }
}
```

### Response Type

```rust
pub struct StateServerMessageResponse<S: StateMachine> {
    pub reply_message: MessageToClient<S>,       // To the sender
    pub broadcast_message: Option<MessageToClient<S>>, // To everyone else
}
```

---

## 5. Version Tracking

Version numbers ensure consistency across the network.

### Server Version Increment

```rust
// Server increments version on each accepted transition
self.version.0 += 1;
```

### Client Version Validation

```rust
// Client checks peer transition version
if self.version != version.prior_version() {
    return Some(MessageToServer::RequestState);
}
```

### Version Flow

```
Initial: Server version = 0

Client 1: DoTransition(transition_number=0)
  → Server applies, version = 1
  → ConfirmTransition(version=1) to Client 1
  → PeerTransition(version=1) to Client 2

Client 2: Receives PeerTransition(version=1)
  → Validates: self.version(0) == version.prior(1) ✓
  → Applies transition
```

---

## 6. Optimistic State Updates

Optimistic updates make the UI feel instant.

### The Pattern

```rust
// 1. User clicks button
on_click() {
    // 2. Create transition
    let transition = Counter::increment(1);

    // 3. Push transition (applies optimistically)
    let message = client.push_transition(transition).unwrap();

    // 4. UI updates immediately (using optimistic state)
    render(client.state());

    // 5. Send to server (async)
    websocket.send(message);
}
```

### State Layers

```
┌────────────────────────────────────┐
│  Optimistic State (what UI sees)   │
│  = Golden State + Pending Trans.   │
├────────────────────────────────────┤
│  Golden State (server-confirmed)   │
├────────────────────────────────────┤
│  Pending Transitions (unconfirmed) │
└────────────────────────────────────┘
```

### Replaying Pending Transitions

When peer transitions arrive:

```rust
// Start from new golden state
let mut state = self.golden_state.clone();

// Replay our pending transitions in order
for (_, transition) in &self.transitions {
    if let Ok(st) = state.apply(transition) {
        state = Rc::new(st);
    }
}
self.optimistic_state = state;
```

---

## 7. Conflict and Recovery

### Types of Conflicts

**Business Logic Conflict**: Transition is invalid

```rust
// Example: Insufficient funds
account.apply(&Withdraw { amount: 1000 })
// Returns Err(InsufficientFunds)
```

**Version Conflict**: State drifted

```rust
// Client's version doesn't match server's expectation
if self.version != version.prior_version() {
    // Versions don't line up - request full state
    return Some(MessageToServer::RequestState);
}
```

### Conflict Response

```rust
// Server rejects with conflict
MessageToClient::Conflict {
    transition_number: ClientTransitionNumber(5),
    conflict: BankingConflict::InsufficientFunds { ... },
}

// Client removes transition from queue
// Optimistic state rolls back
if let Some((optimistic_transition_number, _)) = self.transitions.pop_front() {
    // Transition removed, state recalculated from golden + remaining pending
}
```

### State Recovery

When things go wrong, client requests full state:

```rust
// Client sends
MessageToServer::RequestState

// Server responds with complete state
MessageToClient::SetState {
    state: current_state,
    version: current_version,
}

// Client resets everything
self.golden_state = Rc::new(state);
self.optimistic_state = self.golden_state.clone();
self.transitions = VecDeque::new();
self.version = version;
```

---

## 8. Complete Flow Example

Let's trace a complete multi-client scenario:

### Initial State

```
Server: Counter(0), version=0
Client 1: Not connected
Client 2: Not connected
```

### Client 1 Connects

```
Client 1 → Server: (WebSocket connection)
Server → Client 1: SetState { state: Counter(0), version: 0 }
Client 1: golden_state=Counter(0), optimistic_state=Counter(0)
```

### Client 1 Increments

```
User action: Click "+1"
Client 1: push_transition(Increment(1))
  - optimistic_state = Counter(1)
  - transitions = [(0, Increment(1))]
Client 1 → Server: DoTransition { transition_number: 0, transition: Increment(1) }

Server: apply(Increment(1)) → Counter(1)
  - version = 1
Server → Client 1: ConfirmTransition { transition_number: 0, version: 1 }
Server → Client 2: (not connected yet)

Client 1: receive_message(ConfirmTransition)
  - golden_state = Counter(1)
  - transitions = []
  - version = 1
```

### Client 2 Connects

```
Client 2 → Server: (WebSocket connection)
Server → Client 2: SetState { state: Counter(1), version: 1 }
Client 2: golden_state=Counter(1), optimistic_state=Counter(1)
```

### Concurrent Modifications

```
Client 1: Click "+1" → DoTransition(Increment(1), transition_number=1)
Client 2: Click "+5" → DoTransition(Increment(5), transition_number=0)

Server receives Client 1's transition first:
  - apply(Increment(1)) → Counter(2)
  - version = 2
  - → ConfirmTransition(1, version=2) to Client 1
  - → PeerTransition(Increment(1), version=2) to Client 2

Server receives Client 2's transition:
  - apply(Increment(5)) → Counter(7)
  - version = 3
  - → ConfirmTransition(0, version=3) to Client 2
  - → PeerTransition(Increment(5), version=3) to Client 1

Final state on all clients: Counter(7), version=3
```

---

## Summary

| Component | Purpose |
|-----------|---------|
| StateClient | Manages client-side optimistic state |
| StateServer | Authoritative state on server |
| MessageToServer | Client → Server messages |
| MessageToClient | Server → Client messages |
| Version Numbers | Track state consistency |
| Optimistic Updates | Instant UI feedback |
| Conflict Recovery | RequestState resets client |

---

## Next Steps

Continue to [04-stateroom-deep-dive.md](04-stateroom-deep-dive.md) to learn about Stateroom server integration.
