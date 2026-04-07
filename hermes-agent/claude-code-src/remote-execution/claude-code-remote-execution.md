# Claude Code Remote Execution — Deep Dive Exploration

**Project:** Hermes Agent Deep Dive  
**Subject:** Claude Code Remote Execution for Mobile & Desktop  
**Created:** 2026-04-07  
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/`

---

## Executive Summary

Claude Code remote execution enables users to run Claude Code sessions on remote infrastructure (CCR — Claude Code Remote) while controlling them from any device (mobile app, desktop CLI, web UI). This document provides a complete start-to-end exploration of:

1. **How devices connect** — WebSocket protocols, authentication flows
2. **How commands are transmitted securely** — Token-based auth, work secrets, session ingress
3. **How permissions are handled** — Cross-device permission prompts, control request/response protocol
4. **Network architecture** — CCR containers, bridge environments, session management

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Claude Code Remote Execution Architecture            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐                  │
│  │   Mobile     │    │   Desktop    │    │     Web      │                  │
│  │     App      │    │     CLI      │    │     UI       │                  │
│  └──────┬───────┘    └──────┬───────┘    └──────┬───────┘                  │
│         │                   │                   │                          │
│         └───────────────────┼───────────────────┘                          │
│                             │                                              │
│                             ▼                                              │
│              ┌──────────────────────────────┐                             │
│              │   Anthropic API Gateway      │                             │
│              │   (wss://api.anthropic.com)  │                             │
│              └──────────────┬───────────────┘                             │
│                             │                                              │
│         ┌───────────────────┼───────────────────┐                         │
│         │                   │                   │                         │
│         ▼                   ▼                   ▼                         │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐                     │
│  │  Sessions   │   │  Environments│   │    CCR      │                     │
│  │     API     │   │     API      │   │  Containers │                     │
│  │  /v1/       │   │  /v1/       │   │  (Workers)  │                     │
│  │  sessions   │   │  environments│   │             │                     │
│  └──────┬──────┘   └──────┬───────┘   └──────┬──────┘                     │
│         │                 │                  │                             │
│         └─────────────────┼──────────────────┘                             │
│                           │                                                │
│                           ▼                                                │
│              ┌────────────────────────────┐                               │
│              │   Session Ingress Layer    │                               │
│              │   (JWT + WebSocket)        │                               │
│              └────────────────────────────┘                               │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Key Components

| Component | Purpose |
|-----------|---------|
| **Mobile App / Desktop CLI / Web UI** | Client devices that initiate and control remote sessions |
| **API Gateway** | Central routing for all Claude Code API requests |
| **Sessions API** | Session lifecycle management (create, fetch, archive) |
| **Environments API** | Bridge environment registration and work polling |
| **CCR Containers** | Remote execution workers running Claude Code |
| **Session Ingress** | Real-time WebSocket layer for message streaming |

---

## 1. Connection Flow — Start to End

### 1.1 Session Creation (Mobile/Desktop)

When a user starts a remote session from their mobile app or desktop:

```
User Device (Mobile/Desktop)
       │
       │ POST /v1/sessions
       │ Authorization: Bearer <OAuth Token>
       │ {
       │   title: "My Session",
       │   environment_id: "...",
       │   session_context: { ... }
       │ }
       ▼
API Gateway
       │
       │ Validate OAuth token
       │ Check org membership
       ▼
Sessions API
       │
       │ Create session record
       │ Allocate CCR container
       ▼
CCR Container (Worker)
       │
       │ Register via POST /worker/register
       │ Receive worker_epoch
       ▼
Session Created
       │
       │ Response: { id, session_status, environment_id }
       ▼
