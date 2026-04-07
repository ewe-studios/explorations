# Types Module - Deep Dive Exploration

**Source Directory:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/types/`

**Files:** 11 files total (7 core type files + 4 generated protobuf types)

---

## Table of Contents

1. [File Inventory](#file-inventory)
2. [Module Overview](#module-overview)
3. [Key Exports and Type Signatures](#key-exports-and-type-signatures)
4. [Line-by-Line Analysis](#line-by-line-analysis)
5. [Integration Points](#integration-points)

---

## File Inventory

### Core Type Files

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `command.ts` | ~217 | `Command`, `PromptCommand`, `LocalCommand`, `LocalJSXCommand`, `CommandBase`, `CommandAvailability` | Command system types for slash commands, skills, and local/JSX commands |
| `hooks.ts` | ~291 | `HookCallback`, `HookResult`, `HookJSONOutput`, `PromptRequest`, `PermissionRequestResult`, hook schemas | Hook system types for pre/post event callbacks with Zod schemas |
| `ids.ts` | ~45 | `SessionId`, `AgentId`, `asSessionId`, `asAgentId`, `toAgentId` | Branded types for session and agent IDs |
| `logs.ts` | ~331 | `LogOption`, `Entry`, `TranscriptMessage`, `SerializedMessage`, `AttributionSnapshotMessage`, `ContextCollapseCommitEntry` | Session logging, transcript entries, and attribution tracking |
| `permissions.ts` | ~442 | `PermissionResult`, `PermissionDecision`, `PermissionRule`, `PermissionMode`, `ToolPermissionContext`, `YoloClassifierResult` | Permission system types for tool approval/denial |
| `plugin.ts` | ~364 | `PluginManifest`, `LoadedPlugin`, `PluginError`, `PluginLoadResult`, `BuiltinPluginDefinition` | Plugin system types with 25 error types |
| `textInputTypes.ts` | ~388 | `BaseTextInputProps`, `VimTextInputProps`, `QueuedCommand`, `PromptInputMode`, `QueuePriority`, `OrphanedPermission` | Text input component props and command queue types |

### Generated Protobuf Types

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `generated/events_mono/claude_code/v1/claude_code_internal_event.ts` | ~866 | `ClaudeCodeInternalEvent`, `EnvironmentMetadata`, `GitHubActionsMetadata`, `SlackContext` | Analytics event schema for Statsig logging |
| `generated/events_mono/common/v1/auth.ts` | ~101 | `PublicApiAuth` | Authentication context for API events |
| `generated/events_mono/growthbook/v1/growthbook_experiment_event.ts` | ~224 | `GrowthbookExperimentEvent` | A/B testing experiment assignment events |
| `generated/google/protobuf/timestamp.ts` | ~188 | `Timestamp` | Standard protobuf timestamp type |

**Total Lines:** ~3,557 lines (core: ~2,078, generated: ~1,479)

---

## Module Overview

### Purpose and Responsibilities

The `types/` module serves as the **type definition backbone** for the entire Claude Code application. It provides:

1. **Domain model definitions**: Commands, hooks, permissions, plugins, sessions
2. **API contracts**: Protobuf-generated types for analytics and events
3. **Component interfaces**: TextInput props, command queue structures
4. **Type safety**: Branded types to prevent ID mixing at compile time
5. **Shared vocabulary**: Consistent type definitions used across all modules

### Architectural Patterns

The module demonstrates several key patterns:

1. **Discriminated Unions**: Most types use discriminators (`type` field) for type narrowing
2. **Branded Types**: `SessionId` and `AgentId` use nominal typing to prevent mixing
3. **Zod Schema Validation**: Runtime validation paired with TypeScript types
4. **Protocol Buffers**: Generated types for cross-language event schemas
5. **Generic Constraints**: Reusable type utilities with proper variance

---

## Key Exports and Type Signatures

### command.ts - Command System

```typescript
// Command availability - who can use this command
export type CommandAvailability = 'claude-ai' | 'console';

// Prompt command - returns content blocks for the model
export type PromptCommand = {
  type: 'prompt';
  progressMessage: string;
  contentLength: number;  // For token estimation
  argNames?: string[];
  allowedTools?: string[];
  model?: string;
  source: SettingSource | 'builtin' | 'mcp' | 'plugin' | 'bundled';
  pluginInfo?: {
    pluginManifest: PluginManifest;
    repository: string;
  };
  context?: 'inline' | 'fork';  // Execution context
  agent?: string;  // Agent type for forked execution
  effort?: EffortValue;
  paths?: string[];  // Glob patterns for file matching
  getPromptForCommand(args: string, context: ToolUseContext): Promise<ContentBlockParam[]>;
};

// Local command - programmatic implementation
export type LocalCommandCall = (
  args: string,
  context: LocalJSXCommandContext,
) => Promise<LocalCommandResult>;

export type LocalCommandResult =
  | { type: 'text'; value: string }
  | { type: 'compact'; compactionResult: CompactionResult; displayText?: string }
  | { type: 'skip' };

// Local JSX command - renders React UI
export type LocalJSXCommandCall = (
  onDone: LocalJSXCommandOnDone,
  context: ToolUseContext & LocalJSXCommandContext,
  args: string,
) => Promise<React.ReactNode>;

