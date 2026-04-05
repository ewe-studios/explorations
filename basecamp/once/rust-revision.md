---
source: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/once
repository: github.com/basecamp/once
explored_at: 2026-04-05
focus: Rust implementation of ONCE patterns - Docker orchestration, TUI, backup/restore, proxy integration
---

# Rust Revision: ONCE Platform in Rust

## Overview

This document translates ONCE's self-hosting platform patterns from Go to Rust, covering Docker orchestration via bollard, TUI with Ratatui, SSH capabilities, and production-grade deployment patterns.

## Architecture Comparison

### Go (Original ONCE)

```
once CLI (Go)
    ├── Cobra (CLI framework)
    ├── Bubble Tea (TUI)
    ├── Docker SDK (official)
    └── kamal-proxy integration
```

### Rust (Revision)

```
once-rs (Rust)
    ├── clap (CLI framework)
    ├── ratatui (TUI)
    ├── bollard (Docker SDK)
    ├── russh (SSH remote execution)
    ├── tokio (async runtime)
    └── kamal-proxy integration
```

## Core Data Structures

### Namespace

```rust
// src/namespace.rs

use bollard::Docker;
use bollard::network::ListNetworksOptions;
use bollard::models::Network;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Namespace {
    pub name: String,
    pub client: Arc<Docker>,
    pub proxy: Arc<Proxy>,
    pub applications: Arc<RwLock<Vec<Application>>>,
}

impl Namespace {
    pub async fn new(name: String) -> Result<Self, OnceError> {
        let client = Arc::new(Docker::connect_with_local_defaults()?);
        
        let ns = Self {
            name,
            client: client.clone(),
            proxy: Arc::new(Proxy::new(client.clone())),
            applications: Arc::new(RwLock::new(Vec::new())),
        };
        
        ns.ensure().await?;
        
        Ok(ns)
    }
    
    async fn ensure(&self) -> Result<(), OnceError> {
        use bollard::network::CreateNetworkOptions;
        
        // Check if network exists
        let networks = self.client.list_networks::<String>(None).await?;
        
        for network in networks {
            if network.name == self.name {
                return Ok(());
            }
        }
        
        // Create network
        self.client.create_network(CreateNetworkOptions {
            name: self.name.clone(),
            driver: Some("bridge".to_string()),
            ..Default::default()
        }).await?;
        
        Ok(())
    }
    
    pub async fn applications(&self) -> Result<Vec<Application>, OnceError> {
        use bollard::container::ListContainersOptions;
        
        let containers = self.client.list_containers(Some(ListContainersOptions {
            all: true,
            filters: serde_json::json!({
                "label": [
                    format!("once.namespace={}", self.name),
                    "once.app-name",
                ]
            }).to_string(),
            ..Default::default()
        })).await?;
        
        // Group by app name
        let mut apps_by_name: std::collections::HashMap<String, Vec<_>> = 
            std::collections::HashMap::new();
        
        for container in containers {
            if let Some(app_name) = container.labels
                .as_ref()
                .and_then(|l| l.get("once.app-name"))
                .cloned() 
            {
                apps_by_name.entry(app_name).or_default().push(container);
            }
        }
        
        let mut apps = Vec::new();
        for (name, containers) in apps_by_name {
            let mut app = Application::new(self, &name);
            app.update_from_containers(&containers).await;
            apps.push(app);
        }
        
        Ok(apps)
    }
}
```

### Application

