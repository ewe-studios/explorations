# Bridge Module Deep Dive Documentation

## Overview

The `bridge/` module is the **critical communication layer** between the local CLI and claude.ai web interface. It implements a sophisticated remote control protocol that allows users to interact with their local Claude Code instance from the claude.ai web UI.

**Source Directory:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/bridge/`

**Total Files:** 31 TypeScript files  
**Total Lines:** ~880,000+ lines of TypeScript code

---

## Table of Contents

1. [Module File Inventory](#1-module-file-inventory)
2. [Architecture Overview](#2-architecture-overview)
3. [Core Protocol Implementation](#3-core-protocol-implementation)
4. [Transport Layer (replBridgeTransport.ts)](#4-transport-layer-replbridgetransportts)
5. [JWT Authentication System](#5-jwt-authentication-system)
6. [Remote Bridge Core (remoteBridgeCore.ts)](#6-remote-bridge-core-remotebridgecorets)
7. [Bridge API Client (bridgeApi.ts)](#7-bridge-api-client-bridgeapits)
8. [Session Management (sessionRunner.ts, createSession.ts)](#8-session-management-sessionrunnerts-createsessionts)
9. [Message Protocol (bridgeMessaging.ts)](#9-message-protocol-bridgemessagingts)
10. [Complete Message Flow](#10-complete-message-flow)
11. [Work Secret Protocol](#11-work-secret-protocol)
12. [Environment Registration](#12-environment-registration)
13. [Trusted Device Authentication](#13-trusted-device-authentication)
14. [Configuration System](#14-configuration-system)
15. [Error Handling and Recovery](#15-error-handling-and-recovery)

---

## 1. Module File Inventory

### Core Files (by Line Count)

| File | Lines | Purpose |
|------|-------|---------|
| `bridgeMain.ts` | 115,571 | Standalone bridge daemon entry point, multi-session orchestration |
| `replBridge.ts` | 100,537 | REPL-integrated bridge initialization and core loop |
| `remoteBridgeCore.ts` | 39,434 | Environment-less (v2) bridge core for direct CCR v2 protocol |
| `initReplBridge.ts` | 23,849 | REPL bootstrap wrapper, gates, OAuth, title derivation |
| `bridgeUI.ts` | 16,780 | Terminal status display, QR code generation, spinners |
| `bridgeMessaging.ts` | 15,703 | Message parsing, echo dedup, control request/response handling |
| `createSession.ts` | 12,157 | Session creation API, title updates, archival |
| `types.ts` | 10,161 | TypeScript type definitions for all bridge concepts |
| `jwtUtils.ts` | 9,444 | Token refresh scheduler, JWT expiry decoding |
| `bridgeEnabled.ts` | 8,442 | Feature gates, policy checks, version requirements |
| `bridgePointer.ts` | 7,611 | Crash-recovery pointer file for perpetual sessions |
| `trustedDevice.ts` | 7,764 | Trusted device enrollment, device token management |
| `envLessBridgeConfig.ts` | 7,250 | V2 bridge configuration via GrowthBook |
| `inboundAttachments.ts` | 6,267 | Attachment handling for inbound messages |
| `bridgeStatusUtil.ts` | 5,143 | Status line utilities, duration formatting |
| `pollConfig.ts` | 4,562 | Poll interval configuration with schema validation |
| `codeSessionApi.ts` | 4,840 | CCR v2 session API (create, bridge credentials) |
| `workSecret.ts` | 4,672 | Work secret decoding, SDK URL building, worker registration |
| `bridgeDebug.ts` | 4,926 | Debug utilities, fault injection for testing |
| `pollConfigDefaults.ts` | 4,018 | Default poll configuration values |
| `sessionRunner.ts` | 18,020 | Child process spawner, activity tracking |
| `replBridgeTransport.ts` | 15,523 | Transport abstraction for v1/v2 protocols |
| `bridgeApi.ts` | 18,066 | API client for environment/work endpoints |
| `bridgeConfig.ts` | 1,695 | Bridge token/URL override helpers |
| `bridgePermissionCallbacks.ts` | 1,411 | Permission callback wrappers |
| `replBridgeHandle.ts` | 1,473 | Handle type definitions |
| `sessionIdCompat.ts` | 2,536 | Session ID format compatibility (cse_* vs session_*) |
| `capacityWake.ts` | 1,841 | Sleep/wake primitive for poll loops |
| `flushGate.ts` | 1,981 | Message flush gating during history sync |
| `inboundMessages.ts` | 2,727 | Inbound message routing |
| `debugUtils.ts` | 4,240 | Debug logging, error formatting |

---

## 2. Architecture Overview

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            claude.ai Web UI                                  │
│                    (User types, clicks, sees responses)                      │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ HTTPS/WSS
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Anthropic Backend                                   │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────┐  │
│  │  Environments   │  │  CCR v2         │  │  Session Ingress            │  │
│  │  API            │  │  /worker/*      │  │  /v{1,2}/session_ingress/*  │  │
│  │                 │  │                 │  │                             │  │
│  │  - register     │  │  - events       │  │  - WebSocket message        │  │
│  │  - poll work    │  │  - state        │  │    delivery                 │  │
│  │  - heartbeat    │  │  - heartbeat    │  │  - control_request/response │  │
│  │  - ack/stop     │  │  - metadata     │  │                             │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
            │                    │                          │
            │                    │                          │
            ▼                    ▼                          ▼
    ┌──────────────┐     ┌──────────────┐          ┌──────────────┐
    │  v1 Protocol │     │  v2 Protocol │          │  Hybrid      │
    │  (Env-based) │     │  (Direct)    │          │  Transport   │
    │              │     │              │          │              │
    │  - register  │     │  - POST      │          │  - WS read   │
    │  - poll      │     │    /bridge   │          │  - HTTP POST │
    │  - ack       │     │  - SSE read  │          │    write     │
    │  - heartbeat │     │  - CCR write │          │              │
    └──────────────┘     └──────────────┘          └──────────────┘
            │                    │                          │
            └────────────────────┼──────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Local CLI (bridge module)                           │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                    Bridge Core (replBridge.ts)                        │  │
│  │                                                                       │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │  │
│  │  │ Transport   │  │ Messaging   │  │ Session     │  │ JWT         │  │  │
│  │  │ Adapter     │  │ Handler     │  │ Spawner     │  │ Scheduler   │  │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘  │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                 │                                           │
│                                 ▼                                           │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                    Child Process (claude --sdk-url)                   │  │
│  │                    - Runs actual inference                            │  │
│  │                    - Reads from stdin, writes to stdout               │  │
│  │                    - Connected via NDJSON stream                      │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Two Protocol Versions

**V1 Protocol (Environment-Based):**
- Uses Environments API for work dispatch
- Poll-based: `GET /v1/environments/{id}/work/poll`
- WebSocket for real-time message delivery
- OAuth tokens for authentication
- Requires explicit ack/stop lifecycle

**V2 Protocol (CCR v2 - Direct):**
- Direct connection to CCR (Claude Code Runtime) v2 endpoints
- SSE (Server-Sent Events) for inbound messages
- HTTP POST to `/worker/events` for outbound
- JWT authentication (session_ingress_token)
- Heartbeat-based lease management
- Epoch-based concurrency control

### Key Components

1. **Transport Layer** - Abstracts v1/v2 protocol differences
2. **Message Handler** - Parses, dedups, routes ingress messages
3. **Session Spawner** - Manages child Claude processes
4. **JWT Scheduler** - Proactively refreshes tokens before expiry
5. **API Client** - HTTP wrappers for all backend endpoints
6. **UI Logger** - Terminal status display with QR codes

---

## 3. Core Protocol Implementation

### 3.1 Bridge Core Initialization Flow

The core initialization is implemented in `initBridgeCore()` (replBridge.ts) and `initEnvLessBridgeCore()` (remoteBridgeCore.ts).

**V1 (Environment-Based) Flow:**

```typescript
export async function initBridgeCore(
  params: BridgeCoreParams,
): Promise<BridgeCoreHandle | null> {
  // 1. Read crash-recovery pointer (perpetual mode)
  const rawPrior = perpetual ? await readBridgePointer(dir) : null
  const prior = rawPrior?.source === 'repl' ? rawPrior : null

  // 2. Create API client with auth
  const rawApi = createBridgeApiClient({
    baseUrl,
    getAccessToken,
    runnerVersion: MACRO.VERSION,
    onDebug: logForDebugging,
    onAuth401,
    getTrustedDeviceToken,
  })

  // 3. Register bridge environment
  const bridgeConfig: BridgeConfig = {
    dir,
    machineName,
    branch,
    gitRepoUrl,
    maxSessions: 1,
    spawnMode: 'single-session',
    verbose: false,
    sandbox: false,
    bridgeId: randomUUID(),
    workerType,
    environmentId: randomUUID(),
    reuseEnvironmentId: prior?.environmentId,  // Crash recovery
    apiBaseUrl: baseUrl,
    sessionIngressUrl,
  }

  let environmentId: string
  let environmentSecret: string
  try {
    const reg = await api.registerBridgeEnvironment(bridgeConfig)
    environmentId = reg.environment_id
    environmentSecret = reg.environment_secret
  } catch (err) {
    onStateChange?.('failed', errorMessage(err))
    return null
  }

  // 4. Reconnect in-place (perpetual mode recovery)
  const reusedPriorSession = prior
    ? await tryReconnectInPlace(prior.environmentId, prior.sessionId)
    : false

  // 5. Create or reconnect session
  let currentSessionId: string
  if (reusedPriorSession && prior) {
    currentSessionId = prior.sessionId
  } else {
    const createdSessionId = await createSession({
      environmentId,
      title,
      gitRepoUrl,
      branch,
      signal: AbortSignal.timeout(15_000),
    })
    if (!createdSessionId) {
      await api.deregisterEnvironment(environmentId)
      return null
    }
    currentSessionId = createdSessionId
  }

  // 6. Write crash-recovery pointer
  await writeBridgePointer(dir, {
    sessionId: currentSessionId,
    environmentId,
    source: 'repl',
  })

  // 7. Start poll loop for work items
  // ... (poll loop implementation)
}
```

**V2 (Environment-Less) Flow:**

```typescript
export async function initEnvLessBridgeCore(
  params: EnvLessBridgeParams,
): Promise<ReplBridgeHandle | null> {
  const cfg = await getEnvLessBridgeConfig()

  // 1. Create session (POST /v1/code/sessions, no env_id)
  const accessToken = getAccessToken()
  if (!accessToken) return null

  const createdSessionId = await withRetry(
    () => createCodeSession(baseUrl, accessToken, title, cfg.http_timeout_ms, tags),
    'createCodeSession',
    cfg,
  )
  if (!createdSessionId) {
    onStateChange?.('failed', 'Session creation failed')
    logBridgeSkip('v2_session_create_failed', undefined, true)
    return null
  }
  const sessionId: string = createdSessionId

  // 2. Fetch bridge credentials (POST /bridge → worker_jwt, expires_in)
  const credentials = await withRetry(
    () => fetchRemoteCredentials(sessionId, baseUrl, accessToken, cfg.http_timeout_ms),
    'fetchRemoteCredentials',
    cfg,
  )
  if (!credentials) {
    void archiveSession(sessionId, baseUrl, accessToken, orgUUID, cfg.http_timeout_ms)
    return null
  }

  // 3. Build v2 transport (SSETransport + CCRClient)
  const sessionUrl = buildCCRv2SdkUrl(credentials.api_base_url, sessionId)
  let transport: ReplBridgeTransport
  try {
    transport = await createV2ReplTransport({
      sessionUrl,
      ingressToken: credentials.worker_jwt,
      sessionId,
      epoch: credentials.worker_epoch,
      heartbeatIntervalMs: cfg.heartbeat_interval_ms,
      getAuthToken: () => credentials.worker_jwt,  // Per-instance closure
      outboundOnly,
    })
  } catch (err) {
    onStateChange?.('failed', `Transport setup failed`)
    void archiveSession(sessionId, baseUrl, accessToken, orgUUID, cfg.http_timeout_ms)
    return null
  }

  // 4. JWT refresh scheduler
  const refresh = createTokenRefreshScheduler({
    refreshBufferMs: cfg.token_refresh_buffer_ms,
    getAccessToken: async () => {
      const stale = getAccessToken()
      if (onAuth401) await onAuth401(stale ?? '')
      return getAccessToken() ?? stale
    },
    onRefresh: (sid, oauthToken) => {
      void (async () => {
        if (authRecoveryInFlight || tornDown) return
        authRecoveryInFlight = true
        try {
          const fresh = await fetchRemoteCredentials(sid, baseUrl, oauthToken, cfg.http_timeout_ms)
          if (!fresh || tornDown) return
          await rebuildTransport(fresh, 'proactive_refresh')
        } finally {
          authRecoveryInFlight = false
        }
      })()
    },
    label: 'remote',
  })
  refresh.scheduleFromExpiresIn(sessionId, credentials.expires_in)

  // 5. Wire transport callbacks
  wireTransportCallbacks()
  transport.connect()

  return {
    bridgeSessionId: sessionId,
    environmentId: '',  // V2 has no environment
    sessionIngressUrl: credentials.api_base_url,
    writeMessages,
    writeSdkMessages,
    sendControlRequest,
    sendControlResponse,
    sendControlCancelRequest,
    sendResult,
    teardown,
  }
}
```

### 3.2 Poll Loop (V1 Protocol)

The V1 poll loop is the heart of the environment-based bridge:

```typescript
while (!loopSignal.aborted) {
  // 1. Determine poll interval based on capacity
  const pollConfig = getPollIntervalConfig()
  const notAtCapacity = activeSessions.size < config.maxSessions
  const isAtCapacity = activeSessions.size >= config.maxSessions
  let pollIntervalMs: number

  if (notAtCapacity) {
    pollIntervalMs = pollConfig.poll_interval_ms_not_at_capacity
  } else {
    // At capacity: use heartbeat OR at-capacity polling
    if (pollConfig.non_exclusive_heartbeat_interval_ms > 0) {
      const now = Date.now()
      if (now - lastHeartbeatTime >= pollConfig.non_exclusive_heartbeat_interval_ms) {
        const result = await heartbeatActiveWorkItems()
        lastHeartbeatTime = now

        if (result === 'auth_failed') {
          // JWT expired - trigger re-dispatch
          continue  // Poll immediately for fresh work
        }
      }
    }
    pollIntervalMs = pollConfig.poll_interval_ms_at_capacity
  }

  // 2. Poll for work
  let work: WorkResponse | null
  try {
    work = await api.pollForWork(environmentId, environmentSecret, pollSignal)
  } catch (err) {
    // Handle poll errors with exponential backoff
    // ...
    continue
  }

  if (!work) {
    // No work available - sleep and retry
    await sleepWithSignal(pollIntervalMs, capacitySignal)
    continue
  }

  // 3. Work received - dispatch session
  if (work.data.type === 'session') {
    const sessionId = work.data.id
    const ingressToken = work.data.session_ingress_token

    // Acknowledge work to server
    await api.acknowledgeWork(environmentId, work.id, ingressToken)

    // Spawn or reuse session handle
    let handle: SessionHandle
    if (existingHandle) {
      // Reuse existing session (session resurrection)
      handle = existingHandle
      handle.updateAccessToken(ingressToken)
    } else {
      // Fresh spawn
      handle = safeSpawn(spawner, {
        sessionId,
        sdkUrl: buildSdkUrl(credentials.api_base_url, sessionId),
        accessToken: ingressToken,
        useCcrV2: work.secret.use_code_sessions,
        workerEpoch: work.secret.use_code_sessions ? await registerWorker(...) : undefined,
      }, config.dir)
    }

    activeSessions.set(sessionId, handle)
    sessionWorkIds.set(sessionId, work.id)
    sessionIngressTokens.set(sessionId, ingressToken)

    // Start session timeout watchdog
    const timer = setTimeout(() => {
      timedOutSessions.add(sessionId)
      handle.forceKill()
    }, config.sessionTimeoutMs)
    sessionTimers.set(sessionId, timer)

    // Create transport and connect
    if (work.secret.use_code_sessions) {
      // V2 transport
      transport = await createV2ReplTransport({ ... })
    } else {
      // V1 transport (HybridTransport)
      transport = createV1ReplTransport(hybridTransport)
    }

    transport.connect()
  }
}
```

---

## 4. Transport Layer (replBridgeTransport.ts)

### 4.1 Transport Interface

The transport abstraction allows the bridge core to work with both V1 and V2 protocols seamlessly:

```typescript
export type ReplBridgeTransport = {
  write(message: StdoutMessage): Promise<void>
  writeBatch(messages: StdoutMessage[]): Promise<void>
  close(): void
  isConnectedStatus(): boolean
  getStateLabel(): string
  setOnData(callback: (data: string) => void): void
  setOnClose(callback: (closeCode?: number) => void): void
  setOnConnect(callback: () => void): void
  connect(): void
  getLastSequenceNum(): number  // SSE sequence number for replay prevention
  readonly droppedBatchCount: number  // Batches dropped due to failures
  reportState(state: SessionState): void  // V2 only
  reportMetadata(metadata: Record<string, unknown>): void  // V2 only
  reportDelivery(eventId: string, status: 'processing' | 'processed'): void  // V2 only
  flush(): Promise<void>  // V2 only - drain write queue
}
```

### 4.2 V1 Transport Adapter (HybridTransport)

```typescript
export function createV1ReplTransport(
  hybrid: HybridTransport,
): ReplBridgeTransport {
  return {
    write: msg => hybrid.write(msg),
    writeBatch: msgs => hybrid.writeBatch(msgs),
    close: () => hybrid.close(),
    isConnectedStatus: () => hybrid.isConnectedStatus(),
    getStateLabel: () => hybrid.getStateLabel(),
    setOnData: cb => hybrid.setOnData(cb),
    setOnClose: cb => hybrid.setOnClose(cb),
    setOnConnect: cb => hybrid.setOnConnect(cb),
    connect: () => void hybrid.connect(),
    // V1 Session-Ingress WS doesn't use SSE sequence numbers
    getLastSequenceNum: () => 0,
    get droppedBatchCount() {
      return hybrid.droppedBatchCount
    },
    // V1 no-ops - these are CCR v2 specific
    reportState: () => {},
    reportMetadata: () => {},
    reportDelivery: () => {},
    flush: () => Promise.resolve(),
  }
}
```

### 4.3 V2 Transport Adapter (SSETransport + CCRClient)

The V2 transport is more complex, combining SSE for reads and CCRClient for writes:

```typescript
export async function createV2ReplTransport(opts: {
  sessionUrl: string
  ingressToken: string
  sessionId: string
  initialSequenceNum?: number  // Resume from last SSE sequence
  epoch?: number  // Worker epoch from /bridge response
  heartbeatIntervalMs?: number
  heartbeatJitterFraction?: number
  outboundOnly?: boolean  // Skip SSE read stream
  getAuthToken?: () => string | undefined  // Per-instance auth
}): Promise<ReplBridgeTransport> {
  const {
    sessionUrl,
    ingressToken,
    sessionId,
    initialSequenceNum,
    getAuthToken,
  } = opts

  // Build auth header closure
  let getAuthHeaders: (() => Record<string, string>) | undefined
  if (getAuthToken) {
    getAuthHeaders = (): Record<string, string> => {
      const token = getAuthToken()
      if (!token) return {}
      return { Authorization: `Bearer ${token}` }
    }
  } else {
    // Legacy: write to process-wide env var
    updateSessionIngressAuthToken(ingressToken)
  }

  // Register worker (or use provided epoch)
  const epoch = opts.epoch ?? (await registerWorker(sessionUrl, ingressToken))

  // Build SSE stream URL
  const sseUrl = new URL(sessionUrl)
  sseUrl.pathname = sseUrl.pathname.replace(/\/$/, '') + '/worker/events/stream'

  // Create SSE transport for reads
  const sse = new SSETransport(
    sseUrl,
    {},
    sessionId,
    undefined,
    initialSequenceNum,  // Resume from last sequence
    getAuthHeaders,
  )

  // Create CCRClient for writes, heartbeat, state
  const ccr = new CCRClient(sse, new URL(sessionUrl), {
    getAuthHeaders,
    heartbeatIntervalMs: opts.heartbeatIntervalMs,
    heartbeatJitterFraction: opts.heartbeatJitterFraction,
    onEpochMismatch: () => {
      // Epoch superseded - close and notify for poll-loop recovery
      try {
        ccr.close()
        sse.close()
        onCloseCb?.(4090)  // 4090 = epoch mismatch
      } catch (closeErr: unknown) {
        logForDebugging(`[bridge:repl] CCR v2: error during cleanup: ${errorMessage(closeErr)}`)
      }
      throw new Error('epoch superseded')
    },
  })

  // Wire SSE delivery ACKs
  sse.setOnEvent(event => {
    ccr.reportDelivery(event.event_id, 'received')
    ccr.reportDelivery(event.event_id, 'processed')  // Immediate ACK
  })

  let onConnectCb: (() => void) | undefined
  let onCloseCb: ((closeCode?: number) => void) | undefined
  let ccrInitialized = false
  let closed = false

  return {
    write(msg) {
      return ccr.writeEvent(msg)
    },
    async writeBatch(msgs) {
      for (const m of msgs) {
        if (closed) break
        await ccr.writeEvent(m)
      }
    },
    close() {
      closed = true
      ccr.close()
      sse.close()
    },
    isConnectedStatus() {
      return ccrInitialized  // Write-readiness
    },
    getStateLabel() {
      if (sse.isClosedStatus()) return 'closed'
      if (sse.isConnectedStatus()) return ccrInitialized ? 'connected' : 'init'
      return 'connecting'
    },
    setOnData(cb) {
      sse.setOnData(cb)
    },
    setOnClose(cb) {
      onCloseCb = cb
      sse.setOnClose(code => {
        ccr.close()
        cb(code ?? 4092)  // 4092 = SSE reconnect budget exhausted
      })
    },
    setOnConnect(cb) {
      onConnectCb = cb
    },
    getLastSequenceNum() {
      return sse.getLastSequenceNum()
    },
    droppedBatchCount: 0,  // V2 doesn't drop batches
    reportState(state) {
      ccr.reportState(state)
    },
    reportMetadata(metadata) {
      ccr.reportMetadata(metadata)
    },
    reportDelivery(eventId, status) {
      ccr.reportDelivery(eventId, status)
    },
    flush() {
      return ccr.flush()
    },
    connect() {
      if (!opts.outboundOnly) {
        void sse.connect()  // Fire-and-forget read stream
      }
      void ccr.initialize(epoch).then(
        () => {
          ccrInitialized = true
          onConnectCb?.()
        },
        (err: unknown) => {
          ccr.close()
          sse.close()
          onCloseCb?.(4091)  // 4091 = init failure
        },
      )
    },
  }
}
```

### 4.4 Transport States and Close Codes

```
┌─────────────────────────────────────────────────────────────────┐
│                    V2 Transport State Machine                    │
└─────────────────────────────────────────────────────────────────┘

                          ┌──────────────┐
                          │   CREATED    │
                          └──────┬───────┘
                                 │ connect()
                                 ▼
                    ┌────────────────────────┐
                    │    CONNECTING          │
                    │  (sse.connect() +      │
                    │   ccr.initialize())    │
                    └───────────┬────────────┘
                                │
                ┌───────────────┼───────────────┐
                │               │               │
        ccr.init success    init failure   epoch mismatch
                │               │               │
                ▼               ▼               ▼
        ┌──────────────┐  ┌──────────┐    ┌────────────┐
        │  CONNECTED   │  │  CLOSED  │    │   CLOSED   │
        │  (ready)     │  │ (4091)   │    │   (4090)   │
        └──────────────┘  └──────────┘    └────────────┘
                │
                │ SSE close (budget exhausted, 401, etc.)
                ▼
        ┌─────────────────┐
        │     CLOSED      │
        │    (4092)       │
        └─────────────────┘

