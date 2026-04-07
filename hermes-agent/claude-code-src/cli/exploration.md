# Claude Code CLI Module — Deep-Dive Exploration

**Module:** `cli/`  
**Parent Project:** [index.md](../index.md)  
**Task Reference:** [task.md](../task.md) — Task #11  
**Created:** 2026-04-07  
**Files:** 19 TypeScript/TSX files

---

## 1. Module Overview

The `cli/` module implements the **SDK print mode** and **remote session I/O** layer for Claude Code. It provides the transport infrastructure for headless/SDK operation, enabling programmatic interaction with Claude Code via structured NDJSON messaging over stdio or remote HTTP connections.

### Core Responsibilities

1. **Transport Layer** — Abstracted communication protocols:
   - `WebSocketTransport`: Bidirectional WS with auto-reconnect, ping/pong keepalive
   - `HybridTransport`: WS reads + HTTP POST writes with batch buffering
   - `SSETransport`: Server-Sent Events for reads + HTTP POST for writes (CCR v2)

2. **CCR v2 Client** — Session worker protocol:
   - Epoch management for worker lifecycle
   - Heartbeat for liveness detection
   - Event batching with exponential backoff

3. **Structured I/O** — SDK message parsing:
   - NDJSON stdin/stdout serialization
   - Control request/response handling
   - Hook integration for permission prompts

4. **Print Mode Entry Point** — Headless orchestration:
   - Non-interactive session runner
   - Tool pooling and MCP integration
   - Queue management for batched commands

### Key Design Patterns

- **SerialBatchEventUploader**: Ordered event delivery with backpressure control
- **Text Delta Coalescing**: Accumulates chunks into full-so-far snapshots
- **FlushGate**: State machine for gating writes during history flush
- **CircularBuffer**: Bounded FIFO for message replay on reconnect

---

## 2. File Inventory

| File | Lines | Description |
|------|-------|-------------|
| `exit.ts` | 31 | CLI exit helpers (`cliError`, `cliOk`) |
| `ndjsonSafeStringify.ts` | 32 | Escapes U+2028/U+2029 for safe NDJSON line-splitting |
| `transports/transportUtils.ts` | 45 | Transport factory (v1/v2 selection via env) |
| `handlers/agents.ts` | 70 | `claude agents` subcommand handler |
| `handlers/util.tsx` | 109 | Misc handlers: `setup-token`, `doctor`, `install` |
| `transports/WorkerStateUploader.ts` | 131 | Coalescing PUT /worker with RFC 7396 merge |
| `handlers/autoMode.ts` | 170 | `claude auto-mode` defaults/config/critique |
| `remoteIO.ts` | 255 | Bidirectional streaming for SDK mode with CCR v2 |
| `transports/SerialBatchEventUploader.ts` | 275 | Serial ordered uploader with backpressure |
| `transports/HybridTransport.ts` | 282 | WS reads + HTTP POST writes with 100ms batching |
| `handlers/auth.ts` | 330 | `claude auth login/logout/status` handlers |
| `handlers/mcp.tsx` | 361 | `claude mcp serve/remove/list/get/add-json/import` |
| `update.ts` | 422 | Auto-update logic with diagnostic checks |
| `transports/SSETransport.ts` | 711 | SSE parsing with reconnection, Last-Event-ID |
| `transports/WebSocketTransport.ts` | 800 | Full WS transport with auto-reconnect, ping/pong |
| `structuredIO.ts` | 859 | SDK message parsing via stdin/stdout |
| `handlers/plugins.ts` | 878 | `claude plugin/marketplace` subcommands |
| `transports/ccrClient.ts` | 998 | CCR v2 worker lifecycle management |
| `print.ts` | 5594 | Main entry point for SDK print mode |
| **Total** | **11,953** | 19 TypeScript/TSX files |

---

## 3. Key Exports

### Transport Layer

```typescript
// transports/Transport.ts (inferred interface)
interface Transport {
  connect(): Promise<void>
  write(message: StdoutMessage): Promise<void>
  setOnData(callback: (data: string) => void): void
  setOnConnect(callback: () => void): void
  setOnClose(callback: (closeCode?: number) => void): void
  close(): void
  isConnectedStatus(): boolean
  isClosedStatus(): boolean
}
```

```typescript
// transports/HybridTransport.ts
export class HybridTransport extends WebSocketTransport {
  private streamEventBuffer: StdoutMessage[]
  private streamEventTimer: ReturnType<typeof setTimeout> | null
  private readonly uploader: SerialBatchEventUploader<StdoutMessage>
  
  // Overrides write() to batch stream_event messages
  async write(message: StdoutMessage): Promise<void>
}
```

```typescript
// transports/SSETransport.ts
export class SSETransport implements Transport {
  private eventSource: EventSource | null
  private lastEventId: string
  private reconnectAttempts: number
  
  // Parses SSE frames: event:, id:, data: fields
  async connect(): Promise<void>
  private parseSSEFrames(buffer: string): { frames: SSEFrame[], remaining: string }
}
```

