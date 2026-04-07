# Upstreamproxy Module — Deep-Dive Exploration

**Module:** `upstreamproxy/`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/upstreamproxy/`  
**Files:** 2 TypeScript files  
**Created:** 2026-04-07

---

## 1. Module Overview

The `upstreamproxy/` module implements **CCR (Claude Code Remote) upstream proxy relay** — a CONNECT-over-WebSocket tunnel that enables CCR session containers to route traffic through org-configured upstream proxies with credential injection. This is critical for enterprise deployments where containers must use corporate proxies with authentication.

### Core Responsibilities

1. **Session Token Management** — Secure token handling:
   - Read token from `/run/ccr/session_token`
   - Set `prctl(PR_SET_DUMPABLE, 0)` to block ptrace
   - Unlink token file after relay starts (heap-only thereafter)

2. **CA Certificate Bundle** — TLS trust configuration:
   - Download upstreamproxy CA cert from CCR server
   - Concatenate with system bundle
   - Configure via `SSL_CERT_FILE`, `NODE_EXTRA_CA_CERTS`

3. **CONNECT Relay** — HTTP CONNECT to WebSocket tunnel:
   - Listen on localhost TCP (ephemeral port)
   - Accept HTTP CONNECT from curl/gh/kubectl
   - Tunnel bytes over WebSocket to CCR server
   - Protobuf message wrapping (`UpstreamProxyChunk`)

4. **Environment Injection** — Subprocess proxy configuration:
   - `HTTPS_PROXY`, `NO_PROXY` for agent subprocesses
   - `SSL_CERT_FILE`, `NODE_EXTRA_CA_CERTS` for TLS trust
   - Inherited proxy vars passed through

### Key Design Patterns

- **Fail-Open Design**: Any error disables proxy, doesn't break session
- **Security Hardening**: `prctl(PR_SET_DUMPABLE, 0)` prevents heap dump
- **Protobuf Encoding**: Hand-coded protobuf for single-field message
- **Dual Runtime Support**: Bun.listen() or Node net.createServer

---

## 2. File Inventory

| File | Lines | Description |
|------|-------|-------------|
| `upstreamproxy.ts` | ~300+ | Main proxy initialization and env injection |
| `relay.ts` | ~500+ | CONNECT-over-WebSocket relay server |

**Total:** ~800+ lines across 2 files

---

## 3. Key Exports

### Proxy Initialization (`upstreamproxy.ts`)

```typescript
// Constants
export const SESSION_TOKEN_PATH = '/run/ccr/session_token'

// Relay state
type UpstreamProxyState = {
  enabled: boolean
  port?: number
  caBundlePath?: string
}

// Initialize proxy (called from init.ts)
export async function initUpstreamProxy(opts?: {
  tokenPath?: string
  systemCaPath?: string
  caBundlePath?: string
  ccrBaseUrl?: string
}): Promise<UpstreamProxyState>

// Get env vars for subprocesses
export function getUpstreamProxyEnv(): Record<string, string>
```

### Relay Server (`relay.ts`)

```typescript
// Protobuf encoding/decoding
export function encodeChunk(data: Uint8Array): Uint8Array
export function decodeChunk(buf: Uint8Array): Uint8Array | null

// Relay type
export type UpstreamProxyRelay = {
  port: number
  stop: () => void
}

