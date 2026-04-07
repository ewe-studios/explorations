# Claude Code Services Module — Deep-Dive Exploration

**Module:** `services/`  
**Parent Project:** [index.md](../index.md)  
**Created:** 2026-04-07  
**Files:** 130 TypeScript/TSX files  
**Total Lines:** ~53,680 lines

---

## 1. Module Overview

The `services/` module implements **long-running background processes and backend integration services** for Claude Code. It provides the infrastructure for API communication, MCP server management, OAuth authentication, analytics/telemetry, settings synchronization, and various specialized services like LSP, voice input, and session compaction.

### Core Responsibilities

1. **API Services** — HTTP client infrastructure for Anthropic API and backend services:
   - Request/response handling with retry logic
   - Rate limiting and quota management
   - Error classification and recovery
   - Token counting and usage tracking

2. **MCP Services** — Model Context Protocol integration:
   - Server connection lifecycle management
   - Tool discovery and normalization
   - OAuth authentication for MCP servers
   - Channel permissions and elicitation handling

3. **Analytics Services** — Telemetry and feature flagging:
   - GrowthBook integration for A/B testing
   - Datadog event streaming
   - First-party event logging
   - Event sampling and killswitches

4. **OAuth Services** — Authentication infrastructure:
   - OAuth 2.0 PKCE flow implementation
   - Token management and refresh
   - Secure storage integration
   - Profile and subscription info

5. **Settings Services** — Configuration synchronization:
   - Remote settings sync (CLI ↔ CCR)
   - Team memory synchronization
   - Enterprise managed settings
   - Cache management

6. **Specialized Services** — Domain-specific functionality:
   - LSP (Language Server Protocol) integration
   - Session compaction and context management
   - Voice input and audio capture
   - Plugin lifecycle management
   - Tips and onboarding

### Key Design Patterns

- **Singleton Services**: OAuth, GrowthBook, MCP ConnectionManager
- **Event-Driven Architecture**: Analytics sinks, state change listeners
- **Retry with Backoff**: API calls, settings sync, team memory operations
- **Coalescing/Batching**: Analytics events, settings uploads, stream deltas
- **Feature-Gated Services**: GrowthBook gates control service availability

---

## 2. File Inventory

### 2.1 API Services (21 files)

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `api/adminRequests.ts` | 119 | `submitAdminRequest()` | Admin-only API endpoint access |
| `api/bootstrap.ts` | 141 | `checkApiConnectivity()` | Startup API connectivity checks |
| `api/claude.ts` | 3419 | `queryModelWithStreaming()`, `getMaxOutputTokensForModel()` | Core API client with streaming, retries, fallbacks |
| `api/client.ts` | 389 | `getAnthropicClient()` | Client factory for direct API/Bedrock/Foundry/Vertex |
| `api/dumpPrompts.ts` | 226 | `dumpPrompts()` | Debug prompt export functionality |
| `api/emptyUsage.ts` | 22 | `EMPTY_USAGE` | Zero-usage constant for error paths |
| `api/errors.ts` | 1207 | `isPromptTooLongMessage()`, `getPromptTooLongTokenGap()` | Error classification and message formatting |
| `api/errorUtils.ts` | 260 | `formatAPIError()`, `extractConnectionErrorDetails()` | Error detail extraction utilities |
| `api/filesApi.ts` | 748 | `uploadFile()`, `downloadFile()` | File upload/download via API |
| `api/firstTokenDate.ts` | 60 | `getFirstTokenDate()` | First-token timing tracking |
| `api/grove.ts` | 357 | `queryGrove()` | Grove vector database integration |
| `api/logging.ts` | 788 | `logAPIQuery()`, `logAPISuccessAndDuration()` | API request/response logging |
| `api/metricsOptOut.ts` | 159 | `checkMetricsOptOut()` | Privacy/metrics opt-out handling |
| `api/overageCreditGrant.ts` | 137 | `checkOverageCredit()` | Overage credit eligibility checks |
| `api/promptCacheBreakDetection.ts` | 727 | `checkResponseForCacheBreak()` | Prompt caching health detection |
| `api/referral.ts` | 281 | `submitReferral()` | Referral program integration |
| `api/sessionIngress.ts` | 514 | `sendSessionIngress()` | Session ingress event streaming |
| `api/ultrareviewQuota.ts` | 38 | `checkUltrareviewQuota()` | Ultrareview quota checks |
| `api/usage.ts` | 63 | `getUsage()` | Usage statistics retrieval |
| `api/withRetry.ts` | 822 | `withRetry()` | Exponential backoff retry wrapper |

### 2.2 MCP Services (17 files)

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `mcp/auth.ts` | 2465 | `ClaudeAuthProvider`, `wrapFetchWithStepUpDetection()` | MCP OAuth authentication |
| `mcp/channelAllowlist.ts` | 76 | `isChannelAllowed()` | MCP channel access control |
| `mcp/channelNotification.ts` | 316 | `ChannelMessageNotificationSchema` | Channel message handling |
| `mcp/channelPermissions.ts` | 240 | `createChannelPermissionCallbacks()` | Channel permission relay |
| `mcp/claudeai.ts` | 164 | `fetchClaudeAIMcpConfigsIfEligible()` | Claude.ai connector configs |
| `mcp/client.ts` | 3348 | `getMcpToolsCommandsAndResources()`, `MCPTool`, `MCPProgress` | MCP client core |
| `mcp/config.ts` | 1578 | `getAllMcpConfigs()`, `dedupPluginMcpServers()` | MCP configuration management |
| `mcp/elicitationHandler.ts` | 313 | `runElicitationHooks()` | MCP elicitation handling |
| `mcp/envExpansion.ts` | 38 | `expandEnvVarsInString()` | Environment variable expansion |
| `mcp/headersHelper.ts` | 138 | `getMcpServerHeaders()` | Dynamic header generation |
| `mcp/InProcessTransport.ts` | 63 | `InProcessTransport` | In-process MCP transport |
| `mcp/MCPConnectionManager.tsx` | 72 | `MCPConnectionManager` | Connection lifecycle (React) |
| `mcp/mcpStringUtils.ts` | 106 | `buildMcpToolName()` | MCP string utilities |
| `mcp/normalization.ts` | 23 | `normalizeNameForMCP()` | Name normalization |
| `mcp/oauthPort.ts` | 78 | `findAvailablePort()` | OAuth callback port finder |
| `mcp/officialRegistry.ts` | 72 | `OFFICIAL_MCP_REGISTRY` | Official MCP server registry |
| `mcp/SdkControlTransport.ts` | 136 | `SdkControlClientTransport` | SDK control protocol transport |
| `mcp/types.ts` | 258 | `McpServerConfig`, `ConnectedMCPServer` | MCP type definitions |
| `mcp/useManageMCPConnections.ts` | 1141 | `useManageMCPConnections()` | React hook for MCP management |
| `mcp/utils.ts` | 575 | `commandBelongsToServer()`, `getMcpServerScopeFromToolName()` | MCP utilities |
| `mcp/vscodeSdkMcp.ts` | 112 | `createVSCodeSdkMcp()` | VSCode SDK MCP integration |
| `mcp/xaa.ts` | 511 | `performCrossAppAccess()` | Cross-app access (SEP-990) |
| `mcp/xaaIdpLogin.ts` | 487 | `acquireIdpIdToken()` | XAA IdP login flow |

### 2.3 Analytics Services (8 files)

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `analytics/config.ts` | 38 | `ANALYTICS_ENDPOINT` | Analytics endpoint configuration |
| `analytics/datadog.ts` | 307 | `DatadogSink` | Datadog event streaming |
| `analytics/firstPartyEventLogger.ts` | 449 | `FirstPartyEventLogger` | 1P event logging provider |
| `analytics/firstPartyEventLoggingExporter.ts` | 806 | `exportEvents()` | Event export pipeline |
| `analytics/growthbook.ts` | 1155 | `getFeatureValue_CACHED_MAY_BE_STALE()`, `onGrowthBookRefresh()` | GrowthBook client |
| `analytics/index.ts` | 173 | `logEvent()`, `attachAnalyticsSink()` | Analytics public API |
| `analytics/metadata.ts` | 973 | `extractToolInputForTelemetry()` | Metadata extraction utilities |
| `analytics/sink.ts` | 114 | `createAnalyticsSink()` | Analytics sink creation |
| `analytics/sinkKillswitch.ts` | 25 | `isSinkKilled()` | Killswitch for analytics |