```typescript
// transports/SerialBatchEventUploader.ts
export class SerialBatchEventUploader<T> {
  private pending: T[]
  private draining: boolean
  private backpressureResolvers: Array<() => void>
  
  constructor(config: {
    send: (batch: T[]) => Promise<boolean>
    maxQueueSize: number
    maxBatchSize: number
    baseDelayMs: number
    maxDelayMs: number
  })
  
  async enqueue(events: T | T[]): Promise<void>
  private async drain(): Promise<void>
}
```

### CCR Client

```typescript
// transports/ccrClient.ts
export class CCRClient {
  private workerEpoch: number
  private heartbeatTimer: NodeJS.Timeout | null
  private streamEventBuffer: SDKPartialAssistantMessage[]
  private streamTextAccumulator: StreamAccumulatorState
  
  constructor(
    transport: SSETransport,
    sessionUrl: URL,
    opts?: { onEpochMismatch?: () => never }
  )
  
  initialize(): Promise<void>
  writeEvent(event: SDKPartialAssistantMessage): void
  updateWorkerState(state: Partial<SessionState>): void
  
  // Heartbeat loop (20s interval with jitter)
  private startHeartbeatLoop(): void
}

export function createStreamAccumulator(): StreamAccumulatorState
export function accumulateStreamEvents(
  buffer: SDKPartialAssistantMessage[],
  state: StreamAccumulatorState
): EventPayload[]
export function clearStreamAccumulatorForMessage(
  state: StreamAccumulatorState,
  assistant: { session_id: string; parent_tool_use_id: string | null; message: { id: string } }
): void
```

### Structured I/O

```typescript
// structuredIO.ts
export class StructuredIO {
  readonly outbound: Stream<StdoutMessage>
  private readonly pendingRequests: Map<string, PendingRequest<unknown>>
  private readonly resolvedToolUseIds: Set<string>
  
  constructor(
    input: AsyncIterable<string>,
    replayUserMessages?: boolean
  )
  
  async *read(): AsyncGenerator<StdinMessage | SDKMessage>
  prependUserMessage(content: string): void
  injectControlResponse(response: SDKControlResponse): void
  
  // Control request/response RPC pattern
  sendRequest<T>(request: SDKControlRequest, schema?: z.Schema): Promise<T>
}
```

```typescript
// remoteIO.ts
export class RemoteIO extends StructuredIO {
  private transport: Transport
  private ccrClient: CCRClient | null
  private internalEventBuffer: WorkerEvent[]
  
  constructor(
    streamUrl: string,
    initialPrompt?: string,
    replayUserMessages?: boolean
  )
  
  flushInternalEvents(): Promise<void>
  get internalEventsPending(): number
}
```

### Print Mode

```typescript
// print.ts
export async function runHeadless(
  inputPrompt: string | AsyncIterable<string>,
  getAppState: () => AppState,
  setAppState: (f: (prev: AppState) => AppState) => void,
  commands: Command[],
  tools: Tools,
  sdkMcpConfigs: Record<string, McpSdkServerConfig>,
  agents: AgentDefinition[],
  options: {
    continue?: boolean
    resume?: string | boolean
    verbose?: boolean
    outputFormat?: string
    jsonSchema?: Record<string, unknown>
    maxTurns?: number
    maxBudgetUsd?: number
    // ... more options
  }
): Promise<void>

export function joinPromptValues(values: PromptValue[]): PromptValue
export function canBatchWith(head: QueuedCommand, next: QueuedCommand): boolean
```

---

## 4. Line-by-Line Analysis

### 4.1 NDJSON Safe Stringify (`ndjsonSafeStringify.ts`)

```typescript
const JS_LINE_TERMINATORS = /\u2028|\u2029/g

export function ndjsonSafeStringify(value: unknown): string {
  return escapeJsLineTerminators(jsonStringify(value))
}

function escapeJsLineTerminators(str: string): string {
  return str.replace(JS_LINE_TERMINATORS, match =>
    match === '\u2028' ? '\\u2028' : '\\u2029'
  )
}
```

**Purpose**: JSON.stringify doesn't escape Unicode line terminators (U+2028 LINE SEPARATOR, U+2029 PARAGRAPH SEPARATOR). These are valid in JSON strings but break NDJSON line-splitting (splitting on `\n`). This function ensures safe serialization.

---

### 4.2 SerialBatchEventUploader (`transports/SerialBatchEventUploader.ts`)

```typescript
export class SerialBatchEventUploader<T> {
  private pending: T[] = []
  private draining = false
  private backpressureResolvers: Array<() => void> = []
  
  constructor(private readonly config: {
    send: (batch: T[]) => Promise<boolean>
    maxQueueSize: number
    maxBatchSize: number
    baseDelayMs: number
    maxDelayMs: number
  }) {}
  
  async enqueue(events: T | T[]): Promise<void> {
    const items = Array.isArray(events) ? events : [events]
    
    // Backpressure: wait if queue is full
    while (this.pending.length + items.length > this.config.maxQueueSize) {
      await new Promise<void>(resolve => this.backpressureResolvers.push(resolve))
    }
    
    this.pending.push(...items)
    void this.drain()
  }
  
  private async drain(): Promise<void> {
    if (this.draining || this.pending.length === 0) return
    this.draining = true
    
    try {
      while (this.pending.length > 0) {
        const batch = this.pending.splice(0, this.config.maxBatchSize)
        const success = await this.config.send(batch)
        
        if (!success) {
          // Re-queue failed batch at front
          this.pending.unshift(...batch)
          await this.retryDelay()
        }
      }
    } finally {
      this.draining = false
      // Release backpressure waiters
      while (this.backpressureResolvers.length > 0) {
        this.backpressureResolvers.shift()?.()
      }
    }
  }
  
  private async retryDelay(): Promise<void> {
    // Exponential backoff with jitter
  }
}
```