User Device
```

**Key File:** `bridge/createSession.ts`

```typescript
export async function createBridgeSession({
  environmentId,
  title,
  events,
  gitRepoUrl,
  branch,
  signal,
  baseUrl: baseUrlOverride,
  getAccessToken,
  permissionMode,
}: {
  environmentId: string
  title?: string
  events: SessionEvent[]
  gitRepoUrl: string | null
  branch: string
  signal: AbortSignal
  getAccessToken?: () => string | undefined
  permissionMode?: string
}): Promise<string | null> {
  const accessToken = getAccessToken?.() ?? getClaudeAIOAuthTokens()?.accessToken
  const orgUUID = await getOrganizationUUID()
  
  const headers = {
    ...getOAuthHeaders(accessToken),
    'anthropic-beta': 'ccr-byoc-2025-07-29',
    'x-organization-uuid': orgUUID,
  }
  
  const response = await axios.post(url, requestBody, { headers })
  return response.data.id
}
```

### 1.2 WebSocket Connection

After session creation, the client establishes a WebSocket for real-time communication:

```
User Device
       │
       │ WebSocket: wss://api.anthropic.com/v1/sessions/ws/{sessionId}/subscribe
       │ Headers:
       │   Authorization: Bearer <OAuth Token>
       │   anthropic-version: 2023-06-01
       ▼
Sessions WebSocket
       │
       │ Validate token
       │ Subscribe to session event stream
       ▼
Connection Established
       │
       │ ← SDK Messages (assistant, system, tool_progress)
       │ → Control Responses (permission decisions)
```

**Key File:** `remote/SessionsWebSocket.ts`

```typescript
async connect(): Promise<void> {
  const baseUrl = getOauthConfig().BASE_API_URL.replace('https://', 'wss://')
  const url = `${baseUrl}/v1/sessions/ws/${this.sessionId}/subscribe?organization_uuid=${this.orgUuid}`
  
  const accessToken = this.getAccessToken()
  const headers = {
    Authorization: `Bearer ${accessToken}`,
    'anthropic-version': '2023-06-01',
  }
  
  // Bun runtime (native WebSocket with headers support)
  if (typeof Bun !== 'undefined') {
    const ws = new globalThis.WebSocket(url, {
      headers,
      proxy: getWebSocketProxyUrl(url),
      tls: getWebSocketTLSOptions() || undefined,
    } as unknown as string[])
    this.ws = ws
  } else {
    // Node.js runtime (ws package)
    const { default: WS } = await import('ws')
    const ws = new WS(url, {
      headers,
      agent: getWebSocketProxyAgent(url),
      ...getWebSocketTLSOptions(),
    })
    this.ws = ws
  }
}
```

### 1.3 Dual Runtime Support

Claude Code runs on both Bun (native) and Node.js (ws package):

| Runtime | WebSocket | Headers | Proxy |
|---------|-----------|---------|-------|
| **Bun** | `globalThis.WebSocket` | Native support | `proxy` option |
| **Node.js** | `ws` package | Constructor option | `agent` option |

---

## 2. Authentication & Security

### 2.1 Token Types

Claude Code uses multiple token types for different purposes:

| Token Type | Prefix | Purpose | Lifetime |
|------------|--------|---------|----------|
| **OAuth Access Token** | N/A | User authentication | ~1 hour |
| **Session Ingress Token** | `sk-ant-si-` | Session-specific auth | Session lifetime |
| **Work Secret** | N/A | Bridge environment auth | Work item lifetime |
| **Session Key** | `sk-ant-sid-` | Cookie-based session | Browser session |

### 2.2 Session Ingress Token Flow

```
User OAuth Token
       │
       │ POST /v1/sessions/{id}/worker/register
       ▼
Server
       │
       │ Generate JWT with session_id claim
       │ Encode as sk-ant-si-{jwt}
       ▼
Work Secret Response
       │
       │ {
       │   version: 1,
       │   session_ingress_token: "sk-ant-si-{jwt}",
       │   api_base_url: "https://..."
       │ }
       ▼
Client stores token
```

**Key File:** `bridge/workSecret.ts`

```typescript
/** Decode a base64url-encoded work secret and validate its version. */
export function decodeWorkSecret(secret: string): WorkSecret {
  const json = Buffer.from(secret, 'base64url').toString('utf-8')
  const parsed: unknown = jsonParse(json)
  if (!parsed || typeof parsed !== 'object' || !('version' in parsed) || parsed.version !== 1) {
    throw new Error(`Unsupported work secret version: ${...}`)
  }
  // Returns: { session_ingress_token, api_base_url, sources, auth, ... }
  return parsed as WorkSecret
}
```

### 2.3 Token Refresh Scheduler

Tokens expire — the system proactively refreshes them before expiry:

```typescript
// From remote/SessionsWebSocket.ts
const TOKEN_REFRESH_BUFFER_MS = 5 * 60 * 1000  // 5 minutes before expiry
const FALLBACK_REFRESH_INTERVAL_MS = 30 * 60 * 1000  // 30 minutes
const MAX_REFRESH_FAILURES = 3