// Start relay
export async function startUpstreamProxyRelay(opts: {
  wsUrl: string
  sessionId: string
  token: string
}): Promise<UpstreamProxyRelay>
```

---

## 4. Line-by-Line Analysis

### 4.1 NO_PROXY List (`upstreamproxy.ts` lines 36-63)

```typescript
// Hosts the proxy must NOT intercept. Covers loopback, RFC1918, the IMDS
// range, and the package registries + GitHub that CCR containers already
// reach directly. Mirrors airlock/scripts/sandbox-shell-ccr.sh.
const NO_PROXY_LIST = [
  'localhost',
  '127.0.0.1',
  '::1',
  '169.254.0.0/16',
  '10.0.0.0/8',
  '172.16.0.0/12',
  '192.168.0.0/16',
  // Anthropic API: no upstream route will ever match, and the MITM breaks
  // non-Bun runtimes (Python httpx/certifi doesn't trust the forged CA).
  // Three forms because NO_PROXY parsing differs across runtimes:
  //   *.anthropic.com  — Bun, curl, Go (glob match)
  //   .anthropic.com   — Python urllib/httpx (suffix match, strips leading dot)
  //   anthropic.com    — apex domain fallback
  'anthropic.com',
  '.anthropic.com',
  '*.anthropic.com',
  'github.com',
  'api.github.com',
  '*.github.com',
  '*.githubusercontent.com',
  'registry.npmjs.org',
  'pypi.org',
  'files.pythonhosted.org',
  'index.crates.io',
  'proxy.golang.org',
].join(',')
```

**Comprehensive Exclusions**:
- Loopback addresses
- RFC1918 private ranges
- IMDS (instance metadata)
- Package registries (npm, pypi, crates, golang)
- GitHub (all domains)
- Anthropic API (all forms for runtime compatibility)

### 4.2 Proxy Initialization (`upstreamproxy.ts` lines 79-153)

```typescript
export async function initUpstreamProxy(opts?: {...}): Promise<UpstreamProxyState> {
  // Check CCR environment
  if (!isEnvTruthy(process.env.CLAUDE_CODE_REMOTE)) {
    return state  // Not in CCR
  }
  
  // Check server-side feature gate (injected via StartupContext)
  if (!isEnvTruthy(process.env.CCR_UPSTREAM_PROXY_ENABLED)) {
    return state  // Feature disabled
  }

  const sessionId = process.env.CLAUDE_CODE_REMOTE_SESSION_ID
  if (!sessionId) {
    logForDebugging('[upstreamproxy] CLAUDE_CODE_REMOTE_SESSION_ID unset')
    return state
  }

  // Read session token
  const token = await readToken(opts?.tokenPath ?? SESSION_TOKEN_PATH)
  if (!token) {
    logForDebugging('[upstreamproxy] no session token file')
    return state
  }

  // Security: block ptrace of heap
  setNonDumpable()

  // Download CA bundle
  const baseUrl = opts?.ccrBaseUrl ?? process.env.ANTHROPIC_BASE_URL
  const caBundlePath = opts?.caBundlePath ?? join(homedir(), '.ccr', 'ca-bundle.crt')
  
  const caOk = await downloadCaBundle(baseUrl, systemCaPath, caBundlePath)
  if (!caOk) return state  // Fail open

  // Start WebSocket relay
  try {
    const wsUrl = baseUrl.replace(/^http/, 'ws') + '/v1/code/upstreamproxy/ws'
    const relay = await startUpstreamProxyRelay({ wsUrl, sessionId, token })
    registerCleanup(async () => relay.stop())
    state = { enabled: true, port: relay.port, caBundlePath }
    logForDebugging(`[upstreamproxy] enabled on 127.0.0.1:${relay.port}`)
    
    // Unlink token file after relay is up
    await unlink(tokenPath).catch(() => {
      logForDebugging('[upstreamproxy] token file unlink failed', { level: 'warn' })
    })
  } catch (err) {
    logForDebugging(`[upstreamproxy] relay start failed: ${err}`)
  }

  return state
}
```

**Fail-Open Design**: Every error returns `{enabled: false}` without breaking session.

**Security**: `setNonDumpable()` blocks `ptrace(PTRACE_ATTACH)` even from same UID.

**Token Lifecycle**: File exists only during init, then unlinked (heap-only thereafter).

### 4.3 Environment Injection (`upstreamproxy.ts` lines 160-199)

```typescript
export function getUpstreamProxyEnv(): Record<string, string> {
  if (!state.enabled || !state.port || !state.caBundlePath) {
    // Inherited proxy vars from parent? Pass through
    if (process.env.HTTPS_PROXY && process.env.SSL_CERT_FILE) {
      const inherited: Record<string, string> = {}
      for (const key of [
        'HTTPS_PROXY', 'https_proxy',
        'NO_PROXY', 'no_proxy',
        'SSL_CERT_FILE',
        'NODE_EXTRA_CA_CERTS',
        'REQUESTS_CA_BUNDLE',
        'CURL_CA_BUNDLE',
      ]) {
        if (process.env[key]) inherited[key] = process.env[key]
      }
      return inherited
    }
    return {}
  }
  
  const proxyUrl = `http://127.0.0.1:${state.port}`
  
  // HTTPS only: the relay handles CONNECT and nothing else
  return {
    HTTPS_PROXY: proxyUrl,
    https_proxy: proxyUrl,
    NO_PROXY: NO_PROXY_LIST,
    no_proxy: NO_PROXY_LIST,
    SSL_CERT_FILE: state.caBundlePath,
    NODE_EXTRA_CA_CERTS: state.caBundlePath,
    REQUESTS_CA_BUNDLE: state.caBundlePath,
    CURL_CA_BUNDLE: state.caBundlePath,
  }
}
```

**Inheritance Mode**: If proxy not enabled but parent has proxy vars, pass through.

**HTTPS Only**: Relay handles CONNECT (TLS tunnel), not plain HTTP.

**Multiple CA Vars**: Set all common CA bundle env vars for runtime compatibility.

### 4.4 Protobuf Encoding (`relay.ts` lines 66-81)

```typescript
/**
 * Encode an UpstreamProxyChunk protobuf message by hand.
 *
 * For `message UpstreamProxyChunk { bytes data = 1; }` the wire format is:
 *   tag = (field_number << 3) | wire_type = (1 << 3) | 2 = 0x0a
 *   followed by varint length, followed by the bytes.
 */