**Key Patterns**:
- **Serial ordering**: Events are sent in FIFO order, no reordering
- **Backpressure**: `enqueue()` awaits when queue exceeds `maxQueueSize`
- **Batching**: Multiple events coalesced into single HTTP POST
- **Retry with backoff**: Failed batches retry indefinitely with exponential delay

---

### 4.3 HybridTransport (`transports/HybridTransport.ts`)

```typescript
export class HybridTransport extends WebSocketTransport {
  private streamEventBuffer: StdoutMessage[] = []
  private streamEventTimer: ReturnType<typeof setTimeout> | null = null
  private readonly uploader: SerialBatchEventUploader<StdoutMessage>
  
  async write(message: StdoutMessage): Promise<void> {
    if (message.type === 'stream_event') {
      // Batch stream_event messages (100ms window)
      this.streamEventBuffer.push(message)
      if (!this.streamEventTimer) {
        this.streamEventTimer = setTimeout(() => this.flushStreamEvents(), 100)
      }
      return
    }
    
    // Non-stream events: send immediately with buffered stream events
    await this.uploader.enqueue([...this.takeStreamEvents(), message])
  }
  
  private flushStreamEvents(): void {
    const events = this.takeStreamEvents()
    if (events.length > 0) {
      void this.uploader.enqueue(events)
    }
    this.streamEventTimer = null
  }
}
```

**Design Rationale**:
- Stream events (content deltas) are high-frequency — batching reduces POST count
- 100ms window balances latency vs. throughput
- Non-stream events (tool results, errors) bypass the delay for immediate delivery

---

### 4.4 CCRClient — Text Delta Coalescing (`transports/ccrClient.ts`)

```typescript
/**
 * Accumulate text_delta stream_events into full-so-far snapshots.
 * Each flush emits ONE event per touched block containing the FULL
 * accumulated text from the start of the block.
 */
export function accumulateStreamEvents(
  buffer: SDKPartialAssistantMessage[],
  state: StreamAccumulatorState
): EventPayload[] {
  const out: EventPayload[] = []
  const touched = new Map<string[], CoalescedStreamEvent>()
  
  for (const msg of buffer) {
    switch (msg.event.type) {
      case 'message_start': {
        const id = msg.event.message.id
        const prevId = state.scopeToMessage.get(scopeKey(msg))
        if (prevId) state.byMessage.delete(prevId)  // Cleanup prior message
        state.scopeToMessage.set(scopeKey(msg), id)
        state.byMessage.set(id, [])
        out.push(msg)
        break
      }
      case 'content_block_delta': {
        if (msg.event.delta.type !== 'text_delta') {
          out.push(msg)
          break
        }
        const messageId = state.scopeToMessage.get(scopeKey(msg))
        const blocks = messageId ? state.byMessage.get(messageId) : undefined
        if (!blocks) {
          // No preceding message_start — pass through raw
          out.push(msg)
          break
        }
        const chunks = (blocks[msg.event.index] ??= [])
        chunks.push(msg.event.delta.text)
        const existing = touched.get(chunks)
        if (existing) {
          // Rewrite existing snapshot (full-so-far)
          existing.event.delta.text = chunks.join('')
          break
        }
        // Create new snapshot event
        const snapshot: CoalescedStreamEvent = {
          type: 'stream_event',
          uuid: msg.uuid,
          session_id: msg.session_id,
          parent_tool_use_id: msg.parent_tool_use_id,
          event: {
            type: 'content_block_delta',
            index: msg.event.index,
            delta: { type: 'text_delta', text: chunks.join('') },
          },
        }
        touched.set(chunks, snapshot)
        out.push(snapshot)
        break
      }
      default:
        out.push(msg)
    }
  }
  return out
}
```

**Why Coalescing Matters**:
- Without coalescing: 100 text_delta events → 100 POST requests
- With coalescing: 100 deltas → 1 snapshot event per flush
- Mid-stream reconnections see complete text, not fragments

---

### 4.5 StructuredIO — Control Request/Response (`structuredIO.ts`)

```typescript
export class StructuredIO {
  private readonly pendingRequests = new Map<string, PendingRequest<unknown>>()
  private readonly resolvedToolUseIds = new Set<string>()
  
  async *read(): AsyncGenerator<StdinMessage | SDKMessage> {
    // Parse NDJSON lines, handle control_response
    for await (const block of this.input) {
      content += block
      // Split on newlines, process each line
      for (const line of lines) {
        const message = await this.processLine(line)
        if (message?.type === 'control_response') {
          const { request_id, response } = message.response
          const pending = this.pendingRequests.get(request_id)
          if (pending) {
            this.resolvedToolUseIds.add(request_id)  // Track as resolved
            this.pendingRequests.delete(request_id)
            // Resolve or reject the pending promise
          }
        }
        yield message
      }
    }
  }
  
  async sendRequest<T>(request: SDKControlRequest, schema?: z.Schema): Promise<T> {
    return new Promise((resolve, reject) => {
      this.pendingRequests.set(request.request_id, { resolve, reject, schema, request })
      void this.outbound.write({
        type: 'control_request',
        request_id: request.request_id,
        request: request.request,
      })
    })
  }
}
```

