# Moltbot/Clawdbot - Core Agent Architecture Deep Dive

## Overview

Moltbot (also known as Clawdbot) is the central AI agent gateway in the Moltbook ecosystem. It serves as the orchestration layer that connects AI models to communication channels, tools, and external services.

**Repository:** `github:clawdbot/moltbot` (formerly `github:moltbot/moltbot`)

**Live Sites:**
- Landing Page: `https://molt.bot`
- Documentation: `https://docs.openclaw.ai`

## Core Architecture Principles

### 1. Gateway-Centric Design

A single long-lived **Gateway** owns all messaging surfaces:
- WhatsApp via Baileys
- Telegram via grammY
- Discord, Slack, Signal, iMessage
- WebChat over WebSocket

Control-plane clients (macOS app, CLI, web UI, automations) connect to the Gateway over **WebSocket** on the configured bind host (default `127.0.0.1:18789`).

### 2. Plugin Architecture

Core stays lean; capabilities ship as plugins:
- In-process Gateway plugins (`moltbot.plugin.json` manifest)
- npm package distribution preferred
- Local extension loading for development
- High bar for adding plugins to core

### 3. Trait-Based Design

Swappable providers, channels, and tools with config changes:
- **Channels**: WhatsApp, Telegram, Discord, Slack, Signal, iMessage, Mattermost, LINE, Matrix, Nostr, Twitch, Zalo, etc.
- **AI Providers**: OpenAI, Anthropic, OpenRouter, Vercel AI Gateway, Moonshot, Z.AI, GLM, MiniMax, Venice AI, Amazon Bedrock, Ollama (local)
- **Memory Backends**: SQLite (hybrid vector + keyword), PostgreSQL, Lucid bridge, Markdown files, No-op (stateless)

### 4. Declarative Configuration

Infrastructure as code, reproducible deployments:
- JSON5 configuration files
- Agent workspace structure (`~/clawd` default)
- Config versioning with `baseHash` for optimistic concurrency

### 5. Security by Default

Gateway pairing, sandboxing, explicit allowlists:
- Device-based pairing approval
- Local trust auto-approval for same-host
- Non-local connections require signed challenge
- Agent sandboxing with filesystem restrictions

## Technology Stack

| Layer | Technology |
|-------|------------|
| **Runtime** | Node.js 22.12+ (Bun not recommended for Gateway) |
| **Language** | TypeScript 5.9+ |
| **Package Manager** | pnpm 10.23+ |
| **Build** | TypeScript compiler + Rolldown |
| **Linting** | Oxlint (type-aware) + Oxlint-tsgolint |
| **Formatting** | Oxfmt + SwiftFormat (macOS/iOS apps) |
| **Testing** | Vitest 4+ (unit, E2E, live tests) |
| **Coverage** | V8 coverage (70% threshold) |
| **WebSocket** | `ws` library with JSON Schema validation |
| **Database** | SQLite with sqlite-vec extension |
| **Vector Search** | sqlite-vec + remote embeddings |

## Repository Structure