function schedule(sessionId: string, token: string): void {
  const expiry = decodeJwtExpiry(token)  // Extract 'exp' claim from JWT
  const delayMs = expiry * 1000 - Date.now() - TOKEN_REFRESH_BUFFER_MS
  
  const timer = setTimeout(doRefresh, delayMs, sessionId, gen)
  timers.set(sessionId, timer)
}

async function doRefresh(sessionId: string, gen: number): Promise<void> {
  const oauthToken = await getAccessToken()
  onRefresh(sessionId, oauthToken)  // Inject fresh token into transport
}
```

**Token Refresh Flow:**
1. Decode JWT expiry claim (`exp`)
2. Schedule refresh 5 minutes before expiry
3. Fetch fresh OAuth token
4. Inject into transport (update env var or WebSocket reconnect)
5. Schedule follow-up refresh (30-minute fallback)

### 2.4 Authentication Header Strategies

Different endpoints use different auth methods:

```typescript
// From utils/sessionIngressAuth.ts
export function getSessionIngressAuthHeaders(): Record<string, string> {
  const token = getSessionIngressAuthToken()
  if (!token) return {}
  
  // Session keys (sk-ant-sid-) use Cookie auth
  if (token.startsWith('sk-ant-sid')) {
    const headers: Record<string, string> = {
      Cookie: `sessionKey=${token}`,
    }
    const orgUuid = process.env.CLAUDE_CODE_ORGANIZATION_UUID
    if (orgUuid) {
      headers['X-Organization-Uuid'] = orgUuid
    }
    return headers
  }
  
  // JWTs use Bearer auth
  return { Authorization: `Bearer ${token}` }
}
```

---

## 3. Work Secret & Bridge Registration

### 3.1 Bridge Environment Registration

Bridge environments (desktop machines running `claude remote-control`) register with the server:

```
Desktop Bridge
       │
       │ POST /v1/environments
       │ {
       │   machine_name: "...",
       │   git_repo_url: "...",
       │   branch: "...",
       │   metadata: { worker_type: "claude_code" }
       │ }
       ▼
Environments API
       │
       │ Create environment record
       │ Generate environment_secret
       ▼
Response
       │
       │ {
       │   environment_id: "...",
       │   environment_secret: "..."
       │ }
```

**Key Types:** `bridge/types.ts`

```typescript
export type BridgeConfig = {
  dir: string
  machineName: string
  branch: string
  gitRepoUrl: string | null
  maxSessions: number
  spawnMode: SpawnMode  // 'single-session' | 'worktree' | 'same-dir'
  bridgeId: string  // Client-generated UUID
  workerType: string  // Sent as metadata.worker_type
  environmentId: string
  apiBaseUrl: string
  sessionIngressUrl: string
  sessionTimeoutMs?: number
}

export type WorkSecret = {
  version: number
  session_ingress_token: string
  api_base_url: string
  sources: Array<{ type: string; git_info?: {...} }>
  auth: Array<{ type: string; token: string }>
  use_code_sessions?: boolean  // Server-driven CCR v2 selector
}
```

### 3.2 Work Polling Loop

Once registered, the bridge continuously polls for work:

```typescript
// Poll loop (simplified from replBridge.ts)
async function pollForWork(): Promise<void> {
  while (!aborted) {
    const work = await apiClient.pollForWork(
      environmentId,
      environmentSecret,
      abortSignal,
    )
    
    if (work) {
      // Decode work secret
      const secret = decodeWorkSecret(work.secret)
      
      // Build session URL
      const sessionUrl = buildCCRv2SdkUrl(secret.api_base_url, work.id)
      
      // Spawn session
      await spawnSession({
        sessionId: work.id,
        sdkUrl: sessionUrl,
        accessToken: secret.session_ingress_token,
        useCcrV2: secret.use_code_sessions,
      })
    }
    
    // Heartbeat — extend lease
    await apiClient.heartbeatWork(environmentId, work?.id, token)
    
    // Wait before next poll
    await sleep(getPollIntervalConfig().connected * 1000)
  }
}
```

**Polling Intervals (GrowthBook configurable):**
- **Idle:** 60 seconds
- **Connected:** 5 seconds
- **Reconnecting:** Exponential backoff (2s, 4s, 8s, ...)

---

## 4. Message Transmission Protocol

### 4.1 Sending User Messages (Client → CCR)

```
User Device
       │
       │ POST /v1/sessions/{id}/events
       │ {
       │   events: [{
       │     uuid: "...",
       │     type: 'user',
       │     message: { role: 'user', content: "..." }
       │   }]
       │ }
       ▼