export function encodeChunk(data: Uint8Array): Uint8Array {
  const len = data.length
  // varint encoding of length
  const varint: number[] = []
  let n = len
  while (n > 0x7f) {
    varint.push((n & 0x7f) | 0x80)
    n >>>= 7
  }
  varint.push(n)
  
  const out = new Uint8Array(1 + varint.length + len)
  out[0] = 0x0a  // tag: field 1, wire type 2 (length-delimited)
  out.set(varint, 1)
  out.set(data, 1 + varint.length)
  return out
}
```

**Hand-Coded Protobuf**: Avoids runtime dependency (protobufjs) for single-field message.

**Wire Format**: Tag (0x0a) + varint length + raw bytes.

### 4.5 Protobuf Decoding (`relay.ts` lines 87-103)

```typescript
export function decodeChunk(buf: Uint8Array): Uint8Array | null {
  if (buf.length === 0) return new Uint8Array(0)
  if (buf[0] !== 0x0a) return null  // Wrong tag
  
  // Decode varint length
  let len = 0
  let shift = 0
  let i = 1
  while (i < buf.length) {
    const b = buf[i]!
    len |= (b & 0x7f) << shift
    i++
    if ((b & 0x80) === 0) break
    shift += 7
    if (shift > 28) return null  // Varint too long
  }
  
  if (i + len > buf.length) return null  // Incomplete chunk
  return buf.subarray(i, i + len)
}
```

**Tolerant Decoding**: Returns `null` for malformed chunks, doesn't throw.

**Zero-Length Handling**: Empty buffer → empty array (keepalive semantics).

### 4.6 WebSocket Relay (`relay.ts` lines 155-174)

```typescript
export async function startUpstreamProxyRelay(opts: {
  wsUrl: string
  sessionId: string
  token: string
}): Promise<UpstreamProxyRelay> {
  const authHeader =
    'Basic ' + Buffer.from(`${opts.sessionId}:${opts.token}`).toString('base64')
  
  // WS upgrade requires JWT auth (separate from CONNECT's Proxy-Authorization)
  const wsAuthHeader = `Bearer ${opts.token}`

  // Use Bun.listen when available, otherwise Node's net.createServer
  const relay =
    typeof Bun !== 'undefined'
      ? startBunRelay(opts.wsUrl, authHeader, wsAuthHeader)
      : await startNodeRelay(opts.wsUrl, authHeader, wsAuthHeader)

  logForDebugging(`[upstreamproxy] relay listening on 127.0.0.1:${relay.port}`)
  return relay
}
```

**Dual Auth**:
- **WS Upgrade**: JWT bearer token (`wsAuthHeader`)
- **CONNECT**: Basic auth with session ID + token (`authHeader`)

**Runtime Detection**: Bun.listen() or Node net.createServer based on runtime.

---

## 5. Integration Points

### 5.1 With `init.ts`

| Component | Integration |
|-----------|-------------|
| `upstreamproxy.ts` | Called during startup in CCR environments |

### 5.2 With `utils/subprocessEnv.js`

| Component | Integration |
|-----------|-------------|
| `getUpstreamProxyEnv()` | Registered for subprocess injection |

### 5.3 With `utils/cleanupRegistry.js`

| Component | Integration |
|-----------|-------------|
| `startUpstreamProxyRelay()` | Relay cleanup registered |

### 5.4 With `utils/mtls.js`

| Component | Integration |
|-----------|-------------|
| `relay.ts` | Uses `getWebSocketTLSOptions()` for mTLS |

---

## 6. Data Flow

### 6.1 Proxy Initialization Flow

```
CCR container startup
    │
    ▼