// Command result display options
export type CommandResultDisplay = 'skip' | 'system' | 'user';

export type LocalJSXCommandOnDone = (
  result?: string,
  options?: {
    display?: CommandResultDisplay;
    shouldQuery?: boolean;  // Send messages to model after complete
    metaMessages?: string[];
    nextInput?: string;
    submitNextInput?: boolean;
  },
) => void;

// Base command properties
export type CommandBase = {
  availability?: CommandAvailability[];
  description: string;
  isEnabled?: () => boolean;
  isHidden?: boolean;
  name: string;
  aliases?: string[];
  argumentHint?: string;
  whenToUse?: string;
  version?: string;
  disableModelInvocation?: boolean;
  userInvocable?: boolean;
  immediate?: boolean;  // Execute without waiting for stop point
  isSensitive?: boolean;  // Redact args from history
  userFacingName?: () => string;
};

// Union of all command types
export type Command = CommandBase & (PromptCommand | LocalCommand | LocalJSXCommand);
```

### hooks.ts - Hook System

```typescript
// Prompt elicitation protocol
export const promptRequestSchema = lazySchema(() =>
  z.object({
    prompt: z.string(),  // request id
    message: z.string(),
    options: z.array(
      z.object({
        key: z.string(),
        label: z.string(),
        description: z.string().optional(),
      }),
    ),
  }),
);

export type PromptRequest = z.infer<typeof promptRequestSchema>;

export type PromptResponse = {
  prompt_response: string;  // request id
  selected: string;
};

// Sync hook response
export const syncHookResponseSchema = lazySchema(() =>
  z.object({
    continue: z.boolean().optional(),
    suppressOutput: z.boolean().optional(),
    stopReason: z.string().optional(),
    decision: z.enum(['approve', 'block']).optional(),
    reason: z.string().optional(),
    systemMessage: z.string().optional(),
    hookSpecificOutput: z.union([
      // PreToolUse
      z.object({
        hookEventName: z.literal('PreToolUse'),
        permissionBehavior: permissionBehaviorSchema().optional(),
        updatedInput: z.record(z.string(), z.unknown()).optional(),
        additionalContext: z.string().optional(),
      }),
      // UserPromptSubmit
      z.object({
        hookEventName: z.literal('UserPromptSubmit'),
        additionalContext: z.string().optional(),
      }),
      // SessionStart
      z.object({
        hookEventName: z.literal('SessionStart'),
        watchPaths: z.array(z.string()).optional(),
      }),
      // PermissionRequest
      z.object({
        hookEventName: z.literal('PermissionRequest'),
        decision: z.union([
          z.object({
            behavior: z.literal('allow'),
            updatedInput: z.record(z.string(), z.unknown()).optional(),
            updatedPermissions: z.array(permissionUpdateSchema()).optional(),
          }),
          z.object({
            behavior: z.literal('deny'),
            message: z.string().optional(),
            interrupt: z.boolean().optional(),
          }),
        ]),
      }),
      // ... 10+ more event-specific outputs
    ]).optional(),
  }),
);

// Async hook response
export const asyncHookResponseSchema = z.object({
  async: z.literal(true),
  asyncTimeout: z.number().optional(),
});

export type HookJSONOutput = z.infer<typeof hookJSONOutputSchema>;

// Hook callback type
export type HookCallback = {
  type: 'callback';
  callback: (
    input: HookInput,
    toolUseID: string | null,
    abort: AbortSignal | undefined,
    hookIndex?: number,
    context?: HookCallbackContext,
  ) => Promise<HookJSONOutput>;
  timeout?: number;  // Timeout in seconds
  internal?: boolean;  // Exclude from metrics
};

// Hook result
export type HookResult = {
  message?: Message;
  systemMessage?: Message;
  blockingError?: HookBlockingError;
  outcome: 'success' | 'blocking' | 'non_blocking_error' | 'cancelled';
  preventContinuation?: boolean;
  stopReason?: string;
  permissionBehavior?: 'ask' | 'deny' | 'allow' | 'passthrough';
  additionalContext?: string;
  updatedInput?: Record<string, unknown>;
  permissionRequestResult?: PermissionRequestResult;
  retry?: boolean;
};

// Permission request result
export type PermissionRequestResult =
  | {
      behavior: 'allow';
      updatedInput?: Record<string, unknown>;
      updatedPermissions?: PermissionUpdate[];
    }
  | {
      behavior: 'deny';
      message?: string;
      interrupt?: boolean;
    };
```

### ids.ts - Branded ID Types

```typescript
/**
 * Branded types prevent mixing up session IDs and agent IDs at compile time.
 */

/**
 * Session ID uniquely identifies a Claude Code session.
 */
export type SessionId = string & { readonly __brand: 'SessionId' };

/**
 * Agent ID uniquely identifies a subagent within a session.
 * Format: `a` + optional `<label>-` + 16 hex chars
 */
export type AgentId = string & { readonly __brand: 'AgentId' };

/**
 * Cast a raw string to SessionId. Use sparingly.
 */
