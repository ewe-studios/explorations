# src.openclaw Architecture Deep Dive

## Overview

This document provides a comprehensive deep dive into the architecture of **src.openclaw**, the main source repository for OpenClaw - a multi-channel AI gateway platform.

**Source Path:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.Moltbot/src.openclaw/`

---

## Table of Contents

1. [System Architecture](#system-architecture)
2. [Gateway Core Architecture](#gateway-core-architecture)
3. [Channel Architecture](#channel-architecture)
4. [Agent Architecture](#agent-architecture)
5. [Plugin Architecture](#plugin-architecture)
6. [Data Flow](#data-flow)
7. [Rust Components Architecture](#rust-components-architecture)
8. [Security Architecture](#security-architecture)

---

## System Architecture

### Component Diagram

```
                                    ┌─────────────────────┐
                                    │   External Services │
                                    │  (LLM APIs, Web,    │
                                    │   Databases, etc.)  │
                                    └──────────┬──────────┘
                                               │
                                               ▼
┌──────────────────────────────────────────────────────────────────────┐
│                         OpenClaw Platform                             │
│                                                                       │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │                     Messaging Channels                         │  │
│  │  ┌────────┐ ┌────────┐ ┌───────┐ ┌────────┐ ┌────────┐        │  │
│  │  │Telegram│ │Discord │ │ Slack │ │ Signal │ │WhatsApp│  ...   │  │
│  │  └────┬───┘ └────┬───┘ └───┬───┘ └────┬───┘ └────┬───┘        │  │
│  │       │          │         │          │          │             │  │
│  │  ┌────▼──────────▼─────────▼──────────▼──────────▼────────┐   │  │
│  │  │              Channel Adapter Layer                      │   │  │
│  │  │  (Unified interface for all messaging platforms)        │   │  │
│  │  └─────────────────────┬───────────────────────────────────┘   │  │
│  └─────────────────────────┼───────────────────────────────────────┘
│                            │
│  ┌─────────────────────────▼───────────────────────────────────┐   │
│  │                   Gateway Core                              │   │
│  │  ┌─────────────┐  ┌──────────────┐  ┌─────────────────┐    │   │
│  │  │   Session   │  │    Message   │  │  Authentication │    │   │
│  │  │  Manager    │  │    Router    │  │    & AuthZ      │    │   │
│  │  └─────────────┘  └──────────────┘  └─────────────────┘    │   │
│  │  ┌─────────────┐  ┌──────────────┐  ┌─────────────────┐    │   │
│  │  │    Chat     │  │     Event    │  │   Rate Limit    │    │   │
│  │  │  Processor  │  │     Bus      │  │   & Throttle    │    │   │
│  │  └─────────────┘  └──────────────┘  └─────────────────┘    │   │
│  │  ┌─────────────┐  ┌──────────────┐  ┌─────────────────┐    │   │
│  │  │    Tool     │  │     Hook     │  │    Audit &      │    │   │
│  │  │  Invoker    │  │    Engine    │  │     Audit       │    │   │
│  │  └─────────────┘  └──────────────┘  └─────────────────┘    │   │
│  └─────────────────────────┬───────────────────────────────────┘   │
│                            │
│  ┌─────────────────────────▼───────────────────────────────────┐   │
│  │                   Agent Layer                               │   │
│  │  ┌─────────────┐  ┌──────────────┐  ┌─────────────────┐    │   │
│  │  │   Agent     │  │    Memory    │  │     Skills      │    │   │
│  │  │   Scope     │  │    System    │  │    Registry     │    │   │
│  │  └─────────────┘  └──────────────┘  └─────────────────┘    │   │
│  │  ┌─────────────┐  ┌──────────────┐  ┌─────────────────┐    │   │
│  │  │   Context   │  │    Model     │  │     Tools       │    │   │
│  │  │   Engine    │  │   Providers  │  │    Catalog      │    │   │
│  │  └─────────────┘  └──────────────┘  └─────────────────┘    │   │
│  └─────────────────────────┬───────────────────────────────────┘   │
│                            │
│  ┌─────────────────────────▼───────────────────────────────────┐   │
│  │                Extension/Skill Layer                        │   │
│  │  ┌─────────────┐  ┌──────────────┐  ┌─────────────────┐    │   │
│  │  │  Plugin     │  │   Rust       │  │   External      │    │   │
│  │  │    SDK      │  │   Skills     │  │  Integrations   │    │   │
│  │  └─────────────┘  └──────────────┘  └─────────────────┘    │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