Close Code Semantics:
  4090 - CCR epoch mismatch (worker superseded)
  4091 - CCR initialization failure
  4092 - SSE reconnect budget exhausted
  401  - JWT invalid/expired (triggers auth recovery)
```

---

## 5. JWT Authentication System

### 5.1 Token Refresh Scheduler (jwtUtils.ts)

The JWT refresh scheduler is a critical component that proactively refreshes session tokens before they expire:

```typescript
/** Refresh buffer: request a new token before expiry. */
const TOKEN_REFRESH_BUFFER_MS = 5 * 60 * 1000  // 5 minutes

/** Fallback refresh interval when new token's expiry is unknown. */
const FALLBACK_REFRESH_INTERVAL_MS = 30 * 60 * 1000  // 30 minutes

/** Max consecutive failures before giving up. */
const MAX_REFRESH_FAILURES = 3

/** Retry delay when getAccessToken returns undefined. */
const REFRESH_RETRY_DELAY_MS = 60_000  // 1 minute

export function createTokenRefreshScheduler({
  getAccessToken,
  onRefresh,
  label,
  refreshBufferMs = TOKEN_REFRESH_BUFFER_MS,
}: {
  getAccessToken: () => string | undefined | Promise<string | undefined>
  onRefresh: (sessionId: string, oauthToken: string) => void
  label: string
  refreshBufferMs?: number
}): {
  schedule: (sessionId: string, token: string) => void
  scheduleFromExpiresIn: (sessionId: string, expiresInSeconds: number) => void
  cancel: (sessionId: string) => void
  cancelAll: () => void
} {
  const timers = new Map<string, ReturnType<typeof setTimeout>>()
  const failureCounts = new Map<string, number>()
  const generations = new Map<string, number>()  // Generation counter for staleness detection

  function nextGeneration(sessionId: string): number {
    const gen = (generations.get(sessionId) ?? 0) + 1
    generations.set(sessionId, gen)
    return gen
  }

  function schedule(sessionId: string, token: string): void {
    const expiry = decodeJwtExpiry(token)
    if (!expiry) {
      // Token is not a decodable JWT (e.g. OAuth token)
      logForDebugging(`[${label}:token] Could not decode JWT expiry`)
      return
    }

    // Clear existing timer
    const existing = timers.get(sessionId)
    if (existing) clearTimeout(existing)

    const gen = nextGeneration(sessionId)
    const expiryDate = new Date(expiry * 1000).toISOString()
    const delayMs = expiry * 1000 - Date.now() - refreshBufferMs

    if (delayMs <= 0) {
      // Token already near expiry - refresh immediately
      logForDebugging(`[${label}:token] Token expires soon, refreshing immediately`)
      void doRefresh(sessionId, gen)
      return
    }

    logForDebugging(
      `[${label}:token] Scheduled refresh in ${formatDuration(delayMs)} (expires=${expiryDate})`,
    )

    const timer = setTimeout(doRefresh, delayMs, sessionId, gen)
    timers.set(sessionId, timer)
  }

  /** Schedule using explicit TTL (seconds until expiry). */
  function scheduleFromExpiresIn(sessionId: string, expiresInSeconds: number): void {
    const existing = timers.get(sessionId)
    if (existing) clearTimeout(existing)
    const gen = nextGeneration(sessionId)

    // Clamp to 30s floor to avoid tight-loop
    const delayMs = Math.max(expiresInSeconds * 1000 - refreshBufferMs, 30_000)

    logForDebugging(
      `[${label}:token] Scheduled refresh in ${formatDuration(delayMs)} (expires_in=${expiresInSeconds}s)`,
    )

    const timer = setTimeout(doRefresh, delayMs, sessionId, gen)
    timers.set(sessionId, timer)
  }

  async function doRefresh(sessionId: string, gen: number): Promise<void> {
    let oauthToken: string | undefined
    try {
      oauthToken = await getAccessToken()
    } catch (err) {
      logForDebugging(`[${label}:token] getAccessToken threw: ${errorMessage(err)}`, { level: 'error' })
    }

    // Check for staleness (cancelled/rescheduled while awaiting)
    if (generations.get(sessionId) !== gen) {
      logForDebugging(`[${label}:token] doRefresh stale (gen ${gen} vs ${generations.get(sessionId)}), skipping`)
      return
    }

    if (!oauthToken) {
      const failures = (failureCounts.get(sessionId) ?? 0) + 1
      failureCounts.set(sessionId, failures)

      logForDebugging(
        `[${label}:token] No OAuth token (failure ${failures}/${MAX_REFRESH_FAILURES})`,
        { level: 'error' },
      )

      if (failures < MAX_REFRESH_FAILURES) {
        // Retry after delay
        const retryTimer = setTimeout(doRefresh, REFRESH_RETRY_DELAY_MS, sessionId, gen)
        timers.set(sessionId, retryTimer)
      }
      return
    }

    // Reset failure counter on success
    failureCounts.delete(sessionId)

    logForDebugging(`[${label}:token] Refreshing token: new token prefix=${oauthToken.slice(0, 15)}…`)
    logEvent('tengu_bridge_token_refreshed', {})
    onRefresh(sessionId, oauthToken)

    // Schedule follow-up refresh for long-running sessions
    const timer = setTimeout(doRefresh, FALLBACK_REFRESH_INTERVAL_MS, sessionId, gen)
    timers.set(sessionId, timer)

    logForDebugging(
      `[${label}:token] Scheduled follow-up refresh in ${formatDuration(FALLBACK_REFRESH_INTERVAL_MS)}`,
    )
  }

  function cancel(sessionId: string): void {
    nextGeneration(sessionId)  // Invalidate in-flight refreshes
    const timer = timers.get(sessionId)
    if (timer) {
      clearTimeout(timer)
      timers.delete(sessionId)
    }
    failureCounts.delete(sessionId)
  }

  function cancelAll(): void {
    for (const sessionId of generations.keys()) {
      nextGeneration(sessionId)
    }
    for (const timer of timers.values()) {
      clearTimeout(timer)
    }
    timers.clear()
    failureCounts.clear()
  }

  return { schedule, scheduleFromExpiresIn, cancel, cancelAll }
}
```

### 5.2 JWT Payload Decoding

```typescript
/**
 * Decode a JWT's payload segment without verifying the signature.
 * Strips the `sk-ant-si-` session-ingress prefix if present.
 */