Session Events API
       │
       │ Queue event for CCR worker
       ▼
CCR Worker (SSE Stream)
       │
       │ ← SSE: { event_id, data: {...} }
       ▼
Claude Code Process
```

**Key File:** `utils/teleport/api.ts`

```typescript
export async function sendEventToRemoteSession(
  sessionId: string,
  messageContent: RemoteMessageContent,
  opts?: { uuid?: string },
): Promise<boolean> {
  const { accessToken, orgUUID } = await prepareApiRequest()
  
  const url = `${getOauthConfig().BASE_API_URL}/v1/sessions/${sessionId}/events`
  const headers = {
    ...getOAuthHeaders(accessToken),
    'anthropic-beta': 'ccr-byoc-2025-07-29',
    'x-organization-uuid': orgUUID,
  }
  
  const userEvent = {
    uuid: opts?.uuid ?? randomUUID(),
    session_id: sessionId,
    type: 'user',
    parent_tool_use_id: null,
    message: {
      role: 'user',
      content: messageContent,
    },
  }
  
  const response = await axios.post(url, { events: [userEvent] }, {
    headers,
    timeout: 30000,  // Cold-start containers may take time
  })
  
  return response.status === 200 || response.status === 201
}
```

### 4.2 Receiving Messages (CCR → Client)

```
CCR Worker
       │
       │ SSE Stream: wss://.../worker/events/stream
       │ Last-Event-ID: {sequenceNum} (resume from checkpoint)
       ▼
Client SSE Transport
       │
       │ onEvent(event):
       │   - Report 'received' delivery
       │   - Report 'processed' delivery
       │   - Forward to callback
       ▼
Message Handler
```

**SSE Transport:** `cli/transports/SSETransport.ts`

```typescript
// SSE connection with resume support
async connect(): Promise<void> {
  const headers: Record<string, string> = {
    ...this.getAuthHeaders?.(),
  }
  
  // Resume from last sequence number
  if (this.initialSequenceNum !== undefined) {
    headers['Last-Event-ID'] = String(this.initialSequenceNum)
  }
  
  const response = await fetch(this.url, { headers })
  const reader = response.body?.getReader()
  
  while (true) {
    const { done, value } = await reader!.read()
    if (done) break
    
    const text = decoder.decode(value)
    const events = parseSSE(text)
    
    for (const event of events) {
      this.sequenceNum = event.id  // Track high-water mark
      this.onData?.(event.data)
    }
  }
}
```

### 4.3 NDJSON Message Format

Messages are exchanged as newline-delimited JSON:

```json
{"type":"assistant","message":{"id":"msg_123","content":[{"type":"text","text":"Hello!"}]}}
{"type":"control_request","request_id":"req_456","request":{"subtype":"can_use_tool","tool_name":"Bash"}}
{"type":"tool_progress","tool_name":"Bash","elapsed_time_seconds":5}
{"type":"system","subtype":"status","status":"compacting"}
```

---

## 5. Permission Request/Response Protocol

### 5.1 Permission Request Flow

When CCR wants to execute a tool requiring permission:

```
CCR Worker
       │
       │ control_request (subtype: can_use_tool)
       │ {
       │   request_id: "...",
       │   request: {
       │     subtype: "can_use_tool",
       │     tool_name: "Bash",
       │     tool_use_id: "...",
       │     input: { command: "..." }
       │   }
       │ }
       ▼
WebSocket
       ▼
RemoteSessionManager.handleControlRequest()
       │
       │ Store in pendingPermissionRequests
       │ callbacks.onPermissionRequest()
       ▼
UI: Show Permission Prompt
       │
       │ User approves/denies
       ▼
