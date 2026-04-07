# Claude Code MCP Implementation — Deep Dive Exploration

**Project:** Hermes Agent Deep Dive  
**Subject:** MCP (Model Context Protocol) Apps Implementation  
**Created:** 2026-04-07  
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/services/mcp/`

---

## Executive Summary

Claude Code implements comprehensive support for MCP (Model Context Protocol) servers, enabling integration with external tools and services through a standardized protocol. The implementation supports:

1. **Multiple Transport Types** — stdio, SSE, HTTP, WebSocket, SDK, claude.ai proxy
2. **OAuth 2.0 Authentication** — Including XAA (Cross-App Access) enterprise flow
3. **Tool Discovery & Execution** — Dynamic tool registration from MCP servers
4. **Resource Access** — MCP resource listing and reading
5. **Elicitation Support** — Form and URL-based user confirmation flows
6. **Claude.ai Integration** — Organization-managed MCP servers via proxy

**Total Implementation:** ~3,000+ lines across 25+ files in `services/mcp/`

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Claude Code MCP Architecture                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      MCP Client Layer                                │   │
│  │  - Client.ts: Main MCP client orchestration                         │   │
│  │  - connectToServer(): Transport-specific connection logic           │   │
│  │  - Tool wrapping: MCPTool wrapper for each discovered tool          │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│         ┌────────────────────┼────────────────────┐                        │
│         │                    │                    │                        │
│         ▼                    ▼                    ▼                        │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐                    │
│  │  stdio      │    │  SSE/HTTP   │    │  WebSocket  │                    │
│  │  Transport  │    │  Transport  │    │  Transport  │                    │
│  │             │    │             │    │             │                    │
│  │  Command    │    │  OAuth 2.0  │    │  Bun/Node   │                    │
│  │  spawning   │    │  Auth       │    │  WebSocket  │                    │
│  └─────────────┘    └─────────────┘    └─────────────┘                    │
│         │                    │                    │                        │
│         └────────────────────┼────────────────────┘                        │
│                              │                                              │
│                              ▼                                              │
│              ┌───────────────────────────────┐                             │
│              │   MCP Servers (External)      │                             │
│              │   - Filesystem servers        │                             │
│              │   - Database servers          │                             │
│              │   - API wrappers              │                             │
│              │   - Claude.ai proxies         │                             │
│              │   - IDE integrations          │                             │
│              └───────────────────────────────┘                             │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 1. MCP Server Configuration

### 1.1 Server Configuration Types

Claude Code supports 8 server types:

```typescript
// From services/mcp/config.ts
export type McpServerConfig = z.infer<ReturnType<typeof McpServerConfigSchema>>

export const McpServerConfigSchema = lazySchema(() =>
  z.union([
    McpStdioServerConfigSchema(),      // Local command spawning
    McpSSEServerConfigSchema(),        // Server-Sent Events
    McpSSEIDEServerConfigSchema(),     // IDE-specific SSE
    McpWebSocketIDEServerConfigSchema(),// IDE-specific WebSocket
    McpHTTPServerConfigSchema(),       // HTTP Streamable
    McpWebSocketServerConfigSchema(),  // WebSocket MCP protocol
    McpSdkServerConfigSchema(),        // In-process SDK servers
    McpClaudeAIProxyServerConfigSchema(), // Claude.ai proxy
  ]),
)
```

### 1.2 Configuration Schemas

**Stdio Server (local command):**
```typescript
{
  type: 'stdio',
  command: 'npx',
  args: ['-y', '@modelcontextprotocol/server-filesystem'],
  env: { /* optional env vars */ },
}
```

**SSE Server (remote HTTP):**
```typescript
{
  type: 'sse',
  url: 'https://mcp.example.com/sse',
  headers: { 'X-Custom-Header': 'value' },
  oauth: {
    clientId: '...',
    callbackPort: 3118,
    authServerMetadataUrl: 'https://auth.example.com/.well-known/oauth-authorization-server',
    xaa: true,  // Cross-App Access enabled
  },
}
```

**HTTP Streamable Server:**
```typescript
{
  type: 'http',
  url: 'https://mcp.example.com/mcp',
  headers: { ... },
  oauth: { ... },
}
```

**Claude.ai Proxy:**
```typescript
{
  type: 'claudeai-proxy',
  url: 'https://api.anthropic.com/mcp/{server_id}',
  id: 'server-uuid',  // Claude.ai server identifier
}
```

**SDK Server (in-process):**
```typescript
{
  type: 'sdk',
  name: 'my-sdk-mcp',  // References SDK-registered MCP server
}
```

### 1.3 Configuration Scopes

```typescript
export type ConfigScope = 
  | 'local'      // User's local config
  | 'user'       // User-level config
  | 'project'    // Project-specific (.claude/mcp.json)
  | 'dynamic'    // Runtime-added servers
  | 'enterprise' // Enterprise-managed
  | 'claudeai'   // Claude.ai organization
  | 'managed'    // Plugin-provided