export function decodeJwtPayload(token: string): unknown | null {
  const jwt = token.startsWith('sk-ant-si-')
    ? token.slice('sk-ant-si-'.length)
    : token
  const parts = jwt.split('.')
  if (parts.length !== 3 || !parts[1]) return null
  try {
    return jsonParse(Buffer.from(parts[1], 'base64url').toString('utf8'))
  } catch {
    return null
  }
}

/**
 * Decode the `exp` (expiry) claim from a JWT.
 * @returns Unix timestamp in seconds, or null if unparseable
 */
export function decodeJwtExpiry(token: string): number | null {
  const payload = decodeJwtPayload(token)
  if (
    payload !== null &&
    typeof payload === 'object' &&
    'exp' in payload &&
    typeof payload.exp === 'number'
  ) {
    return payload.exp
  }
  return null
}
```

### 5.3 Token Refresh Flow Diagram

```
┌──────────────────────────────────────────────────────────────────────────┐
│                         JWT Refresh Timeline                              │
└──────────────────────────────────────────────────────────────────────────┘

  Session Start          Proactive Refresh         Fallback Refresh
       │                        │                        │
       │                        │                        │
       ▼                        ▼                        ▼
  ┌─────────┐            ┌─────────────┐          ┌─────────────┐
  │ Token   │────────────│ Token       │──────────│ Token       │───►
  │ Created │  ~3h55m    │ Refresh #1  │  ~4h     │ Refresh #2  │
  │ (exp:4h)│            │ (exp:+4h)   │          │ (exp:+4h)   │
  └─────────┘            └─────────────┘          └─────────────┘
       │                        │                        │
       │                        │                        │
       │◄───── 5min buffer ────►│                        │
       │                        │                        │
       │                        │◄─── 30min fallback ──►│
       │                        │                        │

Refresh Triggers:
  1. Proactive: TOKEN_REFRESH_BUFFER_MS (5min) before expiry
  2. Fallback: FALLBACK_REFRESH_INTERVAL_MS (30min) after previous refresh
  3. 401 Recovery: SSE returns 401, triggers immediate JWT refresh
  4. Laptop Wake: Overdue timer + SSE 401 fire simultaneously

Generation Counter Pattern:
  - Each schedule()/cancel() bumps generation
  - In-flight async doRefresh() checks generation on resume
  - Stale generations skip to avoid orphaned timers

Retry Logic:
  - getAccessToken failure → retry after REFRESH_RETRY_DELAY_MS (60s)
  - Max MAX_REFRESH_FAILURES (3) retries before giving up
  - Success resets failure counter
```

---

## 6. Remote Bridge Core (remoteBridgeCore.ts) - Extended

### 6.1 Environment-Less Bridge Architecture

The environment-less bridge (v2) bypasses the Environments API layer entirely, connecting directly to CCR v2 endpoints:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    V1 vs V2 Architecture Comparison                      │
└─────────────────────────────────────────────────────────────────────────┘

V1 (Environment-Based):
  ┌────────┐     ┌──────────────┐     ┌─────────────────┐     ┌──────────┐
  │  CLI   │────▶│ Environments │────▶│ Session Ingress │────▶│  Child   │
  │        │◀────│ API          │◀────│ WebSocket       │◀────│  Process │
  └────────┘     │              │     │                 │     └──────────┘
                 │ - register   │     │ - ws://         │
                 │ - poll       │     │ - control_*     │
                 │ - ack        │     │                 │
                 │ - heartbeat  │     │                 │
                 │ - stop       │     │                 │
                 └──────────────┘     └─────────────────┘

V2 (Environment-Less / Direct):
  ┌────────┐     ┌───────────────────┐     ┌──────────┐
  │  CLI   │────▶│ CCR v2 Endpoints  │────▶│  Child   │
  │        │◀────│                   │◀────│  Process │
  └────────┘     │ POST /bridge      │     └──────────┘
                 │ SSE  /events      │
                 │ POST /events      │
                 │ PUT  /state       │
                 │ POST /heartbeat   │
                 └───────────────────┘

Key Differences:
  - V2: No Environments API poll/dispatch layer
  - V2: Direct JWT authentication (session_ingress_token)
  - V2: SSE for reads, HTTP POST for writes
  - V2: Epoch-based concurrency control
  - V2: Heartbeat-driven lease management
```

### 6.2 Retry with Exponential Backoff

```typescript
/** Retry an async init call with exponential backoff + jitter. */
async function withRetry<T>(
  fn: () => Promise<T | null>,
  label: string,
  cfg: EnvLessBridgeConfig,
): Promise<T | null> {
  const max = cfg.init_retry_max_attempts
  for (let attempt = 1; attempt <= max; attempt++) {
    const result = await fn()
    if (result !== null) return result
    if (attempt < max) {
      const base = cfg.init_retry_base_delay_ms * 2 ** (attempt - 1)
      const jitter =
        base * cfg.init_retry_jitter_fraction * (2 * Math.random() - 1)
      const delay = Math.min(base + jitter, cfg.init_retry_max_delay_ms)
      logForDebugging(
        `[remote-bridge] ${label} failed (attempt ${attempt}/${max}), retrying in ${Math.round(delay)}ms`,
      )
      await sleep(delay)
    }
  }
  return null
}
```

### 6.3 Archive Session on Teardown

```typescript
async function archiveSession(
  sessionId: string,
  baseUrl: string,
  accessToken: string | undefined,
  orgUUID: string,
  timeoutMs: number,
): Promise<ArchiveStatus> {
  if (!accessToken) return 'no_token'

  // Archive lives at the compat layer (/v1/sessions/*, not /v1/code/sessions)
  // compat.parseSessionID only accepts TagSession (session_*), so retag cse_*
  const compatId = toCompatSessionId(sessionId)

  try {
    const response = await axios.post(
      `${baseUrl}/v1/sessions/${compatId}/archive`,
      {},
      {
        headers: {
          ...oauthHeaders(accessToken),
          'anthropic-beta': 'ccr-byoc-2025-07-29',
          'x-organization-uuid': orgUUID,
        },
        timeout: timeoutMs,
        validateStatus: () => true,
      },
    )
    logForDebugging(`[remote-bridge] Archive ${compatId} status=${response.status}`)
    return response.status
  } catch (err) {
    const msg = errorMessage(err)
    logForDebugging(`[remote-bridge] Archive failed: ${msg}`)
    return axios.isAxiosError(err) && err.code === 'ECONNABORTED'
      ? 'timeout'
      : 'error'
  }
}

type ArchiveStatus = number | 'timeout' | 'error' | 'no_token'
type ArchiveTelemetryStatus =
  | 'ok'
  | 'skipped_no_token'
  | 'network_error'
  | 'server_4xx'
  | 'server_5xx'
```

