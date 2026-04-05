---
source: /home/darkvoid/Boxxed/@formulas/src.Peer2Peer/hyperswarm
repository: github.com:holepunchto/hyperswarm
explored_at: 2026-04-04
---

# Rust Revision: Building P2P Applications with libp2p

## Overview

This guide shows how to replicate Hyperswarm's P2P networking patterns in Rust using libp2p. We cover DHT-based discovery, encrypted connections, and application patterns for chat, file sharing, and distributed systems.

## Why libp2p for Rust P2P?

| Feature | Hyperswarm (JS) | Rust libp2p |
|---------|-----------------|-------------|
| DHT | hyperdht (Kademlia) | Kademlia |
| Encryption | Noise Protocol | Noise/TLS |
| Transport | TCP/UDP | TCP/QUIC/WebSocket |
| Multiplexing | Single stream | mTLS/Yamux |
| NAT Traversal | Hole punching | libp2p-relay |
| Identity | Noise keys | Ed25519/RSA |

## Project Setup

### Cargo.toml

```toml
[package]
name = "p2p-network"
version = "0.1.0"
edition = "2021"

[dependencies]
# libp2p core
libp2p = { version = "0.53", features = [
    "tcp",
    "noise",
    "yamux",
    "kademlia",
    "gossipsub",
    "mdns",
    "quic",
    "relay",
    "identify",
    "ping",
    "macros",
    "tokio",
] }

# Async runtime
tokio = { version = "1.35", features = ["full"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Cryptography
ed25519-dalek = "2.1"
sha2 = "0.10"

# Utilities
futures = "0.3"
async-trait = "0.1"
thiserror = "1.0"
anyhow = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

## Core P2P Network Layer

### Network Configuration

```rust
// src/network/mod.rs

use libp2p::{
    identity::{Keypair, ed25519},
    noise, tcp, yamux,
    gossipsub, kademlia, mdns, quic,
    Multiaddr, PeerId, Swarm, SwarmBuilder,
};
use std::time::Duration;

/// P2P Network configuration
pub struct NetworkConfig {
    pub listen_addresses: Vec<Multiaddr>,
    pub bootstrap_nodes: Vec<Multiaddr>,
    pub enable_quic: bool,
    pub enable_relay: bool,
    pub max_connections: u32,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addresses: vec![
                "/ip4/0.0.0.0/tcp/0".parse().unwrap(),
                "/ip4/0.0.0.0/udp/0/quic-v1".parse().unwrap(),
            ],
            bootstrap_nodes: Vec::new(),
            enable_quic: true,
            enable_relay: false,
            max_connections: 100,
        }
    }
}

/// Build libp2p Swarm with all protocols
pub fn build_swarm(
    keypair: Keypair,
    config: &NetworkConfig,
) -> Result<Swarm<NetworkBehaviour>, anyhow::Error> {
    let peer_id = PeerId::from(keypair.public());
    
    // Create transport
    let transport = build_transport(keypair.clone(), config)?;
    
    // Create network behaviour
    let behaviour = NetworkBehaviour::new(peer_id, config)?;
    
    // Build swarm
    let swarm = SwarmBuilder::with_tokio_executor(transport, behaviour, peer_id)
        .with_behaviour(|_| Ok(behaviour))?
        .with_swarm_config(|cfg| {
            cfg.with_max_connections(config.max_connections)
                .with_idle_connection_timeout(Duration::from_secs(60))
        })
        .build();
    
    Ok(swarm)
}

fn build_transport(
    keypair: Keypair,
    config: &NetworkConfig,
) -> Result<libp2p::core::transport::Boxed<(PeerId, libp2p::core::muxing::StreamMuxerBox)>, anyhow::Error> {
    // Noise authentication
    let noise_config = noise::Config::new(&keypair)?;
    
    // Yamux multiplexing
    let yamux_config = yamux::Config::default();
    
    // TCP transport
    let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default())
        .upgrade(libp2p::core::upgrade::Version::V1)
        .authenticate(noise_config.clone())
        .multiplex(yamux_config.clone())
        .boxed();
    
    if !config.enable_quic {
        return Ok(tcp_transport);
    }
    
    // QUIC transport
    let quic_config = quic::Config::new(&keypair);
    let quic_transport = quic::tokio::Transport::new(quic_config)
        .boxed();
    
    // Combine transports
    Ok(tcp_transport
        .or_transport(quic_transport)
        .map(|either, _| match either {
            libp2p::core::either::EitherOutput::First(tcp) => tcp,
            libp2p::core::either::EitherOutput::Second(quic) => quic,
        })
        .boxed())
}