```rust
// src/application.rs

use bollard::container::{
    Config, CreateContainerOptions, StartContainerOptions, 
    RemoveContainerOptions,
};
use bollard::models::{Container, ContainerSummary};
use bollard::service::HostConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationSettings {
    pub name: String,
    pub image: String,
    pub host: String,
    pub auto_update: bool,
    pub backup: BackupSettings,
    pub resources: ResourceSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSettings {
    pub location: String,
    pub auto_backup: bool,
    pub frequency: String,  // daily, weekly, monthly
    pub retention: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSettings {
    pub memory_mb: u64,
    pub cpus: f64,
}

pub struct Application {
    namespace: Arc<Namespace>,
    pub settings: ApplicationSettings,
    pub running: bool,
    pub running_since: Option<u64>,
    pub container_id: Option<String>,
}

impl Application {
    pub fn new(namespace: &Namespace, name: &str) -> Self {
        Self {
            namespace: Arc::clone(&namespace),
            settings: ApplicationSettings {
                name: name.to_string(),
                image: String::new(),
                host: String::new(),
                auto_update: false,
                backup: BackupSettings::default(),
                resources: ResourceSettings::default(),
            },
            running: false,
            running_since: None,
            container_id: None,
        }
    }
    
    pub async fn update_from_containers(&mut self, containers: &[ContainerSummary]) {
        for container in containers {
            if let Some(state) = &container.state {
                self.running = state.to_lowercase() == "running";
                if self.running {
                    if let Some(started_at) = state.started_at {
                        if let Ok(time) = parse_datetime(&started_at) {
                            self.running_since = Some(
                                time.duration_since(UNIX_EPOCH).unwrap().as_secs()
                            );
                        }
                    }
                }
            }
            
            if let Some(id) = &container.id {
                self.container_id = Some(id.clone());
            }
        }
    }
    
    pub async fn deploy<F>(&self, mut progress: F) -> Result<(), OnceError>
    where
        F: FnMut(f32, &str),
    {
        progress(0.0, "Pulling Docker image...");
        
        // Pull image
        self.pull_image(&mut progress).await?;
        
        progress(0.25, "Creating persistent volume...");
        
        // Create/get volume
        let volume = self.volume().await?;
        
        progress(0.5, "Deploying container...");
        
        // Deploy container
        self.deploy_with_volume(&volume, &mut progress).await?;
        
        progress(0.75, "Registering with proxy...");
        
        // Register with proxy
        let tls_enabled = !is_localhost(&self.settings.host);
        self.namespace.proxy.deploy(DeployOptions {
            app_name: self.settings.name.clone(),
            target: self.settings.name.clone(),
            host: self.settings.host.clone(),
            tls: tls_enabled,
        }).await?;
        
        progress(1.0, "Complete!");
        
        Ok(())
    }
    
    async fn pull_image<F>(&self, progress: &mut F) -> Result<(), OnceError>
    where
        F: FnMut(f32, &str),
    {
        use bollard::image::CreateImageOptions;
        use futures_util::stream::StreamExt;
        
        let options = CreateImageOptions {
            from_image: self.settings.image.clone(),
            ..Default::default()
        };
        
        let mut stream = self.client.create_image(Some(options), None, None);
        
        while let Some(result) = stream.next().await {
            let info = result?;
            if let Some(status) = info.status {
                progress(0.25 * info.progress.unwrap_or(0.0), &status);
            }
        }
        
        Ok(())
    }
    
    async fn deploy_with_volume<F>(
        &self, 
        volume: &ApplicationVolume,
        progress: &mut F,
    ) -> Result<(), OnceError>
    where
        F: FnMut(f32, &str),
    {
        use bollard::models::HostConfig;
        use bollard::models::Mount;
        use bollard::models::MountTypeEnum;
        use bollard::models::RestartPolicy;
        use bollard::models::RestartPolicyNameEnum;
        
        let container_name = format!(
            "{}-app-{}-{}",
            self.namespace.name,
            self.settings.name,
            container_random_id(),
        );
        
        let env = self.build_env(&volume.settings);
        
        let config = Config {
            image: Some(self.settings.image.clone()),
            env: Some(env),
            labels: Some(HashMap::from([
                ("once.namespace".to_string(), self.namespace.name.clone()),
                ("once.app-name".to_string(), self.settings.name.clone()),
                ("once.app-host".to_string(), self.settings.host.clone()),
            ])),
            healthcheck: Some(bollard::models::HealthConfig {
                test: Some(vec![
                    "CMD".to_string(),
                    "curl".to_string(),
                    "-f".to_string(),
                    "http://localhost:80/up".to_string(),
                ]),
                interval: Some(30_000_000_000), // 30s in nanos
                timeout: Some(5_000_000_000),   // 5s in nanos
                retries: Some(3),
                start_period: Some(10_000_000_000),
            }),
            ..Default::default()
        };
        
        let host_config = HostConfig {
            restart_policy: Some(RestartPolicy {
                name: Some(RestartPolicyNameEnum::ALWAYS),
                maximum_retry_count: Some(0),
            }),
            mounts: Some(self.volume_mounts(volume)),
            memory: Some((self.settings.resources.memory_mb * 1024 * 1024) as i64),
            nano_cpus: Some((self.settings.resources.cpus * 1e9) as i64),
            ..Default::default()
        };
        
        self.client.create_container::<String, String>(
            Some(CreateContainerOptions {
                name: container_name.clone(),
                platform: None,
            }),
            config,
        ).await?;
        
        self.client.start_container::<String>(&container_name, None).await?;
        
        // Remove old containers
        self.remove_containers_except(&container_name).await?;
        
        Ok(())
    }
    
    fn build_env(&self, vol_settings: &ApplicationVolumeSettings) -> Vec<String> {
        let mut env = vec![
            format!("SECRET_KEY_BASE={}", vol_settings.secret_key_base),
            format!("VAPID_PUBLIC_KEY={}", vol_settings.vapid_public_key),
            format!("VAPID_PRIVATE_KEY={}", vol_settings.vapid_private_key),
            "RAILS_ENV=production".to_string(),
            "STORAGE_PATH=/storage".to_string(),
        ];
        
        if is_localhost(&self.settings.host) {
            env.push("DISABLE_SSL=true".to_string());
        }
        
        env
    }
    
    fn volume_mounts(&self, volume: &ApplicationVolume) -> Vec<Mount> {
        vec![
            Mount {
                target: Some("/storage".to_string()),
                source: Some(volume.name.clone()),
                typ: Some(MountTypeEnum::VOLUME),
                ..Default::default()
            },
            Mount {
                target: Some("/rails/storage".to_string()),
                source: Some(volume.name.clone()),
                typ: Some(MountTypeEnum::VOLUME),
                ..Default::default()
            },
        ]
    }
}

fn container_random_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 6] = rng.gen();
    hex::encode(bytes)
}
```