---

## 7. Bridge API Client (bridgeApi.ts)

### 7.1 API Client Interface

```typescript
export type BridgeApiClient = {
  registerBridgeEnvironment(config: BridgeConfig): Promise<{
    environment_id: string
    environment_secret: string
  }>
  pollForWork(
    environmentId: string,
    environmentSecret: string,
    signal?: AbortSignal,
    reclaimOlderThanMs?: number,
  ): Promise<WorkResponse | null>
  acknowledgeWork(
    environmentId: string,
    workId: string,
    sessionToken: string,
  ): Promise<void>
  stopWork(environmentId: string, workId: string, force: boolean): Promise<void>
  deregisterEnvironment(environmentId: string): Promise<void>
  sendPermissionResponseEvent(
    sessionId: string,
    event: PermissionResponseEvent,
    sessionToken: string,
  ): Promise<void>
  archiveSession(sessionId: string): Promise<void>
  reconnectSession(environmentId: string, sessionId: string): Promise<void>
  heartbeatWork(
    environmentId: string,
    workId: string,
    sessionToken: string,
  ): Promise<{ lease_extended: boolean; state: string }>
}
```

### 7.2 API Client Implementation Details

```typescript
type BridgeApiDeps = {
  baseUrl: string
  getAccessToken: () => string | undefined
  runnerVersion: string
  onDebug?: (msg: string) => void
  onAuth401?: (staleAccessToken: string) => Promise<boolean>
  getTrustedDeviceToken?: () => string | undefined
}

const BETA_HEADER = 'environments-2025-11-01'
const SAFE_ID_PATTERN = /^[a-zA-Z0-9_-]+$/

export function validateBridgeId(id: string, label: string): string {
  if (!id || !SAFE_ID_PATTERN.test(id)) {
    throw new Error(`Invalid ${label}: contains unsafe characters`)
  }
  return id
}

export class BridgeFatalError extends Error {
  readonly status: number
  readonly errorType: string | undefined

  constructor(message: string, status: number, errorType?: string) {
    super(message)
    this.name = 'BridgeFatalError'
    this.status = status
    this.errorType = errorType
  }
}

export function createBridgeApiClient(deps: BridgeApiDeps): BridgeApiClient {
  function debug(msg: string): void {
    deps.onDebug?.(msg)
  }

  let consecutiveEmptyPolls = 0
  const EMPTY_POLL_LOG_INTERVAL = 100

  function getHeaders(accessToken: string): Record<string, string> {
    const headers: Record<string, string> = {
      Authorization: `Bearer ${accessToken}`,
      'Content-Type': 'application/json',
      'anthropic-version': '2023-06-01',
      'anthropic-beta': BETA_HEADER,
      'x-environment-runner-version': deps.runnerVersion,
    }
    const deviceToken = deps.getTrustedDeviceToken?.()
    if (deviceToken) {
      headers['X-Trusted-Device-Token'] = deviceToken
    }
    return headers
  }

  function resolveAuth(): string {
    const accessToken = deps.getAccessToken()
    if (!accessToken) {
      throw new Error(BRIDGE_LOGIN_INSTRUCTION)
    }
    return accessToken
  }

  /** OAuth retry wrapper - retries once after 401 token refresh. */
  async function withOAuthRetry<T>(
    fn: (accessToken: string) => Promise<{ status: number; data: T }>,
    context: string,
  ): Promise<{ status: number; data: T }> {
    const accessToken = resolveAuth()
    const response = await fn(accessToken)

    if (response.status !== 401) {
      return response
    }

    if (!deps.onAuth401) {
      debug(`[bridge:api] ${context}: 401 received, no refresh handler`)
      return response
    }

    debug(`[bridge:api] ${context}: 401 received, attempting token refresh`)
    const refreshed = await deps.onAuth401(accessToken)

    if (refreshed) {
      debug(`[bridge:api] ${context}: Token refreshed, retrying request`)
      const newToken = resolveAuth()
      const retryResponse = await fn(newToken)
      if (retryResponse.status !== 401) {
        return retryResponse
      }
      debug(`[bridge:api] ${context}: Retry after refresh also got 401`)
    } else {
      debug(`[bridge:api] ${context}: Token refresh failed`)
    }

    return response
  }

  return {
    async registerBridgeEnvironment(config: BridgeConfig): Promise<{
      environment_id: string
      environment_secret: string
    }> {
      debug(`[bridge:api] POST /v1/environments/bridge bridgeId=${config.bridgeId}`)

      const response = await withOAuthRetry(
        (token: string) =>
          axios.post(
            `${deps.baseUrl}/v1/environments/bridge`,
            {
              machine_name: config.machineName,
              directory: config.dir,
              branch: config.branch,
              git_repo_url: config.gitRepoUrl,
              max_sessions: config.maxSessions,
              metadata: { worker_type: config.workerType },
              ...(config.reuseEnvironmentId && {
                environment_id: config.reuseEnvironmentId,
              }),
            },
            {
              headers: getHeaders(token),
              timeout: 15_000,
              validateStatus: status => status < 500,
            },
          ),
        'Registration',
      )

      handleErrorStatus(response.status, response.data, 'Registration')
      debug(`[bridge:api] Registration -> ${response.status} environment_id=${response.data.environment_id}`)
      return response.data
    },

    async pollForWork(
      environmentId: string,
      environmentSecret: string,
      signal?: AbortSignal,
      reclaimOlderThanMs?: number,
    ): Promise<WorkResponse | null> {
      validateBridgeId(environmentId, 'environmentId')

      const prevEmptyPolls = consecutiveEmptyPolls
      consecutiveEmptyPolls = 0

      const response = await axios.get<WorkResponse | null>(
        `${deps.baseUrl}/v1/environments/${environmentId}/work/poll`,
        {
          headers: getHeaders(environmentSecret),
          params: reclaimOlderThanMs !== undefined
            ? { reclaim_older_than_ms: reclaimOlderThanMs }
            : undefined,
          timeout: 10_000,
          signal,
          validateStatus: status => status < 500,
        },
      )

      handleErrorStatus(response.status, response.data, 'Poll')

      if (!response.data) {
        consecutiveEmptyPolls = prevEmptyPolls + 1
        if (consecutiveEmptyPolls === 1 || consecutiveEmptyPolls % EMPTY_POLL_LOG_INTERVAL === 0) {
          debug(`[bridge:api] Poll -> ${response.status} (no work, ${consecutiveEmptyPolls} consecutive)`)
        }
        return null
      }

      debug(
        `[bridge:api] Poll -> ${response.status} workId=${response.data.id} sessionId=${response.data.data?.id}`,
      )
      return response.data
    },

    async acknowledgeWork(
      environmentId: string,
      workId: string,
      sessionToken: string,
    ): Promise<void> {
      validateBridgeId(environmentId, 'environmentId')
      validateBridgeId(workId, 'workId')
      debug(`[bridge:api] POST .../work/${workId}/ack`)

      const response = await axios.post(
        `${deps.baseUrl}/v1/environments/${environmentId}/work/${workId}/ack`,
        {},
        {
          headers: getHeaders(sessionToken),
          timeout: 10_000,
          validateStatus: s => s < 500,
        },
      )

      handleErrorStatus(response.status, response.data, 'Acknowledge')
      debug(`[bridge:api] Ack -> ${response.status}`)
    },

    async stopWork(
      environmentId: string,
      workId: string,
      force: boolean,
    ): Promise<void> {
      validateBridgeId(environmentId, 'environmentId')
      validateBridgeId(workId, 'workId')
      debug(`[bridge:api] POST .../work/${workId}/stop force=${force}`)

      const response = await withOAuthRetry(
        (token: string) =>
          axios.post(
            `${deps.baseUrl}/v1/environments/${environmentId}/work/${workId}/stop`,
            { force },
            {
              headers: getHeaders(token),
              timeout: 10_000,
              validateStatus: s => s < 500,
            },
          ),
        'StopWork',
      )

      handleErrorStatus(response.status, response.data, 'StopWork')
      debug(`[bridge:api] StopWork -> ${response.status}`)
    },

    async heartbeatWork(
      environmentId: string,
      workId: string,
      sessionToken: string,
    ): Promise<{ lease_extended: boolean; state: string }> {
      validateBridgeId(environmentId, 'environmentId')
      validateBridgeId(workId, 'workId')
      debug(`[bridge:api] POST .../work/${workId}/heartbeat`)

      const response = await axios.post<{
        lease_extended: boolean
        state: string
        last_heartbeat: string
        ttl_seconds: number
      }>(
        `${deps.baseUrl}/v1/environments/${environmentId}/work/${workId}/heartbeat`,
        {},
        {
          headers: getHeaders(sessionToken),
          timeout: 10_000,
          validateStatus: s => s < 500,
        },
      )

      handleErrorStatus(response.status, response.data, 'Heartbeat')
      debug(
        `[bridge:api] Heartbeat -> ${response.status} lease_extended=${response.data.lease_extended} state=${response.data.state}`,
      )
      return response.data
    },

    async reconnectSession(
      environmentId: string,
      sessionId: string,
    ): Promise<void> {
      validateBridgeId(environmentId, 'environmentId')
      validateBridgeId(sessionId, 'sessionId')
      debug(`[bridge:api] POST .../bridge/reconnect session_id=${sessionId}`)

      const response = await withOAuthRetry(
        (token: string) =>
          axios.post(
            `${deps.baseUrl}/v1/environments/${environmentId}/bridge/reconnect`,
            { session_id: sessionId },
            {
              headers: getHeaders(token),
              timeout: 10_000,
              validateStatus: s => s < 500,
            },
          ),
        'ReconnectSession',
      )

      handleErrorStatus(response.status, response.data, 'ReconnectSession')
      debug(`[bridge:api] ReconnectSession -> ${response.status}`)
    },
  }
}

function handleErrorStatus(status: number, data: unknown, context: string): void {
  if (status === 200 || status === 204) {
    return
  }

  const detail = extractErrorDetail(data)
  const errorType = extractErrorTypeFromData(data)

  switch (status) {
    case 401:
      throw new BridgeFatalError(
        `${context}: Authentication failed (401)${detail ? `: ${detail}` : ''}. ${BRIDGE_LOGIN_INSTRUCTION}`,
        401,
        errorType,
      )
    case 403:
      throw new BridgeFatalError(
        isExpiredErrorType(errorType)
          ? 'Remote Control session has expired.'
          : `${context}: Access denied (403)${detail ? `: ${detail}` : ''}`,
        403,
        errorType,
      )
    case 404:
      throw new BridgeFatalError(
        detail ?? `${context}: Not found (404)`,
        404,
        errorType,
      )
    case 410:
      throw new BridgeFatalError(
        detail ?? 'Remote Control session has expired.',
        410,
        errorType ?? 'environment_expired',
      )
    case 429:
      throw new Error(`${context}: Rate limited (429)`)
    default:
      throw new Error(`${context}: Failed with status ${status}${detail ? `: ${detail}` : ''}`)
  }
}

export function isExpiredErrorType(errorType: string | undefined): boolean {
  if (!errorType) return false
  return errorType.includes('expired') || errorType.includes('lifetime')
}

export function isSuppressible403(err: BridgeFatalError): boolean {
  if (err.status !== 403) return false
  return (
    err.message.includes('external_poll_sessions') ||
    err.message.includes('environments:manage')
  )
}
```

---

## 8. Session Management

### 8.1 Session Handle Interface

```typescript
export type SessionHandle = {
  sessionId: string
  done: Promise<SessionDoneStatus>  // 'completed' | 'failed' | 'interrupted'
  kill(): void
  forceKill(): void
  activities: SessionActivity[]  // Ring buffer of last ~10 activities
  currentActivity: SessionActivity | null
  accessToken: string
  lastStderr: string[]  // Ring buffer of last stderr lines
  writeStdin(data: string): void
  updateAccessToken(token: string): void
}

export type SessionActivity = {
  type: SessionActivityType  // 'tool_start' | 'text' | 'result' | 'error'
  summary: string  // e.g. "Editing src/foo.ts", "Reading package.json"
  timestamp: number
}

export type SessionSpawnOpts = {
  sessionId: string
  sdkUrl: string
  accessToken: string
  useCcrV2?: boolean
  workerEpoch?: number
  onFirstUserMessage?: (text: string) => void
}
```

### 8.2 Session Spawner Implementation