```
moltbot/
├── src/                          # Main source code
│   ├── gateway/                  # WebSocket Gateway server
│   ├── channels/                 # Channel implementations
│   │   ├── whatsapp/             # Baileys-based WhatsApp
│   │   ├── telegram/             # grammY-based Telegram
│   │   ├── discord/              # Discord API
│   │   ├── slack/                # Slack Bolt SDK
│   │   ├── signal/               # signal-cli integration
│   │   ├── imessage/             # macOS native iMessage
│   │   └── ...                   # Other channels
│   ├── providers/                # AI model providers
│   │   ├── openai/               # OpenAI/GPT
│   │   ├── anthropic/            # Anthropic/Claude
│   │   ├── openrouter/           # OpenRouter multi-model
│   │   ├── ollama/               # Local Ollama
│   │   └── ...                   # Other providers
│   ├── plugins/                  # Plugin system
│   │   └── plugin-sdk/           # Plugin SDK
│   ├── memory/                   # Memory system
│   │   ├── memory-core/          # Core memory plugin
│   │   └── memory-lancedb/       # LanceDB backend
│   ├── agents/                   # Agent loop implementation
│   ├── cli/                      # CLI commands
│   ├── tui/                      # Terminal UI
│   ├── acp/                      # ACP bridge for IDEs
│   ├── browser/                  # Browser automation
│   ├── canvas-host/              # A2UI canvas hosting
│   ├── cron/                     # Scheduled jobs
│   ├── hooks/                    # Runtime hooks
│   ├── security/                 # Security features
│   └── protocol/                 # WebSocket protocol
├── extensions/                   # Built-in extensions
│   ├── bluebubbles/              # BlueBubbles iMessage
│   ├── discord/                  # Discord extension
│   ├── lobster/                  # Lobster workflow integration
│   ├── voice-call/               # Voice call support
│   └── ...                       # Other extensions
├── apps/                         # Native applications
│   ├── macos/                    # macOS app (Swift)
│   ├── ios/                      # iOS app (Swift)
│   └── android/                  # Android app (Kotlin)
├── docs/                         # Documentation
│   ├── concepts/                 # Core concepts
│   ├── channels/                 # Channel guides
│   ├── providers/                # Provider guides
│   ├── cli/                      # CLI reference
│   ├── gateway/                  # Gateway docs
│   ├── plugins/                  # Plugin docs
│   ├── memory/                   # Memory docs
│   └── automation/               # Automation guides
├── assets/                       # Static assets
├── skills/                       # Bundled skills
├── scripts/                      # Build and utility scripts
└── packages/
    └── clawdbot/                 # Core SDK package
```

## Gateway Architecture

### WebSocket Protocol

The Gateway exposes a typed WS API with requests, responses, and server-push events:

```typescript
// Connection handshake (first frame MUST be connect)
{
  type: "req",
  id: "uuid",
  method: "connect",
  params: {
    deviceIdentity: {...},
    role: "node" | "client",
    caps: [...],
    commands: [...],
    permissions: [...]
  }
}

// Request format
{
  type: "req",
  id: "uuid",
  method: "agent" | "send" | "health" | ...,
  params: {...}
}

// Response format
{
  type: "res",
  id: "uuid",
  ok: boolean,
  payload?: {...},
  error?: {message: string, code?: string}
}

// Server events
{
  type: "event",
  event: "agent" | "chat" | "presence" | "health" | "heartbeat" | "cron",
  payload: {...},
  seq?: number,
  stateVersion?: number
}
```

### Connection Lifecycle

```
Client                    Gateway
  |                          |
  |---- req:connect -------->|
  |<------ res (ok) ---------|   (payload=hello-ok carries snapshot)
  |                          |
  |<------ event:presence ---|
  |<------ event:tick -------|
  |                          |
  |------- req:agent ------->|
  |<------ res:agent --------|   (ack: {runId,status:"accepted"})
  |<------ event:agent ------|   (streaming)
  |<------ res:agent --------|   (final: {runId,status,summary})
```

### Canvas Host

A separate HTTP server (default port `18793`) hosts agent-editable HTML and A2UI interfaces:
- Static UI served for WebChat
- Agent-modifiable canvas content
- A2UI specification compliance (0.8/0.9)

## Channel System

### Supported Channels