respondToPermissionRequest()
       │
       │ control_response
       │ {
       │   type: "control_response",
       │   response: {
       │     subtype: "success",
       │     request_id: "...",
       │     response: {
       │       behavior: "allow",
       │       updatedInput: { ... }  // Or { message: "...", behavior: "deny" }
       │     }
       │   }
       │ }
       ▼
WebSocket
       ▼
CCR Worker
```

**Key File:** `remote/RemoteSessionManager.ts`

```typescript
private handleControlRequest(request: SDKControlRequest): void {
  const { request_id, request: inner } = request
  
  if (inner.subtype === 'can_use_tool') {
    // Track pending request
    this.pendingPermissionRequests.set(request_id, inner)
    
    // Notify UI
    this.callbacks.onPermissionRequest(inner, request_id)
  } else {
    // Unknown subtype — send error response
    const response: SDKControlResponse = {
      type: 'control_response',
      response: {
        subtype: 'error',
        request_id,
        error: `Unsupported control request subtype: ${inner.subtype}`,
      },
    }
    this.websocket?.sendControlResponse(response)
  }
}

respondToPermissionRequest(requestId: string, result: RemotePermissionResponse): void {
  const pendingRequest = this.pendingPermissionRequests.get(requestId)
  if (!pendingRequest) return
  
  this.pendingPermissionRequests.delete(requestId)
  
  const response: SDKControlResponse = {
    type: 'control_response',
    response: {
      subtype: 'success',
      request_id: requestId,
      response: {
        behavior: result.behavior,
        ...(result.behavior === 'allow'
          ? { updatedInput: result.updatedInput }
          : { message: result.message }),
      },
    },
  }
  
  this.websocket?.sendControlResponse(response)
}
```

### 5.2 Synthetic Messages for Remote Permissions

For remote sessions, the local CLI doesn't have the actual assistant message — it creates a synthetic one:

```typescript
// From remote/remotePermissionBridge.ts
export function createSyntheticAssistantMessage(
  request: SDKControlPermissionRequest,
  requestId: string,
): AssistantMessage {
  return {
    type: 'assistant',
    uuid: randomUUID(),
    message: {
      id: `remote-${requestId}`,
      type: 'message',
      role: 'assistant',
      content: [
        {
          type: 'tool_use',
          id: request.tool_use_id,
          name: request.tool_name,
          input: request.input,
        },
      ],
      // ... placeholder values for other fields
    } as AssistantMessage['message'],
    timestamp: new Date().toISOString(),
  }
}

export function createToolStub(toolName: string): Tool {
  return {
    name: toolName,
    inputSchema: {} as Tool['inputSchema'],
    isEnabled: () => true,
    userFacingName: () => toolName,
    renderToolUseMessage: (input) => {
      // Render minimal description from input
      return Object.entries(input).slice(0, 3).map(...).join(', ')
    },
    call: async () => ({ data: '' }),
    isReadOnly: () => false,
    isMcp: false,
    needsPermissions: () => true,
  } as unknown as Tool
}
```

---

## 6. CCR v1 vs CCR v2 Transport

Claude Code supports two transport modes for remote execution:

### 6.1 CCR v1 (Session Ingress)

```
┌─────────────────────────────────────────────────────────┐
│                   CCR v1 Architecture                    │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  Client ──WebSocket──► Session Ingress ◄──WebSocket── CCR│
│                       wss://.../v1/                      │
│                       /session_ingress/                  │
│                       /ws/{sessionId}                    │
│                                                          │
│  Client ──HTTP POST──► Session Ingress ◄──Stdin── CCR   │
│                       /v1/sessions/{id}/events          │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

**Characteristics:**
- WebSocket for inbound messages (subscribe)
- HTTP POST for outbound messages (events)
- OAuth-based authentication
- Used by REPL bridge (`replBridge.ts`)

### 6.2 CCR v2 (Worker API)