export function asSessionId(id: string): SessionId {
  return id as SessionId;
}

/**
 * Cast a raw string to AgentId. Use sparingly.
 */
export function asAgentId(id: string): AgentId {
  return id as AgentId;
}

/**
 * Validate and brand a string as AgentId.
 * Returns null if the string doesn't match the pattern.
 */
const AGENT_ID_PATTERN = /^a(?:.+-)?[0-9a-f]{16}$/;

export function toAgentId(s: string): AgentId | null {
  return AGENT_ID_PATTERN.test(s) ? (s as AgentId) : null;
}
```

### logs.ts - Session Logging

```typescript
/**
 * Log entry for session history in ~/.claude/sessions/
 */
export type LogOption = {
  date: string;
  messages: SerializedMessage[];
  fullPath?: string;
  value: number;
  created: Date;
  modified: Date;
  firstPrompt: string;
  messageCount: number;
  fileSize?: number;
  isSidechain: boolean;
  isLite?: boolean;  // Messages not loaded
  sessionId?: string;
  teamName?: string;
  agentName?: string;
  agentColor?: string;
  agentSetting?: string;
  isTeammate?: boolean;
  summary?: string;
  customTitle?: string;
  tag?: string;
  fileHistorySnapshots?: FileHistorySnapshot[];
  attributionSnapshots?: AttributionSnapshotMessage[];
  contextCollapseCommits?: ContextCollapseCommitEntry[];
  contextCollapseSnapshot?: ContextCollapseSnapshotEntry;
  gitBranch?: string;
  projectPath?: string;
  prNumber?: number;
  prUrl?: string;
  prRepository?: string;
  mode?: 'coordinator' | 'normal';
  worktreeSession?: PersistedWorktreeSession | null;
  contentReplacements?: ContentReplacementRecord[];
};

/**
 * Serialized message with metadata for disk persistence
 */
export type SerializedMessage = Message & {
  cwd: string;
  userType: string;
  entrypoint?: string;  // CLAUDE_CODE_ENTRYPOINT
  sessionId: string;
  timestamp: string;
  version: string;
  gitBranch?: string;
  slug?: string;  // For resume
};

/**
 * Special transcript message types
 */
export type SummaryMessage = { type: 'summary'; leafUuid: UUID; summary: string };
export type CustomTitleMessage = { type: 'custom-title'; sessionId: UUID; customTitle: string };
export type AiTitleMessage = { type: 'ai-title'; sessionId: UUID; aiTitle: string };
export type LastPromptMessage = { type: 'last-prompt'; sessionId: UUID; lastPrompt: string };
export type TaskSummaryMessage = { type: 'task-summary'; sessionId: UUID; summary: string; timestamp: string };
export type TagMessage = { type: 'tag'; sessionId: UUID; tag: string };
export type AgentNameMessage = { type: 'agent-name'; sessionId: UUID; agentName: string };
export type AgentColorMessage = { type: 'agent-color'; sessionId: UUID; agentColor: string };
export type PRLinkMessage = { type: 'pr-link'; sessionId: UUID; prNumber: number; prUrl: string; prRepository: string; timestamp: string };

/**
 * Context collapse for compressing conversation history
 */
export type ContextCollapseCommitEntry = {
  type: 'marble-origami-commit';
  sessionId: UUID;
  collapseId: string;  // 16-digit ID
  summaryUuid: string;
  summaryContent: string;  // Full <collapsed id="...">text</collapsed>
  summary: string;  // Plain text for ctx_inspect
  firstArchivedUuid: string;
  lastArchivedUuid: string;
};

export type ContextCollapseSnapshotEntry = {
  type: 'marble-origami-snapshot';
  sessionId: UUID;
  staged: Array<{
    startUuid: string;
    endUuid: string;
    summary: string;
    risk: number;
    stagedAt: number;
  }>;
  armed: boolean;
  lastSpawnTokens: number;
};

/**
 * File attribution for commit attribution
 */
export type FileAttributionState = {
  contentHash: string;  // SHA-256
  claudeContribution: number;  // Characters written by Claude
  mtime: number;
};

export type AttributionSnapshotMessage = {
  type: 'attribution-snapshot';
  messageId: UUID;
  surface: string;  // cli, ide, web, api
  fileStates: Record<string, FileAttributionState>;
  promptCount?: number;
  permissionPromptCount?: number;
  escapeCount?: number;
};

// Union of all transcript entry types
export type Entry =
  | TranscriptMessage
  | SummaryMessage
  | CustomTitleMessage
  | AiTitleMessage
  | LastPromptMessage
  | TaskSummaryMessage
  | TagMessage
  | AgentNameMessage
  | AgentColorMessage
  | PRLinkMessage
  | FileHistorySnapshotMessage
  | AttributionSnapshotMessage
  | QueueOperationMessage
  | SpeculationAcceptMessage
  | ModeEntry
  | WorktreeStateEntry
  | ContentReplacementEntry
  | ContextCollapseCommitEntry
  | ContextCollapseSnapshotEntry;
```

### permissions.ts - Permission System

```typescript
// Permission modes
export const EXTERNAL_PERMISSION_MODES = [
  'acceptEdits',
  'bypassPermissions',
  'default',
  'dontAsk',
  'plan',
] as const;