/// Combined network behaviour
#[derive(libp2p::NetworkBehaviour)]
pub struct NetworkBehaviour {
    pub kademlia: kademlia::Behaviour,
    pub gossipsub: gossipsub::Behaviour,
    pub mdns: mdns::tokio::Behaviour,
    pub identify: libp2p::identify::Behaviour,
    pub ping: libp2p::ping::Behaviour,
}

impl NetworkBehaviour {
    pub fn new(peer_id: PeerId, config: &NetworkConfig) -> Result<Self, anyhow::Error> {
        // Kademlia configuration
        let kademlia_config = kademlia::Config::default();
        let kademlia_store_config = kademlia::store::MemoryStoreConfig::new(1024);
        let kademlia = kademlia::Behaviour::with_config(
            peer_id,
            kademlia::store::MemoryStore::with_config(peer_id, kademlia_store_config),
            kademlia_config,
        );
        
        // Gossipsub configuration
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .build()?;
        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(keypair.clone()),
            gossipsub_config,
        )?;
        
        // mDNS for local discovery
        let mdns = mdns::tokio::Behaviour::new(
            mdns::Config::default(),
            peer_id,
        )?;
        
        // Identify protocol
        let identify_config = libp2p::identify::Config::new(
            "/p2p-network/1.0.0".to_string(),
            keypair.public(),
        );
        let identify = libp2p::identify::Behaviour::new(identify_config);
        
        // Ping protocol
        let ping = libp2p::ping::Behaviour::default();
        
        Ok(Self {
            kademlia,
            gossipsub,
            mdns,
            identify,
            ping,
        })
    }
}
```

### Topic-Based Discovery

```rust
// src/discovery/topic.rs

use libp2p::{kademlia, Multiaddr, PeerId};
use sha2::{Sha256, Digest};
use std::collections::HashMap;

/// Topic for peer grouping (32 bytes like hyperswarm)
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Topic(pub [u8; 32]);

impl Topic {
    /// Create topic from string
    pub fn from_string(s: &str) -> Self {
        let mut hash = [0u8; 32];
        let digest = Sha256::digest(s.as_bytes());
        hash.copy_from_slice(&digest);
        Topic(hash)
    }
    
    /// Create topic from bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Topic(bytes)
    }
    
    /// Get Kademlia record key
    pub fn to_kademlia_key(&self) -> kademlia::record::Key {
        kademlia::record::Key::new(&self.0)
    }
}

/// Peer discovery manager
pub struct TopicDiscovery {
    topics: HashMap<Topic, DiscoveryState>,
    local_peer_id: PeerId,
}

enum DiscoveryState {
    Announcing,  // Server mode
    Discovering, // Client mode
    Both,        // Server + Client
}

impl TopicDiscovery {
    pub fn new(local_peer_id: PeerId) -> Self {
        Self {
            topics: HashMap::new(),
            local_peer_id,
        }
    }
    
    /// Join topic in server mode (announce)
    pub fn announce(&mut self, topic: Topic) {
        self.topics.insert(topic.clone(), DiscoveryState::Announcing);
        // Add provider record to Kademlia
        // kademlia.add_provider(topic.to_kademlia_key(), provider_record)
    }
    
    /// Join topic in client mode (discover)
    pub fn discover(&mut self, topic: Topic) {
        self.topics.insert(topic.clone(), DiscoveryState::Discovering);
        // Query Kademlia for providers
        // kademlia.get_providers(topic.to_kademlia_key())
    }
    
    /// Get peers for topic
    pub fn get_peers(&self, topic: &Topic) -> Vec<PeerId> {
        // Return peers discovered for this topic
        Vec::new()
    }
}

/// Message framing for topic-based routing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TopicMessage {
    pub topic: [u8; 32],
    pub payload: Vec<u8>,
    pub sender: String,
    pub timestamp: u64,
}