**RPC Pattern**: `sendRequest()` returns a Promise that resolves when the matching `control_response` arrives. The `resolvedToolUseIds` set prevents duplicate processing of late/duplicate responses.

---

### 4.6 WorkerStateUploader — Coalescing Patches (`transports/WorkerStateUploader.ts`)

```typescript
/**
 * Coalescing uploader for PUT /worker (session state + metadata).
 * - 1 in-flight PUT + 1 pending patch
 * - New calls coalesce into pending (never grows beyond 1 slot)
 * - On success: send pending if exists
 * - On failure: exponential backoff, retries indefinitely
 */
export class WorkerStateUploader {
  private inflight: Promise<void> | null = null
  private pending: Record<string, unknown> | null = null
  
  enqueue(patch: Record<string, unknown>): void {
    if (this.closed) return
    this.pending = this.pending ? coalescePatches(this.pending, patch) : patch
    void this.drain()
  }
  
  private async sendWithRetry(payload: Record<string, unknown>): Promise<void> {
    let current = payload
    let failures = 0
    while (!this.closed) {
      const ok = await this.config.send(current)
      if (ok) return
      
      failures++
      await sleep(this.retryDelay(failures))
      
      // Absorb any patches that arrived during the retry
      if (this.pending && !this.closed) {
        current = coalescePatches(current, this.pending)
        this.pending = null
      }
    }
  }
}

/**
 * RFC 7396 merge for metadata keys (external_metadata, internal_metadata).
 * Top-level keys: overlay replaces base (last value wins).
 */
function coalescePatches(
  base: Record<string, unknown>,
  overlay: Record<string, unknown>
): Record<string, unknown> {
  const merged = { ...base }
  for (const [key, value] of Object.entries(overlay)) {
    if (
      (key === 'external_metadata' || key === 'internal_metadata') &&
      merged[key] &&
      typeof merged[key] === 'object' &&
      typeof value === 'object' &&
      value !== null
    ) {
      // RFC 7396: overlay keys are added/overwritten, null values preserved
      merged[key] = { ...merged[key], ...value }
    } else {
      merged[key] = value
    }
  }
  return merged
}
```

**Why Coalescing Matters**:
- Multiple state updates (e.g., tool calls, permission changes) arrive rapidly
- Without coalescing: N updates → N HTTP PUTs
- With coalescing: N updates → 1 PUT with merged state

---

### 4.7 WebSocketTransport — Auto-Reconnect (`transports/WebSocketTransport.ts`)

```typescript
export class WebSocketTransport implements Transport {
  private reconnectAttempts = 0
  private reconnectStartTime: number | null = null
  private lastReconnectAttemptTime: number | null = null
  private messageBuffer: CircularBuffer<StdoutMessage>
  
  private handleConnectionError(closeCode?: number): void {
    // Permanent close codes: don't retry
    if (
      closeCode != null &&
      PERMANENT_CLOSE_CODES.has(closeCode)
    ) {
      this.state = 'closed'
      this.onCloseCallback?.(closeCode)
      return
    }
    
    // Schedule reconnection with exponential backoff
    const now = Date.now()
    if (!this.reconnectStartTime) {
      this.reconnectStartTime = now
    }
    
    // Detect system sleep/wake
    if (
      this.lastReconnectAttemptTime !== null &&
      now - this.lastReconnectAttemptTime > SLEEP_DETECTION_THRESHOLD_MS
    ) {
      this.reconnectStartTime = now
      this.reconnectAttempts = 0  // Reset budget
    }
    
    const elapsed = now - this.reconnectStartTime
    if (elapsed < DEFAULT_RECONNECT_GIVE_UP_MS) {
      this.reconnectAttempts++
      const delay = this.calculateBackoffDelay()
      this.reconnectTimer = setTimeout(() => this.connect(), delay)
    } else {
      this.state = 'closed'
      this.onCloseCallback?.(closeCode)
    }
  }
  
  private replayBufferedMessages(lastId: string): void {
    const messages = this.messageBuffer.toArray()
    // Find where to start replay based on server's last received message
    const startIndex = messages.findIndex(m => m.uuid === lastId) + 1
    for (const message of messages.slice(startIndex)) {
      this.sendLine(jsonStringify(message) + '\n')
    }
  }
}
```

**Key Features**:
- **Exponential backoff**: 1s, 2s, 4s, 8s, 16s, 32s (capped at 30s)
- **Time budget**: Gives up after 10 minutes of continuous failures
- **Sleep detection**: Resets budget if machine slept (gap > 60s)
- **Message replay**: Buffered messages replay on reconnect (deduped by UUID)

---

### 4.8 Print Mode — Headless Runner (`print.ts`)