| Channel | Implementation | Notes |
|---------|----------------|-------|
| WhatsApp | Baileys (v7.0.0-rc.9) | QR pairing, most popular |
| Telegram | grammY | Bot API, fastest setup |
| Discord | Discord API | Servers, channels, DMs |
| Slack | Bolt SDK | Workspace apps |
| Google Chat | HTTP webhook | App via webhook |
| Mattermost | Bot API + WebSocket | Plugin, separate install |
| Signal | signal-cli | Privacy-focused |
| BlueBubbles | REST API | **Recommended for iMessage** |
| iMessage | imsg (native) | macOS only, legacy |
| Microsoft Teams | Bot Framework | Plugin, enterprise |
| LINE | LINE Messaging API | Plugin |
| Nextcloud Talk | Talk API | Plugin, self-hosted |
| Matrix | Matrix protocol | Plugin |
| Nostr | NIP-04 | Plugin, decentralized |
| Tlon | Urbit messenger | Plugin |
| Twitch | IRC connection | Plugin |
| Zalo | Zalo Bot API | Plugin, Vietnam |
| Zalo Personal | QR login | Plugin |
| WebChat | WebSocket UI | Built-in |

### Channel Routing

Channels can run simultaneously; Moltbot routes per chat:
- Group behavior varies by channel
- DM pairing and allowlists enforced
- Broadcast groups supported

### WhatsApp (Baileys) Details

```typescript
// WhatsApp channel setup
import makeWASocket from '@whiskeysockets/baileys';

const socket = makeWASocket({
  auth: creds,
  printQRInTerminal: true,
  browser: ['Moltbot', 'Safari', '15.0'],
});

// Message handling
socket.ev.on('messages.upsert', async ({ messages }) => {
  const msg = messages[0];
  if (msg.key.fromMe) return;

  await gateway.handleMessage({
    channel: 'whatsapp',
    from: msg.key.remoteJid,
    text: msg.message?.conversation || '',
  });
});
```

### Telegram (grammY) Details

```typescript
// Telegram channel setup
import { Bot } from 'grammy';
import { transformThrottler } from '@grammyjs/transformer-throttler';

const bot = new Bot(process.env.TELEGRAM_BOT_TOKEN!);

// Rate limiting
bot.api.config.use(transformThrottler());

// Message handling
bot.on('message:text', async (ctx) => {
  await gateway.handleMessage({
    channel: 'telegram',
    from: ctx.chat.id.toString(),
    text: ctx.message.text,
  });
});
```

## AI Provider System

### Supported Providers

| Provider | Models | Auth Method |
|----------|--------|-------------|
| OpenAI | GPT-4, GPT-5, Codex | API key / OAuth |
| Anthropic | Claude 3/4, Opus | API key / setup-token |
| OpenRouter | Multi-model | API key |
| Vercel AI Gateway | Multi-provider | API key |
| Moonshot | Kimi, Kimi Code | API key |
| Z.AI | Z models | API key |
| GLM | GLM models | API key |
| MiniMax | MiniMax models | API key |
| Venice AI | Llama 3.3 70B, Claude Opus | API key |
| Amazon Bedrock | Bedrock models | AWS credentials |
| Ollama | Local models | Local HTTP |

### Provider Configuration

```json5
{
  agents: {
    defaults: {
      model: {
        primary: "anthropic/claude-opus-4-5",
        fallbacks: ["openai/gpt-4o", "openrouter/anthropic/claude-3-5-sonnet"]
      }
    }
  },
  models: {
    providers: {
      anthropic: {
        apiKey: "${ANTHROPIC_API_KEY}",
      },
      openai: {
        apiKey: "${OPENAI_API_KEY}",
      }
    }
  }
}
```

### Recommended: Venice AI (Venius)

Venice AI is the recommended privacy-first setup:

```json5
{
  agents: {
    defaults: {
      model: {
        primary: "venice/claude-opus-45",  // Best overall
        // or: "venice/llama-3.3-70b"      // Default, privacy-first
      }
    }
  }
}
```

## Memory System

### Architecture

Moltbot memory is **plain Markdown in the agent workspace**:

```
~/clawd/                    # Agent workspace
├── MEMORY.md               # Curated long-term memory
└── memory/
    ├── 2026-01-06.md       # Daily log (append-only)
    ├── 2026-01-07.md
    └── ...
```

### Memory Layers

