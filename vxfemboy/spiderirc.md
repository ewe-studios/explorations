# SpiderIRC - P2P Chat Application

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/spiderirc/`

---

## Overview

**SpiderIRC** is a decentralized, peer-to-peer chat application built on libp2p. It creates a mesh network where each participant (spider) helps weave and maintain the communication web, with no central servers required.

### What It Does

1. **Creates P2P connections** between participants using libp2p
2. **Broadcasts messages** using the Floodsub gossip protocol
3. **Discovers peers** locally via mDNS and globally via Kademlia DHT
4. **Traverses NATs** using AutoNAT and relay nodes
5. **Supports topic-based channels** for organized discussions

### Key Features

- **Completely decentralized** - No single point of failure
- **Direct peer connections** - Messages flow directly between users
- **NAT traversal** - Works behind routers with AutoNAT
- **Local discovery** - Finds peers on LAN via mDNS
- **Global routing** - Uses Kademlia DHT for internet-wide discovery
- **Relay support** - Can route through relay nodes when direct connection fails

---

## Architecture

### Network Topology

```
┌─────────────────────────────────────────────────────────────────┐
│                    SpiderIRC Network                              │
│                                                                   │
│   Internet                                                        │
│   ┌─────────────────────────────────────────────────────────┐   │
││                                                              │   │
││    ┌─────────┐         ┌─────────┐         ┌─────────┐     ││   │
││    │ Peer A  │◀───────▶│ Peer B  │◀───────▶│ Peer C  │     ││   │
││    │ (You)   │  mDNS   │         │Floodsub │         │     ││   │
││    └────┬────┘         └────┬────┘         └────┬────┘     ││   │
││         │                   │                   │          ││   │
││         │    ┌──────────────┴──────────────┐    │          ││   │
││         │    │      Kademlia DHT           │    │          ││   │
││         │    │   (Distributed Hash Table)  │    │          ││   │
││         │    │   Finds peers globally      │    │          ││   │
││         │    └──────────────┬──────────────┘    │          ││   │
││         ▼                   ▼                   ▼          ││   │
││    ┌─────────┐         ┌─────────┐         ┌─────────┐     ││   │
││    │ Peer D  │◀───────▶│ Peer E  │◀───────▶│ Peer F  │     ││   │
││    └─────────┘         └─────────┘         └─────────┘     ││   │
││                                                              │   │
││   ┌──────────────────────────────────────────────────────┐ │   │
││   │          Bootstrap Nodes (Public, Well-Known)        │ │   │
││   │  /dnsaddr/bootstrap.libp2p.io/p2p/QmNnooDu7bfj...   │ │   │
││   │  /dnsaddr/bootstrap.libp2p.io/p2p/QmQCU2EcMqAqQ...  │ │   │
││   │  /dnsaddr/bootstrap.libp2p.io/p2p/QmbLHAnMoJPWSCR... │ │   │
││   └──────────────────────────────────────────────────────┘ │   │
││                                                              │   │
│└─────────────────────────────────────────────────────────────┘   │
│                                                                   │
│   Local Network (LAN)                                             │
│   ┌─────────┐         ┌─────────┐         ┌─────────┐          │
│   │Spider 1 │◀───────▶│Spider 2 │◀───────▶│Spider 3 │          │
│   └─────────┘  mDNS   └─────────┘  mDNS   └─────────┘          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Module Structure

```
src/
├── main.rs             # Entry point, libp2p swarm setup, event loop
├── channel.rs          # Channel store for topic management
├── discovery.rs        # Kademlia DHT discovery behavior
├── message.rs          # Message types (Chat, Join, Leave, etc.)
└── node.rs             # Peer tracking and management
```

---

## Implementation Details

### 1. Libp2p Stack Configuration

