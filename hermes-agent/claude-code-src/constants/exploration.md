# Claude Code Constants Module — Deep-Dive Exploration

**Module:** `src/constants/`  
**Parent Project:** Claude Code CLI  
**Created:** 2026-04-07  
**Files:** 21 TypeScript files

---

## 1. Module Overview

The `constants/` module is the **centralized definition layer** for Claude Code CLI, containing app-wide constants, configuration defaults, magic numbers, beta headers, prompts, and tool definitions. Despite the name "constants," this module includes dynamic computation functions, feature-flagged values, and environment-aware configuration.

### Core Responsibilities

1. **API Limits** — Anthropic API enforcement:
   - Image size limits (base64 and raw)
   - PDF page and size limits
   - Media items per request

2. **Beta Headers** — API feature flags:
   - Interleaved thinking, context 1M, tool search
   - Platform-specific headers (Bedrock, Vertex)
   - Sticky-on latches for cache stability

3. **System Prompts** — Claude behavior configuration:
   - Dynamic prompt sections with caching
   - Session-specific guidance
   - Output style configurations

4. **Tool Definitions** — Tool names and restrictions:
   - Allowed/disallowed tools for agents
   - Tool result size limits
   - Tool summary constraints

5. **OAuth Configuration** — Authentication endpoints:
   - Prod/staging/local configurations
   - Scope definitions
   - Client ID metadata

6. **UI Constants** — Visual elements:
   - Figures and symbols
   - Spinner verbs
   - Output styles

7. **File Handling** — Binary detection:
   - Binary extension lists
   - Content-based binary detection
   - PDF handling thresholds

8. **XML Tags** — Message structure:
   - Terminal output tags
   - Task notification tags
   - Cross-session message tags

### Key Design Patterns

- **Dead Code Elimination (DCE)**: Feature-flagged exports for tree-shaking
- **Platform Abstraction**: Environment-aware configuration (prod/staging/local)
- **Memoization**: Cached computations for performance
- **Type-Safe Constants**: TypeScript enums and discriminated unions
- **Lazy Evaluation**: Environment variables checked at access time

---

## 2. File Inventory

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `apiLimits.ts` | ~95 | `API_IMAGE_MAX_BASE64_SIZE`, `PDF_MAX_PAGES` | Anthropic API limits (images, PDFs, media) |
| `betas.ts` | ~53 | `INTERLEAVED_THINKING_BETA_HEADER`, `BEDROCK_EXTRA_PARAMS_HEADERS` | API beta header definitions |
| `common.ts` | ~34 | `getLocalISODate()`, `getSessionStartDate` | Date utilities with memoization |
| `cyberRiskInstruction.ts` | ~25 | `CYBER_RISK_INSTRUCTION` | Security behavior guidelines |
| `errorIds.ts` | ~16 | `E_TOOL_USE_SUMMARY_GENERATION_FAILED` | Obfuscated error tracking IDs |
| `figures.ts` | ~46 | `BLACK_CIRCLE`, `EFFORT_*`, `DIAMOND_*` | Unicode symbols and indicators |
| `files.ts` | ~157 | `BINARY_EXTENSIONS`, `isBinaryContent()` | Binary file detection |
| `github-app.ts` | ~145 | `WORKFLOW_CONTENT`, `PR_BODY` | GitHub Actions workflow templates |
| `keys.ts` | ~12 | `getGrowthBookClientKey()` | GrowthBook API key resolver |
| `messages.ts` | ~2 | `NO_CONTENT_MESSAGE` | Empty state messages |
| `oauth.ts` | ~235 | `getOauthConfig()`, `ALL_OAUTH_SCOPES` | OAuth endpoint configuration |
| `outputStyles.ts` | ~217 | `OUTPUT_STYLE_CONFIG`, `getOutputStyleConfig()` | Output style definitions (Explanatory, Learning) |
| `product.ts` | ~77 | `getRemoteSessionUrl()`, `isRemoteSessionStaging()` | Remote session URL management |
| `prompts.ts` | ~914 | `getSystemPrompt()`, `computeEnvInfo()` | System prompt generation |
| `spinnerVerbs.ts` | ~205 | `SPINNER_VERBS`, `getSpinnerVerbs()` | Loading message verbs |
| `systemPromptSections.ts` | ~69 | `systemPromptSection()`, `resolveSystemPromptSections()` | Prompt section registry |
| `system.ts` | ~96 | `getAttributionHeader()`, `CLI_SYSPROMPT_PREFIXES` | System prompt framing |
| `toolLimits.ts` | ~57 | `MAX_TOOL_RESULT_TOKENS`, `DEFAULT_MAX_RESULT_SIZE_CHARS` | Tool result size constraints |
| `tools.ts` | ~113 | `ALL_AGENT_DISALLOWED_TOOLS`, `ASYNC_AGENT_ALLOWED_TOOLS` | Tool allowlists/blocklists |
| `turnCompletionVerbs.ts` | ~13 | `TURN_COMPLETION_VERBS` | Turn completion past-tense verbs |
| `xml.ts` | ~87 | `TERMINAL_OUTPUT_TAGS`, `TEAMMATE_MESSAGE_TAG` | XML tag definitions |

**Total Lines:** ~2,651 lines

---

## 3. Key Exports by Category

### 3.1 API Limits (`apiLimits.ts`)

```typescript
// Image limits
export const API_IMAGE_MAX_BASE64_SIZE = 5 * 1024 * 1024  // 5 MB
export const IMAGE_TARGET_RAW_SIZE = (API_IMAGE_MAX_BASE64_SIZE * 3) / 4  // 3.75 MB
export const IMAGE_MAX_WIDTH = 2000
export const IMAGE_MAX_HEIGHT = 2000

// PDF limits
export const PDF_TARGET_RAW_SIZE = 20 * 1024 * 1024  // 20 MB
export const API_PDF_MAX_PAGES = 100
export const PDF_EXTRACT_SIZE_THRESHOLD = 3 * 1024 * 1024  // 3 MB
export const PDF_MAX_EXTRACT_SIZE = 100 * 1024 * 1024  // 100 MB
export const PDF_MAX_PAGES_PER_READ = 20
export const PDF_AT_MENTION_INLINE_THRESHOLD = 10

// Media limits
export const API_MAX_MEDIA_PER_REQUEST = 100
```

**Last Verified:** 2025-12-22 (per file comment)

---

### 3.2 Beta Headers (`betas.ts`)