### 2.4 OAuth Services (5 files)

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `oauth/auth-code-listener.ts` | 211 | `AuthCodeListener` | Local server for OAuth callbacks |
| `oauth/client.ts` | 566 | `exchangeCodeForTokens()`, `fetchProfileInfo()` | OAuth client operations |
| `oauth/crypto.ts` | 23 | `generateCodeVerifier()`, `generateCodeChallenge()` | PKCE cryptographic utilities |
| `oauth/getOauthProfile.ts` | 53 | `getOauthProfile()` | Profile info retrieval |
| `oauth/index.ts` | 198 | `OAuthService` | OAuth service main class |

### 2.5 Settings Sync Services (8 files)

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `remoteManagedSettings/index.ts` | 638 | `loadRemoteManagedSettings()`, `waitForRemoteManagedSettingsToLoad()` | Enterprise managed settings |
| `remoteManagedSettings/securityCheck.tsx` | 73 | `checkManagedSettingsSecurity()` | Security validation |
| `remoteManagedSettings/syncCache.ts` | 112 | `isRemoteManagedSettingsEligible()` | Eligibility and caching |
| `remoteManagedSettings/syncCacheState.ts` | 96 | `getSettingsPath()` | Cache state management |
| `remoteManagedSettings/types.ts` | 31 | `RemoteManagedSettingsFetchResult` | Type definitions |
| `settingsSync/index.ts` | 581 | `uploadUserSettingsInBackground()`, `downloadUserSettings()` | Settings sync service |
| `settingsSync/types.ts` | 67 | `SettingsSyncUploadResult` | Type definitions |
| `teamMemorySync/index.ts` | 1256 | `pullTeamMemory()`, `pushTeamMemory()` | Team memory sync |
| `teamMemorySync/secretScanner.ts` | 324 | `scanForSecrets()` | Secret scanning before upload |
| `teamMemorySync/teamMemSecretGuard.ts` | 44 | `isTeamMemPath()` | Team memory path guards |
| `teamMemorySync/types.ts` | 156 | `TeamMemorySyncPushResult` | Type definitions |
| `teamMemorySync/watcher.ts` | 387 | `createTeamMemoryWatcher()` | File watcher for auto-sync |

### 2.6 Session Compaction Services (10 files)

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `compact/apiMicrocompact.ts` | 153 | `getAPIContextManagement()` | API-side context management |
| `compact/autoCompact.ts` | 351 | `shouldAutoCompact()` | Automatic compaction triggers |
| `compact/compact.ts` | 1705 | `compactMessages()`, `compactMessagesWithStreaming()` | Core compaction logic |
| `compact/compactWarningHook.ts` | 16 | `compactWarningHook()` | Pre-compact warning |
| `compact/compactWarningState.ts` | 18 | `CompactWarningState` | Warning state management |
| `compact/grouping.ts` | 63 | `groupMessagesByApiRound()` | Message grouping for compaction |
| `compact/microCompact.ts` | 530 | `pinCacheEdits()`, `consumePendingCacheEdits()` | Micro-compaction |
| `compact/postCompactCleanup.ts` | 77 | `postCompactCleanup()` | Post-compaction restoration |
| `compact/prompt.ts` | 374 | `getCompactPrompt()` | Compaction prompt generation |
| `compact/sessionMemoryCompact.ts` | 630 | `sessionMemoryCompact()` | Session-aware compaction |
| `compact/timeBasedMCConfig.ts` | 43 | `getTimeBasedMCConfig()` | Time-based config |

### 2.7 Plugin Services (3 files)

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `plugins/pluginCliCommands.ts` | 344 | `pluginInstall()`, `pluginUninstall()` | CLI plugin commands |
| `plugins/PluginInstallationManager.ts` | 184 | `PluginInstallationManager` | Plugin installation lifecycle |
| `plugins/pluginOperations.ts` | 1088 | `installPlugin()`, `updatePlugin()` | Core plugin operations |

### 2.8 Tool Services (5 files)

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `tools/StreamingToolExecutor.ts` | 530 | `StreamingToolExecutor` | Streaming tool execution |
| `tools/toolExecution.ts` | 1745 | `executeTool()`, `classifyToolError()` | Core tool execution |
| `tools/toolHooks.ts` | 650 | `runPreToolUseHooks()`, `runPostToolUseHooks()` | Tool lifecycle hooks |
| `tools/toolOrchestration.ts` | 188 | `orchestrateTools()` | Tool orchestration logic |
| `toolUseSummary/toolUseSummaryGenerator.ts` | 112 | `generateToolUseSummary()` | Tool use summary generation |

### 2.9 LSP Services (7 files)

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `lsp/config.ts` | 79 | `getAllLspServers()` | LSP configuration loading |
| `lsp/LSPClient.ts` | 447 | `LSPClient` | LSP client implementation |
| `lsp/LSPDiagnosticRegistry.ts` | 386 | `LSPDiagnosticRegistry` | Diagnostic registration |
| `lsp/LSPServerInstance.ts` | 511 | `LSPServerInstance` | Individual server instance |
| `lsp/LSPServerManager.ts` | 420 | `createLSPServerManager()` | Multi-server manager |
| `lsp/manager.ts` | 289 | `initializeLSP()` | LSP initialization |
| `lsp/passiveFeedback.ts` | 328 | `passiveFeedback()` | Passive feedback from LSP |

### 2.10 Other Services (22 files)

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `AgentSummary/agentSummary.ts` | 179 | `generateAgentSummary()` | Agent summary generation |
| `autoDream/autoDream.ts` | 324 | `autoDream()` | AutoDream consolidation |
| `autoDream/config.ts` | 21 | `getAutoDreamConfig()` | AutoDream configuration |
| `autoDream/consolidationLock.ts` | 140 | `acquireConsolidationLock()` | Consolidation locking |
| `autoDream/consolidationPrompt.ts` | 65 | `getConsolidationPrompt()` | Consolidation prompts |
| `awaySummary.ts` | 74 | `generateAwaySummary()` | Away summary generation |
| `claudeAiLimits.ts` | 515 | `checkQuotaStatus()`, `emitStatusChange()` | Rate limit/quota tracking |
| `claudeAiLimitsHook.ts` | 23 | `claudeAiLimitsHook()` | Limits display hook |
| `diagnosticTracking.ts` | 397 | `trackDiagnostics()` | Diagnostic tracking |
| `extractMemories/extractMemories.ts` | 615 | `extractMemories()` | Memory extraction from sessions |
| `extractMemories/prompts.ts` | 154 | `getExtractMemoriesPrompt()` | Extraction prompts |
| `internalLogging.ts` | 90 | `logPermissionContextForAnts()` | Internal debug logging |
| `MagicDocs/magicDocs.ts` | 254 | `generateMagicDocs()` | Documentation generation |
| `MagicDocs/prompts.ts` | 127 | `getMagicDocsPrompt()` | MagicDocs prompts |
| `mcpServerApproval.tsx` | 40 | `McpServerApproval` | MCP approval UI |
| `mockRateLimits.ts` | 882 | `checkMockRateLimitError()` | Mock rate limits for testing |
| `notifier.ts` | 156 | `Notifier` | Desktop notification service |
| `policyLimits/index.ts` | 663 | `checkPolicyLimits()` | Policy-based limits |
| `policyLimits/types.ts` | 27 | `PolicyLimitsResult` | Policy types |
| `preventSleep.ts` | 165 | `preventSleep()` | System sleep prevention |
| `PromptSuggestion/promptSuggestion.ts` | 523 | `generatePromptSuggestion()` | Prompt suggestions |
| `PromptSuggestion/speculation.ts` | 991 | `speculate()` | Speculative prompts |
| `rateLimitMessages.ts` | 344 | `getRateLimitErrorMessage()` | Rate limit messaging |
| `rateLimitMocking.ts` | 144 | `processRateLimitHeaders()` | Rate limit mocking |
| `SessionMemory/prompts.ts` | 324 | `getSessionMemoryPrompt()` | Session memory prompts |
| `SessionMemory/sessionMemory.ts` | 495 | `processSessionMemory()` | Session memory processing |
| `SessionMemory/sessionMemoryUtils.ts` | 207 | `getSessionMemoryConfig()` | Session memory utilities |
| `tips/tipHistory.ts` | 17 | `getTipHistory()` | Tip history tracking |
| `tips/tipRegistry.ts` | 686 | `TIP_REGISTRY` | Tip definitions |
| `tips/tipScheduler.ts` | 58 | `scheduleTip()` | Tip scheduling |
| `tokenEstimation.ts` | 495 | `roughTokenCountEstimation()` | Token estimation |
| `vcr.ts` | 406 | `withVCR()` | Video capture/replay for testing |
| `voice.ts` | 525 | `checkVoiceDependencies()` | Voice input service |
| `voiceKeyterms.ts` | 112 | `VOICE_KEYTERMS` | Voice keyword detection |
| `voiceStreamSTT.ts` | 544 | `streamSTT()` | Streaming speech-to-text |

