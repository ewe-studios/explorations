---
source: /home/darkvoid/Boxxed/@formulas/src.superfly
repository: github.com/superfly (flyctl, corrosion, libkrunfw, metrics, tokenizer)
explored_at: 2026-04-05
focus: Rust implementation of Fly.io platform patterns - microVMs, service discovery, edge deployment, observability
---

# Rust Revision: Fly.io Platform in Rust

## Overview

This document translates Fly.io's platform patterns from Go to Rust, covering microVM management, distributed service discovery, edge deployment orchestration, and observability. We'll examine how to build a Fly.io-like platform using Rust's safety and performance guarantees.

## Architecture Comparison

### Go (Original Fly.io Stack)

```
Fly.io Platform (Go)
├── flyctl (CLI tool)
│   ├── Cobra (command framework)
│   ├── Scanner (auto-configuration)
│   └── Machines API client
├── corrosion (service discovery)
│   ├── Gossip protocol
│   ├── SQLite CRDT storage
│   └── Service registration
├── libkrunfw (microVM creation)
│   ├── Firecracker integration
│   ├── Kernel bundling
│   └── Rootfs management
├── metrics (observability)
│   ├── metrics-rs (Rust metrics)
│   ├── Prometheus export
│   └── Regional aggregation
└── tokenizer (auth proxy)
    ├── Macaroon tokens
    ├── Credential injection
    └── Token validation
```

### Rust (Revision)

```
Edge Platform (Rust)
├── edge-cli (CLI tool)
│   ├── Clap (command framework)
│   ├── Auto-detection scanners
│   └── Machines API (bollard + custom)
├── service-mesh (service discovery)
│   ├── Gossip protocol (tokio + UDP)
│   ├── SQLite CRDT storage (rusqlite)
│   └── Service registration API (axum)
├── microvm-manager (hypervisor)
│   ├── Firecracker client (async)
│   ├── Kernel bundling (embedded)
│   └── Rootfs creation
├── observability (metrics)
│   ├── metrics-rs (native Rust)
│   ├── Prometheus exporter
│   └── Distributed aggregation
└── auth-proxy (authentication)
    ├── Macaroon implementation
    ├── Credential injection
    └── Token validation
```

## Core Data Structures

```rust
// src/types.rs

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub app_name: String,
    pub primary_region: String,
    pub regions: Vec<String>,
    pub build: BuildConfig,
    pub services: Vec<ServiceConfig>,
    pub mounts: Vec<MountConfig>,
    pub env: HashMap<String, String>,
    pub secrets: Vec<String>,
    pub vm: VmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    pub dockerfile: Option<String>,
    pub builder: Option<String>,
    pub build_args: HashMap<String, String>,
    pub image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub internal_port: u16,
    pub protocol: String,  // "tcp" or "udp"
    pub ports: Vec<PortConfig>,
    pub checks: Vec<HealthCheckConfig>,
    pub concurrency: Option<ConcurrencyConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortConfig {
    pub port: u16,
    pub handlers: Vec<String>,  // "http", "tls", "proxy"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub path: String,
    pub interval_secs: u32,
    pub timeout_secs: u32,
    pub grace_period_secs: u32,
    pub method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountConfig {
    pub source: String,
    pub destination: String,
    pub initial_size_gb: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    pub cpu_kind: String,  // "shared" or "performance"
    pub cpus: u8,
    pub memory_mb: u16,
}

/// Machine state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MachineState {
    Created,
    Starting,
    Running,
    Stopping,
    Stopped,
    Destroyed,
}

/// Machine representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Machine {
    pub id: String,
    pub name: String,
    pub state: MachineState,
    pub region: String,
    pub config: MachineConfig,
    pub image_ref: String,
    pub private_ip: String,
    pub public_ip: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineConfig {
    pub init: Option<InitConfig>,
    pub image: String,
    pub restart: RestartPolicy,
    pub env: HashMap<String, String>,
    pub services: Vec<ServiceConfig>,
    pub checks: HashMap<String, HealthCheckConfig>,
    pub mounts: Vec<MountConfig>,
    pub guest: GuestConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitConfig {
    pub cmd: Option<Vec<String>>,
    pub entrypoint: Option<Vec<String>>,
    pub exec: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartPolicy {
    pub policy: String,  // "no", "on-failure", "always", "unless-stopped"
    pub max_retries: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestConfig {
    pub cpu_kind: String,
    pub cpus: u8,
    pub memory_mb: u16,
}

/// Service instance for discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInstance {
    pub instance_id: String,
    pub machine_id: String,
    pub service_name: String,
    pub address: String,
    pub port: u16,
    pub region: String,
    pub tags: Vec<String>,
    pub healthy: bool,
    pub last_seen: DateTime<Utc>,
}

/// CRDT operation for gossip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtOperation {
    pub operation_id: String,
    pub node_id: String,
    pub timestamp: u64,
    pub operation_type: String,  // "insert", "update", "delete"
    pub table_name: String,
    pub record_key: String,
    pub data: serde_json::Value,
    pub vector_clock: HashMap<String, u64>,
}
```