```typescript
export const CLAUDE_CODE_20250219_BETA_HEADER = 'claude-code-20250219'
export const INTERLEAVED_THINKING_BETA_HEADER = 'interleaved-thinking-2025-05-14'
export const CONTEXT_1M_BETA_HEADER = 'context-1m-2025-08-07'
export const CONTEXT_MANAGEMENT_BETA_HEADER = 'context-management-2025-06-27'
export const STRUCTURED_OUTPUTS_BETA_HEADER = 'structured-outputs-2025-12-15'
export const WEB_SEARCH_BETA_HEADER = 'web-search-2025-03-05'
export const TOOL_SEARCH_BETA_HEADER_1P = 'advanced-tool-use-2025-11-20'  // Claude API
export const TOOL_SEARCH_BETA_HEADER_3P = 'tool-search-tool-2025-10-19'  // Vertex/Bedrock
export const EFFORT_BETA_HEADER = 'effort-2025-11-24'
export const TASK_BUDGETS_BETA_HEADER = 'task-budgets-2026-03-13'
export const PROMPT_CACHING_SCOPE_BETA_HEADER = 'prompt-caching-scope-2026-01-05'
export const FAST_MODE_BETA_HEADER = 'fast-mode-2026-02-01'
export const REDACT_THINKING_BETA_HEADER = 'redact-thinking-2026-02-12'
export const TOKEN_EFFICIENT_TOOLS_BETA_HEADER = 'token-efficient-tools-2026-03-28'

// Feature-flagged betas
export const SUMMARIZE_CONNECTOR_TEXT_BETA_HEADER = feature('CONNECTOR_TEXT')
  ? 'summarize-connector-text-2026-03-13'
  : ''
export const AFK_MODE_BETA_HEADER = feature('TRANSCRIPT_CLASSIFIER')
  ? 'afk-mode-2026-01-31'
  : ''
export const CLI_INTERNAL_BETA_HEADER = process.env.USER_TYPE === 'ant'
  ? 'cli-internal-2026-02-09'
  : ''
export const ADVISOR_BETA_HEADER = 'advisor-tool-2026-03-01'

// Platform-specific sets
export const BEDROCK_EXTRA_PARAMS_HEADERS = new Set([
  INTERLEAVED_THINKING_BETA_HEADER,
  CONTEXT_1M_BETA_HEADER,
  TOOL_SEARCH_BETA_HEADER_3P,
])

export const VERTEX_COUNT_TOKENS_ALLOWED_BETAS = new Set([
  CLAUDE_CODE_20250219_BETA_HEADER,
  INTERLEAVED_THINKING_BETA_HEADER,
  CONTEXT_MANAGEMENT_BETA_HEADER,
])
```

**Key Insight:** Bedrock supports limited beta headers only through `extraBodyParams`, not HTTP headers.

---

### 3.3 OAuth Configuration (`oauth.ts`)

```typescript
// Scopes
export const CLAUDE_AI_INFERENCE_SCOPE = 'user:inference'
export const CLAUDE_AI_PROFILE_SCOPE = 'user:profile'
export const CONSOLE_OAUTH_SCOPES = [CONSOLE_SCOPE, CLAUDE_AI_PROFILE_SCOPE]
export const CLAUDE_AI_OAUTH_SCOPES = [
  CLAUDE_AI_PROFILE_SCOPE,
  CLAUDE_AI_INFERENCE_SCOPE,
  'user:sessions:claude_code',
  'user:mcp_servers',
  'user:file_upload',
]
export const ALL_OAUTH_SCOPES = Array.from(new Set([...CONSOLE_OAUTH_SCOPES, ...CLAUDE_AI_OAUTH_SCOPES]))

// Beta header
export const OAUTH_BETA_HEADER = 'oauth-2025-04-20'

// Client metadata for MCP OAuth (SEP-991)
export const MCP_CLIENT_METADATA_URL = 'https://claude.ai/oauth/claude-code-client-metadata'

// Configuration type
type OauthConfig = {
  BASE_API_URL: string
  CONSOLE_AUTHORIZE_URL: string
  CLAUDE_AI_AUTHORIZE_URL: string
  CLAUDE_AI_ORIGIN: string
  TOKEN_URL: string
  API_KEY_URL: string
  ROLES_URL: string
  CONSOLE_SUCCESS_URL: string
  CLAUDEAI_SUCCESS_URL: string
  MANUAL_REDIRECT_URL: string
  CLIENT_ID: string
  OAUTH_FILE_SUFFIX: string
  MCP_PROXY_URL: string
  MCP_PROXY_PATH: string
}

// Production config
const PROD_OAUTH_CONFIG = {
  BASE_API_URL: 'https://api.anthropic.com',
  CONSOLE_AUTHORIZE_URL: 'https://platform.claude.com/oauth/authorize',
  CLAUDE_AI_AUTHORIZE_URL: 'https://claude.com/cai/oauth/authorize',  // Bounces through claude.com
  CLAUDE_AI_ORIGIN: 'https://claude.ai',
  TOKEN_URL: 'https://platform.claude.com/v1/oauth/token',
  API_KEY_URL: 'https://api.anthropic.com/api/oauth/claude_cli/create_api_key',
  ROLES_URL: 'https://api.anthropic.com/api/oauth/claude_cli/roles',
  CONSOLE_SUCCESS_URL: 'https://platform.claude.com/buy_credits?returnUrl=/oauth/code/success%3Fapp%3Dclaude-code',
  CLAUDEAI_SUCCESS_URL: 'https://platform.claude.com/oauth/code/success?app=claude-code',
  MANUAL_REDIRECT_URL: 'https://platform.claude.com/oauth/code/callback',
  CLIENT_ID: '9d1c250a-e61b-44d9-88ed-5944d1962f5e',
  OAUTH_FILE_SUFFIX: '',
  MCP_PROXY_URL: 'https://mcp-proxy.anthropic.com',
  MCP_PROXY_PATH: '/v1/mcp/{server_id}',
}
```

**Staging/Local Configs:** Also defined with different base URLs and client IDs.

---

### 3.4 Output Styles (`outputStyles.ts`)