```rust
// src/main.rs
use libp2p::{
    swarm::{NetworkBehaviour, SwarmEvent},
    core::{upgrade, transport::Transport},
    floodsub::{Floodsub, FloodsubEvent, Topic},
    identity,
    mdns,
    noise,
    tcp,
    yamux,
    autonat,
    identify,
    relay,
};

// Generate random peer ID
let local_key_pair = identity::Keypair::generate_ed25519();
let local_peer_id = PeerId::from(local_key_pair.public());

// Create network behavior
let mut behaviour = ChatBehaviour {
    floodsub: Floodsub::new(local_peer_id),
    mdns: mdns::tokio::Behaviour::new(
        mdns::Config::default(),
        local_peer_id
    )?,
    identify: identify::Behaviour::new(
        identify::Config::new(
            "/chat/1.0.0".to_string(),
            local_key_pair.public(),
        )
    ),
    autonat: autonat::Behaviour::new(
        local_peer_id,
        autonat::Config::default(),
    ),
    relay: relay::Behaviour::new(
        local_peer_id,
        relay::Config::default(),
    ),
};

// Subscribe to topic
behaviour.floodsub.subscribe(topic.clone());
```

### 2. SwarmBuilder (libp2p 0.55 API)

```rust
// Build and start the swarm using the new API in libp2p 0.55
let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key_pair)
    .with_tokio()
    .with_tcp(
        tcp::Config::default(),
        noise::Config::new,
        || yamux::Config::default()  // Wrap in a closure as required
    )?
    .with_behaviour(|_| behaviour)?
    .with_swarm_config(|c| c)
    .build();

// Listen on all interfaces with random port
swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
```

### 3. Bootstrap Node Dialing

```rust
// These are public libp2p bootstrap nodes that help with peer discovery
for addr in [
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN",
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa",
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb",
] {
    match addr.parse::<libp2p::Multiaddr>() {
        Ok(addr) => {
            println!("Dialing bootstrap node: {}", addr);
            match swarm.dial(addr) {
                Ok(_) => println!("Dialed bootstrap node successfully"),
                Err(e) => println!("Failed to dial bootstrap node: {:?}", e),
            }
        },
        Err(err) => println!("Failed to parse bootstrap address: {:?}", err),
    }
}
```

### 4. Main Event Loop

```rust
// Main event loop
loop {
    select! {
        // Handle user input from stdin
        line = stdin.next_line() => {
            if let Ok(Some(line)) = line {
                if line.starts_with("/quit") {
                    break;
                }

                let chat_message = ChatMessage {
                    sender: username.clone(),
                    channel: args.channel.clone(),
                    message: line,
                    timestamp: chrono::Utc::now().timestamp(),
                };

                let json = serde_json::to_string(&chat_message)?;
                swarm.behaviour_mut().floodsub.publish(
                    topic.clone(),
                    json.clone().into_bytes()
                );
            }
        },

        // Handle incoming messages from other peers
        message = response_rcv.recv() => {
            if let Some(message) = message {
                if message.channel == args.channel {
                    println!("[{}] {}: {}",
                        message.channel, message.sender, message.message);
                }
            }
        },

        // Handle swarm events
        event = swarm.select_next_some() => {
            match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Listening on: {}", address);
                }
                SwarmEvent::Behaviour(ChatEvent::Floodsub(event)) => {
                    handle_floodsub_message(event, &chat_context);
                }
                SwarmEvent::Behaviour(ChatEvent::Mdns(event)) => {
                    handle_mdns_event(event, &mut swarm.behaviour_mut().floodsub);
                }
                SwarmEvent::Behaviour(ChatEvent::Identify(event)) => {
                    handle_identify_event(event, &mut swarm);
                }
                SwarmEvent::Behaviour(ChatEvent::AutoNat(event)) => {
                    handle_autonat_event(event);
                }
                SwarmEvent::Behaviour(ChatEvent::Relay(event)) => {
                    handle_relay_event(event);
                }
                _ => {}
            }
        }
    }
}
```

### 5. Floodsub Message Handling

```rust
fn handle_floodsub_message(
    event: FloodsubEvent,
    ctx: &ChatContext,
) {
    if let FloodsubEvent::Message(message) = event {
        if let Ok(chat_message) = serde_json::from_slice::<ChatMessage>(
            &message.data
        ) {
            let _ = ctx.response_sender.send(chat_message);
        }
    }
}
```

### 6. mDNS Peer Discovery