export type ExternalPermissionMode = (typeof EXTERNAL_PERMISSION_MODES)[number];
export type InternalPermissionMode = ExternalPermissionMode | 'auto' | 'bubble';
export type PermissionMode = InternalPermissionMode;

// Permission behaviors
export type PermissionBehavior = 'allow' | 'deny' | 'ask';

// Permission rules
export type PermissionRuleSource =
  | 'userSettings'
  | 'projectSettings'
  | 'localSettings'
  | 'flagSettings'
  | 'policySettings'
  | 'cliArg'
  | 'command'
  | 'session';

export type PermissionRuleValue = {
  toolName: string;
  ruleContent?: string;
};

export type PermissionRule = {
  source: PermissionRuleSource;
  ruleBehavior: PermissionBehavior;
  ruleValue: PermissionRuleValue;
};

// Permission updates
export type PermissionUpdate =
  | { type: 'addRules'; destination: PermissionUpdateDestination; rules: PermissionRuleValue[]; behavior: PermissionBehavior }
  | { type: 'replaceRules'; destination: PermissionUpdateDestination; rules: PermissionRuleValue[]; behavior: PermissionBehavior }
  | { type: 'removeRules'; destination: PermissionUpdateDestination; rules: PermissionRuleValue[]; behavior: PermissionBehavior }
  | { type: 'setMode'; destination: PermissionUpdateDestination; mode: ExternalPermissionMode }
  | { type: 'addDirectories'; destination: PermissionUpdateDestination; directories: string[] }
  | { type: 'removeDirectories'; destination: PermissionUpdateDestination; directories: string[] };

// Permission decisions
export type PermissionAllowDecision<Input extends { [key: string]: unknown } = { [key: string]: unknown }> = {
  behavior: 'allow';
  updatedInput?: Input;
  userModified?: boolean;
  decisionReason?: PermissionDecisionReason;
  toolUseID?: string;
  acceptFeedback?: string;
  contentBlocks?: ContentBlockParam[];
};

export type PermissionAskDecision<Input extends { [key: string]: unknown } = { [key: string]: unknown }> = {
  behavior: 'ask';
  message: string;
  updatedInput?: Input;
  decisionReason?: PermissionDecisionReason;
  suggestions?: PermissionUpdate[];
  blockedPath?: string;
  metadata?: PermissionMetadata;
  pendingClassifierCheck?: PendingClassifierCheck;
  contentBlocks?: ContentBlockParam[];
};

export type PermissionDenyDecision = {
  behavior: 'deny';
  message: string;
  decisionReason: PermissionDecisionReason;
  toolUseID?: string;
};

export type PermissionDecision<Input extends { [key: string]: unknown } = { [key: string]: unknown }> =
  | PermissionAllowDecision<Input>
  | PermissionAskDecision<Input>
  | PermissionDenyDecision;

export type PermissionResult<Input extends { [key: string]: unknown } = { [key: string]: unknown }> =
  | PermissionDecision<Input>
  | {
      behavior: 'passthrough';
      message: string;
      decisionReason?: PermissionDecisionReason;
      suggestions?: PermissionUpdate[];
      blockedPath?: string;
      pendingClassifierCheck?: PendingClassifierCheck;
    };

// Decision reason types
export type PermissionDecisionReason =
  | { type: 'rule'; rule: PermissionRule }
  | { type: 'mode'; mode: PermissionMode }
  | { type: 'subcommandResults'; reasons: Map<string, PermissionResult> }
  | { type: 'permissionPromptTool'; permissionPromptToolName: string; toolResult: unknown }
  | { type: 'hook'; hookName: string; hookSource?: string; reason?: string }
  | { type: 'asyncAgent'; reason: string }
  | { type: 'sandboxOverride'; reason: 'excludedCommand' | 'dangerouslyDisableSandbox' }
  | { type: 'classifier'; classifier: string; reason: string }
  | { type: 'workingDir'; reason: string }
  | { type: 'safetyCheck'; reason: string; classifierApprovable: boolean }
  | { type: 'other'; reason: string };

// Bash classifier types
export type YoloClassifierResult = {
  thinking?: string;
  shouldBlock: boolean;
  reason: string;
  unavailable?: boolean;
  transcriptTooLong?: boolean;
  model: string;
  usage?: ClassifierUsage;
  durationMs?: number;
  stage?: 'fast' | 'thinking';
  stage1Usage?: ClassifierUsage;
  stage1RequestId?: string;
  stage1MsgId?: string;
  stage2Usage?: ClassifierUsage;
  stage2DurationMs?: number;
  stage2RequestId?: string;
  stage2MsgId?: string;
};
```

### textInputTypes.ts - Text Input and Command Queue

```typescript
// Queue priority levels
export type QueuePriority = 'now' | 'next' | 'later';
// - 'now': Interrupt and send immediately (aborts in-flight tool call)
// - 'next': Mid-turn drain (after current tool result, before next API call)
// - 'later': End-of-turn drain (after current turn finishes)