### Layer Responsibilities

| Layer | Responsibility | Implementation |
|-------|---------------|----------------|
| Channel | Protocol translation, message send/receive | `src/telegram/`, `src/discord/`, etc. |
| Gateway | Routing, session management, auth | `src/gateway/` |
| Agent | AI logic, memory, skills | `src/agents/` |
| Extension | Custom functionality | `src/plugin-sdk/`, `skills/` |

---

## Gateway Core Architecture

### Server Implementation Structure

The gateway server is implemented across multiple files:

```
src/gateway/
├── server.impl.ts           # 36.9K lines - Core server implementation
├── server-http.ts           # 26.9K lines - HTTP API server
├── server-chat.ts           # 19.6K lines - Chat processing
├── server-node-events.ts    # 19.5K lines - Node event handling
├── server-cron.ts           # 16.9K lines - Cron/scheduled jobs
├── server-channels.ts       # 15.5K lines - Channel management
├── auth.ts                  # 15.6K lines - Authentication
├── hooks.ts                 # 12.9K lines - Hook system
├── net.ts                   # 12.4K lines - Network handling
├── openai-http.ts           # 17.4K lines - OpenAI API compatibility
├── openresponses-http.ts    # 25.2K lines - OpenAI Responses API
└── [100+ supporting files]
```

### Gateway Request Flow

```
1. Incoming Request (WebSocket/HTTP)
         │
         ▼
2. Connection Auth (connection-auth.ts)
         │
         ▼
3. Rate Limit Check (auth-rate-limit.ts)
         │
         ▼
4. Session Resolution (sessions-resolve.ts)
         │
         ▼
5. Message Processing (chat-attachments.ts, chat-sanitize.ts)
         │
         ▼
6. Agent Routing (agent-prompt.ts)
         │
         ▼
7. Tool Invocation (tools-invoke-http.ts)
         │
         ▼
8. Hook Execution (hooks.ts)
         │
         ▼
9. Response Send (server-chat.ts)
         │
         ▼
10. Audit Log (control-plane-audit.ts)
```

### Session Management Architecture

```typescript
// Session key structure
type SessionKey = string;  // Format: "agent:<agentId>:<mainKey>"

// Session resolution flow
resolveSessionKey(input: string | null, options: {
  dmScope: DMScope;
  identityLinks: Map<string, string>;
  channelType: ChannelType;
  accountId?: string;
}): SessionKey {
  // 1. Check for explicit session key
  if (input?.startsWith('agent:')) return input;

  // 2. Apply DM scoping policy
  switch (dmScope) {
    case 'main':
      return `agent:${agentId}:main`;
    case 'per-peer':
      return `agent:${agentId}:${peerId}`;
    case 'per-channel-peer':
      return `agent:${agentId}:${channelType}:${peerId}`;
    case 'per-account-channel-peer':
      return `agent:${agentId}:${accountId}:${channelType}:${peerId}`;
  }
}
```

### Session Storage Format

```json
{
  "sessions": {
    "agent:main:main": {
      "key": "agent:main:main",
      "label": "Main Session",
      "createdAt": "2026-03-22T00:00:00Z",
      "lastActivityAt": "2026-03-22T12:00:00Z",
      "channelType": "telegram",
      "peerId": "123456789",
      "compactedHistory": [...],
      "metadata": {}
    }
  },
  "hmacChain": {
    "lastHash": "sha256(...)",
    "keyId": "key-123"
  }
}
```

---

## Channel Architecture

### Channel Plugin Interface

All channels implement a common interface defined in the Plugin SDK:

```typescript
// src/plugin-sdk/index.ts
interface ChannelPlugin {
  // Channel identity
  get type(): ChannelType;
  get capabilities(): ChannelCapabilities;

  // Outbound messaging
  send(message: OutboundMessage): Promise<SendResult>;

  // Inbound messaging (polling or webhook)
  poll(): AsyncIterable<InboundMessage>;
  handleWebhook(request: Request): Promise<Response>;

  // Channel management
  start(): Promise<void>;
  stop(): Promise<void>;
  getStatus(): ChannelStatus;

  // Configuration
  validateConfig(config: Record<string, unknown>): ValidationResult;
}
```

