---
title: 14 — Data Flow — End-to-End Flows, State Machines, Request/Response Shapes
---

# 14 — Data Flow — End-to-End Flows, State Machines, Request/Response Shapes

This document traces the key data flows across the nexu ecosystem, from agent detection through artifact generation.

## Agent Detection Flow

```mermaid
sequenceDiagram
    participant User
    participant UI as Web UI
    participant Daemon as Open Design Daemon
    participant PATH as System PATH
    participant Agent as Coding Agent CLI

    User->>UI: Open Settings → Agents
    UI->>Daemon: GET /api/agents (SSE)
    Daemon->>PATH: Scan for known binaries
    loop For each agent def (23)
        Daemon->>PATH: resolveAgentExecutable(bin)
        PATH-->>Daemon: resolved path or null
        alt Path found
            Daemon->>Agent: <bin> --version
            Agent-->>Daemon: version string
            Daemon->>Agent: <bin> --help (capabilities)
            Agent-->>Daemon: help text
            Daemon->>Agent: <bin> models list
            Agent-->>Daemon: model list
            Daemon->>Agent: <bin> auth status
            Agent-->>Daemon: auth status
            Daemon-->>UI: DetectedAgent (SSE event)
        else Path not found
            Daemon-->>UI: DetectedAgent{available: false}
        end
    end
    UI->>UI: Paint agent cards as received
```

Source: `open-design/apps/daemon/src/runtimes/detection.ts:346-363` — detectAgentsStream().

## Chat Flow

```mermaid
sequenceDiagram
    participant User
    participant Web as Web UI
    participant Daemon as Open Design Daemon
    participant Agent as Coding Agent CLI
    participant Store as Artifact Store

    User->>Web: Type prompt, click Send
    Web->>Daemon: POST /api/chat {prompt, agent, model, designSystem}
    Daemon->>Daemon: resolveAgentExecutable(agent)
    Daemon->>Daemon: buildArgs(prompt, context)
    Daemon->>Agent: spawn(bin, args, env)
    Agent-->>Daemon: stdout stream (JSONL)
    Daemon->>Daemon: parse stream events
    loop For each token
        Daemon-->>Web: SSE: token {content}
        Web->>Web: Append to chat
    end
    Agent-->>Daemon: stdout stream (HTML output)
    Daemon->>Store: save artifact
    Daemon-->>Web: SSE: complete {artifactId}
    Web->>Web: Render artifact in iframe
```

Source: `open-design/apps/daemon/src/runtimes/claude-stream.ts` — Claude stream parser.

## Nexu Config → OpenClaw Flow

```mermaid
sequenceDiagram
    participant UI as Nexu Web UI
    participant Ctrl as Nexu Controller
    participant Store as ConfigStore
    participant Writer as ConfigWriter
    participant OC as OpenClaw Process
    participant GW as OpenClaw Gateway

    UI->>Ctrl: PUT /api/channels/slack {credentials}
    Ctrl->>Store: write channel config
    Ctrl->>Writer: compile OpenClaw config
    Writer->>Writer: merge agents + channels + bindings
    Writer->>Writer: validate cross-references
    Writer->>Writer: write OPENCLAW_CONFIG_PATH
    Ctrl->>OC: restart("config_changed")
    OC->>GW: start with new config
    GW-->>Ctrl: WebSocket connected
    Ctrl->>Ctrl: update runtimeState
    Ctrl-->>UI: 200 OK
    UI->>Ctrl: GET /health
    Ctrl-->>UI: {status, sync, gateway}
```

Source: `nexu/apps/controller/src/lib/openclaw-config-compiler.ts` — Config compiler.

## Looper Planner Flow

```mermaid
sequenceDiagram
    participant GH as GitHub
    participant Scheduler as Queue Scheduler
    participant Planner as Planner Runner
    participant Git as Git
    participant Agent as Coding Agent
    participant DB as SQLite

    Scheduler->>Planner: ClaimNextOfType("planner")
    Planner->>GH: ViewIssue(repo, number)
    GH-->>Planner: Issue detail
    Planner->>DB: Acquire lock(issue)
    Planner->>Git: CreateWorktree(branch)
    Git-->>Planner: Worktree created
    Planner->>Agent: spawn(agent, prompt)
    Agent-->>Planner: Spec output
    Planner->>Git: Commit, InspectHead
    Planner->>Git: Push
    Planner->>GH: CreatePullRequest
    GH-->>Planner: PR created
    Planner->>GH: AddLabels("specpr:reviewing")
    Planner->>GH: AddReviewers
    Planner->>DB: Complete queue item, persist checkpoint
```