---

## 3. Service Categories and Architecture

### 3.1 API Services Layer

The API services layer handles all communication with Anthropic's API and backend services.

#### Core Architecture: `api/claude.ts` (3419 lines)

This is the central API service, responsible for:
- Querying models with streaming support
- Retry logic with exponential backoff
- Model fallback on repeated failures
- Fast mode handling and cooldown
- Token counting and usage tracking
- Prompt caching health detection

```typescript
// Key exports from api/claude.ts
export async function* withRetry<T>(
  getClient: () => Promise<Anthropic>,
  operation: (client: Anthropic, attempt: number, context: RetryContext) => Promise<T>,
  options: RetryOptions
): AsyncGenerator<SystemAPIErrorMessage, T>

export async function queryModelWithStreaming(
  options: QueryModelOptions
): AsyncGenerator<SystemAPIErrorMessage, QueryModelResult>

export function getMaxOutputTokensForModel(model: string, thinkingConfig?: ThinkingConfig): number
```

#### Retry Logic: `api/withRetry.ts` (822 lines)

Implements exponential backoff with:
- Configurable max retries (default: 10)
- Special handling for 529 (Overloaded) errors (max 3 retries)
- Fast mode cooldown on rejection
- Model fallback after repeated failures
- Persistent retry mode for unattended sessions

```typescript
// Retry context and options
export interface RetryContext {
  maxTokensOverride?: number
  model: string
  thinkingConfig: ThinkingConfig
  fastMode?: boolean
}

// Foreground sources that retry on 529
const FOREGROUND_529_RETRY_SOURCES = new Set<QuerySource>([
  'repl_main_thread',
  'sdk',
  'agent:custom',
  'agent:default',
  'compact',
  'hook_agent',
  'hook_prompt',
  'verification_agent',
  'side_question',
  'auto_mode',
  'bash_classifier',
])
```

#### Error Classification: `api/errors.ts` (1207 lines)

Comprehensive error handling with:
- Prompt-too-long detection and token gap extraction
- Media size error detection (images, PDFs)
- Authentication error handling (401, 403, token revoked)
- Rate limit error extraction
- CCR mode detection for appropriate messaging

```typescript
export function isPromptTooLongMessage(msg: AssistantMessage): boolean
export function getPromptTooLongTokenGap(msg: AssistantMessage): number | undefined
export function isMediaSizeError(raw: string): boolean
export function isMediaSizeErrorMessage(msg: AssistantMessage): boolean
```

#### Client Factory: `api/client.ts` (389 lines)

Creates Anthropic clients for different providers:
- **Direct API**: Standard Anthropic API with API key
- **Bedrock**: AWS Bedrock with IAM credentials
- **Foundry**: Azure Foundry with Azure AD
- **Vertex**: GCP Vertex AI with Google credentials

```typescript
export async function getAnthropicClient({
  apiKey,
  maxRetries,
  model,
  fetchOverride,
  source,
}): Promise<Anthropic>
```

---

### 3.2 MCP Services Layer

The MCP services layer implements the Model Context Protocol for integrating external tools and resources.

#### Core Client: `mcp/client.ts` (3348 lines)

Central MCP client managing:
- Server connections (stdio, SSE, HTTP, WebSocket, SDK)
- Tool discovery and registration
- Resource listing and reading
- Prompt listing and execution
- Progress notifications
- Error handling and reconnection

```typescript
// Key exports
export class McpAuthError extends Error {
  serverName: string
}

export class McpToolCallError_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS extends TelemetrySafeError {
  readonly mcpMeta?: { _meta?: Record<string, unknown> }
}

export function isMcpSessionExpiredError(error: Error): boolean

// Tool execution
export async function callMcpTool(
  serverName: string,
  toolName: string,
  input: unknown,
  abortSignal?: AbortSignal
): Promise<MCPToolResult>
```

#### Connection Management: `mcp/config.ts` (1578 lines)

Manages MCP configuration from multiple sources:
- `.mcp.json` (project-local)
- `.claude/settings.json` (user-level)
- Enterprise managed settings
- Claude.ai connectors
- Plugin-provided servers

```typescript
// Server signature for deduplication
export function getMcpServerSignature(config: McpServerConfig): string | null {
  const cmd = getServerCommandArray(config)
  if (cmd) {
    return `stdio:${jsonStringify(cmd)}`
  }
  const url = getServerUrl(config)
  if (url) {
    return `url:${unwrapCcrProxyUrl(url)}`
  }
  return null
}

// Deduplication: plugin servers vs manual config
export function dedupPluginMcpServers(
  pluginServers: Record<string, ScopedMcpServerConfig>,
  manualServers: Record<string, ScopedMcpServerConfig>
): {
  servers: Record<string, ScopedMcpServerConfig>
  suppressed: Array<{ name: string; duplicateOf: string }>
}
```

#### OAuth Authentication: `mcp/auth.ts` (2465 lines)

Implements OAuth 2.0 for MCP servers:
- RFC 9728/8414 metadata discovery
- Dynamic Client Registration (DCR)
- PKCE authorization code flow
- Token refresh with retry
- Cross-App Access (XAA/SEP-990)
- IdP integration for enterprise SSO

```typescript
export class ClaudeAuthProvider implements OAuthClientProvider {
  constructor(
    private readonly serverName: string,
    private readonly serverUrl: string,
    private readonly configuredMetadataUrl: string | undefined
  )

  async redirectToAuthorization(authUrl: URL): Promise<void>
  async saveCodeVerifier(codeVerifier: string): Promise<void>
  async saveToken(token: OAuthTokens): Promise<void>
  async loadTokens(): Promise<OAuthTokens | null>
}

// Normalizes non-standard OAuth error codes (e.g., Slack's invalid_refresh_token → invalid_grant)
export async function normalizeOAuthErrorBody(response: Response): Promise<Response>
```

#### React Hook: `mcp/useManageMCPConnections.ts` (1141 lines)

React hook for managing MCP connections in the UI:
- Connection lifecycle (connect, disconnect, reconnect)
- State synchronization with AppState
- Automatic reconnection with exponential backoff
- Error reporting to AppState
- Channel permission callbacks

```typescript
export function useManageMCPConnections(
  dynamicMcpConfig: Record<string, ScopedMcpServerConfig> | undefined,
  isStrictMcpConfig = false
): {
  reconnect: (serverName?: string) => Promise<void>
  reconnectingServers: Set<string>
}
```