// Queued command
export type QueuedCommand = {
  value: string | Array<ContentBlockParam>;
  mode: PromptInputMode;
  priority?: QueuePriority;
  uuid?: UUID;
  orphanedPermission?: OrphanedPermission;
  pastedContents?: Record<number, PastedContent>;
  preExpansionValue?: string;  // Before [Pasted text #N] expansion
  skipSlashCommands?: boolean;  // Treat as plain text even if starts with /
  bridgeOrigin?: boolean;  // Filter through isBridgeSafeCommand()
  isMeta?: boolean;  // UserMessage gets isMeta: true
  origin?: MessageOrigin;  // undefined = human (keyboard)
  workload?: string;  // For billing-header attribution
  agentId?: AgentId;  // undefined = main thread
};

export type PromptInputMode =
  | 'bash'
  | 'prompt'
  | 'orphaned-permission'
  | 'task-notification';

export type EditablePromptInputMode = Exclude<PromptInputMode, `${string}-notification`>;

// Base text input props
export type BaseTextInputProps = {
  readonly onHistoryUp?: () => void;
  readonly onHistoryDown?: () => void;
  readonly placeholder?: string;
  readonly multiline?: boolean;
  readonly focus?: boolean;
  readonly mask?: string;
  readonly showCursor?: boolean;
  readonly highlightPastedText?: boolean;
  readonly value: string;
  readonly onChange: (value: string) => void;
  readonly onSubmit?: (value: string) => void;
  readonly onExit?: () => void;
  readonly onExitMessage?: (show: boolean, key?: string) => void;
  readonly onHistoryReset?: () => void;
  readonly onClearInput?: () => void;
  readonly columns: number;
  readonly maxVisibleLines?: number;
  readonly onImagePaste?: (
    base64Image: string,
    mediaType?: string,
    filename?: string,
    dimensions?: ImageDimensions,
    sourcePath?: string,
  ) => void;
  readonly onPaste?: (text: string) => void;
  readonly onIsPastingChange?: (isPasting: boolean) => void;
  readonly disableCursorMovementForUpDownKeys?: boolean;
  readonly disableEscapeDoublePress?: boolean;
  readonly cursorOffset: number;
  onChangeCursorOffset: (offset: number) => void;
  readonly argumentHint?: string;
  readonly onUndo?: () => void;
  readonly dimColor?: boolean;
  readonly highlights?: TextHighlight[];
  readonly placeholderElement?: React.ReactNode;
  readonly inlineGhostText?: InlineGhostText;
  readonly inputFilter?: (input: string, key: Key) => string;
};

// Vim-specific props
export type VimTextInputProps = BaseTextInputProps & {
  readonly initialMode?: VimMode;
  readonly onModeChange?: (mode: VimMode) => void;
};

export type VimMode = 'INSERT' | 'NORMAL';

// Input state
export type BaseInputState = {
  onInput: (input: string, key: Key) => void;
  renderedValue: string;
  offset: number;
  setOffset: (offset: number) => void;
  cursorLine: number;
  cursorColumn: number;
  viewportCharOffset: number;
  viewportCharEnd: number;
  isPasting?: boolean;
  pasteState?: {
    chunks: string[];
    timeoutId: ReturnType<typeof setTimeout> | null;
  };
};

export type TextInputState = BaseInputState;
export type VimInputState = BaseInputState & {
  mode: VimMode;
  setMode: (mode: VimMode) => void;
};

// Inline ghost text for autocomplete
export type InlineGhostText = {
  readonly text: string;
  readonly fullCommand: string;
  readonly insertPosition: number;
};

// Orphaned permission (permission without associated tool use)
export type OrphanedPermission = {
  permissionResult: PermissionResult;
  assistantMessage: AssistantMessage;
};
```

### plugin.ts - Plugin System

```typescript
// Built-in plugin definition
export type BuiltinPluginDefinition = {
  name: string;
  description: string;
  version?: string;
  skills?: BundledSkillDefinition[];
  hooks?: HooksSettings;
  mcpServers?: Record<string, McpServerConfig>;
  isAvailable?: () => boolean;
  defaultEnabled?: boolean;
};

// Loaded plugin
export type LoadedPlugin = {
  name: string;
  manifest: PluginManifest;
  path: string;
  source: string;
  repository: string;
  enabled?: boolean;
  isBuiltin?: boolean;
  sha?: string;  // Git commit SHA for version pinning
  commandsPath?: string;
  commandsPaths?: string[];
  commandsMetadata?: Record<string, CommandMetadata>;
  agentsPath?: string;
  agentsPaths?: string[];
  skillsPath?: string;
  skillsPaths?: string[];
  outputStylesPath?: string;
  outputStylesPaths?: string[];
  hooksConfig?: HooksSettings;
  mcpServers?: Record<string, McpServerConfig>;
  lspServers?: Record<string, LspServerConfig>;
  settings?: Record<string, unknown>;
};

export type PluginComponent = 'commands' | 'agents' | 'skills' | 'hooks' | 'output-styles';

