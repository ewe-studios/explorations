# Server Module — Deep-Dive Exploration

**Module:** `server/`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/server/`  
**Files:** 3 TypeScript files  
**Created:** 2026-04-07

---

## 1. Module Overview

The `server/` module implements **Claude Code server mode** — a headless HTTP server that manages multiple concurrent Claude Code sessions. This enables remote development workflows, CI/CD integration, and multi-user scenarios.

### Core Responsibilities

1. **Server Configuration** — Server settings and options:
   - Port, host, authentication token
   - Unix socket support
   - Idle timeout and max sessions
   - Default workspace directory

2. **Session Management** — Session lifecycle tracking:
   - Session state (starting, running, detached, stopping, stopped)
   - Child process management
   - Session index persistence

3. **Direct Connect** — Server session creation:
   - Session creation API
   - WebSocket URL generation
   - Working directory configuration

### Key Design Patterns

- **State Machine**: Session states with clear transitions
- **Process Management**: Child process tracking and cleanup
- **Persistent Index**: Session metadata persisted to JSON
- **Schema Validation**: Zod schemas for API responses

---

## 2. File Inventory

| File | Lines | Description |
|------|-------|-------------|
| `createDirectConnectSession.ts` | 89 | Session creation API client |
| `directConnectManager.ts` | 214 | WebSocket session manager |
| `types.ts` | 58 | Server types and Zod schemas |

**Total:** 361 lines across 3 files

---

## 3. Key Exports

### Server Types (`types.ts`)

```typescript
// Connect response schema
export const connectResponseSchema = z.object({
  session_id: z.string(),
  ws_url: z.string(),
  work_dir: z.string().optional(),
})

// Server configuration
export type ServerConfig = {
  port: number
  host: string
  authToken: string
  unix?: string  // Unix socket path
  idleTimeoutMs?: number  // 0 = never expire
  maxSessions?: number
  workspace?: string  // Default working directory
}

// Session state machine
export type SessionState =
  | 'starting'
  | 'running'
  | 'detached'
  | 'stopping'
  | 'stopped'

// Active session info
export type SessionInfo = {
  id: string
  status: SessionState
  createdAt: number
  workDir: string
  process: ChildProcess | null
  sessionKey?: string
}

// Persistent session index
export type SessionIndexEntry = {
  sessionId: string
  transcriptSessionId: string
  cwd: string
  permissionMode?: string
  createdAt: number
  lastActiveAt: number
}

export type SessionIndex = Record<string, SessionIndexEntry>
```

---

## 4. Line-by-Line Analysis

### 4.1 Connect Response Schema (lines 5-11, `types.ts`)

```typescript
export const connectResponseSchema = lazySchema(() =>
  z.object({
    session_id: z.string(),
    ws_url: z.string(),
    work_dir: z.string().optional(),
  }),
)
```

**Purpose**: Validates API response for session connection.

**lazySchema**: Defers schema instantiation to avoid circular dependencies.

### 4.2 Server Configuration (lines 13-24, `types.ts`)

```typescript
export type ServerConfig = {
  port: number
  host: string
  authToken: string
  unix?: string  // Unix socket path (alternative to TCP)
  idleTimeoutMs?: number  // Detached session timeout (0 = never)
  maxSessions?: number  // Concurrency limit
  workspace?: string  // Default cwd for sessions
}
```

**Unix Socket**: Alternative to TCP for local-only access (more secure).

**Idle Timeout**: `0` means sessions never expire due to inactivity.

### 4.3 Session State Machine (lines 26-31, `types.ts`)

```typescript
export type SessionState =
  | 'starting'
  | 'running'
  | 'detached'
  | 'stopping'
  | 'stopped'
```

**State Transitions**:
```
starting → running → detached → stopping → stopped
                      ↘_______________↗