```typescript
export type OutputStyleConfig = {
  name: string
  description: string
  prompt: string
  source: SettingSource | 'built-in' | 'plugin'
  keepCodingInstructions?: boolean
  forceForPlugin?: boolean  // Auto-apply when plugin enabled
}

export const DEFAULT_OUTPUT_STYLE_NAME = 'default'

export const OUTPUT_STYLE_CONFIG: OutputStyles = {
  [DEFAULT_OUTPUT_STYLE_NAME]: null,
  Explanatory: {
    name: 'Explanatory',
    source: 'built-in',
    description: 'Claude explains implementation choices and codebase patterns',
    keepCodingInstructions: true,
    prompt: `You are an interactive CLI tool... In addition to software engineering tasks, you should provide educational explanations...`,
  },
  Learning: {
    name: 'Learning',
    source: 'built-in',
    description: 'Claude pauses and asks you to write small pieces of code for hands-on practice',
    keepCodingInstructions: true,
    prompt: `You are an interactive CLI tool... Balance task completion with learning by requesting user input...`,
  },
}
```

**Learning Mode Key Feature:** Requests user contribute 2-10 lines for 20+ line generations involving design decisions.

---

### 3.5 Tool Allowlists (`tools.ts`)

```typescript
// Tools disallowed for ALL agents (nested agents)
export const ALL_AGENT_DISALLOWED_TOOLS = new Set([
  TASK_OUTPUT_TOOL_NAME,
  EXIT_PLAN_MODE_V2_TOOL_NAME,
  ENTER_PLAN_MODE_TOOL_NAME,
  ...(process.env.USER_TYPE === 'ant' ? [] : [AGENT_TOOL_NAME]),  // Ant allows nested agents
  ASK_USER_QUESTION_TOOL_NAME,
  TASK_STOP_TOOL_NAME,
  ...(feature('WORKFLOW_SCRIPTS') ? [WORKFLOW_TOOL_NAME] : []),  // Prevent recursive workflow
])

// Async agent allowed tools (fork subagents)
export const ASYNC_AGENT_ALLOWED_TOOLS = new Set([
  FILE_READ_TOOL_NAME,
  WEB_SEARCH_TOOL_NAME,
  TODO_WRITE_TOOL_NAME,
  GREP_TOOL_NAME,
  WEB_FETCH_TOOL_NAME,
  GLOB_TOOL_NAME,
  ...SHELL_TOOL_NAMES,
  FILE_EDIT_TOOL_NAME,
  FILE_WRITE_TOOL_NAME,
  NOTEBOOK_EDIT_TOOL_NAME,
  SKILL_TOOL_NAME,
  SYNTHETIC_OUTPUT_TOOL_NAME,
  TOOL_SEARCH_TOOL_NAME,
  ENTER_WORKTREE_TOOL_NAME,
  EXIT_WORKTREE_TOOL_NAME,
])

// In-process teammate extras (injected by inProcessRunner.ts)
export const IN_PROCESS_TEAMMATE_ALLOWED_TOOLS = new Set([
  TASK_CREATE_TOOL_NAME,
  TASK_GET_TOOL_NAME,
  TASK_LIST_TOOL_NAME,
  TASK_UPDATE_TOOL_NAME,
  SEND_MESSAGE_TOOL_NAME,
  ...(feature('AGENT_TRIGGERS')
    ? [CRON_CREATE_TOOL_NAME, CRON_DELETE_TOOL_NAME, CRON_LIST_TOOL_NAME]
    : []),
])

// Coordinator mode (supervisory only)
export const COORDINATOR_MODE_ALLOWED_TOOLS = new Set([
  AGENT_TOOL_NAME,
  TASK_STOP_TOOL_NAME,
  SEND_MESSAGE_TOOL_NAME,
  SYNTHETIC_OUTPUT_TOOL_NAME,
])
```

---

### 3.6 Tool Result Limits (`toolLimits.ts`)

```typescript
// Default max size before persistence to disk
export const DEFAULT_MAX_RESULT_SIZE_CHARS = 50_000

// Token-based limit (~400KB at 4 bytes/token)
export const MAX_TOOL_RESULT_TOKENS = 100_000
export const BYTES_PER_TOKEN = 4
export const MAX_TOOL_RESULT_BYTES = MAX_TOOL_RESULT_TOKENS * BYTES_PER_TOKEN

// Per-message aggregate limit (single turn's parallel tool results)
export const MAX_TOOL_RESULTS_PER_MESSAGE_CHARS = 200_000

// Summary truncation for compact views
export const TOOL_SUMMARY_MAX_LENGTH = 50
```

**Design Rationale:** `MAX_TOOL_RESULTS_PER_MESSAGE_CHARS` prevents N parallel tools from collectively producing 400K+ in one user message.

---

### 3.7 Binary File Detection (`files.ts`)

```typescript
export const BINARY_EXTENSIONS = new Set([
  // Images
  '.png', '.jpg', '.jpeg', '.gif', '.bmp', '.ico', '.webp', '.tiff', '.tif',
  // Videos
  '.mp4', '.mov', '.avi', '.mkv', '.webm', '.wmv', '.flv', '.m4v', '.mpeg', '.mpg',
  // Audio
  '.mp3', '.wav', '.ogg', '.flac', '.aac', '.m4a', '.wma', '.aiff', '.opus',
  // Archives
  '.zip', '.tar', '.gz', '.bz2', '.7z', '.rar', '.xz', '.z', '.tgz', '.iso',
  // Executables
  '.exe', '.dll', '.so', '.dylib', '.bin', '.o', '.a', '.obj', '.lib', '.app', '.msi',
  // Documents (PDF is here; FileReadTool excludes at call site)
  '.pdf', '.doc', '.docx', '.xls', '.xlsx', '.ppt', '.pptx',
  // Fonts
  '.ttf', '.otf', '.woff', '.woff2', '.eot',
  // Bytecode
  '.pyc', '.pyo', '.class', '.jar', '.war', '.ear', '.node', '.wasm', '.rlib',
  // Database
  '.sqlite', '.sqlite3', '.db', '.mdb', '.idx',
  // Design/3D
  '.psd', '.ai', '.eps', '.sketch', '.fig', '.xd', '.blend', '.3ds', '.max',
  // Lock/profiling
  '.lockb', '.dat', '.data',
])

export function hasBinaryExtension(filePath: string): boolean {
  const ext = filePath.slice(filePath.lastIndexOf('.')).toLowerCase()
  return BINARY_EXTENSIONS.has(ext)
}

export function isBinaryContent(buffer: Buffer): boolean {
  const checkSize = Math.min(buffer.length, BINARY_CHECK_SIZE)  // 8192 bytes
  let nonPrintable = 0
  
  for (let i = 0; i < checkSize; i++) {
    const byte = buffer[i]!
    if (byte === 0) return true  // Null byte = strong binary indicator
    
    // Count non-printable, non-whitespace
    if (byte < 32 && byte !== 9 && byte !== 10 && byte !== 13) {
      nonPrintable++
    }
  }
  
  // >10% non-printable = likely binary
  return nonPrintable / checkSize > 0.1
}
```