// Plugin error types (25 variants)
export type PluginError =
  | { type: 'path-not-found'; source: string; plugin?: string; path: string; component: PluginComponent }
  | { type: 'git-auth-failed'; source: string; plugin?: string; gitUrl: string; authType: 'ssh' | 'https' }
  | { type: 'git-timeout'; source: string; plugin?: string; gitUrl: string; operation: 'clone' | 'pull' }
  | { type: 'network-error'; source: string; plugin?: string; url: string; details?: string }
  | { type: 'manifest-parse-error'; source: string; plugin?: string; manifestPath: string; parseError: string }
  | { type: 'manifest-validation-error'; source: string; plugin?: string; manifestPath: string; validationErrors: string[] }
  | { type: 'plugin-not-found'; source: string; pluginId: string; marketplace: string }
  | { type: 'marketplace-not-found'; source: string; marketplace: string; availableMarketplaces: string[] }
  | { type: 'marketplace-load-failed'; source: string; marketplace: string; reason: string }
  | { type: 'mcp-config-invalid'; source: string; plugin: string; serverName: string; validationError: string }
  | { type: 'mcp-server-suppressed-duplicate'; source: string; plugin: string; serverName: string; duplicateOf: string }
  | { type: 'hook-load-failed'; source: string; plugin: string; hookPath: string; reason: string }
  | { type: 'component-load-failed'; source: string; plugin: string; component: PluginComponent; path: string; reason: string }
  | { type: 'dependency-unsatisfied'; source: string; plugin: string; dependency: string; reason: 'not-enabled' | 'not-found' }
  | { type: 'generic-error'; source: string; plugin?: string; error: string }
  // ... plus LSP-specific and MCPB-specific errors

export type PluginLoadResult = {
  enabled: LoadedPlugin[];
  disabled: LoadedPlugin[];
  errors: PluginError[];
};
```

### Generated Protobuf Types

```typescript
// Analytics event for Statsig logging
export interface ClaudeCodeInternalEvent {
  event_name?: string;
  client_timestamp?: Date;
  model?: string;
  session_id?: string;
  user_type?: string;
  betas?: string;
  env?: EnvironmentMetadata;
  entrypoint?: string;
  agent_sdk_version?: string;
  is_interactive?: boolean;
  client_type?: string;
  process?: string;  // JSON string with process metrics
  additional_metadata?: string;  // JSON with event-specific fields
  auth?: PublicApiAuth;
  server_timestamp?: Date;
  event_id?: string;
  device_id?: string;
  swe_bench_run_id?: string;
  swe_bench_instance_id?: string;
  agent_id?: string;
  parent_session_id?: string;
  slack?: SlackContext;
  team_name?: string;
  skill_name?: string;
  plugin_name?: string;
  marketplace_name?: string;
}

// Environment and runtime information
export interface EnvironmentMetadata {
  platform?: string;
  node_version?: string;
  terminal?: string;
  package_managers?: string;
  runtimes?: string;
  is_running_with_bun?: boolean;
  is_ci?: boolean;
  is_github_action?: boolean;
  version?: string;
  github_event_name?: string;
  github_actions_metadata?: GitHubActionsMetadata;
  linux_distro_id?: string;
  linux_distro_version?: string;
  vcs?: string;
  // ... 30+ more fields
}

// GrowthBook A/B test event
export interface GrowthbookExperimentEvent {
  event_id?: string;
  timestamp?: Date;
  experiment_id?: string;
  variation_id?: number;  // 0=control, 1+=variants
  environment?: string;
  user_attributes?: string;
  device_id?: string;
  auth?: PublicApiAuth;
  session_id?: string;
  anonymous_id?: string;
}
```

---

## Line-by-Line Analysis

### command.ts - Command Discriminator Pattern

**Lines 16-57: PromptCommand structure**

```typescript
export type PromptCommand = {
  type: 'prompt';
  progressMessage: string;
  contentLength: number;  // Length of command content in characters (used for token estimation)
  argNames?: string[];
  allowedTools?: string[];
  model?: string;
  source: SettingSource | 'builtin' | 'mcp' | 'plugin' | 'bundled';
  pluginInfo?: {
    pluginManifest: PluginManifest;
    repository: string;
  };
  context?: 'inline' | 'fork';  // Execution context
  agent?: string;  // Agent type for forked execution
  effort?: EffortValue;
  paths?: string[];  // Glob patterns for file matching
  getPromptForCommand(
    args: string,
    context: ToolUseContext,
  ): Promise<ContentBlockParam[]>;
};
```

**Explanation:**
- `type: 'prompt'` discriminator distinguishes from `local` and `local-jsx` commands
- `contentLength` used for token estimation before API call (characters / ~4 = tokens)
- `context` determines if command runs inline (current conversation) or forked (sub-agent)
- `paths` enables file-gated commands - only visible after model touches matching files
- `getPromptForCommand` is async to allow file reading, API calls, etc.

**Lines 205-216: Command utilities**

```typescript
export function getCommandName(cmd: CommandBase): string {
  return cmd.userFacingName?.() ?? cmd.name;
}