## CLI Implementation (flyctl in Rust)

```rust
// src/cli.rs

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "edge")]
#[command(about = "Edge platform CLI", long_about = None)]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    /// API access token
    #[arg(long, env = "EDGE_API_TOKEN")]
    access_token: Option<String>,
    
    /// Configuration file path
    #[arg(long, short = 'c', default_value = "edge.toml")]
    config: PathBuf,
    
    /// Enable debug logging
    #[arg(long)]
    debug: bool,
    
    /// Default region
    #[arg(long, default_value = "iad")]
    region: String,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with the platform
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
    
    /// Launch a new application
    Launch {
        /// App name
        #[arg(long)]
        name: Option<String>,
        
        /// Generate name
        #[arg(long)]
        generate_name: bool,
        
        /// Deploy from image
        #[arg(long)]
        image: Option<String>,
    },
    
    /// Deploy application
    Deploy {
        /// App directory
        #[arg(long, default_value = ".")]
        path: PathBuf,
        
        /// Image to deploy
        #[arg(long)]
        image: Option<String>,
        
        /// Deployment strategy
        #[arg(long, default_value = "rolling")]
        strategy: String,
    },
    
    /// Manage machines
    Machines {
        #[command(subcommand)]
        action: MachineAction,
    },
    
    /// Manage regions
    Regions {
        #[command(subcommand)]
        action: RegionAction,
    },
    
    /// Manage secrets
    Secrets {
        #[command(subcommand)]
        action: SecretsAction,
    },
    
    /// Manage volumes
    Volumes {
        #[command(subcommand)]
        action: VolumeAction,
    },
    
    /// Stream application logs
    Logs {
        /// App name
        #[arg(long)]
        app: Option<String>,
        
        /// Follow logs
        #[arg(short = 'f', long)]
        follow: bool,
        
        /// Number of lines
        #[arg(long, short = 'n', default_value = "100")]
        num: u32,
    },
    
    /// Show application status
    Status {
        /// App name
        #[arg(long)]
        app: Option<String>,
    },
    
    /// Open application in browser
    Open {
        /// App name
        #[arg(long)]
        app: Option<String>,
    },
}

#[derive(Subcommand)]
enum AuthAction {
    Login,
    Logout,
    Whoami,
    Token,
}

#[derive(Subcommand)]
enum MachineAction {
    List {
        #[arg(long)]
        app: Option<String>,
    },
    Status {
        #[arg()]
        machine_id: String,
    },
    Restart {
        #[arg()]
        machine_id: String,
    },
    Stop {
        #[arg()]
        machine_id: String,
        
        #[arg(long)]
        signal: Option<String>,
    },
    Destroy {
        #[arg()]
        machine_id: String,
        
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum RegionAction {
    List,
    Set {
        #[arg()]
        region: String,
    },
    Add {
        #[arg()]
        regions: Vec<String>,
    },
}

#[derive(Subcommand)]
enum SecretsAction {
    List {
        #[arg(long)]
        app: Option<String>,
    },
    Set {
        #[arg(long)]
        app: Option<String>,
        
        #[arg()]
        secrets: Vec<String>,  // KEY=VALUE format
    },
    Unset {
        #[arg(long)]
        app: Option<String>,
        
        #[arg()]
        names: Vec<String>,
    },
}

#[derive(Subcommand)]
enum VolumeAction {
    List {
        #[arg(long)]
        app: Option<String>,
    },
    Create {
        #[arg()]
        name: String,
        
        #[arg(long)]
        region: String,
        
        #[arg(long)]
        size: u32,
    },
    Extend {
        #[arg()]
        name: String,
        
        #[arg(long)]
        size: u32,
    },
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    // Initialize logging
    if cli.debug {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::init();
    
    // Load or create config
    let config = load_config(&cli.config)?;
    
    match cli.command {
        Commands::Auth { action } => cmd_auth(action, config).await?,
        Commands::Launch { name, generate_name, image } => {
            cmd_launch(name, generate_name, image, config).await?
        }
        Commands::Deploy { path, image, strategy } => {
            cmd_deploy(path, image, &strategy, config).await?
        }
        Commands::Machines { action } => cmd_machines(action, config).await?,
        Commands::Regions { action } => cmd_regions(action, config).await?,
        Commands::Secrets { action } => cmd_secrets(action, config).await?,
        Commands::Volumes { action } => cmd_volumes(action, config).await?,
        Commands::Logs { app, follow, num } => {
            cmd_logs(app, follow, num, config).await?
        }
        Commands::Status { app } => cmd_status(app, config).await?,
        Commands::Open { app } => cmd_open(app, config).await?,
    }
    
    Ok(())
}

async fn cmd_launch(
    name: Option<String>,
    generate_name: bool,
    image: Option<String>,
    config: Config,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::scanner::scan_directory;
    
    println!("Scanning current directory...");
    
    // Scan directory for project type
    let source_info = scan_directory(std::env::current_dir()?)?;
    
    println!("Detected: {} project", source_info.family);
    
    // Generate or use provided app name
    let app_name = if generate_name {
        generate_app_name()?
    } else if let Some(name) = name {
        name
    } else {
        // Prompt for name or generate
        generate_app_name()?
    };
    
    // Create app
    let client = crate::api::Client::new(&config.api_token)?;
    let app = client.create_app(&app_name, &config.default_region).await?;
    
    println!("Created app: {}", app.name);
    
    // Generate config
    let mut app_config = generate_app_config(&app_name, &source_info);
    
    if let Some(img) = image {
        app_config.build.image = Some(img);
    }
    
    // Write config file
    let config_content = toml::to_string_pretty(&app_config)?;
    std::fs::write("edge.toml", &config_content)?;
    
    println!("Generated edge.toml configuration");
    
    // Ask if user wants to deploy now
    println!("\nDeploy now? (y/n): ");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    
    if input.trim().to_lowercase() == "y" {
        cmd_deploy(
            std::env::current_dir()?,
            image,
            "rolling",
            config,
        ).await?;
    }
    
    Ok(())
}

fn generate_app_config(name: &str, source_info: &crate::scanner::SourceInfo) -> crate::types::AppConfig {
    crate::types::AppConfig {
        app_name: name.to_string(),
        primary_region: "iad".to_string(),
        regions: vec!["iad".to_string()],
        build: crate::types::BuildConfig {
            dockerfile: source_info.dockerfile.clone(),
            builder: source_info.builder.clone(),
            build_args: source_info.build_args.clone(),
            image: None,
        },
        services: vec![crate::types::ServiceConfig {
            internal_port: source_info.port,
            protocol: "tcp".to_string(),
            ports: vec![crate::types::PortConfig {
                port: 80,
                handlers: vec!["http".to_string()],
            }, crate::types::PortConfig {
                port: 443,
                handlers: vec!["tls".to_string(), "http".to_string()],
            }],
            checks: vec![crate::types::HealthCheckConfig {
                path: source_info.http_check_path.clone().unwrap_or("/health".to_string()),
                interval_secs: 15,
                timeout_secs: 5,
                grace_period_secs: 10,
                method: "GET".to_string(),
            }],
            concurrency: None,
        }],
        mounts: source_info.volumes.iter().map(|v| {
            crate::types::MountConfig {
                source: v.source.clone(),
                destination: v.destination.clone(),
                initial_size_gb: 10,
            }
        }).collect(),
        env: source_info.env.clone(),
        secrets: source_info.secrets.clone(),
        vm: crate::types::VmConfig {
            cpu_kind: "shared".to_string(),
            cpus: 1,
            memory_mb: 512,
        },
    }
}
```