---

### 3.3 Analytics Services Layer

The analytics services layer handles telemetry, feature flagging, and event logging.

#### GrowthBook Integration: `analytics/growthbook.ts` (1155 lines)

Feature flag client with:
- Remote evaluation from backend
- Disk caching for offline access
- Environment variable overrides (ant-only)
- Periodic refresh (default: 15 minutes)
- Exposure logging for experiment tracking

```typescript
// Cached feature value getter (primary API)
export function getFeatureValue_CACHED_MAY_BE_STALE<T>(
  feature: string,
  defaultValue: T
): T

// Listen for feature refreshes
export function onGrowthBookRefresh(listener: () => void | Promise<void>): () => void

// Config overrides (ant-only, runtime)
export function setGrowthBookConfigOverride(feature: string, value: unknown): void
export function getGrowthBookConfigOverrides(): Record<string, unknown>
```

#### Event Logging: `analytics/index.ts` (173 lines)

Public API for analytics events:
- Synchronous and asynchronous logging
- Event queuing before sink attachment
- PII stripping for general-access sinks
- Type-safe metadata markers

```typescript
// Marker types for PII safety
export type AnalyticsMetadata_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS = never
export type AnalyticsMetadata_I_VERIFIED_THIS_IS_PII_TAGGED = never

// Strip PII-tagged fields before general-access sinks
export function stripProtoFields<V>(metadata: Record<string, V>): Record<string, V>

// Event logging
export function logEvent(eventName: string, metadata: LogEventMetadata): void
export async function logEventAsync(eventName: string, metadata: LogEventMetadata): Promise<void>

// Sink attachment (called once during startup)
export function attachAnalyticsSink(newSink: AnalyticsSink): void
```

#### Datadog Sink: `analytics/datadog.ts` (307 lines)

Streams events to Datadog:
- Batch upload with configurable size
- Retry with exponential backoff
- Event deduplication
- Network error handling

#### First-Party Event Logger: `analytics/firstPartyEventLogger.ts` (449 lines)

Internal event logging pipeline:
- OpenTelemetry integration
- Batch export with configurable intervals
- Proto field handling for PII-tagged columns
- Experiment exposure logging

---

### 3.4 OAuth Services Layer

The OAuth services layer handles Claude.ai authentication.

#### Main Service: `oauth/index.ts` (198 lines)

Implements OAuth 2.0 PKCE flow:
- Automatic flow (localhost callback)
- Manual flow (copy/paste code)
- Token exchange and profile fetch
- Cleanup on completion

```typescript
export class OAuthService {
  private codeVerifier: string
  private authCodeListener: AuthCodeListener | null
  private manualAuthCodeResolver: ((authorizationCode: string) => void) | null

  async startOAuthFlow(
    authURLHandler: (url: string, automaticUrl?: string) => Promise<void>,
    options?: {
      loginWithClaudeAi?: boolean
      inferenceOnly?: boolean
      expiresIn?: number
      orgUUID?: string
      skipBrowserOpen?: boolean
    }
  ): Promise<OAuthTokens>

  handleManualAuthCodeInput(params: { authorizationCode: string; state: string }): void
  cleanup(): void
}
```

#### Auth Code Listener: `oauth/auth-code-listener.ts` (211 lines)

Local HTTP server for OAuth callbacks:
- Finds available port dynamically
- Handles automatic redirect
- Renders success/error pages
- Timeout handling

#### Client Operations: `oauth/client.ts` (566 lines)

OAuth API operations:
- Authorization URL building
- Code exchange for tokens
- Token refresh
- Profile info retrieval

---

### 3.5 Settings Sync Services Layer

The settings sync layer handles configuration synchronization across environments.

#### Settings Sync: `settingsSync/index.ts` (581 lines)

Syncs user settings between CLI and CCR:
- **Upload** (interactive CLI): Incremental upload of changed entries
- **Download** (CCR mode): Full download before plugin installation
- Fire-and-forget with retry logic
- File size limits (500KB per file)

```typescript
// Upload local settings (interactive CLI, fire-and-forget)
export async function uploadUserSettingsInBackground(): Promise<void>

// Download settings (CCR mode, cached promise)
export function downloadUserSettings(): Promise<boolean>

// Force fresh download (for /reload-plugins)
export function redownloadUserSettings(): Promise<boolean>
```

#### Team Memory Sync: `teamMemorySync/index.ts` (1256 lines)

Syncs team memory files with server:
- Checksum-based delta sync
- File watcher for auto-sync
- Secret scanning before upload
- Server-enforced entry limits
- ETag-based conditional requests

```typescript
export type SyncState = {
  lastKnownChecksum: string | null
  serverChecksums: Map<string, string>
  serverMaxEntries: number | null
}

export function createSyncState(): SyncState
export function hashContent(content: string): string

export async function pullTeamMemory(
  state: SyncState,
  repoSlug: string,
  signal?: AbortSignal
): Promise<TeamMemorySyncFetchResult>

export async function pushTeamMemory(
  state: SyncState,
  repoSlug: string
): Promise<TeamMemorySyncPushResult>
```

#### Remote Managed Settings: `remoteManagedSettings/index.ts` (638 lines)

Enterprise managed settings:
- Checksum-based caching
- Background polling (1-hour interval)
- Security validation
- Graceful degradation on failures

```typescript
// Initialize loading promise (called early in startup)
export function initializeRemoteManagedSettingsLoadingPromise(): void

// Wait for initial load to complete
export async function waitForRemoteManagedSettingsToLoad(): Promise<void>

// Eligibility check
export function isEligibleForRemoteManagedSettings(): boolean
```

---

### 3.6 Session Compaction Services

The compaction services manage conversation context size.

#### Core Compaction: `compact/compact.ts` (1705 lines)

Implements conversation compaction:
- Full compaction (entire conversation)
- Partial compaction (from cursor)
- Streaming compaction with retries
- Post-compaction restoration
- Image/document stripping
- Token budget management

```typescript
export interface CompactionResult {
  boundaryMarker: SystemMessage
  newHistory: Message[]
  usage: BetaUsage
  cost: number
}

export async function compactMessages(
  messages: Message[],
  canUseTool: CanUseToolFn,
  model: string,
  signal?: AbortSignal
): Promise<CompactionResult | null>

export async function compactMessagesWithStreaming(
  messages: Message[],
  canUseTool: CanUseToolFn,
  model: string,
  signal?: AbortSignal
): Promise<CompactionResult | null>
```

#### Micro-Compaction: `compact/microCompact.ts` (530 lines)

Fine-grained cache management:
- Pin/unpin cache edits
- Consume pending edits
- Tool state tracking

#### Auto-Compaction: `compact/autoCompact.ts` (351 lines)

Determines when to trigger compaction:
- Token threshold checks
- User activity detection
- Warning hooks

---

### 3.7 Plugin Services

The plugin services manage plugin lifecycle.

#### Plugin Operations: `plugins/pluginOperations.ts` (1088 lines)

Core plugin operations:
- Install from marketplace or local path
- Uninstall with dependency checking
- Enable/disable
- Update to latest version
- Scope management (user, project, local)

```typescript
export async function installPlugin(
  plugin: string,
  scope: InstallableScope,
  options?: { force?: boolean }
): Promise<PluginOperationResult>

export async function uninstallPlugin(
  plugin: string,
  options?: { purgeData?: boolean }
): Promise<PluginOperationResult>

export async function updatePlugin(plugin: string): Promise<PluginUpdateResult>
```

---

### 3.8 Tool Execution Services

The tool execution services handle tool invocation.

#### Core Execution: `tools/toolExecution.ts` (1745 lines)

Tool execution engine:
- Permission checking
- Hook execution (pre/post)
- Progress tracking
- Error classification
- Telemetry integration
- OTel span management

```typescript
export function classifyToolError(error: unknown): string

export async function executeTool(
  toolUse: ToolUseBlock,
  tools: Tool[],
  canUseTool: CanUseToolFn,
  abortSignal: AbortSignal,
  querySource?: QuerySource
): Promise<ToolResultBlockParam>
```

