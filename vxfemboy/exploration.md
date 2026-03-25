# vxfemboy Projects - Comprehensive Exploration

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/`

**Date:** 2026-03-26

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Project Overview](#project-overview)
3. [Individual Project Analyses](#individual-project-analyses)
4. [Common Rust Patterns](#common-rust-patterns)
5. [Rust Replication Plan](#rust-replication-plan)

---

## Executive Summary

The vxfemboy project collection consists of **5 distinct Rust (and one embedded C++) projects** focused on security tools, cryptography, communication, and data wiping. Each project demonstrates practical Rust application development with a focus on:

- **Security & Privacy:** Port spoofing, encryption, secure data wiping
- **Communication:** P2P chat, IRC clients
- **Embedded Systems:** ESP32-based hardware firmware

### Projects at a Glance

| Project | Type | Primary Use | Rust Crates |
|---------|------|-------------|-------------|
| **ghostport** | Security | Port spoofing/deception | tokio, clap, tracing |
| **purrcrypt** | Cryptography | PGP-like file encryption | k256, aes-gcm, serde |
| **spiderirc** | Communication | Decentralized P2P chat | libp2p, tokio, serde |
| **wipedicks** | Security | Secure file/device wiping | rand, clap |
| **acid-drop** | Embedded | T-Deck IRC client firmware | PlatformIO/Arduino |

---

## Project Overview

### 1. Ghostport - Port Spoofing Tool

**Purpose:** Confuse and mislead port scanners by responding with fake service signatures.

**Key Features:**
- Dynamic port emulation with customizable signatures
- Async TCP handling with Tokio
- Regex-based signature patterns
- Multiple logging levels (debug, verbose, quiet)

**Architecture:**
```
┌─────────────────────────────────────────────────────────┐
│                     Ghostport                            │
├─────────────────────────────────────────────────────────┤
│  CLI (clap)  │  Signature Parser  │  TCP Listener      │
│      │              │                    │              │
│      ▼              ▼                    ▼              │
│  ┌──────────────────────────────────────────────────┐  │
│  │              Tokio Async Runtime                  │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌──────────┐ │  │
│  │  │ Connection  │  │  Signature  │  │ Response │ │  │
│  │  │  Handler    │  │   Matcher   │  │  Writer  │ │  │
│  │  └─────────────┘  └─────────────┘  └──────────┘ │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### 2. PurrCrypt - Cat/Dog-Themed Encryption

**Purpose:** PGP-like file encryption with steganographic pet-speak encoding.

**Key Features:**
- Elliptic curve cryptography (secp256k1 - same as Bitcoin)
- AES-256-GCM authenticated encryption
- Steganographic encoding (cat/dog sounds)
- Key management with secure permissions

**Cryptography Pipeline:**
```
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│  Plain Text  │───▶│   ZLIB       │───▶│   ECDH       │
│   (Input)    │    │  Compress    │    │  Key Exchange│
└──────────────┘    └──────────────┘    └──────────────┘
                                              │
                                              ▼
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│  Pet Speak   │◀───│   Base64     │◀───│   AES-256    │
│   (Output)   │    │   Encode     │    │   -GCM       │
└──────────────┘    └──────────────┘    └──────────────┘
```

**Key Exchange (ECDH):**
```
┌─────────────┐                          ┌─────────────┐
│   Alice     │                          │    Bob      │
│             │                          │             │
│  Generate   │                          │   Generate  │
│  Ephemeral  │                          │   Keypair   │
│  Keypair    │                          │             │
│             │                          │             │
│  Compute    │────── Public Key ──────▶│   Receive   │
│  DH Secret  │                          │             │
│             │◀───── Ephemeral Pub ────│   Compute   │
│   Decrypt   │                          │   DH Secret │
│             │                          │             │
│  (Both derive identical shared secret) │             │
└─────────────┘                          └─────────────┘
```

### 3. SpiderIRC - P2P Chat Application

**Purpose:** Decentralized, serverless IRC-like chat using libp2p.

**Key Features:**
- Completely decentralized (no central servers)
- libp2p with Floodsub for message broadcasting
- mDNS for local peer discovery
- Kademlia DHT for global peer routing
- NAT traversal with AutoNAT
- Relay support for connectivity

