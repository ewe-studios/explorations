---
location: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/once
repository: git@github.com:basecamp/once.git
explored_at: 2026-03-29
language: Go
category: Self-Hosting Platform
---

# ONCE Platform - Exploration

## Overview

ONCE is a **self-hosting platform for Docker-based web applications** designed to make installing and managing applications as simple as possible. It provides automatic updates, backups, and system monitoring through both a TUI (Terminal User Interface) dashboard and CLI commands.

### Key Value Proposition

- **Zero-Config Deployment**: Install apps with `curl https://get.once.com | sh`
- **Automatic Operations**: Auto-updates, scheduled backups, health monitoring
- **Unified Management**: TUI dashboard + CLI for all operations
- **Multi-Platform**: Runs on Linux, macOS, Raspberry Pi, cloud VPS, bare metal
- **Persistent Storage**: Docker volumes with automatic backup/restore
- **Reverse Proxy**: Built-in proxy with automatic TLS/SSL management

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        ONCE Platform                            │
│                                                                 │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │   CLI (once)    │  │   TUI Dashboard │  │  Background     │ │
│  │                 │  │   (Bubble Tea)  │  │  Runner         │ │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘ │
│           │                    │                    │           │
│           └────────────────────┼────────────────────┘           │
│                                │                                │
│                    ┌───────────▼───────────┐                   │
│                    │   Docker Namespace    │                   │
│                    │   (bridge network)    │                   │
│                    └───────────┬───────────┘                   │
│                                │                                │
│         ┌──────────────────────┼──────────────────────┐        │
│         │                      │                      │        │
│  ┌──────▼──────┐      ┌───────▼───────┐     ┌───────▼──────┐ │
│  │ kamal-proxy │      │  App Container│     │ App Container│ │
│  │ (reverse    │      │  (once-app-*) │     │ (once-app-*) │ │
│  │  proxy)     │      │               │     │              │ │
│  └─────────────┘      └───────────────┘     └──────────────┘ │
│         │                    │                      │         │
│         └────────────────────┴──────────────────────┘         │
│                                │                                │
│                    ┌───────────▼───────────┐                   │
│                    │   Docker Volumes      │                   │
│                    │   /storage persistent │                   │
│                    └───────────────────────┘                   │
└─────────────────────────────────────────────────────────────────┘
```

## Monorepo Structure

```
once/
├── cmd/
│   └── once/                 # Main CLI binary entry point
├── installer/                # Auto-installer script
│   ├── main.go              # Installer logic
│   └── uninstall.sh         # Uninstall script
├── internal/
│   ├── background/          # Background runner (auto-update/backup)
│   │   └── runner.go        # Periodic task scheduler
│   ├── command/             # CLI command handlers
│   │   ├── root.go          # Root command (Cobra)
│   │   ├── deploy.go        # Deploy applications
│   │   ├── backup.go        # Backup operations
│   │   ├── remove.go        # Remove applications
│   │   ├── start.go/stop.go # Container lifecycle
│   │   ├── update.go        # Update applications
│   │   └── ...
│   ├── docker/              # Docker integration layer
│   │   ├── namespace.go     # Docker network namespace management
│   │   ├── application.go   # Application lifecycle (deploy/update)
│   │   ├── application_settings.go  # App configuration
│   │   ├── application_volume.go    # Volume management
│   │   ├── application_backup.go    # Backup/restore
│   │   ├── container.go     # Container operations
│   │   ├── proxy.go         # kamal-proxy integration
│   │   └── volume.go        # Docker volume handling
│   ├── ui/                  # TUI implementation
│   │   ├── dashboard/       # Main dashboard screen
│   │   ├── application/     # Application detail views
│   │   ├── settings/        # Configuration screens
│   │   ├── actions/         # Action menus
│   │   └── components/      # Reusable UI components
│   ├── mouse/               # Mouse event handling for TUI
│   ├── fsutil/              # Filesystem utilities
│   ├── logging/             # Structured logging (slog)
│   ├── metrics/             # System metrics collection
│   ├── userstats/           # Anonymous usage statistics
│   ├── service/             # System service registration
│   ├── system/              # OS-level utilities
│   └── version/             # Version info and self-update
├── integration/             # Integration tests
├── go.mod                   # Go module definition
├── Makefile                 # Build targets
└── README.md                # Documentation
```

## Core Concepts

### 1. Namespace

A **Namespace** is an isolated Docker environment for ONCE applications:

```go
type Namespace struct {
    name         string
    client       *client.Client  // Docker client
    proxy        *Proxy          // Reverse proxy
    applications []*Application  // Managed apps
}
```

**Key responsibilities:**
- Creates isolated Docker bridge network (`once` by default)
- Manages `kamal-proxy` container for routing
- Discovers and tracks running applications via container labels
- Coordinates backup/restore operations

### 2. Application

An **Application** represents a deployed Docker container:

```go
type Application struct {
    namespace    *Namespace
    Settings     ApplicationSettings
    Running      bool
    RunningSince time.Time
}