### Channel Implementation Structure

```
src/telegram/
├── index.ts                 # Channel registration
├── bot.ts                   # Telegram bot wrapper
├── polling.ts               # Long polling implementation
├── webhook.ts               # Webhook handling
├── message-formatting.ts    # Message conversion
└── config.ts                # Configuration schema

src/discord/
├── index.ts                 # Channel registration
├── client.ts                # Discord client wrapper
├── gateway.ts               # Discord Gateway connection
├── message-formatting.ts    # Message conversion
└── permissions.ts           # Permission handling

[Similar structure for other channels]
```

### Channel Message Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    Inbound Message Flow                          │
└─────────────────────────────────────────────────────────────────┘

Channel (Telegram)
       │
       ▼
┌──────────────┐
│ Raw Message  │  (Telegram Update)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Parse &    │  (message-formatting.ts)
│  Normalize   │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Channel    │  (InboundEnvelope)
│   Envelope   │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Routing    │  (Apply DM scoping, group policy)
│   Decision   │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Session    │  (Resolve session key)
│  Resolution  │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Gateway    │  (Queue for processing)
│    Queue     │
└──────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                   Outbound Message Flow                          │
└─────────────────────────────────────────────────────────────────┘

Agent Response
       │
       ▼
┌──────────────┐
│   Hook       │  (pre-send hooks)
│  Processing  │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Channel    │  (Select outbound channel)
│  Selection   │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Message    │  (Convert to channel format)
│  Formatting  │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Channel    │  (Send via Telegram/Discord/etc.)
│     Send     │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   External   │  (User receives message)
│   Channel    │
└──────────────┘
```

---

## Agent Architecture

### Agent Scope System

```typescript
// src/agents/agent-scope.ts
class AgentScope {
  private agentId: string;
  private modelConfig: ModelConfig;
  private tools: Tool[];
  private memory: MemoryBackend;
  private hooks: HookRegistry;

  async processPrompt(prompt: Prompt): Promise<AgentResponse> {
    // 1. Load session context
    const context = await this.loadContext(prompt.sessionKey);

    // 2. Apply hooks (pre-prompt)
    await this.hooks.execute('pre-prompt', { prompt, context });

    // 3. Build model request
    const request = this.buildModelRequest(prompt, context);

    // 4. Invoke model
    const response = await this.invokeModel(request);

    // 5. Process tool calls
    const toolResults = await this.executeToolCalls(response.toolCalls);

    // 6. Apply hooks (post-response)
    await this.hooks.execute('post-response', { response, toolResults });

    // 7. Update memory
    await this.memory.store(prompt, response, toolResults);

    return { response, toolResults };
  }
}
```

### Memory System Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                     Memory System                             │
│                                                               │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────────┐     │
│  │   Vector    │  │  Episodic    │  │   Semantic      │     │
│  │   Memory    │  │   Memory     │  │    Memory       │     │
│  │  (LanceDB)  │  │  (Sessions)  │  │   (Facts)       │     │
│  └─────────────┘  └──────────────┘  └─────────────────┘     │
│                                                               │
│  ┌─────────────────────────────────────────────────────┐     │
│  │              Memory Backend Interface                │     │
│  │                                                       │     │
│  │  - store(event: MemoryEvent): Promise<void>          │     │
│  │  - search(query: string, limit: number): Promise<>   │     │
│  │  - get(sessionKey: string): Promise<SessionData>     │     │
│  │  - compact(sessionKey: string): Promise<void>        │     │
│  └─────────────────────────────────────────────────────┘     │
└──────────────────────────────────────────────────────────────┘
```

### Tool Invocation Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                   Tool Invocation Flow                           │
└─────────────────────────────────────────────────────────────────┘

Agent decides to invoke tool
         │
         ▼
┌─────────────────┐
│ Tool Permission │  (Check tool allowlist, approvals)
│     Check       │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   Approval      │  (Wait for user approval if required)
│   Required?     │
└────────┬────────┘
         │
    ┌────┴────┐
    │  Yes    │  No
    ▼         ▼