**Network Architecture:**
```
┌─────────────────────────────────────────────────────────┐
│                    SpiderIRC Network                     │
│                                                          │
│    ┌─────────┐         ┌─────────┐         ┌─────────┐ │
│    │ Peer A  │◀───────▶│ Peer B  │◀───────▶│ Peer C  │ │
│    │         │  mDNS   │         │  Floodsub│         │ │
│    └────┬────┘         └────┬────┘         └────┬────┘ │
│         │                   │                   │       │
│         │    ┌──────────────┴──────────────┐    │       │
│         │    │      Kademlia DHT           │    │       │
│         │    │   (Distributed Hash Table)  │    │       │
│         │    └──────────────┬──────────────┘    │       │
│         ▼                   ▼                   ▼       │
│    ┌─────────┐         ┌─────────┐         ┌─────────┐ │
│    │ Peer D  │◀───────▶│ Peer E  │◀───────▶│ Peer F  │ │
│    └─────────┘         └─────────┘         └─────────┘ │
│                                                          │
│    ┌──────────────────────────────────────────────┐     │
│    │           Bootstrap Nodes (Public)            │     │
│    │  /dnsaddr/bootstrap.libp2p.io/p2p/Qm...      │     │
│    └──────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────┘
```

### 4. WipeDicks - Secure Data Wiping

**Purpose:** Multi-threaded secure file/device wiping with ASCII art overwrite.

**Key Features:**
- Multi-threaded wiping for performance
- Recursive directory wiping
- Configurable overwrite rounds
- Free space wiping option
- ASCII art overwrite patterns

**Wiping Process:**
```
┌─────────────────────────────────────────────────────────┐
│                     WipeDicks                            │
├─────────────────────────────────────────────────────────┤
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐   │
│  │   Parse     │──▶│   Spawn     │──▶│  Overwrite  │   │
│  │  Arguments  │   │   Threads   │   │   Pattern   │   │
│  └─────────────┘   └─────────────┘   └─────────────┘   │
│         │                │                   │          │
│         ▼                ▼                   ▼          │
│  -r Recursive    - One per file    - 8=D patterns      │
│  -n Rounds       - Thread pool     - Random selection  │
│  -w Free space   - Async I/O       - Multiple passes   │
└─────────────────────────────────────────────────────────┘
```

### 5. Acid-Drop - T-Deck IRC Firmware

**Purpose:** Custom firmware for LilyGo T-Deck (ESP32-S3) with IRC client functionality.

**Key Features:**
- LVGL-based UI with status bar
- WiFi network scanning and selection
- IRC client with 99 color support
- WireGuard VPN support
- Speaker for notifications
- Stored preferences

**Hardware Target:** LilyGo T-Deck (ESP32-S3FN16R8)

---

## Individual Project Analyses

For detailed analysis of each project, see:

- [`acid-drop.md`](./acid-drop.md) - Embedded IRC client firmware
- [`ghostport.md`](./ghostport.md) - Port spoofing tool
- [`purrcrypt.md`](./purrcrypt.md) - Encryption with pet-speak encoding
- [`spiderirc.md`](./spiderirc.md) - P2P chat application
- [`wipedicks.md`](./wipedicks.md) - Secure data wiping

---

## Common Rust Patterns

### 1. Error Handling with `thiserror`

Both `purrcrypt` and other projects use `thiserror` for elegant error types:

```rust
#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Key error: {0}")]
    Key(#[from] KeyError),
    #[error("Base64 error: {0}")]
    Base64(String),
}
```

### 2. Async Runtime with Tokio

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind(&cli.listen).await?;

    loop {
        let (mut stream, address) = listener.accept().await?;
        tokio::spawn(async move {
            // Handle connection
        });
    }
}
```

### 3. CLI with `clap` Derive

```rust
#[derive(Parser, Debug)]
struct Args {
    #[arg(short = 's', long = "signatures")]
    pub signatures: String,

    #[arg(short = 'd', long = "debug")]
    pub debug: bool,
}
```

### 4. Secure Key Management

```rust
#[cfg(unix)]
fs::set_permissions(secret_path, fs::Permissions::from_mode(0o600))?;
```

### 5. Structured Logging with `tracing`

```rust
use tracing::{debug, info, error, Level};

let subscriber = FmtSubscriber::builder()
    .with_max_level(Level::DEBUG)
    .finish();