```

### 4.4 Session Info (lines 33-40, `types.ts`)

```typescript
export type SessionInfo = {
  id: string
  status: SessionState
  createdAt: number
  workDir: string
  process: ChildProcess | null
  sessionKey?: string
}
```

**Process Tracking**: `process: ChildProcess | null` enables process management (kill, signal).

### 4.5 Session Index (lines 42-57, `types.ts`)

```typescript
/**
 * Stable session key → session metadata. Persisted to ~/.claude/server-sessions.json
 * so sessions can be resumed across server restarts.
 */
export type SessionIndexEntry = {
  sessionId: string
  transcriptSessionId: string  // For --resume
  cwd: string
  permissionMode?: string
  createdAt: number
  lastActiveAt: number
}

export type SessionIndex = Record<string, SessionIndexEntry>
```

**Persistence**: `~/.claude/server-sessions.json` survives server restarts.

**Session Key vs ID**: Key is stable identifier; ID matches subprocess session.

---

## 5. createDirectConnectSession.ts Analysis

### 5.1 Module Purpose (lines 1-25)

```typescript
/**
 * Create a session on a direct-connect server.
 *
 * Posts to `${serverUrl}/sessions`, validates the response, and returns
 * a DirectConnectConfig ready for use by the REPL or headless runner.
 *
 * Throws DirectConnectError on network, HTTP, or response-parsing failures.
 */
