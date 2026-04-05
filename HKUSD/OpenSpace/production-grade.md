# OpenSpace Production-Grade Deployment Guide

A comprehensive guide for deploying, scaling, and operating OpenSpace in production environments. Covers architecture, deployment strategies, security, monitoring, and CI/CD pipelines.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Deployment Strategies](#2-deployment-strategies)
3. [Cloud Platform Scaling](#3-cloud-platform-scaling)
4. [Database Operations](#4-database-operations)
5. [MCP Server Production](#5-mcp-server-production)
6. [Security](#6-security)
7. [Monitoring](#7-monitoring)
8. [CI/CD](#8-cicd)

---

## 1. Architecture Overview

### 1.1 Local Deployment Model

```
┌─────────────────────────────────────────────────────────────────┐
│                        Developer Workstation                     │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                    Agent Host                               │ │
│  │  (Claude Code / OpenClaw / nanobot / Cursor)               │ │
│  │                          │                                  │ │
│  │                          │ MCP Protocol (stdio)            │ │
│  └──────────────────────────┼─────────────────────────────────┘ │
│                             │                                    │
│                             ▼                                    │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │              OpenSpace MCP Server                           │ │
│  │  ┌──────────────────────────────────────────────────────┐  │ │
│  │  │  FastMCP Server Instance                              │  │ │
│  │  │  - Tools: execute_task, search_skills,               │  │ │
│  │  │           fix_skill, upload_skill                     │  │ │
│  │  └──────────────────────────────────────────────────────┘  │ │
│  └────────────────────────────────────────────────────────────┘ │
│                             │                                    │
│                             ▼                                    │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │              OpenSpace Engine                               │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │ │
│  │  │ SkillRegistry│  │GroundingAgent│  │SkillEvolver  │     │ │
│  │  │ (discovery)  │  │ (execution)  │  │ (evolution)  │     │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘     │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │ │
│  │  │ SkillStore   │  │ Execution    │  │ ToolQuality  │     │ │
│  │  │ (SQLite)     │  │ Analyzer     │  │ Manager      │     │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘     │ │
│  └────────────────────────────────────────────────────────────┘ │
│                             │                                    │
│           ┌─────────────────┼─────────────────┐                 │
│           ▼                 ▼                 ▼                 │
│  ┌─────────────────┐ ┌──────────────┐ ┌──────────────┐         │
│  │ Shell Backend   │ │ GUI Backend  │ │ MCP Backend  │         │
│  │ (run_shell)     │ │ (computer    │ │ (stdio/HTTP) │         │
│  │                 │ │  use)        │ │              │         │
│  └─────────────────┘ └──────────────┘ └──────────────┘         │
│                                                                  │
│  ┌─────────────────┐ ┌──────────────────────────────────────┐  │
│  │ Skills Directory│ │ Recording Manager                    │  │
│  │ ./skills/       │ │ (screenshots/video logs)             │  │
│  └─────────────────┘ └──────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

**Local Deployment Characteristics:**

| Component | Location | Storage |
|-----------|----------|---------|
| MCP Server | Local process | N/A |
| OpenSpace Engine | Local process | N/A |
| Skill Store | Local file | `~/.openspace/skill_store.db` (SQLite) |
| Skills | Local directory | `./skills/` or configured path |
| Recordings | Local directory | `./logs/recordings/` |
| Cloud Client | HTTP client | API key from env |

### 1.2 Cloud Platform Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         OPENSPACE CLOUD PLATFORM                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────┐         ┌─────────────────┐         ┌───────────────┐ │
│  │   Agent Host    │         │   Agent Host    │         │  Agent Host   │ │
│  │  (Claude Code)  │         │  (OpenClaw)     │         │  (nanobot)    │ │
│  │        │        │         │        │        │         │       │       │ │
│  │   OpenSpace     │         │   OpenSpace     │         │  OpenSpace    │ │
│  │   Client        │         │   Client        │         │  Client       │ │
│  └────────┬────────┘         └────────┬────────┘         └───────┬───────┘ │
│           │                           │                          │         │
│           └───────────────────────────┼──────────────────────────┘         │
│                                       │                                     │
│                                       ▼                                     │
│           ┌───────────────────────────────────────────────────────────┐    │
│           │              open-space.cloud Platform                     │    │
│           │  ┌─────────────────────────────────────────────────────┐  │    │
│           │  │         Load Balancer (nginx/ALB)                    │  │    │
│           │  └─────────────────────────────────────────────────────┘  │    │
│           │                          │                                 │    │
│           │     ┌────────────────────┼────────────────────┐           │    │
│           │     ▼                    ▼                    ▼           │    │
│           │  ┌─────────┐      ┌─────────────┐     ┌─────────────┐    │    │
│           │  │  Auth   │      │   Skills    │     │  Embedding  │    │    │
│           │  │ Service │      │   Service   │     │   Service   │    │    │
│           │  │ (3 pods)│      │  (5 pods)   │     │  (2 pods)   │    │    │
│           │  └─────────┘      └─────────────┘     └─────────────┘    │    │
│           │                          │                                 │    │
│           │     ┌────────────────────┼────────────────────┐           │    │
│           │     ▼                    ▼                    ▼           │    │
│           │  ┌─────────┐      ┌─────────────┐     ┌─────────────┐    │    │
│           │  │  Redis  │      │ PostgreSQL  │     │    S3 /     │    │    │
│           │  │ Cluster │      │  + pgvector │     │   Blob      │    │    │
│           │  │ (6 nodes│      │  (Primary   │     │  Storage    │    │    │
│           │  │  + HA)  │      │  + Replica) │     │             │    │    │
│           │  └─────────┘      └─────────────┘     └─────────────┘    │    │
│           │                                                           │    │
│           └───────────────────────────────────────────────────────────┘    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Cloud Platform Services:**

| Service | Purpose | Scaling |
|---------|---------|---------|
| Auth Service | API key validation, user lookup | Horizontal (3+ pods) |
| Skills Service | CRUD operations, search, diff storage | Horizontal (5+ pods) |
| Embedding Service | Generate/query embeddings | Horizontal (2+ pods) |
| PostgreSQL | Skill records, pgvector similarity | Primary + Read Replicas |
| Redis | Caching, rate limiting, sessions | Cluster (6 nodes + HA) |
| S3/Blob | Skill artifacts, diff storage | Auto-scaling |

### 1.3 MCP Server Deployment

```
┌─────────────────────────────────────────────────────────────────┐
│                    MCP Server Process Model                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │  Process: openspace-mcp                                    │ │
│  │  ┌─────────────────────────────────────────────────────┐  │ │
│  │  │  Main Thread (asyncio event loop)                   │  │ │
│  │  │  ├─ stdin reader (JSON-RPC requests)                │  │ │
│  │  │  ├─ stdout writer (JSON-RPC responses)              │  │ │
│  │  │  ├─ Tool handlers (async)                           │  │ │
│  │  │  └─ Health check endpoint                           │  │ │
│  │  └─────────────────────────────────────────────────────┘  │ │
│  └───────────────────────────────────────────────────────────┘ │
│                                                                 │
│  Resource Limits:                                               │
│  - Memory: 512MB - 2GB (configurable)                          │
│  - CPU: 1-4 cores                                              │
│  - File descriptors: 1024+                                     │
│  - Network: Outbound HTTPS (cloud API)                         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 1.4 Agent Integration Patterns

**Pattern 1: Direct MCP Integration**

```json
{
  "mcpServers": {
    "openspace": {
      "command": "openspace-mcp",
      "env": {
        "OPENSPACE_MODEL": "openrouter/anthropic/claude-sonnet-4.5",
        "OPENROUTER_API_KEY": "${OPENROUTER_API_KEY}",
        "OPENSPACE_API_KEY": "${OPENSPACE_API_KEY}",
        "OPENSPACE_WORKSPACE": "/workspace"
      }
    }
  }
}
```

**Pattern 2: Docker MCP Server**

```yaml
# docker-compose.yml for agent + OpenSpace
version: '3.8'
services:
  agent:
    image: claude-code:latest
    volumes:
      - ./workspace:/workspace
      - ./mcp-config.json:/root/.claude/settings.json
    depends_on:
      - openspace-mcp

  openspace-mcp:
    build: ./openspace
    volumes:
      - ./workspace:/workspace
      - ./skills:/skills
      - openspace-data:/root/.openspace
    environment:
      - OPENSPACE_MODEL=openrouter/anthropic/claude-sonnet-4.5
      - OPENROUTER_API_KEY=${OPENROUTER_API_KEY}
      - OPENSPACE_API_KEY=${OPENSPACE_API_KEY}

volumes:
  openspace-data:
```

**Pattern 3: Kubernetes Sidecar**

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: agent-pod
spec:
  containers:
  - name: agent
    image: claude-code:latest
    volumeMounts:
    - name: mcp-socket
      path: /var/run/mcp
  - name: openspace-mcp
    image: openspace-mcp:latest
    ports:
    - containerPort: 8080
    env:
    - name: OPENSPACE_MODEL
      value: "openrouter/anthropic/claude-sonnet-4.5"
    volumeMounts:
    - name: mcp-socket
      path: /var/run/mcp
  volumes:
  - name: mcp-socket
    emptyDir: {}
```

---

## 2. Deployment Strategies

### 2.1 Python Package (pip install)

**Setup Requirements:**
- Python 3.10+
- pip 21.0+
- Virtual environment (recommended)

**Installation:**

```bash
# Create virtual environment
python -m venv .venv
source .venv/bin/activate  # Linux/macOS
# or .venv\Scripts\activate  # Windows

# Install OpenSpace
pip install openspace

# Verify installation
openspace --version
openspace-mcp --version
```

**Configuration File (`~/.openspace/config.yaml`):**

```yaml
# OpenSpace Configuration
llm:
  model: openrouter/anthropic/claude-sonnet-4.5
  api_key: ${OPENROUTER_API_KEY}
  timeout: 120
  max_tokens: 4096

workspace:
  dir: /workspace
  skills_dir: /workspace/skills
  recordings_dir: /workspace/logs/recordings

grounding:
  max_iterations: 20
  enabled_backends:
    - shell
    - mcp
  security:
    blocked_commands:
      - "rm -rf /"
      - "mkfs"
      - ":(){:|:&};:"

cloud:
  api_key: ${OPENSPACE_API_KEY}
  api_base: https://api.open-space.cloud
  enabled: true

logging:
  level: INFO
  file: /var/log/openspace/openspace.log
  format: "%(asctime)s - %(name)s - %(levelname)s - %(message)s"
```

**Systemd Service (Linux):**

```ini
# /etc/systemd/system/openspace-mcp.service
[Unit]
Description=OpenSpace MCP Server
After=network.target

[Service]
Type=simple
User=openspace
Group=openspace
Environment="PATH=/opt/openspace/.venv/bin"
Environment="OPENSPACE_MODEL=openrouter/anthropic/claude-sonnet-4.5"
EnvironmentFile=/etc/openspace/env
ExecStart=/opt/openspace/.venv/bin/openspace-mcp
Restart=always
RestartSec=5
LimitNOFILE=65535

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=read-only
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

### 2.2 Docker Containers

**Production Dockerfile:**

```dockerfile
# Dockerfile
FROM python:3.11-slim AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc \
    && rm -rf /var/lib/apt/lists/*

# Install Python dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir --user -r requirements.txt

# Production stage
FROM python:3.11-slim

# Create non-root user
RUN groupadd -r openspace && useradd -r -g openspace openspace

# Copy installed packages from builder
COPY --from=builder /root/.local /home/openspace/.local

# Copy application
COPY --chown=openspace:openspace ./openspace /app/openspace
COPY --chown=openspace:openspace ./scripts/entrypoint.sh /entrypoint.sh

WORKDIR /app

# Set environment
ENV PATH=/home/openspace/.local/bin:$PATH \
    PYTHONUNBUFFERED=1 \
    PYTHONDONTWRITEBYTECODE=1 \
    OPENSPACE_WORKSPACE=/workspace

# Create directories
RUN mkdir -p /workspace/skills /workspace/logs/recordings /var/log/openspace \
    && chown -R openspace:openspace /workspace /var/log/openspace

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD python -c "import openspace; print('healthy')" || exit 1

USER openspace

ENTRYPOINT ["/entrypoint.sh"]
CMD ["openspace-mcp"]
```

**Entrypoint Script (`scripts/entrypoint.sh`):**

```bash
#!/bin/bash
set -e

# Wait for dependencies (if any)
if [ -n "$DATABASE_URL" ]; then
    echo "Waiting for database..."
    until python -c "import psycopg2; psycopg2.connect('$DATABASE_URL')" 2>/dev/null; do
        sleep 1
    done
    echo "Database ready"
fi

# Initialize workspace
if [ ! -d "$OPENSPACE_WORKSPACE/skills" ]; then
    mkdir -p "$OPENSPACE_WORKSPACE/skills"
fi

# Set log permissions
chown -R openspace:openspace /var/log/openspace 2>/dev/null || true

# Execute main command
exec "$@"
```

**Docker Compose (Development):**

```yaml
# docker-compose.yml
version: '3.8'

services:
  openspace-mcp:
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "8080:8080"
    volumes:
      - ./workspace:/workspace
      - ./skills:/skills
    environment:
      - OPENSPACE_MODEL=openrouter/anthropic/claude-sonnet-4.5
      - OPENROUTER_API_KEY=${OPENROUTER_API_KEY}
      - OPENSPACE_API_KEY=${OPENSPACE_API_KEY}
      - OPENSPACE_MAX_ITERATIONS=20
    env_file:
      - .env
    networks:
      - openspace-net

  postgres:
    image: pgvector/pgvector:pg16
    environment:
      - POSTGRES_USER=openspace
      - POSTGRES_PASSWORD=${POSTGRES_PASSWORD:-openspace123}
      - POSTGRES_DB=openspace
    volumes:
      - postgres-data:/var/lib/postgresql/data
      - ./init-db.sql:/docker-entrypoint-initdb.d/init.sql
    networks:
      - openspace-net
    ports:
      - "5432:5432"

  redis:
    image: redis:7-alpine
    command: redis-server --appendonly yes
    volumes:
      - redis-data:/data
    networks:
      - openspace-net
    ports:
      - "6379:6379"

volumes:
  postgres-data:
  redis-data:

networks:
  openspace-net:
    driver: bridge
```

### 2.3 Kubernetes Deployments

**Namespace and ConfigMap:**

```yaml
# openspace-namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: openspace
  labels:
    name: openspace

---
# openspace-configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: openspace-config
  namespace: openspace
data:
  config.yaml: |
    llm:
      model: ${OPENSPACE_MODEL:-openrouter/anthropic/claude-sonnet-4.5}
      timeout: 120
      max_tokens: 4096

    workspace:
      dir: /workspace
      skills_dir: /workspace/skills
      recordings_dir: /workspace/logs/recordings

    grounding:
      max_iterations: 20
      enabled_backends:
        - shell
        - mcp

    logging:
      level: ${LOG_LEVEL:-INFO}
      format: "json"

  security-policies.yaml: |
    blocked_commands:
      - "rm -rf /"
      - "mkfs"
      - ":(){:|:&};:"
      - "dd if=/dev/zero"
      - "wget * | sh"
      - "curl * | sh"

    allowed_paths:
      - /workspace
      - /tmp

    max_memory_mb: 2048
    max_cpu_percent: 80
```

**MCP Server Deployment:**

```yaml
# openspace-mcp-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: openspace-mcp
  namespace: openspace
  labels:
    app: openspace-mcp
spec:
  replicas: 3
  selector:
    matchLabels:
      app: openspace-mcp
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  template:
    metadata:
      labels:
        app: openspace-mcp
        version: v1
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "8080"
        prometheus.io/path: "/metrics"
    spec:
      serviceAccountName: openspace-sa
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        fsGroup: 1000
      initContainers:
      - name: init-workspace
        image: busybox:1.36
        command: ['sh', '-c', 'mkdir -p /workspace/skills /workspace/logs/recordings && chown -R 1000:1000 /workspace']
        volumeMounts:
        - name: workspace
          mountPath: /workspace
      containers:
      - name: openspace-mcp
        image: openspace-mcp:latest
        imagePullPolicy: Always
        ports:
        - containerPort: 8080
          name: http
          protocol: TCP
        env:
        - name: OPENSPACE_MODEL
          valueFrom:
            configMapKeyRef:
              name: openspace-config
              key: model
        - name: OPENSPACE_WORKSPACE
          value: /workspace
        - name: OPENSPACE_MAX_ITERATIONS
          value: "20"
        - name: LOG_LEVEL
          valueFrom:
            configMapKeyRef:
              name: openspace-config
              key: log_level
        - name: OPENROUTER_API_KEY
          valueFrom:
            secretKeyRef:
              name: openspace-secrets
              key: openrouter-api-key
        - name: OPENSPACE_API_KEY
          valueFrom:
            secretKeyRef:
              name: openspace-secrets
              key: openspace-api-key
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: openspace-secrets
              key: database-url
        - name: REDIS_URL
          value: "redis://openspace-redis:6379/0"
        volumeMounts:
        - name: config
          mountPath: /app/config
        - name: workspace
          mountPath: /workspace
        - name: logs
          mountPath: /var/log/openspace
        resources:
          requests:
            memory: "512Mi"
            cpu: "250m"
          limits:
            memory: "2Gi"
            cpu: "1000m"
        livenessProbe:
          httpGet:
            path: /health/live
            port: 8080
          initialDelaySeconds: 15
          periodSeconds: 20
          timeoutSeconds: 5
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 10
          timeoutSeconds: 3
          failureThreshold: 3
        lifecycle:
          preStop:
            exec:
              command: ["sh", "-c", "sleep 10"]
      volumes:
      - name: config
        configMap:
          name: openspace-config
      - name: workspace
        persistentVolumeClaim:
          claimName: openspace-workspace-pvc
      - name: logs
        emptyDir: {}
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
          - weight: 100
            podAffinityTerm:
              labelSelector:
                matchLabels:
                  app: openspace-mcp
              topologyKey: kubernetes.io/hostname
      tolerations:
      - key: "openspace"
        operator: "Equal"
        value: "true"
        effect: "NoSchedule"
```

**Horizontal Pod Autoscaler:**

```yaml
# openspace-mcp-hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: openspace-mcp-hpa
  namespace: openspace
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: openspace-mcp
  minReplicas: 3
  maxReplicas: 20
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
  - type: Pods
    pods:
      metric:
        name: requests_per_second
      target:
        type: AverageValue
        averageValue: "100"
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
      - type: Percent
        value: 50
        periodSeconds: 60
    scaleUp:
      stabilizationWindowSeconds: 60
      policies:
      - type: Percent
        value: 100
        periodSeconds: 60
      - type: Pods
        value: 4
        periodSeconds: 60
      selectPolicy: Max
```

**Service and Ingress:**

```yaml
# openspace-mcp-service.yaml
apiVersion: v1
kind: Service
metadata:
  name: openspace-mcp
  namespace: openspace
  labels:
    app: openspace-mcp
  annotations:
    service.beta.kubernetes.io/aws-load-balancer-internal: "true"
    service.beta.kubernetes.io/aws-load-balancer-type: "nlb"
spec:
  type: ClusterIP
  ports:
  - port: 8080
    targetPort: 8080
    protocol: TCP
    name: http
  - port: 9090
    targetPort: 9090
    protocol: TCP
    name: metrics
  selector:
    app: openspace-mcp

---
# openspace-mcp-ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: openspace-mcp-ingress
  namespace: openspace
  annotations:
    kubernetes.io/ingress.class: nginx
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    nginx.ingress.kubernetes.io/proxy-body-size: "50m"
    nginx.ingress.kubernetes.io/proxy-read-timeout: "120"
    nginx.ingress.kubernetes.io/proxy-send-timeout: "120"
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
spec:
  tls:
  - hosts:
    - openspace-mcp.example.com
    secretName: openspace-mcp-tls
  rules:
  - host: openspace-mcp.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: openspace-mcp
            port:
              number: 8080
```

### 2.4 Cloud Deployments

#### AWS Deployment

**EKS with Terraform:**

```hcl
# terraform/eks/main.tf
terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "~> 2.23"
    }
  }
}

provider "aws" {
  region = var.aws_region
}

# EKS Cluster
resource "aws_eks_cluster" "openspace" {
  name     = "openspace-${var.environment}"
  role_arn = aws_iam_role.eks_cluster.arn
  version  = "1.28"

  vpc_config {
    subnet_ids              = aws_subnet.private[*].id
    endpoint_private_access = true
    endpoint_public_access  = true
    security_group_ids      = [aws_security_group.eks.id]
  }

  enabled_cluster_log_types = ["api", "audit", "authenticator"]

  tags = {
    Environment = var.environment
    Project     = "openspace"
  }
}

# EKS Node Group
resource "aws_eks_node_group" "openspace" {
  cluster_name    = aws_eks_cluster.openspace.name
  node_group_name = "openspace-mcp"
  node_role_arn   = aws_iam_role.node.arn
  subnet_ids      = aws_subnet.private[*].id

  scaling_config {
    desired_size = 3
    max_size     = 10
    min_size     = 2
  }

  instance_types = ["m6i.xlarge"]

  labels = {
    role = "openspace-mcp"
  }

  taint {
    key    = "openspace"
    value  = "true"
    effect = "NO_SCHEDULE"
  }

  depends_on = [aws_iam_role_policy_attachment.node]
}

# RDS PostgreSQL (with pgvector extension)
resource "aws_db_instance" "openspace" {
  identifier           = "openspace-${var.environment}"
  engine               = "postgres"
  engine_version       = "15.7"
  instance_class       = "db.r6g.xlarge"
  allocated_storage    = 100
  storage_encrypted    = true
  kms_key_id          = aws_kms_key.rds.arn

  db_name  = "openspace"
  username = var.db_username
  password = var.db_password

  vpc_security_group_ids = [aws_security_group.rds.id]
  db_subnet_group_name   = aws_db_subnet_group.openspace.name

  backup_retention_period = 7
  backup_window          = "03:00-04:00"
  maintenance_window     = "Mon:04:00-Mon:05:00"

  multi_az               = true
  automatically_pause    = false

  enabled_cloudwatch_logs_exports = ["postgresql"]

  tags = {
    Environment = var.environment
    Project     = "openspace"
  }
}

# ElastiCache Redis
resource "aws_elasticache_cluster" "openspace" {
  cluster_id           = "openspace-${var.environment}"
  engine               = "redis"
  node_type            = "cache.r6g.large"
  num_cache_nodes      = 3
  parameter_group_name = "default.redis7"
  port                 = 6379

  security_group_ids = [aws_security_group.redis.id]
  subnet_group_name  = aws_elasticache_subnet_group.openspace.name

  tags = {
    Environment = var.environment
    Project     = "openspace"
  }
}

# S3 Bucket for artifacts
resource "aws_s3_bucket" "openspace_artifacts" {
  bucket = "openspace-artifacts-${var.environment}-${data.aws_caller_identity.current.account_id}"

  tags = {
    Environment = var.environment
    Project     = "openspace"
  }
}

resource "aws_s3_bucket_versioning" "openspace_artifacts" {
  bucket = aws_s3_bucket.openspace_artifacts.id
  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "openspace_artifacts" {
  bucket = aws_s3_bucket.openspace_artifacts.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "aws:kms"
      kms_master_key_id = aws_kms_key.s3.arn
    }
  }
}
```

#### GCP Deployment

**GKE with Terraform:**

```hcl
# terraform/gke/main.tf
provider "google" {
  project = var.project_id
  region  = var.region
}

# GKE Cluster
resource "google_container_cluster" "openspace" {
  name     = "openspace-${var.environment}"
  location = var.region

  remove_default_node_pool = true
  initial_node_count       = 1

  networking_mode = "VPC_NATIVE"
  network         = google_compute_network.vpc.name
  subnetwork      = google_compute_subnetwork.subnet.name

  private_cluster_config {
    enable_private_nodes    = true
    enable_private_endpoint = false
    master_ipv4_cidr_block  = "172.16.0.0/28"
  }

  master_auth {
    client_certificate_config {
      issue_client_certificate = false
    }
  }

  addons_config {
    http_load_balancing {
      disabled = false
    }
    horizontal_pod_autoscaling {
      disabled = false
    }
  }

  release_channel {
    channel = "REGULAR"
  }
}

resource "google_container_node_pool" "openspace_nodes" {
  name       = "openspace-mcp-pool"
  location   = var.region
  cluster    = google_container_cluster.openspace.name
  node_count = 3

  autoscaling {
    min_node_count = 2
    max_node_count = 10
  }

  management {
    auto_repair  = true
    auto_upgrade = true
  }

  node_config {
    machine_type = "n2-standard-4"

    workload_metadata_config {
      mode = "GKE_METADATA"
    }

    labels = {
      role = "openspace-mcp"
    }

    taint {
      key    = "openspace"
      value  = "true"
      effect = "NO_SCHEDULE"
    }
  }
}

# Cloud SQL PostgreSQL (with pgvector)
resource "google_sql_database_instance" "openspace" {
  name             = "openspace-${var.environment}"
  database_version = "POSTGRES_15"
  region           = var.region

  settings {
    tier              = "db-custom-4-15360"
    availability_type = "REGIONAL"

    backup_configuration {
      enabled            = true
      start_time         = "03:00"
      point_in_time_recovery_enabled = true
    }

    ip_configuration {
      ipv4_enabled = true
      private_network = google_compute_network.vpc.id
    }

    insights_config {
      query_insights_enabled = true
    }
  }

  deletion_protection = true
}

resource "google_sql_database" "openspace" {
  name     = "openspace"
  instance = google_sql_database_instance.openspace.name
}

# Memorystore Redis
resource "google_redis_instance" "openspace" {
  name               = "openspace-${var.environment}"
  tier               = "STANDARD_HA"
  memory_size_gb     = 10
  region             = var.region
  redis_version      = "REDIS_7_0"
  display_name       = "OpenSpace Redis"
  authorized_network = google_compute_network.vpc.id

  redis_config {
    maxmemory_policy = "volatile-lru"
  }
}

# Cloud Storage for artifacts
resource "google_storage_bucket" "openspace_artifacts" {
  name          = "openspace-artifacts-${var.environment}"
  location      = var.region
  force_destroy = true

  uniform_bucket_level_access = true

  encryption {
    default_kms_key_name = google_kms_crypto_key.s3.id
  }
}

resource "google_storage_bucket_versioning" "openspace_artifacts" {
  bucket = google_storage_bucket.openspace_artifacts.name
  enabled = true
}
```

#### Azure Deployment

**AKS with Terraform:**

```hcl
# terraform/aks/main.tf
provider "azurerm" {
  features {}
}

# Resource Group
resource "azurerm_resource_group" "openspace" {
  name     = "openspace-${var.environment}-rg"
  location = var.location
}

# AKS Cluster
resource "azurerm_kubernetes_cluster" "openspace" {
  name                = "openspace-${var.environment}"
  location            = azurerm_resource_group.openspace.location
  resource_group_name = azurerm_resource_group.openspace.name
  dns_prefix          = "openspace-${var.environment}"
  kubernetes_version  = "1.28"

  default_node_pool {
    name                = "system"
    node_count          = 2
    vm_size             = "Standard_DS2_v2"
    enable_auto_scaling = true
    min_count           = 2
    max_count           = 5
  }

  node_pool {
    name                = "openspace"
    node_count          = 3
    vm_size             = "Standard_D4s_v3"
    enable_auto_scaling = true
    min_count           = 2
    max_count           = 10
    node_labels = {
      role = "openspace-mcp"
    }
    node_taints = [
      "openspace=true:NoSchedule"
    ]
  }

  identity {
    type = "SystemAssigned"
  }

  network_profile {
    network_plugin    = "azure"
    network_policy    = "calico"
    load_balancer_sku = "standard"
  }

  oms_agent {
    log_analytics_workspace_id = azurerm_log_analytics_workspace.openspace.id
  }

  tags = {
    Environment = var.environment
    Project     = "openspace"
  }
}

# Azure Database for PostgreSQL
resource "azurerm_postgresql_flexible_server" "openspace" {
  name                   = "openspace-${var.environment}"
  resource_group_name    = azurerm_resource_group.openspace.name
  location               = azurerm_resource_group.openspace.location
  version                = "15"
  delegated_subnet_id    = azurerm_subnet.postgresql.id
  private_dns_zone_id    = azurerm_private_dns_zone.postgresql.id
  administrator_login    = var.db_admin
  administrator_password = var.db_password
  zone                   = "1"

  sku_name = "GP_Standard_D4s_v3"

  storage_mb = 131072

  high_availability {
    mode = "ZoneRedundant"
  }

  backup_retention_days = 7

  tags = {
    Environment = var.environment
  }
}

# Azure Cache for Redis
resource "azurerm_redis_cache" "openspace" {
  name                = "openspace-${var.environment}"
  location            = azurerm_resource_group.openspace.location
  resource_group_name = azurerm_resource_group.openspace.name
  capacity            = 2
  family              = "P"
  sku_name            = "Premium"
  enable_non_ssl_port = false
  minimum_tls_version = "1.2"

  redis_configuration {
    maxmemory_reserved = "200"
    maxmemory_delta    = "200"
    maxmemory_policy   = "volatile-lru"
  }
}

# Azure Storage for artifacts
resource "azurerm_storage_account" "openspace" {
  name                     = "openspace${var.environment}artifacts"
  resource_group_name      = azurerm_resource_group.openspace.name
  location                 = azurerm_resource_group.openspace.location
  account_tier             = "Standard"
  account_replication_type = "GRS"
  account_kind             = "StorageV2"

  blob_properties {
    versioning_enabled = true
  }
}
```

---

## 3. Cloud Platform Scaling

### 3.1 API Server Scaling

**Load Balancer Configuration (nginx):**

```nginx
# nginx.conf for OpenSpace API
worker_processes auto;
worker_rlimit_nofile 65535;

events {
    worker_connections 4096;
    use epoll;
    multi_accept on;
}

http {
    # Timeouts
    client_body_timeout 120;
    client_header_timeout 120;
    keepalive_timeout 65;
    send_timeout 120;

    # Buffers
    client_body_buffer_size 10M;
    client_max_body_size 50M;

    # Logging
    log_format openspace '$remote_addr - $remote_user [$time_local] '
                        '"$request" $status $body_bytes_sent '
                        '"$http_referer" "$http_user_agent" '
                        'rt=$request_time uct=$upstream_connect_time '
                        'uht=$upstream_header_time urt=$upstream_response_time';

    # Rate limiting zones
    limit_req_zone $binary_remote_addr zone=api_limit:10m rate=100r/s;
    limit_conn_zone $binary_remote_addr zone=conn_limit:10m;

    # Upstream for API servers
    upstream openspace_api {
        least_conn;
        server api-1.openspace.internal:8080 weight=1 max_fails=3 fail_timeout=30s;
        server api-2.openspace.internal:8080 weight=1 max_fails=3 fail_timeout=30s;
        server api-3.openspace.internal:8080 weight=1 max_fails=3 fail_timeout=30s;
        server api-4.openspace.internal:8080 weight=1 max_fails=3 fail_timeout=30s;
        server api-5.openspace.internal:8080 weight=1 max_fails=3 fail_timeout=30s;
        keepalive 32;
    }

    server {
        listen 443 ssl http2;
        server_name api.open-space.cloud;

        ssl_certificate /etc/ssl/certs/openspace.crt;
        ssl_certificate_key /etc/ssl/private/openspace.key;
        ssl_session_timeout 1d;
        ssl_session_cache shared:SSL:50m;
        ssl_session_tickets off;

        # Security headers
        add_header Strict-Transport-Security "max-age=63072000" always;
        add_header X-Frame-Options DENY;
        add_header X-Content-Type-Options nosniff;

        location /api/ {
            limit_req zone=api_limit burst=200 nodelay;
            limit_conn conn_limit 100;

            proxy_pass http://openspace_api;
            proxy_http_version 1.1;
            proxy_set_header Connection "";
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;

            proxy_connect_timeout 5s;
            proxy_send_timeout 120s;
            proxy_read_timeout 120s;

            # Health check endpoint
            location /api/health {
                access_log off;
                proxy_pass http://openspace_api;
            }
        }
    }
}
```

**API Auto-scaling Rules:**

```yaml
# Horizontal Pod Autoscaler for API
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: openspace-api-hpa
  namespace: openspace
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: openspace-api
  minReplicas: 5
  maxReplicas: 50
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 60
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 75
  - type: Pods
    pods:
      metric:
        name: http_requests_per_second
      target:
        type: AverageValue
        averageValue: "50"
  - type: Object
    object:
      describedObject:
        apiVersion: v1
        kind: Service
        name: openspace-api
      metric:
        name: requests_per_target
      target:
        type: AverageValue
        averageValue: "1000m"
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
      - type: Percent
        value: 30
        periodSeconds: 120
    scaleUp:
      stabilizationWindowSeconds: 30
      policies:
      - type: Percent
        value: 100
        periodSeconds: 30
      - type: Pods
        value: 10
        periodSeconds: 30
      selectPolicy: Max
```

### 3.2 PostgreSQL with pgvector

**Database Schema:**

```sql
-- PostgreSQL Schema for OpenSpace Cloud Platform
-- Requires: PostgreSQL 15+ with pgvector extension

-- Enable pgvector extension
CREATE EXTENSION IF NOT EXISTS vector;

-- Create enum types
CREATE TYPE skill_visibility AS ENUM ('public', 'private', 'group_only');
CREATE TYPE skill_origin AS ENUM ('imported', 'derived', 'fixed', 'captured');
CREATE TYPE skill_level AS ENUM ('workflow', 'tool', 'pattern');

-- Users table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    api_key_hash VARCHAR(255) NOT NULL,
    api_key_prefix VARCHAR(20) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    last_login_at TIMESTAMP WITH TIME ZONE,
    is_active BOOLEAN DEFAULT true,
    metadata JSONB DEFAULT '{}'::jsonb
);

CREATE INDEX idx_users_api_key_prefix ON users(api_key_prefix);
CREATE INDEX idx_users_email ON users(email);

-- Groups table
CREATE TABLE groups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    metadata JSONB DEFAULT '{}'::jsonb
);

CREATE TABLE group_members (
    group_id UUID REFERENCES groups(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(50) DEFAULT 'member',
    joined_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (group_id, user_id)
);

-- Skill records main table
CREATE TABLE skill_records (
    record_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    skill_id VARCHAR(255) NOT NULL,
    artifact_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    level skill_level DEFAULT 'workflow',
    visibility skill_visibility DEFAULT 'public',
    origin skill_origin NOT NULL,
    parent_skill_ids UUID[] DEFAULT '{}',
    tags TEXT[] DEFAULT '{}',
    content_diff TEXT,
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,

    -- Metrics
    total_downloads INTEGER DEFAULT 0,
    total_views INTEGER DEFAULT 0,
    success_rate DECIMAL(5,4) DEFAULT 1.0,
    avg_latency_ms DECIMAL(10,2),

    -- pgvector embedding for semantic search
    embedding vector(1536),

    CONSTRAINT unique_skill_id UNIQUE (skill_id)
);

-- Indexes for skill_records
CREATE INDEX idx_skill_records_skill_id ON skill_records(skill_id);
CREATE INDEX idx_skill_records_name ON skill_records(name);
CREATE INDEX idx_skill_records_origin ON skill_records(origin);
CREATE INDEX idx_skill_records_visibility ON skill_records(visibility);
CREATE INDEX idx_skill_records_created_by ON skill_records(created_by);
CREATE INDEX idx_skill_records_tags ON skill_records USING GIN(tags);
CREATE INDEX idx_skill_records_embedding ON skill_records USING hnsw (embedding vector_cosine_ops);
CREATE INDEX idx_skill_records_created_at ON skill_records(created_at DESC);

-- Skill lineage (parent-child relationships)
CREATE TABLE skill_lineage_parents (
    skill_id UUID PRIMARY KEY REFERENCES skill_records(record_id) ON DELETE CASCADE,
    parent_skill_ids UUID[] NOT NULL,
    origin skill_origin NOT NULL,
    generation INTEGER DEFAULT 1
);

-- Skill tool dependencies
CREATE TABLE skill_tool_deps (
    record_id UUID REFERENCES skill_records(record_id) ON DELETE CASCADE,
    tool_name VARCHAR(255) NOT NULL,
    backend VARCHAR(50),
    is_critical BOOLEAN DEFAULT true,
    PRIMARY KEY (record_id, tool_name)
);

-- Execution analyses
CREATE TABLE execution_analyses (
    analysis_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_id VARCHAR(255) NOT NULL,
    recorded_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    total_tokens INTEGER,
    task_outcome VARCHAR(50),
    metadata JSONB DEFAULT '{}'::jsonb
);

-- Skill judgments within analyses
CREATE TABLE skill_judgments (
    judgment_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    analysis_id UUID REFERENCES execution_analyses(analysis_id) ON DELETE CASCADE,
    skill_id VARCHAR(255) NOT NULL,
    was_applied BOOLEAN,
    was_effective BOOLEAN,
    quality_score DECIMAL(5,4),
    notes TEXT
);

CREATE INDEX idx_skill_judgments_skill_id ON skill_judgments(skill_id);
CREATE INDEX idx_skill_judgments_analysis_id ON skill_judgments(analysis_id);

-- Tool quality metrics
CREATE TABLE tool_quality_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tool_key VARCHAR(255) NOT NULL,
    backend VARCHAR(50),
    success_count INTEGER DEFAULT 0,
    failure_count INTEGER DEFAULT 0,
    total_latency_ms DECIMAL(15,2) DEFAULT 0,
    last_success_at TIMESTAMP WITH TIME ZONE,
    last_failure_at TIMESTAMP WITH TIME ZONE,
    degradation_detected BOOLEAN DEFAULT false,
    degradation_since TIMESTAMP WITH TIME ZONE,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (tool_key, backend)
);

-- Create function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Add triggers
CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_skill_records_updated_at
    BEFORE UPDATE ON skill_records
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
```

**Connection Pooling (PgBouncer):**

```ini
# pgbouncer.ini
[databases]
openspace = host=localhost port=5432 dbname=openspace pool_mode=transaction

[pgbouncer]
listen_port = 6432
listen_addr = 0.0.0.0

auth_type = md5
auth_file = /etc/pgbouncer/userlist.txt

# Pool settings
pool_mode = transaction
max_client_conn = 10000
default_pool_size = 50
min_pool_size = 20
reserve_pool_size = 10
reserve_pool_timeout = 3

# Timeouts
server_connect_timeout = 5
server_idle_timeout = 600
server_lifetime = 3600
client_idle_timeout = 0

# Logging
log_connections = 0
log_disconnections = 0
log_pooler_errors = 1
stats_period = 60
```

**Query Optimization:**

```sql
-- Analyze tables for query optimization
ANALYZE skill_records;
ANALYZE users;
ANALYZE skill_lineage_parents;

-- Vacuum regularly
VACUUM ANALYZE skill_records;

-- Explain query plans
EXPLAIN (ANALYZE, BUFFERS)
SELECT sr.*, u.email as creator_email
FROM skill_records sr
JOIN users u ON sr.created_by = u.id
WHERE sr.visibility = 'public'
  AND sr.embedding <=> $1::vector < 0.3
ORDER BY sr.embedding <=> $1::vector
LIMIT 50;

-- Create partial indexes for common queries
CREATE INDEX idx_skill_records_public_active
    ON skill_records (embedding vector_cosine_ops)
    WHERE visibility = 'public' AND created_at > NOW() - INTERVAL '30 days';
```

### 3.3 Redis Caching

**Redis Configuration:**

```conf
# redis.conf for production

# Network
bind 0.0.0.0
port 6379
protected-mode yes
requirepass ${REDIS_PASSWORD}

# Performance
maxmemory 8gb
maxmemory-policy volatile-lru
maxclients 10000
timeout 300
tcp-keepalive 60

# Persistence (AOF)
appendonly yes
appendfilename "appendonly.aof"
appendfsync everysec
auto-aof-rewrite-percentage 100
auto-aof-rewrite-min-size 64mb

# Persistence (RDB snapshots)
save 900 1
save 300 10
save 60 10000
dbfilename dump.rdb

# Replication
replica-read-only yes
repl-diskless-sync no
repl-ping-replica-period 10
repl-timeout 60

# Cluster mode (for horizontal scaling)
cluster-enabled yes
cluster-config-file nodes.conf
cluster-node-timeout 15000
cluster-replica-validity-factor 10
cluster-migration-barrier 1
cluster-require-full-coverage no

# Slow log
slowlog-log-slower-than 10000
slowlog-max-len 128

# Monitoring
latency-monitor-threshold 100
```

**Cache Keys Structure:**

```python
# Redis key patterns for OpenSpace
REDIS_KEYS = {
    # User sessions
    "session:{session_id}": "Session data (TTL: 24h)",

    # API key cache
    "apikey:{prefix}": "User ID lookup (TTL: 1h)",

    # Skill cache
    "skill:{skill_id}": "Skill record (TTL: 5m)",
    "skill:embedding:{query_hash}": "Embedding search results (TTL: 10m)",

    # Rate limiting
    "ratelimit:{user_id}:{endpoint}": "Request counter (TTL: 1m)",

    # Tool quality
    "tool:quality:{tool_key}": "Tool metrics (TTL: persistent)",

    # Search cache
    "search:{query_hash}": "Search results (TTL: 15m)",
}
```

**Caching Layer Implementation:**

```python
# caching.py
import asyncio
import hashlib
import json
from typing import Any, Dict, List, Optional

import redis.asyncio as redis

class CacheLayer:
    """Redis caching layer for OpenSpace cloud platform."""

    def __init__(self, redis_url: str, default_ttl: int = 300):
        self.redis = redis.from_url(redis_url, decode_responses=True)
        self.default_ttl = default_ttl

    def _make_key(self, prefix: str, *args: str) -> str:
        """Generate a cache key."""
        key_parts = [prefix] + list(args)
        return ":".join(key_parts)

    def _hash_query(self, query: str) -> str:
        """Generate a hash for long queries."""
        return hashlib.sha256(query.encode()).hexdigest()[:16]

    async def get(self, key: str) -> Optional[Any]:
        """Get value from cache."""
        data = await self.redis.get(key)
        if data is None:
            return None
        try:
            return json.loads(data)
        except json.JSONDecodeError:
            return data

    async def set(
        self,
        key: str,
        value: Any,
        ttl: Optional[int] = None,
    ) -> bool:
        """Set value in cache."""
        if isinstance(value, (dict, list)):
            data = json.dumps(value)
        else:
            data = str(value)

        return await self.redis.set(
            key,
            data,
            ex=ttl or self.default_ttl,
        )

    async def delete(self, *keys: str) -> int:
        """Delete keys from cache."""
        return await self.redis.delete(*keys)

    async def get_skill(self, skill_id: str) -> Optional[Dict]:
        """Get skill from cache."""
        key = self._make_key("skill", skill_id)
        return await self.get(key)

    async def cache_skill(self, skill_id: str, skill_data: Dict) -> None:
        """Cache skill data."""
        key = self._make_key("skill", skill_id)
        await self.set(key, skill_data, ttl=300)  # 5 minutes

    async def get_embedding_search(
        self,
        query: str,
        filters: Dict,
    ) -> Optional[List[Dict]]:
        """Get cached embedding search results."""
        cache_key = self._hash_query(f"{query}:{json.dumps(filters, sort_keys=True)}")
        key = self._make_key("skill:embedding", cache_key)
        return await self.get(key)

    async def cache_embedding_search(
        self,
        query: str,
        filters: Dict,
        results: List[Dict],
    ) -> None:
        """Cache embedding search results."""
        cache_key = self._hash_query(f"{query}:{json.dumps(filters, sort_keys=True)}")
        key = self._make_key("skill:embedding", cache_key)
        await self.set(key, results, ttl=600)  # 10 minutes

    async def increment_rate_limit(
        self,
        user_id: str,
        endpoint: str,
        limit: int,
        window: int = 60,
    ) -> int:
        """Increment rate limit counter and return current count."""
        key = self._make_key("ratelimit", user_id, endpoint)
        current = await self.redis.incr(key)
        if current == 1:
            await self.redis.expire(key, window)
        return current

    async def close(self) -> None:
        """Close Redis connection."""
        await self.redis.close()
```

### 3.4 Embedding Generation

**Embedding Service Architecture:**

```
┌─────────────────────────────────────────────────────────────────┐
│                   Embedding Generation Pipeline                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐       │
│  │   Request   │────▶│   Queue     │────▶│   Worker    │       │
│  │   (HTTP)    │     │   (Redis)   │     │   Pool      │       │
│  └─────────────┘     └─────────────┘     └──────┬──────┘       │
│                                                  │              │
│                    ┌─────────────────────────────┤              │
│                    │                             │              │
│                    ▼                             ▼              │
│            ┌─────────────┐             ┌─────────────┐         │
│            │   Batch     │             │  Embedding  │         │
│            │   Processor │             │   Model     │         │
│            │             │             │  (GPU/CPU)  │         │
│            └──────┬──────┘             └──────┬──────┘         │
│                   │                           │                 │
│                   └───────────┬───────────────┘                 │
│                               │                                 │
│                               ▼                                 │
│                       ┌─────────────┐                          │
│                       │  PostgreSQL │                          │
│                       │  + pgvector │                          │
│                       └─────────────┘                          │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Embedding Worker Implementation:**

```python
# embedding_worker.py
import asyncio
from typing import List, Optional
import numpy as np

from redis.asyncio import Redis
from openai import AsyncOpenAI

class EmbeddingWorker:
    """Worker for generating embeddings asynchronously."""

    def __init__(
        self,
        redis_url: str,
        model: str = "text-embedding-3-large",
        dimensions: int = 1536,
        batch_size: int = 32,
    ):
        self.redis = Redis.from_url(redis_url, decode_responses=True)
        self.model = model
        self.dimensions = dimensions
        self.batch_size = batch_size
        self.client = AsyncOpenAI()

    async def start(self) -> None:
        """Start the embedding worker."""
        print(f"Starting embedding worker (batch_size={self.batch_size})")
        await self.process_queue()

    async def process_queue(self) -> None:
        """Process embedding requests from the queue."""
        while True:
            try:
                # Get batch of jobs
                jobs = await self.redis.blpop(
                    "embedding_queue",
                    timeout=5,
                )

                if not jobs:
                    continue

                batch = [jobs]
                for _ in range(self.batch_size - 1):
                    job = await self.redis.lpop("embedding_queue")
                    if job:
                        batch.append(("embedding_queue", job))
                    else:
                        break

                # Process batch
                await self._process_batch(batch)

            except Exception as e:
                print(f"Error processing queue: {e}")
                await asyncio.sleep(1)

    async def _process_batch(self, batch: List[tuple]) -> None:
        """Process a batch of embedding requests."""
        texts = []
        job_ids = []

        for _, job_data in batch:
            import json
            job = json.loads(job_data)
            texts.append(job["text"])
            job_ids.append(job["job_id"])

        # Generate embeddings
        try:
            response = await self.client.embeddings.create(
                model=self.model,
                input=texts,
                dimensions=self.dimensions,
            )

            # Store results
            for job_id, embedding_obj in zip(job_ids, response.data):
                embedding = embedding_obj.embedding
                await self.redis.set(
                    f"embedding:{job_id}",
                    json.dumps(embedding),
                    ex=3600,  # Cache for 1 hour
                )
                await self.redis.publish(
                    "embedding_complete",
                    json.dumps({"job_id": job_id, "status": "success"}),
                )

        except Exception as e:
            # Handle errors
            for job_id in job_ids:
                await self.redis.publish(
                    "embedding_complete",
                    json.dumps({"job_id": job_id, "status": "error", "error": str(e)}),
                )

    async def enqueue(self, text: str) -> str:
        """Add a text to the embedding queue."""
        import json
        import uuid

        job_id = str(uuid.uuid4())
        job = {"job_id": job_id, "text": text}

        await self.redis.rpush(
            "embedding_queue",
            json.dumps(job),
        )

        return job_id
```

### 3.5 Load Balancing

**AWS ALB Configuration:**

```yaml
# AWS Load Balancer Controller annotations
apiVersion: v1
kind: Service
metadata:
  name: openspace-api
  namespace: openspace
  annotations:
    service.beta.kubernetes.io/aws-load-balancer-type: "external"
    service.beta.kubernetes.io/aws-load-balancer-nlb-target-type: "ip"
    service.beta.kubernetes.io/aws-load-balancer-scheme: "internet-facing"
    service.beta.kubernetes.io/aws-load-balancer-ssl-ports: "443"
    service.beta.kubernetes.io/aws-load-balancer-ssl-cert: "arn:aws:acm:us-east-1:123456789012:certificate/xxx"
    service.beta.kubernetes.io/aws-load-balancer-backend-protocol: "tcp"
    service.beta.kubernetes.io/aws-load-balancer-healthcheck-protocol: "http"
    service.beta.kubernetes.io/aws-load-balancer-healthcheck-port: "8080"
    service.beta.kubernetes.io/aws-load-balancer-healthcheck-path: "/health"
    service.beta.kubernetes.io/aws-load-balancer-healthcheck-interval: "10"
    service.beta.kubernetes.io/aws-load-balancer-healthcheck-threshold: "2"
    service.beta.kubernetes.io/aws-load-balancer-healthcheck-timeout: "5"
    service.beta.kubernetes.io/aws-load-balancer-connection-draining-enabled: "true"
    service.beta.kubernetes.io/aws-load-balancer-connection-draining-timeout: "30"
spec:
  type: LoadBalancer
  ports:
  - name: https
    port: 443
    targetPort: 8080
    protocol: TCP
  selector:
    app: openspace-api
```

---

## 4. Database Operations

### 4.1 PostgreSQL Schema (Local)

```sql
-- SQLite Schema for Local OpenSpace Deployment
-- Note: Uses SQLite for local, PostgreSQL for cloud

-- Skill records table
CREATE TABLE IF NOT EXISTS skill_records (
    skill_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    directory TEXT NOT NULL,
    origin TEXT DEFAULT 'imported',
    parent_skill_ids TEXT,  -- JSON array
    generation INTEGER DEFAULT 0,

    -- Metrics
    total_selections INTEGER DEFAULT 0,
    total_applied INTEGER DEFAULT 0,
    total_successful INTEGER DEFAULT 0,
    total_fallbacks INTEGER DEFAULT 0,
    last_applied_at TIMESTAMP,

    -- Timestamps
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Skill tool dependencies
CREATE TABLE IF NOT EXISTS skill_tool_deps (
    skill_id TEXT,
    tool_name TEXT NOT NULL,
    backend TEXT,
    is_critical INTEGER DEFAULT 1,
    PRIMARY KEY (skill_id, tool_name),
    FOREIGN KEY (skill_id) REFERENCES skill_records(skill_id) ON DELETE CASCADE
);

-- Execution analyses
CREATE TABLE IF NOT EXISTS execution_analyses (
    analysis_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    recorded_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    total_tokens INTEGER,
    task_outcome TEXT,
    metadata TEXT  -- JSON
);

-- Skill judgments within analyses
CREATE TABLE IF NOT EXISTS skill_judgments (
    judgment_id TEXT PRIMARY KEY,
    analysis_id TEXT NOT NULL,
    skill_id TEXT NOT NULL,
    was_applied INTEGER,
    was_effective INTEGER,
    quality_score REAL,
    notes TEXT,
    FOREIGN KEY (analysis_id) REFERENCES execution_analyses(analysis_id) ON DELETE CASCADE,
    FOREIGN KEY (skill_id) REFERENCES skill_records(skill_id) ON DELETE CASCADE
);

-- Tool quality metrics
CREATE TABLE IF NOT EXISTS tool_quality_metrics (
    tool_key TEXT PRIMARY KEY,
    backend TEXT,
    success_count INTEGER DEFAULT 0,
    failure_count INTEGER DEFAULT 0,
    total_latency_ms REAL DEFAULT 0,
    last_success_at TIMESTAMP,
    last_failure_at TIMESTAMP,
    degradation_detected INTEGER DEFAULT 0,
    degradation_since TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_skill_records_name ON skill_records(name);
CREATE INDEX IF NOT EXISTS idx_skill_records_origin ON skill_records(origin);
CREATE INDEX IF NOT EXISTS idx_skill_judgments_skill_id ON skill_judgments(skill_id);
CREATE INDEX IF NOT EXISTS idx_skill_judgments_analysis_id ON skill_judgments(analysis_id);

-- Triggers for updated_at
CREATE TRIGGER IF NOT EXISTS update_skill_records_updated_at
    AFTER UPDATE ON skill_records
    FOR EACH ROW
    BEGIN
        UPDATE skill_records SET updated_at = CURRENT_TIMESTAMP WHERE skill_id = NEW.skill_id;
    END;
```

### 4.2 Skill Storage

**Directory Structure:**

```
skills/
├── document-gen-fallback/
│   ├── SKILL.md              # Skill definition
│   ├── .skill_id             # Unique identifier
│   ├── .upload_meta.json     # Upload metadata
│   └── examples/
│       └── example-usage.md
├── docker-monitor/
│   ├── SKILL.md
│   ├── .skill_id
│   └── .upload_meta.json
└── ...
```

**SKILL.md Format:**

```markdown
---
name: skill-name
description: One-line description
tags: [tag1, tag2]
---

# Skill Name

## When to Use

Circumstances that trigger this skill.

## Core Technique

The main approach or workflow.

## Step-by-Step Workflow

1. Step one with code examples
2. Step two with code examples
3. Step three with verification

## Complete Example

Full working example showing the skill in action.

## Troubleshooting

Common issues and how to resolve them.
```

**Upload Metadata:**

```json
{
  "origin": "derived",
  "visibility": "public",
  "parent_skill_ids": ["skill-abc123"],
  "tags": ["docker", "monitoring"],
  "created_by": "user@example.com",
  "change_summary": "Added memory monitoring and alerting"
}
```

### 4.3 Version Tracking

```sql
-- Version tracking query
SELECT
    sr.skill_id,
    sr.name,
    sr.generation,
    sr.origin,
    STRING_AGG(parent.name, ', ') as parent_skills,
    sr.created_at,
    sr.total_applied,
    sr.total_successful
FROM skill_records sr
LEFT JOIN skill_lineage_parents slp ON sr.skill_id = slp.skill_id
LEFT JOIN skill_records parent ON slp.parent_skill_ids && ARRAY[parent.skill_id]
GROUP BY sr.skill_id, sr.name, sr.generation, sr.origin, sr.created_at
ORDER BY sr.created_at DESC;
```

### 4.4 Backup Strategies

**PostgreSQL Backup Script:**

```bash
#!/bin/bash
# backup-postgresql.sh

set -e

BACKUP_DIR="/var/backups/openspace"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RETENTION_DAYS=7

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Database connection
DB_HOST="${DB_HOST:-localhost}"
DB_NAME="${DB_NAME:-openspace}"
DB_USER="${DB_USER:-openspace}"

# Full backup with pg_dump
echo "Starting PostgreSQL backup at $(date)"
pg_dump -h "$DB_HOST" -U "$DB_USER" -d "$DB_NAME" \
    --format=custom \
    --compress=9 \
    --verbose \
    > "$BACKUP_DIR/openspace_full_$TIMESTAMP.dump"

# Schema-only backup
pg_dump -h "$DB_HOST" -U "$DB_USER" -d "$DB_NAME" \
    --schema-only \
    > "$BACKUP_DIR/openspace_schema_$TIMESTAMP.sql"

# Compress backups
gzip "$BACKUP_DIR/openspace_full_$TIMESTAMP.dump"
gzip "$BACKUP_DIR/openspace_schema_$TIMESTAMP.sql"

# Upload to S3 (if configured)
if [ -n "$AWS_BUCKET" ]; then
    aws s3 cp "$BACKUP_DIR/" "s3://$AWS_BUCKET/backups/" \
        --recursive --exclude "*" \
        --include "openspace_*_$TIMESTAMP*"
fi

# Clean old backups
find "$BACKUP_DIR" -name "openspace_*.dump.gz" -mtime +$RETENTION_DAYS -delete
find "$BACKUP_DIR" -name "openspace_*.sql.gz" -mtime +$RETENTION_DAYS -delete

echo "Backup completed at $(date)"
```

**Automated Backup Cron:**

```bash
# /etc/cron.d/openspace-backup
# PostgreSQL backup - daily at 3 AM
0 3 * * * postgres /usr/local/bin/backup-postgresql.sh >> /var/log/openspace/backup.log 2>&1
```

**Point-in-Time Recovery Configuration:**

```conf
# postgresql.conf for PITR
wal_level = replica
archive_mode = on
archive_command = 'wal-g wal-push %p'
archive_timeout = 300

# Recovery settings
restore_command = 'wal-g wal-fetch %f %p'
recovery_target_timeline = 'latest'
```

### 4.5 Migration Handling

**Migration Script Template:**

```python
# migrations/001_add_skill_tags.py
"""
Migration: Add tags column to skill_records

Created: 2024-01-15
"""

def upgrade(conn):
    """Apply the migration."""
    conn.execute("""
        ALTER TABLE skill_records
        ADD COLUMN IF NOT EXISTS tags TEXT[] DEFAULT '{}'
    """)

    # Backfill existing records
    conn.execute("""
        UPDATE skill_records
        SET tags = ARRAY[origin]
        WHERE tags IS NULL OR tags = '{}'
    """)

def downgrade(conn):
    """Rollback the migration."""
    conn.execute("""
        ALTER TABLE skill_records
        DROP COLUMN IF EXISTS tags
    """)
```

**Migration Runner:**

```python
# migrate.py
import asyncio
import asyncpg
from pathlib import Path
from typing import Callable

MIGRATIONS_DIR = Path(__file__).parent / "migrations"

async def run_migration(conn, migration_file: Path, direction: str):
    """Run a single migration."""
    module_name = migration_file.stem
    spec = __import__.util.spec_from_file_location(module_name, migration_file)
    module = __import__.util.module_from_spec(spec)
    spec.loader.exec_module(module)

    if direction == "up":
        module.upgrade(conn)
    else:
        module.downgrade(conn)

async def migrate(database_url: str, target_version: str = None):
    """Run all pending migrations."""
    conn = await asyncpg.connect(database_url)

    # Create migrations tracking table
    await conn.execute("""
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version TEXT PRIMARY KEY,
            applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
    """)

    # Get applied migrations
    applied = set(row[0] for row in await conn.fetch(
        "SELECT version FROM schema_migrations ORDER BY version"
    ))

    # Get migration files
    migration_files = sorted(MIGRATIONS_DIR.glob("*.py"))

    for migration_file in migration_files:
        version = migration_file.stem.split("_")[0]

        if version not in applied:
            print(f"Applying migration: {migration_file.name}")

            async with conn.transaction():
                await run_migration(conn, migration_file, "up")

            await conn.execute(
                "INSERT INTO schema_migrations (version) VALUES ($1)",
                version,
            )

    await conn.close()

if __name__ == "__main__":
    import os
    database_url = os.getenv("DATABASE_URL")
    asyncio.run(migrate(database_url))
```

---

## 5. MCP Server Production

### 5.1 Process Management

**Systemd Service:**

```ini
# /etc/systemd/system/openspace-mcp.service
[Unit]
Description=OpenSpace MCP Server
Documentation=https://github.com/HKUDS/OpenSpace
After=network.target

[Service]
Type=notify
User=openspace
Group=openspace

# Environment
Environment="PYTHONUNBUFFERED=1"
Environment="PYTHONDONTWRITEBYTECODE=1"
Environment="OPENSPACE_MODEL=openrouter/anthropic/claude-sonnet-4.5"
EnvironmentFile=/etc/openspace/env

# Working directory
WorkingDirectory=/opt/openspace

# Main process
ExecStart=/opt/openspace/.venv/bin/openspace-mcp
ExecReload=/bin/kill -HUP $MAINPID

# Restart policy
Restart=always
RestartSec=5
StartLimitInterval=60
StartLimitBurst=5

# Resource limits
LimitNOFILE=65535
LimitNPROC=4096
MemoryMax=2G
CPUQuota=80%

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=read-only
PrivateTmp=true
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
RestrictSUIDSGID=true
RemoveIPC=true

# Capabilities
CapabilityBoundingSet=
AmbientCapabilities=

# Sandboxing
PrivateDevices=true
ProtectClock=true
ProtectKernelLogs=true
RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX
RestrictNamespaces=true
LockPersonality=true
MemoryDenyWriteExecute=true
RestrictRealtime=true
RestrictSockets=true
SystemCallArchitectures=native
SystemCallFilter=@system-service

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=openspace-mcp

[Install]
WantedBy=multi-user.target
```

**Supervisord Configuration:**

```ini
# /etc/supervisor/conf.d/openspace.conf
[program:openspace-mcp]
command=/opt/openspace/.venv/bin/openspace-mcp
directory=/opt/openspace
user=openspace
autostart=true
autorestart=true
startretries=5
retrywait=5

# Environment
environment=PYTHONUNBUFFERED="1",OPENSPACE_MODEL="openrouter/anthropic/claude-sonnet-4.5"

# Logging
stdout_logfile=/var/log/openspace/mcp.log
stderr_logfile=/var/log/openspace/mcp_error.log
stdout_logfile_maxbytes=50MB
stderr_logfile_maxbytes=50MB
stdout_logfile_backups=5
stderr_logfile_backups=5

# Resource limits
umask=022
priority=100
```

### 5.2 Resource Limits

**Docker Resource Limits:**

```yaml
# docker-compose with resource limits
services:
  openspace-mcp:
    image: openspace-mcp:latest
    deploy:
      resources:
        limits:
          cpus: '2.0'
          memory: 2G
        reservations:
          cpus: '0.5'
          memory: 512M
          devices:
          - driver: nvidia
            device_ids: ['0']
            capabilities: [gpu]
```

**Kubernetes Resource Quotas:**

```yaml
# ResourceQuota for namespace
apiVersion: v1
kind: ResourceQuota
metadata:
  name: openspace-quota
  namespace: openspace
spec:
  hard:
    requests.cpu: "10"
    requests.memory: 20Gi
    limits.cpu: "20"
    limits.memory: 40Gi
    persistentvolumeclaims: "10"
    secrets: "20"
    configmaps: "20"

---
# LimitRange for default limits
apiVersion: v1
kind: LimitRange
metadata:
  name: openspace-limits
  namespace: openspace
spec:
  limits:
  - type: Container
    default:
      cpu: "1"
      memory: 1Gi
    defaultRequest:
      cpu: "250m"
      memory: 512Mi
    max:
      cpu: "4"
      memory: 4Gi
    min:
      cpu: "100m"
      memory: 128Mi
```

### 5.3 Logging

**Structured Logging Configuration:**

```python
# logging_config.py
import logging
import json
from pythonjsonlogger import jsonlogger

class CustomJsonFormatter(jsonlogger.JsonFormatter):
    """Custom JSON formatter for OpenSpace logs."""

    def add_fields(self, log_record, record, message_dict):
        super().add_fields(log_record, record, message_dict)
        log_record['service'] = 'openspace-mcp'
        log_record['level'] = record.levelname
        log_record['logger'] = record.name

LOGGING_CONFIG = {
    'version': 1,
    'disable_existing_loggers': False,
    'formatters': {
        'json': {
            '()': CustomJsonFormatter,
            'format': '%(asctime)s %(name)s %(levelname)s %(message)s',
        },
        'verbose': {
            'format': '%(asctime)s - %(name)s - %(levelname)s - %(message)s'
        },
    },
    'handlers': {
        'console': {
            'class': 'logging.StreamHandler',
            'formatter': 'json',
            'stream': 'ext://sys.stdout',
        },
        'file': {
            'class': 'logging.handlers.RotatingFileHandler',
            'formatter': 'json',
            'filename': '/var/log/openspace/mcp.log',
            'maxBytes': 52428800,  # 50MB
            'backupCount': 5,
        },
        'error_file': {
            'class': 'logging.handlers.RotatingFileHandler',
            'formatter': 'json',
            'filename': '/var/log/openspace/mcp_error.log',
            'level': 'ERROR',
            'maxBytes': 52428800,
            'backupCount': 5,
        },
    },
    'loggers': {
        'openspace': {
            'handlers': ['console', 'file', 'error_file'],
            'level': 'INFO',
            'propagate': False,
        },
        'openspace.skill_registry': {
            'handlers': ['console', 'file'],
            'level': 'DEBUG',
            'propagate': False,
        },
        'openspace.skill_evolver': {
            'handlers': ['console', 'file', 'error_file'],
            'level': 'INFO',
            'propagate': False,
        },
    },
    'root': {
        'handlers': ['console', 'file'],
        'level': 'WARNING',
    },
}
```

**Log Aggregation (Loki + Promtail):**

```yaml
# promtail-config.yaml
server:
  http_listen_port: 9080
  grpc_listen_port: 0

positions:
  filename: /tmp/positions.yaml

clients:
  - url: http://loki:3100/loki/api/v1/push

scrape_configs:
  - job_name: openspace-mcp
    static_configs:
      - targets:
          - localhost
        labels:
          job: openspace-mcp
          __path__: /var/log/openspace/*.log

    pipeline_stages:
      - json:
          expressions:
            level: level
            logger: logger
      - labels:
          level:
          logger:
```

### 5.4 Monitoring

**Prometheus Metrics:**

```python
# metrics.py
from prometheus_client import Counter, Histogram, Gauge, generate_latest
import time

# Metric definitions
REQUESTS_TOTAL = Counter(
    'openspace_requests_total',
    'Total number of requests',
    ['method', 'endpoint', 'status'],
)

REQUEST_DURATION = Histogram(
    'openspace_request_duration_seconds',
    'Request duration in seconds',
    ['method', 'endpoint'],
    buckets=[0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0],
)

SKILLS_TOTAL = Gauge(
    'openspace_skills_total',
    'Total number of skills',
    ['origin', 'visibility'],
)

SKILL_EXECUTIONS = Counter(
    'openspace_skill_executions_total',
    'Total skill executions',
    ['skill_name', 'outcome'],
)

SKILL_EVOLUTIONS = Counter(
    'openspace_skill_evolutions_total',
    'Total skill evolutions',
    ['evolution_type'],
)

TOOL_CALLS = Counter(
    'openspace_tool_calls_total',
    'Total tool calls',
    ['tool_name', 'backend', 'success'],
)

TOOL_LATENCY = Histogram(
    'openspace_tool_latency_seconds',
    'Tool call latency',
    ['tool_name', 'backend'],
    buckets=[0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0],
)

LLM_TOKENS = Counter(
    'openspace_llm_tokens_total',
    'Total LLM tokens used',
    ['model', 'type'],  # type: prompt or completion
)

ACTIVE_CONNECTIONS = Gauge(
    'openspace_active_connections',
    'Number of active MCP connections',
)

# Context managers and decorators
def track_request(method: str, endpoint: str):
    """Decorator to track request metrics."""
    def decorator(func):
        async def wrapper(*args, **kwargs):
            start_time = time.time()
            try:
                result = await func(*args, **kwargs)
                status = 'success'
                return result
            except Exception as e:
                status = 'error'
                raise
            finally:
                duration = time.time() - start_time
                REQUESTS_TOTAL.labels(method, endpoint, status).inc()
                REQUEST_DURATION.labels(method, endpoint).observe(duration)
        return wrapper
    return decorator
```

### 5.5 Health Checks

**Health Check Endpoints:**

```python
# health.py
from fastapi import APIRouter, Response
from prometheus_client import generate_latest
import asyncio
import asyncpg

router = APIRouter()

@router.get("/health/live")
async def liveness_probe() -> Response:
    """Check if the service is running."""
    return Response("OK", media_type="text/plain")

@router.get("/health/ready")
async def readiness_probe() -> Response:
    """Check if the service is ready to accept traffic."""
    # Check database connection
    try:
        conn = await asyncpg.connect(DATABASE_URL)
        await conn.fetchval("SELECT 1")
        await conn.close()
    except Exception as e:
        return Response(f"Database error: {e}", status_code=503)

    # Check cloud connectivity (if configured)
    if CLOUD_ENABLED:
        try:
            await cloud_client.health_check()
        except Exception as e:
            return Response(f"Cloud error: {e}", status_code=503)

    return Response("OK", media_type="text/plain")

@router.get("/health/startup")
async def startup_probe() -> Response:
    """Check if the application has completed startup."""
    if not app_started:
        return Response("Starting up", status_code=503)
    return Response("OK", media_type="text/plain")

@router.get("/metrics")
async def metrics() -> Response:
    """Prometheus metrics endpoint."""
    return Response(
        generate_latest(),
        media_type="text/plain; charset=utf-8",
    )

@router.get("/health/skills")
async def skills_health() -> dict:
    """Detailed skill system health."""
    return {
        "total_skills": skill_store.count(),
        "imported": skill_store.count_by_origin("imported"),
        "derived": skill_store.count_by_origin("derived"),
        "fixed": skill_store.count_by_origin("fixed"),
        "captured": skill_store.count_by_origin("captured"),
    }
```

**Kubernetes Health Probes:**

```yaml
livenessProbe:
  httpGet:
    path: /health/live
    port: 8080
  initialDelaySeconds: 15
  periodSeconds: 20
  timeoutSeconds: 5
  failureThreshold: 3

readinessProbe:
  httpGet:
    path: /health/ready
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 10
  timeoutSeconds: 3
  failureThreshold: 3

startupProbe:
  httpGet:
    path: /health/startup
    port: 8080
  initialDelaySeconds: 0
  periodSeconds: 5
  timeoutSeconds: 3
  failureThreshold: 30  # Up to 150 seconds for startup
```

---

## 6. Security

### 6.1 API Key Management

**API Key Generation:**

```python
# api_keys.py
import secrets
import hashlib
import hmac
from typing import Optional, Tuple

def generate_api_key() -> Tuple[str, str]:
    """Generate a new API key with prefix.

    Returns:
        Tuple of (full_key, prefix)
    """
    # Generate 32 random bytes
    key_bytes = secrets.token_bytes(32)
    full_key = f"sk_{secrets.token_urlsafe(32)}"

    # Prefix is first 8 characters for identification
    prefix = full_key[:12]

    return full_key, prefix

def hash_api_key(api_key: str) -> str:
    """Hash an API key for storage."""
    return hashlib.sha256(api_key.encode()).hexdigest()

def verify_api_key(api_key: str, hashed_key: str) -> bool:
    """Verify an API key against its hash."""
    return hmac.compare_digest(hash_api_key(api_key), hashed_key)

def extract_key_info(api_key: str) -> dict:
    """Extract information from an API key."""
    if not api_key.startswith("sk_"):
        raise ValueError("Invalid API key format")

    return {
        "prefix": api_key[:12],
        "length": len(api_key),
    }
```

**API Key Rotation:**

```python
# key_rotation.py
import asyncio
from datetime import datetime, timedelta

class APIKeyRotator:
    """Handle API key rotation."""

    def __init__(self, db_pool):
        self.db_pool = db_pool
        self.rotation_period_days = 90

    async def rotate_key(self, user_id: str) -> str:
        """Rotate a user's API key."""
        new_key, prefix = generate_api_key()
        hashed_key = hash_api_key(new_key)

        async with self.db_pool.acquire() as conn:
            # Store new key
            await conn.execute("""
                UPDATE users
                SET api_key_hash = $1,
                    api_key_prefix = $2,
                    updated_at = NOW()
                WHERE id = $3
            """, hashed_key, prefix, user_id)

            # Log rotation
            await conn.execute("""
                INSERT INTO api_key_rotations (user_id, rotated_at)
                VALUES ($1, NOW())
            """, user_id)

        return new_key

    async def check_expiration(self, user_id: str) -> bool:
        """Check if a user's key is due for rotation."""
        async with self.db_pool.acquire() as conn:
            row = await conn.fetchrow("""
                SELECT updated_at
                FROM users
                WHERE id = $1
            """, user_id)

            if row is None:
                return True

            age = datetime.now() - row['updated_at']
            return age > timedelta(days=self.rotation_period_days)

    async def rotation_reminder_job(self):
        """Scheduled job to remind users of key rotation."""
        async with self.db_pool.acquire() as conn:
            users = await conn.fetch("""
                SELECT id, email, updated_at
                FROM users
                WHERE updated_at < NOW() - INTERVAL '75 days'
            """)

            for user in users:
                await self._send_rotation_email(user['id'], user['email'])
```

### 6.2 Skill Validation

**Skill Security Validator:**

```python
# skill_validator.py
import re
from pathlib import Path
from typing import List, Tuple

class SkillSecurityValidator:
    """Validate skills for security issues."""

    # Patterns to detect dangerous content
    DANGEROUS_PATTERNS = [
        (r'rm\s+-rf\s+/', 'Destructive rm command'),
        (r'mkfs', 'Filesystem creation'),
        (r':\(\)\{:\|:&\};:', 'Fork bomb'),
        (r'dd\s+if=/dev/zero', 'Disk write'),
        (r'wget.*\|\s*sh', 'Pipe to shell'),
        (r'curl.*\|\s*sh', 'Pipe to shell'),
        (r'eval\s*\(', 'Eval usage'),
        (r'exec\s*\(', 'Exec usage'),
        (r'__import__', 'Dynamic import'),
        (r'subprocess', 'Subprocess execution'),
        (r'os\.system', 'System command'),
        (r'pickle', 'Pickle deserialization'),
    ]

    ALLOWED_PATHS = [
        '/workspace',
        '/tmp',
        '/var/tmp',
    ]

    def __init__(self):
        self.compiled_patterns = [
            (re.compile(pattern, re.IGNORECASE), description)
            for pattern, description in self.DANGEROUS_PATTERNS
        ]

    def validate_skill(self, skill_dir: Path) -> Tuple[bool, List[str]]:
        """Validate a skill directory for security issues.

        Returns:
            Tuple of (is_valid, list_of_issues)
        """
        issues = []

        # Check SKILL.md
        skill_file = skill_dir / "SKILL.md"
        if skill_file.exists():
            content = skill_file.read_text()
            issues.extend(self._check_content(content, "SKILL.md"))

        # Check all .md files
        for md_file in skill_dir.glob("*.md"):
            content = md_file.read_text()
            issues.extend(self._check_content(content, md_file.name))

        # Check for suspicious file types
        for file_path in skill_dir.rglob("*"):
            if file_path.suffix in ['.exe', '.dll', '.so', '.dylib']:
                issues.append(f"Binary file detected: {file_path.name}")

        return len(issues) == 0, issues

    def _check_content(self, content: str, filename: str) -> List[str]:
        """Check content for dangerous patterns."""
        issues = []

        for pattern, description in self.compiled_patterns:
            matches = pattern.findall(content)
            if matches:
                issues.append(
                    f"{filename}: {description} - found '{matches[0]}'"
                )

        return issues

    def validate_paths(self, paths: List[str]) -> Tuple[bool, List[str]]:
        """Validate that paths are within allowed directories."""
        issues = []

        for path in paths:
            is_allowed = any(
                path.startswith(allowed)
                for allowed in self.ALLOWED_PATHS
            )
            if not is_allowed:
                issues.append(f"Path outside allowed directories: {path}")

        return len(issues) == 0, issues
```

### 6.3 Sandbox Execution

**Shell Sandbox Configuration:**

```yaml
# sandbox-config.yaml
security:
  # Blocked commands (exact match)
  blocked_commands:
    - "rm -rf /"
    - "rm -rf /*"
    - "mkfs"
    - "mkfs.ext4"
    - "mkfs.xfs"
    - ":(){:|:&};:"
    - "dd if=/dev/zero"
    - "wget -O- | sh"
    - "curl -s | sh"

  # Blocked command patterns (regex)
  blocked_patterns:
    - "^rm\\s+-rf\\s+/$"
    - "^\\s*:\\(\\)\\{:\\|:&\\};:"
    - "dd\\s+if=/dev/(zero|random|urandom)"

  # Allowed directories
  allowed_paths:
    - /workspace
    - /tmp
    - /var/tmp

  # Blocked directories
  blocked_paths:
    - /etc
    - /root
    - /home
    - /var/log
    - /proc
    - /sys

  # Resource limits for commands
  limits:
    max_memory_mb: 2048
    max_cpu_percent: 80
    max_time_seconds: 300
    max_output_size_mb: 50
    max_file_descriptors: 256

  # Network restrictions
  network:
    allow_outbound: true
    blocked_ports:
      - 22   # SSH
      - 23   # Telnet
      - 3389 # RDP
    allowed_hosts:
      - "api.open-space.cloud"
      - "*.openrouter.ai"
```

**Sandbox Implementation:**

```python
# sandbox.py
import asyncio
import os
import signal
from pathlib import Path
from typing import Optional, Tuple
import subprocess

class SandboxExecutor:
    """Execute commands in a sandboxed environment."""

    def __init__(self, config: dict):
        self.config = config
        self.blocked_commands = set(config.get('blocked_commands', []))
        self.allowed_paths = config.get('allowed_paths', [])
        self.blocked_paths = config.get('blocked_paths', [])

    async def execute(
        self,
        command: str,
        cwd: Optional[str] = None,
        timeout: Optional[int] = None,
    ) -> Tuple[int, str, str]:
        """Execute a command in the sandbox.

        Returns:
            Tuple of (return_code, stdout, stderr)
        """
        # Validate command
        if not self._is_command_allowed(command):
            return -1, "", "Command blocked by security policy"

        # Validate working directory
        if cwd and not self._is_path_allowed(cwd):
            return -1, "", "Working directory not allowed"

        # Set up process limits
        limits = self.config.get('limits', {})

        try:
            process = await asyncio.create_subprocess_shell(
                command,
                cwd=cwd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
                preexec_fn=lambda: self._apply_limits(limits),
            )

            try:
                stdout, stderr = await asyncio.wait_for(
                    process.communicate(),
                    timeout=timeout or self.config.get('max_time_seconds', 300),
                )

                return (
                    process.returncode,
                    stdout.decode()[:10000],  # Limit output size
                    stderr.decode()[:10000],
                )

            except asyncio.TimeoutError:
                process.kill()
                await process.wait()
                return -1, "", "Command timed out"

        except Exception as e:
            return -1, "", f"Execution error: {e}"

    def _is_command_allowed(self, command: str) -> bool:
        """Check if command is allowed."""
        # Check blocked commands
        if command in self.blocked_commands:
            return False

        # Check blocked patterns
        import re
        for pattern in self.config.get('blocked_patterns', []):
            if re.search(pattern, command):
                return False

        return True

    def _is_path_allowed(self, path: str) -> bool:
        """Check if path is allowed."""
        # Check if in blocked paths
        for blocked in self.blocked_paths:
            if path.startswith(blocked):
                return False

        # Check if in allowed paths
        for allowed in self.allowed_paths:
            if path.startswith(allowed):
                return True

        return False

    def _apply_limits(self, limits: dict) -> None:
        """Apply resource limits to process."""
        # Set memory limit
        max_memory = limits.get('max_memory_mb', 2048) * 1024 * 1024
        try:
            import resource
            resource.setrlimit(resource.RLIMIT_AS, (max_memory, max_memory))
        except (ImportError, ValueError):
            pass

        # Set file descriptor limit
        max_fds = limits.get('max_file_descriptors', 256)
        try:
            import resource
            resource.setrlimit(resource.RLIMIT_NOFILE, (max_fds, max_fds))
        except (ImportError, ValueError):
            pass
```

### 6.4 Rate Limiting

**Redis-based Rate Limiter:**

```python
# rate_limiter.py
import time
from typing import Optional
from redis.asyncio import Redis

class RateLimiter:
    """Token bucket rate limiter using Redis."""

    def __init__(
        self,
        redis: Redis,
        default_rate: int = 100,  # requests per window
        default_window: int = 60,  # seconds
    ):
        self.redis = redis
        self.default_rate = default_rate
        self.default_window = default_window

    async def is_allowed(
        self,
        key: str,
        rate: Optional[int] = None,
        window: Optional[int] = None,
    ) -> Tuple[bool, dict]:
        """Check if request is allowed.

        Returns:
            Tuple of (is_allowed, rate_limit_info)
        """
        rate = rate or self.default_rate
        window = window or self.default_window

        current_time = int(time.time())
        window_key = f"ratelimit:{key}:{current_time // window}"

        # Use Lua script for atomic increment
        script = """
        local current = redis.call('INCR', KEYS[1])
        if current == 1 then
            redis.call('EXPIRE', KEYS[1], ARGV[1])
        end
        return current
        """

        current = await self.redis.eval(script, 1, window_key, window)

        remaining = max(0, rate - current)
        reset_time = ((current_time // window) + 1) * window

        return current <= rate, {
            'limit': rate,
            'remaining': remaining,
            'reset': reset_time,
            'retry_after': None if current <= rate else reset_time - current_time,
        }

    async def get_quota_status(self, key: str) -> dict:
        """Get current quota status."""
        keys = await self.redis.keys(f"ratelimit:{key}:*")
        total_used = 0

        for key in keys:
            count = await self.redis.get(key)
            if count:
                total_used += int(count)

        return {
            'used': total_used,
            'keys': len(keys),
        }
```

**Rate Limit Headers:**

```python
# rate_limit_middleware.py
from fastapi import Request, Response
from fastapi.responses import JSONResponse

async def rate_limit_middleware(request: Request, call_next):
    """Add rate limiting to requests."""
    client_ip = request.client.host
    user_id = request.headers.get("X-User-ID", client_ip)

    # Check rate limit
    is_allowed, info = await rate_limiter.is_allowed(user_id)

    if not is_allowed:
        response = JSONResponse(
            status_code=429,
            content={"error": "Rate limit exceeded"},
            headers={
                "X-RateLimit-Limit": str(info['limit']),
                "X-RateLimit-Remaining": "0",
                "X-RateLimit-Reset": str(info['reset']),
                "Retry-After": str(info['retry_after']),
            }
        )
        return response

    # Proceed with request
    response = await call_next(request)

    # Add rate limit headers to response
    response.headers["X-RateLimit-Limit"] = str(info['limit'])
    response.headers["X-RateLimit-Remaining"] = str(info['remaining'])
    response.headers["X-RateLimit-Reset"] = str(info['reset'])

    return response
```

### 6.5 Input Sanitization

**Input Sanitizer:**

```python
# input_sanitizer.py
import html
import re
from typing import Any, Union

class InputSanitizer:
    """Sanitize user inputs to prevent injection attacks."""

    # SQL injection patterns
    SQL_PATTERNS = [
        r"(\b(SELECT|INSERT|UPDATE|DELETE|DROP|UNION|ALTER)\b)",
        r"(--)|(;)|(\/\*)|(\*\/)",
        r"(\bOR\b\s+\d+\s*=\s*\d+)",
        r"(\bAND\b\s+\d+\s*=\s*\d+)",
    ]

    # XSS patterns
    XSS_PATTERNS = [
        r"<script[^>]*>.*?</script>",
        r"javascript:",
        r"on\w+\s*=",
        r"<iframe[^>]*>",
        r"<object[^>]*>",
    ]

    # Path traversal
    PATH_TRAVERSAL = r"\.\.[\/\\]"

    def __init__(self):
        self.sql_regex = [re.compile(p, re.IGNORECASE) for p in self.SQL_PATTERNS]
        self.xss_regex = [re.compile(p, re.IGNORECASE | re.DOTALL) for p in self.XSS_PATTERNS]
        self.path_traversal_regex = re.compile(self.PATH_TRAVERSAL)

    def sanitize_string(self, value: str, context: str = "text") -> str:
        """Sanitize a string value."""
        if not value:
            return value

        # Remove null bytes
        value = value.replace('\x00', '')

        if context == "html":
            # Escape HTML
            value = html.escape(value)
        elif context == "sql":
            # Check for SQL injection
            for pattern in self.sql_regex:
                if pattern.search(value):
                    raise ValueError("Potential SQL injection detected")
        elif context == "path":
            # Check for path traversal
            if self.path_traversal_regex.search(value):
                raise ValueError("Path traversal detected")
            # Normalize path
            value = re.sub(r'[\/\\]+', '/', value)

        # Check for XSS in all contexts
        for pattern in self.xss_regex:
            value = pattern.sub('', value)

        return value

    def sanitize_dict(self, data: dict, skip_fields: list = None) -> dict:
        """Sanitize all string values in a dictionary."""
        skip_fields = skip_fields or []
        sanitized = {}

        for key, value in data.items():
            if key in skip_fields:
                sanitized[key] = value
            elif isinstance(value, str):
                sanitized[key] = self.sanitize_string(value)
            elif isinstance(value, dict):
                sanitized[key] = self.sanitize_dict(value, skip_fields)
            elif isinstance(value, list):
                sanitized[key] = [
                    self.sanitize_string(v) if isinstance(v, str) else v
                    for v in value
                ]
            else:
                sanitized[key] = value

        return sanitized
```

---

## 7. Monitoring

### 7.1 Skill Metrics

**Metrics Dashboard (Grafana):**

```json
{
  "dashboard": {
    "title": "OpenSpace Skill Metrics",
    "panels": [
      {
        "title": "Skill Executions Over Time",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(openspace_skill_executions_total[5m])",
            "legendFormat": "{{skill_name}} - {{outcome}}"
          }
        ]
      },
      {
        "title": "Skill Evolution Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(openspace_skill_evolutions_total[1h])",
            "legendFormat": "{{evolution_type}}"
          }
        ]
      },
      {
        "title": "Top Skills by Usage",
        "type": "table",
        "targets": [
          {
            "expr": "topk(10, sum(increase(openspace_skill_executions_total[24h])) by (skill_name))"
          }
        ]
      },
      {
        "title": "Skill Success Rate",
        "type": "gauge",
        "targets": [
          {
            "expr": "sum(rate(openspace_skill_executions_total{outcome='success'}[5m])) / sum(rate(openspace_skill_executions_total[5m])) * 100"
          }
        ]
      }
    ]
  }
}
```

### 7.2 Evolution Tracking

**Evolution Tracking Query:**

```sql
-- Skill evolution tracking query
WITH RECURSIVE evolution_chain AS (
    -- Base case: root skills
    SELECT
        sr.skill_id,
        sr.name,
        sr.origin,
        sr.generation,
        sr.created_at,
        ARRAY[sr.skill_id] as chain,
        1 as depth
    FROM skill_records sr
    WHERE sr.parent_skill_ids IS NULL
       OR sr.parent_skill_ids = '{}'

    UNION ALL

    -- Recursive case: child skills
    SELECT
        sr.skill_id,
        sr.name,
        sr.origin,
        sr.generation,
        sr.created_at,
        ec.chain || sr.skill_id,
        ec.depth + 1
    FROM skill_records sr
    JOIN evolution_chain ec ON sr.skill_id = ANY(ec.chain)
)
SELECT
    skill_id,
    name,
    origin,
    generation,
    created_at,
    depth,
    array_length(chain, 1) as lineage_length
FROM evolution_chain
ORDER BY created_at DESC;
```

### 7.3 Tool Quality

**Tool Quality Monitoring:**

```python
# tool_quality_monitor.py
from typing import Dict, Optional
from dataclasses import dataclass
from datetime import datetime, timedelta

@dataclass
class ToolMetrics:
    success_count: int
    failure_count: int
    total_latency_ms: float
    last_success_at: Optional[datetime]
    last_failure_at: Optional[datetime]

class ToolQualityManager:
    """Monitor and track tool quality metrics."""

    DEGRADATION_THRESHOLD = 0.7  # 70% success rate
    MIN_SAMPLES = 10  # Minimum samples before degradation detection

    def __init__(self, db_pool):
        self.db_pool = db_pool
        self.degraded_tools: Dict[str, datetime] = {}

    def record_outcome(
        self,
        tool_key: str,
        success: bool,
        latency_ms: float,
    ) -> None:
        """Record a tool execution outcome."""
        # Update metrics in database
        asyncio.create_task(self._update_metrics(tool_key, success, latency_ms))

        # Check for degradation
        asyncio.create_task(self._check_degradation(tool_key))

    async def _update_metrics(
        self,
        tool_key: str,
        success: bool,
        latency_ms: float,
    ) -> None:
        """Update tool metrics in database."""
        async with self.db_pool.acquire() as conn:
            await conn.execute("""
                INSERT INTO tool_quality_metrics (tool_key, success_count, failure_count, total_latency_ms)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (tool_key) DO UPDATE SET
                    success_count = tool_quality_metrics.success_count + $2,
                    failure_count = tool_quality_metrics.failure_count + $3,
                    total_latency_ms = tool_quality_metrics.total_latency_ms + $4,
                    last_success_at = CASE WHEN $2 > 0 THEN NOW() ELSE tool_quality_metrics.last_success_at END,
                    last_failure_at = CASE WHEN $3 > 0 THEN NOW() ELSE tool_quality_metrics.last_failure_at END,
                    updated_at = NOW()
            """,
                tool_key,
                1 if success else 0,
                0 if success else 1,
                latency_ms,
            )

    async def _check_degradation(self, tool_key: str) -> None:
        """Check if tool is degraded."""
        async with self.db_pool.acquire() as conn:
            row = await conn.fetchrow("""
                SELECT success_count, failure_count, degradation_detected
                FROM tool_quality_metrics
                WHERE tool_key = $1
            """, tool_key)

            if row is None:
                return

            total = row['success_count'] + row['failure_count']

            # Need minimum samples
            if total < self.MIN_SAMPLES:
                return

            success_rate = row['success_count'] / total

            # Check for degradation
            if success_rate < self.DEGRADATION_THRESHOLD:
                if tool_key not in self.degraded_tools:
                    self.degraded_tools[tool_key] = datetime.now()
                    await self._on_degradation_detected(tool_key, success_rate)
            else:
                # Tool recovered
                if tool_key in self.degraded_tools:
                    del self.degraded_tools[tool_key]

    async def _on_degradation_detected(
        self,
        tool_key: str,
        success_rate: float,
    ) -> None:
        """Handle tool degradation detection."""
        # Update database
        async with self.db_pool.acquire() as conn:
            await conn.execute("""
                UPDATE tool_quality_metrics
                SET degradation_detected = true,
                    degradation_since = NOW()
                WHERE tool_key = $1
            """, tool_key)

        # Log the degradation
        print(f"Tool degradation detected: {tool_key} (success_rate: {success_rate:.2%})")

        # Trigger skill evolution for dependent skills
        await self._trigger_skill_evolution(tool_key)

    async def _trigger_skill_evolution(self, tool_key: str) -> None:
        """Trigger evolution for skills depending on this tool."""
        # This would integrate with the SkillEvolver
        pass
```

### 7.4 Error Tracking

**Error Tracking with Sentry:**

```python
# error_tracking.py
import sentry_sdk
from sentry_sdk.integrations.asyncio import AsyncioIntegration
from sentry_sdk.integrations.logging import LoggingIntegration

def init_error_tracking(dsn: str, environment: str):
    """Initialize error tracking."""
    sentry_sdk.init(
        dsn=dsn,
        environment=environment,
        integrations=[
            AsyncioIntegration(),
            LoggingIntegration(
                level=logging.INFO,
                event_level=logging.ERROR,
            ),
        ],
        traces_sample_rate=0.1,
        profiles_sample_rate=0.1,
    )

def capture_skill_error(skill_id: str, error: Exception, context: dict):
    """Capture a skill execution error."""
    sentry_sdk.set_tag("skill_id", skill_id)
    sentry_sdk.set_context("skill_context", context)
    sentry_sdk.capture_exception(error)

def capture_tool_error(tool_name: str, error: Exception, context: dict):
    """Capture a tool execution error."""
    sentry_sdk.set_tag("tool_name", tool_name)
    sentry_sdk.set_tag("backend", context.get("backend", "unknown"))
    sentry_sdk.set_context("tool_context", context)
    sentry_sdk.capture_exception(error)
```

### 7.5 Performance Monitoring

**APM with OpenTelemetry:**

```python
# tracing.py
from opentelemetry import trace
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor
from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter
from opentelemetry.instrumentation.aiohttp_client import AioHttpClientInstrumentor
from opentelemetry.instrumentation.asyncio import AsyncioInstrumentor

def setup_tracing(service_name: str, otlp_endpoint: str):
    """Set up distributed tracing."""
    provider = TracerProvider()
    processor = BatchSpanProcessor(
        OTLPSpanExporter(endpoint=otlp_endpoint)
    )
    provider.add_span_processor(processor)
    trace.set_tracer_provider(provider)

    # Auto-instrument libraries
    AioHttpClientInstrumentor().instrument()
    AsyncioInstrumentor().instrument()

    return trace.get_tracer(service_name)

# Usage context manager
async def traced_skill_execution(skill_id: str):
    """Context manager for tracing skill execution."""
    tracer = trace.get_tracer("openspace")

    with tracer.start_as_current_span(
        "skill_execution",
        attributes={"skill_id": skill_id},
    ) as span:
        try:
            yield span
            span.set_attribute("status", "success")
        except Exception as e:
            span.set_attribute("status", "error")
            span.record_exception(e)
            raise
```

---

## 8. CI/CD

### 8.1 Testing Pipelines

**GitHub Actions CI:**

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  PYTHON_VERSION: "3.11"
  POSTGRES_VERSION: "15"

jobs:
  lint:
    name: Lint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Set up Python
        uses: actions/setup-python@v5
        with:
          python-version: ${{ env.PYTHON_VERSION }}

      - name: Install dependencies
        run: |
          python -m pip install --upgrade pip
          pip install -r requirements-dev.txt

      - name: Run ruff
        run: ruff check .

      - name: Run black
        run: black --check .

      - name: Run mypy
        run: mypy openspace/

  test:
    name: Test
    runs-on: ubuntu-latest
    services:
      postgres:
        image: pgvector/pgvector:pg15
        env:
          POSTGRES_USER: test
          POSTGRES_PASSWORD: test
          POSTGRES_DB: openspace_test
        ports:
          - 5432:5432
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

      redis:
        image: redis:7-alpine
        ports:
          - 6379:6379
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - uses: actions/checkout@v4

      - name: Set up Python
        uses: actions/setup-python@v5
        with:
          python-version: ${{ env.PYTHON_VERSION }}

      - name: Install dependencies
        run: |
          python -m pip install --upgrade pip
          pip install -r requirements.txt
          pip install -r requirements-dev.txt

      - name: Run tests
        run: |
          pytest \
            --cov=openspace \
            --cov-report=xml \
            --cov-report=html \
            -v \
            tests/
        env:
          DATABASE_URL: postgresql://test:test@localhost:5432/openspace_test
          REDIS_URL: redis://localhost:6379/0

      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          file: ./coverage.xml
          flags: unittests

  security:
    name: Security Scan
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Set up Python
        uses: actions/setup-python@v5
        with:
          python-version: ${{ env.PYTHON_VERSION }}

      - name: Install safety
        run: pip install safety

      - name: Check dependencies
        run: safety check -r requirements.txt

      - name: Run bandit
        run: bandit -r openspace/ -f json -o bandit-report.json

      - name: Upload security report
        uses: actions/upload-artifact@v3
        with:
          name: bandit-report
          path: bandit-report.json

  build:
    name: Build
    runs-on: ubuntu-latest
    needs: [lint, test, security]
    steps:
      - uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Build Docker image
        uses: docker/build-push-action@v5
        with:
          context: .
          push: false
          tags: openspace-mcp:${{ github.sha }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

  docker:
    name: Docker Push
    runs-on: ubuntu-latest
    needs: [build]
    if: github.ref == 'refs/heads/main'
    steps:
      - uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Login to Docker Hub
        uses: docker/login-action@v3
        with:
          username: ${{ secrets.DOCKER_USERNAME }}
          password: ${{ secrets.DOCKER_PASSWORD }}

      - name: Build and push
        uses: docker/build-push-action@v5
        with:
          context: .
          push: true
          tags: |
            openspace-mcp:latest
            openspace-mcp:${{ github.sha }}
          cache-from: type=gha
          cache-to: type=gha,mode=max
```

### 8.2 Package Publishing

**PyPI Publishing:**

```yaml
# .github/workflows/publish.yml
name: Publish to PyPI

on:
  release:
    types: [published]

jobs:
  publish:
    runs-on: ubuntu-latest
    environment:
      name: pypi
      url: https://pypi.org/p/openspace
    permissions:
      id-token: write

    steps:
      - uses: actions/checkout@v4

      - name: Set up Python
        uses: actions/setup-python@v5
        with:
          python-version: "3.11"

      - name: Install build tools
        run: |
          python -m pip install --upgrade pip
          pip install build twine

      - name: Build package
        run: python -m build

      - name: Publish to PyPI
        uses: pypa/gh-action-pypi-publish@release/v1
        with:
          packages-dir: dist/
```

**Setup.py:**

```python
# setup.py
from setuptools import setup, find_packages

with open("README.md", "r", encoding="utf-8") as fh:
    long_description = fh.read()

setup(
    name="openspace",
    version="0.1.0",
    author="HKUDS",
    author_email="openspace@hkuds.io",
    description="Self-evolving agent skill engine",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/HKUDS/OpenSpace",
    packages=find_packages(),
    classifiers=[
        "Development Status :: 4 - Beta",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
    ],
    python_requires=">=3.10",
    install_requires=[
        "aiohttp>=3.9.0",
        "asyncpg>=0.29.0",
        "pyyaml>=6.0",
        "redis>=5.0.0",
        "openai>=1.0.0",
        "litellm>=1.0.0",
        "mcp>=0.1.0",
        "prometheus-client>=0.19.0",
        "python-json-logger>=2.0.0",
        "sentry-sdk>=1.38.0",
    ],
    extras_require={
        "dev": [
            "pytest>=7.4.0",
            "pytest-cov>=4.1.0",
            "pytest-asyncio>=0.21.0",
            "ruff>=0.1.0",
            "black>=23.0.0",
            "mypy>=1.7.0",
        ],
    },
    entry_points={
        "console_scripts": [
            "openspace-mcp=openspace.mcp_server:main",
            "openspace=openspace.cli:main",
        ],
    },
)
```

### 8.3 Deployment Automation

**GitHub Actions CD:**

```yaml
# .github/workflows/deploy.yml
name: Deploy to Kubernetes

on:
  push:
    branches: [main]
    tags:
      - 'v*'

env:
  CLUSTER_NAME: openspace-prod
  NAMESPACE: openspace

jobs:
  deploy:
    runs-on: ubuntu-latest
    environment: production

    steps:
      - uses: actions/checkout@v4

      - name: Set up kubectl
        uses: azure/setup-kubectl@v3
        with:
          version: 'v1.28.0'

      - name: Configure AWS credentials
        uses: aws-actions/configure-aws-credentials@v4
        with:
          aws-access-key-id: ${{ secrets.AWS_ACCESS_KEY_ID }}
          aws-secret-access-key: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
          aws-region: us-east-1

      - name: Update kubeconfig
        run: aws eks update-kubeconfig --name ${{ env.CLUSTER_NAME }}

      - name: Deploy to Kubernetes
        run: |
          kubectl apply -f k8s/namespace.yaml
          kubectl apply -f k8s/configmap.yaml
          kubectl apply -f k8s/secrets.yaml
          kubectl apply -f k8s/deployment.yaml
          kubectl apply -f k8s/service.yaml
          kubectl apply -f k8s/ingress.yaml

      - name: Wait for rollout
        run: |
          kubectl rollout status deployment/openspace-mcp -n ${{ env.NAMESPACE }}

      - name: Run smoke tests
        run: |
          kubectl run smoke-test --rm -it --restart=Never \
            --image=curlimages/curl \
            -- curl -f https://openspace-mcp.example.com/health/ready

  helm-deploy:
    runs-on: ubuntu-latest
    environment: production

    steps:
      - uses: actions/checkout@v4

      - name: Set up Helm
        uses: azure/setup-helm@v3
        with:
          version: 'v3.13.0'

      - name: Configure AWS credentials
        uses: aws-actions/configure-aws-credentials@v4
        with:
          aws-access-key-id: ${{ secrets.AWS_ACCESS_KEY_ID }}
          aws-secret-access-key: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
          aws-region: us-east-1

      - name: Update kubeconfig
        run: aws eks update-kubeconfig --name ${{ env.CLUSTER_NAME }}

      - name: Deploy with Helm
        run: |
          helm upgrade --install openspace ./helm/openspace \
            --namespace ${{ env.NAMESPACE }} \
            --set image.tag=${{ github.sha }} \
            --set replicaCount=3 \
            --wait --timeout 5m
```

**Terraform CDK Deployment:**

```python
# cdktf/main.py
from constructs import Construct
from cdktf import App, TerraformStack
from cdktf_cdktf_provider_aws import s3, rds, eks

class OpenSpaceStack(TerraformStack):
    def __init__(self, scope: Construct, id: str):
        super().__init__(scope, id)

        # S3 bucket for artifacts
        s3.Bucket(
            self,
            "artifacts",
            bucket_prefix="openspace-artifacts",
            versioning=s3.BucketVersioning(
                enabled=True
            ),
        )

        # RDS PostgreSQL
        rds.DbInstance(
            self,
            "database",
            identifier="openspace-prod",
            engine="postgres",
            engine_version="15.7",
            instance_class="db.r6g.xlarge",
            allocated_storage=100,
            storage_encrypted=True,
            multi_az=True,
            backup_retention_period=7,
        )

        # EKS Cluster
        eks.Cluster(
            self,
            "cluster",
            name="openspace-prod",
            version="1.28",
        )

app = App()
OpenSpaceStack(app, "openspace-prod")
app.synth()
```

### 8.4 Rollback Strategies

**Kubernetes Rollback:**

```bash
#!/bin/bash
# rollback.sh

set -e

NAMESPACE="openspace"
DEPLOYMENT="openspace-mcp"

# Get current and previous revisions
CURRENT_REVISION=$(kubectl rollout history deployment/$DEPLOYMENT -n $NAMESPACE --show-revision | tail -1 | awk '{print $1}')
PREVIOUS_REVISION=$((CURRENT_REVISION - 1))

echo "Current revision: $CURRENT_REVISION"
echo "Rolling back to revision: $PREVIOUS_REVISION"

# Perform rollback
kubectl rollout undo deployment/$DEPLOYMENT -n $NAMESPACE --to-revision=$PREVIOUS_REVISION

# Wait for rollout
kubectl rollout status deployment/$DEPLOYMENT -n $NAMESPACE

echo "Rollback completed"
```

**Automated Rollback on Health Check Failure:**

```yaml
# Argo Rollouts configuration
apiVersion: argoproj.io/v1alpha1
kind: Rollout
metadata:
  name: openspace-mcp
  namespace: openspace
spec:
  replicas: 5
  strategy:
    canary:
      steps:
      - setWeight: 20
      - pause: {duration: 5m}
      - setWeight: 50
      - pause: {duration: 5m}
      - setWeight: 80
      - pause: {duration: 5m}
      - setWeight: 100

      analysis:
        templates:
        - templateName: success-rate
        startingStep: 1
        successfulRunHistoryLimit: 3
        unsuccessfulRunHistoryLimit: 1

---
apiVersion: argoproj.io/v1alpha1
kind: AnalysisTemplate
metadata:
  name: success-rate
spec:
  args:
  - name: service-name
  metrics:
  - name: success-rate
    interval: 1m
    successCondition: result[0] >= 0.95
    failureLimit: 1
    provider:
      prometheus:
        address: http://prometheus:9090
        query: |
          sum(rate(openspace_requests_total{status="success"}[5m])) /
          sum(rate(openspace_requests_total[5m]))
```

---

## Appendix A: Quick Reference

### Environment Variables

```bash
# Required
OPENSPACE_MODEL=openrouter/anthropic/claude-sonnet-4.5
OPENROUTER_API_KEY=sk-or-xxx
OPENSPACE_API_KEY=sk_xxx  # For cloud features

# Optional
OPENSPACE_WORKSPACE=/workspace
OPENSPACE_MAX_ITERATIONS=20
OPENSPACE_LOG_LEVEL=INFO
DATABASE_URL=postgresql://user:pass@host:5432/db
REDIS_URL=redis://host:6379/0
```

### Health Check URLs

| Endpoint | Purpose |
|----------|---------|
| `/health/live` | Liveness probe |
| `/health/ready` | Readiness probe |
| `/health/startup` | Startup probe |
| `/metrics` | Prometheus metrics |

### Default Ports

| Service | Port |
|---------|------|
| MCP Server (stdio) | N/A |
| API Server | 8080 |
| Metrics | 9090 |
| PostgreSQL | 5432 |
| Redis | 6379 |

---

## Appendix B: Troubleshooting

### Common Issues

| Issue | Solution |
|-------|----------|
| MCP server not starting | Check PYTHONPATH and virtual environment |
| Skills not loading | Verify skill directory permissions |
| Cloud upload failing | Validate API key and network access |
| High memory usage | Reduce grounding_max_iterations |
| Slow embedding search | Check pgvector index creation |

### Log Locations

| Component | Log Path |
|-----------|----------|
| MCP Server | /var/log/openspace/mcp.log |
| API Server | /var/log/openspace/api.log |
| PostgreSQL | /var/log/postgresql/ |
| Redis | /var/log/redis/ |

---

*Document Version: 1.0*
*Last Updated: 2024-01-15*
*OpenSpace Version: 0.1.0*
