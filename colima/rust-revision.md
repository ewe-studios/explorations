---
title: "Colima Rust Revision - Complete Translation Guide"
subtitle: "Translating Colima's VM-based container runtime to Rust for ewe_platform"
based_on: "Colima - Lima-based Container Runtime"
target: "Rust with valtron executor (no async/await, no tokio)"
date: 2026-03-27
---

# Colima Rust Revision

## 1. Overview

### 1.1 What We're Translating

Colima is a Go-based container runtime orchestrator that manages Linux VMs (via Lima) and provisions container runtimes (Docker, containerd, Incus, Kubernetes) inside them. The complete translation to Rust involves:

| Go Component | Rust Equivalent |
|--------------|-----------------|
| `environment.Container` interface | `ContainerRuntime` trait |
| `limaVM` struct | `LimaVm` struct |
| `config.Config` struct | `Config` struct |
| `cli.CommandChain` | `TaskIterator` pattern |
| Goroutines/channels | `StreamIterator` pattern |
| Context (`ctx context.Context`) | `Context` with cancellation |

### 1.2 Key Design Decisions

#### Ownership Strategy

```go
// Go uses garbage-collected references
type colimaApp struct {
    guest environment.VM
}

func (c colimaApp) Start(conf config.Config) error {
    cs, _ := c.startWithRuntime(conf)
    c.guest.Start(ctx, conf)
}
```

```rust
// Rust uses explicit ownership with Arc for shared state
use std::sync::Arc;

pub struct ColimaApp {
    guest: Arc<LimaVm>,
    runtimes: Vec<Arc<dyn ContainerRuntime>>,
}

impl ColimaApp {
    pub fn start(&self, conf: &Config) -> Result<()> {
        let runtimes = self.start_with_runtime(conf)?;
        self.guest.start(conf)?;
        for runtime in &runtimes {
            runtime.provision()?;
            runtime.start()?;
        }
        Ok(())
    }
}
```

#### Async Pattern Translation

| Pattern | Go | Rust (valtron) |
|---------|-----|----------------|
| **Sequential** | `a.Add(func() error { ... })` | `TaskIterator::next()` |
| **Parallel** | Goroutines + channels | `StreamIterator` |
| **Retry** | `a.Retry("", time.Second, 60, ...)` | `RetryTaskIterator` |
| **Context** | `context.Context` | `Context` with timeout |

### 1.3 Crate Dependencies

```toml
[dependencies]
# Core
 anyhow = "1.0"              # Error handling
 thiserror = "1.0"           # Custom errors
 serde = { version = "1.0", features = ["derive"] }
 serde_yaml = "0.9"          # Config serialization
 tokio = { version = "1.0", features = ["full"], optional = true }  # Optional async

# System
 nix = "0.27"                # Unix syscalls
 libc = "0.2"                # C bindings
 which = "5.0"               # Binary location

# Networking
 hyper = { version = "1.0", features = ["full"] }
 tokio-tungstenite = "0.21"  # WebSocket (if needed)

# ewe_platform specific
 valtron = { path = "../valtron" }  # TaskIterator/StreamIterator
```

---

## 2. Type System Design

### 2.1 Core Traits

```rust
// Core container runtime trait
use std::sync::Arc;
use anyhow::Result;

pub trait ContainerRuntime: Send + Sync {
    /// Name of the runtime (docker, containerd, incus, kubernetes)
    fn name(&self) -> &'static str;

    /// Provision the runtime (config, services)
    fn provision(&self, ctx: &Context, conf: &Config) -> Result<()>;

    /// Start the runtime services
    fn start(&self, ctx: &Context) -> Result<()>;

    /// Stop the runtime services
    fn stop(&self, ctx: &Context, force: bool) -> Result<()>;

    /// Check if runtime is running
    fn running(&self, ctx: &Context) -> bool;

    /// Teardown runtime configuration
    fn teardown(&self, ctx: &Context) -> Result<()>;

    /// Get runtime version
    fn version(&self, ctx: &Context) -> String;

    /// Update runtime (optional)
    fn update(&self, ctx: &Context) -> Result<bool> {
        Ok(false)  // Default: not supported
    }
}

/// VM operations trait
pub trait VmOps: Send + Sync {
    /// Start the VM
    fn start(&self, conf: &Config) -> Result<()>;

    /// Stop the VM
    fn stop(&self, force: bool) -> Result<()>;

    /// Check if VM is running
    fn running(&self) -> bool;

    /// SSH into the VM
    fn ssh(&self, working_dir: Option<&str>, args: &[&str]) -> Result<()>;

    /// Run command in VM and get output
    fn run_output(&self, args: &[&str]) -> Result<String>;

    /// Run command in VM quietly
    fn run_quiet(&self, args: &[&str]) -> Result<()>;

    /// Get environment variable from VM
    fn env(&self, name: &str) -> Result<String>;

    /// Set configuration in VM
    fn set(&self, key: &str, value: &str) -> Result<()>;

    /// Get configuration from VM
    fn get(&self, key: &str) -> Option<String>;

    /// Get VM architecture
    fn arch(&self) -> Arch;

    /// Get VM user
    fn user(&self) -> Result<String>;
}
```