---

### 3.9 LSP Services

The LSP services provide IDE language server integration.

#### Server Manager: `lsp/LSPServerManager.ts` (420 lines)

Manages multiple LSP servers:
- Extension-based routing
- Server lifecycle (start/stop)
- File synchronization (open/change/save/close)
- Request routing

```typescript
export type LSPServerManager = {
  initialize(): Promise<void>
  shutdown(): Promise<void>
  getServerForFile(filePath: string): LSPServerInstance | undefined
  ensureServerStarted(filePath: string): Promise<LSPServerInstance | undefined>
  sendRequest<T>(filePath: string, method: string, params: unknown): Promise<T | undefined>
  openFile(filePath: string, content: string): Promise<void>
  changeFile(filePath: string, content: string): Promise<void>
  saveFile(filePath: string): Promise<void>
  closeFile(filePath: string): Promise<void>
}
```

---

## 4. Line-by-Line Analysis

### 4.1 OAuth Service Initialization (`oauth/index.ts`)

```typescript
export class OAuthService {
  private codeVerifier: string
  private authCodeListener: AuthCodeListener | null = null
  private port: number | null = null
  private manualAuthCodeResolver: ((authorizationCode: string) => void) | null = null

  constructor() {
    this.codeVerifier = crypto.generateCodeVerifier()
  }

  async startOAuthFlow(
    authURLHandler: (url: string, automaticUrl?: string) => Promise<void>,
    options?: { ... }
  ): Promise<OAuthTokens> {
    // 1. Create and start callback listener
    this.authCodeListener = new AuthCodeListener()
    this.port = await this.authCodeListener.start()

    // 2. Generate PKCE values
    const codeChallenge = crypto.generateCodeChallenge(this.codeVerifier)
    const state = crypto.generateState()

    // 3. Build both automatic and manual URLs
    const opts = { codeChallenge, state, port: this.port, ...options }
    const manualFlowUrl = client.buildAuthUrl({ ...opts, isManual: true })
    const automaticFlowUrl = client.buildAuthUrl({ ...opts, isManual: false })

    // 4. Wait for auth code (either flow)
    const authorizationCode = await this.waitForAuthorizationCode(state, async () => {
      if (options?.skipBrowserOpen) {
        await authURLHandler(manualFlowUrl, automaticFlowUrl)
      } else {
        await authURLHandler(manualFlowUrl)
        await openBrowser(automaticFlowUrl)
      }
    })

    // 5. Exchange code for tokens
    const tokenResponse = await client.exchangeCodeForTokens(
      authorizationCode, state, this.codeVerifier, this.port!,
      !isAutomaticFlow, options?.expiresIn
    )

    // 6. Fetch profile info
    const profileInfo = await client.fetchProfileInfo(tokenResponse.access_token)

    // 7. Handle success redirect (automatic flow only)
    if (isAutomaticFlow) {
      const scopes = client.parseScopes(tokenResponse.scope)
      this.authCodeListener?.handleSuccessRedirect(scopes)
    }

    return this.formatTokens(tokenResponse, profileInfo.subscriptionType, ...)
  }
}
```

**Key Patterns**:
- **Dual-flow support**: Automatic (localhost) and manual (copy/paste) flows run in parallel
- **State tracking**: Manual resolver is nulled when automatic completes, preventing race
- **Cleanup**: `finally` block ensures listener is closed even on error

---

### 4.2 Analytics Event Flow (`analytics/index.ts` → `analytics/sink.ts`)

```typescript
// 1. Event queued before sink attachment
export function logEvent(eventName: string, metadata: LogEventMetadata): void {
  if (sink === null) {
    eventQueue.push({ eventName, metadata, async: false })
    return
  }
  sink.logEvent(eventName, metadata)
}

// 2. Sink attachment drains queue
export function attachAnalyticsSink(newSink: AnalyticsSink): void {
  if (sink !== null) return  // Idempotent
  
  sink = newSink
  if (eventQueue.length > 0) {
    const queuedEvents = [...eventQueue]
    eventQueue.length = 0
    
    queueMicrotask(() => {
      for (const event of queuedEvents) {
        if (event.async) {
          void sink!.logEventAsync(event.eventName, event.metadata)
        } else {
          sink!.logEvent(event.eventName, event.metadata)
        }
      }
    })
  }
}

// 3. Sink routes to Datadog + 1P exporter
export function createAnalyticsSink(): AnalyticsSink {
  const datadogSink = createDatadogSink()
  const exporter = createFirstPartyEventExporter()
  
  return {
    logEvent(eventName, metadata) {
      // Strip PII for Datadog
      const sanitizedMetadata = stripProtoFields(metadata)
      datadogSink.logEvent(eventName, sanitizedMetadata)
      
      // Export with PII to 1P backend
      exporter.export({ eventName, metadata })
    },
    async logEventAsync(eventName, metadata) {
      // Same as above, async
    }
  }
}
```

**Key Patterns**:
- **Queue-and-drain**: Events logged before sink attachment are queued and drained asynchronously
- **Dual export**: Same event → Datadog (sanitized) + 1P backend (with PII)
- **Idempotent attachment**: Can be called from multiple entry points safely

---

### 4.3 MCP Connection Lifecycle (`mcp/useManageMCPConnections.ts`)

```typescript
export function useManageMCPConnections(
  dynamicMcpConfig: Record<string, ScopedMcpServerConfig> | undefined,
  isStrictMcpConfig = false
) {
  const setAppState = useSetAppState()
  const reconnectTimersRef = useRef<Map<string, NodeJS.Timeout>>(new Map())
  
  // Batched state updates (16ms window)
  const pendingUpdatesRef = useRef<PendingUpdate[]>([])
  const flushTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  
  const flushPendingUpdates = useCallback(() => {
    flushTimerRef.current = null
    const updates = pendingUpdatesRef.current
    if (updates.length === 0) return
    pendingUpdatesRef.current = []
    
    setAppState(prevState => {
      let mcp = prevState.mcp
      for (const update of updates) {
        const { tools, commands, resources, ...client } = update
        const prefix = getMcpPrefix(client.name)
        const existingClientIndex = mcp.clients.findIndex(c => c.name === client.name)
        
        if (existingClientIndex >= 0) {
          mcp.clients[existingClientIndex] = client as MCPServerConnection
        } else {
          mcp = { ...mcp, clients: [...mcp.clients, client] }
        }
      }
      return { ...prevState, mcp }
    })
  }, [setAppState])
  
  const updateServerState = useCallback((update: PendingUpdate) => {
    pendingUpdatesRef.current.push(update)
    if (!flushTimerRef.current) {
      flushTimerRef.current = setTimeout(flushPendingUpdates, MCP_BATCH_FLUSH_MS)
    }
  }, [flushPendingUpdates])
  
  // Connect to servers
  useEffect(() => {
    const connectAll = async () => {
      const configs = await getClaudeCodeMcpConfigs()
      const manualServers = configs.manualServers
      const pluginServers = configs.pluginServers
      const claudeAiServers = configs.claudeAiServers
      
      // Deduplication: manual wins over plugin, first-loaded wins among plugins
      const { servers: dedupedPluginServers, suppressed } = dedupPluginMcpServers(
        pluginServers,
        manualServers
      )
      
      const { servers: dedupedClaudeAiServers, suppressed: claudeAiSuppressed } = 
        dedupClaudeAiMcpServers(claudeAiServers, manualServers)
      
      // Connect to each server
      for (const [name, config] of Object.entries({
        ...dedupedPluginServers,
        ...dedupedClaudeAiServers,
        ...manualServers
      })) {
        connectToServer(name, config, updateServerState)
      }
    }
    
    connectAll()
  }, [dynamicMcpConfig])
  
  return { reconnect: handleReconnect, reconnectingServers }
}
```