```

### 1.4 Config File Locations

```
~/.claude/mcp.json              # User-level MCP config
.claude/mcp.json                # Project-level MCP config
~/.claude/global.json           # Global settings (seen MCP connections)
```

**Example mcp.json:**
```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/home/user"]
    },
    "github": {
      "type": "sse",
      "url": "https://github-mcp.example.com/sse",
      "oauth": {
        "clientId": "github-mcp-client-id"
      }
    }
  }
}
```

---

## 2. Transport Layer Implementation

### 2.1 stdio Transport

Local MCP servers spawned as child processes:

```typescript
// SDK's StdioClientTransport used in client.ts
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js'

const transport = new StdioClientTransport({
  command: serverRef.command,
  args: serverRef.args || [],
  env: {
    ...subprocessEnv,
    ...serverRef.env,
  },
})
```

**Process Lifecycle:**
1. Spawn command as child process
2. Connect stdin/stdout for JSON-RPC message exchange
3. Process inherits sanitized environment
4. Cleanup on transport close()

### 2.2 SSE Transport (Server-Sent Events)

Remote servers using SSE for server→client streaming:

```typescript
// From client.ts lines 619-676
const authProvider = new ClaudeAuthProvider(name, serverRef)

const transportOptions: SSEClientTransportOptions = {
  authProvider,
  fetch: wrapFetchWithTimeout(
    wrapFetchWithStepUpDetection(createFetchWithInit(), authProvider),
  ),
  requestInit: {
    headers: {
      'User-Agent': getMCPUserAgent(),
      ...combinedHeaders,
    },
  },
  // EventSource for SSE stream (no timeout - long-lived connection)
  eventSourceInit: {
    fetch: async (url: string | URL, init?: RequestInit) => {
      const authHeaders: Record<string, string> = {}
      const tokens = await authProvider.tokens()
      if (tokens) {
        authHeaders.Authorization = `Bearer ${tokens.access_token}`
      }
      return fetch(url, {
        ...init,
        ...proxyOptions,
        headers: {
          'User-Agent': getMCPUserAgent(),
          ...authHeaders,
          ...combinedHeaders,
          Accept: 'text/event-stream',
        },
      })
    },
  },
}

transport = new SSEClientTransport(new URL(serverRef.url), transportOptions)
```

**Key Design:**
- **EventSource fetch NOT wrapped with timeout** — SSE streams are long-lived
- **POST requests get timeout** — Individual API calls use `wrapFetchWithTimeout`
- **Step-up detection** — 403 errors trigger re-authentication before failing

### 2.3 HTTP Streamable Transport

MCP Streamable HTTP spec with JSON + SSE accept headers:

```typescript
// From client.ts lines 784-864
const MCP_STREAMABLE_HTTP_ACCEPT = 'application/json, text/event-stream'

export function wrapFetchWithTimeout(baseFetch: FetchLike): FetchLike {
  return async (url: string | URL, init?: RequestInit) => {
    const method = (init?.method ?? 'GET').toUpperCase()
    
    // Skip timeout for GET (long-lived SSE streams)
    if (method === 'GET') {
      return baseFetch(url, init)
    }
    
    // Apply 60-second timeout to POST/PUT/DELETE
    const controller = new AbortController()
    const timer = setTimeout(
      c => c.abort(new DOMException('The operation timed out.', 'TimeoutError')),
      MCP_REQUEST_TIMEOUT_MS,  // 60 seconds
      controller,
    )
    timer.unref?.()
    
    // Normalize headers with required Accept
    const headers = new Headers(init?.headers)
    if (!headers.has('accept')) {
      headers.set('accept', MCP_STREAMABLE_HTTP_ACCEPT)
    }
    
    return baseFetch(url, { ...init, headers, signal: controller.signal })
  }
}
```

**Why Both Accept Types:**
```
Accept: application/json, text/event-stream
```

Per MCP Streamable HTTP spec — servers may respond with either format depending on the operation.

### 2.4 WebSocket Transport

Bun and Node.js WebSocket support:

```typescript
// From client.ts lines 735-783
const sessionIngressToken = getSessionIngressAuthToken()
const combinedHeaders = await getMcpServerHeaders(name, serverRef)

const wsHeaders = {
  'User-Agent': getMCPUserAgent(),
  ...(sessionIngressToken && {
    Authorization: `Bearer ${sessionIngressToken}`,
  }),
  ...combinedHeaders,
}