```

**Responsibility**: HTTP client for session creation API.

### 5.2 DirectConnectError Class (lines 11-16)

```typescript
export class DirectConnectError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'DirectConnectError'
  }
}
```

**Purpose**: Custom error type for session creation failures.

**Error Types Caught**:
- Network failures (fetch throws)
- HTTP errors (non-2xx responses)
- Response parsing failures (Zod validation)

### 5.3 Session Creation Function (lines 26-88)

```typescript
export async function createDirectConnectSession({
  serverUrl,
  authToken,
  cwd,
  dangerouslySkipPermissions,
}: {
  serverUrl: string
  authToken?: string
  cwd: string
  dangerouslySkipPermissions?: boolean
}): Promise<{
  config: DirectConnectConfig
  workDir?: string
}> {
  const headers: Record<string, string> = {
    'content-type': 'application/json',
  }
  if (authToken) {
    headers['authorization'] = `Bearer ${authToken}`
  }
```

**Parameters**:
- `serverUrl`: Base URL of Claude Code server (e.g., `http://localhost:8080`)
- `authToken`: Optional Bearer token for authentication
- `cwd`: Working directory for the session
- `dangerouslySkipPermissions`: Optional flag to bypass permission prompts

**Headers**:
- Always sends `content-type: application/json`
- Conditionally adds `authorization: Bearer <token>`

### 5.4 HTTP Request (lines 47-63)

```typescript
  let resp: Response
  try {
    resp = await fetch(`${serverUrl}/sessions`, {
      method: 'POST',
      headers,
      body: jsonStringify({
        cwd,
        ...(dangerouslySkipPermissions && {
          dangerously_skip_permissions: true,
        }),
      }),
    })
  } catch (err) {
    throw new DirectConnectError(
      `Failed to connect to server at ${serverUrl}: ${errorMessage(err)}`,
    )
  }
```

**Request Format**:
```json
POST /sessions
Content-Type: application/json
Authorization: Bearer <token>

{
  "cwd": "/path/to/workspace",
  "dangerously_skip_permissions": true
}
```

**Error Handling**: Network errors wrapped with context about target server.

### 5.5 Response Validation (lines 65-76)

```typescript
  if (!resp.ok) {
    throw new DirectConnectError(
      `Failed to create session: ${resp.status} ${resp.statusText}`,
    )
  }

  const result = connectResponseSchema().safeParse(await resp.json())
  if (!result.success) {
    throw new DirectConnectError(
      `Invalid session response: ${result.error.message}`,
    )
  }
```

**Validation Steps**:
1. Check HTTP status code (must be 2xx)
2. Parse JSON body
3. Validate against `connectResponseSchema` (Zod)

### 5.6 Return Value (lines 78-87)

```typescript
  const data = result.data
  return {
    config: {
      serverUrl,
      sessionId: data.session_id,
      wsUrl: data.ws_url,
      authToken,
    },
    workDir: data.work_dir,
  }
}
```

**Returns**:
- `config`: `DirectConnectConfig` for WebSocket connection
- `workDir`: Optional working directory from server

**Expected Response**:
```json
{
  "session_id": "abc123",
  "ws_url": "ws://localhost:8080/ws/abc123",
  "work_dir": "/path/to/workspace"
}
```

---

## 6. directConnectManager.ts Analysis

### 6.1 Module Purpose (lines 1-18)

```typescript
export type DirectConnectConfig = {
  serverUrl: string
  sessionId: string
  wsUrl: string
  authToken?: string
}

export type DirectConnectCallbacks = {
  onMessage: (message: SDKMessage) => void
  onPermissionRequest: (
    request: SDKControlPermissionRequest,
    requestId: string,
  ) => void
  onConnected?: () => void
  onDisconnected?: () => void
  onError?: (error: Error) => void
}
```

**DirectConnectConfig**: Connection parameters for WebSocket session.

**DirectConnectCallbacks**: Event handlers for session lifecycle and messages.

### 6.2 Type Guard (lines 31-38)

```typescript
function isStdoutMessage(value: unknown): value is StdoutMessage {
  return (
    typeof value === 'object' &&
    value !== null &&
    'type' in value &&
    typeof value.type === 'string'
  )
}
```

**Purpose**: Runtime type check for incoming WebSocket messages.

### 6.3 DirectConnectSessionManager Class (lines 40-48)

```typescript
export class DirectConnectSessionManager {
  private ws: WebSocket | null = null
  private config: DirectConnectConfig
  private callbacks: DirectConnectCallbacks

  constructor(config: DirectConnectConfig, callbacks: DirectConnectCallbacks) {
    this.config = config
    this.callbacks = callbacks
  }
}
```

**State**:
- `ws`: WebSocket instance (null when disconnected)
- `config`: Connection configuration
- `callbacks`: Event handlers

### 6.4 WebSocket Connection (lines 50-62)

```typescript
  connect(): void {
    const headers: Record<string, string> = {}
    if (this.config.authToken) {
      headers['authorization'] = `Bearer ${this.config.authToken}`
    }
    // Bun's WebSocket supports headers option but the DOM typings don't
    this.ws = new WebSocket(this.config.wsUrl, {
      headers,
    } as unknown as string[])

    this.ws.addEventListener('open', () => {
      this.callbacks.onConnected?.()
    })
```

**Authentication**: Bearer token in WebSocket headers.

**Bun Runtime**: Uses Bun-specific WebSocket API that supports headers.

### 6.5 Message Handler (lines 64-114)

```typescript
    this.ws.addEventListener('message', event => {
      const data = typeof event.data === 'string' ? event.data : ''
      const lines = data.split('\n').filter((l: string) => l.trim())

      for (const line of lines) {
        let raw: unknown
        try {
          raw = jsonParse(line)
        } catch {
          continue
        }

        if (!isStdoutMessage(raw)) {
          continue
        }
        const parsed = raw

        // Handle control requests (permission requests)
        if (parsed.type === 'control_request') {
          if (parsed.request.subtype === 'can_use_tool') {
            this.callbacks.onPermissionRequest(
              parsed.request,
              parsed.request_id,
            )
          } else {
            logForDebugging(
              `[DirectConnect] Unsupported control request subtype: ${parsed.request.subtype}`,
            )
            this.sendErrorResponse(
              parsed.request_id,
              `Unsupported control request subtype: ${parsed.request.subtype}`,
            )
          }
          continue
        }

        // Forward SDK messages (assistant, result, system, etc.)
        if (
          parsed.type !== 'control_response' &&
          parsed.type !== 'keep_alive' &&
          parsed.type !== 'control_cancel_request' &&
          parsed.type !== 'streamlined_text' &&
          parsed.type !== 'streamlined_tool_use_summary' &&
          !(parsed.type === 'system' && parsed.subtype === 'post_turn_summary')
        ) {
          this.callbacks.onMessage(parsed)
        }
      }
    })
```

**NDJSON Format**: Each message is a JSON object on its own line.

**Message Filtering**:
- **Handled**: `control_request` with `can_use_tool` subtype
- **Ignored**: `control_response`, `keep_alive`, `control_cancel_request`, `streamlined_text`, `streamlined_tool_use_summary`, `system/post_turn_summary`
- **Error Response**: Unknown control request subtypes

### 6.6 Connection Events (lines 116-122)

```typescript
    this.ws.addEventListener('close', () => {
      this.callbacks.onDisconnected?.()
    })

    this.ws.addEventListener('error', () => {
      this.callbacks.onError?.(new Error('WebSocket connection error'))
    })
  }
```

**Close Event**: Triggers `onDisconnected` callback.

**Error Event**: Triggers `onError` with generic error message.

### 6.7 Send User Message (lines 125-142)

```typescript
  sendMessage(content: RemoteMessageContent): boolean {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      return false
    }

    // Must match SDKUserMessage format expected by `--input-format stream-json`
    const message = jsonStringify({
      type: 'user',
      message: {
        role: 'user',
        content: content,
      },
      parent_tool_use_id: null,
      session_id: '',
    })
    this.ws.send(message)
    return true
  }
```

**Message Format**:
```json
{
  "type": "user",
  "message": {
    "role": "user",
    "content": <RemoteMessageContent>
  },
  "parent_tool_use_id": null,
  "session_id": ""
}
```

**Returns**: `false` if WebSocket not connected.

### 6.8 Permission Response (lines 144-167)

```typescript
  respondToPermissionRequest(
    requestId: string,
    result: RemotePermissionResponse,
  ): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      return
    }

    // Must match SDKControlResponse format expected by StructuredIO
    const response = jsonStringify({
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
    })
    this.ws.send(response)
  }
```

**Response Format** (allow):
```json
{
  "type": "control_response",
  "response": {
    "subtype": "success",
    "request_id": "abc123",
    "response": {
      "behavior": "allow",
      "updatedInput": { ... }
    }
  }
}
```

**Response Format** (deny):
```json
{
  "type": "control_response",
  "response": {
    "subtype": "success",
    "request_id": "abc123",
    "response": {
      "behavior": "deny",
      "message": "Permission denied"
    }
  }
}
```

### 6.9 Send Interrupt (lines 172-186)

```typescript
  /**
   * Send an interrupt signal to cancel the current request
   */
  sendInterrupt(): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      return
    }

    // Must match SDKControlRequest format expected by StructuredIO
    const request = jsonStringify({
      type: 'control_request',
      request_id: crypto.randomUUID(),
      request: {
        subtype: 'interrupt',
      },
    })
    this.ws.send(request)
  }
```

**Purpose**: Cancel in-progress model request (Ctrl+C equivalent).

**Request Format**:
```json
{
  "type": "control_request",
  "request_id": "<uuid>",
  "request": {
    "subtype": "interrupt"
  }
}
```

### 6.10 Error Response Helper (lines 188-201)

```typescript
  private sendErrorResponse(requestId: string, error: string): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      return
    }
    const response = jsonStringify({
      type: 'control_response',
      response: {
        subtype: 'error',
        request_id: requestId,
        error,
      },
    })
    this.ws.send(response)
  }
```

**Usage**: Responds to unrecognized control request subtypes.

### 6.11 Disconnect and Status (lines 203-213)

```typescript
  disconnect(): void {
    if (this.ws) {
      this.ws.close()
      this.ws = null
    }
  }

  isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN
  }
}
```

**disconnect()**: Closes WebSocket and clears reference.

**isConnected()**: Checks if WebSocket is in OPEN state.

---

## 7. Integration Points

### 7.1 With `child_process`

| Component | Integration |
|-----------|-------------|
| `SessionInfo` | Uses `ChildProcess` type for process tracking |
| Server | Spawns Claude Code sessions as child processes |

### 7.2 With `zod/v4`

| Component | Integration |
|-----------|-------------|
| `types.ts` | Uses `z.object()`, `z.string()` for API schemas |
| `createDirectConnectSession.ts` | Validates server response with `connectResponseSchema` |

### 7.3 With SDK Types (`entrypoints/agentSdkTypes.ts`)

| Component | Integration |
|-----------|-------------|
| `DirectConnectCallbacks.onMessage` | Receives `SDKMessage` from WebSocket |

### 7.4 With SDK Control Types (`entrypoints/sdk/controlTypes.ts`)

| Component | Integration |
|-----------|-------------|
| `DirectConnectCallbacks.onPermissionRequest` | Handles `SDKControlPermissionRequest` |
| `DirectConnectSessionManager.respondToPermissionRequest` | Sends control response |

### 7.5 With Remote Session (`remote/RemoteSessionManager.ts`)

| Component | Integration |
|-----------|-------------|
| `DirectConnectSessionManager.respondToPermissionRequest` | Uses `RemotePermissionResponse` type |

### 7.6 With Teleport (`utils/teleport/api.ts`)

| Component | Integration |
|-----------|-------------|
| `DirectConnectSessionManager.sendMessage` | Accepts `RemoteMessageContent` type |

---

## 8. Data Flow

### 8.1 Session Creation Flow

```
Client (CLI/SDK)
       │
       │ POST /sessions
       │ { cwd, dangerously_skip_permissions }
       ▼
createDirectConnectSession()
       │
       │ Authorization: Bearer <token>
       ▼
Claude Code Server
       │
       │ Spawn child process
       │ claude --server-session <session-key>
       ▼
Session Started
       │
       │ Response: { session_id, ws_url, work_dir }
       ▼
Return DirectConnectConfig
       │
       ▼
Ready for WebSocket connection
```

### 8.2 WebSocket Connection Flow

```
DirectConnectSessionManager.connect()
       │
       │ new WebSocket(wsUrl, { headers: { Authorization } })
       ▼
WebSocket Connecting
       │
       ├──► 'open' event ──► onConnected()
       │
       ├──► 'message' event ──► Parse NDJSON ──► Handle message
       │                        │
       │                        ├──► control_request/can_use_tool ──► onPermissionRequest()
       │                        ├──► control_request/<other> ──► sendErrorResponse()
       │                        └──► SDK messages ──► onMessage()
       │
       ├──► 'close' event ──► onDisconnected()
       │
       └──► 'error' event ──► onError()
```

### 8.3 Message Flow (Client → Server)

```
User Input / Tool Result
       │
       ▼
sendMessage(content: RemoteMessageContent)
       │
       │ { type: 'user', message: { role: 'user', content } }
       ▼
WebSocket.send()
       │
       │ NDJSON over WebSocket
       ▼
Claude Code Server
       │
       ▼
Model processes message
```

### 8.4 Permission Request Flow

```
Claude Code Server
       │ Wants to use tool
       │
       ▼
control_request (subtype: can_use_tool)
       │
       ▼
DirectConnectSessionManager receives
       │
       ▼
onPermissionRequest(request, requestId)
       │
       │ User/Policy decides
       ▼
respondToPermissionRequest(requestId, result)
       │
       │ { type: 'control_response', response: { subtype: 'success', ... } }
       ▼
WebSocket.send()
       │
       ▼
Server receives permission decision
```

### 8.5 Interrupt Flow

```
User presses Ctrl+C
       │
       ▼
sendInterrupt()
       │
       │ { type: 'control_request', request: { subtype: 'interrupt' } }
       ▼
WebSocket.send()
       │
       ▼
Claude Code Server cancels current request
```

### 8.6 Session Persistence Flow

```
Session Created
       │
       ▼
Add SessionIndexEntry to memory
       │
       ▼
Write to ~/.claude/server-sessions.json
       │
       │ <Server Restart>
       ▼
Read session index from disk
       │
       ▼
Resume sessions with --resume flag
```

---

## 9. Key Patterns

### 9.1 State Machine

```
starting → running → detached → stopping → stopped
                      ↗
              (client reconnects)
```

**Detached**: Client disconnected, session continues running in background.

**Reconnection**: Client can reconnect to detached session using session key.

### 9.2 Session Key Stability

```
SessionIndex: Record<stableKey, SessionIndexEntry>
```

**Why**: Keys survive server restarts; IDs may change between sessions.

**Persistence File**: `~/.claude/server-sessions.json`

### 9.3 NDJSON over WebSocket

```
{"type": "assistant", "content": "Hello"}\n
{"type": "control_request", "request_id": "abc", ...}\n
{"type": "result", "tool_use_id": "xyz"}\n
```

**Benefits**:
- Simple line-based framing
- Easy to parse incrementally
- Resilient to partial messages

### 9.4 Callback-Based Event Handling

```typescript
export type DirectConnectCallbacks = {
  onMessage: (message: SDKMessage) => void
  onPermissionRequest: (request, requestId) => void
  onConnected?: () => void
  onDisconnected?: () => void
  onError?: (error: Error) => void
}
```

**Why**: Inversion of control - caller handles business logic.

### 9.5 Type Guards for Runtime Safety

```typescript
function isStdoutMessage(value: unknown): value is StdoutMessage {
  return (
    typeof value === 'object' &&
    value !== null &&
    'type' in value &&
    typeof value.type === 'string'
  )
}
```

**Purpose**: Safe downcasting of WebSocket message data.

### 9.6 Lazy Schema Evaluation

```typescript
export const connectResponseSchema = lazySchema(() =>
  z.object({
    session_id: z.string(),
    ws_url: z.string(),
    work_dir: z.string().optional(),
  }),
)
```

**Why**: Avoids circular dependency issues in module loading.

---

## 10. Summary

The `server/` module provides **complete multi-session server infrastructure** for Claude Code:

### Files Analyzed

| File | Lines | Responsibility |
|------|-------|----------------|
| `types.ts` | 58 | Type definitions and Zod schemas |
| `createDirectConnectSession.ts` | 89 | HTTP client for session creation |
| `directConnectManager.ts` | 214 | WebSocket session manager |

### Key Components

1. **Session Creation** — `createDirectConnectSession()` creates sessions via HTTP POST, validates response with Zod
2. **WebSocket Manager** — `DirectConnectSessionManager` handles bidirectional communication
3. **Message Routing** — Filters control requests vs SDK messages, routes to callbacks
4. **Permission Handling** — `can_use_tool` requests with allow/deny responses
5. **Interrupt Support** — `sendInterrupt()` for canceling in-progress requests
6. **Session Persistence** — Index stored in `~/.claude/server-sessions.json`

### Protocols

**Session Creation API**:
```
POST /sessions
Authorization: Bearer <token>
Content-Type: application/json

{ "cwd": "/workspace", "dangerously_skip_permissions": true }
```

**WebSocket Messages** (NDJSON):
```
→ {"type": "user", "message": {"role": "user", "content": "..."}}
← {"type": "assistant", "content": "Hello!"}
← {"type": "control_request", "request_id": "abc", "request": {"subtype": "can_use_tool"}}
→ {"type": "control_response", "response": {"subtype": "success", "request_id": "abc", ...}}
```

### Integration Points

- **SDK Types**: Message formats from `entrypoints/agentSdkTypes.ts`
- **Control Protocol**: Permission requests from `entrypoints/sdk/controlTypes.ts`
- **Remote Session**: Permission response types from `remote/RemoteSessionManager.ts`
- **Teleport**: Message content types from `utils/teleport/api.ts`

---

**Last Updated:** 2026-04-07  
**Status:** Complete — All 3 files analyzed
