---
title: "Colima Production-Grade Implementation"
subtitle: "Performance optimizations, resource management, multi-instance support, monitoring, and security"
based_on: "Colima - Lima-based Container Runtime"
level: "Advanced"
prerequisites: "[Rust Revision](rust-revision.md)"
---

# Production-Grade Colima Implementation

## Table of Contents

1. [Performance Optimizations](#1-performance-optimizations)
2. [Resource Management](#2-resource-management)
3. [Multi-Instance Support](#3-multi-instance-support)
4. [Monitoring and Observability](#4-monitoring-and-observability)
5. [Security Considerations](#5-security-considerations)
6. [High Availability](#6-high-availability)
7. [Deployment Strategies](#7-deployment-strategies)

---

## 1. Performance Optimizations

### 1.1 VM Configuration Tuning

**CPU Pinning:**
```yaml
# ~/.colima/prod/colima.yaml
cpu: 8
cpuType:
  aarch64: firestorm  # Performance cores on M1/M2
  x86_64: host
```

```rust
// Rust: Optimize CPU type selection
fn optimal_cpu_type(arch: Arch) -> &'static str {
    match (arch, detect_host_arch()) {
        (Arch::Aarch64, Arch::Aarch64) => "firestorm",  // M1/M2 performance cores
        (Arch::X86_64, Arch::X86_64) => "host",
        _ => "cortex-a72",  // Safe default for aarch64
    }
}
```

**Memory Optimization:**
```yaml
# Use huge pages for better memory performance
memory: 16
```

**Disk I/O:**
```yaml
# Use virtiofs for best I/O performance (vz backend)
vmType: vz
mountType: virtiofs
disk: 500  # Pre-allocate for production workloads
```

### 1.2 Network Performance

**gRPC Port Forwarder:**
```bash
# Use gRPC instead of SSH for better performance
colima start prod --port-forwarder grpc
```

```rust
// Rust: gRPC forwarder configuration
pub struct PortForwardConfig {
    pub forwarder: PortForwarder,
    pub batch_size: usize,
    pub buffer_size: usize,
}

impl Default for PortForwardConfig {
    fn default() -> Self {
        Self {
            forwarder: PortForwarder::Grpc,
            batch_size: 100,
            buffer_size: 65536,  // 64KB
        }
    }
}
```

**Network Mode Selection:**
| Mode | Latency | Throughput | Use Case |
|------|---------|------------|----------|
| NAT (slirp) | High | Low | Development |
| vmnet (shared) | Medium | Medium | Testing |
| vmnet (bridged) | Low | High | Production |

### 1.3 Container Runtime Tuning

**Docker Daemon Optimizations:**
```json
{
  "features": {
    "containerd-snapshotter": true
  },
  "exec-opts": ["native.cgroupdriver=cgroupfs"],
  "log-driver": "json-file",
  "log-opts": {
    "max-size": "100m",
    "max-file": "5"
  },
  "storage-opts": ["size=50G"],
  "dns": ["8.8.8.8", "1.1.1.1"],
  "default-ulimits": {
    "nofile": {
      "Name": "nofile",
      "Hard": 65536,
      "Soft": 32768
    }
  },
  "live-restore": true,
  "userns-remap": "default"
}
```

**Containerd Optimizations:**
```toml
# /etc/containerd/config.toml
version = 2

[plugins."io.containerd.grpc.v1.cri"]
  sandbox_image = "registry.k8s.io/pause:3.9"

  [plugins."io.containerd.grpc.v1.cri".containerd]
    snapshotter = "overlayfs"
    default_runtime_name = "runc"

    [plugins."io.containerd.grpc.v1.cri".containerd.runtimes.runc]
      runtime_type = "io.containerd.runc.v2"

      [plugins."io.containerd.grpc.v1.cri".containerd.runtimes.runc.options]
        SystemdCgroup = true
        BinaryName = "/usr/bin/runc"

        [plugins."io.containerd.grpc.v1.cri".containerd.runtimes.runc.options.SystemdCgroup]
          EnableUserns = true
```

### 1.4 Build Performance

**Buildkit Cache:**
```bash
# Enable Buildkit cache
export BUILDKIT_FLAGS="--cache-from type=local,src=/var/lib/buildkit --cache-to type=local,dest=/var/lib/buildkit,mode=max"
```

**Parallel Builds:**
```rust
// Rust: Parallel image builds
use rayon::prelude::*;

pub fn build_images_parallel(images: &[ImageConfig]) -> Result<Vec<BuildResult>> {
    images.par_iter()
        .map(|image| build_image(image))
        .collect()
}
```

---

## 2. Resource Management

### 2.1 CPU Quotas

```yaml
# Limit CPU usage
cpu: 4

# Prevent CPU starvation of host
# Lima cgroup configuration
limactl shell colima-prod sudo bash -c '
  echo "50000 100000" > /sys/fs/cgroup/lima/colima/cpu.max
'
```

### 2.2 Memory Limits

```yaml
# Hard memory limit
memory: 8

# OOM configuration in VM
limactl shell colima-prod sudo bash -c '
  echo "8589934592" > /sys/fs/cgroup/lima/colima/memory.max
'
```

### 2.3 Disk Quotas

```yaml
# Disk size with warning threshold
disk: 200
rootDisk: 50

# Monitor disk usage
colima disk-usage prod
```

```rust
// Rust: Disk usage monitoring
pub struct DiskUsage {
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub usage_percent: f32,
}

impl DiskUsage {
    pub fn check_threshold(&self, threshold: f32) -> bool {
        self.usage_percent > threshold
    }

    pub fn warn_if_high(&self) {
        if self.check_threshold(0.8) {
            log::warn!("Disk usage high: {:.1}%", self.usage_percent * 100.0);
        }
        if self.check_threshold(0.95) {
            log::error!("Disk usage critical: {:.1}%", self.usage_percent * 100.0);
        }
    }
}
```

### 2.4 Resource Scheduling

```rust
// Rust: Resource-aware scheduling
pub struct ResourceScheduler {
    cpu_available: usize,
    memory_available: f32,  // GiB
    disk_available: usize,  // GiB
}

impl ResourceScheduler {
    pub fn can_schedule(&self, req: &ResourceRequest) -> bool {
        self.cpu_available >= req.cpu &&
        self.memory_available >= req.memory &&
        self.disk_available >= req.disk
    }

    pub fn allocate(&mut self, req: &ResourceRequest) -> Result<()> {
        if !self.can_schedule(req) {
            return Err(ResourceError::InsufficientResources);
        }

        self.cpu_available -= req.cpu;
        self.memory_available -= req.memory;
        self.disk_available -= req.disk;

        Ok(())
    }

    pub fn deallocate(&mut self, req: &ResourceRequest) {
        self.cpu_available += req.cpu;
        self.memory_available += req.memory;
        self.disk_available += req.disk;
    }
}
```

---

## 3. Multi-Instance Support

### 3.1 Profile Management

```bash
# Create isolated instances
colima start dev --cpu 4 --memory 8 --disk 100
colima start staging --cpu 8 --memory 16 --disk 200
colima start prod --cpu 16 --memory 32 --disk 500

# List all instances
colima list

# Switch between instances
docker context use colima-dev
docker context use colima-staging
docker context use colima-prod
```

### 3.2 Instance Isolation

| Resource | Isolation Method |
|----------|------------------|
| **Network** | Separate NAT/bridged networks |
| **Disk** | Separate diffdisk files |
| **Sockets** | Per-profile socket paths |
| **Config** | Per-profile YAML files |
| **Logs** | Per-profile log files |

```rust
// Rust: Profile isolation
pub struct Profile {
    pub name: String,
    pub id: String,  // colima-{name}
    pub config_dir: PathBuf,
    pub lima_file: PathBuf,
    pub diffdisk: PathBuf,
    pub docker_socket: PathBuf,
    pub ssh_config: PathBuf,
}

impl Profile {
    pub fn new(name: &str) -> Self {
        let base = config::colima_dir().join(name);
        Self {
            name: name.to_string(),
            id: format!("colima-{}", name),
            config_dir: base.clone(),
            lima_file: base.join("colima.yaml"),
            diffdisk: base.join("diffdisk"),
            docker_socket: base.join("docker.sock"),
            ssh_config: base.join("ssh_config"),
        }
    }

    pub fn isolate(&self, other: &Profile) -> bool {
        self.config_dir != other.config_dir &&
        self.diffdisk != other.diffdisk
    }
}
```

### 3.3 Instance Orchestration

```rust
// Rust: Multi-instance manager
pub struct InstanceManager {
    instances: HashMap<String, ColimaApp>,
    shared_resources: Arc<SharedResources>,
}

impl InstanceManager {
    pub fn start_instance(&mut self, name: &str, config: Config) -> Result<()> {
        let app = ColimaApp::new()?;
        app.start(&config)?;
        self.instances.insert(name.to_string(), app);
        Ok(())
    }

    pub fn stop_instance(&mut self, name: &str) -> Result<()> {
        if let Some(app) = self.instances.remove(name) {
            app.stop(false)?;
        }
        Ok(())
    }

    pub fn list_instances(&self) -> Vec<InstanceStatus> {
        self.instances.iter()
            .map(|(name, app)| InstanceStatus {
                name: name.clone(),
                running: app.is_running(),
                runtime: app.get_runtime(),
            })
            .collect()
    }
}
```

---

## 4. Monitoring and Observability

### 4.1 Metrics Collection

**Prometheus Metrics:**
```rust
// Rust: Prometheus metrics
use prometheus::{Registry, Gauge, Counter, Histogram};

pub struct ColimaMetrics {
    registry: Registry,
    cpu_usage: Gauge,
    memory_usage: Gauge,
    disk_usage: Gauge,
    container_count: Gauge,
    start_count: Counter,
    stop_count: Counter,
    operation_duration: Histogram,
}

impl ColimaMetrics {
    pub fn new() -> Result<Self> {
        let registry = Registry::new();

        let cpu_usage = Gauge::new("colima_cpu_usage", "CPU usage percentage")?;
        registry.register(Box::new(cpu_usage.clone()))?;

        let memory_usage = Gauge::new("colima_memory_usage_bytes", "Memory usage in bytes")?;
        registry.register(Box::new(memory_usage.clone()))?;

        // ... register other metrics

        Ok(Self {
            registry,
            cpu_usage,
            memory_usage,
            disk_usage,
            container_count,
            start_count,
            stop_count,
            operation_duration,
        })
    }

    pub fn record_start(&self) {
        self.start_count.inc();
    }

    pub fn update_cpu(&self, usage: f64) {
        self.cpu_usage.set(usage);
    }
}
```

### 4.2 Logging

**Structured Logging:**
```rust
// Rust: Structured logging with tracing
use tracing::{info, warn, error, instrument};
use tracing_subscriber::{fmt, EnvFilter};

#[instrument(skip(conf), fields(profile = conf.profile))]
pub fn start_vm(conf: &Config) -> Result<()> {
    info!("starting VM");

    match lima_start(conf) {
        Ok(_) => {
            info!("VM started successfully");
            Ok(())
        }
        Err(e) => {
            error!(error = %e, "failed to start VM");
            Err(e)
        }
    }
}

// Setup logging
fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(true)
        .with_thread_ids(true)
        .init();
}
```

### 4.3 Health Checks

```rust
// Rust: Health check system
pub struct HealthChecker {
    checks: Vec<Box<dyn HealthCheck + Send + Sync>>,
}

impl HealthChecker {
    pub fn add_check<C: HealthCheck + 'static>(&mut self, check: C) {
        self.checks.push(Box::new(check));
    }

    pub fn run_checks(&self) -> HealthStatus {
        let mut status = HealthStatus::Healthy;

        for check in &self.checks {
            match check.run() {
                Ok(_) => {}
                Err(e) => {
                    if check.is_critical() {
                        return HealthStatus::Unhealthy { reason: e };
                    }
                    status = HealthStatus::Degraded { reason: e };
                }
            }
        }

        status
    }
}

pub trait HealthCheck {
    fn name(&self) -> &'static str;
    fn run(&self) -> Result<()>;
    fn is_critical(&self) -> bool;
}

// VM health check
pub struct VmHealthCheck;

impl HealthCheck for VmHealthCheck {
    fn name(&self) -> &'static str {
        "vm_running"
    }

    fn run(&self) -> Result<()> {
        if !vm_running() {
            return Err(HealthError::VmNotRunning);
        }
        Ok(())
    }

    fn is_critical(&self) -> bool {
        true
    }
}
```

### 4.4 Alerting

```yaml
# Alerting rules (Prometheus)
groups:
  - name: colima
    rules:
      - alert: HighCPUUsage
        expr: colima_cpu_usage > 90
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High CPU usage on {{ $labels.instance }}"

      - alert: DiskSpaceLow
        expr: colima_disk_usage_percent > 85
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Low disk space on {{ $labels.instance }}"

      - alert: VmNotRunning
        expr: colima_vm_running == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "VM {{ $labels.instance }} is not running"
```

---

## 5. Security Considerations

### 5.1 VM Isolation

```yaml
# Enable nested virtualization for additional isolation
nestedVirtualization: true

# Use user namespace
docker:
  userns-remap: default
```

### 5.2 Network Security

```bash
# Restrict network access
colima start prod \
  --network-mode bridged \
  --network-interface en0 \
  --dns 8.8.8.8

# Firewall rules on host
sudo pfctl -f /etc/pf.conf
```

### 5.3 Secrets Management

```rust
// Rust: Secrets handling
use secrecy::{Secret, ExposeSecret};

pub struct SecureConfig {
    pub docker_config: Secret<String>,
    pub kubernetes_token: Option<Secret<String>>,
}

impl SecureConfig {
    pub fn new(docker_config: String) -> Self {
        Self {
            docker_config: Secret::new(docker_config),
            kubernetes_token: None,
        }
    }

    pub fn with_k8s_token(mut self, token: String) -> Self {
        self.kubernetes_token = Some(Secret::new(token));
        self
    }
}

// Usage
let config = SecureConfig::new("...".to_string());
let token = config.kubernetes_token.as_ref().unwrap().expose_secret();
```

### 5.4 Audit Logging

```rust
// Rust: Audit logging
use chrono::Utc;

pub struct AuditLog {
    log_file: File,
}

impl AuditLog {
    pub fn log_operation(&mut self, op: &AuditOperation) {
        let entry = AuditEntry {
            timestamp: Utc::now(),
            operation: op.name(),
            profile: op.profile(),
            user: op.user(),
            success: op.success(),
        };

        writeln!(self.log_file, "{}", serde_json::to_string(&entry).unwrap()).unwrap();
    }
}

#[derive(Serialize)]
struct AuditEntry {
    timestamp: chrono::DateTime<Utc>,
    operation: &'static str,
    profile: String,
    user: String,
    success: bool,
}
```

---

## 6. High Availability

### 6.1 VM Redundancy

```rust
// Rust: Active-passive HA
pub struct HighAvailabilityManager {
    primary: ColimaApp,
    standby: Option<ColimaApp>,
    health_checker: HealthChecker,
}

impl HighAvailabilityManager {
    pub fn new(primary: ColimaApp) -> Self {
        Self {
            primary,
            standby: None,
            health_checker: HealthChecker::new(),
        }
    }

    pub fn with_standby(mut self, standby: ColimaApp) -> Self {
        self.standby = Some(standby);
        self
    }

    pub fn check_and_failover(&mut self) -> Result<()> {
        match self.health_checker.run_checks() {
            HealthStatus::Healthy => Ok(()),
            HealthStatus::Degraded { .. } => {
                log::warn!("primary degraded, monitoring");
                Ok(())
            }
            HealthStatus::Unhealthy { reason } => {
                if let Some(mut standby) = self.standby.take() {
                    log::warn!("failing over to standby: {}", reason);
                    standby.start(&Config::default())?;
                    self.standby = Some(standby);
                    Ok(())
                } else {
                    Err(HaError::NoStandbyAvailable)
                }
            }
        }
    }
}
```

### 6.2 Automatic Recovery

```rust
// Rust: Auto-recovery with backoff
use std::time::Duration;

pub struct AutoRecovery {
    max_retries: usize,
    backoff: Duration,
}

impl AutoRecovery {
    pub fn recover<F>(&self, mut operation: F) -> Result<()>
    where
        F: FnMut() -> Result<()>,
    {
        let mut retries = 0;
        let mut delay = self.backoff;

        loop {
            match operation() {
                Ok(_) => return Ok(()),
                Err(e) if retries >= self.max_retries => {
                    return Err(e);
                }
                Err(_) => {
                    retries += 1;
                    log::warn!("operation failed, retrying in {:?}", delay);
                    std::thread::sleep(delay);
                    delay *= 2;  // Exponential backoff
                }
            }
        }
    }
}
```

---

## 7. Deployment Strategies

### 7.1 Infrastructure as Code

```yaml
# Terraform example
resource "colima_instance" "prod" {
  name     = "prod"
  cpu      = 16
  memory   = 32
  disk     = 500
  vm_type  = "vz"
  runtime  = "docker"

  kubernetes {
    enabled = true
    version = "v1.28.0"
  }

  network {
    address = true
    mode    = "bridged"
  }
}
```

### 7.2 CI/CD Integration

```yaml
# GitHub Actions
name: Deploy Colima

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: macos-13
    steps:
      - uses: actions/checkout@v3

      - name: Install Colima
        run: brew install colima

      - name: Start Colima
        run: |
          colima start prod \
            --cpu 4 \
            --memory 8 \
            --kubernetes

      - name: Deploy Application
        run: |
          kubectl apply -f k8s/
          kubectl rollout status deployment/app

      - name: Run Tests
        run: |
          kubectl run tests --image=tests:latest --rm -it
```

### 7.3 Blue-Green Deployment

```rust
// Rust: Blue-green deployment
pub struct BlueGreenDeploy {
    blue: ColimaApp,
    green: ColimaApp,
    active: ActiveInstance,
}

#[derive(Clone, Copy)]
pub enum ActiveInstance {
    Blue,
    Green,
}

impl BlueGreenDeploy {
    pub fn deploy_new_version(&mut self, config: &Config) -> Result<()> {
        // Deploy to inactive instance
        match self.active {
            ActiveInstance::Blue => {
                self.green.start(config)?;
                self.deploy_application(&self.green)?;
                self.active = ActiveInstance::Green;
            }
            ActiveInstance::Green => {
                self.blue.start(config)?;
                self.deploy_application(&self.blue)?;
                self.active = ActiveInstance::Blue;
            }
        }

        Ok(())
    }

    pub fn rollback(&mut self) -> Result<()> {
        self.active = match self.active {
            ActiveInstance::Blue => ActiveInstance::Green,
            ActiveInstance::Green => ActiveInstance::Blue,
        };
        Ok(())
    }
}
```

---

## Summary

| Topic | Key Points |
|-------|------------|
| **Performance** | CPU pinning, huge pages, gRPC forwarding, virtiofs |
| **Resources** | Quotas, limits, scheduling, monitoring |
| **Multi-Instance** | Profile isolation, orchestration, resource sharing |
| **Monitoring** | Prometheus metrics, structured logging, health checks |
| **Security** | VM isolation, network security, secrets management |
| **HA** | Active-passive, auto-recovery, failover |
| **Deployment** | IaC, CI/CD, blue-green deployments |

---

*Next: [Valtron Integration](05-valtron-integration.md)*