let wsClient: WsClientLike
if (typeof Bun !== 'undefined') {
  // Bun native WebSocket
  wsClient = new globalThis.WebSocket(serverRef.url, {
    protocols: ['mcp'],
    headers: wsHeaders,
    proxy: getWebSocketProxyUrl(serverRef.url),
    tls: tlsOptions || undefined,
  } as unknown as string[])
} else {
  // Node.js ws package
  const wsModule = await import('ws')
  const WS = wsModule.default as unknown as new (...) => WsClientLike
  wsClient = new WS(serverRef.url, ['mcp'], {
    headers: wsHeaders,
    agent: getWebSocketProxyAgent(serverRef.url),
    ...(tlsOptions || {}),
  })
}

transport = new WebSocketTransport(wsClient)
```

### 2.5 Claude.ai Proxy Transport

Organization-managed MCP servers proxied through Claude.ai:

```typescript
// From client.ts lines 868-898
const proxyUrl = `${oauthConfig.MCP_PROXY_URL}${oauthConfig.MCP_PROXY_PATH.replace('{server_id}', serverRef.id)}`

const fetchWithAuth = createClaudeAiProxyFetch(globalThis.fetch)

const transportOptions: StreamableHTTPClientTransportOptions = {
  fetch: wrapFetchWithTimeout(fetchWithAuth),
  requestInit: {
    headers: {
      'User-Agent': getMCPUserAgent(),
      'X-Mcp-Client-Session-Id': getSessionId(),
    },
  },
}

transport = new StreamableHTTPClientTransport(new URL(proxyUrl), transportOptions)
```

**createClaudeAiProxyFetch (auth.ts):**
```typescript
export function createClaudeAiProxyFetch(innerFetch: FetchLike): FetchLike {
  return async (url, init) => {
    const doRequest = async () => {
      await checkAndRefreshOAuthTokenIfNeeded()
      const currentTokens = getClaudeAIOAuthTokens()
      if (!currentTokens) {
        throw new Error('No claude.ai OAuth token available')
      }
      const headers = new Headers(init?.headers)
      headers.set('Authorization', `Bearer ${currentTokens.accessToken}`)
      return { response: await innerFetch(url, { ...init, headers }), sentToken: currentTokens.accessToken }
    }
    
    const { response, sentToken } = await doRequest()
    if (response.status !== 401) {
      return response
    }
    
    // Retry on 401 (force-refresh OAuth)
    const tokenChanged = await handleOAuth401Error(sentToken).catch(() => false)
    if (!tokenChanged) {
      const now = getClaudeAIOAuthTokens()?.accessToken
      if (!now || now === sentToken) {
        return response  // Token unchanged, return 401
      }
    }
    
    // Retry with fresh token
    return (await doRequest()).response
  }
}
```

### 2.6 SDK Control Transport

In-process MCP servers running in SDK:

```typescript
// From SdkControlTransport.ts
export class SdkControlClientTransport implements Transport {
  constructor(
    private serverName: string,
    private sendMcpMessage: SendMcpMessageCallback,
  ) {}
  
  async send(message: JSONRPCMessage): Promise<void> {
    // Send via stdout to SDK process
    const response = await this.sendMcpMessage(this.serverName, message)
    // Pass response back to MCP client
    if (this.onmessage) {
      this.onmessage(response)
    }
  }
}
```

**Message Flow:**
```
CLI Process                          SDK Process
     │                                    │
     │  MCP Client                        │
     │     │                              │
     │     ▼                              │
     │  SdkControlClientTransport         │
     │     │                              │
     │     │ JSON-RPC                     │
     │     ├─────────────────────────────►│  stdout control_request
     │     │                              │     │
     │     │                              │     ▼
     │     │                              │  MCP Server
     │     │                              │     │
     │     │                              │     ▼
     │     │                              │  Response
     │     │                              │     │
     │     │ JSON-RPC Response            │     │
     │     ◄──────────────────────────────┤  stdin control_response
     │                                    │
```

### 2.7 In-Process Linked Transport

For running MCP servers within the same process:

```typescript
// From InProcessTransport.ts
class InProcessTransport implements Transport {
  private peer: InProcessTransport | undefined
  private closed = false
  
  async send(message: JSONRPCMessage): Promise<void> {
    if (this.closed) throw new Error('Transport closed')
    queueMicrotask(() => {
      this.peer?.onmessage?.(message)
    })
  }
  
  async close(): Promise<void> {
    this.closed = true
    this.onclose?.()
    this.peer?.onclose?.()
  }
}

export function createLinkedTransportPair(): [Transport, Transport] {
  const a = new InProcessTransport()
  const b = new InProcessTransport()
  a._setPeer(b)
  b._setPeer(a)
  return [a, b]
}
```

---

## 3. OAuth 2.0 Authentication

### 3.1 ClaudeAuthProvider

Centralized OAuth provider for MCP servers:

```typescript
// From auth.ts
export class ClaudeAuthProvider {
  constructor(
    private serverName: string,
    private serverRef: ScopedMcpServerConfig,
  ) {}
  