export function isCommandEnabled(cmd: CommandBase): boolean {
  return cmd.isEnabled?.() ?? true;
}
```

**Explanation:**
- `getCommandName` handles overridden display names (plugin prefix stripping)
- `isCommandEnabled` defaults to `true` when no `isEnabled` function provided
- Both handle optional fields gracefully with nullish coalescing

### hooks.ts - Zod Schema Composition

**Lines 50-166: Sync hook response schema**

```typescript
export const syncHookResponseSchema = lazySchema(() =>
  z.object({
    continue: z.boolean().describe('Whether Claude should continue after hook').optional(),
    suppressOutput: z.boolean().describe('Hide stdout from transcript').optional(),
    stopReason: z.string().describe('Message shown when continue is false').optional(),
    decision: z.enum(['approve', 'block']).optional(),
    reason: z.string().describe('Explanation for the decision').optional(),
    hookSpecificOutput: z.union([
      z.object({
        hookEventName: z.literal('PreToolUse'),
        permissionBehavior: permissionBehaviorSchema().optional(),
        updatedInput: z.record(z.string(), z.unknown()).optional(),
        additionalContext: z.string().optional(),
      }),
      z.object({
        hookEventName: z.literal('PermissionRequest'),
        decision: z.union([
          z.object({
            behavior: z.literal('allow'),
            updatedInput: z.record(z.string(), z.unknown()).optional(),
            updatedPermissions: z.array(permissionUpdateSchema()).optional(),
          }),
          z.object({
            behavior: z.literal('deny'),
            message: z.string().optional(),
            interrupt: z.boolean().optional(),
          }),
        ]),
      }),
      // ... 10+ more event-specific outputs
    ]).optional(),
  }),
);
```

**Explanation:**
- `lazySchema` defers schema creation to avoid circular dependencies
- `hookSpecificOutput` is discriminated union by `hookEventName` literal
- Each hook event has its own specific output shape
- Type inference via `z.infer` creates TypeScript types from schema
- Runtime validation ensures hook responses match expected format

### logs.ts - Context Collapse Types

**Lines 255-295: Context collapse entries**

```typescript
/**
 * Persisted context-collapse commit. The archived messages themselves are
 * NOT persisted - they're already in the transcript. We only persist
 * enough to reconstruct the splice instruction and summary placeholder.
 */
export type ContextCollapseCommitEntry = {
  type: 'marble-origami-commit';
  sessionId: UUID;
  collapseId: string;  // 16-digit collapse ID
  summaryUuid: string;
  summaryContent: string;  // Full <collapsed id="...">text</collapsed>
  summary: string;  // Plain text for ctx_inspect
  firstArchivedUuid: string;
  lastArchivedUuid: string;
};

/**
 * Snapshot of the staged queue and spawn trigger state.
 * Unlike commits (append-only), snapshots are last-wins.
 */
export type ContextCollapseSnapshotEntry = {
  type: 'marble-origami-snapshot';
  sessionId: UUID;
  staged: Array<{
    startUuid: string;
    endUuid: string;
    summary: string;
    risk: number;
    stagedAt: number;
  }>;
  armed: boolean;
  lastSpawnTokens: number;
};
```

**Explanation:**
- Context collapse compresses conversation history while preserving structure
- `collapseId` is 16-digit hex for display/stable references
- `summaryContent` contains full XML placeholder for rendering
- Boundaries (`firstArchivedUuid`, `lastArchivedUuid`) identify span in transcript
- Snapshots are last-wins (unlike append-only commits)
- `armed` and `lastSpawnTokens` track auto-summarization trigger state

### permissions.ts - Decision Reason Discriminator

**Lines 271-324: PermissionDecisionReason**

```typescript
export type PermissionDecisionReason =
  | {
      type: 'rule';
      rule: PermissionRule;
    }
  | {
      type: 'mode';
      mode: PermissionMode;
    }
  | {
      type: 'subcommandResults';
      reasons: Map<string, PermissionResult>;
    }
  | {
      type: 'hook';
      hookName: string;
      hookSource?: string;
      reason?: string;
    }
  | {
      type: 'classifier';
      classifier: string;
      reason: string;
    }
  | {
      type: 'safetyCheck';
      reason: string;
      classifierApprovable: boolean;
    }
  | { type: 'other'; reason: string };
```

**Explanation:**
- Every permission decision carries its reason for transparency
- `rule` means a user/project policy matched
- `mode` means the global mode (auto, plan, etc.) determined outcome
- `classifier` means the LLM-based allow classifier was used
- `safetyCheck.classifierApprovable` indicates if classifier can override
- Used in UI to show "Why was this blocked?" explanations

### textInputTypes.ts - Queue Priority Semantics

**Lines 293-358: QueuePriority and QueuedCommand**

```typescript
/**
 * Queue priority levels:
 * - 'now': Interrupt and send immediately. Aborts any in-flight tool call.
 * - 'next': Mid-turn drain. After current tool result, before next API call.
 * - 'later': End-of-turn drain. After current turn finishes.
 */
export type QueuePriority = 'now' | 'next' | 'later';