---

### 3.8 XML Tag Definitions (`xml.ts`)

```typescript
// Skill/command metadata
export const COMMAND_NAME_TAG = 'command-name'
export const COMMAND_MESSAGE_TAG = 'command-message'
export const COMMAND_ARGS_TAG = 'command-args'

// Terminal/bash I/O
export const BASH_INPUT_TAG = 'bash-input'
export const BASH_STDOUT_TAG = 'bash-stdout'
export const BASH_STDERR_TAG = 'bash-stderr'
export const LOCAL_COMMAND_STDOUT_TAG = 'local-command-stdout'
export const LOCAL_COMMAND_STDERR_TAG = 'local-command-stderr'
export const LOCAL_COMMAND_CAVEAT_TAG = 'local-command-caveat'

export const TERMINAL_OUTPUT_TAGS = [
  BASH_INPUT_TAG, BASH_STDOUT_TAG, BASH_STDERR_TAG,
  LOCAL_COMMAND_STDOUT_TAG, LOCAL_COMMAND_STDERR_TAG, LOCAL_COMMAND_CAVEAT_TAG,
]

// Task notifications
export const TASK_NOTIFICATION_TAG = 'task-notification'
export const TASK_ID_TAG = 'task-id'
export const TOOL_USE_ID_TAG = 'tool-use-id'
export const TASK_TYPE_TAG = 'task-type'
export const OUTPUT_FILE_TAG = 'output-file'
export const STATUS_TAG = 'status'
export const SUMMARY_TAG = 'summary'
export const REASON_TAG = 'reason'
export const WORKTREE_TAG = 'worktree'
export const WORKTREE_PATH_TAG = 'worktreePath'
export const WORKTREE_BRANCH_TAG = 'worktreeBranch'

// Inter-agent communication
export const TEAMMATE_MESSAGE_TAG = 'teammate-message'
export const CHANNEL_MESSAGE_TAG = 'channel-message'
export const CHANNEL_TAG = 'channel'
export const CROSS_SESSION_MESSAGE_TAG = 'cross-session-message'

// Fork directive
export const FORK_BOILERPLATE_TAG = 'fork-boilerplate'
export const FORK_DIRECTIVE_PREFIX = 'Your directive: '

// Ultraplan/remote review
export const ULTRAPLAN_TAG = 'ultraplan'
export const REMOTE_REVIEW_TAG = 'remote-review'
export const REMOTE_REVIEW_PROGRESS_TAG = 'remote-review-progress'
```

---

### 3.9 System Prompt Constants (`system.ts`)

```typescript
const DEFAULT_PREFIX = `You are Claude Code, Anthropic's official CLI for Claude.`
const AGENT_SDK_CLAUDE_CODE_PRESET_PREFIX = `You are Claude Code, Anthropic's official CLI for Claude, running within the Claude Agent SDK.`
const AGENT_SDK_PREFIX = `You are a Claude agent, built on Anthropic's Claude Agent SDK.`

export type CLISyspromptPrefix = (typeof CLI_SYSPROMPT_PREFIX_VALUES)[number]

export const CLI_SYSPROMPT_PREFIXES: ReadonlySet<string> = new Set(CLI_SYSPROMPT_PREFIX_VALUES)

export function getCLISyspromptPrefix(options?: {
  isNonInteractive: boolean
  hasAppendSystemPrompt: boolean
}): CLISyspromptPrefix {
  const apiProvider = getAPIProvider()
  if (apiProvider === 'vertex') return DEFAULT_PREFIX
  
  if (options?.isNonInteractive) {
    if (options.hasAppendSystemPrompt) return AGENT_SDK_CLAUDE_CODE_PRESET_PREFIX
    return AGENT_SDK_PREFIX
  }
  return DEFAULT_PREFIX
}

// Attribution header for API requests
export function getAttributionHeader(fingerprint: string): string {
  if (!isAttributionHeaderEnabled()) return ''
  
  const version = `${MACRO.VERSION}.${fingerprint}`
  const entrypoint = process.env.CLAUDE_CODE_ENTRYPOINT ?? 'unknown'
  const cch = feature('NATIVE_CLIENT_ATTESTATION') ? ' cch=00000;' : ''
  const workload = getWorkload()
  const workloadPair = workload ? ` cc_workload=${workload};` : ''
  
  return `x-anthropic-billing-header: cc_version=${version}; cc_entrypoint=${entrypoint};${cch}${workloadPair}`
}
```

**Attestation Placeholder:** `cch=00000` is overwritten by Bun's HTTP stack with computed hash.

---

### 3.10 GitHub Actions Templates (`github-app.ts`)

```typescript
export const PR_TITLE = 'Add Claude Code GitHub Workflow'

export const WORKFLOW_CONTENT = `name: Claude Code

on:
  issue_comment:
    types: [created]
  pull_request_review_comment:
    types: [created]
  issues:
    types: [opened, assigned]
  pull_request_review:
    types: [submitted]

jobs:
  claude:
    if: |
      (github.event_name == 'issue_comment' && contains(github.event.comment.body, '@claude')) ||
      ...
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: read
      issues: read
      id-token: write
      actions: read
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4
      - name: Run Claude Code
        id: claude
        uses: anthropics/claude-code-action@v1
        with:
          anthropic_api_key: \${{ secrets.ANTHROPIC_API_KEY }}
`

export const PR_BODY = `## Installing Claude Code GitHub App

This PR adds a GitHub Actions workflow that enables Claude Code integration...

### Security
- Anthropic API key stored as GitHub Actions secret
- Only users with write access can trigger workflow
- All runs stored in GitHub Actions history
`
```

---

## 4. Line-by-Line Analysis

### 4.1 API Limits Rationale (`apiLimits.ts`)

```typescript
/**
 * Maximum base64-encoded image size (API enforced).
 * Note: This is the base64 length, NOT raw bytes. Base64 increases size by ~33%.
 */