tracing::subscriber::set_global_default(subscriber)?;
```

---

## Rust Replication Plan

### Building Similar Tools

#### 1. Security Tool (like Ghostport)

```rust
// Core dependencies
[dependencies]
tokio = { version = "1", features = ["full"] }
clap = { version = "4", features = ["derive"] }
tracing = "0.1"
anyhow = "1.0"

// Key patterns:
// - Use tokio::spawn for each connection
// - Parse configuration with clap derive
// - Use tracing for structured logging
// - Return anyhow::Result for flexible error handling
```

#### 2. Cryptography Tool (like PurrCrypt)

```rust
// Core dependencies
[dependencies]
k256 = { version = "0.13", features = ["ecdh"] }
aes-gcm = "0.10"
rand_core = "0.6"
thiserror = "2.0"
serde = { version = "1.0", features = ["derive"] }

// Key patterns:
// - Use EphemeralSecret for ECDH key exchange
// - Derive keys with HKDF (extract/expand)
// - Use thiserror for custom error types
// - Set proper file permissions (0o600) for private keys
```

#### 3. P2P Application (like SpiderIRC)

```rust
// Core dependencies
[dependencies]
libp2p = { version = "0.55", features = [
    "floodsub", "mdns", "noise",
    "tcp", "yamux", "kad", "autonat"
] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }

// Key patterns:
// - Use SwarmBuilder for network setup
// - Implement NetworkBehaviour for custom logic
// - Use mDNS for local discovery
// - Use Kademlia for global routing
```

#### 4. CLI Utility (like WipeDicks)

```rust
// Core dependencies
[dependencies]
clap = "4.5"
rand = "0.8"

// Key patterns:
// - Use clap Command/Arg API for simple CLIs
// - Thread::spawn for parallel operations
// - Use OpenOptions for low-level file operations
```

---

## Production Considerations

### Security

1. **Key Management:** Always use 0o600 permissions for private keys
2. **Memory Zeroing:** Clear sensitive data from memory after use
3. **Constant-Time Operations:** Use constant-time comparisons for secrets
4. **Audit Dependencies:** Regularly audit crate dependencies

### Performance

1. **Async I/O:** Use tokio for network-bound operations
2. **Thread Pools:** Use rayon or thread::spawn for CPU-bound work
3. **Buffered I/O:** Use BufReader/BufWriter for file operations
4. **Connection Pooling:** Reuse connections where possible

### Reliability

1. **Error Handling:** Use thiserror for library, anyhow for applications
2. **Logging:** Implement structured logging with tracing
3. **Testing:** Write comprehensive unit and integration tests
4. **Documentation:** Document public APIs with rustdoc

---

## Directory Structure

```
/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/
├── acid-drop/          # ESP32-S3 IRC firmware (C++/Arduino)
│   ├── src/
│   ├── firmware/
│   └── lib/
├── ghostport/          # Port spoofing tool
│   ├── src/
│   │   ├── main.rs
│   │   ├── cli.rs
│   │   └── handler.rs
│   └── signatures.txt
├── purrcrypt/          # PGP-like encryption
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── crypto.rs
│   │   ├── keys.rs
│   │   ├── keystore.rs
│   │   ├── config.rs
│   │   ├── debug.rs
│   │   └── cipher/
│   │       ├── mod.rs
│   │       └── patterns.rs
│   └── Cargo.toml
├── spiderirc/          # P2P chat application
│   ├── src/
│   │   ├── main.rs
│   │   ├── channel.rs
│   │   ├── discovery.rs
│   │   ├── message.rs
│   │   └── node.rs
│   └── Cargo.toml
└── wipedicks/          # Secure data wiping
    ├── src/
    │   └── main.rs
    └── Cargo.toml
```

---

## Conclusion

The vxfemboy projects demonstrate practical Rust development across multiple domains:

1. **Security tools** with async networking (ghostport)
2. **Cryptography** with proper key management (purrcrypt)
3. **P2P networking** with libp2p (spiderirc)
4. **System utilities** with multi-threading (wipedicks)
5. **Embedded firmware** for IoT devices (acid-drop)

Each project follows Rust best practices including proper error handling, async patterns, and secure coding practices. The codebase provides excellent examples for building similar tools in Rust.