**Key Patterns**:
- **Batched updates**: Multiple connection state changes coalesced into single `setAppState`
- **Deduplication**: Same server from multiple sources → single connection
- **Exponential backoff**: Reconnection uses 1s, 2s, 4s, 8s, 16s, 30s (capped)

---

### 4.4 Settings Sync Upload Flow (`settingsSync/index.ts`)

```typescript
export async function uploadUserSettingsInBackground(): Promise<void> {
  // Guard: feature flag, interactive mode, OAuth auth
  if (
    !feature('UPLOAD_USER_SETTINGS') ||
    !getFeatureValue_CACHED_MAY_BE_STALE('tengu_enable_settings_sync_push', false) ||
    !getIsInteractive() ||
    !isUsingOAuth()
  ) {
    logEvent('tengu_settings_sync_upload_skipped_ineligible', {})
    return
  }
  
  // 1. Fetch current remote state
  const result = await fetchUserSettings()
  if (!result.success) {
    logEvent('tengu_settings_sync_upload_fetch_failed', {})
    return
  }
  
  // 2. Build local entries
  const projectId = await getRepoRemoteHash()
  const localEntries = await buildEntriesFromLocalFiles(projectId)
  const remoteEntries = result.isEmpty ? {} : result.data!.content.entries
  
  // 3. Compute delta (changed entries only)
  const changedEntries = pickBy(
    localEntries,
    (value, key) => remoteEntries[key] !== value
  )
  
  const entryCount = Object.keys(changedEntries).length
  if (entryCount === 0) {
    logEvent('tengu_settings_sync_upload_skipped', {})
    return
  }
  
  // 4. Upload delta
  const uploadResult = await uploadUserSettings(changedEntries)
  if (uploadResult.success) {
    logEvent('tengu_settings_sync_upload_success', { entryCount })
  } else {
    logEvent('tengu_settings_sync_upload_failed', { entryCount })
  }
}
```

**Key Patterns**:
- **Delta upload**: Only changed entries are uploaded
- **Fail-open**: Errors are logged but don't block startup
- **Idempotent**: Can run multiple times safely

---

### 4.5 Team Memory Sync with Watcher (`teamMemorySync/index.ts`, `teamMemorySync/watcher.ts`)

```typescript
// Pull: fetch from server, write to local filesystem
export async function pullTeamMemory(
  state: SyncState,
  repoSlug: string,
  signal?: AbortSignal
): Promise<TeamMemorySyncFetchResult> {
  // 1. Fetch with conditional request (ETag)
  const result = await fetchTeamMemoryOnce(state, repoSlug, state.lastKnownChecksum)
  if (result.notModified || !result.success) return result
  
  // 2. Write entries to filesystem
  for (const [key, content] of Object.entries(result.data!.content.entries)) {
    const filePath = getTeamMemPath(key, repoSlug)
    await mkdir(dirname(filePath), { recursive: true })
    await writeFile(filePath, content, 'utf8')
  }
  
  // 3. Update state
  state.lastKnownChecksum = result.data!.checksum
  state.serverChecksums = new Map(
    Object.entries(result.data!.content.entryChecksums)
  )
  
  return result
}

// Push: read local files, upload delta
export async function pushTeamMemory(
  state: SyncState,
  repoSlug: string
): Promise<TeamMemorySyncPushResult> {
  // 1. Read local team memory
  const localEntries = await readLocalTeamMemory(repoSlug, state.serverMaxEntries)
  
  // 2. Compute delta (compare hashes)
  const entriesToUpload: Record<string, string> = {}
  for (const [key, content] of Object.entries(localEntries)) {
    const localHash = hashContent(content)
    const serverHash = state.serverChecksums.get(key)
    if (localHash !== serverHash) {
      entriesToUpload[key] = content
    }
  }
  
  if (Object.keys(entriesToUpload).length === 0) {
    return { success: true, skipped: 'no-changes' }
  }
  
  // 3. Upload (may split into multiple batches if body too large)
  const result = await uploadTeamMemoryEntries(repoSlug, entriesToUpload)
  
  if (result.success) {
    // 4. Update state
    for (const key of Object.keys(entriesToUpload)) {
      state.serverChecksums.set(key, hashContent(localEntries[key]!))
    }
  }
  
  return result
}

// Watcher: auto-sync on file changes
export function createTeamMemoryWatcher(
  state: SyncState,
  repoSlug: string
): { notifyChange: () => void, dispose: () => void } {
  const watcher = watch(teamMemDir, async (event, filename) => {
    if (event === 'change' || event === 'rename') {
      // Debounce + secret scan + push
      await debouncedPush()
    }
  })
  
  return {
    notifyChange: () => debouncedPush(),
    dispose: () => watcher.close()
  }
}
```

**Key Patterns**:
- **Checksum-based delta**: Only changed files are uploaded
- **Batch splitting**: Large pushes split into multiple PUTs (200KB limit)
- **Watcher debounce**: Prevents rapid-fire uploads during editing

---

### 4.6 GrowthBook Feature Access (`analytics/growthbook.ts`)

```typescript
// Cached feature getter (primary API)
const remoteEvalFeatureValues = new Map<string, unknown>()
const loggedExposures = new Set<string>()

export function getFeatureValue_CACHED_MAY_BE_STALE<T>(
  feature: string,
  defaultValue: T
): T {
  // 1. Check env override (ant-only, highest priority)
  const overrides = getEnvOverrides()
  if (overrides !== null && feature in overrides) {
    return overrides[feature] as T
  }
  
  // 2. Check config override (ant-only, runtime)
  const configOverrides = getConfigOverrides()
  if (configOverrides && feature in configOverrides) {
    return configOverrides[feature] as T
  }
  
  // 3. Check remote eval cache
  if (remoteEvalFeatureValues.has(feature)) {
    return remoteEvalFeatureValues.get(feature) as T
  }
  
  // 4. Check disk cache fallback
  const diskCache = getGlobalConfig().cachedGrowthBookFeatures
  if (diskCache && feature in diskCache) {
    return diskCache[feature] as T
  }
  
  // 5. Return default (before init completes)
  return defaultValue
}

// Initialize GrowthBook client
export async function initGrowthBook(): Promise<void> {
  // 1. Check for env overrides (bypass network)
  if (getEnvOverrides()) {
    return
  }
  
  // 2. Load from disk cache (fast path)
  const cached = loadFromDiskCache()
  if (cached && !isExpired(cached)) {
    for (const [key, value] of Object.entries(cached.features)) {
      remoteEvalFeatureValues.set(key, value)
    }
    refreshed.emit()
    return
  }
  
  // 3. Fetch from network
  const features = await fetchFeatures()
  saveToDiskCache(features)
  
  for (const [key, value] of Object.entries(features)) {
    remoteEvalFeatureValues.set(key, value)
  }
  refreshed.emit()
}
```

**Key Patterns**:
- **Priority cascade**: Env → Config → Network cache → Disk cache → Default
- **Stale-ok semantics**: Cached values may be stale; periodic refresh updates them
- **Exposure logging**: First access of each feature logs an exposure event

---

### 4.7 API Retry with Fallback (`api/claude.ts`)