export const API_IMAGE_MAX_BASE64_SIZE = 5 * 1024 * 1024  // 5 MB

/**
 * Target raw image size to stay under base64 limit after encoding.
 * Base64: raw_size * 4/3 = base64_size → raw_size = base64_size * 3/4
 */
export const IMAGE_TARGET_RAW_SIZE = (API_IMAGE_MAX_BASE64_SIZE * 3) / 4  // 3.75 MB
```

**Key Insight:** Client-side target (3.75 MB raw) is derived from API limit (5 MB base64) using base64 expansion formula.

```typescript
/**
 * Client-side maximum dimensions for image resizing.
 * 
 * Note: The API internally resizes images larger than 1568px, but this is
 * handled server-side. These client-side limits (2000px) preserve quality.
 * 
 * The API_IMAGE_MAX_BASE64_SIZE (5MB) is the actual hard limit that causes
 * API errors if exceeded.
 */
export const IMAGE_MAX_WIDTH = 2000
export const IMAGE_MAX_HEIGHT = 2000
```

**Design Choice:** Client limit (2000px) > server internal resize (1568px) to preserve quality when beneficial.

---

### 4.2 Beta Header Feature Flagging (`betas.ts`)

```typescript
// Feature-flagged betas — DCE eliminates unused
export const SUMMARIZE_CONNECTOR_TEXT_BETA_HEADER = feature('CONNECTOR_TEXT')
  ? 'summarize-connector-text-2026-03-13'
  : ''
export const AFK_MODE_BETA_HEADER = feature('TRANSCRIPT_CLASSIFIER')
  ? 'afk-mode-2026-01-31'
  : ''
export const CLI_INTERNAL_BETA_HEADER = process.env.USER_TYPE === 'ant'
  ? 'cli-internal-2026-02-09'
  : ''
```

**DCE Pattern:** Empty string (`''`) for disabled features — bundler eliminates conditional usage.

---

### 4.3 OAuth Environment Detection (`oauth.ts`)

```typescript
function getOauthConfigType(): OauthConfigType {
  if (process.env.USER_TYPE === 'ant') {
    if (isEnvTruthy(process.env.USE_LOCAL_OAUTH)) {
      return 'local'
    }
    if (isEnvTruthy(process.env.USE_STAGING_OAUTH)) {
      return 'staging'
    }
  }
  return 'prod'  // Default for external builds
}

// Dead code elimination — external builds never see staging config
const STAGING_OAUTH_CONFIG = process.env.USER_TYPE === 'ant'
  ? ({
      BASE_API_URL: 'https://api-staging.anthropic.com',
      CLIENT_ID: '22422756-60c9-4084-8eb7-27705fd5cf9a',
      // ...
    } as const)
  : undefined
```

**Security:** External builds DCE staging config entirely — no risk of staging endpoint leakage.

---

### 4.4 OAuth URL Validation (`oauth.ts` lines 176-207)

```typescript
// Allowed base URLs for CLAUDE_CODE_CUSTOM_OAUTH_URL override.
// Only FedStart/PubSec deployments permitted to prevent credential leakage.
const ALLOWED_OAUTH_BASE_URLS = [
  'https://beacon.claude-ai.staging.ant.dev',
  'https://claude.fedstart.com',
  'https://claude-staging.fedstart.com',
]

export function getOauthConfig(): OauthConfig {
  let config: OauthConfig = ...  // prod/staging/local
  
  const oauthBaseUrl = process.env.CLAUDE_CODE_CUSTOM_OAUTH_URL
  if (oauthBaseUrl) {
    const base = oauthBaseUrl.replace(/\/$/, '')
    if (!ALLOWED_OAUTH_BASE_URLS.includes(base)) {
      throw new Error('CLAUDE_CODE_CUSTOM_OAUTH_URL is not an approved endpoint.')
    }
    config = {
      ...config,
      BASE_API_URL: base,
      CONSOLE_AUTHORIZE_URL: `${base}/oauth/authorize`,
      CLAUDE_AI_AUTHORIZE_URL: `${base}/oauth/authorize`,
      // ... all URLs overridden
    }
  }
  return config
}
```

**Security Pattern:** Allowlist prevents OAuth token leakage to arbitrary endpoints.

---

### 4.5 Output Style Loading (`outputStyles.ts` lines 137-175)

```typescript
export const getAllOutputStyles = memoize(async function getAllOutputStyles(
  cwd: string,
): Promise<{ [styleName: string]: OutputStyleConfig | null }> {
  const customStyles = await getOutputStyleDirStyles(cwd)
  const pluginStyles = await loadPluginOutputStyles()
  
  // Start with built-in modes
  const allStyles = { ...OUTPUT_STYLE_CONFIG }
  
  const managedStyles = customStyles.filter(s => s.source === 'policySettings')
  const userStyles = customStyles.filter(s => s.source === 'userSettings')
  const projectStyles = customStyles.filter(s => s.source === 'projectSettings')
  
  // Priority order (lowest to highest): built-in, plugin, user, project, managed
  const styleGroups = [pluginStyles, userStyles, projectStyles, managedStyles]
  
  for (const styles of styleGroups) {
    for (const style of styles) {
      allStyles[style.name] = { ...style }
    }
  }
  
  return allStyles
})
```

**Priority Chain:** Later sources override earlier — managed (policy) has highest priority.

---

### 4.6 Forced Plugin Output Styles (`outputStyles.ts` lines 181-211)

```typescript
export async function getOutputStyleConfig(): Promise<OutputStyleConfig | null> {
  const allStyles = await getAllOutputStyles(getCwd())
  
  // Check for forced plugin output styles FIRST (highest priority)
  const forcedStyles = Object.values(allStyles).filter(
    (style): style is OutputStyleConfig =>
      style !== null &&
      style.source === 'plugin' &&
      style.forceForPlugin === true,
  )
  
  const firstForcedStyle = forcedStyles[0]
  if (firstForcedStyle) {
    if (forcedStyles.length > 1) {
      logForDebugging(
        `Multiple plugins have forced output styles: ${forcedStyles.map(s => s.name).join(', ')}. Using: ${firstForcedStyle.name}`,
        { level: 'warn' },
      )
    }
    logForDebugging(`Using forced plugin output style: ${firstForcedStyle.name}`)
    return firstForcedStyle
  }
  
  // Fall back to user settings
  const settings = getSettings_DEPRECATED()
  const outputStyle = (settings?.outputStyle || DEFAULT_OUTPUT_STYLE_NAME) as string
  return allStyles[outputStyle] ?? null
}
```

**Plugin Auto-Apply:** `forceForPlugin: true` makes output style automatic when plugin enabled.

---

### 4.7 System Prompt Section Caching (`systemPromptSections.ts`)

```typescript
type ComputeFn = () => string | null | Promise<string | null>