impl TopicMessage {
    pub fn new(topic: &Topic, payload: Vec<u8>, sender: PeerId) -> Self {
        Self {
            topic: topic.0,
            payload,
            sender: sender.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}
```

## Application Patterns

### P2P Chat Implementation

```rust
// src/app/chat.rs

use crate::network::{NetworkBehaviour, NetworkConfig};
use futures::{stream::StreamExt, AsyncReadExt, AsyncWriteExt};
use libp2p::{gossipsub, PeerId, Swarm, SwarmEvent};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use tracing::{info, warn, error};

/// P2P Chat application
pub struct P2PChat {
    swarm: Swarm<NetworkBehaviour>,
    topic: gossipsub::IdentTopic,
    peers: Vec<PeerId>,
}

impl P2PChat {
    pub async fn new(room_name: &str, config: NetworkConfig) -> Result<Self, anyhow::Error> {
        let keypair = libp2p::identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());
        
        let mut swarm = crate::network::build_swarm(keypair, &config)?;
        
        // Create gossipsub topic from room name
        let topic = gossipsub::IdentTopic::new(format!("chat-room:{}", room_name));
        
        // Subscribe to topic
        swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
        
        // Listen on addresses
        for addr in &config.listen_addresses {
            swarm.listen_on(addr.clone())?;
        }
        
        // Bootstrap
        for bootstrap_node in &config.bootstrap_nodes {
            swarm.dial(bootstrap_node.clone())?;
        }
        
        Ok(Self {
            swarm,
            topic,
            peers: Vec::new(),
        })
    }
    
    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => {
                    self.handle_event(event).await?;
                }
                
                // Handle user input
                message = tokio::io::stdin().read_line() => {
                    if let Ok(message) = message {
                        self.send_message(message.trim()).await?;
                    }
                }
            }
        }
    }
    
    async fn handle_event(&mut self, event: SwarmEvent<NetworkEvent>) -> Result<(), anyhow::Error> {
        match event {
            SwarmEvent::Behaviour(NetworkBehaviourEvent::Gossipsub(
                gossipsub::Event::Message { message, .. }
            )) => {
                // Received chat message
                let msg: ChatMessage = serde_json::from_slice(&message.data)?;
                println!("[{}]: {}", msg.username, msg.content);
            }
            
            SwarmEvent::Behaviour(NetworkBehaviourEvent::Gossipsub(
                gossipsub::Event::Subscribed { peer_id, .. }
            )) => {
                info!("Peer subscribed: {}", peer_id);
                if !self.peers.contains(&peer_id) {
                    self.peers.push(peer_id);
                }
            }
            
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on: {}", address);
            }
            
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                info!("Connected to: {}", peer_id);
            }
            
            _ => {}
        }
        
        Ok(())
    }
    
    pub async fn send_message(&mut self, content: &str) -> Result<(), anyhow::Error> {
        let message = ChatMessage {
            username: "user".to_string(),
            content: content.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        let data = serde_json::to_vec(&message)?;
        self.swarm.behaviour_mut().gossipsub.publish(self.topic.clone(), data)?;
        
        Ok(())
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    pub username: String,
    pub content: String,
    pub timestamp: u64,
}
```

### File Sharing Implementation

```rust
// src/app/file_share.rs

use bytes::Bytes;
use futures::{AsyncReadExt, AsyncWriteExt};
use libp2p::{PeerId, Stream, Swarm, SwarmEvent};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, error};

#[derive(Debug, Serialize, Deserialize)]
pub enum FileShareMessage {
    ListFiles,
    FileList(Vec<FileInfo>),
    RequestFile { hash: String },
    FileStart { hash: String, name: String, size: u64 },
    FileData { chunk: Vec<u8> },
    FileComplete,
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub hash: String,
    pub name: String,
    pub size: u64,
}

pub struct P2PFileShare {
    swarm: Swarm<NetworkBehaviour>,
    shared_files: HashMap<String, FileInfo>,
    shared_dir: PathBuf,
    download_dir: PathBuf,
}