  async tokens(): Promise<OAuthTokens | null> {
    // Load tokens from secure storage (keychain)
    return loadServerTokens(this.serverName)
  }
  
  async authorize(): Promise<void> {
    // Initiate OAuth flow based on server type
    if (this.serverRef.oauth?.xaa) {
      await performXaaAuth(this.serverName, this.serverRef)
    } else {
      await performStandardOAuth(this.serverName, this.serverRef)
    }
  }
  
  async refresh(): Promise<void> {
    // Refresh expired tokens
    const tokens = await this.tokens()
    if (tokens?.refresh_token) {
      await refreshOAuthToken(this.serverName, tokens.refresh_token)
    }
  }
}
```

### 3.2 XAA (Cross-App Access) Flow

Enterprise Managed Authorization (SEP-990) — no browser consent screen:

```typescript
// From xaa.ts
/**
 * Full XAA flow: PRM → AS metadata → token-exchange → jwt-bearer → access_token
 */
export async function performCrossAppAccess(
  serverUrl: string,
  config: XaaConfig,
  serverName = 'xaa',
): Promise<XaaResult> {
  const fetchFn = makeXaaFetch(abortSignal)
  
  // Step 1: RFC 9728 PRM Discovery
  logMCPDebug(serverName, `XAA: discovering PRM for ${serverUrl}`)
  const prm = await discoverProtectedResource(serverUrl, { fetchFn })
  
  // Step 2: Try each AS for jwt-bearer support
  for (const asUrl of prm.authorization_servers) {
    const asMeta = await discoverAuthorizationServer(asUrl, { fetchFn })
    if (asMeta.grant_types_supported?.includes(JWT_BEARER_GRANT)) {
      break  // Found compatible AS
    }
  }
  
  // Step 3: RFC 8693 Token Exchange at IdP: id_token → ID-JAG
  logMCPDebug(serverName, `XAA: exchanging id_token for ID-JAG at IdP`)
  const jag = await requestJwtAuthorizationGrant({
    tokenEndpoint: config.idpTokenEndpoint,
    audience: asMeta.issuer,
    resource: prm.resource,
    idToken: config.idpIdToken,
    clientId: config.idpClientId,
    clientSecret: config.idpClientSecret,
    fetchFn,
  })
  
  // Step 4: RFC 7523 JWT Bearer Grant at AS: ID-JAG → access_token
  logMCPDebug(serverName, `XAA: exchanging ID-JAG for access_token at AS`)
  const tokens = await exchangeJwtAuthGrant({
    tokenEndpoint: asMeta.token_endpoint,
    assertion: jag.jwtAuthGrant,
    clientId: config.clientId,
    clientSecret: config.clientSecret,
    authMethod: 'client_secret_basic',  // or 'client_secret_post'
    fetchFn,
  })
  
  return { ...tokens, authorizationServerUrl: asMeta.issuer }
}
```

**XAA Flow Diagram:**
```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Client        │     │   IdP           │     │   AS (MCP)      │
│                 │     │   (Identity)    │     │   (Auth Server) │
└────────┬────────┘     └────────┬────────┘     └────────┬────────┘
         │                       │                       │
         │ 1. PRM Discovery      │                       │
         │──────────────────────►│                       │
         │   /.well-known/oauth-protected-resource       │
         │                       │                       │
         │ 2. AS Discovery       │                       │
         │──────────────────────►│                       │
         │   /.well-known/oauth-authorization-server     │
         │                       │                       │
         │ 3. Token Exchange     │                       │
         │   POST /token         │                       │
         │   grant_type=token-exchange                   │
         │   subject_token=id_token                      │
         │──────────────────────►│                       │
         │                       │                       │
         │ 4. ID-JAG Response    │                       │
         │   access_token (ID-JAG)                       │
         │◄──────────────────────│                       │
         │                       │                       │
         │ 5. JWT Bearer Grant   │                       │
         │   POST /token         │                       │
         │   grant_type=jwt-bearer                       │
         │   assertion=<ID-JAG>                          │
         │──────────────────────────────────────────────►│
         │                       │                       │
         │ 6. access_token       │                       │
         │◄──────────────────────────────────────────────│
```

### 3.3 OAuth Token Storage

Tokens stored in OS keychain:

```typescript
// From auth.ts
async function loadServerTokens(serverName: string): Promise<OAuthTokens | null> {
  const keychainKey = `mcp_oauth_${normalizeNameForMCP(serverName)}`
  const encrypted = await secureStorage.getItem(keychainKey)
  if (!encrypted) return null
  return JSON.parse(encrypted)
}