### 2.2 Config Structs

```rust
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub cpu: usize,
    pub disk: usize,           // GiB
    pub root_disk: usize,      // GiB
    pub memory: f32,           // GiB
    pub arch: Arch,
    pub cpu_type: Option<String>,
    pub network: Network,
    pub env: HashMap<String, String>,
    pub hostname: Option<String>,

    // SSH
    pub ssh_port: u16,
    pub forward_agent: bool,
    pub ssh_config: bool,

    // VM
    #[serde(rename = "vmType")]
    pub vm_type: VmType,
    pub vz_rosetta: bool,
    pub binfmt: Option<bool>,
    pub nested_virtualization: bool,
    pub disk_image: Option<String>,
    pub port_forwarder: PortForwarder,

    // Volume mounts
    pub mounts: Vec<Mount>,
    pub mount_type: MountType,
    pub mount_inotify: bool,

    // Runtime
    pub runtime: Runtime,
    pub auto_activate: Option<bool>,

    // Model runner (AI)
    pub model_runner: Option<ModelRunner>,

    // Kubernetes
    pub kubernetes: Kubernetes,

    // Docker config
    pub docker: HashMap<String, serde_yaml::Value>,

    // Provision scripts
    pub provision: Vec<Provision>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Arch {
    Aarch64,
    X86_64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum VmType {
    Qemu,
    Vz,
    Krunkit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Runtime {
    Docker,
    Containerd,
    Incus,
    #[serde(other)]
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MountType {
    Sshfs,
    NineP,
    Virtiofs,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PortForwarder {
    Ssh,
    Grpc,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Network {
    pub address: bool,
    pub dns_resolvers: Vec<IpAddr>,
    pub dns_hosts: HashMap<String, String>,
    pub host_addresses: bool,
    pub mode: NetworkMode,
    pub bridge_interface: Option<String>,
    pub preferred_route: bool,
    pub gateway_address: Option<IpAddr>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NetworkMode {
    Shared,
    Bridged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mount {
    pub location: String,
    #[serde(rename = "mountPoint")]
    pub mount_point: Option<String>,
    pub writable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kubernetes {
    pub enabled: bool,
    pub version: String,
    pub k3s_args: Vec<String>,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelRunner {
    #[serde(rename = "docker")]
    Docker,
    #[serde(rename = "ramalama")]
    Ramalama,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provision {
    pub mode: ProvisionMode,
    pub script: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ProvisionMode {
    AfterBoot,
    Ready,
}
```