```
┌─────────────────────────────────────────────────────────┐
│                   CCR v2 Architecture                    │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  Client ──SSE──────► CCR v2 Worker API ◄──Heartbeat── CCR│
│                       wss://.../v2/                      │
│                       /worker/events/stream              │
│                                                          │
│  Client ──HTTP─────► CCR v2 Worker API ◄──Events──── CCR │
│                       /v1/code/sessions/{id}/            │
│                       /worker/events                     │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

**Characteristics:**
- SSE (Server-Sent Events) for inbound
- HTTP POST for outbound (CCRClient)
- JWT-based authentication (session_ingress_token)
- Used by daemon/Agent SDK (`daemonBridge.ts`)
- Requires worker registration (`POST /worker/register`)

### 6.3 Transport Adapter Pattern

```typescript
// From bridge/replBridgeTransport.ts
export type ReplBridgeTransport = {
  write(message: StdoutMessage): Promise<void>
  writeBatch(messages: StdoutMessage[]): Promise<void>
  close(): void
  isConnectedStatus(): boolean
  setOnData(callback: (data: string) => void): void
  setOnClose(callback: () => void): void
  setOnConnect(callback: () => void): void
  connect(): void
  getLastSequenceNum(): number
  reportState(state: SessionState): void
  flush(): Promise<void>
}

// v1 adapter: Wrap HybridTransport
export function createV1ReplTransport(hybrid: HybridTransport): ReplBridgeTransport {
  return {
    write: msg => hybrid.write(msg),
    writeBatch: msgs => hybrid.writeBatch(msgs),
    close: () => hybrid.close(),
    isConnectedStatus: () => hybrid.isConnectedStatus(),
    // ... adapter methods
  }
}

// v2 adapter: Wrap SSETransport + CCRClient
export async function createV2ReplTransport(opts: {...}): Promise<ReplBridgeTransport> {
  const sse = new SSETransport(sseUrl, ..., initialSequenceNum, getAuthHeaders)
  const ccr = new CCRClient(sse, sessionUrl, { getAuthHeaders, ... })
  
  return {
    write: msg => ccr.writeEvent(msg),
    writeBatch: async msgs => {
      for (const m of msgs) await ccr.writeEvent(m)
    },
    close: () => { ccr.close(); sse.close() },
    // ... adapter methods
  }
}
```

---

## 7. Reconnection & Resilience

### 7.1 WebSocket Reconnection Logic

```typescript
// From remote/SessionsWebSocket.ts
const RECONNECT_DELAY_MS = 2000
const MAX_RECONNECT_ATTEMPTS = 5
const MAX_SESSION_NOT_FOUND_RETRIES = 3
const PERMANENT_CLOSE_CODES = new Set([4003])  // unauthorized

private handleClose(closeCode: number): void {
  // Permanent close codes
  if (PERMANENT_CLOSE_CODES.has(closeCode)) {
    logForDebugging(`Permanent close code ${closeCode}`)
    this.callbacks.onClose?.()
    return
  }
  
  // 4001 (session not found) — transient during compaction
  if (closeCode === 4001) {
    this.sessionNotFoundRetries++
    if (this.sessionNotFoundRetries > MAX_SESSION_NOT_FOUND_RETRIES) {
      this.callbacks.onClose?.()
      return
    }
    this.scheduleReconnect(RECONNECT_DELAY_MS * this.sessionNotFoundRetries, ...)
    return
  }
  
  // General reconnection
  if (this.reconnectAttempts < MAX_RECONNECT_ATTEMPTS) {
    this.reconnectAttempts++
    this.scheduleReconnect(RECONNECT_DELAY_MS, ...)
  } else {
    this.callbacks.onClose?.()
  }
}
```

### 7.2 SSE Sequence Number Resume

When reconnecting, the client resumes from the last received sequence number:

```typescript
// From cli/transports/SSETransport.ts
class SSETransport {
  private sequenceNum: number = 0
  
  async connect(): Promise<void> {
    const headers: Record<string, string> = {}
    
    // Resume from checkpoint
    if (this.initialSequenceNum !== undefined) {
      headers['Last-Event-ID'] = String(this.initialSequenceNum)
    }
    
    const response = await fetch(this.url, { headers })
    // ... stream processing
    
    for (const event of events) {
      this.sequenceNum = event.id  // Track high-water mark
    }
  }
  