async function saveServerTokens(serverName: string, tokens: OAuthTokens): Promise<void> {
  const keychainKey = `mcp_oauth_${normalizeNameForMCP(serverName)}`
  const encrypted = JSON.stringify(tokens)
  await secureStorage.setItem(keychainKey, encrypted)
}
```

### 3.4 OAuth Callback Port

Dynamic port selection for OAuth redirect:

```typescript
// From oauthPort.ts
const REDIRECT_PORT_RANGE = 
  getPlatform() === 'windows' 
    ? { min: 39152, max: 49151 }
    : { min: 49152, max: 65535 }

export async function findAvailablePort(): Promise<number> {
  // Try configured port first
  const configuredPort = getMcpOAuthCallbackPort()
  if (configuredPort) return configuredPort
  
  // Random selection from valid range
  const maxAttempts = 100
  for (let attempt = 0; attempt < maxAttempts; attempt++) {
    const port = min + Math.floor(Math.random() * range)
    try {
      await testPort(port)
      return port
    } catch {
      continue  // Port in use
    }
  }
  
  // Fallback
  return REDIRECT_PORT_FALLBACK  // 3118
}

export function buildRedirectUri(port: number): string {
  return `http://localhost:${port}/callback`
}
```

**RFC 8252 Compliance:** Loopback redirect URIs match any port as long as path matches.

---

## 4. Tool Discovery & Execution

### 4.1 Tool Discovery

When MCP client connects, tools are discovered and wrapped:

```typescript
// From client.ts
const toolsResult: ListToolsResult = await client.listTools()

for (const tool of toolsResult.tools) {
  const wrappedTool = {
    ...MCPTool,  // Base template
    name: buildMcpToolName(serverName, tool.name),
    description: () => truncateDescription(tool.description, MAX_MCP_DESCRIPTION_LENGTH),
    inputSchema: tool.inputSchema,
    call: async (input, opts) => {
      return callMcpTool(client, serverName, tool.name, input, opts)
    },
    isMcp: true,
    originalToolName: tool.name,
  }
  
  tools.push(buildTool(wrappedTool))
}
```

### 4.2 MCPTool Template

Base tool definition extended for each discovered tool:

```typescript
// From tools/MCPTool/MCPTool.ts
export const MCPTool = buildTool({
  isMcp: true,
  isOpenWorld: () => false,
  name: 'mcp',  // Overridden per-tool
  maxResultSizeChars: 100_000,
  
  async description() {
    return DESCRIPTION  // Overridden per-tool
  },
  
  async prompt() {
    return PROMPT  // Overridden per-tool
  },
  
  inputSchema: z.object({}).passthrough(),  // Dynamic per-tool
  
  async call() {
    return { data: '' }  // Overridden per-tool
  },
  
  async checkPermissions(): Promise<PermissionResult> {
    return { behavior: 'passthrough', message: 'MCPTool requires permission.' }
  },
  
  renderToolUseMessage,
  userFacingName: () => 'mcp',  // Overridden per-tool
  renderToolUseProgressMessage,
  renderToolResultMessage,
})
```

### 4.3 Tool Call Execution

```typescript
// From client.ts
async function callMcpTool(
  client: Client,
  serverName: string,
  toolName: string,
  input: Record<string, unknown>,
  opts: { abortSignal?: AbortSignal },
): Promise<{ data: string }> {
  const timeout = getMcpToolTimeoutMs()  // Default: ~27.8 hours
  
  const controller = createAbortController({
    timeoutMs: timeout,
    signal: opts.abortSignal,
  })
  
  try {
    const result = await client.callTool(
      { name: toolName, arguments: input },
      CallToolResultSchema,
      { signal: controller.signal },
    )
    
    // Handle error results
    if (result.isError) {
      throw new McpToolCallError(
        'Tool execution failed',
        `MCP tool ${toolName} returned isError: true`,
        result,
      )
    }
    
    // Format result content
    const formatted = formatMcpContent(result.content, serverName, toolName)
    return { data: formatted }
  } catch (error) {
    if (isMcpSessionExpiredError(error)) {
      throw new McpSessionExpiredError(serverName)
    }
    throw error
  }
}
```

### 4.4 Tool Result Formatting

```typescript
// From client.ts
function formatMcpContent(
  content: ContentBlock[],
  serverName: string,
  toolName: string,
): string {
  const parts: string[] = []
  
  for (const block of content) {
    switch (block.type) {
      case 'text':
        parts.push(block.text)
        break
      
      case 'image':
        // Persist image and return reference
        const saved = await persistBinaryContent(block.data, block.mimeType)
        parts.push(getBinaryBlobSavedMessage(saved.filePath, block.mimeType))
        break
      
      case 'resource':
        // Format resource reference
        parts.push(formatResourceBlock(block.resource, serverName))
        break
      
      case 'audio':
      case 'video':
        parts.push(`[${block.mimeType} content not displayed]`)
        break
    }
  }
  
  // Check for truncation
  const result = parts.join('\n')
  if (mcpContentNeedsTruncation(result)) {
    return truncateMcpContentIfNeeded(result, getLargeOutputInstructions())
  }
  
  return result
}
```

### 4.5 Tool Result Persistence

```typescript
// Persist large tool results to disk
async function persistToolResultIfNeeded(
  serverName: string,
  toolName: string,
  result: string,
): Promise<string> {
  const estimate = getContentSizeEstimate(result)
  
  if (estimate > THRESHOLD) {
    const filePath = join(
      getClaudeConfigHomeDir(),
      'mcp-tool-results',
      `${serverName}-${toolName}-${Date.now()}.md`,
    )
    
    await mkdir(dirname(filePath), { recursive: true })
    await writeFile(filePath, result)
    
    return `Tool result persisted to: ${filePath}\n\n${getFormatDescription(filePath)}`
  }
  
  return result
}
```

---

## 5. Resource Access

### 5.1 Resource Discovery

```typescript
// From client.ts
const resourcesResult = await client.listResources()
const serverResources: ServerResource[] = resourcesResult.resources.map(r => ({
  ...r,
  server: serverName,
}))

