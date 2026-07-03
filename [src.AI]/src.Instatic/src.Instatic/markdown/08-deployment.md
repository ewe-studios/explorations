# Deployment: Docker, Database, TLS, and HA

Instatic is designed for self-hosting. This document covers Docker deployment, database setup, TLS configuration, and multi-instance high availability.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Instatic/Instatic/`

## Docker Deployment

### Dockerfile

```dockerfile
# Dockerfile
# Multi-stage build:
# 1. Install dependencies (bun install)
# 2. Build (bun build)
# 3. Run (bun server/index.ts)
```

**Source:** `Dockerfile`.

### Docker Compose

Multiple compose configurations for different scenarios:

| File | Purpose |
|------|---------|
| `docker-compose.yml` | Default development setup |
| `compose.build.yml` | Build configuration |
| `compose.prod.yml` | Production setup |
| `compose.sqlite.yml` | SQLite-only setup |
| `compose.tls.yml` | TLS with Caddy |

### Production Compose

```yaml
# compose.prod.yml
# Production-ready setup with Postgres
```

### SQLite Compose

```yaml
# compose.sqlite.yml
# SQLite-only setup for single-instance deployments
```

## TLS with Caddy

Instatic includes a Caddyfile for automatic TLS:

```
# Caddyfile
# Automatic HTTPS with Let's Encrypt
# Reverse proxy to the Bun server
```

**Source:** `Caddyfile`, `docs/deployment/tls-caddy.md`, `compose.tls.yml`.

## Deployment Options

### Railway

```
docs/deployment/railway.md
```

### Render

```
docs/deployment/render.md
```

### VPS

```
docs/deployment/vps.md
```

### Docker Image

```
docs/deployment/docker-image.md
```

## Database Setup

### Postgres

Postgres via `Bun.sql`:

```
DATABASE_URL=postgresql://user:pass@host:5432/dbname
```

### SQLite

SQLite via `bun:sqlite`:

```
DATABASE_URL=file:./data.db
```

**Aha:** The database is selected entirely by `DATABASE_URL`. Postgres enables multi-instance HA (via advisory locks); SQLite is single-instance by definition. The repository layer is dialect-naive — the same code works on both engines.

**Source:** `docs/reference/database-dialects.md`.

## Multi-Instance HA

When running multiple instances behind a load balancer with Postgres, both schedulers use leader election:

```mermaid
flowchart TD
    LB["Load Balancer"] --> I1["Instance 1"]
    LB --> I2["Instance 2"]
    LB --> I3["Instance 3"]
    I1 --> LOCK["pg_try_advisory_lock
    server/db/advisoryLock.ts"]
    I2 --> LOCK
    I3 --> LOCK
    LOCK --> LEADER["One instance gets the lock
    Others wait"]
```

Each scheduler passes its own distinct lock key:
- Plugin tick scheduler — one lock key
- Scheduled publish scheduler — different lock key

On SQLite, the module returns a no-op sentinel (single-instance by definition).

**Source:** `server/db/advisoryLock.ts`, `docs/architecture.md`.

## Backup and Restore

```
docs/deployment/backup-restore.md
```

## Release Workflow

```
docs/deployment/release-workflow.md
```

## GitHub Actions

```
.github/workflows/
```

**Source:** `.github/workflows/`.

## Environment Variables

| Variable | Purpose | Required |
|----------|---------|----------|
| `DATABASE_URL` | Database connection string | Yes |
| `NEXT_PUBLIC_MAPBOX_TOKEN` | Not used — Instatic doesn't use Mapbox | No |
| Admin session secret | Session cookie signing | Yes (production) |

**Source:** `.env.example`.

## Next

Continue to [09-cross-cutting](09-cross-cutting.md) for banned dependencies, patterns, and testing strategy.