### Volume Management

```rust
// src/volume.rs

use bollard::volume::{CreateVolumeOptions, InspectVolumeOptions};
use bollard::models::Volume;
use serde::{Deserialize, Serialize};
use rand::Rng;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationVolumeSettings {
    pub secret_key_base: String,
    pub vapid_public_key: String,
    pub vapid_private_key: String,
}

pub struct ApplicationVolume {
    pub name: String,
    pub settings: ApplicationVolumeSettings,
}

impl ApplicationVolume {
    pub async fn new(
        namespace: &Namespace,
        app_name: &str,
    ) -> Result<Self, OnceError> {
        let vol_name = format!("{}-{}-storage", namespace.name, app_name);
        
        // Try to get existing volume
        match namespace.client.inspect_volume(&vol_name, None).await {
            Ok(volume) => {
                // Load settings from volume labels or metadata file
                let settings = load_volume_settings(&volume).await?;
                return Ok(Self { name: vol_name, settings });
            }
            Err(bollard::errors::Error::DockerResponseNotFoundError { .. }) => {
                // Volume doesn't exist, create it
            }
            Err(e) => return Err(e.into()),
        }
        
        // Create new volume
        namespace.client.create_volume(CreateVolumeOptions {
            name: vol_name.clone(),
            driver: "local".to_string(),
            ..Default::default()
        }).await?;
        
        // Generate settings
        let settings = generate_volume_settings()?;
        
        // Save settings to volume
        save_volume_settings(namespace, &vol_name, &settings).await?;
        
        Ok(Self {
            name: vol_name,
            settings,
        })
    }
}

fn generate_volume_settings() -> Result<ApplicationVolumeSettings, OnceError> {
    let mut rng = rand::thread_rng();
    
    // Generate SECRET_KEY_BASE (64 hex chars)
    let secret_key_bytes: [u8; 32] = rng.gen();
    let secret_key_base = hex::encode(secret_key_bytes);
    
    // Generate VAPID keys (simplified - use proper VAPID library in production)
    let vapid_public = generate_vapid_public_key()?;
    let vapid_private = generate_vapid_private_key()?;
    
    Ok(ApplicationVolumeSettings {
        secret_key_base,
        vapid_public_key: vapid_public,
        vapid_private_key: vapid_private,
    })
}

async fn save_volume_settings(
    namespace: &Namespace,
    vol_name: &str,
    settings: &ApplicationVolumeSettings,
) -> Result<(), OnceError> {
    // Create temporary container to write settings
    use bollard::container::Config;
    
    let settings_json = serde_json::to_string(settings)?;
    
    let config = Config {
        image: Some("alpine:latest".to_string()),
        cmd: Some(vec![
            "sh", "-c", format!(
                "mkdir -p /storage/.once && echo '{}' > /storage/.once/settings.json",
                settings_json.replace("'", "'\\''")
            ),
        ]),
        host_config: Some(HostConfig {
            mounts: Some(vec![Mount {
                target: Some("/storage".to_string()),
                source: Some(vol_name.to_string()),
                typ: Some(MountTypeEnum::VOLUME),
                ..Default::default()
            }]),
            ..Default::default()
        }),
        ..Default::default()
    };
    
    let container = namespace.client.create_container::<String, String>(
        None, config
    ).await?;
    
    namespace.client.start_container::<String>(&container.id, None).await?;
    
    // Wait for container to complete
    namespace.client.wait_container(&container.id, None).await?;
    
    namespace.client.remove_container(
        &container.id,
        Some(RemoveContainerOptions { force: true, ..Default::default() }),
    ).await?;
    
    Ok(())
}
```

