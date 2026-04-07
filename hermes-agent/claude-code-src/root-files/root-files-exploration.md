---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/
explored_at: 2026-04-07
repository: claude-code-main
---

# Claude Code Root-Level TypeScript Files - Deep Dive Exploration

## Table of Contents

1. [File Inventory](#1-file-inventory)
2. [Module Overview](#2-module-overview)
3. [Key Exports by File](#3-key-exports-by-file)
4. [Line-by-Line Analysis of Critical Files](#4-line-by-line-analysis-of-critical-files)
5. [Integration Points](#5-integration-points)

---

## 1. File Inventory

| File | Lines | Key Exports Count | Primary Purpose |
|------|-------|-------------------|-----------------|
| `main.tsx` | ~4,684 | 15+ | Application entry point, CLI parsing, initialization |
| `commands.ts` | ~755 | 20+ | Command registry, slash command definitions |
| `tools.ts` | ~390 | 10+ | Tool pool assembly, tool registry |
| `Tool.ts` | ~793 | 25+ | Tool base class, interfaces, type definitions |
| `Task.ts` | ~126 | 8 | Task types, task state management |
| `QueryEngine.ts` | ~1,200 | 5+ | Query engine class, session state management |
| `query.ts` | ~1,500 | 10+ | Core query loop, API streaming, tool orchestration |
| `cost-tracker.ts` | ~324 | 15+ | Token cost tracking, usage analytics |
| `dialogLaunchers.tsx` | ~132 | 7 | Dialog/modal launcher functions |
| `history.ts` | ~465 | 10+ | History management, paste storage |
| `interactiveHelpers.tsx` | ~800 | 15+ | Interactive UI helpers, rendering |
| `setup.ts` | ~478 | 1 | Session setup, initialization |

**Total Lines:** ~11,547 lines of TypeScript/TSX code

---

## 2. Module Overview

### 2.1 main.tsx - Application Entry Point (~4,684 lines)

**Purpose:** The primary entry point for the Claude Code CLI application. Handles:
- CLI argument parsing with Commander.js
- Pre-initialization profiling and prefetching
- Feature flag gating and conditional module loading
- Session initialization and REPL launching
- Migration management for settings and configurations
- Entry point detection (CLI, SDK, GitHub Actions, etc.)

**Key Responsibilities:**
1. **Startup Profiling:** Uses `profileCheckpoint()` to mark entry points for performance analysis
2. **Early Prefetching:** Starts MDM reads, keychain prefetches in parallel with module loading
3. **Debug Detection:** Exits if Node.js debugging/inspection is detected
4. **Migration Management:** Runs sync migrations for settings, model strings, auto-updates
5. **CLI Parsing:** Full Commander.js setup with 40+ options including:
   - `-p/--print` for non-interactive mode
   - `--model`, `--effort`, `--thinking` for model configuration
   - `--resume`, `--continue` for session management
   - `--permission-mode`, `--dangerously-skip-permissions` for security
6. **Entry Point Detection:** Distinguishes between CLI, SDK, GitHub Actions, Desktop, Remote modes

### 2.2 commands.ts - Command Registry (~755 lines)

**Purpose:** Central registry for all slash commands available in Claude Code.

**Key Responsibilities:**
1. **Built-in Command Imports:** Imports 60+ built-in commands (help, config, mcp, etc.)
2. **Feature-Gated Commands:** Conditionally loads commands based on feature flags:
   - `PROACTIVE/KAIROS`: proactive, brief, assistant commands
   - `BRIDGE_MODE`: bridge, remoteControlServer commands
   - `WORKFLOW_SCRIPTS`: workflows command
   - `UDS_INBOX`: peers command
3. **Skill Loading:** Loads skills from directories, plugins, and bundled sources
4. **Command Filtering:** Filters commands by availability (auth requirements) and enabled state
5. **Remote Mode Safety:** Defines `REMOTE_SAFE_COMMANDS` and `BRIDGE_SAFE_COMMANDS` sets
6. **Memoization:** Uses lodash memoize for expensive command loading operations

### 2.3 tools.ts - Tool Pool Assembly (~390 lines)

**Purpose:** Assembles the complete tool pool from built-in tools and MCP tools.

**Key Responsibilities:**
1. **Tool Imports:** Imports 40+ tool classes (BashTool, FileReadTool, AgentTool, etc.)
2. **Feature-Gated Tools:** Conditionally includes tools based on environment:
   - `USER_TYPE === 'ant'`: ConfigTool, TungstenTool, REPLTool
   - `MONITOR_TOOL`: MonitorTool
   - `KAIROS`: SleepTool, SendUserFileTool, PushNotificationTool
3. **Tool Presets:** Defines `TOOL_PRESETS` (currently just 'default')
4. **Permission Filtering:** `filterToolsByDenyRules()` removes blanket-denied tools
5. **Tool Pool Assembly:** `assembleToolPool()` combines built-in and MCP tools with deduplication
6. **Simple Mode:** Returns only Bash, Read, Edit tools in `CLAUDE_CODE_SIMPLE` mode

### 2.4 Tool.ts - Tool Base Class and Interfaces (~793 lines)

**Purpose:** Defines the core Tool interface and all related types for tool implementation.

**Key Responsibilities:**
1. **Tool Interface:** Complete Tool interface with 35+ properties/methods including:
   - `call()`: Execute the tool
   - `description()`: Generate tool description for the model
   - `inputSchema`: Zod schema for input validation
   - `isConcurrencySafe()`, `isReadOnly()`, `isDestructive()`
   - `renderToolUseMessage()`, `renderToolResultMessage()`
   - `checkPermissions()`: Tool-specific permission checks
2. **Tool Defaults:** `TOOL_DEFAULTS` object with safe defaults for optional methods
3. **buildTool()**: Helper function to create tools with defaults applied
4. **Type Definitions:**
   - `ToolUseContext`: Context passed to tool calls (60+ properties)
   - `ToolPermissionContext`: Permission filtering context
   - `ToolResult<T>`: Tool execution result type
   - `Tools`: Readonly array of Tool instances
5. **Utility Functions:** `toolMatchesName()`, `findToolByName()` for tool lookup

### 2.5 Task.ts - Task State Management (~126 lines)

**Purpose:** Defines task types and state management for background tasks.

**Key Responsibilities:**
1. **Task Type Definitions:**
   - `TaskType`: 'local_bash', 'local_agent', 'remote_agent', 'in_process_teammate', etc.
   - `TaskStatus`: 'pending', 'running', 'completed', 'failed', 'killed'
2. **Task State Base:** Common fields for all task states
3. **Task ID Generation:** `generateTaskId()` creates prefixed IDs (b=local_bash, a=local_agent, etc.)
4. **Terminal Status Check:** `isTerminalTaskStatus()` guards against dead task operations

### 2.6 QueryEngine.ts - Query Engine Class (~1,200 lines)

**Purpose:** Manages the query lifecycle and session state for conversations.

**Key Responsibilities:**
1. **QueryEngine Class:** One instance per conversation, manages:
   - Mutable message store
   - Abort controller for cancellation
   - Permission denial tracking
   - Usage accumulation
   - File read cache
2. **submitMessage():** Main generator function that:
   - Processes user input and slash commands
   - Builds system prompts with memory mechanics
   - Calls the core query loop
   - Yields SDK-compatible messages
3. **State Management:** Tracks discovered skills, loaded memory paths across turns
4. **Permission Tracking:** Wraps canUseTool to track denials for SDK reporting
5. **Session Persistence:** Records transcripts and flushes session storage

### 2.7 query.ts - Core Query Loop (~1,500 lines)

**Purpose:** Implements the core query loop that communicates with the Anthropic API.

**Key Responsibilities:**
1. **Query Loop:** Main generator that:
   - Applies context management (snip, microcompact, autocompact, context collapse)
   - Calls the model API with streaming
   - Executes tools via StreamingToolExecutor
   - Handles recovery from token limit errors
2. **Token Budget Management:** `createBudgetTracker()` and `checkTokenBudget()` for token limits
3. **Context Management:**
   - `snipCompactIfNeeded()`: Removes old history while preserving context
   - `microcompact()`: Caches editing for context reduction
   - `autocompact()`: Automatic context compaction when thresholds exceeded
   - `applyCollapsesIfNeeded()`: Context collapse for granular summaries
4. **Tool Execution:** Integrates with `runTools()` for tool orchestration
5. **Error Recovery:** Handles max_output_tokens errors with fallback models

### 2.8 cost-tracker.ts - Token Cost Tracking (~324 lines)

**Purpose:** Tracks API costs, token usage, and session metrics.

**Key Responsibilities:**
1. **Cost Functions:**
   - `addToTotalSessionCost()`: Adds cost and usage, tracks advisor costs
   - `formatTotalCost()`: Formats cost display with model breakdown
   - `getTotalCostUSD()`: Returns accumulated session cost
2. **Usage Tracking:**
   - Input/output tokens per model
   - Cache read/creation tokens
   - Web search requests
   - Lines added/removed
3. **Session Persistence:**
   - `saveCurrentSessionCosts()`: Saves costs to project config
   - `restoreCostStateForSession()`: Restores costs when resuming
   - `getStoredSessionCosts()`: Reads stored cost data
4. **Analytics Integration:** Logs usage to Statsig via `getCostCounter()`, `getTokenCounter()`

### 2.9 dialogLaunchers.tsx - Dialog Launchers (~132 lines)

**Purpose:** Thin wrappers for launching modal dialogs dynamically.

**Key Responsibilities:**
1. **Dialog Launchers:**
   - `launchSnapshotUpdateDialog()`: Agent memory snapshot update prompt
   - `launchInvalidSettingsDialog()`: Settings validation errors
   - `launchAssistantSessionChooser()`: Bridge session picker
   - `launchAssistantInstallWizard()`: Assistant installation wizard
   - `launchTeleportResumeWrapper()`: Teleport session picker
   - `launchTeleportRepoMismatchDialog()`: Repository mismatch resolver
   - `launchResumeChooser()`: Session resume picker
2. **Dynamic Imports:** All components are dynamically imported to reduce bundle size
3. **Callback Wiring:** Consistent `done` callback pattern for all dialogs

### 2.10 history.ts - History Management (~465 lines)

**Purpose:** Manages command history and pasted content storage.

**Key Responsibilities:**
1. **History Storage:**
   - JSONL format in `~/.claude/history.jsonl`
   - File locking for concurrent access
   - Pending entries buffer with async flush
2. **Pasted Content:**
   - Hash-based storage for large text (>1KB)
   - Inline storage for small text
   - Image exclusion (stored separately)
3. **History Readers:**
   - `getHistory()`: Project-scoped history with session ordering
   - `getTimestampedHistory()`: For ctrl+r picker with timestamps
   - `makeHistoryReader()`: Full history reader
4. **Reference Expansion:** `expandPastedTextRefs()` replaces placeholders with actual content
5. **Undo Support:** `removeLastFromHistory()` for auto-restore-on-interrupt

### 2.11 interactiveHelpers.tsx - Interactive UI Helpers (~800 lines)

**Purpose:** Helper functions for interactive terminal UI rendering and management.

**Key Responsibilities:**
1. **Rendering:**
   - `renderAndRun()`: Mount Ink components to terminal
   - `showSetupDialog()`: Show modal dialogs blocking input
   - `showSetupScreens()`: Multi-step setup wizard
2. **Exit Handling:**
   - `exitWithError()`: Error exit with message
   - `exitWithMessage()`: Clean exit with message
   - `getRenderContext()`: Get current Ink render context
3. **Cursor Management:** `resetCursor()` restores cursor on exit
4. **App State Integration:** Wire up AppState changes to UI updates

### 2.12 setup.ts - Session Setup (~478 lines)

**Purpose:** Performs session initialization and setup tasks.

**Key Responsibilities:**
1. **Environment Validation:**
   - Node.js version check (requires >= 18)
   - Root/sudo restrictions for `--dangerously-skip-permissions`
   - Docker/sandbox validation
2. **Worktree Management:**
   - Creates worktrees for isolated sessions
   - Optional tmux session creation
   - Main repo resolution when inside worktrees
3. **Background Jobs:**
   - Session memory initialization
   - Context collapse initialization
   - Version locking
   - Attribution hooks (ant-only)
   - Team memory sync watcher
4. **Prefetching:**
   - API key prefetch from helper
   - Release notes check
   - Command loading
   - Plugin hooks loading
5. **Terminal Restoration:**
   - iTerm2 backup restoration
   - Terminal.app backup restoration

---

## 3. Key Exports by File

### 3.1 main.tsx Exports

```typescript
// Main entry function
export async function main(): Promise<void>

// Deferred prefetches (called after first render)
export function startDeferredPrefetches(): void

// Type exports
type PendingConnect = { ... }
type PendingAssistantChat = { ... }
type PendingSSH = { ... }
```

### 3.2 commands.ts Exports

```typescript
// Command loading
export async function getCommands(cwd: string): Promise<Command[]>
export function getSkillToolCommands(cwd: string): Promise<Command[]>
export function getSlashCommandToolSkills(cwd: string): Promise<Command[]>

// Command filtering
export function filterCommandsForRemoteMode(commands: Command[]): Command[]
export function isBridgeSafeCommand(cmd: Command): boolean
export function meetsAvailabilityRequirement(cmd: Command): boolean

// Command lookup
export function findCommand(commandName: string, commands: Command[]): Command | undefined
export function getCommand(commandName: string, commands: Command[]): Command
export function hasCommand(commandName: string, commands: Command[]): boolean

// MCP skills
export function getMcpSkillCommands(mcpCommands: readonly Command[]): readonly Command[]

// Cache management
export function clearCommandsCache(): void
export function clearCommandMemoizationCaches(): void

// Safe command sets
export const REMOTE_SAFE_COMMANDS: Set<Command>
export const BRIDGE_SAFE_COMMANDS: Set<Command>
export const INTERNAL_ONLY_COMMANDS: Command[]
```

### 3.3 tools.ts Exports

```typescript
// Tool assembly
export function assembleToolPool(
  permissionContext: ToolPermissionContext,
  mcpTools: Tools
): Tools

export function getMergedTools(
  permissionContext: ToolPermissionContext,
  mcpTools: Tools
): Tools

export function getTools(permissionContext: ToolPermissionContext): Tools
export function getAllBaseTools(): Tools

// Tool filtering
export function filterToolsByDenyRules<T>(
  tools: readonly T[],
  permissionContext: ToolPermissionContext
): T[]

// Tool presets
export function getToolsForDefaultPreset(): string[]
export function parseToolPreset(preset: string): ToolPreset | null

// Constants
export const TOOL_PRESETS: readonly string[]
export const REPL_ONLY_TOOLS: Set<string>
export const ALL_AGENT_DISALLOWED_TOOLS: string[]
export const CUSTOM_AGENT_DISALLOWED_TOOLS: string[]
export const ASYNC_AGENT_ALLOWED_TOOLS: string[]
export const COORDINATOR_MODE_ALLOWED_TOOLS: string[]
```

### 3.4 Tool.ts Exports

```typescript
// Tool utilities
export function toolMatchesName(
  tool: { name: string; aliases?: string[] },
  name: string
): boolean

export function findToolByName(tools: Tools, name: string): Tool | undefined

// Tool builder
export function buildTool<D extends AnyToolDef>(def: D): BuiltTool<D>

// Empty permission context
export const getEmptyToolPermissionContext: () => ToolPermissionContext

// Progress filtering
export function filterToolProgressMessages(
  progressMessagesForMessage: ProgressMessage[]
): ProgressMessage<ToolProgressData>[]

// Type exports
export type Tool<Input, Output, P> = { ... }
export type ToolDef<Input, Output, P> = { ... }
export type Tools = readonly Tool[]
export type ToolUseContext = { ... }
export type ToolPermissionContext = { ... }
export type ToolResult<T> = { ... }
export type ToolProgress<P> = { ... }
```

### 3.5 Task.ts Exports

```typescript
// Task state utilities
export function isTerminalTaskStatus(status: TaskStatus): boolean
export function generateTaskId(type: TaskType): string
export function createTaskStateBase(
  id: string,
  type: TaskType,
  description: string,
  toolUseId?: string
): TaskStateBase

// Type exports
export type TaskType = 'local_bash' | 'local_agent' | ...
export type TaskStatus = 'pending' | 'running' | ...
export type TaskHandle = { taskId: string; cleanup?: () => void }
export type TaskContext = { ... }
export type TaskStateBase = { ... }
```

### 3.6 QueryEngine.ts Exports

```typescript
// Main class
export class QueryEngine {
  constructor(config: QueryEngineConfig)
  submitMessage(prompt: string | ContentBlockParam[]): AsyncGenerator<SDKMessage>
}

// Type exports
export type QueryEngineConfig = { ... }
```

### 3.7 query.ts Exports

```typescript
// Main query function
export async function* query(params: QueryParams): AsyncGenerator<...>

// Type exports
export type QueryParams = { ... }
```

### 3.8 cost-tracker.ts Exports

```typescript
// Cost tracking
export function addToTotalSessionCost(
  cost: number,
  usage: Usage,
  model: string
): number

export function formatTotalCost(): string
export function formatCost(cost: number, maxDecimalPlaces?: number): string

// Session persistence
export function saveCurrentSessionCosts(fpsMetrics?: FpsMetrics): void
export function restoreCostStateForSession(sessionId: string): boolean
export function getStoredSessionCosts(sessionId: string): StoredCostState | undefined

// Re-exports from state
export function getTotalCostUSD(): number
export function getTotalDuration(): number
export function getTotalInputTokens(): number
export function getTotalOutputTokens(): number
export function hasUnknownModelCost(): boolean
export function resetCostState(): void
export function getModelUsage(): { [modelName: string]: ModelUsage }
export function getUsageForModel(model: string): ModelUsage | undefined
```

### 3.9 dialogLaunchers.tsx Exports

```typescript
export async function launchSnapshotUpdateDialog(
  root: Root,
  props: { agentType: string; scope: AgentMemoryScope; snapshotTimestamp: string }
): Promise<'merge' | 'keep' | 'replace'>

export async function launchInvalidSettingsDialog(
  root: Root,
  props: { settingsErrors: ValidationError[]; onExit: () => void }
): Promise<void>

export async function launchAssistantSessionChooser(
  root: Root,
  props: { sessions: AssistantSession[] }
): Promise<string | null>

export async function launchAssistantInstallWizard(
  root: Root
): Promise<string | null>

export async function launchTeleportResumeWrapper(
  root: Root
): Promise<TeleportRemoteResponse | null>

export async function launchTeleportRepoMismatchDialog(
  root: Root,
  props: { targetRepo: string; initialPaths: string[] }
): Promise<string | null>

export async function launchResumeChooser(
  root: Root,
  appProps: { ... },
  worktreePathsPromise: Promise<string[]>,
  resumeProps: Omit<ResumeConversationProps, 'worktreePaths'>
): Promise<void>
```

### 3.10 history.ts Exports

```typescript
// History reading
export async function* getHistory(): AsyncGenerator<HistoryEntry>
export async function* getTimestampedHistory(): AsyncGenerator<TimestampedHistoryEntry>
export async function* makeHistoryReader(): AsyncGenerator<HistoryEntry>

// History manipulation
export function addToHistory(command: HistoryEntry | string): void
export function removeLastFromHistory(): void
export function clearPendingHistoryEntries(): void

// Pasted content formatting
export function formatPastedTextRef(id: number, numLines: number): string
export function formatImageRef(id: number): string
export function parseReferences(input: string): Array<{ id: number; match: string; index: number }>
export function expandPastedTextRefs(
  input: string,
  pastedContents: Record<number, PastedContent>
): string
export function getPastedTextRefNumLines(text: string): number
```

### 3.11 interactiveHelpers.tsx Exports

```typescript
// Rendering
export async function renderAndRun(root: Root, element: React.ReactNode): Promise<void>
export async function showSetupDialog<T>(root: Root, render: (done: (result: T) => void) => React.ReactNode): Promise<T>
export async function showSetupScreens(root: Root, screens: SetupScreen[]): Promise<void>

// Exit handling
export function exitWithError(error: unknown): never
export function exitWithMessage(message: string, code?: number): never
export function getRenderContext(): RenderContext | null

// Cursor management
export function resetCursor(): void
```

### 3.12 setup.ts Exports

```typescript
// Main setup function
export async function setup(
  cwd: string,
  permissionMode: PermissionMode,
  allowDangerouslySkipPermissions: boolean,
  worktreeEnabled: boolean,
  worktreeName: string | undefined,
  tmuxEnabled: boolean,
  customSessionId?: string | null,
  worktreePRNumber?: number,
  messagingSocketPath?: string
): Promise<void>
```

---

## 4. Line-by-Line Analysis of Critical Files

### 4.1 main.tsx - Application Entry Point

#### Module Evaluation Phase (Lines 1-20)

```typescript
// Startup profiling checkpoint BEFORE any imports
profileCheckpoint('main_tsx_entry');

// Start MDM (Mobile Device Management) raw read in parallel
// This fetches macOS defaults / Windows registry settings
startMdmRawRead();

// Start keychain prefetch in parallel
// Reads OAuth tokens and legacy API keys from macOS keychain
startKeychainPrefetch();
```

These three lines implement critical startup optimization:
1. **Profile checkpoint** marks the entry for performance analysis
2. **MDM read** fires a subprocess that runs in parallel with ~135ms of module loading
3. **Keychain prefetch** avoids ~65ms sync reads later during settings initialization

#### Imports Section (Lines 20-206)

The imports are organized into logical groups:

1. **Core Dependencies** (Lines 20-27):
   - `feature` from `bun:bundle` for feature flag gating
   - Commander.js for CLI parsing
   - Chalk for terminal colors
   - React for UI components
   - Lodash utilities

2. **Application Modules** (Lines 28-72):
   - Constants, context, entrypoints
   - History, commands, tools
   - Analytics, API, MCP services
   - Lazy requires for circular dependency breaking

3. **Feature-Gated Imports** (Lines 73-122):
   - `COORDINATOR_MODE`: coordinator mode module
   - `KAIROS`: assistant mode modules
   - Dynamic imports that are tree-shaken in external builds

4. **Utility Modules** (Lines 123-206):
   - Auth, config, settings
   - Git, filesystem, platform detection
   - Session management, migrations
   - Plugin and skill management

#### Debug Detection (Lines 230-270)

```typescript
function isBeingDebugged() {
  const isBun = isRunningWithBun();
  
  // Check for inspect flags in process arguments
  const hasInspectArg = process.execArgv.some(arg => {
    if (isBun) {
      return /--inspect(-brk)?/.test(arg);
    } else {
      return /--inspect(-brk)?|--debug(-brk)?/.test(arg);
    }
  });
  
  // Check NODE_OPTIONS for inspect flags
  const hasInspectEnv = process.env.NODE_OPTIONS && 
    /--inspect(-brk)?|--debug(-brk)?/.test(process.env.NODE_OPTIONS);
  
  // Check if inspector is available and active
  try {
    const inspector = (global as any).require('inspector');
    const hasInspectorUrl = !!inspector.url();
    return hasInspectorUrl || hasInspectArg || hasInspectEnv;
  } catch {
    return hasInspectArg || hasInspectEnv;
  }
}

// Exit if debugging detected (external builds only)
if ("external" !== 'ant' && isBeingDebugged()) {
  process.exit(1);
}
```

**Security Purpose:** Prevents debugging of external builds (non-ant versions). This protects proprietary code from runtime inspection.

#### Migration System (Lines 322-351)

```typescript
const CURRENT_MIGRATION_VERSION = 11;

function runMigrations(): void {
  if (getGlobalConfig().migrationVersion !== CURRENT_MIGRATION_VERSION) {
    migrateAutoUpdatesToSettings();
    migrateBypassPermissionsAcceptedToSettings();
    migrateEnableAllProjectMcpServersToSettings();
    resetProToOpusDefault();
    migrateSonnet1mToSonnet45();
    migrateLegacyOpusToCurrent();
    migrateSonnet45ToSonnet46();
    migrateOpusToOpus1m();
    migrateReplBridgeEnabledToRemoteControlAtStartup();
    
    if (feature('TRANSCRIPT_CLASSIFIER')) {
      resetAutoModeOptInForDefaultOffer();
    }
    
    if ("external" === 'ant') {
      migrateFennecToOpus();
    }
    
    saveGlobalConfig(prev => 
      prev.migrationVersion === CURRENT_MIGRATION_VERSION 
        ? prev 
        : { ...prev, migrationVersion: CURRENT_MIGRATION_VERSION }
    );
  }
  
  // Async migration - fire and forget
  migrateChangelogFromConfig().catch(() => {});
}
```

**Migration Pattern:**
1. Check current migration version against config
2. Run all migrations sequentially (order matters!)
3. Update migration version atomically
4. Fire-and-forget async migrations

#### Deferred Prefetches (Lines 381-430)

```typescript
export function startDeferredPrefetches(): void {
  // Skip when benchmarking or in bare mode
  if (isEnvTruthy(process.env.CLAUDE_CODE_EXIT_AFTER_FIRST_RENDER) ||
      isBareMode()) {
    return;
  }

  // Process-spawning prefetches (user is still typing)
  void initUser();
  void getUserContext();
  prefetchSystemContextIfSafe();
  void getRelevantTips();
  
  // Provider credential prefetches
  if (isEnvTruthy(process.env.CLAUDE_CODE_USE_BEDROCK)) {
    void prefetchAwsCredentialsAndBedRockInfoIfSafe();
  }
  if (isEnvTruthy(process.env.CLAUDE_CODE_USE_VERTEX)) {
    void prefetchGcpCredentialsIfSafe();
  }
  
  // File counting (3s timeout)
  void countFilesRoundedRg(getCwd(), AbortSignal.timeout(3000), []);
  
  // Analytics and feature initialization
  void initializeAnalyticsGates();
  void prefetchOfficialMcpUrls();
  void refreshModelCapabilities();
  
  // File change detectors
  void settingsChangeDetector.initialize();
  if (!isBareMode()) {
    void skillChangeDetector.initialize();
  }
  
  // Event loop stall detector (ant-only)
  if ("external" === 'ant') {
    void import('./utils/eventLoopStallDetector.js')
      .then(m => m.startEventLoopStallDetector());
  }
}
```

**Performance Pattern:** These prefetches run AFTER first render, hiding latency behind user typing time.

#### CLI Option Definition (Lines 884-999+)

```typescript
async function run(): Promise<CommanderCommand> {
  const program = new CommanderCommand()
    .configureHelp(createSortedHelpConfig())
    .enablePositionalOptions();
  
  // Pre-action hook runs before ANY command executes
  program.hook('preAction', async thisCommand => {
    // Await prefetches started at module evaluation
    await Promise.all([
      ensureMdmSettingsLoaded(),
      ensureKeychainPrefetchCompleted()
    ]);
    
    // Initialize telemetry, auth, settings
    await init();
    
    // Set process title
    if (!isEnvTruthy(process.env.CLAUDE_CODE_DISABLE_TERMINAL_TITLE)) {
      process.title = 'claude';
    }
    
    // Attach logging sinks
    const { initSinks } = await import('./utils/sinks.js');
    initSinks();
    
    // Wire up --plugin-dir for subcommands
    const pluginDir = thisCommand.getOptionValue('pluginDir');
    if (Array.isArray(pluginDir) && pluginDir.length > 0) {
      setInlinePlugins(pluginDir);
      clearPluginCache('preAction: --plugin-dir inline plugins');
    }
    
    // Run migrations
    runMigrations();
    
    // Load remote managed settings (non-blocking)
    void loadRemoteManagedSettings();
    void loadPolicyLimits();
    
    // Upload user settings (non-blocking)
    if (feature('UPLOAD_USER_SETTINGS')) {
      void import('./services/settingsSync/index.js')
        .then(m => m.uploadUserSettingsInBackground());
    }
  });
  
  // Define command options...
  program
    .name('claude')
    .description('Claude Code - starts an interactive session by default')
    .argument('[prompt]', 'Your prompt', String)
    .helpOption('-h, --help', 'Display help for command')
    .option('-d, --debug [filter]', 'Enable debug mode')
    .option('-p, --print', 'Print response and exit')
    .option('--bare', 'Minimal mode')
    // ... 40+ more options
}
```

### 4.2 commands.ts - Command Registry

#### Command Imports (Lines 1-141)

```typescript
// Built-in command imports
import addDir from './commands/add-dir/index.js'
import help from './commands/help/index.js'
import config from './commands/config/index.js'
import mcp from './commands/mcp/index.js'
// ... 50+ more

// Feature-gated command imports (lazy require pattern)
const agentsPlatform = process.env.USER_TYPE === 'ant'
  ? require('./commands/agents-platform/index.js').default
  : null

const proactive = feature('PROACTIVE') || feature('KAIROS')
  ? require('./commands/proactive.js').default
  : null

const bridge = feature('BRIDGE_MODE')
  ? require('./commands/bridge/index.js').default
  : null
```

**Pattern:** Built-in commands use static imports (included in all builds). Feature-gated commands use `require()` with dead code elimination via `bun:bundle` feature flags.

#### INTERNAL_ONLY_COMMANDS (Lines 224-254)

```typescript
export const INTERNAL_ONLY_COMMANDS = [
  backfillSessions,
  breakCache,
  bughunter,
  commit,
  commitPushPr,
  ctx_viz,
  goodClaude,
  issue,
  initVerifiers,
  forceSnip,      // feature-gated
  mockLimits,
  bridgeKick,
  version,
  ultraplan,      // feature-gated
  subscribePr,    // feature-gated
  resetLimits,
  resetLimitsNonInteractive,
  onboarding,
  share,
  summary,
  teleport,
  antTrace,
  perfIssue,
  env,
  oauthRefresh,
  debugToolCall,
  agentsPlatform, // feature-gated
  autofixPr,
].filter(Boolean)
```

These commands are ONLY available when `USER_TYPE === 'ant'` (Anthropic internal builds).

#### COMMANDS Memoization (Lines 256-346)

```typescript
const COMMANDS = memoize((): Command[] => [
  addDir,
  advisor,
  agents,
  branch,
  // ... 50+ built-in commands
  
  // Feature-gated commands
  ...(webCmd ? [webCmd] : []),
  ...(forkCmd ? [forkCmd] : []),
  ...(proactive ? [proactive] : []),
  ...(briefCommand ? [briefCommand] : []),
  ...(assistantCommand ? [assistantCommand] : []),
  ...(bridge ? [bridge] : []),
  ...(remoteControlServerCommand ? [remoteControlServerCommand] : []),
  ...(voiceCommand ? [voiceCommand] : []),
  
  thinkback,
  thinkbackPlay,
  permissions,
  plan,
  privacySettings,
  hooks,
  exportCommand,
  sandboxToggle,
  
  // Auth-gated commands
  ...(!isUsing3PServices() ? [logout, login()] : []),
  
  passes,
  peersCmd ? [peersCmd] : [],
  tasks,
  workflowsCmd ? [workflowsCmd] : [],
  
  // Internal only (ant users)
  ...(process.env.USER_TYPE === 'ant' && !process.env.IS_DEMO
    ? INTERNAL_ONLY_COMMANDS
    : []),
])
```

**Memoization Pattern:** `lodash/memoize` caches by `cwd` since command loading depends on the current directory (for skill/plugin discovery).

#### getCommands (Lines 476-517)

```typescript
export async function getCommands(cwd: string): Promise<Command[]> {
  // Load all command sources (memoized by cwd)
  const allCommands = await loadAllCommands(cwd)
  
  // Get dynamic skills discovered during file operations
  const dynamicSkills = getDynamicSkills()
  
  // Filter by availability and enabled state
  const baseCommands = allCommands.filter(
    _ => meetsAvailabilityRequirement(_) && isCommandEnabled(_)
  )
  
  if (dynamicSkills.length === 0) {
    return baseCommands
  }
  
  // Dedupe dynamic skills
  const baseCommandNames = new Set(baseCommands.map(c => c.name))
  const uniqueDynamicSkills = dynamicSkills.filter(
    s => !baseCommandNames.has(s.name) &&
         meetsAvailabilityRequirement(s) &&
         isCommandEnabled(s)
  )
  
  if (uniqueDynamicSkills.length === 0) {
    return baseCommands
  }
  
  // Insert dynamic skills after plugin skills but before built-in commands
  const builtInNames = new Set(COMMANDS().map(c => c.name))
  const insertIndex = baseCommands.findIndex(c => builtInNames.has(c.name))
  
  if (insertIndex === -1) {
    return [...baseCommands, ...uniqueDynamicSkills]
  }
  
  return [
    ...baseCommands.slice(0, insertIndex),
    ...uniqueDynamicSkills,
    ...baseCommands.slice(insertIndex),
  ]
}
```

**Ordering Logic:** Dynamic skills are inserted between plugin skills and built-in commands for consistent ordering.

#### Remote Mode Safety (Lines 618-686)

```typescript
export const REMOTE_SAFE_COMMANDS: Set<Command> = new Set([
  session,      // Shows QR code / URL for remote session
  exit,         // Exit the TUI
  clear,        // Clear screen
  help,         // Show help
  theme,        // Change terminal theme
  color,        // Change agent color
  vim,          // Toggle vim mode
  cost,         // Show session cost (local tracking)
  usage,        // Show usage info
  copy,         // Copy last message
  btw,          // Quick note
  feedback,     // Send feedback
  plan,         // Plan mode toggle
  keybindings,  // Keybinding management
  statusline,   // Status line toggle
  stickers,     // Stickers
  mobile,       // Mobile QR code
])

export const BRIDGE_SAFE_COMMANDS: Set<Command> = new Set([
  compact,      // Shrink context
  clear,        // Wipe transcript
  cost,         // Show session cost
  summary,      // Summarize conversation
  releaseNotes, // Show changelog
  files,        // List tracked files
])

export function filterCommandsForRemoteMode(commands: Command[]): Command[] {
  return commands.filter(cmd => REMOTE_SAFE_COMMANDS.has(cmd))
}

export function isBridgeSafeCommand(cmd: Command): boolean {
  if (cmd.type === 'local-jsx') return false
  if (cmd.type === 'prompt') return true
  return BRIDGE_SAFE_COMMANDS.has(cmd)
}
```

**Security Model:** Remote commands (from mobile/web clients) are restricted to TUI-only operations that don't affect the local filesystem or execution.

### 4.3 tools.ts - Tool Pool Assembly

#### Tool Imports (Lines 1-96)

```typescript
// Core tool imports
import { AgentTool } from './tools/AgentTool/AgentTool.js'
import { BashTool } from './tools/BashTool/BashTool.js'
import { FileEditTool } from './tools/FileEditTool/FileEditTool.js'
import { FileReadTool } from './tools/FileReadTool/FileReadTool.js'
import { FileWriteTool } from './tools/FileWriteTool/FileWriteTool.js'
// ... 30+ more

// Feature-gated tools (lazy require pattern)
const REPLTool = process.env.USER_TYPE === 'ant'
  ? require('./tools/REPLTool/REPLTool.js').REPLTool
  : null

const SleepTool = feature('PROACTIVE') || feature('KAIROS')
  ? require('./tools/SleepTool/SleepTool.js').SleepTool
  : null

const cronTools = feature('AGENT_TRIGGERS') ? [
  require('./tools/ScheduleCronTool/CronCreateTool.js').CronCreateTool,
  require('./tools/ScheduleCronTool/CronDeleteTool.js').CronDeleteTool,
  require('./tools/ScheduleCronTool/CronListTool.js').CronListTool,
] : []

const MonitorTool = feature('MONITOR_TOOL')
  ? require('./tools/MonitorTool/MonitorTool.js').MonitorTool
  : null

// Lazy require for circular dependency breaking
const getTeamCreateTool = () =>
  require('./tools/TeamCreateTool/TeamCreateTool.js').TeamCreateTool
```

#### getAllBaseTools (Lines 190-251)

```typescript
export function getAllBaseTools(): Tools {
  return [
    AgentTool,
    TaskOutputTool,
    BashTool,
    
    // Conditionally exclude Glob/Grep when embedded search tools available
    ...(hasEmbeddedSearchTools() ? [] : [GlobTool, GrepTool]),
    
    ExitPlanModeV2Tool,
    FileReadTool,
    FileEditTool,
    FileWriteTool,
    NotebookEditTool,
    WebFetchTool,
    TodoWriteTool,
    WebSearchTool,
    TaskStopTool,
    AskUserQuestionTool,
    SkillTool,
    EnterPlanModeTool,
    
    // Ant-only tools
    ...(process.env.USER_TYPE === 'ant' ? [ConfigTool] : []),
    ...(process.env.USER_TYPE === 'ant' ? [TungstenTool] : []),
    
    // Feature-gated tools
    ...(SuggestBackgroundPRTool ? [SuggestBackgroundPRTool] : []),
    ...(WebBrowserTool ? [WebBrowserTool] : []),
    
    // Task management v2
    ...(isTodoV2Enabled()
      ? [TaskCreateTool, TaskGetTool, TaskUpdateTool, TaskListTool]
      : []),
    
    // Testing/debug tools
    ...(OverflowTestTool ? [OverflowTestTool] : []),
    ...(CtxInspectTool ? [CtxInspectTool] : []),
    ...(TerminalCaptureTool ? [TerminalCaptureTool] : []),
    
    // LSP (opt-in via env)
    ...(isEnvTruthy(process.env.ENABLE_LSP_TOOL) ? [LSPTool] : []),
    
    // Worktree mode
    ...(isWorktreeModeEnabled() ? [EnterWorktreeTool, ExitWorktreeTool] : []),
    
    // Lazy-loaded tools (circular deps)
    getSendMessageTool(),
    
    // Peer/list tools
    ...(ListPeersTool ? [ListPeersTool] : []),
    
    // Agent swarms
    ...(isAgentSwarmsEnabled()
      ? [getTeamCreateTool(), getTeamDeleteTool()]
      : []),
    
    // Plan verification (ant-only)
    ...(VerifyPlanExecutionTool ? [VerifyPlanExecutionTool] : []),
    
    // REPL (ant-only)
    ...(process.env.USER_TYPE === 'ant' && REPLTool ? [REPLTool] : []),
    
    // Workflow scripts
    ...(WorkflowTool ? [WorkflowTool] : []),
    
    // Sleep tool
    ...(SleepTool ? [SleepTool] : []),
    
    // Cron tools
    ...cronTools,
    
    // Remote trigger
    ...(RemoteTriggerTool ? [RemoteTriggerTool] : []),
    
    // Monitor tool
    ...(MonitorTool ? [MonitorTool] : []),
    
    BriefTool,
    
    // Kairos tools
    ...(SendUserFileTool ? [SendUserFileTool] : []),
    ...(PushNotificationTool ? [PushNotificationTool] : []),
    ...(SubscribePRTool ? [SubscribePRTool] : []),
    
    // PowerShell (opt-in)
    ...(getPowerShellTool() ? [getPowerShellTool()] : []),
    
    // History snip
    ...(SnipTool ? [SnipTool] : []),
    
    // Testing
    ...(process.env.NODE_ENV === 'test' ? [TestingPermissionTool] : []),
    
    // MCP resource tools
    ListMcpResourcesTool,
    ReadMcpResourceTool,
    
    // Tool search
    ...(isToolSearchEnabledOptimistic() ? [ToolSearchTool] : []),
  ]
}
```

**Note Comment:** Line 191 includes a critical note:
```typescript
/**
 * NOTE: This MUST stay in sync with 
 * https://console.statsig.com/.../claude_code_global_system_caching
 * in order to cache the system prompt across users.
 */
```

This means the tool order affects prompt caching - any change busts the cache and increases token costs 12x.

#### getTools (Lines 271-327)

```typescript
export const getTools = (permissionContext: ToolPermissionContext): Tools => {
  // Simple mode: only Bash, Read, Edit
  if (isEnvTruthy(process.env.CLAUDE_CODE_SIMPLE)) {
    if (isReplModeEnabled() && REPLTool) {
      const replSimple: Tool[] = [REPLTool]
      if (feature('COORDINATOR_MODE') && coordinatorModeModule?.isCoordinatorMode()) {
        replSimple.push(TaskStopTool, getSendMessageTool())
      }
      return filterToolsByDenyRules(replSimple, permissionContext)
    }
    
    const simpleTools: Tool[] = [BashTool, FileReadTool, FileEditTool]
    if (feature('COORDINATOR_MODE') && coordinatorModeModule?.isCoordinatorMode()) {
      simpleTools.push(AgentTool, TaskStopTool, getSendMessageTool())
    }
    return filterToolsByDenyRules(simpleTools, permissionContext)
  }

  // Get all base tools, exclude special tools added elsewhere
  const specialTools = new Set([
    ListMcpResourcesTool.name,
    ReadMcpResourceTool.name,
    SYNTHETIC_OUTPUT_TOOL_NAME,
  ])

  const tools = getAllBaseTools().filter(tool => !specialTools.has(tool.name))

  // Filter by deny rules
  let allowedTools = filterToolsByDenyRules(tools, permissionContext)

  // REPL mode: hide primitive tools
  if (isReplModeEnabled()) {
    const replEnabled = allowedTools.some(tool => toolMatchesName(tool, REPL_TOOL_NAME))
    if (replEnabled) {
      allowedTools = allowedTools.filter(tool => !REPL_ONLY_TOOLS.has(tool.name))
    }
  }

  // Filter by isEnabled()
  const isEnabled = allowedTools.map(_ => _.isEnabled())
  return allowedTools.filter((_, i) => isEnabled[i])
}
```

**Permission Filtering:** The `filterToolsByDenyRules()` function removes tools that match blanket deny rules in the permission context.

#### assembleToolPool (Lines 345-367)

```typescript
export function assembleToolPool(
  permissionContext: ToolPermissionContext,
  mcpTools: Tools,
): Tools {
  const builtInTools = getTools(permissionContext)

  // Filter MCP tools by deny rules
  const allowedMcpTools = filterToolsByDenyRules(mcpTools, permissionContext)

  // Sort for prompt-cache stability
  // Built-ins must stay contiguous for cache breakpoint
  const byName = (a: Tool, b: Tool) => a.name.localeCompare(b.name)
  return uniqBy(
    [...builtInTools].sort(byName).concat(allowedMcpTools.sort(byName)),
    'name',
  )
}
```

**Cache Stability Comment:**
```typescript
// The server's claude_code_system_cache_policy places a global cache
// breakpoint after the last prefix-matched built-in tool; a flat sort
// would interleave MCP tools into built-ins and invalidate all downstream
// cache keys whenever an MCP tool sorts between existing built-ins.
```

This is why built-ins and MCP tools are sorted separately then concatenated.

### 4.4 Tool.ts - Tool Base Class

#### Tool Interface Definition (Lines 362-695)

The Tool interface is the core abstraction for all tools. Key sections:

**Basic Properties:**
```typescript
export type Tool<Input, Output, P> = {
  readonly name: string
  aliases?: string[]
  searchHint?: string  // For ToolSearch keyword matching
  
  readonly inputSchema: Input  // Zod schema
  readonly inputJSONSchema?: ToolInputJSONSchema  // Direct JSON Schema (MCP)
  outputSchema?: z.ZodType<unknown>
  
  maxResultSizeChars: number  // When to persist results to disk
  
  readonly shouldDefer?: boolean  // Send with defer_loading: true
  readonly alwaysLoad?: boolean  // Never defer (model needs on turn 1)
  readonly strict?: boolean  // Enable strict mode
}
```

**Execution:**
```typescript
call(
  args: z.infer<Input>,
  context: ToolUseContext,
  canUseTool: CanUseToolFn,
  parentMessage: AssistantMessage,
  onProgress?: ToolCallProgress<P>
): Promise<ToolResult<Output>>

description(
  input: z.infer<Input>,
  options: {
    isNonInteractiveSession: boolean
    toolPermissionContext: ToolPermissionContext
    tools: Tools
  }
): Promise<string>
```

**Permission & Validation:**
```typescript
validateInput?(
  input: z.infer<Input>,
  context: ToolUseContext
): Promise<ValidationResult>

checkPermissions(
  input: z.infer<Input>,
  context: ToolUseContext
): Promise<PermissionResult>

preparePermissionMatcher?(
  input: z.infer<Input>
): Promise<(pattern: string) => boolean>
```

**Rendering:**
```typescript
renderToolUseMessage(
  input: Partial<z.infer<Input>>,
  options: { theme: ThemeName; verbose: boolean; commands?: Command[] }
): React.ReactNode

renderToolResultMessage?(
  content: Output,
  progressMessagesForMessage: ProgressMessage<P>[],
  options: { ... }
): React.ReactNode

renderToolUseProgressMessage?(
  progressMessagesForMessage: ProgressMessage<P>[],
  options: { ... }
): React.ReactNode
```

**UI Helpers:**
```typescript
userFacingName(input: Partial<z.infer<Input>> | undefined): string

getActivityDescription?(
  input: Partial<z.infer<Input>> | undefined
): string | null

getToolUseSummary?(
  input: Partial<z.infer<Input>> | undefined
): string | null

isSearchOrReadCommand?(input: z.infer<Input>): {
  isSearch: boolean
  isRead: boolean
  isList?: boolean
}
```

#### Tool Defaults (Lines 757-792)

```typescript
const TOOL_DEFAULTS = {
  isEnabled: () => true,
  isConcurrencySafe: (_input?: unknown) => false,
  isReadOnly: (_input?: unknown) => false,
  isDestructive: (_input?: unknown) => false,
  checkPermissions: (
    input: { [key: string]: unknown },
    _ctx?: ToolUseContext,
  ): Promise<PermissionResult> =>
    Promise.resolve({ behavior: 'allow', updatedInput: input }),
  toAutoClassifierInput: (_input?: unknown) => '',
  userFacingName: (_input?: unknown) => '',
}

export function buildTool<D extends AnyToolDef>(def: D): BuiltTool<D> {
  return {
    ...TOOL_DEFAULTS,
    userFacingName: () => def.name,
    ...def,
  } as BuiltTool<D>
}
```

**Pattern:** `buildTool()` is used by all tool implementations to avoid boilerplate. Defaults are "fail-closed" where it matters (e.g., `isConcurrencySafe: false`).

#### ToolUseContext (Lines 158-300)

This 142-line type defines the context passed to every tool call:

```typescript
export type ToolUseContext = {
  options: {
    commands: Command[]
    debug: boolean
    mainLoopModel: string
    tools: Tools
    verbose: boolean
    mcpClients: MCPServerConnection[]
    mcpResources: Record<string, ServerResource[]>
    isNonInteractiveSession: boolean
    agentDefinitions: AgentDefinitionsResult
    maxBudgetUsd?: number
    customSystemPrompt?: string
    appendSystemPrompt?: string
    querySource?: QuerySource
    refreshTools?: () => Tools
  }
  
  abortController: AbortController
  readFileState: FileStateCache
  getAppState(): AppState
  setAppState(f: (prev: AppState) => AppState): void
  setAppStateForTasks?: (f: (prev: AppState) => AppState) => void
  
  handleElicitation?: (
    serverName: string,
    params: ElicitRequestURLParams,
    signal: AbortSignal,
  ) => Promise<ElicitResult>
  
  setToolJSX?: SetToolJSXFn
  addNotification?: (notif: Notification) => void
  appendSystemMessage?: (msg: SystemMessage) => void
  sendOSNotification?: (opts: { message: string; notificationType: string }) => void
  
  // Memory attachment tracking
  nestedMemoryAttachmentTriggers?: Set<string>
  loadedNestedMemoryPaths?: Set<string>
  dynamicSkillDirTriggers?: Set<string>
  discoveredSkillNames?: Set<string>
  
  userModified?: boolean
  setInProgressToolUseIDs: (f: (prev: Set<string>) => Set<string>) => void
  setHasInterruptibleToolInProgress?: (v: boolean) => void
  setResponseLength: (f: (prev: number) => number) => void
  
  // API metrics (ant-only)
  pushApiMetricsEntry?: (ttftMs: number) => void
  
  setStreamMode?: (mode: SpinnerMode) => void
  onCompactProgress?: (event: CompactProgressEvent) => void
  setSDKStatus?: (status: SDKStatus) => void
  openMessageSelector?: () => void
  
  updateFileHistoryState: (updater: (prev: FileHistoryState) => FileHistoryState) => void
  updateAttributionState: (updater: (prev: AttributionState) => AttributionState) => void
  setConversationId?: (id: UUID) => void
  
  agentId?: AgentId
  agentType?: string
  requireCanUseTool?: boolean
  messages: Message[]
  fileReadingLimits?: { maxTokens?: number; maxSizeBytes?: number }
  globLimits?: { maxResults?: number }
  toolDecisions?: Map<string, { source: string; decision: 'accept' | 'reject'; timestamp: number }>
  queryTracking?: QueryChainTracking
  requestPrompt?: (sourceName: string) => (request: PromptRequest) => Promise<PromptResponse>
  toolUseId?: string
  criticalSystemReminder_EXPERIMENTAL?: string
  preserveToolUseResults?: boolean
  localDenialTracking?: DenialTrackingState
  contentReplacementState?: ContentReplacementState
  renderedSystemPrompt?: SystemPrompt
}
```

This is a comprehensive context object that gives tools access to:
- App state management
- File system caching
- Abort control
- UI rendering
- Notifications
- Memory/skill tracking
- Attribution state
- Query chain tracking

### 4.5 QueryEngine.ts - Query Engine Class

#### Class Structure (Lines 183-206)

```typescript
export class QueryEngine {
  private config: QueryEngineConfig
  private mutableMessages: Message[]
  private abortController: AbortController
  private permissionDenials: SDKPermissionDenial[]
  private totalUsage: NonNullableUsage
  private hasHandledOrphanedPermission = false
  private readFileState: FileStateCache
  private discoveredSkillNames = new Set<string>()
  private loadedNestedMemoryPaths = new Set<string>()

  constructor(config: QueryEngineConfig) {
    this.config = config
    this.mutableMessages = config.initialMessages ?? []
    this.abortController = config.abortController ?? createAbortController()
    this.permissionDenials = []
    this.readFileState = config.readFileCache
    this.totalUsage = EMPTY_USAGE
  }
}
```

**State Management:** The QueryEngine maintains conversation state across multiple `submitMessage()` calls within a single session.

#### submitMessage Generator (Lines 208-400+)

```typescript
async *submitMessage(
  prompt: string | ContentBlockParam[],
  options?: { uuid?: string; isMeta?: boolean }
): AsyncGenerator<SDKMessage, void, unknown> {
  // Extract config
  const { cwd, commands, tools, mcpClients, verbose, ... } = this.config
  
  // Clear turn-scoped tracking
  this.discoveredSkillNames.clear()
  setCwd(cwd)
  
  const persistSession = !isSessionPersistenceDisabled()
  const startTime = Date.now()
  
  // Wrap canUseTool to track permission denials
  const wrappedCanUseTool: CanUseToolFn = async (...) => {
    const result = await canUseTool(...)
    if (result.behavior !== 'allow') {
      this.permissionDenials.push({
        tool_name: sdkCompatToolName(tool.name),
        tool_use_id: toolUseID,
        tool_input: input,
      })
    }
    return result
  }
  
  // Get initial state
  const initialAppState = getAppState()
  const initialMainLoopModel = userSpecifiedModel
    ? parseUserSpecifiedModel(userSpecifiedModel)
    : getMainLoopModel()
  
  // Build system prompt
  const { defaultSystemPrompt, userContext, systemContext } = 
    await fetchSystemPromptParts({ ... })
  
  // Inject memory mechanics prompt if custom prompt + memory path override
  const memoryMechanicsPrompt = customPrompt !== undefined && hasAutoMemPathOverride()
    ? await loadMemoryPrompt()
    : null
  
  const systemPrompt = asSystemPrompt([
    ...(customPrompt !== undefined ? [customPrompt] : defaultSystemPrompt),
    ...(memoryMechanicsPrompt ? [memoryMechanicsPrompt] : []),
    ...(appendSystemPrompt ? [appendSystemPrompt] : []),
  ])
  
  // Register structured output hook
  if (jsonSchema && hasStructuredOutputTool) {
    registerStructuredOutputEnforcement(setAppState, getSessionId())
  }
  
  // Build processUserInputContext
  let processUserInputContext: ProcessUserInputContext = {
    messages: this.mutableMessages,
    setMessages: fn => {
      this.mutableMessages = fn(this.mutableMessages)
    },
    // ... 30+ more properties
  }
  
  // Handle orphaned permission (once per engine lifetime)
  if (orphanedPermission && !this.hasHandledOrphanedPermission) {
    this.hasHandledOrphanedPermission = true
    for await (const message of handleOrphanedPermission(...)) {
      yield message
    }
  }
  
  // Process user input (slash commands, attachments)
  const { messages: messagesFromUserInput, shouldQuery, allowedTools, ... } = 
    await processUserInput({
      input: prompt,
      mode: 'prompt',
      setToolJSX: () => {},
      context: { ...processUserInputContext, messages: this.mutableMessages },
      messages: this.mutableMessages,
      uuid: options?.uuid,
      isMeta: options?.isMeta,
      querySource: 'sdk',
    })
  
  // Push new messages
  this.mutableMessages.push(...messagesFromUserInput)
  
  // Persist to transcript BEFORE query loop
  if (persistSession && messagesFromUserInput.length > 0) {
    const transcriptPromise = recordTranscript(messages)
    if (isBareMode()) {
      void transcriptPromise  // Fire-and-forget
    } else {
      await transcriptPromise
      if (isEnvTruthy(process.env.CLAUDE_CODE_EAGER_FLUSH) ||
          isEnvTruthy(process.env.CLAUDE_CODE_IS_COWORK)) {
        await flushSessionStorage()
      }
    }
  }
  
  // Update permission context
  setAppState(prev => ({
    ...prev,
    toolPermissionContext: {
      ...prev.toolPermissionContext,
      alwaysAllowRules: {
        ...prev.toolPermissionContext.alwaysAllowRules,
        command: allowedTools,
      },
    },
  }))
  
  // Build system init message
  yield buildSystemInitMessage({
    tools, mcpClients, model: mainLoopModel,
    permissionMode: initialAppState.toolPermissionContext.mode,
    commands, agents, skills, plugins, fastMode: initialAppState.fastMode,
  })
  
  // If no query needed (local command output only)
  if (!shouldQuery) {
    // Yield command output messages
    for (const msg of messagesFromUserInput) {
      if (msg.type === 'user' && msg.message.content.includes(`<${LOCAL_COMMAND_STDOUT_TAG}>`)) {
        yield { type: 'user', message: { ...msg.message, content: stripAnsi(msg.message.content) }, ... }
      }
      // ... handle other message types
    }
    
    // Yield result
    yield {
      type: 'result',
      subtype: 'success',
      is_error: false,
      duration_ms: Date.now() - startTime,
      duration_api_ms: getTotalAPIDuration(),
      num_turns: messages.length - 1,
      result: resultText ?? '',
      stop_reason: null,
      total_cost_usd: getTotalCost(),
      usage: this.totalUsage,
      modelUsage: getModelUsage(),
      permission_denials: this.permissionDenials,
      fast_mode_state: getFastModeState(mainLoopModel, initialAppState.fastMode),
    }
    return
  }
  
  // File history snapshots
  if (fileHistoryEnabled() && persistSession) {
    messagesFromUserInput
      .filter(messageSelector().selectableUserMessagesFilter)
      .forEach(message => {
        void fileHistoryMakeSnapshot(
          (updater) => setAppState(prev => ({ ...prev, fileHistory: updater(prev.fileHistory) })),
          message.uuid,
        )
      })
  }
  
  // Query loop
  let currentMessageUsage: NonNullableUsage = EMPTY_USAGE
  let turnCount = 1
  let hasAcknowledgedInitialMessages = false
  const errorLogWatermark = getInMemoryErrors().at(-1)
  const initialStructuredOutputCalls = jsonSchema
    ? countToolCalls(this.mutableMessages, SYNTHETIC_OUTPUT_TOOL_NAME)
    : 0
  
  for await (const message of query({
    messages,
    systemPrompt,
    userContext,
    systemContext,
    canUseTool: wrappedCanUseTool,
    toolUseContext: processUserInputContext,
    fallbackModel,
    querySource: 'sdk',
    maxTurns,
    taskBudget,
  })) {
    // Process messages from query
    switch (message.type) {
      case 'assistant':
        this.mutableMessages.push(message)
        yield* normalizeMessage(message)
        break
      case 'user':
        this.mutableMessages.push(message)
        yield* normalizeMessage(message)
        break
      // ... handle other types
    }
  }
}
```

This is the core query flow:
1. Process user input and attachments
2. Build system prompt with optional memory mechanics
3. Handle orphaned permissions
4. Persist transcript before query
5. Call the core `query()` generator
6. Yield normalized messages to SDK caller
7. Track usage and permission denials

### 4.6 query.ts - Core Query Loop

#### Query Loop State (Lines 200-217)

```typescript
type State = {
  messages: Message[]
  toolUseContext: ToolUseContext
  autoCompactTracking: AutoCompactTrackingState | undefined
  maxOutputTokensRecoveryCount: number
  hasAttemptedReactiveCompact: boolean
  maxOutputTokensOverride: number | undefined
  pendingToolUseSummary: Promise<ToolUseSummaryMessage | null> | undefined
  stopHookActive: boolean | undefined
  turnCount: number
  transition: Continue | undefined  // Why previous iteration continued
}
```

This state is carried between loop iterations and updated at continue sites.

#### Query Loop Entry (Lines 240-380)

```typescript
async function* queryLoop(
  params: QueryParams,
  consumedCommandUuids: string[],
): AsyncGenerator<...> {
  // Extract immutable params
  const { systemPrompt, userContext, systemContext, canUseTool, ... } = params
  const deps = params.deps ?? productionDeps()

  // Initialize state
  let state: State = {
    messages: params.messages,
    toolUseContext: params.toolUseContext,
    maxOutputTokensOverride: params.maxOutputTokensOverride,
    autoCompactTracking: undefined,
    maxOutputTokensRecoveryCount: 0,
    hasAttemptedReactiveCompact: false,
    turnCount: 1,
    pendingToolUseSummary: undefined,
    stopHookActive: undefined,
    transition: undefined,
  }
  
  const budgetTracker = feature('TOKEN_BUDGET') ? createBudgetTracker() : null
  let taskBudgetRemaining: number | undefined = undefined
  
  // Build query config (env/statsig/session state)
  const config = buildQueryConfig()
  
  // Start memory prefetch (fire-and-forget)
  using pendingMemoryPrefetch = startRelevantMemoryPrefetch(
    state.messages,
    state.toolUseContext,
  )
  
  // Main query loop
  while (true) {
    // Destructure state
    let { toolUseContext } = state
    const { messages, autoCompactTracking, maxOutputTokensRecoveryCount, ... } = state
    
    // Skill discovery prefetch
    const pendingSkillPrefetch = skillPrefetch?.startSkillDiscoveryPrefetch(...)
    
    yield { type: 'stream_request_start' }
    
    // Initialize query chain tracking
    const queryTracking = toolUseContext.queryTracking
      ? { chainId: ..., depth: toolUseContext.queryTracking.depth + 1 }
      : { chainId: deps.uuid(), depth: 0 }
    
    toolUseContext = { ...toolUseContext, queryTracking }
    
    // Get messages after compact boundary
    let messagesForQuery = [...getMessagesAfterCompactBoundary(messages)]
    let tracking = autoCompactTracking
    
    // Apply tool result budget (persist replacements)
    const persistReplacements = querySource.startsWith('agent:') || 
                                   querySource.startsWith('repl_main_thread')
    messagesForQuery = await applyToolResultBudget(
      messagesForQuery,
      toolUseContext.contentReplacementState,
      persistReplacements ? records => recordContentReplacement(...) : undefined,
      new Set(tools.filter(t => !Number.isFinite(t.maxResultSizeChars)).map(t => t.name)),
    )
    
    // Apply snip (feature-gated)
    let snipTokensFreed = 0
    if (feature('HISTORY_SNIP')) {
      const snipResult = snipModule!.snipCompactIfNeeded(messagesForQuery)
      messagesForQuery = snipResult.messages
      snipTokensFreed = snipResult.tokensFreed
      if (snipResult.boundaryMessage) {
        yield snipResult.boundaryMessage
      }
    }
    
    // Apply microcompact
    const microcompactResult = await deps.microcompact(
      messagesForQuery,
      toolUseContext,
      querySource,
    )
    messagesForQuery = microcompactResult.messages
    
    // Apply context collapse (feature-gated)
    if (feature('CONTEXT_COLLAPSE') && contextCollapse) {
      const collapseResult = await contextCollapse.applyCollapsesIfNeeded(...)
      messagesForQuery = collapseResult.messages
    }
    
    // Apply autocompact
    const { compactionResult, consecutiveFailures } = await deps.autocompact(
      messagesForQuery,
      toolUseContext,
      { systemPrompt, userContext, systemContext, toolUseContext, ... },
      querySource,
      tracking,
      snipTokensFreed,
    )
    
    // Handle compaction result
    if (compactionResult) {
      // Update task_budget remaining
      if (params.taskBudget) {
        const preCompactContext = finalContextTokensFromLastResponse(messagesForQuery)
        taskBudgetRemaining = Math.max(0, 
          (taskBudgetRemaining ?? params.taskBudget.total) - preCompactContext
        )
      }
      
      // Reset tracking
      tracking = { compacted: true, turnId: deps.uuid(), turnCounter: 0, consecutiveFailures: 0 }
      
      // Build and yield post-compact messages
      const postCompactMessages = buildPostCompactMessages(compactionResult)
      for (const message of postCompactMessages) {
        yield message
      }
      
      messagesForQuery = postCompactMessages
    } else if (consecutiveFailures !== undefined) {
      tracking = { ...(tracking ?? { compacted: false, turnId: '', turnCounter: 0 }), consecutiveFailures }
    }
    
    toolUseContext = { ...toolUseContext, messages: messagesForQuery }
    
    // ... rest of loop
  }
}
```

#### Context Management Pipeline

The query loop applies context management in this order:
1. **Tool Result Budget:** Persists large tool results to disk, replaces with file references
2. **Snip:** Removes old history while preserving context (feature-gated)
3. **Microcompact:** Caches editing for context reduction
4. **Context Collapse:** Projects collapsed view, commits staged collapses (feature-gated)
5. **Autocompact:** Automatic compaction when token thresholds exceeded

This ordering ensures each layer operates on the output of the previous layer.

### 4.7 history.ts - History Management

#### Core Data Structures (Lines 19-75)

```typescript
const MAX_HISTORY_ITEMS = 100
const MAX_PASTED_CONTENT_LENGTH = 1024

type StoredPastedContent = {
  id: number
  type: 'text' | 'image'
  content?: string        // Inline for small pastes
  contentHash?: string    // Hash reference for large pastes
  mediaType?: string
  filename?: string
}

type LogEntry = {
  display: string
  pastedContents: Record<number, StoredPastedContent>
  timestamp: number
  project: string
  sessionId?: string
}
```

**Storage Strategy:** Small text (<1KB) stored inline, large text stored by hash reference in paste store.

#### History Reader (Lines 106-149)

```typescript
async function* makeLogEntryReader(): AsyncGenerator<LogEntry> {
  const currentSession = getSessionId()

  // Start with pending entries (not yet flushed to disk)
  for (let i = pendingEntries.length - 1; i >= 0; i--) {
    yield pendingEntries[i]!
  }

  // Read from global history file (newest first)
  const historyPath = join(getClaudeConfigHomeDir(), 'history.jsonl')

  try {
    for await (const line of readLinesReverse(historyPath)) {
      try {
        const entry = deserializeLogEntry(line)
        
        // Skip entries that were flushed before removal
        if (entry.sessionId === currentSession && 
            skippedTimestamps.has(entry.timestamp)) {
          continue
        }
        yield entry
      } catch (error) {
        logForDebugging(`Failed to parse history line: ${error}`)
      }
    }
  } catch (e: unknown) {
    const code = getErrnoCode(e)
    if (code === 'ENOENT') {
      return  // File doesn't exist yet
    }
    throw e
  }
}

export async function* makeHistoryReader(): AsyncGenerator<HistoryEntry> {
  for await (const entry of makeLogEntryReader()) {
    yield await logEntryToHistoryEntry(entry)
  }
}
```

**Design Notes:**
1. Pending entries yielded first (most recent)
2. File read in reverse order (newest first)
3. Skipped timestamps handle race between flush and remove
4. Malformed lines logged but not fatal

#### Add to History (Lines 355-434)

```typescript
async function addToPromptHistory(command: HistoryEntry | string): Promise<void> {
  const entry = typeof command === 'string'
    ? { display: command, pastedContents: {} }
    : command

  const storedPastedContents: Record<number, StoredPastedContent> = {}
  
  if (entry.pastedContents) {
    for (const [id, content] of Object.entries(entry.pastedContents)) {
      // Filter out images (stored separately)
      if (content.type === 'image') continue

      // Small content stored inline
      if (content.content.length <= MAX_PASTED_CONTENT_LENGTH) {
        storedPastedContents[Number(id)] = {
          id: content.id,
          type: content.type,
          content: content.content,
          mediaType: content.mediaType,
          filename: content.filename,
        }
      } else {
        // Large content: hash and store reference
        const hash = hashPastedText(content.content)
        storedPastedContents[Number(id)] = {
          id: content.id,
          type: content.type,
          contentHash: hash,
          mediaType: content.mediaType,
          filename: content.filename,
        }
        // Fire-and-forget disk write
        void storePastedText(hash, content.content)
      }
    }
  }

  const logEntry: LogEntry = {
    ...entry,
    pastedContents: storedPastedContents,
    timestamp: Date.now(),
    project: getProjectRoot(),
    sessionId: getSessionId(),
  }

  pendingEntries.push(logEntry)
  lastAddedEntry = logEntry
  currentFlushPromise = flushPromptHistory(0)
  void currentFlushPromise
}

export function addToHistory(command: HistoryEntry | string): void {
  // Skip in tmux sessions spawned by Claude Code
  if (isEnvTruthy(process.env.CLAUDE_CODE_SKIP_PROMPT_HISTORY)) {
    return
  }

  // Register cleanup on first use
  if (!cleanupRegistered) {
    cleanupRegistered = true
    registerCleanup(async () => {
      if (currentFlushPromise) {
        await currentFlushPromise
      }
      if (pendingEntries.length > 0) {
        await immediateFlushHistory()
      }
    })
  }

  void addToPromptHistory(command)
}
```

**Async Flush Pattern:** History is buffered and flushed asynchronously to avoid blocking the UI.

### 4.8 cost-tracker.ts - Cost Tracking

#### Cost Storage (Lines 71-123)

```typescript
type StoredCostState = {
  totalCostUSD: number
  totalAPIDuration: number
  totalAPIDurationWithoutRetries: number
  totalToolDuration: number
  totalLinesAdded: number
  totalLinesRemoved: number
  lastDuration: number | undefined
  modelUsage: { [modelName: string]: ModelUsage } | undefined
}

export function getStoredSessionCosts(sessionId: string): StoredCostState | undefined {
  const projectConfig = getCurrentProjectConfig()

  // Only return costs if this is the same session
  if (projectConfig.lastSessionId !== sessionId) {
    return undefined
  }

  // Build model usage with context windows
  let modelUsage: { [modelName: string]: ModelUsage } | undefined
  if (projectConfig.lastModelUsage) {
    modelUsage = Object.fromEntries(
      Object.entries(projectConfig.lastModelUsage).map(([model, usage]) => [
        model,
        {
          ...usage,
          contextWindow: getContextWindowForModel(model, getSdkBetas()),
          maxOutputTokens: getModelMaxOutputTokens(model).default,
        },
      ])
    )
  }

  return {
    totalCostUSD: projectConfig.lastCost ?? 0,
    totalAPIDuration: projectConfig.lastAPIDuration ?? 0,
    totalAPIDurationWithoutRetries: projectConfig.lastAPIDurationWithoutRetries ?? 0,
    totalToolDuration: projectConfig.lastToolDuration ?? 0,
    totalLinesAdded: projectConfig.lastLinesAdded ?? 0,
    totalLinesRemoved: projectConfig.lastLinesRemoved ?? 0,
    lastDuration: projectConfig.lastDuration,
    modelUsage,
  }
}
```

**Session Matching:** Costs are only restored if the session ID matches, preventing cost leakage between sessions.

#### Add to Cost (Lines 278-323)

```typescript
export function addToTotalSessionCost(
  cost: number,
  usage: Usage,
  model: string,
): number {
  const modelUsage = addToTotalModelUsage(cost, usage, model)
  addToTotalCostState(cost, modelUsage, model)

  // Track cost and tokens in Statsig
  const attrs = isFastModeEnabled() && usage.speed === 'fast'
    ? { model, speed: 'fast' }
    : { model }

  getCostCounter()?.add(cost, attrs)
  getTokenCounter()?.add(usage.input_tokens, { ...attrs, type: 'input' })
  getTokenCounter()?.add(usage.output_tokens, { ...attrs, type: 'output' })
  getTokenCounter()?.add(usage.cache_read_input_tokens ?? 0, { ...attrs, type: 'cacheRead' })
  getTokenCounter()?.add(usage.cache_creation_input_tokens ?? 0, { ...attrs, type: 'cacheCreation' })

  let totalCost = cost
  
  // Handle advisor tool token usage
  for (const advisorUsage of getAdvisorUsage(usage)) {
    const advisorCost = calculateUSDCost(advisorUsage.model, advisorUsage)
    logEvent('tengu_advisor_tool_token_usage', {
      advisor_model: advisorUsage.model,
      input_tokens: advisorUsage.input_tokens,
      output_tokens: advisorUsage.output_tokens,
      cache_read_input_tokens: advisorUsage.cache_read_input_tokens ?? 0,
      cache_creation_input_tokens: advisorUsage.cache_creation_input_tokens ?? 0,
      cost_usd_micros: Math.round(advisorCost * 1_000_000),
    })
    totalCost += addToTotalSessionCost(advisorCost, advisorUsage, advisorUsage.model)
  }
  
  return totalCost
}
```

**Advisor Recursion:** Advisor tool calls are tracked recursively, adding their costs to the total.

### 4.9 setup.ts - Session Setup

#### Main Setup Function (Lines 56-200)

```typescript
export async function setup(
  cwd: string,
  permissionMode: PermissionMode,
  allowDangerouslySkipPermissions: boolean,
  worktreeEnabled: boolean,
  worktreeName: string | undefined,
  tmuxEnabled: boolean,
  customSessionId?: string | null,
  worktreePRNumber?: number,
  messagingSocketPath?: string,
): Promise<void> {
  logForDiagnosticsNoPII('info', 'setup_started')

  // Node.js version check
  const nodeVersion = process.version.match(/^v(\d+)\./)?.[1]
  if (!nodeVersion || parseInt(nodeVersion) < 18) {
    console.error(chalk.bold.red('Error: Claude Code requires Node.js version 18 or higher.'))
    process.exit(1)
  }

  // Set custom session ID if provided
  if (customSessionId) {
    switchSession(asSessionId(customSessionId))
  }

  // UDS messaging server (non-bare mode)
  if (!isBareMode() || messagingSocketPath !== undefined) {
    if (feature('UDS_INBOX')) {
      const m = await import('./utils/udsMessaging.js')
      await m.startUdsMessaging(
        messagingSocketPath ?? m.getDefaultUdsSocketPath(),
        { isExplicit: messagingSocketPath !== undefined }
      )
    }
  }

  // Teammate mode snapshot
  if (!isBareMode() && isAgentSwarmsEnabled()) {
    const { captureTeammateModeSnapshot } = await import('./utils/swarm/backends/teammateModeSnapshot.js')
    captureTeammateModeSnapshot()
  }

  // Terminal backup restoration (interactive only)
  if (!getIsNonInteractiveSession()) {
    if (isAgentSwarmsEnabled()) {
      const restoredIterm2Backup = await checkAndRestoreITerm2Backup()
      if (restoredIterm2Backup.status === 'restored') {
        console.log(chalk.yellow('Detected an interrupted iTerm2 setup. Your original settings have been restored.'))
      }
    }

    try {
      const restoredTerminalBackup = await checkAndRestoreTerminalBackup()
      if (restoredTerminalBackup.status === 'restored') {
        console.log(chalk.yellow('Detected an interrupted Terminal.app setup. Your original settings have been restored.'))
      }
    } catch (error) {
      logError(error)
    }
  }

  // Set cwd (MUST be before any cwd-dependent code)
  setCwd(cwd)

  // Capture hooks configuration snapshot
  const hooksStart = Date.now()
  captureHooksConfigSnapshot()
  logForDiagnosticsNoPII('info', 'setup_hooks_captured', { duration_ms: Date.now() - hooksStart })

  // Initialize FileChanged hook watcher
  initializeFileChangedWatcher(cwd)
  
  // ... rest of setup
}
```

#### Worktree Creation (Lines 174-285)

```typescript
if (worktreeEnabled) {
  const hasHook = hasWorktreeCreateHook()
  const inGit = await getIsGit()
  
  if (!hasHook && !inGit) {
    process.stderr.write(chalk.red(
      `Error: Can only use --worktree in a git repository, but ${chalk.bold(cwd)} is not a git repository.`
    ))
    process.exit(1)
  }

  const slug = worktreePRNumber
    ? `pr-${worktreePRNumber}`
    : (worktreeName ?? getPlanSlug())

  let tmuxSessionName: string | undefined
  if (inGit) {
    const mainRepoRoot = findCanonicalGitRoot(getCwd())
    if (!mainRepoRoot) {
      process.stderr.write(chalk.red('Error: Could not determine the main git repository root.\n'))
      process.exit(1)
    }

    // If inside a worktree, switch to main repo for creation
    if (mainRepoRoot !== (findGitRoot(getCwd()) ?? getCwd())) {
      logForDiagnosticsNoPII('info', 'worktree_resolved_to_main_repo')
      process.chdir(mainRepoRoot)
      setCwd(mainRepoRoot)
    }

    tmuxSessionName = tmuxEnabled
      ? generateTmuxSessionName(mainRepoRoot, worktreeBranchName(slug))
      : undefined
  } else {
    tmuxSessionName = tmuxEnabled
      ? generateTmuxSessionName(getCwd(), worktreeBranchName(slug))
      : undefined
  }

  let worktreeSession = await createWorktreeForSession(
    getSessionId(),
    slug,
    tmuxSessionName,
    worktreePRNumber ? { prNumber: worktreePRNumber } : undefined,
  )

  // Create tmux session if enabled
  if (tmuxEnabled && tmuxSessionName) {
    const tmuxResult = await createTmuxSessionForWorktree(tmuxSessionName, worktreeSession.worktreePath)
    if (tmuxResult.created) {
      console.log(chalk.green(
        `Created tmux session: ${chalk.bold(tmuxSessionName)}\n` +
        `To attach: ${chalk.bold(`tmux attach -t ${tmuxSessionName}`)}`
      ))
    } else {
      console.error(chalk.yellow(`Warning: Failed to create tmux session: ${tmuxResult.error}`))
    }
  }

  // Switch to worktree
  process.chdir(worktreeSession.worktreePath)
  setCwd(worktreeSession.worktreePath)
  setOriginalCwd(getCwd())
  setProjectRoot(getCwd())
  saveWorktreeState(worktreeSession)
  clearMemoryFileCaches()
  updateHooksConfigSnapshot()
}
```

**Worktree Flow:**
1. Validate git repo or hook
2. Resolve main repo root (handles nested worktrees)
3. Create worktree at slug path
4. Optionally create tmux session
5. Switch cwd to worktree
6. Clear caches, update hooks

---

## 5. Integration Points

### 5.1 Application Startup Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           main.tsx Entry                                │
│  - Module evaluation with parallel prefetches                           │
│  - Debug detection (exit if debugging external build)                   │
│  - CLI argument parsing with Commander.js                               │
│  - preAction hook fires before any command                              │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         setup.ts                                        │
│  - Node.js version check                                                │
│  - UDS messaging server startup                                         │
│  - Terminal backup restoration                                          │
│  - Worktree creation (if --worktree)                                    │
│  - Hooks snapshot and FileChanged watcher init                          │
│  - Background jobs (context collapse, version lock, etc.)               │
│  - Prefetch: API key, commands, plugin hooks                            │
│  - Release notes check                                                  │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    REPL Launch (main.tsx)                               │
│  - Initialize AppState store                                            │
│  - Load tools, commands, skills, plugins                                │
│  - Build system prompt                                                  │
│  - Render Ink UI                                                        │
│  - startDeferredPrefetches() called after first render                  │
└─────────────────────────────────────────────────────────────────────────┘
```

### 5.2 Query Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    QueryEngine.submitMessage()                          │
│  - Process user input (slash commands, attachments)                     │
│  - Build system prompt with memory mechanics                            │
│  - Persist transcript before query                                      │
│  - Yield system init message                                            │
│  - Call query() generator                                               │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         query.ts Loop                                   │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │ Iteration Setup                                                    │  │
│  │ - Apply tool result budget                                        │  │
│  │ - Apply snip (feature-gated)                                      │  │
│  │ - Apply microcompact                                              │  │
│  │ - Apply context collapse (feature-gated)                          │  │
│  │ - Apply autocompact                                               │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                                    │                                     │
│                                    ▼                                     │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │ API Call with Streaming                                            │  │
│  │ - callModel() with streaming                                      │  │
│  │ - Yield stream events                                             │  │
│  │ - Handle fallback on errors                                       │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                                    │                                     │
│                                    ▼                                     │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │ Tool Execution                                                     │  │
│  │ - StreamingToolExecutor processes tool_use blocks                 │  │
│  │ - runTools() orchestrates parallel execution                      │  │
│  │ - Permission checks via canUseTool                                │  │
│  │ - Yield progress messages                                         │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                                    │                                     │
│                                    ▼                                     │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │ Continue Decision                                                  │  │
│  │ - Tool uses present? → continue for results                       │  │
│  │ - Max output tokens error? → recovery paths                       │  │
│  │ - Stop hook active? → optional retry                              │  │
│  │ - Otherwise → return (terminal)                                   │  │
│  └───────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

### 5.3 Tool Execution Pipeline

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    StreamingToolExecutor                                │
│  - Receives tool_use blocks from API stream                            │
│  - Validates input against Zod schema                                  │
│  - Calls canUseTool() for permission decision                          │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    Permission Check (permissions.ts)                    │
│  Step 1: Check blanket deny rules                                      │
│  Step 2: Run tool-specific checkPermissions()                          │
│  Step 3: Run classification hook (auto-mode)                           │
│  Step 4: Run PreToolUse hooks                                          │
│  Step 5: Show permission dialog (if needed)                            │
│  Step 6: Run always-allow rule check                                   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    Tool.call()                                          │
│  - Execute tool-specific logic                                         │
│  - Report progress via onProgress callback                             │
│  - Return ToolResult with data and optional newMessages                │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    Result Processing                                    │
│  - Apply result size budget (persist if too large)                     │
│  - Create tool_result block for API                                    │
│  - Render result message for UI                                        │
│  - Update file history (for file operations)                           │
│  - Update attribution state (for git operations)                       │
└─────────────────────────────────────────────────────────────────────────┘
```

### 5.4 Command Processing Pipeline

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    processUserInput()                                   │
│  - Parse input for slash commands                                      │
│  - Process attachments (files, images, pasted text)                    │
│  - Expand pasted text references                                       │
│  - Load skill/plugin commands                                          │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    Command Execution                                    │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │ 'prompt' type: Expand to text prompt                              │  │
│  │ - Read skill/command markdown                                     │  │
│  │ - Substitute placeholders                                           │  │
│  │ - Return expanded prompt                                          │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │ 'local' type: Execute locally                                     │  │
│  │ - Run shell command                                               │  │
│  │ - Capture stdout/stderr                                           │  │
│  │ - Return output as message                                        │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │ 'local-jsx' type: Render Ink UI                                   │  │
│  │ - Mount component                                                 │  │
│  │ - Handle user interaction                                         │  │
│  │ - Return result on completion                                     │  │
│  └───────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

### 5.5 Context Management Hierarchy

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Message Store (AppState)                             │
│  - Full conversation history                                           │
│  - All message types (user, assistant, system, progress)               │
│  - Preserved across turns                                              │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    Compact Boundary                                     │
│  - Messages before boundary are summarized                             │
│  - Boundary moves forward on compaction                                │
│  - Preserved segment for link restoration                              │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    Snip (feature-gated)                                 │
│  - Removes oldest messages beyond token threshold                      │
│  - Projects snipped view for UI                                        │
│  - Yields boundary message on first snip                               │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    Microcompact                                         │
│  - Cached context editing                                              │
│  - Edits messages in place to reduce tokens                            │
│  - Operates on cached messages when possible                           │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    Context Collapse (feature-gated)                     │
│  - Stages granular collapses (multi-turn summaries)                    │
│  - Projects collapsed view on read                                     │
│  - Commits collapses on overflow                                       │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    Autocompact                                          │
│  - Triggers when token threshold exceeded                              │
│  - Circuit breaker on consecutive failures                             │
│  - Creates summary + attachments for preserved context                 │
│  - Runs hooks (pre_compact, post_compact)                              │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    API Request                                          │
│  - Final context window sent to model                                  │
│  - Includes usage tracking for token counting                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 5.6 Permission System Integration

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    ToolPermissionContext                                │
│  - mode: 'default' | 'auto' | 'plan' | 'bypassPermissions'             │
│  - alwaysAllowRules: { source: rules[] }                               │
│  - alwaysDenyRules: { source: rules[] }                                │
│  - alwaysAskRules: { source: rules[] }                                 │
│  - additionalWorkingDirectories: Map<path, metadata>                   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    getTools(permissionContext)                          │
│  - Filters tools by blanket deny rules                                 │
│  - Removes tools that match deny patterns                              │
│  - Returns filtered tool list to model                                 │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    canUseTool() at call time                            │
│  Step 1: Check blanket deny (tool name match)                          │
│  Step 2: Check tool-specific checkPermissions()                        │
│  Step 3: Auto-mode classification (feature-gated)                      │
│  Step 4: PreToolUse hooks                                              │
│  Step 5: Permission dialog (if no auto decision)                       │
│  Step 6: Apply always-allow rules                                      │
│  Step 7: Track denial if denied                                        │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Appendix A: Feature Flags

These files use extensive feature flagging via `bun:bundle`'s `feature()` function:

| Feature Flag | Files Using | Purpose |
|-------------|-------------|---------|
| `PROACTIVE` | main.tsx, commands.ts, tools.ts | Proactive mode features |
| `KAIROS` | main.tsx, commands.ts, tools.ts | Assistant mode features |
| `COORDINATOR_MODE` | main.tsx, commands.ts, tools.ts | Multi-agent coordination |
| `BRIDGE_MODE` | main.tsx, commands.ts | Mobile/web bridge |
| `WORKFLOW_SCRIPTS` | main.tsx, commands.ts, tools.ts | Workflow scripting |
| `UDS_INBOX` | main.tsx, commands.ts, setup.ts | Unix domain socket messaging |
| `CONTEXT_COLLAPSE` | main.tsx, query.ts, QueryEngine.ts | Context collapse feature |
| `HISTORY_SNIP` | main.tsx, query.ts, QueryEngine.ts | History snipping |
| `MONITOR_TOOL` | tools.ts | Monitor MCP tool |
| `AGENT_TRIGGERS` | tools.ts | Cron/trigger tools |
| `TRANSCRIPT_CLASSIFIER` | main.tsx, query.ts | Auto-mode classification |
| `TOKEN_BUDGET` | query.ts | Token budget enforcement |
| `DIRECT_CONNECT` | main.tsx | Direct connect feature |
| `SSH_REMOTE` | main.tsx | SSH remote sessions |
| `LODESTONE` | main.tsx | Deep link handling |

---

## Appendix B: File Dependencies

```
main.tsx
├── commands.ts (getCommands, builtInCommandNames)
├── tools.ts (getTools, getAllBaseTools)
├── Tool.ts (Tool, Tools, ToolUseContext types)
├── QueryEngine.ts (QueryEngine class)
├── query.ts (query generator)
├── setup.ts (setup function)
├── history.ts (addToHistory)
├── cost-tracker.ts (getTotalCost, formatTotalCost)
├── dialogLaunchers.tsx (dialog launchers)
├── interactiveHelpers.tsx (renderAndRun, exit handling)
└── Task.ts (Task types)

commands.ts
├── tools.ts (getSkillToolCommands, getSlashCommandToolSkills)
└── Tool.ts (Command type)

tools.ts
├── Tool.ts (Tool, Tools, ToolPermissionContext, buildTool)
└── Individual tool modules (BashTool, FileReadTool, etc.)

Tool.ts
└── (Core type definitions - no internal dependencies)

QueryEngine.ts
├── query.ts (query generator)
├── Tool.ts (ToolUseContext, Tools)
└── cost-tracker.ts (getTotalCost, getModelUsage)

query.ts
├── Tool.ts (ToolUseContext, findToolByName)
├── cost-tracker.ts (getTotalAPIDuration)
└── Various service modules (compact, contextCollapse, etc.)
```

---

## Appendix C: Key Design Patterns

### C.1 Feature Flag Gating

```typescript
// Dead code elimination pattern - strings eliminated from external builds
const proactive = feature('PROACTIVE') || feature('KAIROS')
  ? require('./commands/proactive.js').default
  : null

// Usage with spread for arrays
const tools = [
  BaseTool,
  ...(feature('FEATURE') ? [FeatureTool] : []),
]
```

### C.2 Lazy Require for Circular Dependencies

```typescript
// Lazy require to break circular dependency
const getTeamCreateTool = () =>
  require('./tools/TeamCreateTool/TeamCreateTool.js').TeamCreateTool
```

### C.3 Memoization for Expensive Operations

```typescript
const COMMANDS = memoize((): Command[] => [
  // ... commands
])

const getSkillToolCommands = memoize(async (cwd: string): Promise<Command[]> => {
  // ... loading
})
```

### C.4 Async Generators for Streaming

```typescript
export async function* query(params: QueryParams): AsyncGenerator<...> {
  yield { type: 'stream_request_start' }
  
  for await (const message of callModel(...)) {
    yield message
  }
  
  return { reason: 'complete' }
}
```

### C.5 Continuation-Passing Style for State Updates

```typescript
setAppState(prev => ({
  ...prev,
  toolPermissionContext: {
    ...prev.toolPermissionContext,
    alwaysAllowRules: {
      ...prev.toolPermissionContext.alwaysAllowRules,
      command: allowedTools,
    },
  },
}))
```

---

**Document Statistics:**
- Total files analyzed: 12
- Total lines of code: ~11,547
- Total exports: ~150
- Total integration points documented: 6
