---
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/superfly
source: github.com/superfly (flyctl, corrosion, libkrunfw, metrics, tokenizer)
explored_at: 2026-04-05
prerequisites: Basic Docker knowledge, CLI familiarity, Cloud concepts
---

# Zero to Fly.io Engineer - Complete Fundamentals

## Table of Contents

1. [What is Fly.io?](#what-is-flyio)
2. [Core Concepts](#core-concepts)
3. [Getting Started](#getting-started)
4. [Your First Deployment](#your-first-deployment)
5. [Application Configuration](#application-configuration)
6. [Global Deployment](#global-deployment)
7. [Persistent Storage](#persistent-storage)
8. [Managed Databases](#managed-databases)
9. [Monitoring and Debugging](#monitoring-and-debugging)
10. [Architecture Deep Dive](#architecture-deep-dive)

## What is Fly.io?

**Fly.io** is an edge cloud platform that enables developers to deploy applications in microVMs close to users worldwide. Unlike traditional cloud providers that force you to pick a single region, Fly.io lets you run your application in 80+ locations globally with minimal configuration.

### The Problem Fly.io Solves

Traditional cloud deployment:
```
1. Pick a region (us-east-1, eu-west-1, etc.)
2. Deploy application
3. Users far from region experience high latency
4. To go global, you need:
   - Multiple deployments
   - Load balancer configuration
   - DNS routing setup
   - Database replication
   - Complex orchestration
```

Fly.io deployment:
```
1. Write fly.toml config
2. Run: fly deploy
3. Application runs in 35+ regions automatically
4. Users connect to nearest edge location
5. Done.
```

### Key Features

| Feature | Description |
|---------|-------------|
| **Edge Computing** | Deploy to 80+ locations worldwide |
| **MicroVMs** | Firecracker-based isolation (faster than containers, lighter than VMs) |
| **Global Anycast** | Single IP address routes to nearest edge |
| **Persistent Volumes** | Stateful applications with replicated storage |
| **Managed Databases** | PostgreSQL and Redis with automatic replication |
| **Automatic Scaling** | Scale to zero when idle, scale up on demand |
| **WireGuard Mesh** | Private network between all applications |

### Fly.io vs Alternatives

| Platform | Best For | Limitations |
|----------|----------|-------------|
| **Fly.io** | Full applications at edge | Not for serverless functions |
| **Heroku** | Simple deployments | Single region, slower |
| **Vercel** | Frontend/Next.js | Limited backend support |
| **Cloudflare Workers** | Serverless functions | Limited runtime, no full apps |
| **AWS/Azure** | Enterprise workloads | Complex, high latency if single region |

## Core Concepts

### 1. flyctl - The CLI Tool

`flyctl` (pronounced "fly control") is the command-line interface for interacting with Fly.io:

```bash
# Install flyctl
curl -L https://fly.io/install.sh | sh

# Authenticate
fly auth login

# Deploy application
fly deploy

# View logs
fly logs

# Open SSH to running VM
fly ssh console
```

### 2. fly.toml - Configuration File

The `fly.toml` file defines your application:

```toml
app = "my-app"
primary_region = "iad"

[build]
  dockerfile = "Dockerfile"

[env]
  PORT = "8080"
  LOG_LEVEL = "info"

[http_service]
  internal_port = 8080
  force_https = true

[[vm]]
  cpu_kind = "shared"
  cpus = 1
  memory_mb = 512
```

### 3. Machines (MicroVMs)

Fly.io runs your application in **Firecracker microVMs**:

```
Traditional Container:
┌─────────────────────────────┐
│     Host OS Kernel          │
├─────────────────────────────┤
│  Container  │  Container    │
│  (isolated) │  (isolated)   │
└─────────────────────────────┘

Fly.io MicroVM:
┌─────────────────────────────┐
│     Host OS                 │
├─────────────────────────────┤
│  MicroVM    │  MicroVM     │
│  (kernel)   │  (kernel)    │
│  Container  │  Container   │
└─────────────────────────────┘
```

**Benefits:**
- Stronger isolation (separate kernel)
- Faster cold starts than containers (~100ms)
- Lower overhead than traditional VMs
- Support for any language/runtime

### 4. Regions

Fly.io has 80+ edge locations:

```
Major regions:
- iad: Washington, D.C. (US East)
- lax: Los Angeles (US West)
- ord: Chicago (US Central)
- fra: Frankfurt (Europe)
- lhr: London (UK)
- cdg: Paris (France)
- nrt: Tokyo (Japan)
- syd: Sydney (Australia)

See all: fly platform regions
```

### 5. Services

Services define how traffic reaches your application:

```toml
[http_service]
  internal_port = 8080  # Your app listens on this
  force_https = true

  [http_service.concurrency]
    type = "connections"
    hard_limit = 100
    soft_limit = 80
```

### 6. Volumes

Persistent storage that survives restarts:

```toml
[[mounts]]
  source = "data"      # Volume name
  destination = "/data"  # Where to mount
  initial_size = "10gb"
```

## Getting Started

### Step 1: Install flyctl

```bash
# macOS / Linux
curl -L https://fly.io/install.sh | sh

# Add to PATH (follow installer instructions)
# Usually: export FLYCTL_INSTALL="$HOME/.fly"
#          export PATH="$FLYCTL_INSTALL/bin:$PATH"

# Verify installation
fly version

# Output: fly v0.1.xxx
```

### Step 2: Authenticate

```bash
fly auth login

# Opens browser for authentication
# Creates ~/.config/fly/config.yml with token
```

### Step 3: Create Application

```bash
# Create new app
fly launch

# You'll be prompted:
# 1. App name (or auto-generated)
# 2. Select region (default: iad)
# 3. Add PostgreSQL? (y/n)
# 4. Add Redis? (y/n)
# 5. Deploy now? (y/n)
```

### Step 4: View Your App

```bash
# Open in browser
fly open

# View status
fly status

# View logs
fly logs
```

## Your First Deployment

### Option A: Deploy Existing Docker Image

```bash
# Create app from existing image
fly launch --image nginx:latest

# Or update existing app
fly config set image=nginx:latest

# Deploy
fly deploy
```

### Option B: Deploy from Source

```bash
# Create directory
mkdir my-app && cd my-app

# Initialize (generates fly.toml)
fly launch

# For Node.js example:
cat > package.json <<EOF
{
  "name": "my-app",
  "version": "1.0.0",
  "scripts": {
    "start": "node server.js"
  }
}
EOF

cat > server.js <<EOF
const http = require('http');
const port = process.env.PORT || 3000;

const server = http.createServer((req, res) => {
  res.statusCode = 200;
  res.setHeader('Content-Type', 'text/plain');
  res.end('Hello from Fly.io!');
});

server.listen(port, () => {
  console.log(`Server running on port ${port}`);
});
EOF

# Deploy
fly deploy
```

### Option C: Deploy from Dockerfile

```bash
# Create Dockerfile
cat > Dockerfile <<EOF
FROM node:18-alpine

WORKDIR /app

COPY package*.json ./
RUN npm install --production

COPY . .

EXPOSE 3000

CMD ["npm", "start"]
EOF

# Create fly.toml
fly launch --generate-name

# Deploy
fly deploy
```

### Understanding the Deploy Process

```
fly deploy does:

1. Validates fly.toml configuration
2. Checks for Dockerfile or buildpacks
3. Builds Docker image locally or on Fly.io builders
4. Pushes image to Fly.io registry
5. Creates/updates Machines (microVMs)
6. Waits for health checks to pass
7. Switches traffic to new version
8. Cleans up old Machines
```

### Deployment Output

```
==> Verifying app config
--> Verified: fly.toml is valid

==> Building image
[+] Building 2.5s (12/12) DONE
 => => naming to registry.fly.io/my-app:latest

==> Creating launch
--> Created launch configuration

==> Creating Machines
  Machine a1b2c3d app/my-app [iad] task:app (created)
  Machine e4f5g6h app/my-app [ord] task:app (created)

==> Monitoring deployment
  a1b2c3d started: OK
  e4f5g6h started: OK

--> Deployment complete!

Visit: https://my-app.fly.dev
```

## Application Configuration

### fly.toml Complete Example

```toml
# Application identity
app = "my-app"
primary_region = "iad"
kill_signal = "SIGINT"
kill_timeout = 5

# Build configuration
[build]
  dockerfile = "Dockerfile"
  # Or use buildpacks:
  # builder = "heroku/buildpacks:2"
  
  # Build arguments
  [build.args]
    NODE_ENV = "production"

# Environment variables
[env]
  PORT = "8080"
  LOG_LEVEL = "info"
  DATABASE_URL = "postgres://user:pass@db.internal:5432/myapp"

# Deploy strategy
[deploy]
  strategy = "rolling"  # rolling, immediate, canary
  max_unavailable = 0.33  # Max % of machines down during deploy

# Persistent volumes
[[mounts]]
  source = "data"
  destination = "/data"
  initial_size = "10gb"

# HTTP service
[http_service]
  internal_port = 8080
  force_https = true
  auto_stop_machines = true   # Save money when idle
  auto_start_machines = true  # Wake on request
  
  # Concurrency limits
  [http_service.concurrency]
    type = "connections"
    hard_limit = 100  # Max connections before rejecting
    soft_limit = 80   # Start scaling at this point
  
  # Health checks
  [[http_service.checks]]
    grace_period = "10s"    # Wait before first check
    interval = "15s"        # Time between checks
    timeout = "5s"          # Wait for response
    method = "GET"
    path = "/health"
  
  # Additional checks
  [[http_service.checks]]
    interval = "30s"
    timeout = "10s"
    path = "/ready"

# VM configuration
[[vm]]
  cpu_kind = "shared"       # shared, performance
  cpus = 1
  memory_mb = 512
  
  # Or for dedicated CPUs:
  # cpu_kind = "performance"
  # cpus = 2
  # memory_mb = 4096

# Processes (multiple commands)
[processes]
  app = "npm start"
  worker = "npm run worker"
  release = "npm run migrate"

# Static files (optional)
[statics]
  guest_path = "/app/public"
  url_prefix = "/static"
```

### Environment Variables

```bash
# Set secrets (encrypted, not in fly.toml)
fly secrets set DATABASE_URL="postgres://..."
fly secrets set API_KEY="secret-key"

# View secrets (names only, not values)
fly secrets list

# Remove secrets
fly secrets unset API_KEY

# Set from file
fly secrets set MY_CERT=@/path/to/cert.pem
```

## Global Deployment

### Deploy to Multiple Regions

```bash
# Set primary region
fly regions set iad

# Add more regions
fly regions add lax ord fra lhr nrt

# View current regions
fly regions list

# Deploy to all configured regions
fly deploy
```

### Region-Specific Configuration

```toml
# Different VM sizes per region
[[vm]]
  region = "iad"
  cpu_kind = "shared"
  cpus = 2
  memory_mb = 1024

[[vm]]
  region = "fra"
  cpu_kind = "performance"
  cpus = 1
  memory_mb = 2048
```

### Anycast IP

Fly.io provides a single IP that routes to nearest edge:

```bash
# Get your app's IP
fly ips list

# Output:
# VERSION IP TYPE
# v4 123.45.67.89 Public
# v6 abcd::1234 Public
```

All regions share this IP - users automatically connect to nearest location.

## Persistent Storage

### Creating Volumes

```bash
# Create volume
fly volumes create data --region iad --size 10

# List volumes
fly volumes list

# Extend volume
fly volumes extend data --size 20
```

### Using Volumes

```toml
[[mounts]]
  source = "data"
  destination = "/data"
  
  # Optional settings
  auto_extend = true      # Auto-grow when full
  auto_extend_threshold = 0.8  # Extend at 80%
  auto_extend_size = 5    # Grow by 5GB increments
```

### Volume Best Practices

```bash
# Always backup important data
fly ssh console  # Then use tar, pg_dump, etc.

# Use volumes for:
# - Databases (SQLite, etc.)
# - File uploads
# - Cache directories
# - Application state

# Don't use volumes for:
# - Temporary files (use /tmp)
# - Logs (use fly logs)
# - Shared state (use Redis/Postgres)
```

## Managed Databases

### PostgreSQL

```bash
# Create database cluster
fly pg create --name myapp-db

# Attach to your app
fly pg attach myapp-db

# This sets DATABASE_URL secret automatically

# Connect to database
fly pg connect myapp-db

# View cluster status
fly pg status myapp-db

# Create database
fly pg connect myapp-db
> CREATE DATABASE myapp_production;
```

### PostgreSQL Architecture

```
Primary Region (iad):
┌──────────────┐
│   Primary    │
│  (read/write)│
└──────┬───────┘
       │ Replication
┌──────▼───────┐
│   Replica    │
│  (read-only) │
└──────────────┘

Optional: Add read replicas in other regions
fly pg attach myapp-db --region fra
```

### Redis

```bash
# Create Redis cluster
fly redis create myapp-redis

# Attach to app
fly redis attach myapp-redis

# This sets REDIS_URL secret

# Connect to Redis
fly redis connect myapp-redis
> SET foo bar
> GET foo
"bar"
```

## Monitoring and Debugging

### Logs

```bash
# Stream logs
fly logs

# Follow logs (like tail -f)
fly logs --follow

# Show recent logs
fly logs --num 100

# Filter by instance
fly logs --instance-id abc123
```

### SSH Access

```bash
# Open SSH shell to running Machine
fly ssh console

# Run specific command
fly ssh console --command "ls -la /app"

# SSH to specific region
fly ssh console --region iad
```

### Health Checks

```bash
# View health check status
fly health checks

# View specific check
fly health checks --name http
```

### Metrics Dashboard

```bash
# Open monitoring dashboard
fly dashboard

# Shows:
# - CPU usage
# - Memory usage
# - Request counts
# - Response times
# - Error rates
```

### Common Debugging Commands

```bash
# View app status
fly status

# View Machines
fly machines list

# View specific Machine
fly machines status <machine-id>

# Restart Machine
fly machines restart <machine-id>

# Stop app
fly apps stop my-app

# Start app
fly apps start my-app

# View events
fly events list

# View DNS configuration
fly dns export
```

## Architecture Deep Dive

### Fly.io Infrastructure

```
┌─────────────────────────────────────────────────────────────┐
│                    Fly.io Platform                           │
│                                                              │
│  ┌──────────────────┐  ┌──────────────────┐                │
│  │   API Gateway    │  │   Anycast IPs    │                │
│  │   (GraphQL)      │  │   (Global DNS)   │                │
│  └────────┬─────────┘  └────────┬─────────┘                │
│           │                     │                           │
│  ┌────────▼─────────────────────▼─────────┐               │
│  │          Edge Orchestrator              │               │
│  │          (Machine Scheduling)           │               │
│  └────────┬────────────────────────────────┘               │
│           │                                                │
│  ┌────────▼────────────────────────────────┐              │
│  │       Edge Locations (80+)              │              │
│  │                                         │              │
│  │  ┌───────────┐  ┌───────────┐         │              │
│  │  │ Firecracker│ │ Firecracker│         │              │
│  │  │  microVM  │ │  microVM  │  ...    │              │
│  │  │           │ │           │         │              │
│  │  │ - App     │ │ - App     │         │              │
│  │  │ - Sidecar │ │ - Sidecar │         │              │
│  │  └───────────┘ └───────────┘         │              │
│  └───────────────────────────────────────┘              │
│                                                          │
│  ┌─────────────────────────────────────────┐            │
│  │       Supporting Services               │            │
│  │  - Corrosion (Service Discovery)        │            │
│  │  - Flycast (Service Mesh)              │            │
│  │  - NATS (Log Streaming)                │            │
│  │  - Prometheus (Metrics)                │            │
│  └─────────────────────────────────────────┘            │
└───────────────────────────────────────────────────────────┘
```

### Request Flow

```
1. User Request
   ↓
2. DNS Resolution (anycast IP)
   ↓
3. Nearest Edge Location
   ↓
4. Proxy (fly-proxy)
   ↓
5. Firecracker microVM
   ↓
6. Application Container
   ↓
7. Response (reverse path)
```

### Machine Lifecycle

```
fly deploy triggers:

1. Create new Machine
   └─> Pull image
   └─> Configure volumes
   └─> Set environment variables
   └─> Start Firecracker VM

2. Wait for health checks
   └─> GET /health (or configured path)
   └─> Retry until healthy or timeout

3. Switch traffic
   └─> Update proxy configuration
   └─> Route new requests to new Machine

4. Cleanup
   └─> Stop old Machine
   └─> Wait for drain timeout
   └─> Delete old Machine
```

### Networking Architecture

```
┌─────────────────────────────────────────┐
│          WireGuard Mesh Network         │
│                                         │
│  Every Machine gets:                   │
│  - Private IPv6 (fd00::/8)             │
│  - Can reach any other Machine         │
│  - Encrypted via WireGuard             │
│                                         │
│  App 1 (iad) ──────┬──────> App 2 (fra)│
│  fd00::1          │       fd00::2      │
│                   │                     │
│  App 3 (lax) ─────┘                     │
│  fd00::3                                │
└─────────────────────────────────────────┘

# Access private network from app:
fetch("http://[fd00::2]:8080/api")  // Direct to App 2
```

## Production Checklist

Before deploying to production:

### Configuration
- [ ] Set appropriate region list
- [ ] Configure health checks
- [ ] Set resource limits (CPU, memory)
- [ ] Configure auto-scaling
- [ ] Set up persistent volumes if needed

### Security
- [ ] Use secrets for sensitive data
- [ ] Enable HTTPS (force_https = true)
- [ ] Configure appropriate kill_timeout
- [ ] Review Dockerfile for security

### Databases
- [ ] Create managed PostgreSQL/Redis
- [ ] Configure backups
- [ ] Set up read replicas if needed
- [ ] Test failover procedures

### Monitoring
- [ ] Set up log aggregation
- [ ] Configure alerts
- [ ] Test health check endpoints
- [ ] Set up uptime monitoring

### Deployment
- [ ] Test deployment in staging
- [ ] Document rollback procedure
- [ ] Plan maintenance windows
- [ ] Test database migrations

## Cost Estimation

Fly.io pricing:

```
Free tier:
- 3 shared-cpu-1x VMs (256MB each)
- 3GB persistent volume storage
- Unlimited outgoing data (within limits)

Paid usage:
- VMs: $1.94/month per shared-cpu-1x (256MB)
- Storage: $0.15/GB/month
- Data transfer: $0.02/GB after free tier

Example calculation:
- 2x performance-1x (1GB) in iad, fra: $38.90/month
- 10GB volume: $1.50/month
- 100GB transfer: $1.60/month
Total: ~$42/month
```

## Conclusion

Fly.io provides:

1. **Edge Deployment**: Run applications close to users globally
2. **Simple Workflow**: fly.toml config + fly deploy
3. **MicroVM Isolation**: Firecracker-based security
4. **Global Network**: 80+ locations with single anycast IP
5. **Managed Services**: PostgreSQL, Redis included
6. **Developer Experience**: Excellent CLI, good documentation

## Next Steps

- [exploration.md](./exploration.md) - Full architecture deep dive
- [rust-revision.md](./rust-revision.md) - Implementing edge platform in Rust
- [production-grade.md](./production-grade.md) - Production deployment patterns