| Layer | File | Purpose |
|-------|------|---------|
| Daily | `memory/YYYY-MM-DD.md` | Day-to-day notes, running context |
| Long-term | `MEMORY.md` | Decisions, preferences, durable facts |

### Automatic Memory Flush

Pre-compaction ping triggers silent agentic turn:

```json5
{
  agents: {
    defaults: {
      compaction: {
        reserveTokensFloor: 20000,
        memoryFlush: {
          enabled: true,
          softThresholdTokens: 4000,
          systemPrompt: "Session nearing compaction. Store durable memories now.",
          prompt: "Write any lasting notes to memory/YYYY-MM-DD.md; reply with NO_REPLY."
        }
      }
    }
  }
}
```

### Vector Search (Hybrid BM25 + Vector)

Moltbot combines vector similarity with BM25 keyword search:

```json5
{
  agents: {
    defaults: {
      memorySearch: {
        provider: "openai",  // or "gemini", "local"
        model: "text-embedding-3-small",
        fallback: "openai",
        query: {
          hybrid: {
            enabled: true,
            vectorWeight: 0.7,
            textWeight: 0.3,
            candidateMultiplier: 4
          }
        },
        cache: {
          enabled: true,
          maxEntries: 50000
        },
        sync: {
          watch: true,
          sessions: {
            deltaBytes: 100000,
            deltaMessages: 50
          }
        }
      }
    }
  }
}
```

### SQLite Vector Acceleration

When `sqlite-vec` extension is available:

```json5
{
  agents: {
    defaults: {
      memorySearch: {
        store: {
          vector: {
            enabled: true,
            extensionPath: "/path/to/sqlite-vec"
          }
        }
      }
    }
  }
}
```

### Local Embeddings

Auto-downloads ~0.6GB embedding model:

```json5
{
  agents: {
    defaults: {
      memorySearch: {
        provider: "local",
        local: {
          modelPath: "hf:ggml-org/embeddinggemma-300M-GGUF/embeddinggemma-300M-Q8_0.gguf",
          modelCacheDir: "~/.clawdbot/embedding-cache"
        },
        fallback: "openai"  // or "none" to disable remote
      }
    }
  }
}
```

## Plugin System

### Plugin Manifest

Every plugin must ship `moltbot.plugin.json`:

```json
{
  "id": "my-plugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "description": "A useful plugin",
  "main": "dist/index.js",
  "configSchema": {
    "type": "object",
    "properties": {
      "apiKey": {
        "type": "string",
        "title": "API Key",
        "description": "Your API key"
      }
    },
    "required": ["apiKey"]
  },
  "tools": [
    {
      "name": "my_tool",
      "description": "Does something useful",
      "inputSchema": {...}
    }
  ]
}
```

### Plugin SDK

```typescript
// src/plugin-sdk/index.ts
import type { PluginContext, ToolDefinition } from '@moltbot/plugin-sdk';

export function definePlugin(ctx: PluginContext) {
  return {
    tools: [
      {
        name: 'my_tool',
        description: 'Performs an action',
        inputSchema: {
          type: 'object',
          properties: {
            param: { type: 'string' }
          },
          required: ['param']
        },
        execute: async (params) => {
          // Tool implementation
          return { result: 'success' };
        }
      }
    ],
    hooks: {
      'message:received': async (msg) => {
        // Message hook
      }
    }
  };
}
```

### Installing Plugins

```bash
# Install from npm
moltbot plugins install my-plugin

# Install from local path (linked)
moltbot plugins install -l ./my-plugin

# Enable plugin
moltbot plugins enable my-plugin

# Check for issues
moltbot plugins doctor
```

## CLI System

### Command Tree