```rust
fn handle_mdns_event(
    event: mdns::Event,
    floodsub: &mut Floodsub,
) {
    match event {
        mdns::Event::Discovered(peers) => {
            for (peer_id, _addr) in peers {
                // Add discovered peer to floodsub mesh
                floodsub.add_node_to_partial_view(peer_id);
            }
        }
        mdns::Event::Expired(peers) => {
            for (peer_id, _addr) in peers {
                // Remove expired peer from mesh
                floodsub.remove_node_from_partial_view(&peer_id);
            }
        }
    }
}
```

### 7. Identify Protocol Handler

```rust
fn handle_identify_event(
    event: identify::Event,
    swarm: &mut libp2p::Swarm<ChatBehaviour>,
) {
    match event {
        identify::Event::Received { peer_id, info, .. } => {
            println!("Identified peer {}: {:?}", peer_id, info);

            // Add external addresses if publicly reachable
            for addr in info.listen_addrs {
                if !addr.to_string().contains("/ip4/127.0.0.1/") &&
                   !addr.to_string().contains("/ip4/192.168.") &&
                   !addr.to_string().contains("/ip4/10.") {
                    println!("Adding potentially public address: {}", addr);
                    swarm.add_external_address(addr);
                }
            }
        }
        _ => {}
    }
}
```

### 8. AutoNAT Status Monitoring

```rust
fn handle_autonat_event(event: autonat::Event) {
    match event {
        autonat::Event::StatusChanged { old, new } => {
            println!("NAT status changed from {:?} to {:?}", old, new);
            match new {
                autonat::NatStatus::Public(external_addr) => {
                    println!("We are publicly reachable at {}", external_addr);
                }
                autonat::NatStatus::Private => {
                    println!("We are behind a NAT and not publicly reachable");
                }
                _ => {}
            }
        }
        _ => {}
    }
}
```

---

## Libp2p Protocol Stack

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                         │
│                     (SpiderIRC Chat)                         │
├─────────────────────────────────────────────────────────────┤
│                   Application Protocols                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │  Floodsub   │  │  Identify   │  │   AutoNAT   │         │
│  │  (Gossip)   │  │  (Info)     │  │  (NAT Test) │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
├─────────────────────────────────────────────────────────────┤
│                    Routing Layer                              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Kademlia DHT                            │   │
│  │   - Peer routing                                      │   │
│  │   - Content addressing                                │   │
│  │   - Record storage                                    │   │
│  └─────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────┤
│                    Transport Layer                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │     TCP     │  │    mDNS     │  │    Relay    │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
├─────────────────────────────────────────────────────────────┤
│                 Security/Multiplexing                         │
│  ┌─────────────┐  ┌─────────────┐                          │
│  │    NOISE    │  │   Yamux     │                          │
│  │  (Encrypt)  │  │ (Multiplex) │                          │
│  └─────────────┘  └─────────────┘                          │
└─────────────────────────────────────────────────────────────┘
```

---

## Dependencies

```toml
[package]
name = "spiderirc"
version = "0.1.0"
edition = "2021"

