# free-code Production Guide

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code`

A comprehensive guide for deploying and operating free-code in production environments.

---

## Table of Contents

1. [System Requirements](#system-requirements)
2. [Installation](#installation)
3. [Configuration](#configuration)
4. [Model Provider Setup](#model-provider-setup)
5. [Security Considerations](#security-considerations)
6. [Permission Modes](#permission-modes)
7. [Team Deployment](#team-deployment)
8. [Monitoring & Observability](#monitoring--observability)
9. [Backup & Recovery](#backup--recovery)
10. [Troubleshooting](#troubleshooting)
11. [Performance Tuning](#performance-tuning)

---

## System Requirements

### Minimum Requirements

| Component | Requirement |
|-----------|-------------|
| CPU | 2 cores (4 recommended) |
| RAM | 512MB (2GB recommended) |
| Storage | 500MB for binary + cache |
| OS | macOS 12+, Linux (glibc/musl) |
| Runtime | Bun >= 1.3.11 |

### Recommended for Multi-User

| Component | Recommendation |
|-----------|----------------|
| CPU | 8+ cores |
| RAM | 8GB+ |
| Storage | SSD with 10GB+ free |
| Network | Low-latency to API provider |

### Container Requirements

```dockerfile
FROM oven/bun:1.3.11

WORKDIR /app
COPY . .

RUN bun install --frozen-lockfile
RUN bun run build

ENV NODE_OPTIONS="--max-old-space-size=4096"

ENTRYPOINT ["./cli"]
```

---

## Installation

### Option 1: One-Line Install (Recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/paoloanzn/free-code/main/install.sh | bash
```

The install script:
- Checks system requirements
- Installs Bun if needed
- Clones the repository
- Builds with all experimental features
- Symlinks `free-code` to PATH

### Option 2: Manual Install

```bash
# Clone repository
git clone https://github.com/paoloanzn/free-code.git
cd free-code

# Install dependencies
bun install --frozen-lockfile

# Build production binary
bun run build

# Verify installation
./cli --version
```

### Option 3: From Source (Development)

```bash
# Install dependencies
bun install

# Run directly (slower startup)
bun run dev

# Or build dev binary
bun run build:dev
```

### Option 4: Custom Feature Set

```bash
# Build with specific features only
bun run ./scripts/build.ts \
  --feature=ULTRAPLAN \
  --feature=ULTRATHINK \
  --feature=BRIDGE_MODE

# Build dev version with additional flag
bun run ./scripts/build.ts --dev --feature=AGENT_TRIGGERS
```

### Verification

```bash
# Check version
free-code --version

# Check help
free-code /help

# Test model connection
free-code /model
```

---

## Configuration

### Configuration Hierarchy

```
1. Command-line flags (highest priority)
2. Environment variables
3. Project config (.claude/settings.json)
4. Global config (~/.claude/global.json)
5. User config (~/.claude/config.json)
6. Defaults (lowest priority)
```

### Global Configuration

Location: `~/.claude/config.json`

```json
{
  "theme": "dark",
  "vimMode": false,
  "model": "claude-sonnet-4-6",
  "permissionMode": "workspace-write",
  "autoApproveTools": ["Read", "Glob"],
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "~/projects"]
    }
  },
  "hooks": {
    "beforeToolUse": {
      "Bash": "~/.claude/hooks/before-bash.sh"
    },
    "afterToolUse": {
      "FileEdit": "~/.claude/hooks/after-edit.sh"
    }
  }
}
```

### Project Configuration

Location: `.claude/settings.json`

```json
{
  "model": "claude-opus-4-6",
  "permissionMode": "read-only",
  "allowedTools": ["Read", "Glob", "Grep"],
  "deniedTools": ["Bash", "FileWrite"],
  "contextFiles": ["README.md", "package.json"],
  "memoryFiles": [".claude/TEAM.md"]
}
```

### Environment Variables