```
moltbot
├── setup                    # Initialize config + workspace
├── onboard                  # Interactive wizard
├── configure                # Configuration wizard
├── config                   # Get/set/unset config
├── doctor                   # Health checks + fixes
├── security                 # Audit + fix security
├── reset                    # Reset local state
├── uninstall                # Uninstall gateway
├── update                   # Update installation
├── channels                 # Manage channels
│   ├── list/status/logs
│   ├── add/remove
│   └── login/logout
├── skills                   # List/inspect skills
├── plugins                  # Manage plugins
├── memory                   # Vector search
├── message                  # Unified messaging
├── agent                    # Run one agent turn
├── agents                   # Manage agents
├── acp                      # ACP bridge for IDEs
├── status                   # Session health
├── health                   # Gateway health
├── sessions                 # List sessions
├── gateway                  # WebSocket gateway
├── logs                     # Tail logs
├── system                   # System events
├── models                   # Model management
├── sandbox                  # Sandbox management
├── cron                     # Scheduled jobs
├── nodes                    # Node management
├── browser                  # Browser control
├── hooks                    # Runtime hooks
├── webhooks                 # Webhook management
├── pairing                  # Pairing approval
├── docs                     # Docs search
├── dns                      # Discovery DNS
├── tui                      # Terminal UI
└── voicecall                # Voice calls (plugin)
```

### Global Flags

```bash
moltbot --dev <command>           # Dev mode isolation
moltbot --profile <name> <cmd>    # Profile isolation
moltbot --no-color <command>      # Disable colors
moltbot --update                  # Shorthand for update
```

### Output Styling