```typescript
export async function runHeadless(
  inputPrompt: string | AsyncIterable<string>,
  getAppState: () => AppState,
  setAppState: (f: (prev: AppState) => AppState) => void,
  commands: Command[],
  tools: Tools,
  sdkMcpConfigs: Record<string, McpSdkServerConfig>,
  agents: AgentDefinition[],
  options: { ... }
): Promise<void> {
  const structuredIO = getStructuredIO(inputPrompt, options)
  
  // Install guard to divert non-JSON lines to stderr
  if (options.outputFormat === 'stream-json') {
    installStreamJsonStdoutGuard()
  }
  
  // Initialize sandbox (optional)
  if (SandboxManager.isSandboxingEnabled()) {
    await SandboxManager.initialize(structuredIO.createSandboxAskCallback())
  }
  
  // Load initial messages (resume, continue, fork)
  const { messages: initialMessages } = await loadInitialMessages(...)
  
  // Main loop: process commands, run tools, stream events
  const queueManager = createMessageQueueManager()
  const idleTimeout = createIdleTimeoutManager()
  
  for await (const message of structuredIO.structuredInput) {
    if (message.type === 'user') {
      // Queue user turn
      await queueManager.enqueue({ mode: 'prompt', prompt: message.message })
    } else if (message.type === 'stream_event') {
      // Forward to stdout for SDK client
      await structuredIO.write(message)
    } else if (message.type === 'assistant') {
      // Process tool calls, permission prompts
      await processAssistantMessage(message)
    }
  }
}
```

**Orchestration Flow**:
1. Parse incoming NDJSON from stdin
2. Queue user turns and tool results
3. Batch compatible commands (same mode, workload)
4. Call API, stream response events back
5. Handle tool execution, permission prompts via control_request
6. Graceful shutdown on EOF or max turns/budget

---

## 5. Component Relationships

```
┌─────────────────────────────────────────────────────────────────┐
│                        SDK Host (Caller)                        │
│  (VS Code extension, custom script, claude.ai bridge)           │
└────────────────────────┬────────────────────────────────────────┘
                         │ NDJSON over stdio or remote URL
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                      print.ts (Entry Point)                     │
│  - runHeadless(): Main orchestration loop                       │
│  - commandQueue: Batches compatible turns                       │
│  - idleTimeout: Auto-shutdown on inactivity                     │
└────────────────────────┬────────────────────────────────────────┘
                         │
         ┌───────────────┴───────────────┐
         │                               │
         ▼                               ▼
┌─────────────────┐            ┌─────────────────┐
│ structuredIO.ts │            │   remoteIO.ts   │
│ - NDJSON parse  │            │ - Extends       │
│ - Control RPC   │            │   StructuredIO  │
│ - Hook handling │            │ - CCR v2 client │
└────────┬────────┘            └────────┬────────┘
         │                              │
         │                              ▼
         │               ┌─────────────────────────┐
         │               │      ccrClient.ts       │
         │               │ - Heartbeat (20s)       │
         │               │ - Epoch management      │
         │               │ - Event coalescing      │
         │               └───────────┬─────────────┘
         │                           │
         ▼                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Transport Layer                            │
│  ┌──────────────────┐  ┌──────────────────┐  ┌───────────────┐ │
│  │ WebSocketTransport│  │ HybridTransport  │  │ SSETransport  │ │
│  │ - WS full-duplex  │  │ - WS reads       │  │ - SSE reads   │ │
│  │ - Auto-reconnect  │  │ - POST writes    │  │ - POST writes │ │
│  │ - Ping/pong       │  │ - 100ms batching │  │ - Last-Event-ID││
│  └──────────────────┘  └──────────────────┘  └───────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

---

## 6. Data Flow

### 6.1 Outbound (SDK → Claude Code)

```
SDK Host
   │
   │ NDJSON line: {"type": "user", "message": {...}}
   ▼
structuredIO.read()
   │
   │ Parsed SDKUserMessage
   ▼
print.ts main loop
   │
   │ Enqueue command
   ▼
queueManager.enqueue()
   │
   │ Batch compatible commands
   ▼
ask() → API call
   │
   │ Stream response
   ▼
```

### 6.2 Inbound (Claude Code → SDK)

```
ask() response stream
   │
   │ SDKAssistantMessage events
   ▼
ccrClient.writeEvent()
   │
   │ Accumulate text_deltas
   ▼
accumulateStreamEvents()
   │
   │ Coalesced snapshots
   ▼
eventUploader.enqueue()
   │
   │ HTTP POST /sessions/{id}/events
   ▼
SSETransport / HybridTransport
   │
   │ NDJSON lines to stdout
   ▼
SDK Host
```

### 6.3 Permission Prompt Flow

```
Tool call detected
   │
   │ Check permissions
   ▼
requiresAction: 'tool_use'
   │
   │ Send control_request
   ▼
structuredIO.sendRequest({
  subtype: 'can_use_tool',
  tool_use_id: '...',
  tool_name: 'Bash',
  input: {...}
})
   │
   │ Wait for control_response
   ▼
SDK Host receives request
   │
   │ User approves/denies
   ▼
SDK Host sends control_response
   │
   ▼
structuredIO.injectControlResponse({
  response: {
    request_id: '...',
    subtype: 'can_use_tool_response',
    response: { decision: 'approved' }
  }
})
   │
   │ Resolve pending promise
   ▼