### 2.3 Error Types

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ColimaError {
    #[error("VM operation failed: {0}")]
    VmError(String),

    #[error("Runtime error for {runtime}: {source}")]
    RuntimeError {
        runtime: String,
        #[source]
        source: anyhow::Error,
    },

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Mount error: {0}")]
    MountError(String),

    #[error("Dependency missing: {0}")]
    DependencyMissing(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("YAML error: {0}")]
    YamlError(#[from] serde_yaml::Error),

    #[error(" Lima error: {0}")]
    LimaError(String),
}

pub type Result<T> = std::result::Result<T, ColimaError>;
```

---

## 3. TaskIterator Implementation

### 3.1 CommandChain Translation

Go's `CommandChain` with `a.Add()` becomes `TaskIterator`:

```go
// Go original
func (l *limaVM) Start(ctx context.Context, conf config.Config) error {
    a := l.Init(ctx)

    a.Add(func() error {
        l.limaConf, err = newConf(ctx, conf)
        return err
    })

    a.Add(func() error {
        return l.createRuntimeDisk(conf)
    })

    a.Add(func() error {
        return yamlutil.WriteYAML(l.limaConf, confFile)
    })

    a.Add(func() error {
        return l.host.Run(limactl, "start", "--tty=false", confFile)
    })

    return a.Exec()
}
```

```rust
// Rust translation with TaskIterator
use valtron::{TaskIterator, TaskStatus, NoSpawner};

pub struct StartVmTask {
    lima_conf: LimaConfig,
    conf: Config,
    state: StartVmState,
}

enum StartVmState {
    CreatingConf,
    CreatingDisk,
    WritingYaml,
    StartingLima,
    Done,
}

impl TaskIterator for StartVmTask {
    type Ready = Result<()>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.state {
            StartVmState::CreatingConf => {
                self.lima_conf = match new_conf(&self.conf) {
                    Ok(conf) => conf,
                    Err(e) => return Some(TaskStatus::Ready(Err(e))),
                };
                self.state = StartVmState::CreatingDisk;
                None  // Continue to next iteration
            }
            StartVmState::CreatingDisk => {
                if let Err(e) = self.create_runtime_disk() {
                    return Some(TaskStatus::Ready(Err(e)));
                }
                self.state = StartVmState::WritingYaml;
                None
            }
            StartVmState::WritingYaml => {
                let conf_file = format!("/tmp/{}.yaml", self.conf.profile.id());
                if let Err(e) = serde_yaml::to_writer(File::create(&conf_file).unwrap(), &self.lima_conf) {
                    return Some(TaskStatus::Ready(Err(ColimaError::from(e))));
                }
                self.state = StartVmState::StartingLima;
                None
            }
            StartVmState::StartingLima => {
                let result = Command::new("limactl")
                    .args(["start", "--tty=false", &conf_file])
                    .output();

                match result {
                    Ok(output) if output.status.success() => {
                        self.state = StartVmState::Done;
                        Some(TaskStatus::Ready(Ok(())))
                    }
                    Ok(output) => {
                        Some(TaskStatus::Ready(Err(ColimaError::LimaError(
                            String::from_utf8_lossy(&output.stderr).to_string()
                        ))))
                    }
                    Err(e) => {
                        Some(TaskStatus::Ready(Err(ColimaError::from(e))))
                    }
                }
            }
            StartVmState::Done => None,
        }
    }
}
```

### 3.2 Retry Pattern

```go
// Go original
a.Retry("", time.Second, 60, func(int) error {
    return d.systemctl.Start("docker.service")
})
```

```rust
// Rust translation with retry
use valtron::{TaskIterator, TaskStatus, NoSpawner};
use std::time::Duration;
use std::thread;

pub struct RetryTask<F, T>
where
    F: FnMut() -> Result<T>,
{
    func: F,
    interval: Duration,
    max_attempts: usize,
    current_attempt: usize,
    last_error: Option<anyhow::Error>,
}

impl<F, T> TaskIterator for RetryTask<F, T>
where
    F: FnMut() -> Result<T>,
    T: 'static,
{
    type Ready = Result<T>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        if self.current_attempt >= self.max_attempts {
            return Some(TaskStatus::Ready(Err(
                self.last_error.take().unwrap_or_else(|| anyhow::anyhow!("max retries exceeded"))
            )));
        }

        match (self.func)() {
            Ok(result) => {
                Some(TaskStatus::Ready(Ok(result)))
            }
            Err(e) => {
                self.last_error = Some(e);
                self.current_attempt += 1;
                thread::sleep(self.interval);
                None  // Continue retrying
            }
        }
    }
}

// Usage
let retry_task = RetryTask {
    func: || systemctl_start("docker.service"),
    interval: Duration::from_secs(1),
    max_attempts: 60,
    current_attempt: 0,
    last_error: None,
};
```

### 3.3 StreamIterator for Daemon Processes

```go
// Go: Background process monitoring
func (f *inotifyProcess) handleEvents(ctx context.Context, watcher watcher) error {
    for {
        select {
        case <-ctx.Done():
            return nil
        case event := <-watcher.Events():
            f.forwardEvent(event)
        }
    }
}
```

```rust
// Rust: StreamIterator for event handling
use valtron::{StreamIterator, StreamStatus, StreamYield};
use std::pin::Pin;
use std::task::{Context, Poll};
use futures::Stream;

pub struct InotifyStream {
    watcher: InotifyWatcher,
    running: bool,
}