[dependencies]
libp2p = { version = "0.55", features = [
    "floodsub",      # Gossip/flood protocol
    "mdns",          # Local peer discovery
    "noise",         # Encryption
    "tcp",           # TCP transport
    "yamux",         # Connection multiplexing
    "kad",           # Kademlia DHT
    "macros",        # NetworkBehaviour derive
    "tokio",         # Async runtime
    "autonat",       # NAT traversal
    "identify",      # Peer identification
    "relay",         # Relay nodes
] }
tokio = { version = "1.14", features = ["full"] }
futures = "0.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
clap = { version = "4.0.0-rc.3", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = "0.3"
chrono = "0.4"
```

---

## Usage

### Building

```bash
git clone https://github.com/vxfemboy/spiderirc.git
cd spiderirc
cargo build
```

### Running

```bash
# With default username and channel
cargo run

# With custom username
cargo run -- --username SpiderFriend

# With custom channel
cargo run -- --channel spiderfriends --username MySpider
```

### Command Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `--channel <NAME>` | Channel/topic to join | `general` |
| `--username <NAME>` | Your username | `anon-XXXXX` |

---

## Message Format

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
struct ChatMessage {
    sender: String,
    channel: String,
    message: String,
    timestamp: i64,
}
```

Messages are JSON-encoded before being published via Floodsub.

---

## Peer Discovery Mechanisms

### 1. mDNS (Local Network)

```
┌─────────────────────────────────────────────────────────┐
│                  Local Network (LAN)                     │
│                                                          │
│   ┌─────────┐      ┌─────────┐      ┌─────────┐        │
│   │ Spider 1│◀────▶│ Spider 2│◀────▶│ Spider 3│        │
│   └─────────┘ mDNS └─────────┘ mDNS └─────────┘        │
│        │              │              │                  │
│        └──────────────┴──────────────┘                  │
│              Multicast Discovery                         │
│                                                          │
│   "Hey, who's on the network?"                          │
│   "I am Spider 2 at 192.168.1.100:45678!"              │
│   "I am Spider 3 at 192.168.1.101:45679!"              │
└─────────────────────────────────────────────────────────┘
```

### 2. Kademlia DHT (Global)

```
┌─────────────────────────────────────────────────────────────┐
│                      Internet                                 │
│                                                               │
│   ┌─────────┐         ┌─────────┐         ┌─────────┐      │
│   │ Peer A  │◀───────▶│ Peer B  │◀───────▶│ Peer C  │      │
│   │         │  Query  │         │  Query  │         │      │
│   └────┬────┘ ─ ─ ─ ─ └────┬────┘ ─ ─ ─ └────┬────┘      │
│        │                   │                   │           │
│        │    ┌──────────────┴──────────────┐    │           │
│        │    │    Kademlia DHT Routing     │    │           │
│        │    │  "Find peers near key X"    │    │           │
│        │    │  "Store peer info at X"     │    │           │
│        │    └─────────────────────────────┘    │           │
│        ▼                                       ▼           │
│   ┌─────────┐                            ┌─────────┐       │
│   │ Peer D  │                            │ Peer E  │       │
│   └─────────┘                            └─────────┘       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## NAT Traversal

### The NAT Problem

```
┌─────────────────────────────────────────────────────────────┐
│                                                              │
│   Your Computer           NAT/Router          Internet       │
│   ┌──────────┐           ┌─────────┐          ┌──────────┐ │
│   │ Spider   │           │         │          │  Other   │ │
│   │ 192.168. │◀─────────▶│  NAT    │◀───────▶│  Peers   │ │
│   │   1.100  │  Private  │ Public  │  Public  │          │ │
│   └──────────┘  Address  │  IP     │  Address │          │ │
│                          │         │          │          │ │
│   Problem: Can't receive incoming connections!              │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### AutoNAT Solution

```rust
// AutoNAT determines if you're behind NAT
fn handle_autonat_event(event: autonat::Event) {
    match event {
        autonat::Event::StatusChanged { old, new } => {
            match new {
                autonat::NatStatus::Public(external_addr) => {
                    println!("Publicly reachable at {}", external_addr);
                    // Peers can connect directly
                }
                autonat::NatStatus::Private => {
                    println!("Behind NAT");
                    // Need relay or hole punching
                }
            }
        }
        _ => {}
    }
}
```

### Relay Fallback

When direct connection fails, messages can route through relay nodes:

```
┌─────────────┐    ┌─────────┐    ┌─────────────┐
│   Peer A    │───▶│  Relay  │───▶│   Peer B    │
│ (Behind NAT)│    │  Node   │    │ (Behind NAT)│
└─────────────┘    └─────────┘    └─────────────┘
```

---

## Files

- **Main Entry:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/spiderirc/src/main.rs`
- **Channel:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/spiderirc/src/channel.rs`
- **Discovery:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/spiderirc/src/discovery.rs`
- **Message:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/spiderirc/src/message.rs`
- **Node:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/spiderirc/src/node.rs`
- **Cargo.toml:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/spiderirc/Cargo.toml`
- **Documentation:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/spiderirc/README.md`

---

## Summary

SpiderIRC demonstrates:

1. **Libp2p 0.55 API** - Modern swarm builder pattern
2. **Multiple discovery protocols** - mDNS + Kademlia DHT
3. **Floodsub gossip** - Message broadcasting to mesh
4. **NAT traversal** - AutoNAT + relay support
5. **Identify protocol** - Peer information exchange
6. **Tokio async** - Select-based event loop
7. **Serde serialization** - JSON message encoding

It's an excellent example of building decentralized applications with libp2p in Rust.