```typescript
const MAX_ACTIVITIES = 10
const MAX_STDERR_LINES = 10

const TOOL_VERBS: Record<string, string> = {
  Read: 'Reading',
  Write: 'Writing',
  Edit: 'Editing',
  MultiEdit: 'Editing',
  Bash: 'Running',
  Glob: 'Searching',
  Grep: 'Searching',
  WebFetch: 'Fetching',
  WebSearch: 'Searching',
  Task: 'Running task',
  // ... more tools
}

function toolSummary(name: string, input: Record<string, unknown>): string {
  const verb = TOOL_VERBS[name] ?? name
  const target =
    (input.file_path as string) ??
    (input.filePath as string) ??
    (input.pattern as string) ??
    (input.command as string | undefined)?.slice(0, 60) ??
    ''
  if (target) return `${verb} ${target}`
  return verb
}

export function createSessionSpawner(deps: SessionSpawnerDeps): SessionSpawner {
  return {
    spawn(opts: SessionSpawnOpts, dir: string): SessionHandle {
      const safeId = safeFilenameId(opts.sessionId)
      let debugFile: string | undefined

      // Determine debug file path
      if (deps.debugFile) {
        const ext = deps.debugFile.lastIndexOf('.')
        debugFile = ext > 0
          ? `${deps.debugFile.slice(0, ext)}-${safeId}${deps.debugFile.slice(ext)}`
          : `${deps.debugFile}-${safeId}`
      } else if (deps.verbose || process.env.USER_TYPE === 'ant') {
        debugFile = join(tmpdir(), 'claude', `bridge-session-${safeId}.log`)
      }

      // Create transcript stream for raw NDJSON
      let transcriptStream: WriteStream | null = null
      if (debugFile) {
        transcriptStream = createWriteStream(
          join(dirname(debugFile), `bridge-transcript-${safeId}.jsonl`),
          { flags: 'a' }
        )
      }

      // Build child process args
      const args = [
        ...deps.scriptArgs,
        '--print',
        '--sdk-url',
        opts.sdkUrl,
        '--session-id',
        opts.sessionId,
        '--input-format',
        'stream-json',
        '--output-format',
        'stream-json',
        '--replay-user-messages',
        ...(deps.verbose ? ['--verbose'] : []),
        ...(debugFile ? ['--debug-file', debugFile] : []),
        ...(deps.permissionMode ? ['--permission-mode', deps.permissionMode] : []),
      ]

      // Build environment variables
      const env: NodeJS.ProcessEnv = {
        ...deps.env,
        CLAUDE_CODE_OAUTH_TOKEN: undefined,  // Strip OAuth - use session token
        CLAUDE_CODE_ENVIRONMENT_KIND: 'bridge',
        ...(deps.sandbox && { CLAUDE_CODE_FORCE_SANDBOX: '1' }),
        CLAUDE_CODE_SESSION_ACCESS_TOKEN: opts.accessToken,
        CLAUDE_CODE_POST_FOR_SESSION_INGRESS_V2: '1',
        ...(opts.useCcrV2 && {
          CLAUDE_CODE_USE_CCR_V2: '1',
          CLAUDE_CODE_WORKER_EPOCH: String(opts.workerEpoch),
        }),
      }

      deps.onDebug(`[bridge:session] Spawning sessionId=${opts.sessionId}`)
      deps.onDebug(`[bridge:session] Child args: ${args.join(' ')}`)

      const child: ChildProcess = spawn(deps.execPath, args, {
        cwd: dir,
        stdio: ['pipe', 'pipe', 'pipe'],
        env,
        windowsHide: true,
      })

      deps.onDebug(`[bridge:session] sessionId=${opts.sessionId} pid=${child.pid}`)

      const activities: SessionActivity[] = []
      let currentActivity: SessionActivity | null = null
      const lastStderr: string[] = []
      let sigkillSent = false
      let firstUserMessageSeen = false

      // Buffer stderr lines
      if (child.stderr) {
        const stderrRl = createInterface({ input: child.stderr })
        stderrRl.on('line', line => {
          if (deps.verbose) process.stderr.write(line + '\n')
          if (lastStderr.length >= MAX_STDERR_LINES) lastStderr.shift()
          lastStderr.push(line)
        })
      }

      // Parse NDJSON from stdout
      if (child.stdout) {
        const rl = createInterface({ input: child.stdout })
        rl.on('line', line => {
          // Write to transcript
          if (transcriptStream) transcriptStream.write(line + '\n')

          deps.onDebug(`[bridge:ws] <<< ${debugTruncate(line)}`)
          if (deps.verbose) process.stderr.write(line + '\n')

          // Extract activities
          const extracted = extractActivities(line, opts.sessionId, deps.onDebug)
          for (const activity of extracted) {
            if (activities.length >= MAX_ACTIVITIES) activities.shift()
            activities.push(activity)
            currentActivity = activity
            deps.onActivity?.(opts.sessionId, activity)
          }

          // Detect control_request and user messages
          let parsed: unknown
          try { parsed = jsonParse(line) } catch { /* skip */ }

          if (parsed && typeof parsed === 'object') {
            const msg = parsed as Record<string, unknown>

            if (msg.type === 'control_request') {
              const request = msg.request as Record<string, unknown> | undefined
              if (request?.subtype === 'can_use_tool' && deps.onPermissionRequest) {
                deps.onPermissionRequest(
                  opts.sessionId,
                  parsed as PermissionRequest,
                  opts.accessToken
                )
              }
            } else if (
              msg.type === 'user' &&
              !firstUserMessageSeen &&
              opts.onFirstUserMessage
            ) {
              const text = extractUserMessageText(msg)
              if (text) {
                firstUserMessageSeen = true
                opts.onFirstUserMessage(text)
              }
            }
          }
        })
      }

      // Create done promise
      const done = new Promise<SessionDoneStatus>(resolve => {
        child.on('close', (code, signal) => {
          if (transcriptStream) transcriptStream.end()

          if (signal === 'SIGTERM' || signal === 'SIGINT') {
            deps.onDebug(`[bridge:session] sessionId=${opts.sessionId} interrupted signal=${signal}`)
            resolve('interrupted')
          } else if (code === 0) {
            deps.onDebug(`[bridge:session] sessionId=${opts.sessionId} completed exit_code=0`)
            resolve('completed')
          } else {
            deps.onDebug(`[bridge:session] sessionId=${opts.sessionId} failed exit_code=${code}`)
            resolve('failed')
          }
        })

        child.on('error', err => {
          deps.onDebug(`[bridge:session] sessionId=${opts.sessionId} spawn error: ${err.message}`)
          resolve('failed')
        })
      })

      return {
        sessionId: opts.sessionId,
        done,
        activities,
        accessToken: opts.accessToken,
        lastStderr,
        get currentActivity(): SessionActivity | null {
          return currentActivity
        },
        kill(): void {
          if (!child.killed) {
            deps.onDebug(`[bridge:session] Sending SIGTERM to sessionId=${opts.sessionId}`)
            if (process.platform === 'win32') {
              child.kill()
            } else {
              child.kill('SIGTERM')
            }
          }
        },
        forceKill(): void {
          if (!sigkillSent && child.pid) {
            sigkillSent = true
            deps.onDebug(`[bridge:session] Sending SIGKILL to sessionId=${opts.sessionId}`)
            if (process.platform === 'win32') {
              child.kill()
            } else {
              child.kill('SIGKILL')
            }
          }
        },
        writeStdin(data: string): void {
          if (child.stdin && !child.stdin.destroyed) {
            deps.onDebug(`[bridge:ws] >>> ${debugTruncate(data)}`)
            child.stdin.write(data)
          }
        },
        updateAccessToken(token: string): void {
          this.accessToken = token
          this.writeStdin(
            jsonStringify({
              type: 'update_environment_variables',
              variables: { CLAUDE_CODE_SESSION_ACCESS_TOKEN: token },
            }) + '\n',
          )
          deps.onDebug(`[bridge:session] Sent token refresh via stdin for sessionId=${opts.sessionId}`)
        },
      }
    },
  }
}
```

### 8.3 Activity Extraction

```typescript
function extractActivities(
  line: string,
  sessionId: string,
  onDebug: (msg: string) => void,
): SessionActivity[] {
  let parsed: unknown
  try {
    parsed = jsonParse(line)
  } catch {
    return []
  }

  if (!parsed || typeof parsed !== 'object') return []

  const msg = parsed as Record<string, unknown>
  const activities: SessionActivity[] = []
  const now = Date.now()

  switch (msg.type) {
    case 'assistant': {
      const message = msg.message as Record<string, unknown> | undefined
      if (!message) break
      const content = message.content
      if (!Array.isArray(content)) break

      for (const block of content) {
        if (!block || typeof block !== 'object') continue
        const b = block as Record<string, unknown>

        if (b.type === 'tool_use') {
          const name = (b.name as string) ?? 'Tool'
          const input = (b.input as Record<string, unknown>) ?? {}
          const summary = toolSummary(name, input)
          activities.push({
            type: 'tool_start',
            summary,
            timestamp: now,
          })
          onDebug(`[bridge:activity] sessionId=${sessionId} tool_use name=${name}`)
        } else if (b.type === 'text') {
          const text = (b.text as string) ?? ''
          if (text.length > 0) {
            activities.push({
              type: 'text',
              summary: text.slice(0, 80),
              timestamp: now,
            })
          }
        }
      }
      break
    }
    case 'result': {
      const subtype = msg.subtype as string | undefined
      if (subtype === 'success') {
        activities.push({
          type: 'result',
          summary: 'Session completed',
          timestamp: now,
        })
      } else if (subtype) {
        const errors = msg.errors as string[] | undefined
        const errorSummary = errors?.[0] ?? `Error: ${subtype}`
        activities.push({
          type: 'error',
          summary: errorSummary,
          timestamp: now,
        })
      }
      break
    }
  }

  return activities
}

function extractUserMessageText(msg: Record<string, unknown>): string | undefined {
  // Skip tool-result user messages and synthetic caveat messages
  if (msg.parent_tool_use_id != null || msg.isSynthetic || msg.isReplay) return undefined

  const message = msg.message as Record<string, unknown> | undefined
  const content = message?.content
  let text: string | undefined

  if (typeof content === 'string') {
    text = content
  } else if (Array.isArray(content)) {
    for (const block of content) {
      if (
        block &&
        typeof block === 'object' &&
        (block as Record<string, unknown>).type === 'text'
      ) {
        text = (block as Record<string, unknown>).text as string | undefined
        break
      }
    }
  }

  text = text?.trim()
  return text ? text : undefined
}
```

---

## 9. Message Protocol (bridgeMessaging.ts)

### 9.1 Message Type Guards

```typescript
/** Type predicate for parsed WebSocket messages. */
export function isSDKMessage(value: unknown): value is SDKMessage {
  return (
    value !== null &&
    typeof value === 'object' &&
    'type' in value &&
    typeof value.type === 'string'
  )
}

/** Type predicate for control_response messages from the server. */
export function isSDKControlResponse(value: unknown): value is SDKControlResponse {
  return (
    value !== null &&
    typeof value === 'object' &&
    'type' in value &&
    value.type === 'control_response' &&
    'response' in value
  )
}

/** Type predicate for control_request messages from the server. */
export function isSDKControlRequest(value: unknown): value is SDKControlRequest {
  return (
    value !== null &&
    typeof value === 'object' &&
    'type' in value &&
    value.type === 'control_request' &&
    'request_id' in value &&
    'request' in value
  )
}

/**
 * True for message types that should be forwarded to the bridge transport.
 */
export function isEligibleBridgeMessage(m: Message): boolean {
  // Virtual messages (REPL inner calls) are display-only
  if ((m.type === 'user' || m.type === 'assistant') && m.isVirtual) {
    return false
  }
  return (
    m.type === 'user' ||
    m.type === 'assistant' ||
    (m.type === 'system' && m.subtype === 'local_command')
  )
}
```

### 9.2 Title Text Extraction

```typescript
/**
 * Extract title-worthy text from a Message for onUserMessage.
 * Returns undefined for non-user, meta, tool results, or synthetic messages.
 */
export function extractTitleText(m: Message): string | undefined {
  if (m.type !== 'user' || m.isMeta || m.toolUseResult || m.isCompactSummary) {
    return undefined
  }
  if (m.origin && m.origin.kind !== 'human') return undefined

  const content = m.message.content
  let raw: string | undefined

  if (typeof content === 'string') {
    raw = content
  } else {
    for (const block of content) {
      if (block.type === 'text') {
        raw = block.text
        break
      }
    }
  }

  if (!raw) return undefined
  const clean = stripDisplayTagsAllowEmpty(raw)
  return clean || undefined
}
```