impl StreamIterator for InotifyStream {
    type Item = Result<InotifyEvent>;
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<StreamStatus<Self::Item, Self::Spawner>> {
        if !self.running {
            return None;
        }

        match self.watcher.read_event() {
            Ok(event) => {
                Some(StreamStatus::Yield(StreamYield::Item(Ok(event))))
            }
            Err(InotifyError::NoMoreEvents) => {
                // No events yet, yield pending
                Some(StreamStatus::Yield(StreamYield::Pending))
            }
            Err(e) => {
                self.running = false;
                Some(StreamStatus::Yield(StreamYield::Item(Err(e.into()))))
            }
        }
    }
}

// Usage in executor
let mut inotify_stream = InotifyStream {
    watcher: InotifyWatcher::new(dirs)?,
    running: true,
};

while let Some(status) = inotify_stream.next() {
    match status {
        StreamStatus::Yield(StreamYield::Item(Ok(event))) => {
            forward_event(event)?;
        }
        StreamStatus::Yield(StreamYield::Item(Err(e))) => {
            eprintln!("inotify error: {}", e);
            break;
        }
        StreamStatus::Yield(StreamYield::Pending) => {
            // Wait for more events
            std::thread::sleep(Duration::from_millis(100));
        }
        _ => {}
    }
}
```

---

## 4. Runtime Implementations

### 4.1 Docker Runtime

```rust
use std::sync::Arc;
use std::process::Command;

pub struct DockerRuntime {
    host: Arc<dyn HostOps>,
    guest: Arc<dyn GuestOps>,
    systemctl: Systemctl,
}

impl DockerRuntime {
    pub fn new(host: Arc<dyn HostOps>, guest: Arc<dyn GuestOps>) -> Self {
        Self {
            host,
            guest,
            systemctl: Systemctl::new(guest.clone()),
        }
    }

    fn provision_containerd(&self) -> Result<()> {
        // Provision containerd (Docker uses it internally)
        // ... implementation
        Ok(())
    }

    fn create_daemon_file(&self, docker_conf: &HashMap<String, Value>, env: &HashMap<String, String>) -> Result<()> {
        let mut config = serde_json::Map::new();

        // Default config
        config.insert("feature".to_string(), json!({
            "containerd-snapshotter": true
        }));
        config.insert("exec-opts".to_string(), json!(["native.cgroupdriver=cgroupfs"]));
        config.insert("log-driver".to_string(), json!("json-file"));
        config.insert("log-opts".to_string(), json!({
            "max-size": "10m",
            "max-file": "3"
        }));

        // Merge user config
        for (k, v) in docker_conf {
            config.insert(k.clone(), v.clone());
        }

        // Write to /etc/docker/daemon.json
        let content = serde_json::to_string_pretty(&config)?;
        self.guest.write("/etc/docker/daemon.json", content.as_bytes())?;

        Ok(())
    }

    fn setup_context(&self) -> Result<()> {
        let name = current_profile().id();
        let socket = format!("unix://{}", docker_host_socket_file());

        if !self.has_context(&name) {
            self.host.run_quiet(&[
                "docker", "context", "create", &name,
                "--description", &name,
                "--docker", &format!("host={}", socket),
            ])?;
        }

        Ok(())
    }
}