impl P2PFileShare {
    pub async fn new(config: NetworkConfig) -> Result<Self, anyhow::Error> {
        let keypair = libp2p::identity::Keypair::generate_ed25519();
        let mut swarm = crate::network::build_swarm(keypair, &config)?;
        
        let shared_dir = std::env::current_dir()?.join("shared");
        let download_dir = std::env::current_dir()?.join("downloads");
        
        tokio::fs::create_dir_all(&shared_dir).await?;
        tokio::fs::create_dir_all(&download_dir).await?;
        
        let mut this = Self {
            swarm,
            shared_files: HashMap::new(),
            shared_dir,
            download_dir,
        };
        
        // Index shared files
        this.index_shared_files().await?;
        
        Ok(this)
    }
    
    async fn index_shared_files(&mut self) -> Result<(), anyhow::Error> {
        let mut entries = tokio::fs::read_dir(&self.shared_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let metadata = entry.metadata().await?;
            
            if metadata.is_file() {
                let content = tokio::fs::read(&path).await?;
                let hash = format!("{:x}", md5::compute(&content));
                
                self.shared_files.insert(hash.clone(), FileInfo {
                    hash,
                    name: entry.file_name().to_string_lossy().to_string(),
                    size: metadata.len(),
                });
            }
        }
        
        Ok(())
    }
    
    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => {
                    self.handle_event(event).await?;
                }
            }
        }
    }
    
    async fn handle_event(&mut self, event: SwarmEvent<NetworkEvent>) -> Result<(), anyhow::Error> {
        // Handle connection events and protocol streams
        Ok(())
    }
    
    async fn handle_incoming_stream(&mut self, stream: Stream) -> Result<(), anyhow::Error> {
        let (mut reader, mut writer) = stream.split();
        
        // Read message
        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        
        let mut data = vec![0u8; len];
        reader.read_exact(&mut data).await?;
        
        let message: FileShareMessage = serde_json::from_slice(&data)?;
        
        match message {
            FileShareMessage::ListFiles => {
                // Send file list
                let files: Vec<FileInfo> = self.shared_files.values().cloned().collect();
                self.send_message(&mut writer, &FileShareMessage::FileList(files)).await?;
            }
            
            FileShareMessage::RequestFile { hash } => {
                // Send requested file
                if let Some(file_info) = self.shared_files.get(&hash) {
                    self.send_file(&mut writer, file_info).await?;
                } else {
                    self.send_message(&mut writer, &FileShareMessage::Error {
                        message: "File not found".to_string(),
                    }).await?;
                }
            }
            
            _ => {}
        }
        
        Ok(())
    }
    
    async fn send_file(
        &self,
        writer: &mut futures::io::WriteHalf<'_, Stream>,
        file_info: &FileInfo,
    ) -> Result<(), anyhow::Error> {
        // Send file header
        self.send_message(writer, &FileShareMessage::FileStart {
            hash: file_info.hash.clone(),
            name: file_info.name.clone(),
            size: file_info.size,
        }).await?;
        
        // Stream file content
        let file_path = self.shared_dir.join(&file_info.name);
        let mut file = File::open(&file_path).await?;
        
        let mut buffer = vec![0u8; 8192];
        loop {
            let n = file.read(&mut buffer).await?;
            if n == 0 {
                break;
            }
            
            self.send_message(writer, &FileShareMessage::FileData {
                chunk: buffer[..n].to_vec(),
            }).await?;
        }
        
        self.send_message(writer, &FileShareMessage::FileComplete).await?;
        
        Ok(())
    }
    
    async fn send_message(
        &self,
        writer: &mut futures::io::WriteHalf<'_, Stream>,
        message: &FileShareMessage,
    ) -> Result<(), anyhow::Error> {
        let data = serde_json::to_vec(message)?;
        let len = data.len() as u32;
        
        writer.write_all(&len.to_be_bytes()).await?;
        writer.write_all(&data).await?;
        writer.flush().await?;
        
        Ok(())
    }
}
```

## Conclusion

Rust libp2p provides:

1. **Type Safety**: Compile-time protocol verification
2. **Performance**: Zero-cost abstractions, async runtime
3. **Modularity**: Mix and match protocols
4. **Security**: Built-in encryption and authentication
5. **Production Ready**: Used by IPFS, Polkadot, Ethereum
