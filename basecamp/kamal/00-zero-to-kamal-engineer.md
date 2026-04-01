---
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/basecamp/kamal
explored_at: 2026-03-29
prerequisites: Basic SSH, Docker fundamentals, Linux command-line
---

# Zero to Kamal Engineer - Complete Fundamentals

## Table of Contents

1. [What is Kamal?](#what-is-kamal)
2. [Core Concepts](#core-concepts)
3. [Installation](#installation)
4. [Configuration](#configuration)
5. [Your First Deploy](#your-first-deploy)
6. [Multi-Server Deploys](#multi-server-deploys)
7. [Roles and Workers](#roles-and-workers)
8. [Accessories](#accessories)
9. [Zero-Downtime Deploys](#zero-downtime-deploys)
10. [Troubleshooting](#troubleshooting)

## What is Kamal?

Kamal is a **zero-downtime deployment tool** for Dockerized applications. It uses SSH to deploy to any Linux server and a smart proxy to switch traffic atomically.

### The Problem Kamal Solves

Traditional deployment approaches:

**Manual Docker:**
```bash
# On each server (error-prone):
docker pull myapp:latest
docker stop myapp
docker rm myapp
docker run -d --name myapp -p 80:3000 myapp:latest
# Downtime during swap!
```

**Kubernetes:**
```yaml
# Complex YAML files, steep learning curve
apiVersion: apps/v1
kind: Deployment
metadata:
  name: myapp
spec:
  replicas: 3
  selector:
    matchLabels:
      app: myapp
  template:
    metadata:
      labels:
        app: myapp
    spec:
      containers:
      - name: myapp
        image: myapp:latest
        ports:
        - containerPort: 3000
# Plus: Services, Ingress, ConfigMaps, Secrets...
```

**Kamal:**
```bash
# One command, zero downtime
kamal deploy
```

### Key Features

| Feature | Description |
|---------|-------------|
| **Zero-Downtime** | Traffic switches atomically via proxy |
| **Multi-Server** | Deploy to 1 or 100 servers in parallel |
| **Role-Based** | Different server types (web, workers) |
| **No Vendor Lock-in** | Works on any SSH-accessible Linux |
| **Simple Config** | Single YAML file |
| **Built-in Proxy** | kamal-proxy for smart routing |

## Core Concepts

### 1. Service

A **Service** is your application name:
```yaml
service: myapp  # Used in container names, labels
```

### 2. Image

The Docker image to deploy:
```yaml
image: myorg/myapp:latest
```

### 3. Servers

List of servers to deploy to:
```yaml
servers:
  - 192.168.0.1
  - 192.168.0.2
  - 192.168.0.3
```

### 4. Roles

**Roles** define different server types:
```yaml
roles:
  web:
    servers:
      - 192.168.0.1
      - 192.168.0.2
    proxy: true  # Run kamal-proxy here
  workers:
    servers:
      - 192.168.0.3
    cmd: bundle exec sidekiq
```

### 5. kamal-proxy

A reverse proxy that:
- Routes traffic to correct container
- Health-checks new containers before switching
- Enables zero-downtime deploys

### 6. Version

Each deploy has a version (usually git SHA):
```
Container name: myapp-web-production-abc123
                                   ^^^^^^^
                                   version
```

## Installation

### Install Kamal

```bash
# macOS (Homebrew)
brew install basecamp/kamal/kamal

# RubyGems (any platform with Ruby)
gem install kamal

# Verify installation
kamal version
```

### Server Requirements

Each server needs:
- Linux (Ubuntu 20.04+ recommended)
- Docker installed
- SSH access
- Ruby (for Kamal CLI, runs locally)

### Docker Setup on Servers

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y docker.io
sudo usermod -aG docker $USER
# Logout and back in for group change to apply

# Verify
docker run hello-world
```

### SSH Setup

```bash
# Generate SSH key (if you don't have one)
ssh-keygen -t ed25519 -C "your@email.com"

# Copy to servers
ssh-copy-id user@192.168.0.1
ssh-copy-id user@192.168.0.2

# Test SSH
ssh user@192.168.0.1  # Should login without password
```

## Configuration

### Basic deploy.yml

```yaml
# deploy.yml
service: myapp
image: myorg/myapp

servers:
  - 192.168.0.1
  - 192.168.0.2

env:
  clear:
    RAILS_ENV: production
  secret:
    - RAILS_MASTER_KEY
    - DATABASE_URL

# SSH configuration
ssh:
  user: deploy
  port: 22

# Registry (Docker Hub, GitHub, etc.)
registry:
  username: myuser
  password:
    - DOCKER_REGISTRY_TOKEN
```

### With Roles

```yaml
service: myapp
image: myorg/myapp

servers:
  web:
    - 192.168.0.1
    - 192.168.0.2
  workers:
    - 192.168.0.3

roles:
  web:
    proxy: true
    env:
      clear:
        PORT: 3000
  workers:
    cmd: bundle exec sidekiq
    env:
      clear:
        CONCURRENT_PROCESSES: 5
```

### With Accessories

```yaml
service: myapp
image: myorg/myapp

servers:
  - 192.168.0.1

accessories:
  redis:
    image: redis:7-alpine
    port: 6379
    volumes:
      - redis_data:/data
  mysql:
    image: mysql:8
    env:
      MYSQL_ROOT_PASSWORD:
        - MYSQL_ROOT_PASSWORD
    volumes:
      - mysql_data:/var/lib/mysql
```

### Asset Handling (Rails)

```yaml
service: myapp
image: myorg/myapp

servers:
  - 192.168.0.1

assets:
  roles:
    - web
  path: /public/assets

# Kamal will:
# 1. Extract assets from container
# 2. Compress (gzip, zstd)
# 3. Serve via proxy for caching
```

## Your First Deploy

### Step 1: Create deploy.yml

```yaml
# deploy.yml
service: myapp
image: myorg/myapp

servers:
  - 192.168.0.1

env:
  clear:
    RAILS_ENV: production
  secret:
    - RAILS_MASTER_KEY
```

### Step 2: Setup Secrets

```bash
# Create .env file (gitignored!)
echo "RAILS_MASTER_KEY=$(cat config/master.key)" > .env

# Or use a secrets provider
# See: Secrets Management section
```

### Step 3: Deploy

```bash
# First deploy (setup + deploy)
kamal setup

# This will:
# 1. Create Docker network on servers
# 2. Boot kamal-proxy
# 3. Build and push image
# 4. Deploy application
# 5. Health check
```

### Step 4: Verify

```bash
# Check status
kamal app status

# View logs
kamal app logs --follow

# Check proxy
kamal proxy status
```

## Multi-Server Deploys

### Parallel Execution

Kamal deploys to all servers in parallel:

```bash
kamal deploy
# → SSH to 192.168.0.1 (parallel)
# → SSH to 192.168.0.2 (parallel)
# → SSH to 192.168.0.3 (parallel)
```

### Targeted Deploys

```bash
# Deploy to specific role
kamal deploy --roles web

# Deploy to specific host
kamal deploy --hosts 192.168.0.1

# Deploy to multiple hosts
kamal deploy --hosts 192.168.0.1,192.168.0.2

# Deploy to specific role AND host
kamal deploy --roles workers --hosts 192.168.0.3
```

### Rolling Deploys

For controlled rollouts:

```bash
# Deploy to 1 server first
kamal deploy --hosts 192.168.0.1

# Verify it works
kamal app status --hosts 192.168.0.1

# Deploy to rest
kamal deploy
```

## Roles and Workers

### Defining Roles

```yaml
service: myapp
image: myorg/myapp

servers:
  web:
    - 192.168.0.1
    - 192.168.0.2
  workers:
    - 192.168.0.3
    - 192.168.0.4
  api:
    - 192.168.0.5

roles:
  web:
    proxy: true
    cmd: bin/rails server
  workers:
    cmd: bundle exec sidekiq
  api:
    cmd: bin/rails server -p 3001
```

### Role-Specific Configuration

```yaml
roles:
  web:
    labels:
      traefik.http.routers.web.rule: Host(`example.com`)
    env:
      clear:
        PORT: 3000
    volumes:
      - /shared/data:/data

  workers:
    labels:
      traefik.enable: "false"  # No web traffic
    env:
      clear:
        CONCURRENT_PROCESSES: 5
        MAX_MEMORY: 2G
```

### Deploying to Roles

```bash
# Deploy only workers
kamal deploy --roles workers

# Reboot workers
kamal app reboot --roles workers

# Run command on workers
kamal app exec --roles workers "bundle exec sidekiq jobs"
```

## Accessories

Accessory services (Redis, MySQL, etc.):

```yaml
accessories:
  redis:
    image: redis:7-alpine
    port: 6379
    volumes:
      - redis_data:/data
    healthcheck:
      test: redis-cli ping
      interval: 10s

  mysql:
    image: mysql:8
    env:
      MYSQL_ROOT_PASSWORD:
        - MYSQL_ROOT_PASSWORD
      MYSQL_DATABASE: myapp
    volumes:
      - mysql_data:/var/lib/mysql
      - ./config/mysql.cnf:/etc/mysql/conf.d/custom.cnf
```

### Deploying Accessories

```bash
# Deploy all accessories
kamal accessory deploy

# Deploy specific accessory
kamal accessory deploy redis

# Reboot accessory
kamal accessory reboot redis

# View logs
kamal accessory logs redis
```

## Zero-Downtime Deploys

### How It Works

```
┌─────────────────────────────────────────────────────────────┐
│ Phase 1: Start New Container                                │
│ ─────────────────────────────────                           │
│ 1. Pull new image                                           │
│ 2. Start new container (abc123)                             │
│ 3. Register with proxy                                      │
│ 4. Proxy health-checks new container                        │
│                                                             │
│ Current traffic: ──────→ old-container (def456)            │
│ New traffic:   ─ ─ ─ ─ → new-container (abc123) [checking] │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ Phase 2: Switch Traffic                                     │
│ ───────────────────                                         │
│ 5. Health check passes                                      │
│ 6. Proxy atomically switches traffic                        │
│ 7. Stop old container                                       │
│ 8. Remove old container                                     │
│                                                             │
│ All traffic: ──────────→ new-container (abc123)            │
│ Old: [stopped/removed]                                      │
└─────────────────────────────────────────────────────────────┘
```

### Deployment Commands

```bash
# Full deploy (zero-downtime)
kamal deploy

# Deploy with specific version
kamal deploy -v abc123

# Reboot (stop all, start fresh - has downtime)
kamal app reboot

# Rolling reboot (zero-downtime)
kamal app reboot --rolling
```

### Health Checks

Add to your Dockerfile:

```dockerfile
HEALTHCHECK --interval=5s --timeout=3s --start-period=30s --retries=3 \
  CMD curl -f http://localhost:3000/up || exit 1
```

Or in deploy.yml:

```yaml
healthcheck:
  cmd: curl -f http://localhost:3000/up
  interval: 5s
  timeout: 3s
  retries: 3
```

## Troubleshooting

### Common Issues

**SSH Connection Failed:**
```bash
# Test SSH manually
ssh user@192.168.0.1

# Check SSH key permissions
chmod 600 ~/.ssh/id_ed25519

# Add key to agent
ssh-add ~/.ssh/id_ed25519
```

**Docker Permission Denied:**
```bash
# Add user to docker group
sudo usermod -aG docker $USER
# Logout and back in
```

**Container Won't Start:**
```bash
# Check logs
kamal app logs --lines 100

# Check container status
kamal app status

# SSH to server and inspect
ssh user@192.168.0.1
docker ps -a
docker logs myapp-web-production-abc123
```

**Proxy Issues:**
```bash
# Reboot proxy
kamal proxy reboot

# Check proxy logs
kamal proxy logs

# Verify proxy is running
kamal proxy status
```

### Debug Mode

```bash
# Verbose output
kamal deploy --verbose

# Skip health checks (for debugging)
kamal deploy --skip-healthcheck

# Dry run (see commands without executing)
kamal deploy --dry-run
```

### Rollback

```bash
# Rollback to previous version
kamal rollback

# Rollback to specific version
kamal rollback -v abc123

# Manual rollback
kamal app stop
kamal app start --version abc123
```

## CLI Reference

```bash
# Deploy
kamal deploy
kamal deploy -v abc123
kamal deploy --roles web

# App Management
kamal app status
kamal app logs
kamal app logs --follow
kamal app reboot
kamal app reboot --rolling

# Execute Commands
kamal app exec "rails db:migrate"
kamal app exec --roles workers "sidekiq jobs"

# Build
kamal build
kamal build --push

# Accessories
kamal accessory deploy redis
kamal accessory logs redis
kamal accessory reboot redis

# Proxy
kamal proxy boot
kamal proxy reboot
kamal proxy logs

# Cleanup
kamal prune
kamal prune --dry-run
```

---

**Next Steps:**
- [01-kamal-deployment-exploration.md](./01-kamal-deployment-exploration.md) - Full architecture
- [02-proxy-deep-dive.md](./02-proxy-deep-dive.md) - kamal-proxy internals
- [03-secrets-management-deep-dive.md](./03-secrets-management-deep-dive.md) - Secrets adapters