### 9.3 Ingress Message Handler

```typescript
export function handleIngressMessage(
  data: string,
  recentPostedUUIDs: BoundedUUIDSet,
  recentInboundUUIDs: BoundedUUIDSet,
  onInboundMessage: ((msg: SDKMessage) => void | Promise<void>) | undefined,
  onPermissionResponse?: ((response: SDKControlResponse) => void) | undefined,
  onControlRequest?: ((request: SDKControlRequest) => void) | undefined,
): void {
  try {
    const parsed: unknown = normalizeControlMessageKeys(jsonParse(data))

    // control_response is not an SDKMessage
    if (isSDKControlResponse(parsed)) {
      logForDebugging('[bridge:repl] Ingress message type=control_response')
      onPermissionResponse?.(parsed)
      return
    }

    // control_request from server
    if (isSDKControlRequest(parsed)) {
      logForDebugging(
        `[bridge:repl] Inbound control_request subtype=${parsed.request.subtype}`,
      )
      onControlRequest?.(parsed)
      return
    }

    if (!isSDKMessage(parsed)) return

    // Check for UUID echo
    const uuid = 'uuid' in parsed && typeof parsed.uuid === 'string'
      ? parsed.uuid
      : undefined

    if (uuid && recentPostedUUIDs.has(uuid)) {
      logForDebugging(`[bridge:repl] Ignoring echo: type=${parsed.type} uuid=${uuid}`)
      return
    }

    // Defensive dedup for re-delivered inbound prompts
    if (uuid && recentInboundUUIDs.has(uuid)) {
      logForDebugging(
        `[bridge:repl] Ignoring re-delivered inbound: type=${parsed.type} uuid=${uuid}`,
      )
      return
    }

    logForDebugging(
      `[bridge:repl] Ingress message type=${parsed.type}${uuid ? ` uuid=${uuid}` : ''}`,
    )

    if (parsed.type === 'user') {
      if (uuid) recentInboundUUIDs.add(uuid)
      logEvent('tengu_bridge_message_received', { is_repl: true })
      void onInboundMessage?.(parsed)
    } else {
      logForDebugging(`[bridge:repl] Ignoring non-user inbound: type=${parsed.type}`)
    }
  } catch (err) {
    logForDebugging(`[bridge:repl] Failed to parse ingress message: ${errorMessage(err)}`)
  }
}
```

### 9.4 Server Control Request Handler

```typescript
export type ServerControlRequestHandlers = {
  transport: ReplBridgeTransport | null
  sessionId: string
  outboundOnly?: boolean
  onInterrupt?: () => void
  onSetModel?: (model: string | undefined) => void
  onSetMaxThinkingTokens?: (maxTokens: number | null) => void
  onSetPermissionMode?: (
    mode: PermissionMode
  ) => { ok: true } | { ok: false; error: string }
}

const OUTBOUND_ONLY_ERROR =
  'This session is outbound-only. Enable Remote Control locally to allow inbound control.'

export function handleServerControlRequest(
  request: SDKControlRequest,
  handlers: ServerControlRequestHandlers,
): void {
  const {
    transport,
    sessionId,
    outboundOnly,
    onInterrupt,
    onSetModel,
    onSetMaxThinkingTokens,
    onSetPermissionMode,
  } = handlers

  if (!transport) {
    logForDebugging('[bridge:repl] Cannot respond to control_request: transport not configured')
    return
  }

  let response: SDKControlResponse

  // Outbound-only: reply error for mutable requests
  if (outboundOnly && request.request.subtype !== 'initialize') {
    response = {
      type: 'control_response',
      response: {
        subtype: 'error',
        request_id: request.request_id,
        error: OUTBOUND_ONLY_ERROR,
      },
    }
    const event = { ...response, session_id: sessionId }
    void transport.write(event)
    logForDebugging(
      `[bridge:repl] Rejected ${request.request.subtype} (outbound-only) request_id=${request.request_id}`,
    )
    return
  }

  switch (request.request.subtype) {
    case 'initialize':
      response = {
        type: 'control_response',
        response: {
          subtype: 'success',
          request_id: request.request_id,
          response: {
            commands: [],
            output_style: 'normal',
            available_output_styles: ['normal'],
            models: [],
            account: {},
            pid: process.pid,
          },
        },
      }
      break

    case 'set_model':
      onSetModel?.(request.request.model)
      response = {
        type: 'control_response',
        response: {
          subtype: 'success',
          request_id: request.request_id,
        },
      }
      break

    case 'set_max_thinking_tokens':
      onSetMaxThinkingTokens?.(request.request.max_thinking_tokens)
      response = {
        type: 'control_response',
        response: {
          subtype: 'success',
          request_id: request.request_id,
        },
      }
      break

    case 'set_permission_mode': {
      const verdict = onSetPermissionMode?.(request.request.mode) ?? {
        ok: false,
        error: 'set_permission_mode is not supported in this context',
      }
      if (verdict.ok) {
        response = {
          type: 'control_response',
          response: {
            subtype: 'success',
            request_id: request.request_id,
          },
        }
      } else {
        response = {
          type: 'control_response',
          response: {
            subtype: 'error',
            request_id: request.request_id,
            error: verdict.error,
          },
        }
      }
      break
    }

    case 'interrupt':
      onInterrupt?.()
      response = {
        type: 'control_response',
        response: {
          subtype: 'success',
          request_id: request.request_id,
        },
      }
      break

    default:
      response = {
        type: 'control_response',
        response: {
          subtype: 'error',
          request_id: request.request_id,
          error: `REPL bridge does not handle control_request subtype: ${request.request.subtype}`,
        },
      }
  }

  const event = { ...response, session_id: sessionId }
  void transport.write(event)
  logForDebugging(
    `[bridge:repl] Sent control_response for ${request.request.subtype} request_id=${request.request_id}`,
  )
}
```

### 9.5 Bounded UUID Set (Echo Dedup)

```typescript
/**
 * FIFO-bounded set backed by a circular buffer.
 * Evicts oldest entry when capacity is reached.
 */
export class BoundedUUIDSet {
  private readonly capacity: number
  private readonly ring: (string | undefined)[]
  private readonly set = new Set<string>()
  private writeIdx = 0

  constructor(capacity: number) {
    this.capacity = capacity
    this.ring = new Array<string | undefined>(capacity)
  }

  add(uuid: string): void {
    if (this.set.has(uuid)) return

    // Evict oldest entry
    const evicted = this.ring[this.writeIdx]
    if (evicted !== undefined) {
      this.set.delete(evicted)
    }

    this.ring[this.writeIdx] = uuid
    this.set.add(uuid)
    this.writeIdx = (this.writeIdx + 1) % this.capacity
  }

  has(uuid: string): boolean {
    return this.set.has(uuid)
  }

  clear(): void {
    this.set.clear()
    this.ring.fill(undefined)
    this.writeIdx = 0
  }
}
```

### 9.6 Result Message Builder

```typescript
/**
 * Build a minimal SDKResultSuccess message for session archival.
 * The server needs this event before WS close to trigger archival.
 */
export function makeResultMessage(sessionId: string): SDKResultSuccess {
  return {
    type: 'result',
    subtype: 'success',
    duration_ms: 0,
    duration_api_ms: 0,
    is_error: false,
    num_turns: 0,
    result: '',
    stop_reason: null,
    total_cost_usd: 0,
    usage: { ...EMPTY_USAGE },
    modelUsage: {},
    permission_denials: [],
    session_id: sessionId,
    uuid: randomUUID(),
  }
}
```

---

## 10. Complete Message Flow

### 10.1 Full Message Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                         COMPLETE MESSAGE FLOW DIAGRAM                            │
└─────────────────────────────────────────────────────────────────────────────────┘

User on claude.ai types "Hello"
           │
           │ 1. HTTPS POST
           ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           Anthropic Backend                                      │
│                                                                                  │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────────────────┐  │
│  │  Session Store  │───▶│  Work Dispatcher│───▶│  Event Queue                │  │
│  │                 │    │                 │    │                             │  │
│  │  - session_id   │    │  - finds idle   │    │  - user message events      │  │
│  │  - state        │    │    worker       │    │  - control_requests         │  │
│  │  - cursor       │    │  - marks ACK'd  │    │  - system events            │  │
│  └─────────────────┘    └─────────────────┘    └─────────────────────────────┘  │
│                              │                              │                    │
│                              │ 2. Work poll response        │ 3. SSE stream      │
│                              │    (work_id, JWT)            │    delivery        │
└──────────────────────────────┼──────────────────────────────┼────────────────────┘
                               │                              │
                               ▼                              ▼
                    ┌──────────────────┐            ┌──────────────────┐
                    │  Poll Loop       │            │  SSE Transport   │
                    │  (bridgeMain)    │            │  (replBridge)    │
                    │                  │            │                  │
                    │  GET /work/poll  │◀───────────│  onData()        │
                    │                  │            │  parses JSON     │
                    └────────┬─────────┘            └────────┬─────────┘
                             │                               │
                             │ 4. ACK work                   │
                             ▼                               │
                    ┌──────────────────┐                     │
                    │  POST /ack       │                     │
                    │                  │                     │
                    │  Marks work as   │                     │
                    │  claimed         │                     │
                    └──────────────────┘                     │
                             │                               │
                             │ 5. Spawn child process        │
                             │    claude --sdk-url           │
                             ▼                               │
                    ┌──────────────────┐                     │
                    │  Child Process   │                     │
                    │  (claude CLI)    │                     │
                    │                  │◀────────────────────┤
                    │  - stdin write   │    6. Write batch   │
                    │  - stdout read   │     {type:'user',   │
                    │                  │    content:'Hello'} │
                    └────────┬─────────┘                     │
                             │                               │
                             │ 7. Process message            │
                             │    - Run inference            │
                             │    - Generate response        │
                             │                               │
                             │ 8. Output NDJSON              │
                             ▼                               │
                    ┌──────────────────┐                     │
                    │  stdout          │                     │
                    │                  │─────────────────────┤
                    │  {type:'assistant',                    │
                    │   content:[{type:'text',               │
                    │             text:'Hi there!'}]}        │
                    └──────────────────┘                     │
                             │                               │
                             │ 9. Write to transport         │
                             │    transport.writeBatch()     │
                             ▼                               │
                    ┌──────────────────┐                     │
                    │  CCRClient       │                     │
                    │  (v2) / Hybrid   │                     │
                    │                  │                     │
                    │  POST /events    │                     │
                    └────────┬─────────┘                     │
                             │                               │
                             │ 10. HTTPS POST                │
                             ▼                               │
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           Anthropic Backend                                      │
│                                                                                  │
│                              ┌─────────────────────────────────┐                │
│                              │  Event Store                     │                │
│                              │                                 │                │
│                              │  - Stores assistant response    │                │
│                              │  - Updates session state        │                │
│                              │  - Pushes to web UI via WS      │                │
│                              └─────────────────────────────────┘                │
│                                          │                                       │
│                                          │ 11. WebSocket push                   │
└──────────────────────────────────────────┼───────────────────────────────────────┘
                                           │
                                           ▼
                              ┌─────────────────────────┐
                              │   claude.ai Web UI      │
                              │                         │
                              │   Displays response:    │
                              │   "Hi there!"           │
                              └─────────────────────────┘
```

### 10.2 Control Request/Response Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│              CONTROL REQUEST/RESPONSE FLOW (Permission Prompt)           │
└─────────────────────────────────────────────────────────────────────────┘

Child Process                          Bridge                            Server
     │                                   │                                  │
     │  type:'control_request'           │                                  │
     │  subtype:'can_use_tool'           │                                  │
     │  tool_name:'Write'                │                                  │
     │──────────────────────────────────▶│                                  │
     │                                   │                                  │
     │                                   │ handleServerControlRequest()     │
     │                                   │ - Calls onPermissionResponse     │
     │                                   │                                  │
     │                                   │  control_response                │
     │                                   │  subtype:'success'/'error'       │
     │                                   │─────────────────────────────────▶│
     │                                   │                                  │
     │                                   │                                  │ Updates session state
     │                                   │                                  │ - Permission granted/denied
     │                                   │                                  │
     │                                   │◀─────────────────────────────────│
     │                                   │  (Next events flow)              │
     │◀──────────────────────────────────│                                  │
     │  (Tool executes or skipped)       │                                  │
     │                                   │                                  │
```