Lobster palette for CLI output:
- `accent` (#FF5A2D): headings, labels, primary highlights
- `accentBright` (#FF7A3D): command names, emphasis
- `success` (#2FBF71): success states
- `error` (#E23D2D): errors, failures
- `warn` (#FFB020): warnings
- `muted` (#8B7F77): de-emphasis

## Security Features

### Pairing System

Device-based pairing approval:

```typescript
// Device connect handshake
{
  type: "req",
  method: "connect",
  params: {
    deviceIdentity: {
      id: "device-uuid",
      publicKey: "ed25519-public-key",
      displayName: "MacBook Pro"
    },
    role: "node" | "client",
    caps: ["canvas.*", "camera.*"],
    commands: ["canvas.snapshot", "camera.snap"]
  }
}

// Gateway issues device token after approval
{
  type: "res",
  ok: true,
  payload: {
    deviceToken: "signed-jwt-token"
  }
}
```

### Local Trust

- Loopback connects auto-approved
- Tailnet same-host auto-approved
- Non-local requires signed challenge

### Agent Sandboxing

```json5
{
  agents: {
    entries: {
      "default": {
        sandbox: {
          enabled: true,
          workspaceAccess: "rw",  // or "ro" / "none"
          allowedPaths: ["~/clawd"],
          blockedCommands: ["rm -rf /", "mkfs"],
          networkAccess: "outbound-only"
        }
      }
    }
  }
}
```

### Security Audit

```bash
# Quick audit
moltbot security audit

# Deep audit with live probe
moltbot security audit --deep

# Apply safe fixes
moltbot security audit --fix
```

Audit checks:
- DM scope configuration
- Small model sandboxing
- Web/browser tool restrictions
- Config/state file permissions

## Native Applications

### macOS App

Swift-native application with:
- Menu bar presence
- Native notifications
- System integration (Shortcuts, Automator)
- TUI embedded view
- Quick access to logs/status

### iOS App

Swift iOS application:
- Remote gateway connection
- Push notifications
- Message interface
- Status monitoring

### Android App

Kotlin Android application:
- Similar feature set to iOS
- Material Design UI

## ACP Bridge

Agent Communication Protocol bridge for IDE integration:

```bash
# Run ACP bridge
moltbot acp

# Connect IDE to ACP
# VS Code, JetBrains, etc. connect via stdio/WebSocket
```

ACP enables:
- IDE-native agent interaction
- Code-aware completions
- Inline edits
- Terminal integration

## Browser Automation

Dedicated Chrome/Brave/Edge/Chromium control:

```bash
# Start browser
moltbot browser start

# Navigate
moltbot browser open https://example.com

# Take screenshot
moltbot browser screenshot

# Full automation
moltbot browser click "submit-btn"
moltbot browser type "email" "test@example.com"
moltbot browser navigate "https://example.com/dashboard"
```

## Cron System

Scheduled jobs with Gateway RPC:

```bash
# List cron jobs
moltbot cron list

# Add scheduled job
moltbot cron add \
  --name "daily-summary" \
  --cron "0 8 * * *" \
  --system-event "Generate daily summary"

# Run job manually
moltbot cron run daily-summary
```

Cron configuration:

```json5
{
  cron: {
    entries: {
      "daily-summary": {
        name: "Daily Summary",
        cron: "0 8 * * *",
        enabled: true,
        systemEvent: {
          text: "Generate daily summary"
        }
      }
    }
  }
}
```

## Session Management

### Compaction Lifecycle

Sessions auto-compact when approaching context limits:

```json5
{
  agents: {
    defaults: {
      compaction: {
        reserveTokensFloor: 20000,
        memoryFlush: {
          enabled: true,
          softThresholdTokens: 4000
        }
      }
    }
  }
}
```

### Session Storage

```
~/.clawdbot/agents/<agentId>/sessions/
├── sessions.json           # Session metadata
└── <session-id>.jsonl      # Conversation transcript
```

## Node Host System

Headless nodes for device capabilities:

```bash
# Run headless node
moltbot node run --host <gateway-host> --port 18789

# Install as service
moltbot node install --host <gateway-host>

# Invoke commands
moltbot nodes invoke \
  --node <id> \
  --command canvas.snapshot
```

### Node Commands

| Command | Description |
|---------|-------------|
| `canvas.*` | Screen capture, A2UI |
| `camera.*` | Front/back camera snapshots |
| `screen.record` | Screen recording |
| `location.get` | GPS location |

## Testing Infrastructure

### Test Commands

```bash
# Run all tests
pnpm test

# Watch mode
pnpm test:watch

# Coverage (70% threshold)
pnpm test:coverage

# E2E tests
pnpm test:e2e

# Live model tests
pnpm test:live

# Docker E2E suite
pnpm test:docker:all
```

### Docker E2E Tests

```bash
# Full E2E suite
./scripts/e2e/onboard-docker.sh        # Onboarding test
./scripts/e2e/gateway-network-docker.sh # Gateway networking
./scripts/e2e/qr-import-docker.sh      # WhatsApp QR
./scripts/e2e/doctor-install-switch-docker.sh
./scripts/e2e/plugins-docker.sh        # Plugin system
```

## Performance Optimizations

### Memory Indexing

- Debounced file watching (1.5s)
- Async sync on session start/search
- Delta thresholds for session transcripts
- Embedding cache (50k entries)

### Batch Embeddings

```json5
{
  agents: {
    defaults: {
      memorySearch: {
        remote: {
          batch: {
            enabled: true,
            concurrency: 2,
            wait: true,
            pollIntervalMs: 1000,
            timeoutMinutes: 30
          }
        }
      }
    }
  }
}
```

### SQLite Optimization

- `sqlite-vec` for vector queries
- FTS5 for BM25 keyword search
- Hybrid search candidate pooling

## Related Projects

- **MoltHub** - Skill registry (SKILL.md/SOUL.md)
- **Lobster** - Workflow automation with approval gates
- **Moltinators** - NixOS on AWS (CLAWDINATOR instances)
- **OpenClaw** - Upstream agent framework
- **Zeroclaw** - Rust lightweight agent (<5MB RAM)
- **Nanobot** - Python lightweight agent (~3400 lines)
- **ThePopebot** - GitHub Actions-based agents
- **Barnacle** - Discord bot with Carbon framework

---

*Moltbot deep dive - Part of Moltbook ecosystem exploration*