Tool execution proceeds
```

---

## 7. Key Patterns

### 7.1 Batching Windows

| Location | Window | Purpose |
|----------|--------|---------|
| `HybridTransport` | 100ms | Batch `stream_event` messages |
| `ccrClient` | 100ms | Accumulate text deltas for coalescing |
| `SerialBatchEventUploader` | Immediate + retry delay | Serialize POSTs, avoid concurrent requests |
| `WorkerStateUploader` | Coalescing | Merge N patches into 1 PUT |

---

### 7.2 Backpressure Control

```typescript
// SerialBatchEventUploader.enqueue()
while (this.pending.length + items.length > this.config.maxQueueSize) {
  await new Promise<void>(resolve => this.backpressureResolvers.push(resolve))
}
```

**Why**: Prevents memory explosion when network is slower than event generation. Callers await until queue has capacity.

### 7.3 Idempotency via UUIDs

All outbound events carry a `uuid` field. The server deduplicates by UUID, enabling:
- Safe retry on network failure
- Message replay on reconnect
- Exactly-once processing semantics

### 7.4 Epoch Management (CCR v2)

```typescript
// ccrClient.ts
private workerEpoch = 0

initialize(): Promise<void> {
  this.workerEpoch = parseInt(process.env.CLAUDE_CODE_WORKER_EPOCH, 10)
  // Register worker with epoch
  // If 409 Conflict: newer epoch exists → exit
}
```

**Purpose**: Prevents zombie workers from stale processes. Only the latest epoch writes succeed.

---

## 8. Integration Points

### 8.1 With `bridge/` Module

| CLI Component | Bridge Consumer | Contract |
|---------------|-----------------|----------|
| `ReplBridgeTransport` | `bridgeMain.ts` | Transport interface for v1/v2 |
| `ccrClient.ts` | `remoteBridgeCore.ts` | JWT refresh, 401 recovery |
| `SerialBatchEventUploader` | `bridgeApi.ts` | Retry pattern with backoff |
| `structuredIO.ts` | `sessionRunner.ts` | Control request for permissions |

### 8.2 With `services/` Module

| CLI Component | Service | Purpose |
|---------------|---------|---------|
| `print.ts` | `MCPConnectionManager` | MCP server lifecycle |
| `print.ts` | `policyLimits` | Token budget enforcement |
| `update.ts` | `autoUpdater` | Version check + install |
| `auth.ts` | `OAuthService` | Token exchange |

### 8.3 With `utils/` Module

| CLI Component | Utility | Purpose |
|---------------|---------|---------|
| `WebSocketTransport` | `CircularBuffer` | Message replay buffer |
| `structuredIO.ts` | `Stream` | Outbound message queue |
| `print.ts` | `sessionState` | Permission mode tracking |
| `WorkerStateUploader` | `sleep` | Exponential backoff delays |

---

## 9. Error Handling

### 9.1 Transport-Level Errors

```typescript
// WebSocketTransport.handleConnectionError()
const PERMANENT_CLOSE_CODES = new Set([
  1002,  // Protocol error
  4001,  // Session expired
  4003,  // Unauthorized (unless headers refreshed)
])
```

**Behavior**:
- Permanent codes: Transition to `closed`, notify callback, no retry
- Transient codes: Exponential backoff with 10-minute budget
- Sleep detection: Reset budget if gap > 60s

### 9.2 CCR Client Errors

```typescript
// ccrClient.ts
private async request(path: string, options: RequestInit): Promise<Response> {
  const response = await this.http.post(...)
  
  if (response.status === 409) {
    // Epoch mismatch — newer worker superseded us
    this.onEpochMismatch()  // Default: process.exit(1)
  }
  
  if (response.status === 401 || response.status === 403) {
    this.consecutiveAuthFailures++
    if (this.consecutiveAuthFailures >= MAX_CONSECUTIVE_AUTH_FAILURES) {
      throw new Error('Too many auth failures')
    }
  } else {
    this.consecutiveAuthFailures = 0
  }
}
```

### 9.3 StructuredIO Errors

```typescript
// structuredIO.ts
private async *read() {
  for await (const block of this.input) {
    try {
      const message = jsonParse(block)
      // Process message...
    } catch (err) {
      // Malformed JSON — log and skip
      logForDebugging(`Failed to parse NDJSON: ${err}`)
    }
  }
}
```

---

## 10. Testing Considerations

### 10.1 Mocking Transports

```typescript
// Test: HybridTransport batches stream events
const transport = new HybridTransport(url, headers)
const written: StdoutMessage[] = []
transport.setOnData(data => written.push(jsonParse(data)))

// Rapid-fire stream events
for (let i = 0; i < 10; i++) {
  await transport.write({ type: 'stream_event', uuid: randomUUID(), ... })
}

// Wait for 100ms flush window
await sleep(150)

// Assert: single batch POST, not 10 individual writes
assert.strictEqual(batchWriteCallCount, 1)
```

### 10.2 Testing Coalescing

```typescript
// Test: accumulateStreamEvents produces full-so-far snapshots
const state = createStreamAccumulator()
const buffer: SDKPartialAssistantMessage[] = [
  { event: { type: 'message_start', message: { id: 'msg_1' } } },
  { event: { type: 'content_block_delta', index: 0, delta: { type: 'text_delta', text: 'Hello' } } },
  { event: { type: 'content_block_delta', index: 0, delta: { type: 'text_delta', text: ' World' } } },
]