init.ts calls initUpstreamProxy()
    │
    ├──► Check CLAUDE_CODE_REMOTE (CCR env)
    ├──► Check CCR_UPSTREAM_PROXY_ENABLED (feature gate)
    ├──► Read /run/ccr/session_token
    │    └──► setNonDumpable() → block ptrace
    │
    ├──► Download CA bundle from CCR server
    │    └──► Concatenate with system CA
    │
    ├──► startUpstreamProxyRelay()
    │    ├──► WebSocket to CCR server
    │    ├──► Listen on localhost:ephemeral
    │    └──► Unlink token file
    │
    ▼
Return {enabled: true, port, caBundlePath}
```

### 6.2 CONNECT Tunnel Flow

```
Agent subprocess (curl/gh/python)
    │
    ▼
HTTPS_PROXY=http://127.0.0.1:<port>
    │
    ▼
HTTP CONNECT to proxy
    │
    ├──► Parse CONNECT request
    ├──► Open WebSocket to CCR server
    │    └──► Auth: Basic + Bearer
    │
    ├──► Forward CONNECT response (200/4xx/5xx)
    │
    ▼
Tunnel bytes bidirectionally
    │
    ├──► Client → WS: encodeChunk()
    └──► WS → Client: decodeChunk()
```

### 6.3 Environment Injection Flow

```
Spawn agent subprocess
    │
    ▼
getUpstreamProxyEnv()
    │
    ├──► Proxy enabled?
    │    ├──► Yes → HTTPS_PROXY, SSL_CERT_FILE, etc.
    │    └──► No → Inherit from parent (if present)
    │
    ▼
Merge into subprocess env
```

---

## 7. Key Patterns

### 7.1 Fail-Open Design

```typescript
// Every error path returns {enabled: false}
if (!isEnvTruthy(process.env.CLAUDE_CODE_REMOTE)) {
  return state  // Not CCR
}
if (!token) {
  return state  // No token
}
if (!caOk) {
  return state  // CA download failed
}
try {
  // Relay start failed
} catch (err) {
  return state
}
```

**Why**: A broken proxy must never break a working session.

### 7.2 Security: Non-Dumpable

```typescript
function setNonDumpable(): void {
  // prctl(PR_SET_DUMPABLE, 0)
  // Blocks ptrace(PTRACE_ATTACH) even from same UID
  // Prevents heap dump to steal session token
}
```

**Threat Model**: Same-UID attacker in shared container namespace.

### 7.3 Dual Authentication

```
WebSocket Upgrade
    │
    └──► Authorization: Bearer <JWT>
         │
         ▼
         CCR gateway authenticates JWT
         
CONNECT Request (inside tunnel)
    │
    └──► Proxy-Authorization: Basic <sessionId:token>
         │
         ▼
         Upstream proxy authenticates
```

**Defense in Depth**: Two auth layers for two different systems.

---

## 8. Environment Variables

| Variable | Purpose | Values |
|----------|---------|--------|
| `CLAUDE_CODE_REMOTE` | CCR environment detection | `'true'`/undefined |
| `CCR_UPSTREAM_PROXY_ENABLED` | Server-side feature gate | `'true'`/undefined |
| `CLAUDE_CODE_REMOTE_SESSION_ID` | Session identifier | UUID |
| `ANTHROPIC_BASE_URL` | API base URL for CA download | URL |
| `HTTPS_PROXY` | Proxy URL for subprocesses | `http://127.0.0.1:<port>` |
| `SSL_CERT_FILE` | CA bundle path | `~/.ccr/ca-bundle.crt` |

---

## 9. Summary

The `upstreamproxy/` module provides **CCR upstream proxy tunneling**:

1. **Secure Token Handling** — Read, protect, unlink
2. **CA Bundle Management** — Download, concatenate, configure
3. **CONNECT Relay** — HTTP CONNECT to WebSocket tunnel
4. **Environment Injection** — Subprocess proxy configuration

**Key Design Decisions**:
- **Fail-open** — Broken proxy doesn't break session
- **Non-dumpable** — Blocks ptrace heap dumps
- **Protobuf encoding** — Hand-coded for minimal deps
- **Dual auth** — WebSocket JWT + CONNECT Basic

**Security Model**:
- Token file unlinked after relay starts (heap-only)
- `prctl(PR_SET_DUMPABLE, 0)` blocks ptrace
- CA bundle downloaded over authenticated connection
- NO_PROXY excludes sensitive endpoints

---

**Last Updated:** 2026-04-07  
**Status:** Complete — 2 of 2 files analyzed