## Service Discovery (Corrosion in Rust)

```rust
// src/service_discovery.rs

use std::sync::Arc;
use tokio::sync::RwLock;
use rusqlite::{Connection, params};
use tokio::time::{interval, Duration};

pub struct ServiceDiscovery {
    db: Arc<RwLock<Connection>>,
    node_id: String,
    gossip: Arc<GossipProtocol>,
}

impl ServiceDiscovery {
    pub async fn new(node_id: String, db_path: &str) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(db_path)?;
        
        // Initialize schema
        Self::init_schema(&conn)?;
        
        let db = Arc::new(RwLock::new(conn));
        
        let gossip = Arc::new(GossipProtocol::new(
            node_id.clone(),
            db.clone(),
        ));
        
        Ok(Self {
            db,
            node_id,
            gossip,
        })
    }
    
    fn init_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
        conn.execute_batch(include_str!("schema.sql"))?;
        Ok(())
    }
    
    /// Register a service instance
    pub async fn register_service(
        &self,
        service_name: &str,
        instance_id: &str,
        address: &str,
        port: u16,
        tags: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.db.write().await;
        
        let tags_json = serde_json::to_string(&tags)?;
        
        conn.execute(
            "INSERT OR REPLACE INTO service_instances 
             (service_name, instance_id, node_id, address, port, tags, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, unixepoch())",
            params![
                service_name,
                instance_id,
                self.node_id,
                address,
                port,
                tags_json,
            ],
        )?;
        
        // Log CRDT operation for gossip
        self.log_crdt_operation(
            &conn,
            "insert",
            "service_instances",
            &format!("{}.{}", service_name, instance_id),
            &serde_json::json!({
                "service_name": service_name,
                "instance_id": instance_id,
                "address": address,
                "port": port,
                "tags": tags,
            }),
        )?;
        
        Ok(())
    }
    
    /// Deregister a service instance
    pub async fn deregister_service(
        &self,
        service_name: &str,
        instance_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.db.write().await;
        
        conn.execute(
            "DELETE FROM service_instances 
             WHERE service_name = ?1 AND instance_id = ?2 AND node_id = ?3",
            params![service_name, instance_id, self.node_id],
        )?;
        
        // Log CRDT operation for gossip
        self.log_crdt_operation(
            &conn,
            "delete",
            "service_instances",
            &format!("{}.{}", service_name, instance_id),
            &serde_json::json!({
                "service_name": service_name,
                "instance_id": instance_id,
            }),
        )?;
        
        Ok(())
    }
    
    /// Query service instances
    pub async fn query_service(
        &self,
        service_name: &str,
    ) -> Result<Vec<crate::types::ServiceInstance>, Box<dyn std::error::Error>> {
        let conn = self.db.read().await;
        
        let mut stmt = conn.prepare(
            "SELECT instance_id, node_id, address, port, tags, region
             FROM service_instances
             WHERE service_name = ?1
             ORDER BY region, node_id"
        )?;
        
        let rows = stmt.query_map(params![service_name], |row| {
            let tags_json: String = row.get(4)?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            
            Ok(crate::types::ServiceInstance {
                instance_id: row.get(0)?,
                machine_id: row.get(1)?,
                service_name: service_name.to_string(),
                address: row.get(2)?,
                port: row.get(3)?,
                region: row.get(5).unwrap_or_else(|_| "unknown".to_string()),
                tags,
                healthy: true,
                last_seen: chrono::Utc::now(),
            })
        })?;
        
        let mut instances = Vec::new();
        for row in rows {
            instances.push(row?);
        }
        
        Ok(instances)
    }
    
    /// Start the gossip protocol
    pub async fn run_gossip(&self, bind_addr: std::net::SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        // Start gossip protocol in background
        let gossip = self.gossip.clone();
        tokio::spawn(async move {
            gossip.run(bind_addr).await
        });
        
        Ok(())
    }
    
    fn log_crdt_operation(
        &self,
        conn: &Connection,
        op_type: &str,
        table: &str,
        key: &str,
        data: &serde_json::Value,
    ) -> Result<(), rusqlite::Error> {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        let operation_id = format!("{}-{}-{}", self.node_id, table, timestamp);
        
        conn.execute(
            "INSERT INTO crdt_operations 
             (operation_id, node_id, timestamp, operation_type, table_name, record_key, data, processed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, FALSE)",
            params![operation_id, self.node_id, timestamp, op_type, table, key, data],
        )?;
        
        Ok(())
    }
}

/// Gossip protocol implementation
pub struct GossipProtocol {
    node_id: String,
    db: Arc<RwLock<Connection>>,
    peers: Arc<RwLock<Vec<PeerInfo>>>,
}

#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub node_id: String,
    pub address: String,
    pub port: u16,
    pub last_contact: std::time::SystemTime,
}

impl GossipProtocol {
    pub fn new(node_id: String, db: Arc<RwLock<Connection>>) -> Self {
        Self {
            node_id,
            db,
            peers: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub async fn run(&self, bind_addr: std::net::SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        // Bind UDP socket for gossip
        let socket = tokio::net::UdpSocket::bind(bind_addr).await?;
        
        let mut gossip_interval = interval(Duration::from_secs(1));
        
        loop {
            gossip_interval.tick().await;
            
            // Send heartbeat
            self.send_heartbeat().await?;
            
            // Gossip round
            self.gossip_round(&socket).await?;
        }
    }
    
    async fn send_heartbeat(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Update local node heartbeat in database
        let conn = self.db.read().await;
        
        conn.execute(
            "INSERT OR REPLACE INTO nodes (node_id, address, port, region, last_heartbeat)
             VALUES (?1, ?2, ?3, ?4, unixepoch())",
            params![self.node_id, "127.0.0.1", bind_addr.port(), "local"],
        )?;
        
        Ok(())
    }
    
    async fn gossip_round(&self, socket: &tokio::net::UdpSocket) -> Result<(), Box<dyn std::error::Error>> {
        // Get unprocessed operations
        let conn = self.db.read().await;
        
        let mut stmt = conn.prepare(
            "SELECT operation_id, node_id, timestamp, operation_type, table_name, record_key, data
             FROM crdt_operations
             WHERE NOT processed
             ORDER BY timestamp ASC
             LIMIT 100"
        )?;
        
        let ops: Vec<crate::types::CrdtOperation> = stmt
            .query_map([], |row| {
                Ok(crate::types::CrdtOperation {
                    operation_id: row.get(0)?,
                    node_id: row.get(1)?,
                    timestamp: row.get(2)?,
                    operation_type: row.get(3)?,
                    table_name: row.get(4)?,
                    record_key: row.get(5)?,
                    data: row.get(6)?,
                    vector_clock: HashMap::new(),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        
        if ops.is_empty() {
            return Ok(());
        }
        
        // Get peers
        let peers = self.get_peers().await?;
        
        // Send to random peers
        let serialized = serde_json::to_vec(&ops)?;
        
        for peer in peers.iter().take(3) {
            let addr = format!("{}:{}", peer.address, peer.port);
            socket.send_to(&serialized, &addr).await?;
        }
        
        // Mark operations as processed
        for op in &ops {
            conn.execute(
                "UPDATE crdt_operations SET processed = TRUE WHERE operation_id = ?1",
                params![op.operation_id],
            )?;
        }
        
        Ok(())
    }
    
    async fn get_peers(&self) -> Result<Vec<PeerInfo>, rusqlite::Error> {
        let conn = self.db.read().await;
        
        let threshold = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() - 30) as i64;
        
        let mut stmt = conn.prepare(
            "SELECT node_id, address, port FROM nodes WHERE last_heartbeat > ?1"
        )?;
        
        let rows = stmt.query_map(params![threshold], |row| {
            Ok(PeerInfo {
                node_id: row.get(0)?,
                address: row.get(1)?,
                port: row.get(2)?,
                last_contact: std::time::SystemTime::now(),
            })
        })?;
        
        let mut peers = Vec::new();
        for row in rows {
            if let Ok(peer) = row {
                if peer.node_id != self.node_id {
                    peers.push(peer);
                }
            }
        }
        
        Ok(peers)
    }
}
```

## Conclusion

The Rust implementation of Fly.io patterns provides:

1. **Type Safety**: Compile-time checking of configurations and states
2. **Async Runtime**: Non-blocking operations via tokio
3. **Memory Safety**: No segfaults from buffer overflows or use-after-free
4. **CRDT Implementation**: Conflict-free replicated data types with SQLite
5. **CLI Ergonomics**: clap for modern command-line interfaces
6. **Performance**: Zero-cost abstractions and minimal runtime overhead

Key differences from Go:
- Rust's ownership model eliminates data races at compile time
- tokio provides structured concurrency vs Go's goroutines
- rusqlite offers type-safe SQLite bindings
- clap derives provide automatic CLI help and validation
- serde ensures safe serialization/deserialization