const result = accumulateStreamEvents(buffer, state)

// Assert: single snapshot with accumulated text
assert.strictEqual(result.length, 2)  // message_start + 1 coalesced delta
assert.strictEqual(result[1].event.delta.text, 'Hello World')
```

### 10.3 Testing Backpressure

```typescript
// Test: SerialBatchEventUploader applies backpressure
const uploader = new SerialBatchEventUploader({
  maxQueueSize: 10,
  send: async () => false,  // Always fail
})

// Fill queue
const fillPromise = uploader.enqueue(Array(10).fill({ type: 'test' }))

// Next enqueue should await (backpressure)
let backpressureReleased = false
const backpressurePromise = uploader.enqueue({ type: 'test' })
  .then(() => { backpressureReleased = true })

await sleep(50)
assert.strictEqual(backpressureReleased, false)  // Still waiting

// Simulate success
uploader.config.send = async () => true
await fillPromise

assert.strictEqual(backpressureReleased, true)  // Released
```

---

## 11. Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `CLAUDE_CODE_USE_CCR_V2` | Use SSE transport + CCR client | `false` |
| `CLAUDE_CODE_POST_FOR_SESSION_INGRESS_V2` | Use HybridTransport (WS reads + POST writes) | `false` |
| `CLAUDE_CODE_WORKER_EPOCH` | Worker epoch for CCR v2 | Required for CCR |
| `CLAUDE_CODE_REMOTE` | Enable remote session mode | `false` |
| `CLAUDE_CODE_SESSION_ACCESS_TOKEN` | JWT for session auth | — |

---

## 12. Telemetry Events

| Event | Location | Fields |
|-------|----------|--------|
| `tengu_ws_transport_reconnecting` | `WebSocketTransport` | `attempt`, `elapsedMs`, `delayMs` |
| `tengu_ws_transport_reconnected` | `WebSocketTransport` | `attempts`, `downtimeMs` |
| `tengu_ws_transport_closed` | `WebSocketTransport` | `closeCode`, `msSinceLastActivity`, `wasConnected` |
| `tengu_update_check` | `update.ts` | — |
| `tengu_oauth_flow_start` | `auth.ts` | `loginWithClaudeAi` |
| `tengu_plugin_list_command` | `plugins.ts` | — |
| `tengu_marketplace_added` | `plugins.ts` | `source_type` |

---

## 13. Appendix: Util Handlers (`handlers/util.tsx`)

Miscellaneous subcommand handlers extracted from `main.tsx` for lazy loading (109 lines):

#### 13.1.1 Exports

```typescript
// setup-token — OAuth token setup for Claude account
export async function setupTokenHandler(root: Root): Promise<void>

// doctor — Diagnostic tool with plugin management
export async function doctorHandler(root: Root): Promise<void>

// install — Install Claude Code to system PATH
export async function installHandler(
  target: string | undefined,
  options: { force?: boolean }
): Promise<void>
```

#### 13.1.2 Doctor Handler Architecture

```typescript
// DoctorWithPlugins wrapper — lazy loads Doctor screen + manages plugins
const DoctorLazy = React.lazy(() =>
  import('../../screens/Doctor.js').then(m => ({ default: m.Doctor }))
)

function DoctorWithPlugins({ onDone }: { onDone: () => void }): React.ReactNode {
  useManagePlugins()  // Fetch, validate, install plugins on mount
  return (
    <React.Suspense fallback={null}>
      <DoctorLazy onDone={onDone} />
    </React.Suspense>
  )
}

export async function doctorHandler(root: Root): Promise<void> {
  logEvent('tengu_doctor_command', {})
  await new Promise<void>(resolve => {
    root.render(
      <AppStateProvider>
        <KeybindingSetup>
          <MCPConnectionManager dynamicMcpConfig={undefined} isStrictMcpConfig={false}>
            <DoctorWithPlugins onDone={() => { void resolve() }} />
          </MCPConnectionManager>
        </KeybindingSetup>
      </AppStateProvider>
    )
  })
  root.unmount()
  process.exit(0)
}
```

#### 13.1.3 Setup Token Flow

```typescript
export async function setupTokenHandler(root: Root): Promise<void> {
  logEvent('tengu_setup_token_command', {})
  
  const showAuthWarning = !isAnthropicAuthEnabled()
  const { ConsoleOAuthFlow } = await import('../../components/ConsoleOAuthFlow.js')
  
  await new Promise<void>(resolve => {
    root.render(
      <AppStateProvider onChangeAppState={onChangeAppState}>
        <KeybindingSetup>
          <Box flexDirection="column" gap={1}>
            <WelcomeV2 />
            {showAuthWarning && (
              <Box flexDirection="column">
                <Text color="warning">
                  Warning: You already have authentication configured via
                  environment variable or API key helper.
                </Text>
                <Text color="warning">
                  The setup-token command will create a new OAuth token which
                  you can use instead.
                </Text>
              </Box>
            )}
            <ConsoleOAuthFlow
              onDone={() => { void resolve() }}
              mode="setup-token"
              startingMessage="This will guide you through long-lived (1-year) auth token setup..."
            />
          </Box>
        </KeybindingSetup>
      </AppStateProvider>
    )
  })
  root.unmount()
  process.exit(0)
}
```

---

## 14. Appendix: MCP Handlers (`handlers/mcp.tsx`)

The MCP handlers file (361 lines) implements all `claude mcp *` subcommands:

#### 14.1.1 Exports

```typescript
// Server health check (concurrent connection testing)
async function checkMcpServerHealth(
  name: string,
  server: ScopedMcpServerConfig
): Promise<string>
// Returns: '✓ Connected', '! Needs authentication', or '✗ Failed to connect'