```typescript
export async function* withRetry<T>(
  getClient: () => Promise<Anthropic>,
  operation: (client: Anthropic, attempt: number, context: RetryContext) => Promise<T>,
  options: RetryOptions
): AsyncGenerator<SystemAPIErrorMessage, T> {
  const maxRetries = getMaxRetries(options)
  const retryContext: RetryContext = {
    model: options.model,
    thinkingConfig: options.thinkingConfig,
    ...(isFastModeEnabled() && { fastMode: options.fastMode }),
  }
  
  let client: Anthropic | null = null
  let consecutive529Errors = options.initialConsecutive529Errors ?? 0
  let lastError: unknown
  
  for (let attempt = 1; attempt <= maxRetries + 1; attempt++) {
    if (options.signal?.aborted) {
      throw new APIUserAbortError()
    }
    
    try {
      // 1. Check mock rate limits (ant-only)
      if (process.env.USER_TYPE === 'ant') {
        const mockError = checkMockRateLimitError(retryContext.model, wasFastModeActive)
        if (mockError) throw mockError
      }
      
      // 2. Get fresh client after auth errors
      if (
        client === null ||
        (lastError instanceof APIError && lastError.status === 401) ||
        isOAuthTokenRevokedError(lastError) ||
        isStaleConnectionError(lastError)
      ) {
        if (lastError instanceof APIError && lastError.status === 401) {
          const failedAccessToken = getClaudeAIOAuthTokens()?.accessToken
          if (failedAccessToken) {
            await handleOAuth401Error(failedAccessToken)
          }
        }
        client = await getClient()
      }
      
      // 3. Execute operation
      const result = await operation(client, attempt, retryContext)
      return result
      
    } catch (error) {
      lastError = error
      
      // 4. Handle specific error types
      if (error instanceof APIError) {
        if (error.status === 401) {
          // Auth error: retry with fresh credentials
          continue
        }
        
        if (error.status === 429) {
          // Rate limited: check limits, possibly wait
          const limits = await checkQuotaStatus()
          if (limits.status === 'rejected') {
            throw error  // Quota exhausted
          }
        }
        
        if (error.status === 529) {
          // Overloaded: retry with backoff (max 3)
          consecutive529Errors++
          if (consecutive529Errors >= MAX_529_RETRIES) {
            // Trigger fallback model
            if (options.fallbackModel) {
              throw new FallbackTriggeredError(options.model, options.fallbackModel)
            }
            throw error
          }
        }
      }
      
      // 5. Wait before retry
      const delayMs = getRetryDelay(attempt)
      yield createSystemAPIErrorMessage({ /* progress message */ })
      await sleep(delayMs)
    }
  }
  
  throw new CannotRetryError(lastError, retryContext)
}
```

**Key Patterns**:
- **Async generator**: Yields progress messages during retries
- **529 handling**: Max 3 retries, then fallback model
- **Auth recovery**: 401 triggers token refresh before retry
- **Persistent mode**: Unattended sessions retry indefinitely

---

## 5. Key Patterns

### 5.1 Service Initialization Pattern

Services follow a consistent initialization pattern:

```typescript
// 1. Create singleton/instance
const service = createService()

// 2. Initialize (async)
await service.initialize()

// 3. Attach to AppState/hooks
setAppState(prev => ({ ...prev, service }))

// 4. Cleanup on shutdown
registerCleanup(() => service.dispose())
```

**Examples**:
- `OAuthService`: Created on-demand, no global state
- `GrowthBook`: Singleton, initialized in `init.ts`
- `MCPConnectionManager`: React hook, cleaned up on unmount
- `AnalyticsSink`: Attached once in `setup()`

---

### 5.2 Event-Driven State Updates

Services use event-driven patterns to update state:

```typescript
// Create signal/event emitter
const refreshed = createSignal()

// Subscribe (returns unsubscribe function)
const unsubscribe = refreshed.subscribe(() => {
  // Handle refresh
})

// Emit (notifies all subscribers)
refreshed.emit()

// Cleanup
unsubscribe()
```

**Examples**:
- `GrowthBook`: `onGrowthBookRefresh()` for feature changes
- `claudeAiLimits`: `statusListeners` for quota updates
- `teamMemorySync`: Watcher triggers on file changes

---

### 5.3 Singleton with Lazy Initialization

Services use lazy initialization to avoid startup cost:

```typescript
let service: ServiceType | null = null
let servicePromise: Promise<ServiceType> | null = null

function getService(): Promise<ServiceType> {
  servicePromise ??= (async () => {
    service = await createService()
    return service
  })()
  return servicePromise
}
```

**Examples**:
- `audioNapi` in `voice.ts`: Loaded on first voice keypress
- `authCachePromise` in `mcp/client.ts`: Shared file read for concurrent auth checks
- `downloadPromise` in `settingsSync/index.ts`: Cached download for CCR startup

---

### 5.4 Retry with Exponential Backoff

Consistent retry pattern across services:

```typescript
const DEFAULT_MAX_RETRIES = 3
const BASE_DELAY_MS = 500

async function retryWithBackoff<T>(
  operation: () => Promise<T>,
  maxRetries = DEFAULT_MAX_RETRIES
): Promise<T> {
  let lastError: unknown
  
  for (let attempt = 1; attempt <= maxRetries + 1; attempt++) {
    try {
      return await operation()
    } catch (error) {
      lastError = error
      
      if (attempt > maxRetries) break
      
      const delayMs = BASE_DELAY_MS * Math.pow(2, attempt - 1)
      await sleep(delayMs)
    }
  }
  
  throw lastError
}
```

**Variations**:
- `api/withRetry.ts`: Full-featured with fallback, 529 handling
- `settingsSync/index.ts`: Simple retry for upload/download
- `teamMemorySync/index.ts`: Retry with conflict detection
- `remoteManagedSettings/index.ts`: Retry with checksum caching

---

### 5.5 Feature-Gated Services

Services are gated by GrowthBook features:

```typescript
// Static feature gate (compile-time)
if (feature('MY_FEATURE')) {
  // Code only included if feature is in build
}

// Dynamic feature gate (runtime)
if (getFeatureValue_CACHED_MAY_BE_STALE('tengu_my_feature', false)) {
  // Code runs based on remote config
}

// Combined pattern
const myService = feature('MY_FEATURE') && 
  getFeatureValue_CACHED_MAY_BE_STALE('tengu_enable_my_service', false)
  ? createMyService()
  : null
```

**Examples**:
- `settingsSync`: `UPLOAD_USER_SETTINGS` (static) + `tengu_enable_settings_sync_push` (dynamic)
- `voice`: `EXPERIMENTAL_VOICE` (static)
- `mcp`: `MCP_SKILLS` (static)

---

## 6. Integration Points

### 6.1 With `state/` Module

Services read from and write to `AppState`:

| Service | Reads From | Writes To |
|---------|-----------|-----------|
| `mcp/useManageMCPConnections` | `mcp.config` | `mcp.clients`, `mcp.errors` |
| `claudeAiLimits` | — | Global `currentLimits`, `statusListeners` |
| `settingsSync` | `authVersion` | Triggers `settingsChangeDetector` |
| `teamMemorySync` | `auth` | Syncs files (not AppState) |
| `analytics/growthbook` | `sessionId`, `userType` | Disk cache, `remoteEvalFeatureValues` |

---

### 6.2 With `commands/` Module

Services provide functionality to commands:

| Service | Commands Using It |
|---------|------------------|
| `plugins/pluginOperations` | `/plugin install`, `/plugin uninstall`, `/plugin update` |
| `mcp/config` | `/mcp add`, `/mcp remove`, `/mcp list` |
| `oauth/index` | `/login`, `/logout`, `/login status` |
| `settingsSync` | `/reload-plugins` (triggers redownload) |
| `compact/compact` | `/compact` (manual compaction) |

---

### 6.3 With `tools/` Module

Services support tool execution:

| Service | Tool Integration |
|---------|-----------------|
| `mcp/client` | `MCPTool` — discovers and executes MCP tools |
| `tools/toolExecution` | Core execution engine for all tools |
| `lsp/LSPServerManager` | LSP-based tools (diagnostics, definitions) |
| `policyLimits` | Enforces tool execution policies |

---

### 6.4 With `utils/` Module

Services depend on utilities:

| Service | Utility Dependencies |
|---------|---------------------|
| `api/*` | `auth.ts`, `model/`, `errors.ts`, `log.ts` |
| `mcp/*` | `config.ts`, `auth.ts`, `platform.ts`, `secureStorage/` |
| `analytics/*` | `config.ts`, `user.ts`, `http.ts` |
| `oauth/*` | `browser.ts`, `secureStorage/`, `platform.ts` |
| `settingsSync/*` | `config.ts`, `git.ts`, `auth.ts` |

---

## 7. Error Handling

### 7.1 API Error Classification