type ApplicationSettings struct {
    Name       string
    Image      string         // Docker image reference
    Host       string         // Hostname for routing
    AutoUpdate bool           // Enable auto-updates
    Backup     BackupSettings // Backup configuration
    Resources  ResourceSettings // CPU/memory limits
}
```

**Deployment flow:**
```go
func (a *Application) Deploy(ctx context.Context, progress DeployProgressCallback) error {
    // 1. Pull Docker image
    if _, err := a.pullImage(ctx, progress); err != nil {
        return err
    }

    // 2. Create/get persistent volume
    vol, err := a.Volume(ctx)
    if err != nil {
        return fmt.Errorf("getting volume: %w", err)
    }

    // 3. Deploy container with volume mounts
    return a.deployWithVolume(ctx, vol, progress)
}
```

### 3. Container Deployment

The deployment process creates containers with specific configuration:

```go
func (a *Application) deployWithVolume(...) error {
    // Generate random container ID for zero-downtime deploys
    id, _ := ContainerRandomID()
    containerName := fmt.Sprintf("%s-app-%s-%s", namespace, appName, id)

    // Build environment variables
    env := a.Settings.BuildEnv(vol.Settings)
    // Includes: SECRET_KEY_BASE, VAPID_*, SMTP_*, DISABLE_SSL, NUM_CPUS

    // Configure container
    hostConfig := &container.HostConfig{
        RestartPolicy: {Name: "always"},
        LogConfig:     ContainerLogConfig(),
        Mounts:        a.volumeMounts(vol),  // /storage and /rails/storage
        Resources: container.Resources{
            Memory:   int64(memoryMB) * 1024 * 1024,
            NanoCPUs: int64(cpus) * 1e9,
        },
    }

    // Create and start container
    resp, _ := client.ContainerCreate(ctx, config, hostConfig, networking, nil, containerName)
    client.ContainerStart(ctx, resp.ID, container.StartOptions{})

    // Register with proxy for routing
    a.namespace.Proxy().Deploy(ctx, DeployOptions{
        AppName: appName,
        Target:  containerID[:12],
        Host:    host,
        TLS:     tlsEnabled,
    })

    // Remove old containers
    a.removeContainersExcept(ctx, containerName)

    return nil
}
```

### 4. Volume Management

Persistent data is stored in Docker volumes:

```go
type ApplicationVolume struct {
    Name     string
    Settings ApplicationVolumeSettings
}

type ApplicationVolumeSettings struct {
    SecretKeyBase   string  // Cryptographic signing key
    VAPIDPublicKey  string  // WebPush public key
    VAPIDPrivateKey string  // WebPush private key
}
```

**Volume mount points:**
- `/storage` - Primary data directory
- `/rails/storage` - Rails convention (same volume, different path)

### 5. Background Runner

Automatic operations run on a schedule:

```go
type Runner struct {
    namespace string
}

func (r *Runner) Run(ctx context.Context) error {
    scraper := userstats.NewScraper(r.namespace)
    go scraper.Run(ctx)  // Anonymous usage stats

    ticker := time.NewTicker(CheckInterval)  // 5 minutes
    defer ticker.Stop()

    for {
        select {
        case <-ctx.Done():
            return nil
        case <-ticker.C:
            r.check(ctx)  // Check for updates, backups
        }
    }
}

func (r *Runner) check(ctx context.Context) {
    // Check self-update
    if state.UpdateDue(appName) && app.Settings.AutoUpdate {
        app.Update(ctx, nil)  // Pull new image, redeploy
    }

    // Check backups
    if state.BackupDue(appName) && app.Settings.Backup.AutoBackup {
        app.Backup(ctx)  // Create backup archive
        app.TrimBackups()  // Keep only recent backups
    }
}
```

### 6. Backup System

Backups are compressed tar archives:

```go
func (a *Application) Backup(ctx context.Context) error {
    // 1. Call pre-backup hook if exists
    if hasPreBackupHook {
        exec(container, "/hooks/pre-backup")
    } else {
        pause(container)  // Ensure consistent backup
    }

    // 2. Extract volume data
    reader, _ := client.CopyFromContainer(ctx, containerID, "/storage")

    // 3. Create tar archive
    archive := tar.gz{
        "app-settings.json":  marshal(a.Settings),
        "vol-settings.json":  marshal(vol.Settings),
        "data/":              volumeData,
    }

    // 4. Write to backup location
    os.WriteFile(backupPath, archive.Bytes(), 0644)

    // 5. Unpause container
    unpause(container)

    return nil
}
```

**Restore flow:**
```go
func (n *Namespace) Restore(ctx context.Context, r io.Reader) (*Application, error) {
    // Parse backup archive
    appSettings, volSettings, volumeData := parseBackup(r)

    // Generate unique name
    name := UniqueName(NameFromImageRef(appSettings.Image))

    // Create volume with restored settings
    vol := CreateVolume(ctx, namespace, name, volSettings)

    // Restore volume data
    vol.Restore(ctx, volumeData)

    // Deploy application
    app := NewApplication(namespace, appSettings)
    app.Deploy(ctx, nil)

    return app, nil
}
```

## Proxy Integration (kamal-proxy)

ONCE uses `kamal-proxy` as its reverse proxy for routing:

```go
type Proxy struct {
    namespace *Namespace
    Settings  *ProxySettings
}