// mcp serve — Start MCP server for headless operation
export async function mcpServeHandler({
  debug,
  verbose
}: {
  debug?: boolean;
  verbose?: boolean;
}): Promise<void>

// mcp remove — Remove server by name (handles multi-scope ambiguity)
export async function mcpRemoveHandler(
  name: string,
  options: { scope?: string }
): Promise<void>

// mcp list — List all configured servers with health status
export async function mcpListHandler(): Promise<void>

// mcp get — Get detailed server config and status
export async function mcpGetHandler(name: string): Promise<void>

// mcp add-json — Add server from JSON config (supports OAuth secrets)
export async function mcpAddJsonHandler(
  name: string,
  json: string,
  options: {
    scope?: string;
    clientSecret?: true;
  }
): Promise<void>

// mcp import — Import servers from Desktop config
export async function mcpImportHandler(options: {
  file?: string;
  scope?: string;
}): Promise<void>
```

#### 14.1.2 Key Patterns

**Multi-Scope Resolution** (`mcpRemoveHandler`):
```typescript
// Server exists in multiple scopes → require explicit scope flag
const scopes: Array<Exclude<ConfigScope, 'dynamic'>> = []
if (projectConfig.mcpServers?.[name]) scopes.push('local')
if (mcpJsonExists) scopes.push('project')
if (globalConfig.mcpServers?.[name]) scopes.push('user')

if (scopes.length > 1) {
  // Ambiguous — show all locations and require -s flag
  process.stderr.write(`MCP server "${name}" exists in multiple scopes:\n`)
  scopes.forEach(scope => {
    process.stderr.write(`  - ${getScopeLabel(scope)}\n`)
  })
  cliError()  // Exit with error
}
```

**Secure Storage Cleanup**:
```typescript
// Before removing config, clean up OAuth tokens from secure storage
const cleanupSecureStorage = () => {
  if (serverBeforeRemoval && (serverBeforeRemoval.type === 'sse' || serverBeforeRemoval.type === 'http')) {
    clearServerTokensFromLocalStorage(name, serverBeforeRemoval)
    clearMcpClientConfig(name, serverBeforeRemoval)
  }
}
```

**Concurrent Health Checks** (`mcpListHandler`):
```typescript
// Check all servers concurrently with bounded concurrency
const results = await pMap(entries, async ([name, server]) => ({
  name,
  server,
  status: await checkMcpServerHealth(name, server)
}), {
  concurrency: getMcpServerConnectionBatchSize()  // Default: 10
})
```

**OAuth Secret Handling** (`mcpAddJsonHandler`):
```typescript
// Read secret BEFORE writing config — cancellation doesn't leave partial state
const needsSecret = /* detect OAuth config */
const clientSecret = needsSecret ? await readClientSecret() : undefined
await addMcpConfig(name, parsedJson, scope)  // Write config first
if (clientSecret) {
  saveMcpClientSecret(name, { url: parsedJson.url, clientSecret })
}
```

### 14.1.3 Server Types Supported

| Type | Fields | Description |
|------|--------|-------------|
| `stdio` | `command`, `args`, `env`, `cwd` | Spawn subprocess with stdio transport |
| `sse` | `url`, `headers`, `oauth` | Server-Sent Events (MCP spec) |
| `http` | `url`, `headers`, `oauth` | HTTP POST transport (MCP spec) |
| `claudeai-proxy` | `url` | Proxy through claude.ai backend |

---

**File Count:** 19 TypeScript/TSX files  
**Total Lines:** 11,953 lines (excluding tests)

---

## 15. Summary

The `cli/` module is the **SDK runtime layer** for Claude Code, enabling:

1. **Headless operation** via NDJSON stdio protocol
2. **Remote session connectivity** via WebSocket/SSE + HTTP POST
3. **Efficient event streaming** via batching and coalescing
4. **Resilient connections** via auto-reconnect with exponential backoff
5. **Structured permission prompts** via control_request/response RPC
6. **MCP server management** via serve/remove/list/get/add-json/import commands
7. **Interactive setup flows** via setup-token/doctor/install handlers

The module mirrors patterns from `bridge/` (transport abstraction, serial uploader, JWT refresh) but targets SDK/embedded use cases rather than interactive REPL sessions.

### Documentation Changelog

| Section | Content Added |
|---------|---------------|
| File Inventory | Accurate line counts for all 19 files |
| MCP Handlers (14) | Full coverage of `mcp.tsx` (361 lines) |
| Util Handlers (13) | Full coverage of `util.tsx` (109 lines) |
| Batching Table (7.1) | Added WorkerStateUploader coalescing pattern |
| Summary (15) | Extended with MCP and Util handler coverage |

---

**Last Updated:** 2026-04-07  
**Status:** Complete — all 19 files inventoried, core files analyzed line-by-line