```bash
# Provider selection
export CLAUDE_CODE_USE_OPENAI=1      # Use OpenAI Codex
export CLAUDE_CODE_USE_BEDROCK=1     # Use AWS Bedrock
export CLAUDE_CODE_USE_VERTEX=1      # Use Google Vertex
export CLAUDE_CODE_USE_FOUNDRY=1     # Use Anthropic Foundry

# API keys
export ANTHROPIC_API_KEY="sk-ant-..."
export AWS_ACCESS_KEY_ID="..."
export AWS_SECRET_ACCESS_KEY="..."
export AWS_REGION="us-east-1"
export GOOGLE_APPLICATION_CREDENTIALS="/path/to/creds.json"
export ANTHROPIC_FOUNDRY_API_KEY="..."

# Model overrides
export ANTHROPIC_MODEL="claude-opus-4-6"
export ANTHROPIC_BASE_URL="https://api.anthropic.com"

# Feature toggles (runtime)
export CLAUDE_CODE_SIMPLE=1          # Simple mode (Bash/Read/Edit only)
export CLAUDE_CODE_DISABLE_AUTO_COMPACT=1  # Disable auto-compaction
export CLAUDE_CODE_DISABLE_BACKGROUND_TASKS=1  # Disable background tasks
```

---

## Model Provider Setup

### Anthropic (Direct API) — Default

**Setup:**
```bash
export ANTHROPIC_API_KEY="sk-ant-..."
free-code
```

**Available Models:**
| Model | ID | Best For |
|-------|-----|----------|
| Claude Opus 4.6 | `claude-opus-4-6` | Complex reasoning |
| Claude Sonnet 4.6 | `claude-sonnet-4-6` | Balanced performance |
| Claude Haiku 4.5 | `claude-haiku-4-5` | Fast, cheap tasks |

**Change Model:**
```bash
free-code /model claude-opus-4-6
```

### OpenAI Codex

**Setup:**
```bash
export CLAUDE_CODE_USE_OPENAI=1
free-code /login  # OAuth flow
```

**Available Models:**
| Model | ID |
|-------|-----|
| GPT-5.3 Codex | `gpt-5.3-codex` |
| GPT-5.4 | `gpt-5.4` |
| GPT-5.4 Mini | `gpt-5.4-mini` |

**Features:**
- Native thinking animation support
- Token billing display
- Vision input translation

### AWS Bedrock

**Setup:**
```bash
export CLAUDE_CODE_USE_BEDROCK=1
export AWS_REGION="us-east-1"
# Credentials via ~/.aws/credentials or IAM role
free-code
```

**Model Mapping:**
Models are automatically mapped to Bedrock ARN format:
- `claude-opus-4-6` → `us.anthropic.claude-opus-4-6-v1`

**Custom Endpoint:**
```bash
export ANTHROPIC_BEDROCK_BASE_URL="https://bedrock.us-east-1.amazonaws.com"
```

### Google Vertex AI

**Setup:**
```bash
export CLAUDE_CODE_USE_VERTEX=1
gcloud auth application-default login
free-code
```

**Model Mapping:**
- `claude-opus-4-6` → `claude-opus-4-6@latest`

### Anthropic Foundry

**Setup:**
```bash
export CLAUDE_CODE_USE_FOUNDRY=1
export ANTHROPIC_FOUNDRY_API_KEY="..."
free-code
```

**Custom Deployment:**
```bash
export ANTHROPIC_FOUNDRY_BASE_URL="https://foundry.anthropic.com"
# Model name = deployment ID
free-code --model "my-deployment-id"
```

---

## Security Considerations

### Permission Modes

**Read-Only Mode:**
```bash
free-code --permission-mode read-only
# Or in config
echo '{"permissionMode": "read-only"}' >> ~/.claude/config.json
```

**Workspace Write Mode:**
```bash
free-code --permission-mode workspace-write
```

**Full Access Mode:**
```bash
free-code --permission-mode danger-full-access
```

### Tool Allow/Deny Lists

```json
{
  "allowedTools": ["Read", "Glob", "Grep", "WebSearch"],
  "deniedTools": ["Bash", "FileWrite", "FileEdit", "Agent"]
}
```

### Auto-Approval

```json
{
  "autoApproveTools": ["Read", "Glob", "Grep"]
}
```

### Network Security

**Custom API Endpoint:**
```bash
export ANTHROPIC_BASE_URL="https://api.internal.company.com"
```