### 10.3 Session Teardown Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        SESSION TEARDOWN FLOW                              │
└─────────────────────────────────────────────────────────────────────────┘

User presses Ctrl+C or /exit
           │
           ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  gracefulShutdown()                                                      │
│                                                                          │
│  1. Run cleanup functions (2s timeout)                                  │
│     - bridge.teardown()                                                 │
│     - analytics shutdown                                                │
│     - etc.                                                              │
│                                                                          │
│  2. If not complete by 2s, forceExit()                                   │
└─────────────────────────────────────────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  bridge.teardown()                                                       │
│                                                                          │
│  tornDown = true                                                         │
│  refresh.cancelAll()         // Stop JWT refresh                         │
│  flushGate.drop()            // Drop queued messages                     │
│                                                                          │
│  1. Send result message (best-effort)                                   │
│     transport.reportState('idle')                                        │
│     transport.write(makeResultMessage(sessionId))                        │
│                                                                          │
│  2. Archive session (with 401 retry)                                    │
│     status = POST /v1/sessions/{id}/archive                              │
│     if status === 401:                                                   │
│       onAuth401(staleToken)  // Refresh OAuth                            │
│       status = retry archive                                             │
│                                                                          │
│  3. Close transport                                                     │
│     transport.close()                                                    │
│                                                                          │
│  4. Log telemetry                                                       │
│     logEvent('tengu_bridge_repl_teardown', {                             │
│       archive_status: 'ok' | 'network_error' | 'server_4xx' | ...,       │
│       archive_ok: status < 400,                                          │
│     })                                                                   │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 11. Work Secret Protocol

### 11.1 Work Secret Structure

```typescript
export type WorkSecret = {
  version: number
  session_ingress_token: string  // JWT for session ingress endpoints
  api_base_url: string           // Base URL for API calls
  sources: Array<{
    type: string
    git_info?: {
      type: string
      repo: string
      ref?: string
      token?: string
    }
  }>
  auth: Array<{ type: string; token: string }>
  claude_code_args?: Record<string, string> | null
  mcp_config?: unknown | null
  environment_variables?: Record<string, string> | null
  use_code_sessions?: boolean  // CCR v2 selector
}
```

### 11.2 Work Secret Decoding

```typescript
export function decodeWorkSecret(secret: string): WorkSecret {
  // Base64url decode
  const json = Buffer.from(secret, 'base64url').toString('utf-8')
  const parsed: unknown = jsonParse(json)

  // Validate version
  if (
    !parsed ||
    typeof parsed !== 'object' ||
    !('version' in parsed) ||
    parsed.version !== 1
  ) {
    throw new Error(
      `Unsupported work secret version: ${
        parsed && typeof parsed === 'object' && 'version' in parsed
          ? parsed.version
          : 'unknown'
      }`,
    )
  }

  const obj = parsed as Record<string, unknown>

  // Validate required fields
  if (
    typeof obj.session_ingress_token !== 'string' ||
    obj.session_ingress_token.length === 0
  ) {
    throw new Error('Invalid work secret: missing or empty session_ingress_token')
  }

  if (typeof obj.api_base_url !== 'string') {
    throw new Error('Invalid work secret: missing api_base_url')
  }

  return parsed as WorkSecret
}
```

### 11.3 SDK URL Building

```typescript
/**
 * Build WebSocket SDK URL from API base URL and session ID.
 *
 * Uses /v2/ for localhost (direct to session-ingress)
 * and /v1/ for production (Envoy rewrite).
 */
export function buildSdkUrl(apiBaseUrl: string, sessionId: string): string {
  const isLocalhost =
    apiBaseUrl.includes('localhost') || apiBaseUrl.includes('127.0.0.1')
  const protocol = isLocalhost ? 'ws' : 'wss'
  const version = isLocalhost ? 'v2' : 'v1'
  const host = apiBaseUrl.replace(/^https?:\/\//, '').replace(/\/+$/, '')

  return `${protocol}://${host}/${version}/session_ingress/ws/${sessionId}`
}

/**
 * Build CCR v2 session URL (HTTP, not WebSocket).
 */
export function buildCCRv2SdkUrl(
  apiBaseUrl: string,
  sessionId: string,
): string {
  const base = apiBaseUrl.replace(/\/+$/, '')
  return `${base}/v1/code/sessions/${sessionId}`
}
```

### 11.4 Worker Registration

```typescript
/**
 * Register this bridge as the worker for a CCR v2 session.
 * Returns the worker_epoch for heartbeat/state/event requests.
 */
export async function registerWorker(
  sessionUrl: string,
  accessToken: string,
): Promise<number> {
  const response = await axios.post(
    `${sessionUrl}/worker/register`,
    {},
    {
      headers: {
        Authorization: `Bearer ${accessToken}`,
        'Content-Type': 'application/json',
        'anthropic-version': '2023-06-01',
      },
      timeout: 10_000,
    },
  )

  // protojson serializes int64 as string for JS precision
  const raw = response.data?.worker_epoch
  const epoch = typeof raw === 'string' ? Number(raw) : raw

  if (
    typeof epoch !== 'number' ||
    !Number.isFinite(epoch) ||
    !Number.isSafeInteger(epoch)
  ) {
    throw new Error(
      `registerWorker: invalid worker_epoch: ${jsonStringify(response.data)}`,
    )
  }

  return epoch
}
```

### 11.5 Session ID Comparison

```typescript
/**
 * Compare two session IDs regardless of their tagged-ID prefix.
 * Handles both session_* and cse_* formats.
 */
export function sameSessionId(a: string, b: string): boolean {
  if (a === b) return true

  // The body is everything after the last underscore
  const aBody = a.slice(a.lastIndexOf('_') + 1)
  const bBody = b.slice(b.lastIndexOf('_') + 1)

  // Require minimum length to avoid accidental matches
  return aBody.length >= 4 && aBody === bBody
}
```

---

## 12. Environment Registration

### 12.1 Bridge Configuration

```typescript
export type BridgeConfig = {
  dir: string                    // Working directory
  machineName: string            // Hostname
  branch: string                 // Git branch
  gitRepoUrl: string | null      // Git remote URL
  maxSessions: number            // Session capacity
  spawnMode: SpawnMode           // 'single-session' | 'worktree' | 'same-dir'
  verbose: boolean
  sandbox: boolean
  bridgeId: string               // Client-generated UUID
  workerType: string             // Sent as metadata.worker_type
  environmentId: string          // Client-generated UUID for idempotency
  reuseEnvironmentId?: string    // Backend-issued ID for re-register
  apiBaseUrl: string             // API base URL for polling
  sessionIngressUrl: string      // WebSocket ingress base URL
  debugFile?: string             // Debug log path
  sessionTimeoutMs?: number      // Per-session timeout
}

export type SpawnMode = 'single-session' | 'worktree' | 'same-dir'

export type BridgeWorkerType = 'claude_code' | 'claude_code_assistant'
```

### 12.2 Registration Request/Response

```
POST /v1/environments/bridge
Authorization: Bearer {oauth_token}
Content-Type: application/json
anthropic-version: 2023-06-01
anthropic-beta: environments-2025-11-01
x-environment-runner-version: {version}
X-Trusted-Device-Token: {device_token}  // Optional

Request Body:
{
  "machine_name": "alex-macbook-pro",
  "directory": "/Users/alex/project",
  "branch": "main",
  "git_repo_url": "https://github.com/alex/project",
  "max_sessions": 1,
  "metadata": {
    "worker_type": "claude_code"
  },
  "environment_id": "{client-uuid}"  // For idempotent re-register
}

Response (200):
{
  "environment_id": "{backend-issued-id}",
  "environment_secret": "{base64url-encoded-secret}"
}
```

### 12.3 Work Poll Response

```
GET /v1/environments/{environment_id}/work/poll
Authorization: Bearer {environment_secret}
anthropic-beta: environments-2025-11-01

Response (200) - Work available:
{
  "id": "{work_id}",
  "type": "work",
  "environment_id": "{environment_id}",
  "state": "waiting",
  "data": {
    "type": "session",
    "id": "{session_id}"
  },
  "secret": "{base64url-encoded-work-secret}",
  "created_at": "2024-01-01T00:00:00Z"
}

Response (200) - No work:
null
```

---

## 13. Trusted Device Authentication

### 13.1 Overview

Bridge sessions have `SecurityTier=ELEVATED` on the server (CCR v2). The server gates `ConnectBridgeWorker` on its flag `sessions_elevated_auth_enforcement`. This CLI-side flag controls whether the CLI sends `X-Trusted-Device-Token` at all.

### 13.2 Device Enrollment

```typescript
const TRUSTED_DEVICE_GATE = 'tengu_sessions_elevated_auth_enforcement'

// Memoized - secureStorage.read() spawns macOS `security` subprocess (~40ms)
const readStoredToken = memoize((): string | undefined => {
  const envToken = process.env.CLAUDE_TRUSTED_DEVICE_TOKEN
  if (envToken) return envToken
  return getSecureStorage().read()?.trustedDeviceToken
})

export function getTrustedDeviceToken(): string | undefined {
  if (!isGateEnabled()) return undefined
  return readStoredToken()
}

export async function enrollTrustedDevice(): Promise<void> {
  if (!(await checkGate_CACHED_OR_BLOCKING(TRUSTED_DEVICE_GATE))) {
    logForDebugging('[trusted-device] Gate off, skipping enrollment')
    return
  }

  const accessToken = getClaudeAIOAuthTokens()?.accessToken
  if (!accessToken) {
    logForDebugging('[trusted-device] No OAuth token, skipping enrollment')
    return
  }

  const baseUrl = getOauthConfig().BASE_API_URL

  const response = await axios.post<{
    device_token?: string
    device_id?: string
  }>(
    `${baseUrl}/api/auth/trusted_devices`,
    {
      display_name: `Claude Code on ${hostname()} · ${process.platform}`,
    },
    {
      headers: {
        Authorization: `Bearer ${accessToken}`,
        'Content-Type': 'application/json',
      },
      timeout: 10_000,
      validateStatus: s => s < 500,
    },
  )

  if (response.status !== 200 && response.status !== 201) {
    logForDebugging(`[trusted-device] Enrollment failed ${response.status}`)
    return
  }

  const token = response.data?.device_token
  if (!token || typeof token !== 'string') {
    logForDebugging('[trusted-device] Enrollment response missing device_token')
    return
  }

  // Persist to secure storage
  const storageData = secureStorage.read()
  if (storageData) {
    storageData.trustedDeviceToken = token
    secureStorage.update(storageData)
    readStoredToken.cache?.clear?.()
    logForDebugging(`[trusted-device] Enrolled device_id=${response.data.device_id}`)
  }
}
```

### 13.3 Token Usage in API Calls

```typescript
function getHeaders(accessToken: string): Record<string, string> {
  const headers: Record<string, string> = {
    Authorization: `Bearer ${accessToken}`,
    'Content-Type': 'application/json',
    'anthropic-version': '2023-06-01',
    'anthropic-beta': BETA_HEADER,
    'x-environment-runner-version': deps.runnerVersion,
  }

  const deviceToken = deps.getTrustedDeviceToken?.()
  if (deviceToken) {
    headers['X-Trusted-Device-Token'] = deviceToken
  }

  return headers
}
```

---

## 14. Configuration System

### 14.1 Poll Interval Configuration

```typescript
export type PollIntervalConfig = {
  poll_interval_ms_not_at_capacity: number   // Default: 2000ms
  poll_interval_ms_at_capacity: number       // Default: 30000ms (or 0 = disabled)
  non_exclusive_heartbeat_interval_ms: number // Default: 0 (disabled)
  multisession_poll_interval_ms_not_at_capacity: number
  multisession_poll_interval_ms_partial_capacity: number
  multisession_poll_interval_ms_at_capacity: number
  reclaim_older_than_ms: number              // Default: 5000ms
  session_keepalive_interval_v2_ms: number   // Default: 120000ms
}