impl ContainerRuntime for DockerRuntime {
    fn name(&self) -> &'static str {
        "docker"
    }

    fn provision(&self, ctx: &Context, conf: &Config) -> Result<()> {
        // Provision containerd
        self.provision_containerd()?;

        // Create daemon.json
        self.create_daemon_file(&conf.docker, &conf.env)?;

        // Setup context
        self.setup_context()?;

        if conf.auto_activate.unwrap_or(true) {
            self.use_context()?;
        }

        Ok(())
    }

    fn start(&self, ctx: &Context) -> Result<()> {
        // Start docker.service with retry
        let mut attempt = 0;
        while attempt < 60 {
            match self.systemctl.start("docker.service") {
                Ok(_) => break,
                Err(_) => {
                    attempt += 1;
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }
        }

        // Verify docker is responsive
        attempt = 0;
        while attempt < 60 {
            if self.guest.run_quiet(&["sudo", "docker", "info"]).is_ok() {
                break;
            }
            attempt += 1;
            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        Ok(())
    }

    fn stop(&self, ctx: &Context, force: bool) -> Result<()> {
        if self.running(ctx) {
            self.systemctl.stop("docker.service", force)?;
        }
        self.teardown_context()?;
        Ok(())
    }

    fn running(&self, ctx: &Context) -> bool {
        self.systemctl.active("docker.service").unwrap_or(false)
    }

    fn teardown(&self, ctx: &Context) -> Result<()> {
        self.teardown_context()?;
        Ok(())
    }

    fn version(&self, ctx: &Context) -> String {
        self.host.run_output(&[
            "docker", "--context", &current_profile().id(),
            "version", "--format", "client: v{{.Client.Version}}\nserver: v{{.Server.Version}}"
        ]).unwrap_or_default()
    }
}
```

### 4.2 Containerd Runtime

```rust
pub struct ContainerdRuntime {
    host: Arc<dyn HostOps>,
    guest: Arc<dyn GuestOps>,
    systemctl: Systemctl,
}

impl ContainerdRuntime {
    fn provision_config(&self, profile_path: &str, central_path: &str, guest_path: &str, default_conf: &[u8]) -> Result<()> {
        // Priority: per-profile > central > embedded default
        if let Ok(data) = std::fs::read(profile_path) {
            self.guest.write(guest_path, &data)?;
            return Ok(());
        }

        if let Ok(data) = std::fs::read(central_path) {
            self.guest.write(guest_path, &data)?;
            return Ok(());
        }

        // Write default to central location for discoverability
        std::fs::create_dir_all(std::path::Path::new(central_path).parent().unwrap())?;
        std::fs::write(central_path, default_conf)?;
        self.guest.write(guest_path, default_conf)?;

        Ok(())
    }
}

impl ContainerRuntime for ContainerdRuntime {
    fn name(&self) -> &'static str {
        "containerd"
    }

    fn provision(&self, ctx: &Context, conf: &Config) -> Result<()> {
        let config_dir = current_profile().config_dir();

        // containerd config
        let profile_path = format!("{}/containerd/config.toml", config_dir);
        let central_path = format!("{}/.config/containerd/config.toml", std::env::var("HOME")?);
        self.provision_config(&profile_path, &central_path, "/etc/containerd/config.toml", CONTAINERD_CONF)?;

        // buildkitd config
        let profile_path = format!("{}/containerd/buildkitd.toml", config_dir);
        let central_path = format!("{}/.config/buildkit/buildkitd.toml", std::env::var("HOME")?);
        self.provision_config(&profile_path, &central_path, "/etc/buildkit/buildkitd.toml", BUILDKIT_CONF)?;

        Ok(())
    }

    fn start(&self, ctx: &Context) -> Result<()> {
        self.systemctl.restart("containerd.service")?;

        // Verify containerd is responsive
        let mut attempt = 0;
        while attempt < 10 {
            if self.guest.run_quiet(&["sudo", "nerdctl", "info"]).is_ok() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(5));
            attempt += 1;
        }

        self.systemctl.start("buildkit.service")?;

        Ok(())
    }

    fn stop(&self, ctx: &Context, force: bool) -> Result<()> {
        self.systemctl.stop("containerd.service", force)?;
        Ok(())
    }

    fn running(&self, ctx: &Context) -> bool {
        self.systemctl.active("containerd.service").unwrap_or(false)
    }

    fn version(&self, ctx: &Context) -> String {
        self.guest.run_output(&[
            "sudo", "nerdctl", "version",
            "--format", "client: {{.Client.Version}}\nserver: {{(index .Server.Components 0).Version}}"
        ]).unwrap_or_default()
    }
}
```

---

## 5. VM Management

### 5.1 Lima VM Struct

```rust
use std::sync::Arc;
use std::path::PathBuf;

pub struct LimaVm {
    host: Arc<dyn HostOps>,
    conf: Option<Config>,
    lima_conf: LimaConfig,
    lima_home: PathBuf,
    daemon: Arc<dyn DaemonManager>,
}

impl LimaVm {
    pub fn new(host: Arc<dyn HostOps>) -> Self {
        let lima_home = config::lima_dir();
        let envs = vec![
            format!("LIMA_HOME={}", lima_home.display()),
            format!("LIMA_INSTANCE={}", current_profile().id()),
            format!("COLIMA_BINARY={}", std::env::current_exe().unwrap().display()),
        ];

        Self {
            host: host.with_env(&envs),
            conf: None,
            lima_conf: LimaConfig::default(),
            lima_home,
            daemon: Arc::new(ProcessManager::new(host.clone())),
        }
    }

    fn created(&self) -> bool {
        current_profile().lima_file().exists()
    }