type SystemPromptSection = {
  name: string
  compute: ComputeFn
  cacheBreak: boolean  // Recompute every turn?
}

/**
 * Create a memoized system prompt section.
 * Computed once, cached until /clear or /compact.
 */
export function systemPromptSection(
  name: string,
  compute: ComputeFn,
): SystemPromptSection {
  return { name, compute, cacheBreak: false }
}

/**
 * Create a volatile system prompt section that recomputes every turn.
 * Requires a reason explaining why cache-breaking is necessary.
 */
export function DANGEROUS_uncachedSystemPromptSection(
  name: string,
  compute: ComputeFn,
  _reason: string,  // Force documentation of why cache-breaking is needed
): SystemPromptSection {
  return { name, compute, cacheBreak: true }
}

export async function resolveSystemPromptSections(
  sections: SystemPromptSection[],
): Promise<(string | null)[]> {
  const cache = getSystemPromptSectionCache()
  
  return Promise.all(
    sections.map(async s => {
      if (!s.cacheBreak && cache.has(s.name)) {
        return cache.get(s.name) ?? null  // Cache hit
      }
      const value = await s.compute()
      setSystemPromptSectionCacheEntry(s.name, value)
      return value
    }),
  )
}
```

**Cache Strategy:** Most sections cached; volatile sections (MCP instructions) recompute every turn.

---

### 4.8 Attribution Header Construction (`system.ts`)

```typescript
export function getAttributionHeader(fingerprint: string): string {
  if (!isAttributionHeaderEnabled()) return ''
  
  const version = `${MACRO.VERSION}.${fingerprint}`
  const entrypoint = process.env.CLAUDE_CODE_ENTRYPOINT ?? 'unknown'
  
  // cch=00000 placeholder overwritten by Bun's HTTP stack with attestation token
  const cch = feature('NATIVE_CLIENT_ATTESTATION') ? ' cch=00000;' : ''
  
  // cc_workload: turn-scoped hint for QoS routing (e.g., cron-initiated → lower QoS)
  const workload = getWorkload()
  const workloadPair = workload ? ` cc_workload=${workload};` : ''
  
  const header = `x-anthropic-billing-header: cc_version=${version}; cc_entrypoint=${entrypoint};${cch}${workloadPair}`
  
  logForDebugging(`attribution header ${header}`)
  return header
}
```

**Attestation Mechanism:** Placeholder `00000` is same-length replacement — avoids Content-Length changes.

---

### 4.9 Spinner Verb Customization (`spinnerVerbs.ts`)

```typescript
export function getSpinnerVerbs(): string[] {
  const settings = getInitialSettings()
  const config = settings.spinnerVerbs
  
  if (!config) return SPINNER_VERBS
  
  if (config.mode === 'replace') {
    return config.verbs.length > 0 ? config.verbs : SPINNER_VERBS
  }
  // config.mode === 'append'
  return [...SPINNER_VERBS, ...config.verbs]
}

export const SPINNER_VERBS = [
  'Accomplishing', 'Actioning', 'Actualizing', 'Architecting',
  'Baking', 'Beaming', "Beboppin'", 'Befuddling', 'Billowing',
  // ... 187 total verbs
  'Wrangling', 'Zesting', 'Zigzagging',
]
```

**User Customization:** `replace` mode substitutes; `append` mode extends the built-in list.

---

### 4.10 GitHub Workflow Template (`github-app.ts`)

```typescript
export const WORKFLOW_CONTENT = `name: Claude Code

on:
  issue_comment:
    types: [created]
  pull_request_review_comment:
    types: [created]
  issues:
    types: [opened, assigned]
  pull_request_review:
    types: [submitted]

jobs:
  claude:
    if: |
      (github.event_name == 'issue_comment' && contains(github.event.comment.body, '@claude')) ||
      (github.event_name == 'pull_request_review_comment' && contains(github.event.comment.body, '@claude')) ||
      (github.event_name == 'pull_request_review' && contains(github.event.review.body, '@claude')) ||
      (github.event_name == 'issues' && (contains(github.event.issue.body, '@claude') || contains(github.event.issue.title, '@claude')))
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: read
      issues: read
      id-token: write
      actions: read  # Required for Claude to read CI results on PRs
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4
        with:
          fetch-depth: 1  # Shallow clone for speed
      - name: Run Claude Code
        id: claude
        uses: anthropics/claude-code-action@v1
        with:
          anthropic_api_key: \${{ secrets.ANTHROPIC_API_KEY }}
          additional_permissions: |
            actions: read  # Optional: Claude reads CI results
`
```

**Conditional Trigger:** Workflow triggers on `@claude` mentions in comments, reviews, and issues.

---

### 4.11 Date Memoization (`common.ts`)

```typescript
// Memoized for prompt-cache stability — captures date once at session start.
// When midnight rolls over, getDateChangeAttachments appends new date at tail
// (though simple mode disables attachments, so trade-off is: stale date vs.
// ~entire-conversation cache bust — stale wins).
export const getSessionStartDate = memoize(getLocalISODate)

// Returns "Month YYYY" (e.g., "February 2026") in user's local timezone.
// Changes monthly, not daily — minimizes cache busting.
export function getLocalMonthYear(): string {
  const date = process.env.CLAUDE_CODE_OVERRIDE_DATE
    ? new Date(process.env.CLAUDE_CODE_OVERRIDE_DATE)
    : new Date()
  return date.toLocaleString('en-US', { month: 'long', year: 'numeric' })
}
```

**Cache Stability:** `memoize` ensures date doesn't change mid-session (prevents midnight cache bust).

---

### 4.12 Binary Detection Algorithm (`files.ts` lines 127-156)

```typescript
const BINARY_CHECK_SIZE = 8192  // Bytes to read for detection