  getLastSequenceNum(): number {
    return this.sequenceNum
  }
}
```

**Transport Swap Flow:**
1. Old transport closes
2. Capture `lastSequenceNum` from old transport
3. Create new transport with `initialSequenceNum`
4. New transport connects with `Last-Event-ID` header
5. Server resumes from checkpoint (no full replay)

### 7.3 Epoch Mismatch Recovery (CCR v2)

CCR v2 uses epochs to detect stale workers:

```typescript
// From bridge/replBridgeTransport.ts (CCRClient onEpochMismatch)
onEpochMismatch: () => {
  logForDebugging('[bridge:repl] CCR v2: epoch superseded (409)')
  
  try {
    ccr.close()
    sse.close()
    onCloseCb?.(4090)  // Signal epoch mismatch
  } catch (closeErr) {
    logForDebugging(`Error during epoch-mismatch cleanup: ${errorMessage(closeErr)}`)
  }
  
  throw new Error('epoch superseded')  // Unwind caller
}
```

**Epoch Flow:**
1. Worker registers, receives `worker_epoch`
2. Worker includes epoch in all requests
3. Server may re-dispatch work with higher epoch
4. Worker detects 409 Conflict (epoch superseded)
5. Worker closes, poll loop picks up new dispatch

---

## 8. Security Hardening

### 8.1 Token Storage

Tokens are stored securely using multiple strategies:

```typescript
// From utils/sessionIngressAuth.ts
export function getSessionIngressAuthToken(): string | null {
  // Priority 1: Environment variable (spawn-time injection)
  const envToken = process.env.CLAUDE_CODE_SESSION_ACCESS_TOKEN
  if (envToken) return envToken
  
  // Priority 2: File descriptor (inherited from parent)
  const fdEnv = process.env.CLAUDE_CODE_WEBSOCKET_AUTH_FILE_DESCRIPTOR
  if (fdEnv) {
    const fd = parseInt(fdEnv, 10)
    const fdPath = process.platform === 'darwin' ? `/dev/fd/${fd}` : `/proc/self/fd/${fd}`
    const token = fsOps.readFileSync(fdPath, 'utf8').trim()
    return token
  }
  
  // Priority 3: Well-known file (fallback for subprocesses)
  const path = process.env.CLAUDE_SESSION_INGRESS_TOKEN_FILE ?? CCR_SESSION_INGRESS_TOKEN_PATH
  const fromFile = readTokenFromWellKnownFile(path, 'session ingress token')
  return fromFile
}
```

### 8.2 prctl Security (Upstream Proxy)

The upstream proxy uses prctl for security hardening:

```typescript
// From upstreamproxy/upstreamproxy.ts (analyzed in upstreamproxy/exploration.md)
import { prctl } from 'prctl'

// Make the process non-dumpable (prevents ptrace attacks)
prctl(PR_SET_DUMPABLE, 0)
```

### 8.3 NO_PROXY List

The upstream proxy bypasses certain domains:

```typescript
const NO_PROXY_LIST = [
  'localhost',
  '127.0.0.1',
  '::1',
  'internal.anthropic.com',  // Internal services
  'metadata.google.internal', // GCP metadata
  // ... more entries with rationale
]
```

---

## 9. Bridge Modes

### 9.1 Spawn Modes

```typescript
export type SpawnMode = 'single-session' | 'worktree' | 'same-dir'

/**
 * How `claude remote-control` chooses session working directories:
 * - `single-session`: One session in cwd, bridge tears down when it ends
 * - `worktree`: Persistent server, every session gets isolated git worktree
 * - `same-dir`: Persistent server, every session shares cwd (can stomp)
 */
```

### 9.2 Outbound-Only Mode

For mirror-mode attachments that forward events but never receive:

```typescript
// From bridge/replBridgeTransport.ts
export async function createV2ReplTransport(opts: {
  outboundOnly?: boolean  // Skip SSE read stream
  ...
}): Promise<ReplBridgeTransport> {
  if (!opts.outboundOnly) {
    void sse.connect()  // Fire-and-forget read stream
  }
  // CCRClient write path always initialized
  void ccr.initialize(epoch).then(...)
}
```

**Use Cases:**
- SDK attachments that mirror events
- Web viewers (no local execution)
- Read-only session monitoring

---

## 10. Integration Points

| Module | Integration |
|--------|-------------|
| `utils/teleport/api.ts` | HTTP POST to session events API |
| `constants/oauth.ts` | OAuth configuration, API URLs |
| `utils/mtls.ts` | mTLS certificates for WebSocket |
| `utils/proxy.ts` | Corporate proxy support |
| `services/analytics/` | Event logging (bridge_token_refreshed, tengu_bridge_message_received) |
| `utils/permissions/PermissionMode.ts` | Permission mode handling |

---

## 11. Data Flow Summary

### 11.1 Complete Message Journey

```
User (Mobile App)
       │
       │ 1. Type message
       ▼