// Group by URI
const resourcesByUri = new Map<string, ServerResource[]>()
for (const resource of serverResources) {
  const existing = resourcesByUri.get(resource.uri) || []
  resourcesByUri.set(resource.uri, [...existing, resource])
}
```

### 5.2 ReadMcpResourceTool

Tool for reading MCP resources:

```typescript
// From tools/ReadMcpResourceTool/ReadMcpResourceTool.ts
export const ReadMcpResourceTool = buildTool({
  name: 'read_mcp_resource',
  description: () => 'Read content from an MCP resource URI',
  inputSchema: z.object({
    uri: z.string().describe('Resource URI to read'),
    server: z.string().optional().describe('MCP server name'),
  }),
  
  async call(input) {
    const resource = await client.readResource({ uri: input.uri })
    return { data: formatResourceContent(resource.contents) }
  },
  
  isMcp: true,
  isReadOnly: () => true,
})
```

### 5.3 ListMcpResourcesTool

```typescript
// From tools/ListMcpResourcesTool/ListMcpResourcesTool.ts
export const ListMcpResourcesTool = buildTool({
  name: 'list_mcp_resources',
  description: () => 'List available resources from MCP servers',
  inputSchema: z.object({
    server: z.string().optional().describe('Filter by server name'),
  }),
  
  async call(input) {
    const resources = input.server 
      ? resourcesByServer.get(input.server) || []
      : [...resourcesByServer.values()].flat()
    
    return { 
      data: resources.map(r => `- ${r.uri}: ${r.name}`).join('\n') 
    }
  },
  
  isMcp: true,
  isReadOnly: () => true,
})
```

---

## 6. Elicitation (User Confirmation)

### 6.1 Elicitation Request Handler

MCP servers can request user confirmation via forms or URLs:

```typescript
// From elicitationHandler.ts
export function registerElicitationHandler(
  client: Client,
  serverName: string,
  setAppState: (f: (prevState: AppState) => AppState) => void,
): void {
  client.setRequestHandler(ElicitRequestSchema, async (request, extra) => {
    const mode = request.params.mode === 'url' ? 'url' : 'form'
    
    logEvent('tengu_mcp_elicitation_shown', { mode })
    
    // Run hooks first (may provide programmatic response)
    const hookResponse = await runElicitationHooks(serverName, request.params, extra.signal)
    if (hookResponse) {
      return hookResponse
    }
    
    // Show UI to user
    return new Promise<ElicitResult>(resolve => {
      setAppState(prev => ({
        ...prev,
        elicitation: {
          queue: [
            ...prev.elicitation.queue,
            {
              serverName,
              requestId: extra.requestId,
              params: request.params,
              signal: extra.signal,
              respond: (result: ElicitResult) => {
                resolve(result)
              },
            },
          ],
        },
      }))
    })
  })
}
```

### 6.2 Elicitation Modes

**Form Mode:**
```typescript
{
  mode: 'form',
  message: 'Confirm this action',
  requestedSchema: {
    type: 'object',
    properties: {
      confirm: { type: 'boolean' },
      reason: { type: 'string' },
    },
  },
}
```

**URL Mode:**
```typescript
{
  mode: 'url',
  message: 'Visit URL to confirm',
  url: 'https://auth.example.com/confirm?token=xyz',
  elicitationId: 'elicitation-123',
}
```

### 6.3 Elicitation Completion

URL-based elicitations send completion notifications:

```typescript
client.setNotificationHandler(ElicitationCompleteNotificationSchema, notification => {
  const { elicitationId } = notification.params
  
  setAppState(prev => {
    const idx = findElicitationInQueue(prev.elicitation.queue, serverName, elicitationId)
    if (idx === -1) return prev
    
    const queue = [...prev.elicitation.queue]
    queue[idx] = { ...queue[idx]!, completed: true }
    return { ...prev, elicitation: { queue } }
  })
})
```

---

## 7. Claude.ai MCP Servers

### 7.1 Fetching Organization MCP Servers

```typescript
// From claudeai.ts
export const fetchClaudeAIMcpConfigsIfEligible = memoize(
  async (): Promise<Record<string, ScopedMcpServerConfig>> => {
    // Check eligibility
    if (!process.env.ENABLE_CLAUDEAI_MCP_SERVERS) return {}
    
    const tokens = getClaudeAIOAuthTokens()
    if (!tokens?.accessToken || !tokens.scopes?.includes('user:mcp_servers')) {
      return {}  // Not eligible
    }
    
    // Fetch from Claude.ai
    const url = `${getOauthConfig().BASE_API_URL}/v1/mcp_servers?limit=1000`
    const response = await axios.get(url, {
      headers: {
        Authorization: `Bearer ${tokens.accessToken}`,
        'anthropic-beta': 'mcp-servers-2025-12-04',
      },
    })
    
    // Build configs
    const configs: Record<string, ScopedMcpServerConfig> = {}
    for (const server of response.data.data) {
      const name = `claude.ai ${server.display_name}`
      configs[name] = {
        type: 'claudeai-proxy',
        url: server.url,
        id: server.id,
        scope: 'claudeai',
      }
    }
    
    return configs
  },
)
```

### 7.2 Proxy Connection Flow

```
Claude Code Client              Claude.ai API                MCP Server
      │                            │                            │
      │  GET /v1/mcp_servers       │                            │
      │───────────────────────────►│                            │
      │                            │                            │
      │  Response: server list     │                            │
      │◄───────────────────────────│                            │
      │                            │                            │
      │  POST /mcp/{id}/mcp        │                            │
      │  (Streamable HTTP)         │                            │
      │───────────────────────────►│                            │
      │                            │  Forward to MCP server     │
      │                            │───────────────────────────►│
      │                            │                            │
      │                            │  Response                  │
      │                            │◄───────────────────────────│
      │  Response                  │                            │
      │◄───────────────────────────│                            │