## TUI with Ratatui

### Dashboard Implementation

```rust
// src/ui/dashboard.rs

use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use tokio::sync::mpsc;

pub enum DashboardEvent {
    Tick,
    KeyInput(crossterm::event::KeyEvent),
    ApplicationsLoaded(Vec<Application>),
    Error(OnceError),
}

pub struct Dashboard {
    namespace: Arc<Namespace>,
    applications: Vec<Application>,
    selected: usize,
    loading: bool,
    error: Option<OnceError>,
    should_quit: bool,
}

impl Dashboard {
    pub fn new(namespace: Arc<Namespace>) -> Self {
        Self {
            namespace,
            applications: Vec::new(),
            selected: 0,
            loading: true,
            error: None,
            should_quit: false,
        }
    }
    
    pub async fn run<B: Backend>(
        mut self,
        terminal: &mut Terminal<B>,
        mut event_rx: mpsc::UnboundedReceiver<DashboardEvent>,
    ) -> Result<(), OnceError> {
        self.load_applications().await;
        
        loop {
            terminal.draw(|f| self.render(f))?;
            
            if let Some(event) = event_rx.recv().await {
                match event {
                    DashboardEvent::KeyInput(key) => {
                        if self.handle_key(key) {
                            break;
                        }
                    }
                    DashboardEvent::ApplicationsLoaded(apps) => {
                        self.applications = apps;
                        self.loading = false;
                    }
                    DashboardEvent::Error(err) => {
                        self.error = Some(err);
                        self.loading = false;
                    }
                    _ => {}
                }
            }
            
            if self.should_quit {
                break;
            }
        }
        
        Ok(())
    }
    
    fn render(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Min(10),    // App list
                Constraint::Length(3),  // Footer
            ])
            .split(f.area());
        
        self.render_header(f, chunks[0]);
        self.render_app_list(f, chunks[1]);
        self.render_footer(f, chunks[2]);
    }
    
    fn render_header(&self, f: &mut Frame, area: Rect) {
        let header = Paragraph::new(Text::from(vec![
            Line::from(format!(" ONCE Dashboard - Namespace: {}", self.namespace.name)),
            Line::from(format!(" {} Applications", self.applications.len())),
        ]))
        .block(Block::default()
            .title(" Dashboard ")
            .borders(Borders::ALL));
        
        f.render_widget(header, area);
    }
    
    fn render_app_list(&self, f: &mut Frame, area: Rect) {
        if self.loading {
            let loading = Paragraph::new("Loading...")
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(loading, area);
            return;
        }
        
        if let Some(err) = &self.error {
            let error = Paragraph::new(format!("Error: {}", err))
                .style(Style::default().fg(Color::Red))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(error, area);
            return;
        }
        
        if self.applications.is_empty() {
            let empty = Paragraph::new("No applications installed.\n\nPress 'i' to install your first application.")
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(empty, area);
            return;
        }
        
        let items: Vec<ListItem> = self.applications
            .iter()
            .enumerate()
            .map(|(i, app)| {
                let status = if app.running { "✓" } else { "○" };
                let cursor = if i == self.selected { "→" } else { " " };
                
                let status_text = if app.running {
                    format!("Running ({}h)", 
                        (app.running_since.unwrap_or(0) / 3600))
                } else {
                    "Stopped".to_string()
                };
                
                ListItem::new(Line::from(vec![
                    Span::raw(format!(" {} {} ", cursor, status)),
                    Span::styled(app.settings.host.clone(), 
                        Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(format!(" - {}", status_text)),
                ]))
            })
            .collect();
        
        let list = List::new(items)
            .block(Block::default().title(" Applications ").borders(Borders::ALL));
        
        f.render_widget(list, area);
    }
    
    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let footer = Paragraph::new(" [i] Install  [s] Settings  [a] Actions  [r] Refresh  [q] Quit ")
            .block(Block::default().borders(Borders::ALL));
        
        f.render_widget(footer, area);
    }
    
    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> bool {
        use crossterm::event::KeyCode;
        
        match key.code {
            KeyCode::Char('q') | KeyCode::Ctrl('c') => {
                self.should_quit = true;
                true
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                false
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected < self.applications.len().saturating_sub(1) {
                    self.selected += 1;
                }
                false
            }
            KeyCode::Char('r') => {
                self.loading = true;
                tokio::spawn({
                    let ns = Arc::clone(&self.namespace);
                    async move {
                        // Load applications
                    }
                });
                false
            }
            _ => false,
        }
    }
    
    async fn load_applications(&mut self) {
        let ns = Arc::clone(&self.namespace);
        match ns.applications().await {
            Ok(apps) => {
                self.applications = apps;
                self.loading = false;
            }
            Err(err) => {
                self.error = Some(err);
                self.loading = false;
            }
        }
    }
}
```