Source: `looper/internal/planner/runner.go:632-777` — ProcessClaimedItem().

## Design System → Artifact Flow

```mermaid
flowchart LR
    URL[Brand URL] --> DS[Design System Import]
    DS --> ST[SeedToken 20 fields]
    ST --> DT[Derive DesignTokens 80+ fields]
    DT --> EX[Export tokens.json/css/theme.json]
    DT --> KT[Kit themed components]
    KT --> LA[Landing artifact]
    KT --> EM[Email artifact]
    KT --> PO[Poster artifact]
    KT --> DK[Deck artifact]

    EX --> PV[Preview URLs]
    LA --> PV
    EM --> PV
    PO --> PV
    DK --> PV
```

Source: `open-design/apps/daemon/src/brands/engine/types.ts:7-11` — Pipeline comment.

## HTML Video Render Flow

```mermaid
sequenceDiagram
    participant User
    participant Studio as Studio Web UI
    participant CLI as html-video CLI
    participant Engine as Render Engine
    participant Chrome as Headless Chromium
    participant FFmpeg as ffmpeg

    User->>Studio: Select template, set variables
    Studio->>CLI: POST /api/render {template, variables, config}
    CLI->>Engine: adapter.render(input, context)
    Engine->>Engine: Prepare workDir
    Engine->>Engine: Render template with variables
    Engine->>Chrome: Load HTML, record frames
    Chrome-->>Engine: Frame images
    Engine->>FFmpeg: Encode frames to MP4
    FFmpeg-->>Engine: video.mp4
    Engine-->>CLI: RenderOutput {outputPath, meta}
    CLI-->>Studio: SSE: complete
    Studio->>Studio: Display video preview
```

Source: `html-video/packages/adapter-hyperframes/src/render.ts` — HyperFrames render.

## SSE Event Shapes

### Chat Token Event

```json
{
  "event": "token",
  "data": "{\"content\": \"<div class=\\\"hero\\\">...\"}"
}
```

### Chat Complete Event

```json
{
  "event": "complete",
  "data": "{\"artifactId\": \"art_abc123\", \"status\": \"done\"}"
}
```

### Agent Detection Event

```json
{
  "event": "agent",
  "data": "{\"id\": \"claude\", \"name\": \"Claude Code\", \"available\": true, \"version\": \"2025.1.0\", ...}"
}
```

## State Machines

### Loop Status (Looper)

```
queued → running → completed
                → failed → queued (retry)
                → paused → queued (resume)
                → terminated
```

Source: `looper/internal/loops/service.go` — TransitionStatus().

### Queue Item Status

```
queued → downloading → installing-deps → done
                                       → failed
```

Source: `nexu/apps/controller/src/services/skillhub/types.ts:36-42` — QueueItemStatus.

## Request/Response Shapes

### POST /api/chat (Open Design)

**Request:**
```typescript
{
  prompt: string;
  agentId: string;
  model: string;
  designSystem?: string;
  imagePaths?: string[];
  sessionId?: string;
}
```

**Response:** SSE stream

### GET /api/agents (Open Design)

**Response:** `DetectedAgent[]` or SSE stream of `DetectedAgent`

### POST /api/skillhub/install (Nexu)

**Request:**
```typescript
{
  slug: string;
  source: SkillSource;
}
```

**Response:** `QueueItem`

### GET /health (Nexu)

**Response:**
```typescript
{
  status: string;
  controlPlane: { ok: boolean; ... };
  sync: { config: string; skills: string; templates: string };
  gateway: { status: string; lastProbeAt: string; lastError: string };
}
```

Source: `nexu/apps/controller/src/app/create-app.ts:61-82` — Health endpoint.

**Aha:** Every major project uses the same SSE event pattern for streaming agent output. The event names differ (`token`, `agent`, `complete`), but the structure is consistent: JSON-encoded data field with event type. This consistency means the web UI can reuse the same SSE client code across open-design, html-anything, and nexu.

## Next

- [15 — Developer Tooling](15-developer-tooling.md) — CLI, testing, CI/CD, deployment
- [01 — Architecture](01-architecture.md) — Full ecosystem architecture