┌────────┐  ┌─────────────────┐
│ Wait   │  │  Tool Executor  │
│ for    │  │                 │
│ Approv │  └────────┬────────┘
└───┬────┘           │
    │                ▼
    │         ┌─────────────────┐
    │         │   Node Host     │  (Sandbox execution)
    │         │                 │
    │         └────────┬────────┘
    │                  │
    │                  ▼
    │         ┌─────────────────┐
    │         │   Result        │
    │         │   Processing    │
    │         └────────┬────────┘
    │                  │
    └──────────────────┘
                       │
                       ▼
┌─────────────────────────────────┐
│  Result returned to Agent       │
│  (Added to context for next     │
│   model iteration)              │
└─────────────────────────────────┘
```

---

## Plugin Architecture

### Plugin SDK Structure

```
src/plugin-sdk/
├── index.ts                 # Main exports (27.2K lines)
├── channel-plugin-common.ts # Shared channel utilities
├── webhook-targets.ts       # Webhook target management
├── webhook-request-guards.ts # Request validation
├── ssrf-policy.ts           # SSRF protection
├── auth-profiles/           # Auth profile management
├── secrets/                 # Secret handling
└── [100+ utility modules]
```

### Plugin Lifecycle

```
1. Plugin Discovery
   └─> Scan extensions/ directory
   └─> Read package.json for 'openclaw-plugin' field

2. Plugin Validation
   └─> Verify plugin interface implementation
   └─> Check dependencies and permissions

3. Plugin Registration
   └─> Register channel types
   └─> Register tools and commands
   └─> Register webhooks

4. Plugin Execution
   └─> Sandboxed module loading (jiti)
   └─> Capability-based access control
   └─> Rate limiting enforcement
```

### Plugin Security Boundary

```typescript
// Plugin sandbox configuration
const pluginSandbox = {
  // Allowed globals
  globals: {
    console: sanitizedConsole,
    setTimeout: restrictedSetTimeout,
    // ... no process, no require
  },

  // Module resolution
  moduleResolution: {
    alias: {
      'openclaw/plugin-sdk': '/path/to/plugin-sdk',
    },
    blocklist: [
      'fs', 'child_process', 'net',
      'dgram', 'http', 'https'
    ],
  },

  // Network restrictions
  network: {
    ssrfProtection: true,
    allowedHosts: config.allowedHosts,
  },
};
```

---

## Data Flow

### Complete Request Lifecycle

```
┌─────────────────────────────────────────────────────────────────────┐
│                  Complete Request Lifecycle                          │
└─────────────────────────────────────────────────────────────────────┘

1. EXTERNAL REQUEST
   │
   │  (Telegram Message / Discord Event / Webhook / API Call)
   ▼
2. CHANNEL ADAPTER
   │  - Parse raw payload
   │  - Normalize to common format
   │  - Validate signature/auth
   ▼
3. GATEWAY ENTRY
   │  - Connection authentication
   │  - Rate limit check
   │  - Origin/CORS validation
   ▼
4. SESSION RESOLUTION
   │  - Resolve session key from message context
   │  - Apply DM scoping policy
   │  - Load or create session
   ▼
5. MESSAGE QUEUE
   │  - Add to agent processing queue
   │  - Respect concurrency limits
   ▼
6. AGENT PROCESSING
   │  - Load session context/history
   │  - Execute pre-prompt hooks
   │  - Build model request
   │  - Invoke LLM provider
   ▼
7. RESPONSE PROCESSING
   │  - Parse model response
   │  - Extract tool calls
   │  - Validate output
   ▼
8. TOOL EXECUTION (if applicable)
   │  - Check tool permissions
   │  - Request approval if required
   │  - Execute in sandbox
   │  - Capture output
   ▼
9. ITERATION (if tool results)
   │  - Add tool results to context
   │  - Return to step 6
   ▼
10. FINAL RESPONSE
    │  - Execute post-response hooks
    │  - Format for channel
    │  - Send outbound message
    ▼
11. AUDIT & PERSISTENCE
    │  - Log to audit trail
    │  - Update session history
    │  - Trigger compaction if needed
    ▼
12. METRICS & TELEMETRY
       - Update Prometheus metrics
       - Send to telemetry backend (if configured)