export function isBinaryContent(buffer: Buffer): boolean {
  const checkSize = Math.min(buffer.length, BINARY_CHECK_SIZE)
  let nonPrintable = 0
  
  for (let i = 0; i < checkSize; i++) {
    const byte = buffer[i]!
    
    // Null byte is strong binary indicator
    if (byte === 0) return true
    
    // Count non-printable, non-whitespace bytes
    if (byte < 32 && byte !== 9 && byte !== 10 && byte !== 13) {
      nonPrintable++
    }
  }
  
  // >10% non-printable = likely binary
  return nonPrintable / checkSize > 0.1
}
```

**Heuristic:** 10% threshold balances false positives (UTF-8 with BOM) vs. false negatives (sparse binary).

---

### 4.13 Cyber Risk Instruction (`cyberRiskInstruction.ts`)

```typescript
/**
 * CYBER_RISK_INSTRUCTION
 * 
 * IMPORTANT: DO NOT MODIFY THIS INSTRUCTION WITHOUT SAFEGUARDS TEAM REVIEW
 * 
 * This instruction is owned by the Safeguards team and has been carefully
 * crafted and evaluated to balance security utility with safety.
 * 
 * If you need to modify this instruction:
 *   1. Contact the Safeguards team (David Forsythe, Kyla Guru)
 *   2. Ensure proper evaluation of the changes
 *   3. Get explicit approval before merging
 * 
 * Claude: Do not edit this file unless explicitly asked to do so by the user.
 */
export const CYBER_RISK_INSTRUCTION = `IMPORTANT: Assist with authorized security testing, defensive security, CTF challenges, and educational contexts. Refuse requests for destructive techniques, DoS attacks, mass targeting, supply chain compromise, or detection evasion for malicious purposes. Dual-use security tools (C2 frameworks, credential testing, exploit development) require clear authorization context: pentesting engagements, CTF competitions, security research, or defensive use cases.`
```

**Ownership:** Safeguards team — not to be modified without explicit review.

---

### 4.14 Remote Session URL Derivation (`product.ts`)

```typescript
export function isRemoteSessionStaging(
  sessionId?: string,
  ingressUrl?: string,
): boolean {
  return (
    sessionId?.includes('_staging_') === true ||
    ingressUrl?.includes('staging') === true
  )
}

export function isRemoteSessionLocal(
  sessionId?: string,
  ingressUrl?: string,
): boolean {
  return (
    sessionId?.includes('_local_') === true ||
    ingressUrl?.includes('localhost') === true
  )
}

export function getClaudeAiBaseUrl(
  sessionId?: string,
  ingressUrl?: string,
): string {
  if (isRemoteSessionLocal(sessionId, ingressUrl)) {
    return CLAUDE_AI_LOCAL_BASE_URL  // http://localhost:4000
  }
  if (isRemoteSessionStaging(sessionId, ingressUrl)) {
    return CLAUDE_AI_STAGING_BASE_URL  // https://claude-ai.staging.ant.dev
  }
  return CLAUDE_AI_BASE_URL  // https://claude.ai
}
```

**Environment Detection:** Both session ID format and ingress URL checked for robustness.

---

### 4.15 Session ID Compatibility Translation (`product.ts` lines 65-76)

```typescript
/**
 * Get the full session URL for a remote session.
 * 
 * The cse_→session_ translation is a temporary shim gated by
 * tengu_bridge_repl_v2_cse_shim_enabled. Worker endpoints want `cse_*`
 * but the claude.ai frontend routes on `session_*`.
 */