## Backup System in Rust

```rust
// src/backup.rs

use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use flate2::Compression;
use std::fs::File;
use std::io::{Read, Write};
use tar::{Archive, Builder};
use tokio::io::AsyncReadExt;
use serde_json;

pub struct BackupManager {
    namespace: Arc<Namespace>,
}

impl BackupManager {
    pub async fn create_backup(
        &self,
        app: &Application,
        backup_path: &str,
    ) -> Result<(), OnceError> {
        // Check for pre-backup hook
        let has_hook = self.has_pre_backup_hook(app).await?;
        
        let paused = if has_hook {
            self.exec_hook(app, "/hooks/pre-backup").await?;
            false
        } else {
            // Pause container for consistent backup
            if app.running {
                app.pause().await?;
                true
            } else {
                false
            }
        };
        
        // Extract volume data
        let volume_data = self.extract_volume_data(app).await?;
        
        // Create backup archive
        let file = File::create(backup_path)?;
        let encoder = GzEncoder::new(file, Compression::default());
        let mut tar = Builder::new(encoder);
        
        // Write app settings
        let app_settings_json = serde_json::to_vec_pretty(&app.settings)?;
        let mut header = tar::Header::new_gnu();
        header.set_path("app-settings.json")?;
        header.set_size(app_settings_json.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append(&header, app_settings_json.as_slice())?;
        
        // Write volume settings
        let vol = app.volume().await?;
        let vol_settings_json = serde_json::to_vec_pretty(&vol.settings)?;
        let mut header = tar::Header::new_gnu();
        header.set_path("vol-settings.json")?;
        header.set_size(vol_settings_json.len() as u64);
        header.set_mode(0o600);
        header.set_cksum();
        tar.append(&header, vol_settings_json.as_slice())?;
        
        // Write volume data
        self.append_volume_data(&mut tar, volume_data).await?;
        
        tar.finish()?;
        
        // Unpause container
        if paused {
            app.unpause().await?;
        }
        
        Ok(())
    }
    
    pub async fn restore_backup(
        &self,
        backup_path: &str,
    ) -> Result<Application, OnceError> {
        let file = File::open(backup_path)?;
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);
        
        let mut app_settings: Option<ApplicationSettings> = None;
        let mut vol_settings: Option<ApplicationVolumeSettings> = None;
        let mut temp_dir = std::env::temp_dir();
        temp_dir.push("once-restore");
        std::fs::create_dir_all(&temp_dir)?;
        
        // Extract archive
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.to_path_buf();
            
            match path.to_str() {
                Some("app-settings.json") => {
                    let mut contents = Vec::new();
                    entry.read_to_end(&mut contents)?;
                    app_settings = Some(serde_json::from_slice(&contents)?);
                }
                Some("vol-settings.json") => {
                    let mut contents = Vec::new();
                    entry.read_to_end(&mut contents)?;
                    vol_settings = Some(serde_json::from_slice(&contents)?);
                }
                Some(p) if p.starts_with("data/") => {
                    let relative_path = p.strip_prefix("data/").unwrap();
                    let full_path = temp_dir.join(relative_path);
                    
                    if let Some(parent) = full_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    entry.unpack(&full_path)?;
                }
                _ => {}
            }
        }
        
        let app_settings = app_settings.ok_or(OnceError::BackupMissingSettings)?;
        let vol_settings = vol_settings.ok_or(OnceError::BackupMissingSettings)?;
        
        // Create volume with restored settings
        let name = generate_unique_name(&app_settings.name);
        let vol = ApplicationVolume::create_with_settings(
            &self.namespace, &name, vol_settings
        ).await?;
        
        // Restore volume data
        self.restore_volume_data(&vol, &temp_dir).await?;
        
        // Create and deploy application
        let mut app = Application::new(&self.namespace, &name);
        app.settings = app_settings;
        app.deploy(|_, _| {}).await?;
        
        // Run post-restore hook
        if let Err(e) = self.exec_hook(&app, "/hooks/post-restore").await {
            eprintln!("Warning: post-restore hook failed: {}", e);
        }
        
        Ok(app)
    }
}
```

## Conclusion

The Rust implementation of ONCE provides:

1. **Type Safety**: Compile-time guarantees for configuration
2. **Async Runtime**: tokio for concurrent Docker operations
3. **Modern TUI**: Ratatui for terminal interface
4. **Memory Safety**: No GC pauses, predictable performance
5. **Single Binary**: Easy deployment like the Go version
6. **Error Handling**: Result types for explicit error paths