    fn start_daemon(&self, conf: &Config) -> Result<()> {
        // vmnet is needed for QEMU or bridged mode
        let use_vmnet = conf.vm_type == VmType::Qemu || conf.network.mode == NetworkMode::Bridged;
        let mut network_address = conf.network.address && use_vmnet;

        if !util::is_macos() || (!conf.mount_inotify && !network_address) {
            return Ok(());
        }

        // Start daemon with vmnet and/or inotify
        self.daemon.start(conf)?;

        Ok(())
    }
}

impl VmOps for LimaVm {
    fn start(&self, conf: &Config) -> Result<()> {
        if self.created() {
            return self.resume(conf);
        }

        // Fresh start
        self.start_daemon(conf)?;

        // Create Lima config
        self.lima_conf = new_conf(conf)?;

        // Create runtime disk
        self.create_runtime_disk(conf)?;

        // Download disk image if needed
        self.download_disk_image(conf)?;

        // Write Lima config
        let conf_file = format!("/tmp/{}.yaml", current_profile().id());
        let yaml = serde_yaml::to_string(&self.lima_conf)?;
        std::fs::write(&conf_file, yaml)?;

        // Start Lima
        Command::new("limactl")
            .args(["start", "--tty=false", &conf_file])
            .output()?;

        std::fs::remove_file(&conf_file)?;

        // Store config for restart
        // self.conf = Some(conf.clone());

        Ok(())
    }

    fn stop(&self, force: bool) -> Result<()> {
        if !self.running() && !force {
            return Ok(());
        }

        // Stop daemon
        self.daemon.stop(&self.conf.unwrap_or_default())?;

        // Stop Lima
        let args = if force {
            vec!["stop", "--force", &current_profile().id()]
        } else {
            vec!["stop", &current_profile().id()]
        };

        Command::new("limactl").args(&args).output()?;

        Ok(())
    }

    fn running(&self) -> bool {
        if let Ok(instance) = limautil::instance() {
            instance.running()
        } else {
            false
        }
    }

    fn ssh(&self, working_dir: Option<&str>, args: &[&str]) -> Result<()> {
        let mut cmd = Command::new("limactl");
        cmd.args(["ssh", &current_profile().id()]);

        if let Some(dir) = working_dir {
            cmd.arg("--").arg("cd").arg(dir).arg("&&");
        }

        cmd.args(args);
        cmd.status()?;

        Ok(())
    }

    fn run_output(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("limactl")
            .args(["ssh", &current_profile().id(), "--"])
            .args(args)
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(ColimaError::LimaError(
                String::from_utf8_lossy(&output.stderr).to_string()
            ))
        }
    }

    fn run_quiet(&self, args: &[&str]) -> Result<()> {
        let output = Command::new("limactl")
            .args(["ssh", &current_profile().id(), "--"])
            .args(args)
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            Err(ColimaError::LimaError(
                String::from_utf8_lossy(&output.stderr).to_string()
            ))
        }
    }

    fn env(&self, name: &str) -> Result<String> {
        self.run_output(&["echo", &format!("${}", name)])
    }

    fn set(&self, key: &str, value: &str) -> Result<()> {
        // Store in VM metadata
        let file = current_profile().state_file();
        let mut store = store::load()?;
        // ... update store
        store::save(store)?;
        Ok(())
    }

    fn get(&self, key: &str) -> Option<String> {
        // Retrieve from VM metadata
        store::load().ok().and_then(|s| s.metadata.get(key).cloned())
    }

    fn arch(&self) -> Arch {
        let output = self.run_output(&["uname", "-m"]).unwrap_or_default();
        match output.as_str() {
            "aarch64" => Arch::Aarch64,
            _ => Arch::X86_64,
        }
    }

    fn user(&self) -> Result<String> {
        self.run_output(&["whoami"])
    }
}
```

---

## 6. Edge Cases and Safety Guarantees

### 6.1 Error Handling

```rust
// Go: Silent errors with logging
if err := d.createDaemonFile(conf.Docker, conf.Env); err != nil {
    log.Warnln(err)
}

// Rust: Explicit error handling with recovery
match self.create_daemon_file(&conf.docker, &conf.env) {
    Ok(_) => {}
    Err(e) => {
        log::warn!("failed to create daemon file: {}", e);
        // Continue anyway - not fatal
    }
}

