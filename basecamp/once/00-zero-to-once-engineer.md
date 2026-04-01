---
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/basecamp/once
explored_at: 2026-03-29
prerequisites: Basic Docker knowledge, Command-line familiarity
---

# Zero to ONCE Engineer - Complete Fundamentals

## Table of Contents

1. [What is ONCE?](#what-is-once)
2. [Core Concepts](#core-concepts)
3. [Installation](#installation)
4. [Your First Application](#your-first-application)
5. [Application Lifecycle](#application-lifecycle)
6. [Storage and Volumes](#storage-and-volumes)
7. [Backups](#backups)
8. [Monitoring](#monitoring)
9. [Architecture Deep Dive](#architecture-deep-dive)

## What is ONCE?

ONCE is a **self-hosting platform** that makes deploying Docker applications as simple as running a single command. Think of it as a personal app store for your own servers.

### The Problem ONCE Solves

Traditional self-hosting is complex:
```
1. Install Docker
2. Create Docker network
3. Configure reverse proxy (nginx/traefik)
4. Set up TLS certificates
5. Create persistent volumes
6. Configure environment variables
7. Write docker-compose.yml
8. Set up automatic updates
9. Configure backups
10. Monitor health and logs
```

With ONCE:
```
curl https://get.once.com | sh
# Pick an app, enter hostname, done!
```

### Key Features

| Feature | Description |
|---------|-------------|
| **TUI Dashboard** | Beautiful terminal interface for managing apps |
| **Auto-Updates** | Applications update themselves automatically |
| **Auto-Backups** | Scheduled backups with one-click restore |
| **Built-in Proxy** | Automatic HTTPS with Let's Encrypt |
| **Persistent Storage** | Docker volumes survive app reinstalls |
| **Multi-App** | Run multiple apps on same server |

## Core Concepts

### 1. Namespace

A **Namespace** is an isolated environment for your applications:

```
once (default namespace)
├── proxy (kamal-proxy)
├── app-writebook
├── app-pitch
└── app-present
```

Think of it as a dedicated Docker network with management tooling.

### 2. Application

An **Application** is a deployed Docker container with:
- Persistent storage volume
- Environment configuration
- Health monitoring
- Auto-update settings

### 3. Proxy

The **Proxy** (kamal-proxy) routes incoming requests to the correct application:

```
Internet → Port 80/443 → kamal-proxy → app-container
```

### 4. Volume

A **Volume** is persistent storage that survives container restarts:

```
Docker Volume: once-writebook-storage
    └── mounted to: /storage inside container
```

## Installation

### One-Line Install

```bash
curl https://get.once.com | sh
```

This:
1. Downloads the `once` binary for your platform
2. Installs Docker (if not present)
3. Registers the background service
4. Launches the TUI

### Manual Installation

```bash
# 1. Install Docker
# Ubuntu/Debian:
sudo apt-get install docker.io

# macOS:
brew install --cask docker

# 2. Download once binary
# From GitHub releases
wget https://github.com/basecamp/once/releases/latest/download/once-linux-amd64
chmod +x once-linux-amd64
sudo mv once-linux-amd64 /usr/local/bin/once

# 3. Install background service
sudo once background install

# 4. Launch TUI
once
```

### Verification

```bash
# Check once version
once version

# Check background service
sudo systemctl status once

# Check Docker network
docker network ls  # Should show 'once'
```

## Your First Application

### Step 1: Launch ONCE

```bash
once
```

You'll see the dashboard:
```
┌─ ONCE Dashboard ─────────────────────────────────────────────┐
│                                                              │
│  No applications installed.                                  │
│                                                              │
│  Press [i] to install your first application.                │
│                                                              │
│  [i] Install  [r] Refresh  [q] Quit                          │
└──────────────────────────────────────────────────────────────┘
```

### Step 2: Choose Application

Press `i` to install. You'll see:
```
Select Application:
  → Writebook (note-taking)
    Pitch (presentations)
    Present (slides)
    Custom Docker Image...
```

### Step 3: Enter Hostname

```
Enter hostname: notes.example.com

DNS Requirements:
  - Create A record: notes.example.com → YOUR_SERVER_IP
  - Wait for DNS propagation (1-5 minutes)
```

### Step 4: Installation

ONCE will:
1. Pull Docker image
2. Create persistent volume
3. Generate secrets (SECRET_KEY_BASE, VAPID keys)
4. Start container
5. Register with proxy
6. Health check

```
Installing Writebook...
  ✓ Pulled image: 37signals/writebook:latest
  ✓ Created volume: once-writebook-storage
  ✓ Started container: once-app-writebook-abc123
  ✓ Registered with proxy
  ✓ Health check passed

Installation complete!
Visit: https://notes.example.com
```

## Application Lifecycle

### Starting/Stopping

```bash
# Via TUI: Select app → press 'a' → choose action

# Via CLI:
once list                    # List all apps
once stop writebook          # Stop app
once start writebook         # Start app
once restart writebook       # Restart app
```

### Updating

```bash
# Manual update
once update writebook

# Auto-update (configured per-app)
# Settings → Auto-Update → Enable
```

### Removing

```bash
# Remove app, keep data
once remove writebook

# Remove app and data
once remove writebook --purge
```

## Storage and Volumes

### Volume Structure

```
once-{app-name}-storage/
├── production.sqlite3    # SQLite database
├── storage/              # File uploads
│   ├── attachments/
│   └── avatars/
└── logs/                 # Application logs
```

### Accessing Volume Data

```bash
# Via TUI: Select app → Settings → Browse Storage

# Via Docker:
docker run --rm \
  -v once-writebook-storage:/storage \
  alpine ls -la /storage
```

### Volume Backup

```bash
# Manual backup
once backup writebook --output /backups/writebook-$(date +%Y%m%d).tar.gz

# Auto-backup (configured per-app)
# Settings → Backup Location → /backups
# Settings → Auto-Backup → Enable
```

## Backups

### Backup Contents

A backup archive contains:
```
backup-2024-01-01.tar.gz
├── app-settings.json     # App configuration
├── vol-settings.json     # Volume secrets
└── data/                 # All volume data
    ├── production.sqlite3
    └── storage/
```

### Restore Process

```bash
# Restore from backup
once restore /backups/writebook-2024-01-01.tar.gz

# ONCE will:
# 1. Parse backup archive
# 2. Create new volume with saved secrets
# 3. Restore data files
# 4. Deploy fresh container
# 5. Register with proxy
```

### Backup Schedule

| Frequency | When |
|-----------|------|
| Daily | Every 24 hours |
| Weekly | Every 7 days |
| Monthly | Every 30 days |

## Monitoring

### Dashboard Status

```
┌─ Application Status ─────────────────────────────────────────┐
│                                                              │
│  Writebook                              ✓ Running           │
│  Host: notes.example.com                                    │
│  Uptime: 2 days, 4 hours                                    │
│  Memory: 128MB / 512MB                                      │
│  CPU: 2%                                                    │
│                                                              │
│  [l] Logs  [s] Settings  [a] Actions                        │
└──────────────────────────────────────────────────────────────┘
```

### Viewing Logs

```bash
# Via TUI: Select app → press 'l'

# Via CLI:
once logs writebook            # Recent logs
once logs writebook --follow   # Live tail
once logs writebook --lines 100  # Last 100 lines
```

### Health Checks

ONCE health-checks applications:
- `/up` endpoint must return HTTP 200
- Checked every 30 seconds
- Unhealthy apps shown in dashboard

## Architecture Deep Dive

### Component Overview

```
┌─────────────────────────────────────────────────────────────┐
│                         ONCE Stack                           │
│                                                             │
│  ┌────────────────┐                                        │
│  │ once CLI       │ ← User commands                        │
│  └───────┬────────┘                                        │
│          │                                                  │
│  ┌───────▼────────┐                                        │
│  │ Background     │ ← Auto-update, auto-backup            │
│  │ Runner         │   (runs every 5 minutes)               │
│  └───────┬────────┘                                        │
│          │                                                  │
│  ┌───────▼────────────────────────────────┐                │
│  │ Docker Namespace (once)                │                │
│  │                                        │                │
│  │  ┌─────────────┐  ┌──────────────┐   │                │
│  │  │ kamal-proxy │  │ appcontainer │   │                │
│  │  │ :80, :443   │  │ :3000        │   │                │
│  │  └──────┬──────┘  └──────┬───────┘   │                │
│  │         │                │           │                │
│  │  ┌──────▼────────────────▼───────┐   │                │
│  │  │ Docker Volumes                │   │                │
│  │  │ - once-proxy-config           │   │                │
│  │  │ - once-{app}-storage          │   │                │
│  │  └───────────────────────────────┘   │                │
│  └──────────────────────────────────────┘                │
└───────────────────────────────────────────────────────────┘
```

### Network Flow

```
User Request Flow:
1. User → https://notes.example.com
2. DNS → YOUR_SERVER_IP:443
3. kamal-proxy receives request
4. Proxy looks up hostname → app container
5. Proxy forwards to container:3000
6. Container responds
7. Proxy returns response to user
```

### Deployment Flow

```
once deploy writebook:
1. Pull image: docker pull 37signals/writebook:latest
2. Create volume: docker volume create once-writebook-storage
3. Generate secrets: SECRET_KEY_BASE, VAPID_*
4. Create container: docker create --name once-app-writebook-abc123
5. Start container: docker start once-app-writebook-abc123
6. Register proxy: Add routing rule to kamal-proxy
7. Health check: GET https://notes.example.com/up
8. Remove old: docker stop|remove once-app-writebook-old
```

## Environment Variables

ONCE injects these into containers:

| Variable | Purpose | Example |
|----------|---------|---------|
| `SECRET_KEY_BASE` | Crypto signing | 64-char hex string |
| `VAPID_PUBLIC_KEY` | WebPush public | Base64 key |
| `VAPID_PRIVATE_KEY` | WebPush private | Base64 key |
| `DISABLE_SSL` | SSL indicator | `true` if no TLS |
| `SMTP_ADDRESS` | Email server | `smtp.example.com` |
| `SMTP_PORT` | Email port | `587` |
| `SMTP_USERNAME` | Email user | `noreply@example.com` |
| `SMTP_PASSWORD` | Email pass | `secret` |
| `MAILER_FROM_ADDRESS` | Email from | `noreply@example.com` |
| `NUM_CPUS` | CPU limit | `2` |

## Production Checklist

Before deploying to production:

- [ ] DNS configured with A record
- [ ] TLS/SSL enabled (ports 80/443 open)
- [ ] Backup location configured
- [ ] Auto-backup enabled
- [ ] Resource limits set (memory, CPU)
- [ ] Monitoring dashboard checked
- [ ] Logs reviewed

---

**Next Steps:**
- [01-once-platform-exploration.md](./01-once-platform-exploration.md) - Full architecture
- [02-tui-dashboard-deep-dive.md](./02-tui-dashboard-deep-dive.md) - TUI implementation
- [03-docker-orchestration-deep-dive.md](./03-docker-orchestration-deep-dive.md) - Container management