```

---

## 8. Permission Handling

### 8.1 Permission Request Flow

```typescript
// From client.ts
async function handlePermissionRequest(
  request: SDKControlPermissionRequest,
): Promise<void> {
  // Create synthetic assistant message for UI
  const assistantMessage = createSyntheticAssistantMessage(request, requestId)
  
  // Create tool stub if not loaded locally
  const tool = getTool(request.tool_name) || createToolStub(request.tool_name)
  
  // Show permission prompt
  const result = await showPermissionPrompt(assistantMessage, tool, request.input)
  
  // Send response
  respondToPermissionRequest(requestId, {
    behavior: result.allowed ? 'allow' : 'deny',
    updatedInput: result.input,
    message: result.denialReason,
  })
}
```

### 8.2 Synthetic Messages for Remote MCP

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
      role: 'assistant',
      content: [{
        type: 'tool_use',
        id: request.tool_use_id,
        name: request.tool_name,
        input: request.input,
      }],
    },
    timestamp: new Date().toISOString(),
  }
}

export function createToolStub(toolName: string): Tool {
  return {
    name: toolName,
    inputSchema: {},
    isEnabled: () => true,
    userFacingName: () => toolName,
    renderToolUseMessage: (input) => {
      return Object.entries(input).slice(0, 3).map(...).join(', ')
    },
    call: async () => ({ data: '' }),
    isMcp: true,
    needsPermissions: () => true,
  }
}
```

---

## 9. Connection Management

### 9.1 Batch Connection

```typescript
// From client.ts
export function getMcpServerConnectionBatchSize(): number {
  return parseInt(process.env.MCP_SERVER_CONNECTION_BATCH_SIZE || '', 10) || 3
}

function getRemoteMcpServerConnectionBatchSize(): number {
  return parseInt(process.env.MCP_REMOTE_SERVER_CONNECTION_BATCH_SIZE || '', 10) || 20
}

// Parallel connection with concurrency limit
await pMap(
  serverEntries,
  async ([name, config]) => {
    const connection = await connectToServer(name, config)
    return { name, connection }
  },
  { concurrency: batchSize },
)
```

### 9.2 Reconnection Logic

```typescript
// From client.ts
export const connectToServer = memoize(
  async (name: string, serverRef: ScopedMcpServerConfig): Promise<MCPServerConnection> => {
    try {
      // ... transport setup ...
      
      await client.connect(transport, {
        timeout: getConnectionTimeoutMs(),
      })
      
      return {
        client,
        name,
        type: 'connected',
        capabilities: client.getServerCapabilities()!,
        config: serverRef,
        cleanup: async () => {
          await client.close()
        },
      }
    } catch (error) {
      if (error instanceof UnauthorizedError) {
        return { name, type: 'needs-auth', config: serverRef }
      }
      return { 
        name, 
        type: 'failed', 
        config: serverRef, 
        error: errorMessage(error) 
      }
    }
  },
)
```