**Proxy Support:**
```bash
export HTTPS_PROXY="http://proxy.company.com:8080"
export NO_PROXY="localhost,127.0.0.1,.internal"
```

**mTLS:**
```bash
export NODE_EXTRA_CA_CERTS="/path/to/ca.pem"
```

### Credential Management

**Never commit credentials:**
```bash
# Use environment variables
export ANTHROPIC_API_KEY="..."

# Or use credential manager
aws secretsmanager get-secret-value --secret-id anthropic-key
```

**Rotate credentials regularly:**
```bash
# Script to rotate API keys
#!/bin/bash
aws secretsmanager rotate-secret --secret-id anthropic-key
export ANTHROPIC_API_KEY=$(aws secretsmanager get-secret-value ...)
```

---

## Team Deployment

### Team Memory Setup

Create `.claude/TEAM.md` in your repository:

```markdown
# Team Memory: Project X

## Architecture
- Frontend: React/TypeScript
- Backend: Node.js/Express
- Database: PostgreSQL

## Conventions
- Use TypeScript strict mode
- ESLint + Prettier for formatting
- Jest for unit tests

## Key Files
- `src/index.ts` - Entry point
- `src/api/` - API routes
- `src/db/` - Database schemas
```

### Shared Configuration

**Global config template:**
```json
{
  "mcpServers": {
    "shared-db": {
      "command": "npx",
      "args": ["-y", "@company/mcp-db-server", "--host", "db.internal"]
    }
  },
  "hooks": {
    "beforeToolUse": {
      "Bash": "/shared/hooks/security-check.sh"
    }
  }
}
```

### Centralized Management

**Deploy via config management:**
```yaml
# Ansible example
- name: Deploy free-code config
  hosts: developers
  tasks:
    - name: Install free-code
      shell: curl -fsSL https://raw.githubusercontent.com/.../install.sh | bash

    - name: Deploy config
      template:
        src: claude-config.json.j2
        dest: ~/.claude/config.json
```

---

## Monitoring & Observability

### Session Logs

Location: `~/.claude/sessions/<session-id>/`

```bash
# List sessions
ls -la ~/.claude/sessions/

# View session log
cat ~/.claude/sessions/abc123/log.jsonl
```

### Cost Tracking

```bash
# View session cost
free-code /cost

# View usage
free-code /usage
```

### Debug Logging

```bash
# Enable debug output
export DEBUG=1
free-code

# Or use debug command
free-code /doctor
```

### Performance Profiling

```bash
# Heap dump (if enabled)
free-code /heapdump

# Stats
free-code /stats
```

---

## Backup & Recovery

### Backup Session Data

```bash
# Backup all sessions
tar -czf claude-backup-$(date +%Y%m%d).tar.gz \
  ~/.claude/sessions/ \
  ~/.claude/memory/ \
  ~/.claude/config.json
```

### Restore Sessions

```bash
# Extract backup
tar -xzf claude-backup-20260402.tar.gz -C ~/

# Restore specific session
cp ~/.claude/sessions/backup/* ~/.claude/sessions/
```

### Session Export

```bash
# Export session
free-code /export my-session

# Import session
free-code /resume my-session.json
```

---

## Troubleshooting

### Common Issues

**1. API Connection Failed**

```bash
# Check API key
echo $ANTHROPIC_API_KEY

# Test connection
curl -H "X-API-Key: $ANTHROPIC_API_KEY" \
  https://api.anthropic.com/v1/models

# Check proxy
curl -x $HTTPS_PROXY https://api.anthropic.com
```

**2. Permission Denied**

```bash
# Check permission mode
free-code /config permissionMode

# Reset permissions
free-code /permissions reset
```

**3. Session Corruption**

```bash
# Clear session cache
rm -rf ~/.claude/sessions/*

# Or specific session
rm -rf ~/.claude/sessions/<corrupt-session-id>/
```

**4. Model Not Found**

```bash
# List available models
free-code /model

# Check model config
cat ~/.claude/config.json | jq .model
```

**5. MCP Server Errors**

```bash
# List MCP servers
free-code /mcp list

# Restart MCP servers
free-code /mcp restart

# Debug MCP
free-code /mcp debug <server-name>
```