Mobile App
       │
       │ 2. POST /v1/sessions/{id}/events
       │    Authorization: Bearer {oauth}
       ▼
Sessions API
       │
       │ 3. Queue event for CCR worker
       ▼
CCR Worker (SSE)
       │
       │ 4. SSE: { event_id, data }
       ▼
Claude Code Process
       │
       │ 5. Process message, generate response
       │ 6. Tool use: Bash "ls -la"
       ▼
CCR Worker
       │
       │ 7. control_request (can_use_tool)
       ▼
WebSocket
       ▼
Mobile App
       │
       │ 8. Show permission prompt
       │ 9. User approves
       │
       │ 10. control_response (allow)
       ▼
WebSocket
       ▼
CCR Worker
       │
       │ 11. Execute tool
       │ 12. Stream output via SSE
       ▼
Mobile App
       │
       │ 13. Display output
```

### 11.2 Permission Boundary Crossing

```
┌─────────────────────┐         ┌─────────────────────┐
│   Local Device      │         │   Remote CCR        │
│   (Mobile/Desktop)  │         │   (Container)       │
│                     │         │                     │
│  ┌───────────────┐  │         │  ┌───────────────┐  │
│  │ Permission UI │  │◄────────┤  │ Tool Request  │  │
│  │               │  │  WebSocket │               │  │
│  └───────────────┘  │ control_ │  └───────────────┘  │
│          │          │ request  │          │          │
│          ▼          │          │          ▼          │
│  ┌───────────────┐  │         │  ┌───────────────┐  │
│  │ User Decision │  │         │  │ Tool Executor │  │
│  │ (Allow/Deny)  │  │────────►│  │               │  │
│  └───────────────┘  │control_ │  └───────────────┘  │
│                     │response │                     │
└─────────────────────┘         └─────────────────────┘
```

---

## 12. Key Files Reference

| File | Lines | Purpose |
|------|-------|---------|
| `remote/RemoteSessionManager.ts` | 344 | Remote session coordinator |
| `remote/SessionsWebSocket.ts` | 405 | WebSocket client with reconnection |
| `remote/sdkMessageAdapter.ts` | 303 | Message format conversion |
| `remote/remotePermissionBridge.ts` | 79 | Synthetic permission messages |
| `bridge/createSession.ts` | 185 | Bridge session creation |
| `bridge/replBridge.ts` | 2500+ | REPL bridge core logic |
| `bridge/replBridgeTransport.ts` | 371 | Transport abstraction (v1/v2) |
| `bridge/workSecret.ts` | 128 | Work secret decoding, URL building |
| `bridge/bridgeMessaging.ts` | 462 | Message handling, echo dedup |
| `bridge/types.ts` | 263 | Bridge type definitions |
| `utils/teleport/api.ts` | 467 | Teleport API client |
| `utils/sessionIngressAuth.ts` | 141 | Token retrieval logic |
| `cli/transports/SSETransport.ts` | — | SSE transport implementation |
| `cli/transports/CCRClient.ts` | — | CCR v2 client |

---

## 13. Summary

Claude Code remote execution is built on:

1. **WebSocket-based real-time communication** — Dual runtime support (Bun/Node)
2. **JWT-based authentication** — Session ingress tokens with proactive refresh
3. **Work secret protocol** — Bridge registration, work polling, heartbeat
4. **Control request/response** — Permission handling across network boundary
5. **Dual transport modes** — CCR v1 (Session Ingress) and CCR v2 (Worker API)
6. **Resilience patterns** — Reconnection with backoff, SSE sequence resume, epoch recovery
7. **Security hardening** — Token storage strategies, prctl, NO_PROXY list

The architecture enables seamless remote execution where:
- Mobile users can control powerful remote containers
- Desktop bridges can serve as workers for web sessions
- All communication is authenticated and encrypted
- Permission prompts are faithfully relayed across the network

---

**Created:** 2026-04-07  
**Status:** Complete