export type QueuedCommand = {
  value: string | Array<ContentBlockParam>;
  mode: PromptInputMode;
  priority?: QueuePriority;
  uuid?: UUID;
  orphanedPermission?: OrphanedPermission;
  pastedContents?: Record<number, PastedContent>;
  preExpansionValue?: string;  // Before [Pasted text #N] expansion
  skipSlashCommands?: boolean;
  bridgeOrigin?: boolean;
  isMeta?: boolean;  // Hidden in transcript UI but visible to model
  origin?: MessageOrigin;  // undefined = human (keyboard)
  workload?: string;  // For billing-header attribution
  agentId?: AgentId;  // undefined = main thread
};
```

**Explanation:**
- `now` priority used for urgent commands (Esc + send)
- `next` used for background task completions
- `later` is default - waits for current turn
- `orphanedPermission` handles permission prompts without active tool use
- `preExpansionValue` used for ultraplan keyword detection (avoid false positives from pasted content)
- `isMeta` for system-generated prompts (proactive ticks, teammate messages)
- `agentId` routes to specific subagent (unified queue for all agents)

---

## Integration Points

### How types/ Integrates with Other Modules

#### 1. With `state/` Module

```typescript
// logs.ts
export type LogOption = {
  messages: SerializedMessage[];
  // ...
};

// state/AppState.ts uses LogOption for session history
type AppState = {
  logs: LogOption[];
  // ...
};
```

**Integration Pattern:**
- Type definitions in `types/` are consumed by `state/` for store structure
- `LogOption` defines the shape of session history entries
- `SerializedMessage` extends `Message` with persistence metadata

#### 2. With `tools/` Module

```typescript
// permissions.ts
export type PermissionResult<Input> =
  | PermissionAllowDecision<Input>
  | PermissionAskDecision<Input>
  | PermissionDenyDecision
  | { behavior: 'passthrough'; /* ... */ };

// tools/BashTool uses PermissionResult
function bashToolHasPermission(
  command: string,
  context: ToolPermissionContext,
): Promise<PermissionResult>;
```

**Integration Pattern:**
- Permission types define the contract for tool authorization
- Tools call permission check functions returning `PermissionResult`
- Discriminated union enables exhaustive type checking in tool execution

#### 3. With `components/` Module

```typescript
// textInputTypes.ts
export type BaseTextInputProps = {
  readonly value: string;
  readonly onChange: (value: string) => void;
  // ...
};

// components/PromptInput/PromptInput.tsx
function PromptInput(props: BaseTextInputProps & PromptInputSpecificProps) {
  // ...
}
```

**Integration Pattern:**
- TextInput types define reusable component interfaces
- `BaseTextInputProps` shared between `TextInput` and `VimTextInput`
- Component-specific props extend base types

#### 4. With `utils/` Module

```typescript
// plugin.ts
export type PluginError =
  | { type: 'plugin-not-found'; /* ... */ }
  | { type: 'generic-error'; /* ... */ }
  // ...

export function getPluginErrorMessage(error: PluginError): string {
  switch (error.type) {
    case 'plugin-not-found':
      return `Plugin ${error.pluginId} not found in marketplace ${error.marketplace}`;
    // ...
  }
}
```

**Integration Pattern:**
- Error types paired with formatting utilities
- Exhaustive switch ensures all error types handled
- Type-safe error handling prevents string-based matching

#### 5. With `services/` Module (Analytics)

```typescript
// generated/events_mono/claude_code/v1/claude_code_internal_event.ts
export interface ClaudeCodeInternalEvent {
  event_name?: string;
  session_id?: string;
  env?: EnvironmentMetadata;
  // ...
}

// services/statsig.ts uses ClaudeCodeInternalEvent
function logEvent(event: ClaudeCodeInternalEvent): void;
```

**Integration Pattern:**
- Protobuf-generated types provide cross-language event schema
- Analytics service serializes events to protobuf format
- Type safety ensures required fields present before logging

---

## Type Hierarchy and Dependencies

```
types/
├── ids.ts (no dependencies - base types)
├── permissions.ts (depends on ids.ts)
├── command.ts (depends on permissions.ts, types/)
├── hooks.ts (depends on permissions.ts, state/)
├── logs.ts (depends on ids.ts, message.ts)
├── plugin.ts (depends on hooks.ts, services/)
├── textInputTypes.ts (depends on permissions.ts, ids.ts, message.ts)
└── generated/
    ├── auth.ts (standalone)
    ├── timestamp.ts (standalone)
    ├── growthbook_experiment_event.ts (depends on auth.ts, timestamp.ts)
    └── claude_code_internal_event.ts (depends on auth.ts, timestamp.ts)
```

**Key Observations:**

1. **ids.ts is foundational**: No dependencies, used by almost everything
2. **permissions.ts is central**: Referenced by command, hooks, textInputTypes
3. **Generated types are isolated**: Self-contained protobuf definitions
4. **Circular dependencies avoided**: Careful separation of type-only imports

---

## Summary

The `types/` module is the **type definition backbone** of Claude Code, providing:

- **11 files** defining ~3,500 lines of type signatures
- **Discriminated unions** for commands, permissions, hooks, plugins
- **Branded types** for session/agent ID safety
- **Zod schemas** for runtime validation of hook responses
- **Protobuf types** for cross-language analytics events
- **Comprehensive error types** (25 plugin error variants)
- **Context collapse types** for conversation compression
- **Permission system types** with decision reason tracking

The module demonstrates sophisticated TypeScript patterns while maintaining type safety across module boundaries.