### Diagnostic Commands

```bash
# Full diagnostic
free-code /doctor

# Check environment
free-code /env

# View release notes
free-code /release-notes
```

---

## Performance Tuning

### Startup Optimization

**Use compiled binary:**
```bash
# Slow (source mode)
bun run dev

# Fast (compiled)
./cli
```

**Disable unnecessary features:**
```bash
export CLAUDE_CODE_DISABLE_BACKGROUND_TASKS=1
export CLAUDE_CODE_DISABLE_AUTO_MEMORY=1
```

### Memory Management

**Increase heap size:**
```bash
export NODE_OPTIONS="--max-old-space-size=4096"
```

**Clear caches:**
```bash
# Clear file read cache
rm -rf ~/.claude/cache/file-reads/

# Clear session cache
rm -rf ~/.claude/cache/sessions/
```

### Context Optimization

**Enable auto-compaction:**
```json
{
  "autoCompact": true,
  "autoCompactThreshold": 0.8
}
```

**Use context efficiently:**
```bash
# Use @mentions instead of loading all files
@src/index.ts What does this do?

# Use /compact to shrink context
/compact
```

### Network Optimization

**Enable HTTP keepalive:**
```bash
# Built into SDK, but ensure stable connection
```

**Use regional endpoints:**
```bash
# EU users
export ANTHROPIC_BASE_URL="https://api.eu.anthropic.com"
```

**Reduce retry overhead:**
```json
{
  "apiRetryAttempts": 3,
  "apiRetryDelayMs": 1000
}
```

---

## Appendix: Build Reference

### Build Commands

| Command | Output | Use Case |
|---------|--------|----------|
| `bun run build` | `./cli` | Production deployment |
| `bun run build:dev` | `./cli-dev` | Development with version stamp |
| `bun run build:dev:full` | `./cli-dev` | All experimental features |
| `bun run compile` | `./dist/cli` | Alternative output path |

### Feature Flags

**Stable features:**
- `VOICE_MODE` — Voice input/dictation

**Experimental features:**
- `ULTRAPLAN` — Multi-agent planning
- `ULTRATHINK` — Deep thinking mode
- `TOKEN_BUDGET` — Token tracking
- `BRIDGE_MODE` — Remote control / IDE bridge
- `AGENT_TRIGGERS` — Cron-style automation
- `HISTORY_PICKER` — Interactive history

**See [FEATURES.md](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code/FEATURES.md) for complete list.**

### Custom Build Script

```typescript
// scripts/build.ts excerpt

const fullExperimentalFeatures = [
  'AGENT_MEMORY_SNAPSHOT',
  'AGENT_TRIGGERS',
  'BRIDGE_MODE',
  'TOKEN_BUDGET',
  'ULTRAPLAN',
  'ULTRATHINK',
  'VOICE_MODE',
  // ... 47 more
] as const

// Build with custom features
bun build ./src/entrypoints/cli.tsx \
  --compile \
  --minify \
  --feature=VOICE_MODE \
  --feature=ULTRAPLAN
```

---

## Quick Reference

### Essential Commands

```bash
# Start session
free-code

# Specify model
free-code --model claude-opus-4-6

# Simple mode (limited tools)
free-code --bare

# One-shot query
free-code -p "What files are in this directory?"

# OAuth login
free-code /login

# Change model
/model claude-sonnet-4-6

# View cost
/cost

# Compact context
/compact

# Plan mode
/plan

# Exit
/exit
```

### Essential Keybindings

| Key | Action |
|-----|--------|
| `Ctrl+C` | Cancel current operation |
| `Ctrl+D` | Exit |
| `Ctrl+L` | Clear screen |
| `Ctrl+R` | Search history |
| `Esc` | Close dialog |
| `:` | Enter command mode |

---

## Resources

- [00-zero-to-free-code-engineer.md](./00-zero-to-free-code-engineer.md) — Getting started
- [01-free-code-exploration.md](./01-free-code-exploration.md) — Architecture deep-dive
- [FEATURES.md](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code/FEATURES.md) — Feature flag audit
- [README.md](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code/README.md) — Project documentation