```

---

## Rust Components Architecture

### X Intelligence CLI Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                   X Intelligence CLI (xint-rs)                   │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│  CLI Layer (cli.rs - 18.7K lines)                                │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Commands: Search, Watch, Stream, Analyze, Tweet, ...     │  │
│  │  Policy: ReadOnly | Engagement | Moderation               │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Application Layer (main.rs)                                     │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  - Config loading                                          │  │
│  │  - Policy enforcement                                      │  │
│  │  - Command dispatch                                        │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              │               │               │
              ▼               ▼               ▼
┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐
│   API Layer      │ │   Auth Layer     │ │  Command Layer   │
│  (api/)          │ │  (auth/)         │ │  (commands/)     │
│ ┌──────────────┐ │ │ ┌──────────────┐ │ │ ┌──────────────┐ │
│ │twitter.rs    │ │ │ │oauth.rs      │ │ │ │search.rs     │ │
│ │xai.rs        │ │ │ │mod.rs        │ │ │ │watch.rs      │ │
│ │grok.rs       │ │ │ └──────────────┘ │ │ │stream.rs     │ │
│ └──────────────┘ │ │                  │ │ │analyze.rs    │ │
└──────────────────┘ └──────────────────┘ │ │└──────────────┘ │
                                          └──────────────────┘
              │               │               │
              └───────────────┼───────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Core Services                                                   │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────┐   │
│  │  client.rs │ │  costs.rs  │ │  format.rs │ │  models.rs │   │
│  │  (9.2K)    │ │  (9.1K)    │ │  (8.6K)    │ │  (9.9K)    │   │
│  └────────────┘ └────────────┘ └────────────┘ └────────────┘   │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────┐   │
│  │  policy.rs │ │reliability │ │sentiment.rs│ │   mcp.rs   │   │
│  │  (3.0K)    │ │  (7.1K)    │ │  (7.2K)    │ │  (26.5K)   │   │
│  └────────────┘ └────────────┘ └────────────┘ └────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### Clauditor Security Watchdog Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Clauditor Architecture                        │
│                   (Tamper-Resistant Auditing)                    │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│  Userspace Components                                            │
│                                                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  clauditor-cli (CLI Interface)                              │ │
│  │  - daemon: Start watchdog service                          │ │
│  │  - digest: Generate reports                                │ │
│  │  - verify: Verify log integrity                            │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  Collector (collector/)                                     │ │
│  │  - DevCollector: Development mode (no fanotify)            │ │
│  │  - PrivilegedCollector: Production (fanotify FAN_OPEN_EXEC)│ │
│  │  - Event filtering and normalization                       │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  Detector (detector/)                                       │ │
│  │  - SequenceDetector: Detect suspicious patterns            │ │
│  │  - CommandBaseline: Track known commands                   │ │
│  │  - Alert generation with severity levels                   │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  Writer (writer/)                                           │ │
│  │  - AppendWriter: HMAC-chained log writing                  │ │
│  │  - FsyncPolicy: Durability guarantees                      │ │
│  │  - Tamper-evident format                                   │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  Alerter (alerter/)                                         │ │
│  │  - Alert dispatch (syslog, file, command, webhook)         │ │
│  │  - Alert deduplication                                     │ │
│  │  - Severity-based routing                                  │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  Schema (schema/)                                           │ │
│  │  - Log entry schema                                         │ │
│  │  - HMAC chain verification                                 │ │
│  │  - Integrity checking                                      │ │
│  └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Kernel Integration (fanotify)                                   │
│                                                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  Linux Kernel                                                │ │
│  │  ┌──────────────────────────────────────────────────────┐  │ │
│  │  │  fanotify API                                         │  │ │
│  │  │  - FAN_OPEN_EXEC: Exec event notifications           │  │ │
│  │  │  - Path filtering                                     │  │ │
│  │  └──────────────────────────────────────────────────────┘  │ │
│  └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘

Security Model:
┌─────────────────────────────────────────────────────────────────┐
│  Component      │ Owner      │ Perms │ Clawdbot Access         │
│─────────────────────────────────────────────────────────────────│
│  Daemon         │ sysaudit   │ -     │ Cannot kill             │
│  HMAC Key       │ root:sys   │ 640   │ Cannot read             │
│  Log Directory  │ sysaudit   │ 750   │ Cannot write            │
│  Log Files      │ sysaudit   │ 640   │ Read-only (audit trail) │
└─────────────────────────────────────────────────────────────────┘
```