// Or use Result with ? for fatal errors
self.setup_context()?;  // Propagate on error
```

### 6.2 Resource Cleanup

```rust
// Using RAII for cleanup
pub struct TempFile {
    path: PathBuf,
}

impl TempFile {
    pub fn create(prefix: &str) -> Result<Self> {
        let path = std::env::temp_dir().join(format!("{}_{}", prefix, uuid::Uuid::new_v4()));
        std::fs::File::create(&path)?;
        Ok(Self { path })
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

// Usage
let temp_file = TempFile::create("colima")?;
// File is automatically deleted when temp_file goes out of scope
```

### 6.3 Concurrency Safety

```rust
use std::sync::{Arc, Mutex};

// Shared state with mutex
pub struct SharedState {
    runtimes: Mutex<Vec<Arc<dyn ContainerRuntime>>>,
    vm_running: AtomicBool,
}

// Thread-safe access
impl ColimaApp {
    pub fn add_runtime(&self, runtime: Arc<dyn ContainerRuntime>) {
        self.state.runtimes.lock().unwrap().push(runtime);
    }

    pub fn runtimes(&self) -> Vec<Arc<dyn ContainerRuntime>> {
        self.state.runtimes.lock().unwrap().clone()
    }
}
```

---

## 7. Performance Considerations

### 7.1 Memory Management

```rust
// Use Cow for zero-copy string handling
use std::borrow::Cow;

fn get_config_value(key: &str) -> Cow<'static, str> {
    if let Some(val) = CONFIG_CACHE.get(key) {
        Cow::Borrowed(val)
    } else {
        let val = load_config_value(key);
        Cow::Owned(val)
    }
}
```

### 7.2 Efficient String Handling

```rust
// Use String::with_capacity for known sizes
let mut output = String::with_capacity(4096);
Command::new("limactl")
    .args(&["ssh", &profile.id()])
    .output()?
    .stdout
    .read_to_string(&mut output)?;
```

### 7.3 Batch Operations

```rust
// Batch file operations
use std::fs;

pub fn batch_write(files: &[(&str, &[u8])]) -> Result<()> {
    for (path, content) in files {
        fs::write(path, content)?;
    }
    Ok(())
}
```

---

## 8. Code Examples

### 8.1 Complete App Usage

```rust
use colima_rs::{ColimaApp, Config, VmType, Runtime};

fn main() -> anyhow::Result<()> {
    // Initialize app
    let app = ColimaApp::new()?;

    // Create config
    let config = Config {
        cpu: 4,
        memory: 8.0,
        disk: 100,
        vm_type: VmType::Vz,
        runtime: Runtime::Docker,
        kubernetes: Kubernetes {
            enabled: true,
            version: "v1.28.0".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };

    // Start
    app.start(&config)?;

    // Check status
    let status = app.status()?;
    println!("Colima is running: {}", status.running);
    println!("Runtime: {}", status.runtime);
    println!("Kubernetes: {}", status.kubernetes);

    // Stop
    // app.stop(false)?;

    Ok(())
}
```

### 8.2 Custom Runtime

```rust
pub struct CustomRuntime {
    name: &'static str,
    provision_script: &'static str,
}

impl ContainerRuntime for CustomRuntime {
    fn name(&self) -> &'static str {
        self.name
    }

    fn provision(&self, ctx: &Context, conf: &Config) -> Result<()> {
        // Custom provision logic
        Ok(())
    }

    fn start(&self, ctx: &Context) -> Result<()> {
        // Custom start logic
        Ok(())
    }

    fn stop(&self, ctx: &Context, force: bool) -> Result<()> {
        // Custom stop logic
        Ok(())
    }

    fn running(&self, ctx: &Context) -> bool {
        // Custom running check
        true
    }
}

// Register custom runtime
environment::register_container("custom", || {
    Box::new(CustomRuntime {
        name: "custom",
        provision_script: include_str!("custom_provision.sh"),
    })
});
```

---

## Summary

| Go Pattern | Rust Equivalent |
|------------|-----------------|
| `context.Context` | `Context` with cancellation |
| `error` return | `Result<T, ColimaError>` |
| `defer` | `Drop` trait |
| Goroutines | `TaskIterator` / `StreamIterator` |
| Channels | `StreamIterator` yields |
| Interfaces | Traits |
| Structs with methods | Structs with trait impls |
| `nil` checks | `Option<T>` |
| `panic` | `panic!()` or `Err` |

---

*Next: [Production-Grade](production-grade.md)*