type ProxySettings struct {
    HTTPPort  int  // Default: 80
    HTTPSPort int  // Default: 443
    TLSEnabled bool
}

func (p *Proxy) Deploy(ctx context.Context, opts DeployOptions) error {
    // Register target container with proxy
    // Proxy handles health checks, TLS termination, request routing
}
```

**Proxy container setup:**
```go
func (p *Proxy) Boot(ctx context.Context, settings ProxySettings) error {
    // Pull proxy image
    client.ImagePull(ctx, "basecamp/kamal-proxy:latest", ...)

    // Create proxy container
    config := &container.Config{
        Image: "basecamp/kamal-proxy",
        ExposedPorts: nat.PortSet{
            "80/tcp":  {},
            "443/tcp": {},
        },
    }

    hostConfig := &container.HostConfig{
        PortBindings: nat.PortMap{
            "80/tcp":  {{HostPort: strconv.Itoa(settings.HTTPPort)}},
            "443/tcp": {{HostPort: strconv.Itoa(settings.HTTPSPort)}},
        },
        NetworkMode: container.NetworkMode(p.namespace.name),
    }

    client.ContainerCreate(ctx, config, hostConfig, nil, nil, proxyName)
    client.ContainerStart(ctx, proxyID, container.StartOptions{})

    return nil
}
```

## TUI Dashboard

The terminal UI is built with [Bubble Tea](https://github.com/charmbracelet/bubbletea):

```
┌─ ONCE Dashboard ─────────────────────────────────────────────┐
│                                                              │
│  Applications                                                │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ ✓ writebook.once.app          Running (2h ago)        │ │
│  │ ○ pitch.once.app              Stopped                 │ │
│  │ ○ present.once.app            Stopped                 │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  [s] Settings  [a] Actions  [r] Refresh  [q] Quit           │
└──────────────────────────────────────────────────────────────┘
```

**Key screens:**
- **Dashboard**: List of applications with status
- **Application Detail**: Logs, resource usage, actions
- **Settings Menu**: Hostname, backup location, email, fork image
- **Actions Menu**: Start, Stop, Restart, Remove, Backup, Restore

## Environment Variables

ONCE injects these environment variables into containers:

| Variable | Purpose |
|----------|---------|
| `SECRET_KEY_BASE` | Unique cryptographic signing key (per-app) |
| `VAPID_PUBLIC_KEY` | WebPush public key for notifications |
| `VAPID_PRIVATE_KEY` | WebPush private key for notifications |
| `DISABLE_SSL` | `true` if running without TLS |
| `SMTP_ADDRESS` | SMTP server for email |
| `SMTP_PORT` | SMTP port |
| `SMTP_USERNAME` | SMTP username |
| `SMTP_PASSWORD` | SMTP password |
| `MAILER_FROM_ADDRESS` | Default From address |
| `NUM_CPUS` | Allowed CPU count (from cgroup quota) |

## Hook Scripts

Applications can implement optional hooks:

| Hook | Purpose |
|------|---------|
| `/hooks/pre-backup` | Prepare for backup (e.g., SQLite WAL checkpoint) |
| `/hooks/post-restore` | Cleanup after restore (e.g., move files back) |

**Example pre-backup hook for SQLite:**
```bash
#!/bin/bash
# /hooks/pre-backup
sqlite3 /storage/production.sqlite3 ".backup '/storage/backup.sqlite3'"
```

## Production Considerations

### Scaling

- Each application runs in its own container with resource limits
- Proxy handles load balancing across multiple instances if needed
- Namespace isolation allows multiple ONCE installations on same host

### Monitoring

```go
// System metrics collection
type Metrics struct {
    CPUUsage    float64
    MemoryUsage uint64
    DiskUsage   uint64
    NetworkRx   uint64
    NetworkTx   uint64
}
```

### Security

- Docker network isolation (bridge network per namespace)
- TLS termination at proxy
- Secrets stored in volume (not environment)
- Resource limits prevent runaway containers

### Cost

- Free: ONCE itself is MIT licensed
- Infrastructure: Only pay for underlying hosting (VPS, cloud)
- Efficient: Multiple apps share single Docker installation

## Related Deep Dives

- [00-zero-to-once-engineer.md](./00-zero-to-once-engineer.md) - Fundamentals
- [02-tui-dashboard-deep-dive.md](./02-tui-dashboard-deep-dive.md) - Bubble Tea implementation
- [03-docker-orchestration-deep-dive.md](./03-docker-orchestration-deep-dive.md) - Container management
- [04-backup-restore-deep-dive.md](./04-backup-restore-deep-dive.md) - Backup architecture
- [rust-revision.md](./rust-revision.md) - Rust implementation considerations
- [production-grade.md](./production-grade.md) - Production deployment guide