### 9.3 Session Expiry Detection

```typescript
// From client.ts
export function isMcpSessionExpiredError(error: Error): boolean {
  const httpStatus = 'code' in error ? (error as { code?: number }).code : undefined
  if (httpStatus !== 404) return false
  
  // MCP servers return: {"error":{"code":-32001,"message":"Session not found"}}
  return (
    error.message.includes('"code":-32001') ||
    error.message.includes('"code": -32001')
  )
}

class McpSessionExpiredError extends Error {
  constructor(serverName: string) {
    super(`MCP server "${serverName}" session expired`)
    this.name = 'McpSessionExpiredError'
  }
}
```

---

## 10. Auth Cache

### 10.1 Needs-Auth Cache

Prevents repeated auth attempts for servers that need authentication:

```typescript
// From client.ts
const MCP_AUTH_CACHE_TTL_MS = 15 * 60 * 1000  // 15 minutes

function getMcpAuthCachePath(): string {
  return join(getClaudeConfigHomeDir(), 'mcp-needs-auth-cache.json')
}

async function isMcpAuthCached(serverId: string): Promise<boolean> {
  const cache = await getMcpAuthCache()
  const entry = cache[serverId]
  if (!entry) return false
  return Date.now() - entry.timestamp < MCP_AUTH_CACHE_TTL_MS
}

async function setMcpAuthCacheEntry(serverId: string): Promise<void> {
  const cache = await getMcpAuthCache()
  cache[serverId] = { timestamp: Date.now() }
  await writeFile(getMcpAuthCachePath(), jsonStringify(cache))
  authCachePromise = null  // Invalidate read cache
}

export function clearMcpAuthCache(): void {
  authCachePromise = null
  void unlink(getMcpAuthCachePath()).catch(() => {})
}
```

---

## 11. Integration Points

| Module | Integration |
|--------|-------------|
| `services/analytics/` | Event logging (tengu_mcp_*) |
| `tools/MCPTool/` | Tool wrapper implementation |
| `tools/ListMcpResourcesTool/` | Resource listing tool |
| `tools/ReadMcpResourceTool/` | Resource reading tool |
| `tools/McpAuthTool/` | Auth management tool |
| `utils/hooks/` | Elicitation hooks |
| `utils/secureStorage/` | OAuth token keychain storage |
| `bootstrap/state/` | Session ID, original cwd |
| `commands/mcp/` | CLI commands |

---

## 12. CLI Commands

```typescript
// From commands/mcp/index.ts
const mcp = {
  type: 'local-jsx',
  name: 'mcp',
  description: 'Manage MCP servers',
  immediate: true,
  argumentHint: '[enable|disable [server-name]]',
  load: () => import('./mcp.js'),
}
```

**Commands:**
- `/mcp` — List all MCP servers
- `/mcp enable <server>` — Enable a disabled server
- `/mcp disable <server>` — Disable a server
- `/mcp add <config>` — Add new server config

---

## 13. Summary

Claude Code's MCP implementation provides:

| Feature | Description |
|---------|-------------|
| **8 Transport Types** | stdio, SSE, HTTP, WebSocket, SDK, claude.ai-proxy, SSE-IDE, WS-IDE |
| **OAuth 2.0** | Standard flow + XAA enterprise managed authorization |
| **Tool Discovery** | Automatic tool registration from connected servers |
| **Resource Access** | List and read MCP resources via dedicated tools |
| **Elicitation** | Form and URL-based user confirmation |
| **Claude.ai Integration** | Organization-managed servers via proxy |
| **SDK MCP** | In-process MCP servers for extensions |
| **Permission System** | Synthetic messages for remote permission prompts |
| **Auth Caching** | 15-minute needs-auth cache to prevent spam |
| **Batch Connection** | Concurrent server connections (3 local, 20 remote) |
| **Token Refresh** | Proactive OAuth token refresh before expiry |

**Key Files:**
- `services/mcp/client.ts` — Main client orchestration (~2,500 lines)
- `services/mcp/auth.ts` — OAuth provider implementation
- `services/mcp/xaa.ts` — Cross-App Access flow
- `services/mcp/config.ts` — Configuration schemas
- `services/mcp/elicitationHandler.ts` — User confirmation handling
- `services/mcp/InProcessTransport.ts` — Linked transport pair
- `services/mcp/SdkControlTransport.ts` — SDK bridge transport
- `services/mcp/claudeai.ts` — Claude.ai proxy integration

---

**Created:** 2026-04-07  
**Status:** Complete