export function getRemoteSessionUrl(
  sessionId: string,
  ingressUrl?: string,
): string {
  const { toCompatSessionId } = require('../bridge/sessionIdCompat.js')
  const compatId = toCompatSessionId(sessionId)
  const baseUrl = getClaudeAiBaseUrl(compatId, ingressUrl)
  return `${baseUrl}/code/${compatId}`
}
```

**Translation Layer:** `cse_*` (internal) → `session_*` (frontend-compatible).

---

## 5. Component Relationships

```
┌─────────────────────────────────────────────────────────────────┐
│                        constants/ (21 files)                     │
├─────────────────────────────────────────────────────────────────┤
│  API Limits          Beta Headers        OAuth Config           │
│  - apiLimits.ts      - betas.ts          - oauth.ts             │
│  - toolLimits.ts     - system.ts         - keys.ts              │
│  - tools.ts          - product.ts                               │
├─────────────────────────────────────────────────────────────────┤
│  System Prompts      Output Styles       File Handling          │
│  - prompts.ts        - outputStyles.ts   - files.ts             │
│  - systemPrompt...   - spinnerVerbs.ts   - cyberRisk...         │
│  - system.ts         - turnCompletion... - errorIds.ts          │
├─────────────────────────────────────────────────────────────────┤
│  Templates           XML Tags            Utilities              │
│  - github-app.ts     - xml.ts            - common.ts            │
│  - messages.ts       - figures.ts        - toolLimits.ts        │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Consumers                                 │
├─────────────────────────────────────────────────────────────────┤
│  bootstrap/state.ts    state/AppStateStore.ts                  │
│  tools/*               services/api/claude.ts                   │
│  utils/model/*         bridge/remoteBridgeCore.ts               │
└─────────────────────────────────────────────────────────────────┘
```

---

## 6. Data Flow

### 6.1 OAuth Config Selection

```
Application startup
   │
   ▼
getOauthConfig()
   │
   ├─ Check CLAUDE_CODE_CUSTOM_OAUTH_URL
   │  ├─ Validate against ALLOWED_OAUTH_BASE_URLS
   │  └─ Override all URLs if valid
   │
   ├─ Check USER_TYPE === 'ant'
   │  ├─ Check USE_LOCAL_OAUTH → local config
   │  ├─ Check USE_STAGING_OAUTH → staging config
   │  └─ Default → prod config
   │
   └─ External build → prod config (DCE eliminates staging)
   │
   ▼
Return OauthConfig for session
```

### 6.2 System Prompt Generation

```
getSystemPrompt(tools, model, ...)
   │
   ├─ Check CLAUDE_CODE_SIMPLE → minimal prompt
   │
   ├─ Compute static sections
   │  ├─ getSimpleIntroSection()
   │  ├─ getSimpleSystemSection()
   │  ├─ getActionsSection()
   │  ├─ getUsingYourToolsSection()
   │  ├─ getSimpleToneAndStyleSection()
   │  └─ getOutputEfficiencySection()
   │
   ├─ Compute dynamic sections (cached)
   │  ├─ getSessionSpecificGuidanceSection()
   │  ├─ loadMemoryPrompt()
   │  ├─ getAntModelOverrideSection()
   │  ├─ computeEnvInfo()
   │  ├─ getLanguageSection()
   │  ├─ getOutputStyleSection()
   │  ├─ getMcpInstructionsSection()
   │  ├─ getScratchpadInstructions()
   │  └─ getFunctionResultClearingSection()
   │
   └─ Join with SYSTEM_PROMPT_DYNAMIC_BOUNDARY marker
   │
   ▼
Return string[] for API request
```

### 6.3 Output Style Resolution

```
getOutputStyleConfig()
   │
   ├─ getAllOutputStyles(cwd)
   │  ├─ Load built-in styles (default, Explanatory, Learning)
   │  ├─ Load plugin styles
   │  ├─ Load custom styles (user, project, managed)
   │  └─ Merge by priority (managed wins)
   │
   ├─ Check for forced plugin styles
   │  └─ Return first forced style if exists
   │
   ├─ Read settings.outputStyle
   │
   └─ Return matching style or null
```

---

## 7. Key Patterns

### 7.1 Dead Code Elimination (DCE)

Feature-flagged exports eliminated at build time:

```typescript
// External builds: feature() always false → '' (empty string)
export const AFK_MODE_BETA_HEADER = feature('TRANSCRIPT_CLASSIFIER')
  ? 'afk-mode-2026-01-31'
  : ''

// External builds: USER_TYPE !== 'ant' → undefined
const STAGING_OAUTH_CONFIG = process.env.USER_TYPE === 'ant'
  ? ({...})
  : undefined
```

**Benefit:** External builds never contain staging/internal configurations.

---

### 7.2 Sticky-On Latches

Beta headers latched true for cache stability:

```typescript
// In bootstrap/state.ts
if (!getAfkModeHeaderLatched() && shouldEnableAfkMode()) {
  setAfkModeHeaderLatched(true)
}
// Once true, stays true for session
```

**Rationale:** Prompt cache (~50-70K tokens) busts on header change.

---

### 7.3 Memoization for Stability

```typescript
// Date captured once at session start
export const getSessionStartDate = memoize(getLocalISODate)

// Output styles cached by cwd
export const getAllOutputStyles = memoize(async function...)
```

**Benefit:** Prevents mid-session value changes from busting caches.

---

### 7.4 Type-Safe Allowlists

```typescript
export const BINARY_EXTENSIONS = new Set([...])  // 106 extensions
export const TERMINAL_OUTPUT_TAGS = [...] as const  // 6 tags
export const ALL_OAUTH_SCOPES = Array.from(new Set([...]))  // Union of scopes
```

**Benefit:** `Set`/`const` for fast lookups and type inference.

---

### 7.5 Platform-Specific Configuration

```typescript
// Bedrock: limited beta headers via extraBodyParams
export const BEDROCK_EXTRA_PARAMS_HEADERS = new Set([...])

// Vertex: limited betas for countTokens
export const VERTEX_COUNT_TOKENS_ALLOWED_BETAS = new Set([...])

// Tool search: different headers by provider
export const TOOL_SEARCH_BETA_HEADER_1P = 'advanced-tool-use-2025-11-20'  // Claude API
export const TOOL_SEARCH_BETA_HEADER_3P = 'tool-search-tool-2025-10-19'  // Vertex/Bedrock
```

---

## 8. Integration Points

### 8.1 With API Layer

| Constant | Usage |
|----------|-------|
| `API_IMAGE_MAX_BASE64_SIZE` | Image upload validation |
| `API_PDF_MAX_PAGES` | PDF validation |
| `API_MAX_MEDIA_PER_REQUEST` | Message validation |
| `getAttributionHeader()` | API request headers |
| Beta headers | `BetaMessageStreamParams` |

### 8.2 With Tool System

| Constant | Usage |
|----------|-------|
| `ALL_AGENT_DISALLOWED_TOOLS` | Tool filtering for agents |
| `ASYNC_AGENT_ALLOWED_TOOLS` | Fork subagent tool allowlist |
| `DEFAULT_MAX_RESULT_SIZE_CHARS` | Tool result persistence threshold |
| `MAX_TOOL_RESULT_TOKENS` | Result truncation |

### 8.3 With Settings System

| Constant | Usage |
|----------|-------|
| `OUTPUT_STYLE_CONFIG` | Available output styles |
| `getSpinnerVerbs()` | Spinner customization |
| `allowedSettingSources` | Setting source validation |

### 8.4 With Session Management

| Constant | Usage |
|----------|-------|
| `getSessionStartDate` | System prompt date |
| `getRemoteSessionUrl()` | Remote session link |
| `CYBER_RISK_INSTRUCTION` | System prompt injection |

---

## 9. Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `USER_TYPE` | Ant-only features | `undefined` |
| `USE_LOCAL_OAUTH` | Local OAuth for ant | `false` |
| `USE_STAGING_OAUTH` | Staging OAuth for ant | `false` |
| `CLAUDE_CODE_CUSTOM_OAUTH_URL` | FedStart deployment URL | `undefined` |
| `CLAUDE_CODE_OAUTH_CLIENT_ID` | Client ID override | `undefined` |
| `CLAUDE_CODE_ENTRYPOINT` | Attribution header entrypoint | `'unknown'` |
| `CLAUDE_CODE_OVERRIDE_DATE` | Date override for testing | `undefined` |
| `CLAUDE_CODE_SIMPLE` | Minimal system prompt | `false` |

---

## 10. Summary

The `constants/` module (21 files, ~2,651 lines) is Claude Code's **centralized definition layer** with:

1. **API Limits** — Image, PDF, media constraints from Anthropic API
2. **Beta Headers** — 17+ feature flags with platform-specific variants
3. **OAuth Configuration** — Prod/staging/local endpoints with allowlist validation
4. **System Prompts** — Dynamic prompt generation with section caching
5. **Output Styles** — Built-in (Explanatory, Learning) + custom style loading
6. **Tool Definitions** — Allowlists/blocklists for agent modes
7. **File Handling** — Binary detection by extension and content analysis
8. **XML Tags** — 30+ tag definitions for message structure
9. **Templates** — GitHub Actions workflow generation
10. **UI Constants** — 187 spinner verbs, Unicode figures

The module follows **DCE-first design** — feature-flagged exports ensure external builds never contain internal/staging configurations. Memoization and caching patterns optimize for prompt-cache stability across session lifetime.

---

**Last Updated:** 2026-04-07  
**Status:** Complete — all 21 files inventoried and analyzed