const DEFAULT_POLL_CONFIG: PollIntervalConfig = {
  poll_interval_ms_not_at_capacity: 2_000,
  poll_interval_ms_at_capacity: 30_000,
  non_exclusive_heartbeat_interval_ms: 0,
  multisession_poll_interval_ms_not_at_capacity: 2_000,
  multisession_poll_interval_ms_partial_capacity: 10_000,
  multisession_poll_interval_ms_at_capacity: 30_000,
  reclaim_older_than_ms: 5_000,
  session_keepalive_interval_v2_ms: 120_000,
}
```

### 14.2 Environment-Less Bridge Config

```typescript
export type EnvLessBridgeConfig = {
  init_retry_max_attempts: number           // Default: 3
  init_retry_base_delay_ms: number          // Default: 500ms
  init_retry_jitter_fraction: number        // Default: 0.25
  init_retry_max_delay_ms: number           // Default: 4000ms
  http_timeout_ms: number                   // Default: 10000ms
  uuid_dedup_buffer_size: number            // Default: 2000
  heartbeat_interval_ms: number             // Default: 20000ms
  heartbeat_jitter_fraction: number         // Default: 0.1
  token_refresh_buffer_ms: number           // Default: 300000ms (5min)
  teardown_archive_timeout_ms: number       // Default: 1500ms
  connect_timeout_ms: number                // Default: 15000ms
  min_version: string                       // Default: '0.0.0'
  should_show_app_upgrade_message: boolean  // Default: false
}

const DEFAULT_ENV_LESS_BRIDGE_CONFIG: EnvLessBridgeConfig = {
  init_retry_max_attempts: 3,
  init_retry_base_delay_ms: 500,
  init_retry_jitter_fraction: 0.25,
  init_retry_max_delay_ms: 4000,
  http_timeout_ms: 10_000,
  uuid_dedup_buffer_size: 2000,
  heartbeat_interval_ms: 20_000,
  heartbeat_jitter_fraction: 0.1,
  token_refresh_buffer_ms: 300_000,
  teardown_archive_timeout_ms: 1500,
  connect_timeout_ms: 15_000,
  min_version: '0.0.0',
  should_show_app_upgrade_message: false,
}
```

### 14.3 GrowthBook Integration

```typescript
export function getPollIntervalConfig(): PollIntervalConfig {
  const raw = getFeatureValue_CACHED_WITH_REFRESH<unknown>(
    'tengu_bridge_poll_interval_config',
    DEFAULT_POLL_CONFIG,
    5 * 60 * 1000,  // 5-minute refresh window
  )
  const parsed = pollIntervalConfigSchema().safeParse(raw)
  return parsed.success ? parsed.data : DEFAULT_POLL_CONFIG
}

export async function getEnvLessBridgeConfig(): Promise<EnvLessBridgeConfig> {
  const raw = await getFeatureValue_DEPRECATED<unknown>(
    'tengu_bridge_repl_v2_config',
    DEFAULT_ENV_LESS_BRIDGE_CONFIG,
  )
  const parsed = envLessBridgeConfigSchema().safeParse(raw)
  return parsed.success ? parsed.data : DEFAULT_ENV_LESS_BRIDGE_CONFIG
}
```

---

## 15. Error Handling and Recovery

### 15.1 OAuth 401 Recovery

```typescript
async function withOAuthRetry<T>(
  fn: (accessToken: string) => Promise<{ status: number; data: T }>,
  context: string,
): Promise<{ status: number; data: T }> {
  const accessToken = resolveAuth()
  const response = await fn(accessToken)

  if (response.status !== 401) {
    return response
  }

  if (!deps.onAuth401) {
    debug(`[bridge:api] ${context}: 401 received, no refresh handler`)
    return response
  }

  debug(`[bridge:api] ${context}: 401 received, attempting token refresh`)
  const refreshed = await deps.onAuth401(accessToken)

  if (refreshed) {
    debug(`[bridge:api] ${context}: Token refreshed, retrying request`)
    const newToken = resolveAuth()
    const retryResponse = await fn(newToken)
    if (retryResponse.status !== 401) {
      return retryResponse
    }
    debug(`[bridge:api] ${context}: Retry after refresh also got 401`)
  } else {
    debug(`[bridge:api] ${context}: Token refresh failed`)
  }

  return response  // Let handleErrorStatus throw BridgeFatalError
}
```

### 15.2 Poll Error Backoff

```typescript
let connBackoff = 0
let connErrorStart: number | null = null
const backoffConfig = {
  connInitialMs: 2_000,
  connCapMs: 120_000,
  connGiveUpMs: 600_000,
}

while (!loopSignal.aborted) {
  try {
    work = await api.pollForWork(environmentId, environmentSecret, pollSignal)
    // Reset backoff on success
    connBackoff = 0
    connErrorStart = null
  } catch (err) {
    // Connection error - exponential backoff
    if (connErrorStart === null) {
      connErrorStart = Date.now()
    }

    const elapsed = Date.now() - connErrorStart
    if (elapsed >= backoffConfig.connGiveUpMs) {
      logger.logError(`Connection error for ${formatDuration(elapsed)}, giving up`)
      throw new BridgeFatalError('Connection error timeout', 0)
    }

    if (connBackoff === 0) {
      connBackoff = backoffConfig.connInitialMs
    } else {
      connBackoff = Math.min(connBackoff * 2, backoffConfig.connCapMs)
    }

    logForDebugging(`Poll error, backing off ${formatDuration(connBackoff)}`)
    await sleepWithSignal(connBackoff, pollSignal)
    continue
  }
}
```

### 15.3 Session Timeout Watchdog

```typescript
// Start timeout watchdog when session spawns
const timer = setTimeout(() => {
  timedOutSessions.add(sessionId)
  logger.logVerbose(`Session ${sessionId} exceeded timeout, killing`)
  handle.forceKill()
}, config.sessionTimeoutMs)
sessionTimers.set(sessionId, timer)

// In onSessionDone
const wasTimedOut = timedOutSessions.delete(sessionId)
const status: SessionDoneStatus =
  wasTimedOut && rawStatus === 'interrupted' ? 'failed' : rawStatus

if (status === 'failed' && !wasTimedOut) {
  const stderrSummary = handle.lastStderr.join('\n')
  logger.logSessionFailed(sessionId, stderrSummary || 'Process exited with error')
}
// Timeout-killed sessions: log already happened, skip failure log
```

### 15.4 BridgeFatalError Classification

```typescript
export class BridgeFatalError extends Error {
  readonly status: number
  readonly errorType: string | undefined

  constructor(message: string, status: number, errorType?: string) {
    super(message)
    this.name = 'BridgeFatalError'
    this.status = status
    this.errorType = errorType
  }
}

function handleErrorStatus(status: number, data: unknown, context: string): void {
  if (status === 200 || status === 204) return

  const detail = extractErrorDetail(data)
  const errorType = extractErrorTypeFromData(data)

  switch (status) {
    case 401:
      throw new BridgeFatalError(
        `${context}: Authentication failed (401)${detail ? `: ${detail}` : ''}. ${BRIDGE_LOGIN_INSTRUCTION}`,
        401,
        errorType,
      )
    case 403:
      throw new BridgeFatalError(
        isExpiredErrorType(errorType)
          ? 'Remote Control session has expired.'
          : `${context}: Access denied (403)${detail ? `: ${detail}` : ''}`,
        403,
        errorType,
      )
    case 404:
      throw new BridgeFatalError(
        detail ?? `${context}: Not found (404)`,
        404,
        errorType,
      )
    case 410:
      throw new BridgeFatalError(
        detail ?? 'Remote Control session has expired.',
        410,
        errorType ?? 'environment_expired',
      )
    case 429:
      throw new Error(`${context}: Rate limited (429)`)
    default:
      throw new Error(
        `${context}: Failed with status ${status}${detail ? `: ${detail}` : ''}`
      )
  }
}

export function isExpiredErrorType(errorType: string | undefined): boolean {
  if (!errorType) return false
  return errorType.includes('expired') || errorType.includes('lifetime')
}

export function isSuppressible403(err: BridgeFatalError): boolean {
  if (err.status !== 403) return false
  return (
    err.message.includes('external_poll_sessions') ||
    err.message.includes('environments:manage')
  )
}
```

---

## Appendix A: Session ID Format Compatibility

```typescript
const COMPAT_PREFIX = 'session_'
const STAGING_MARKER = '_staging_'
const INFRA_PREFIX = 'cse_'

/**
 * Convert infra ID (cse_*) to compat ID (session_*).
 */
export function toCompatSessionId(id: string): string {
  if (id.startsWith(COMPAT_PREFIX)) return id

  const body = id.includes(STAGING_MARKER)
    ? id.slice(id.indexOf(STAGING_MARKER) + STAGING_MARKER.length)
    : id.slice(INFRA_PREFIX.length)

  return `${COMPAT_PREFIX}${body}`
}

/**
 * Convert compat ID (session_*) to infra ID (cse_*).
 */
export function toInfraSessionId(id: string): string {
  if (id.startsWith(INFRA_PREFIX)) return id

  const body = id.includes(STAGING_MARKER)
    ? id.slice(id.indexOf(STAGING_MARKER) + STAGING_MARKER.length)
    : id.slice(COMPAT_PREFIX.length)

  return `${INFRA_PREFIX}${body}`
}
```

---

## Appendix B: FlushGate State Machine

```typescript
export class FlushGate<T> {
  private _active = false
  private _pending: T[] = []

  get active(): boolean { return this._active }
  get pendingCount(): number { return this._pending.length }

  /** Start flush - enqueue() will queue items. */
  start(): void {
    this._active = true
  }

  /** End flush and return queued items for draining. */
  end(): T[] {
    this._active = false
    return this._pending.splice(0)
  }

  /** Queue items if active, return true if queued. */
  enqueue(...items: T[]): boolean {
    if (!this._active) return false
    this._pending.push(...items)
    return true
  }

  /** Discard all queued items (permanent close). */
  drop(): number {
    this._active = false
    const count = this._pending.length
    this._pending.length = 0
    return count
  }

  /** Clear active without dropping (transport replacement). */
  deactivate(): void {
    this._active = false
  }
}
```

---

## Appendix C: CapacityWake Primitive

```typescript
export type CapacitySignal = { signal: AbortSignal; cleanup: () => void }

export type CapacityWake = {
  signal(): CapacitySignal
  wake(): void
}

export function createCapacityWake(outerSignal: AbortSignal): CapacityWake {
  let wakeController = new AbortController()

  function wake(): void {
    wakeController.abort()
    wakeController = new AbortController()
  }

  function signal(): CapacitySignal {
    const merged = new AbortController()
    const abort = (): void => merged.abort()

    if (outerSignal.aborted || wakeController.signal.aborted) {
      merged.abort()
      return { signal: merged.signal, cleanup: () => {} }
    }

    outerSignal.addEventListener('abort', abort, { once: true })
    const capSig = wakeController.signal
    capSig.addEventListener('abort', abort, { once: true })

    return {
      signal: merged.signal,
      cleanup: () => {
        outerSignal.removeEventListener('abort', abort)
        capSig.removeEventListener('abort', abort)
      },
    }
  }

  return { signal, wake }
}

// Usage in poll loop:
const capacityWake = createCapacityWake(loopSignal)
const { signal: capacitySignal, cleanup } = capacityWake.signal()

await sleepWithSignal(pollIntervalMs, capacitySignal).catch(() => {})
cleanup()  // Remove listeners

// When session completes:
capacityWake.wake()  // Abort current sleep, poll immediately
```

---

## Summary

The bridge module is a sophisticated bi-directional communication layer that:

1. **Implements two protocol versions** (V1 environment-based, V2 direct CCR) with a unified transport abstraction
2. **Manages JWT authentication** with proactive token refresh before expiry
3. **Handles message routing** with echo dedup, re-delivery protection, and control request/response handling
4. **Spawns and monitors child processes** for each active session
5. **Provides crash recovery** via persistent pointer files
6. **Implements robust error handling** with exponential backoff, 401 recovery, and session timeout watchdogs
7. **Supports multi-session concurrency** with capacity-aware polling and worktree isolation
8. **Exposes terminal UI** with QR codes, status displays, and session activity tracking

The module is gated by feature flags and organization policies, allowing Anthropic to control rollout and enforce security requirements.

---

**Document Generated:** 2026-04-07  
**Source Files:** 31 TypeScript files in `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/bridge/`  
**Output:** `/home/darkvoid/Boxxed/@dev/repo-expolorations/hermes-agent/claude-code-src/bridge/exploration.md`