---

## Security Architecture

### Threat Model

OpenClaw's security architecture is based on the following threat assumptions:

1. **Gateway Compromise:** The gateway process may be fully compromised
2. **Credential Theft:** Attacker may obtain stored credentials
3. **Session Hijacking:** Attacker may intercept or hijack sessions
4. **Plugin Malice:** Plugins may attempt malicious actions

### Defense in Depth

```
┌─────────────────────────────────────────────────────────────────┐
│                    Security Layers                               │
└─────────────────────────────────────────────────────────────────┘

Layer 1: Network Security
├─ TLS for all external communication
├─ Origin validation for web connections
├─ CORS policy enforcement
└─ SSRF protection for webhooks

Layer 2: Authentication
├─ Device pairing with QR codes
├─ Token-based API authentication
├─ Session-based auth for web UI
└─ Rate limiting on auth attempts

Layer 3: Authorization
├─ Role-based access control
├─ Tool-level permissions
├─ Command allowlisting
└─ Execution approval workflows

Layer 4: Isolation
├─ Agent scope isolation
├─ Session key separation
├─ Plugin sandboxing
└─ Node execution sandboxes

Layer 5: Audit
├─ Tamper-evident logging (Clauditor)
├─ HMAC-chained log entries
├─ Audit trail for sensitive operations
└─ Security event alerting

Layer 6: Secret Management
├─ Encryption at rest
├─ Memory-only runtime storage
├─ External secret store support
└─ Secret rotation support
```

### Authentication Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                 Authentication Flow                              │
└─────────────────────────────────────────────────────────────────┘

1. Device Pairing Flow
   ┌─────────┐         ┌─────────┐         ┌─────────┐
   │  New    │────────>│  Gen    │────────>│  Show   │
   │ Device  │         │  Pair   │         │   QR    │
   │         │         │  Code   │         │  Code   │
   └─────────┘         └─────────┘         └─────────┘
                                                 │
                                                 ▼
   ┌─────────┐         ┌─────────┐         ┌─────────┐
   │  Store  │<────────│  Verify │<────────│  Scan   │
   │  Token  │         │  Code   │         │   QR    │
   └─────────┘         └─────────┘         └─────────┘

2. Token Auth Flow
   ┌─────────┐         ┌─────────┐         ┌─────────┐
   │ Request │────────>│ Validate│────────>│  Check  │
   │  with   │         │   HMAC  │         │  Rate   │
   │  Token  │         │  Sig    │         │  Limit  │
   └─────────┘         └─────────┘         └─────────┘
                           │
                    ┌──────┴──────┐
                    │  Valid?     │
                    └──────┬──────┘
                         Yes│No
                    ┌──────┴──────┐
                    ▼             ▼
               ┌─────────┐   ┌─────────┐
               │ Proceed │   │ Reject  │
               │  with   │   │   401   │
               │ Request │   │         │
               └─────────┘   └─────────┘
```

### Secret Resolution

```typescript
// Secret resolution flow
resolveSecret(secretRef: string): string {
  // Format: $secret:<name> or $secret:<name>:<field>

  // 1. Parse secret reference
  const { name, field } = parseSecretRef(secretRef);

  // 2. Check local secret store
  let secret = localStore.get(name);

  // 3. Fall back to gateway secret store
  if (!secret) {
    secret = gatewaySecrets.get(name);
  }

  // 4. Extract field if specified
  if (field) {
    secret = secret[field];
  }

  // 5. Decrypt if encrypted
  if (secret.encrypted) {
    secret = decrypt(secret.ciphertext);
  }

  return secret;
}
```

---

## Summary

The src.openclaw architecture demonstrates:

1. **Layered Design** - Clear separation between channels, gateway, agents, and extensions
2. **Event-Driven** - Async message flow with queuing and backpressure handling
3. **Security-First** - Multiple defense layers with tamper-evident logging
4. **Extensible** - Plugin SDK with sandboxing and capability-based access
5. **Production-Grade** - Comprehensive error handling, observability, and deployment options

Key architectural decisions:
- TypeScript for rapid development and ecosystem integration
- Rust for security-critical and performance-sensitive components
- Session-based state management with HMAC integrity
- Policy-based access control throughout
- Hook system for extensibility without code modification