```typescript
// api/errors.ts
export function classifyAPIError(error: unknown): {
  type: 'auth' | 'rate_limit' | 'quota' | 'media_size' | 'prompt_too_long' | 'other'
  message: string
  isRetryable: boolean
} {
  if (error instanceof APIError) {
    switch (error.status) {
      case 401: return { type: 'auth', message: 'Invalid API key', isRetryable: true }
      case 403: return { type: 'auth', message: 'Forbidden', isRetryable: false }
      case 429: return { type: 'rate_limit', message: 'Rate limited', isRetryable: true }
      case 529: return { type: 'rate_limit', message: 'Overloaded', isRetryable: true }
    }
  }
  
  const msg = errorMessage(error)
  if (isPromptTooLongMessage(msg)) return { type: 'prompt_too_long', ... }
  if (isMediaSizeError(msg)) return { type: 'media_size', ... }
  
  return { type: 'other', message: msg, isRetryable: false }
}
```

### 7.2 MCP Error Handling

```typescript
// mcp/client.ts
export class McpAuthError extends Error {
  serverName: string
  constructor(serverName: string, message: string) {
    super(message)
    this.name = 'McpAuthError'
    this.serverName = serverName
  }
}

// Caller catches and updates server status
try {
  await callMcpTool(serverName, toolName, input)
} catch (error) {
  if (error instanceof McpAuthError) {
    setAppState(prev => ({
      ...prev,
      mcp: {
        ...prev.mcp,
        clients: prev.mcp.clients.map(c =>
          c.name === serverName ? { ...c, status: 'needs-auth' } : c
        )
      }
    }))
  }
}
```

### 7.3 Settings Sync Error Handling

```typescript
// settingsSync/index.ts
export async function uploadUserSettingsInBackground(): Promise<void> {
  try {
    // ... upload logic
  } catch {
    // Fail-open: log error but don't block startup
    logForDiagnosticsNoPII('error', 'settings_sync_unexpected_error')
  }
}
```

**Pattern**: Settings sync is **fail-open** — errors are logged but don't block startup or user workflow.

---

## 8. Testing Considerations

### 8.1 Mocking Analytics

```typescript
// Test setup
import { _resetForTesting } from 'src/services/analytics/index'

beforeEach(() => {
  _resetForTesting()  // Clear sink and queue
})

// Mock sink
const mockSink: AnalyticsSink = {
  logEvent: jest.fn(),
  logEventAsync: jest.fn(),
}
attachAnalyticsSink(mockSink)
```

### 8.2 Mocking MCP Servers

```typescript
// Test setup
import { InProcessTransport } from 'src/services/mcp/InProcessTransport'

// Create in-process server for testing
const transport = new InProcessTransport({
  tools: [{ name: 'test_tool', handler: jest.fn() }],
})

// Connect client to in-process server
const client = await connectToServer('test-server', {
  type: 'stdio',
  command: 'node',
  args: ['server.js'],
}, transport)
```

### 8.3 Mocking OAuth Flow

```typescript
// Test setup
import { OAuthService } from 'src/services/oauth'

// Mock auth code listener
const mockAuthCodeListener = {
  start: jest.fn().mockResolvedValue(3000),
  waitForAuthorization: jest.fn().mockResolvedValue('test-code'),
  close: jest.fn(),
}

// Mock client operations
jest.mock('src/services/oauth/client', () => ({
  exchangeCodeForTokens: jest.fn().mockResolvedValue({
    access_token: 'test-token',
    refresh_token: 'test-refresh',
    expires_in: 3600,
  }),
  fetchProfileInfo: jest.fn().mockResolvedValue({
    subscriptionType: 'pro',
    rateLimitTier: 'standard',
  }),
}))
```

---

## 9. Environment Variables

### 9.1 API Configuration

| Variable | Purpose | Default |
|----------|---------|---------|
| `ANTHROPIC_API_KEY` | Direct API authentication | — |
| `ANTHROPIC_BASE_URL` | Custom API endpoint | `https://api.anthropic.com` |
| `CLAUDE_CODE_USE_BEDROCK` | Use AWS Bedrock | `false` |
| `CLAUDE_CODE_USE_FOUNDRY` | Use Azure Foundry | `false` |
| `CLAUDE_CODE_USE_VERTEX` | Use GCP Vertex | `false` |
| `API_TIMEOUT_MS` | API request timeout | `600000` (10 min) |

### 9.2 Feature Flags

| Variable | Purpose | Default |
|----------|---------|---------|
| `CLAUDE_INTERNAL_FC_OVERRIDES` | GrowthBook overrides (JSON) | — |
| `USER_TYPE` | User type (`ant` for employees) | — |
| `CLAUDE_CODE_UNATTENDED_RETRY` | Persistent retry mode | `false` |

### 9.3 MCP Configuration

| Variable | Purpose | Default |
|----------|---------|---------|
| `MCP_TOOL_TIMEOUT` | MCP tool call timeout | `100000000` (~27.8 hours) |
| `TEAM_MEMORY_SYNC_URL` | Custom team memory endpoint | — |

---

## 10. Telemetry Events

### 10.1 OAuth Events

| Event | Location | Fields |
|-------|----------|--------|
| `tengu_oauth_auth_code_received` | `oauth/index.ts` | `automatic` (boolean) |
| `tengu_oauth_flow_start` | `oauth/client.ts` | `loginWithClaudeAi` |
| `tengu_mcp_oauth_flow_error` | `mcp/auth.ts` | `reason` |
| `tengu_mcp_oauth_refresh_failure` | `mcp/auth.ts` | `reason`, `serverName` |

### 10.2 Analytics Events

| Event | Location | Fields |
|-------|----------|--------|
| `tengu_event_logged` | `analytics/sink.ts` | `event_name`, `sample_rate` |
| `tengu_growthbook_initialized` | `analytics/growthbook.ts` | `from_cache`, `feature_count` |
| `tengu_settings_sync_upload_success` | `settingsSync/index.ts` | `entryCount` |
| `tengu_settings_sync_download_success` | `settingsSync/index.ts` | `entryCount` |
| `tengu_team_memory_sync_push` | `teamMemorySync/index.ts` | `entryCount`, `skippedSecrets` |

### 10.3 API Events

| Event | Location | Fields |
|-------|----------|--------|
| `tengu_api_query` | `api/logging.ts` | `model`, `source`, `fast_mode` |
| `tengu_api_error` | `api/logging.ts` | `status`, `type`, `model` |
| `tengu_claudeai_limits_status_changed` | `claudeAiLimits.ts` | `status`, `hoursTillReset` |
| `tengu_rate_limit_encountered` | `rateLimitMessages.ts` | `limit_type`, `action_taken` |

---

## 11. Summary

The `services/` module is the **backend integration layer** for Claude Code, providing:

1. **API Infrastructure** — Reliable communication with Anthropic's API and backend services, including retry logic, rate limiting, and error handling.

2. **MCP Integration** — Full Model Context Protocol support for connecting to external tools, resources, and prompts, with OAuth authentication and channel permissions.

3. **Authentication** — OAuth 2.0 PKCE flow for Claude.ai, with secure token storage and refresh.

4. **Analytics** — Feature flagging via GrowthBook, event logging to Datadog and 1P backends, with PII protection and sampling.

5. **Settings Synchronization** — Cross-environment settings sync, team memory sharing, and enterprise managed settings.

6. **Session Management** — Conversation compaction, context management, and memory extraction.

7. **Plugin System** — Plugin installation, updates, and lifecycle management.

8. **Specialized Services** — LSP integration, voice input, tips/onboarding, and more.

The module follows consistent patterns:
- **Singleton services** with lazy initialization
- **Event-driven state updates** via signals/listeners
- **Retry with exponential backoff** for transient failures
- **Feature-gated functionality** for controlled rollouts
- **Fail-open semantics** for non-critical services

---

**Last Updated:** 2026-04-07  
**Status:** Complete — all 130 files inventoried, critical services analyzed line-by-line
