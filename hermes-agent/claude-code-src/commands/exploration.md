# Commands Module Deep Dive

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/commands/`  
**Output:** `/home/darkvoid/Boxxed/@dev/repo-expolorations/hermes-agent/claude-code-src/commands/exploration.md`

This module contains **189 files** - the largest command implementation module in Claude Code. This document provides a comprehensive deep dive into the command registry architecture, command categorization, implementation patterns, and execution pipeline.

---

## Table of Contents

1. [Command Registry Architecture](#1-command-registry-architecture)
2. [Command Type System](#2-command-type-system)
3. [Command Categorization](#3-command-categorization)
4. [Command Execution Pipeline](#4-command-execution-pipeline)
5. [Major Command Category Deep Dives](#5-major-command-category-deep-dives)
6. [Representative Command Implementations](#6-representative-command-implementations)
7. [Plugin and Skill System Integration](#7-plugin-and-skill-system-integration)
8. [Feature Flags and Conditional Commands](#8-feature-flags-and-conditional-commands)

---

## 1. Command Registry Architecture

### 1.1 Central Command Registry (`commands.ts`)

The command registry is the heart of the commands module, located at `src/commands.ts`. It serves as the central hub for:
- Importing and aggregating all built-in commands
- Managing dynamic commands (skills, plugins, workflows)
- Filtering commands based on availability and feature flags
- Providing memoized command lookup

**File:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/commands.ts`

#### 1.1.1 Command Imports and Aggregation

```typescript
// biome-ignore-all assist/source/organizeImports: ANT-ONLY import markers must not be reordered
import addDir from './commands/add-dir/index.js'
import autofixPr from './commands/autofix-pr/index.js'
import backfillSessions from './commands/backfill-sessions/index.js'
import btw from './commands/btw/index.js'
import goodClaude from './commands/good-claude/index.js'
import issue from './commands/issue/index.js'
import feedback from './commands/feedback/index.js'
import clear from './commands/clear/index.js'
import color from './commands/color/index.js'
import commit from './commands/commit.js'
import copy from './commands/copy/index.js'
// ... 100+ more imports

// Feature-gated imports using conditional require
/* eslint-disable @typescript-eslint/no-require-imports */
const agentsPlatform =
  process.env.USER_TYPE === 'ant'
    ? require('./commands/agents-platform/index.js').default
    : null
/* eslint-enable @typescript-eslint/no-require-imports */

// Feature flag-based conditional imports
const proactive =
  feature('PROACTIVE') || feature('KAIROS')
    ? require('./commands/proactive.js').default
    : null
const briefCommand =
  feature('KAIROS') || feature('KAIROS_BRIEF')
    ? require('./commands/brief.js').default
    : null
const assistantCommand = feature('KAIROS')
  ? require('./commands/assistant/index.js').default
  : null
const bridge = feature('BRIDGE_MODE')
  ? require('./commands/bridge/index.js').default
  : null
```

#### 1.1.2 Internal-Only Commands

Commands that are only available for internal Anthropic users (`USER_TYPE === 'ant'`):

```typescript
// Commands that get eliminated from the external build
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
  ...(forceSnip ? [forceSnip] : []),
  mockLimits,
  bridgeKick,
  version,
  ...(ultraplan ? [ultraplan] : []),
  ...(subscribePr ? [subscribePr] : []),
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
  agentsPlatform,
  autofixPr,
].filter(Boolean)
```

#### 1.1.3 Main Command Array with Memoization

```typescript
// Declared as a function so that we don't run this until getCommands is called,
// since underlying functions read from config, which can't be read at module initialization time
const COMMANDS = memoize((): Command[] => [
  addDir,
  advisor,
  agents,
  branch,
  btw,
  chrome,
  clear,
  color,
  compact,
  config,
  copy,
  desktop,
  context,
  contextNonInteractive,
  cost,
  diff,
  doctor,
  effort,
  exit,
  fast,
  files,
  heapDump,
  help,
  ide,
  init,
  keybindings,
  installGitHubApp,
  installSlackApp,
  mcp,
  memory,
  mobile,
  model,
  outputStyle,
  remoteEnv,
  plugin,
  pr_comments,
  releaseNotes,
  reloadPlugins,
  rename,
  resume,
  session,
  skills,
  stats,
  status,
  statusline,
  stickers,
  tag,
  theme,
  feedback,
  review,
  ultrareview,
  rewind,
  securityReview,
  terminalSetup,
  upgrade,
  extraUsage,
  extraUsageNonInteractive,
  rateLimitOptions,
  usage,
  usageReport,
  vim,
  ...(webCmd ? [webCmd] : []),
  ...(forkCmd ? [forkCmd] : []),
  ...(buddy ? [buddy] : []),
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
  ...(!isUsing3PServices() ? [logout, login()] : []),
  passes,
  ...(peersCmd ? [peersCmd] : []),
  tasks,
  ...(workflowsCmd ? [workflowsCmd] : []),
  ...(torch ? [torch] : []),
  ...(process.env.USER_TYPE === 'ant' && !process.env.IS_DEMO
    ? INTERNAL_ONLY_COMMANDS
    : []),
])
```

#### 1.1.4 Dynamic Command Loading

```typescript
/**
 * Loads all command sources (skills, plugins, workflows). Memoized by cwd
 * because loading is expensive (disk I/O, dynamic imports).
 */
const loadAllCommands = memoize(async (cwd: string): Promise<Command[]> => {
  const [
    { skillDirCommands, pluginSkills, bundledSkills, builtinPluginSkills },
    pluginCommands,
    workflowCommands,
  ] = await Promise.all([
    getSkills(cwd),
    getPluginCommands(),
    getWorkflowCommands ? getWorkflowCommands(cwd) : Promise.resolve([]),
  ])

  return [
    ...bundledSkills,
    ...builtinPluginSkills,
    ...skillDirCommands,
    ...workflowCommands,
    ...pluginCommands,
    ...pluginSkills,
    ...COMMANDS(),
  ]
})
```

#### 1.1.5 Command Availability Filtering

```typescript
/**
 * Filters commands by their declared `availability` (auth/provider requirement).
 * Commands without `availability` are treated as universal.
 * This runs before `isEnabled()` so that provider-gated commands are hidden
 * regardless of feature-flag state.
 *
 * Not memoized — auth state can change mid-session (e.g. after /login),
 * so this must be re-evaluated on every getCommands() call.
 */
export function meetsAvailabilityRequirement(cmd: Command): boolean {
  if (!cmd.availability) return true
  for (const a of cmd.availability) {
    switch (a) {
      case 'claude-ai':
        if (isClaudeAISubscriber()) return true
        break
      case 'console':
        // Console API key user = direct 1P API customer (not 3P, not claude.ai).
        // Excludes 3P (Bedrock/Vertex/Foundry) who don't set ANTHROPIC_BASE_URL
        // and gateway users who proxy through a custom base URL.
        if (
          !isClaudeAISubscriber() &&
          !isUsing3PServices() &&
          isFirstPartyAnthropicBaseUrl()
        )
          return true
        break
      default: {
        const _exhaustive: never = a
        void _exhaustive
        break
      }
    }
  }
  return false
}
```

#### 1.1.6 Public API: getCommands

```typescript
/**
 * Returns commands available to the current user. The expensive loading is
 * memoized, but availability and isEnabled checks run fresh every call so
 * auth changes (e.g. /login) take effect immediately.
 */
export async function getCommands(cwd: string): Promise<Command[]> {
  const allCommands = await loadAllCommands(cwd)

  // Get dynamic skills discovered during file operations
  const dynamicSkills = getDynamicSkills()

  // Build base commands without dynamic skills
  const baseCommands = allCommands.filter(
    _ => meetsAvailabilityRequirement(_) && isCommandEnabled(_),
  )

  if (dynamicSkills.length === 0) {
    return baseCommands
  }

  // Dedupe dynamic skills - only add if not already present
  const baseCommandNames = new Set(baseCommands.map(c => c.name))
  const uniqueDynamicSkills = dynamicSkills.filter(
    s =>
      !baseCommandNames.has(s.name) &&
      meetsAvailabilityRequirement(s) &&
      isCommandEnabled(s),
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

---

## 2. Command Type System

### 2.1 Command Base Types

**File:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/types/command.ts`

```typescript
export type CommandBase = {
  availability?: CommandAvailability[]
  description: string
  hasUserSpecifiedDescription?: boolean
  /** Defaults to true. Only set when the command has conditional enablement (feature flags, env checks, etc). */
  isEnabled?: () => boolean
  /** Defaults to false. Only set when the command should be hidden from typeahead/help. */
  isHidden?: boolean
  name: string
  aliases?: string[]
  isMcp?: boolean
  argumentHint?: string // Hint text for command arguments (displayed in gray after command)
  whenToUse?: string // From the "Skill" spec. Detailed usage scenarios for when to use this command
  version?: string // Version of the command/skill
  disableModelInvocation?: boolean // Whether to disable this command from being invoked by models
  userInvocable?: boolean // Whether users can invoke this skill by typing /skill-name
  loadedFrom?:
    | 'commands_DEPRECATED'
    | 'skills'
    | 'plugin'
    | 'managed'
    | 'bundled'
    | 'mcp' // Where the command was loaded from
  kind?: 'workflow' // Distinguishes workflow-backed commands (badged in autocomplete)
  immediate?: boolean // If true, command executes immediately without waiting for a stop point (bypasses queue)
  isSensitive?: boolean // If true, args are redacted from the conversation history
  /** Defaults to `name`. Only override when the displayed name differs (e.g. plugin prefix stripping). */
  userFacingName?: () => string
}
```

### 2.2 Command Availability Types

```typescript
/**
 * Declares which auth/provider environments a command is available in.
 *
 * This is separate from `isEnabled()`:
 *   - `availability` = who can use this (auth/provider requirement, static)
 *   - `isEnabled()`  = is this turned on right now (GrowthBook, platform, env vars)
 *
 * Commands without `availability` are available everywhere.
 * Commands with `availability` are only shown if the user matches at least one
 * of the listed auth types. See meetsAvailabilityRequirement() in commands.ts.
 */
export type CommandAvailability =
  // claude.ai OAuth subscriber (Pro/Max/Team/Enterprise via claude.ai)
  | 'claude-ai'
  // Console API key user (direct api.anthropic.com, not via claude.ai OAuth)
  | 'console'
```

### 2.3 Prompt Command Type

```typescript
export type PromptCommand = {
  type: 'prompt'
  progressMessage: string
  contentLength: number // Length of command content in characters (used for token estimation)
  argNames?: string[]
  allowedTools?: string[]
  model?: string
  source: SettingSource | 'builtin' | 'mcp' | 'plugin' | 'bundled'
  pluginInfo?: {
    pluginManifest: PluginManifest
    repository: string
  }
  disableNonInteractive?: boolean
  // Hooks to register when this skill is invoked
  hooks?: HooksSettings
  // Base directory for skill resources (used to set CLAUDE_PLUGIN_ROOT environment variable for skill hooks)
  skillRoot?: string
  // Execution context: 'inline' (default) or 'fork' (run as sub-agent)
  // 'inline' = skill content expands into the current conversation
  // 'fork' = skill runs in a sub-agent with separate context and token budget
  context?: 'inline' | 'fork'
  // Agent type to use when forked (e.g., 'Bash', 'general-purpose')
  // Only applicable when context is 'fork'
  agent?: string
  effort?: EffortValue
  // Glob patterns for file paths this skill applies to
  // When set, the skill is only visible after the model touches matching files
  paths?: string[]
  getPromptForCommand(
    args: string,
    context: ToolUseContext,
  ): Promise<ContentBlockParam[]>
}
```

### 2.4 Local Command Type

```typescript
export type LocalCommand = {
  type: 'local'
  supportsNonInteractive: boolean
  load: () => Promise<LocalCommandModule>
}

export type LocalCommandModule = {
  call: LocalCommandCall
}

export type LocalCommandCall = (
  args: string,
  context: LocalJSXCommandContext,
) => Promise<LocalCommandResult>
```

### 2.5 Local JSX Command Type

```typescript
export type LocalJSXCommand = {
  type: 'local-jsx'
  /**
   * Lazy-load the command implementation.
   * Returns a module with a call() function.
   * This defers loading heavy dependencies until the command is invoked.
   */
  load: () => Promise<LocalJSXCommandModule>
}

export type LocalJSXCommandModule = {
  call: LocalJSXCommandCall
}

export type LocalJSXCommandCall = (
  onDone: LocalJSXCommandOnDone,
  context: ToolUseContext & LocalJSXCommandContext,
  args: string,
) => Promise<React.ReactNode>
```

### 2.6 Command Result Types

```typescript
export type LocalCommandResult =
  | { type: 'text'; value: string }
  | {
      type: 'compact'
      compactionResult: CompactionResult
      displayText?: string
    }
  | { type: 'skip' } // Skip messages

export type CommandResultDisplay = 'skip' | 'system' | 'user'

export type LocalJSXCommandOnDone = (
  result?: string,
  options?: {
    display?: CommandResultDisplay
    shouldQuery?: boolean
    metaMessages?: string[]
    nextInput?: string
    submitNextInput?: boolean
  },
) => void
```

### 2.7 Union Command Type

```typescript
export type Command = CommandBase &
  (PromptCommand | LocalCommand | LocalJSXCommand)

/** Resolves the user-visible name, falling back to `cmd.name` when not overridden. */
export function getCommandName(cmd: CommandBase): string {
  return cmd.userFacingName?.() ?? cmd.name
}

/** Resolves whether the command is enabled, defaulting to true. */
export function isCommandEnabled(cmd: CommandBase): boolean {
  return cmd.isEnabled?.() ?? true
}
```

---

## 3. Command Categorization

The commands module organizes commands into several categories based on their functionality:

### 3.1 Session Management Commands

| Command | File | Type | Description |
|---------|------|------|-------------|
| `/clear` | `commands/clear/` | local-jsx | Clear conversation history |
| `/compact` | `commands/compact/` | local | Compact conversation context |
| `/rename` | `commands/rename/` | local-jsx | Rename current session |
| `/resume` | `commands/resume/` | local-jsx | Resume a previous session |
| `/rewind` | `commands/rewind/` | local | Open message selector to rewind |
| `/exit` | `commands/exit/` | local-jsx | Exit the REPL |
| `/branch` | `commands/branch/` | local-jsx | Branch current conversation |

### 3.2 Configuration Commands

| Command | File | Type | Description |
|---------|------|------|-------------|
| `/config` | `commands/config/` | local-jsx | Open configuration settings |
| `/settings` | `commands/config/` | local-jsx | Alias for /config |
| `/hooks` | `commands/hooks/` | local-jsx | Manage hooks configuration |
| `/privacy-settings` | `commands/privacy-settings/` | local-jsx | Privacy and data settings |
| `/sandbox-toggle` | `commands/sandbox-toggle/` | local-jsx | Toggle sandbox mode |
| `/theme` | `commands/theme/` | local-jsx | Change UI theme |
| `/color` | `commands/color/` | local-jsx | Change color scheme |
| `/output-style` | `commands/output-style/` | local-jsx | Configure output format |
| `/keybindings` | `commands/keybindings/` | local-jsx | View/edit keybindings |
| `/model` | `commands/model/` | local-jsx | Select AI model |
| `/permissions` | `commands/permissions/` | local-jsx | Manage tool permissions |

### 3.3 Plugin and Extension Commands

| Command | File | Type | Description |
|---------|------|------|-------------|
| `/plugin` | `commands/plugin/` | local-jsx | Manage plugins (aliases: plugins, marketplace) |
| `/skills` | `commands/skills/` | local-jsx | List and manage skills |
| `/mcp` | `commands/mcp/` | local-jsx | Manage MCP servers |
| `/reload-plugins` | `commands/reload-plugins/` | local-jsx | Reload all plugins |
| `/agents` | `commands/agents/` | local-jsx | Manage agent configurations |

### 3.4 Information and Diagnostic Commands

| Command | File | Type | Description |
|---------|------|------|-------------|
| `/help` | `commands/help/` | local-jsx | Show help and available commands |
| `/status` | `commands/status/` | local-jsx | Show session status |
| `/stats` | `commands/stats/` | local-jsx | Show usage statistics |
| `/cost` | `commands/cost/` | local | Show API cost tracking |
| `/usage` | `commands/usage/` | local-jsx | Show plan usage limits |
| `/doctor` | `commands/doctor/` | local-jsx | Run diagnostic checks |
| `/version` | `commands/version.ts` | prompt | Show version information |
| `/insights` | `commands/insights.ts` | prompt | Generate session analysis report |
| `/feedback` | `commands/feedback/` | local-jsx | Submit feedback |

### 3.5 File and Context Commands

| Command | File | Type | Description |
|---------|------|------|-------------|
| `/files` | `commands/files/` | local | List files in context |
| `/add-dir` | `commands/add-dir/` | local-jsx | Add working directory |
| `/context` | `commands/context/` | local-jsx | Show context information |
| `/diff` | `commands/diff/` | local-jsx | Show file differences |
| `/memory` | `commands/memory/` | local-jsx | Edit Claude memory files |

### 3.6 Authentication Commands

| Command | File | Type | Description |
|---------|------|------|-------------|
| `/login` | `commands/login/` | local-jsx | Sign in to Anthropic account |
| `/logout` | `commands/logout/` | local-jsx | Sign out of account |

### 3.7 Git and Version Control Commands

| Command | File | Type | Description |
|---------|------|------|-------------|
| `/review` | `commands/review.ts` | prompt | Review a pull request |
| `/ultrareview` | `commands/review/` | local-jsx | Deep code review (remote) |
| `/security-review` | `commands/security-review.ts` | prompt | Security-focused code review |
| `/commit` | `commands/commit.ts` | prompt | Commit changes |
| `/commit-push-pr` | `commands/commit-push-pr.ts` | prompt | Commit, push, and create PR |
| `/branch` | `commands/branch/` | local-jsx | Git branch operations |
| `/pr_comments` | `commands/pr_comments/` | local-jsx | Handle PR comments |

### 3.8 Task and Planning Commands

| Command | File | Type | Description |
|---------|------|------|-------------|
| `/plan` | `commands/plan/` | local-jsx | Enable plan mode or view plan |
| `/ultraplan` | `commands/ultraplan.tsx` | local-jsx | Remote multi-agent planning |
| `/tasks` | `commands/tasks/` | local-jsx | Manage tasks |
| `/effort` | `commands/effort/` | local-jsx | Set effort level |
| `/passes` | `commands/passes/` | local-jsx | Configure iteration passes |

### 3.9 Special Feature Commands

| Command | File | Type | Description |
|---------|------|------|-------------|
| `/init` | `commands/init.ts` | prompt | Initialize CLAUDE.md file |
| `/ide` | `commands/ide/` | local-jsx | IDE integration |
| `/desktop` | `commands/desktop/` | local-jsx | Desktop app features |
| `/mobile` | `commands/mobile/` | local-jsx | Mobile features |
| `/chrome` | `commands/chrome/` | local-jsx | Chrome integration |
| `/vim` | `commands/vim/` | local-jsx | Vim mode toggle |

### 3.10 Advanced/Experimental Commands

| Command | File | Type | Description |
|---------|------|------|-------------|
| `/bridge` | `commands/bridge/` | local-jsx | Bridge mode (feature-gated) |
| `/voice` | `commands/voice/` | local-jsx | Voice mode (feature-gated) |
| `/proactive` | `commands/proactive/` | local-jsx | Proactive suggestions |
| `/brief` | `commands/brief/` | local-jsx | Brief mode |
| `/assistant` | `commands/assistant/` | local-jsx | Assistant mode |
| `/fork` | `commands/fork/` | local-jsx | Fork subagent |
| `/torch` | `commands/torch/` | local-jsx | Torch feature |

### 3.11 Remote and Session Commands

| Command | File | Type | Description |
|---------|------|------|-------------|
| `/session` | `commands/session/` | local-jsx | Show remote session URL |
| `/remote-env` | `commands/remote-env/` | local-jsx | Remote environment config |
| `/export` | `commands/export/` | local-jsx | Export session data |
| `/tag` | `commands/tag/` | local-jsx | Tag sessions |

### 3.12 Utility Commands

| Command | File | Type | Description |
|---------|------|------|-------------|
| `/copy` | `commands/copy/` | local-jsx | Copy content |
| `/fast` | `commands/fast/` | local-jsx | Fast mode toggle |
| `/thinkback` | `commands/thinkback/` | local-jsx | Review previous thoughts |
| `/thinkback-play` | `commands/thinkback-play/` | local-jsx | Playback thoughts |
| `/stickers` | `commands/stickers/` | local-jsx | Sticker reactions |
| `/btw` | `commands/btw/` | local-jsx | By the way notes |
| `/advisor` | `commands/advisor.ts` | prompt | Get advisor suggestions |

---

## 4. Command Execution Pipeline

### 4.1 Entry Point: processSlashCommand

**File:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/utils/processUserInput/processSlashCommand.tsx`

```typescript
export async function processSlashCommand(
  inputString: string,
  precedingInputBlocks: ContentBlockParam[],
  imageContentBlocks: ContentBlockParam[],
  attachmentMessages: AttachmentMessage[],
  context: ProcessUserInputContext,
  setToolJSX: SetToolJSXFn,
  uuid?: string,
  isAlreadyProcessing?: boolean,
  canUseTool?: CanUseToolFn,
): Promise<ProcessUserInputBaseResult> {
  // Step 1: Parse the slash command
  const parsed = parseSlashCommand(inputString)
  if (!parsed) {
    logEvent('tengu_input_slash_missing', {})
    const errorMessage = 'Commands are in the form `/command [args]`'
    return {
      messages: [createSyntheticUserCaveatMessage(), ...attachmentMessages, createUserMessage({
        content: prepareUserContent({
          inputString: errorMessage,
          precedingInputBlocks
        })
      })],
      shouldQuery: false,
      resultText: errorMessage
    }
  }
  
  const {
    commandName,
    args: parsedArgs,
    isMcp
  } = parsed
  
  // Step 2: Check if it's a real command
  if (!hasCommand(commandName, context.options.commands)) {
    // Check if this looks like a command name vs a file path
    if (looksLikeCommand(commandName) && !isFilePath) {
      logEvent('tengu_input_slash_invalid', {
        input: commandName as AnalyticsMetadata_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS
      })
      const unknownMessage = `Unknown skill: ${commandName}`
      return {
        messages: [createSyntheticUserCaveatMessage(), ...attachmentMessages, createUserMessage({
          content: prepareUserContent({
            inputString: unknownMessage,
            precedingInputBlocks
          })
        }),
        ...(parsedArgs ? [createSystemMessage(`Args from unknown skill: ${parsedArgs}`, 'warning')] : [])],
        shouldQuery: false,
        resultText: unknownMessage
      }
    }
    // Not a command - treat as regular prompt
    return {
      messages: [createUserMessage({
        content: prepareUserContent({
          inputString,
          precedingInputBlocks
        }),
        uuid: uuid
      }), ...attachmentMessages],
      shouldQuery: true
    }
  }
  
  // Step 3: Get messages for the slash command
  const result = await getMessagesForSlashCommand(
    commandName,
    parsedArgs,
    setToolJSX,
    context,
    precedingInputBlocks,
    imageContentBlocks,
    isAlreadyProcessing,
    canUseTool,
    uuid
  )
  
  return result
}
```

### 4.2 getMessagesForSlashCommand Implementation

```typescript
async function getMessagesForSlashCommand(
  commandName: string,
  args: string,
  setToolJSX: SetToolJSXFn,
  context: ProcessUserInputContext,
  precedingInputBlocks: ContentBlockParam[],
  imageContentBlocks: ContentBlockParam[],
  isAlreadyProcessing: boolean,
  canUseTool: CanUseToolFn | undefined,
  uuid?: string,
): Promise<SlashCommandResult> {
  // Find and validate the command
  const command = findCommand(commandName, context.options.commands)
  if (!command) {
    throw new MalformedCommandError(`Unknown command: ${commandName}`)
  }
  
  // Handle different command types
  switch (command.type) {
    case 'prompt':
      return handlePromptCommand(command, args, context, precedingInputBlocks)
    case 'local':
      return handleLocalCommand(command, args, context, setToolJSX, canUseTool)
    case 'local-jsx':
      return handleLocalJSXCommand(command, args, context, setToolJSX, canUseTool)
  }
}
```

### 4.3 Forked Command Execution

```typescript
/**
 * Executes a slash command with context: fork in a sub-agent.
 */
async function executeForkedSlashCommand(
  command: CommandBase & PromptCommand,
  args: string,
  context: ProcessUserInputContext,
  precedingInputBlocks: ContentBlockParam[],
  setToolJSX: SetToolJSXFn,
  canUseTool: CanUseToolFn,
): Promise<SlashCommandResult> {
  const agentId = createAgentId()
  const pluginMarketplace = command.pluginInfo
    ? parsePluginIdentifier(command.pluginInfo.repository).marketplace
    : undefined
  
  // Log analytics event
  logEvent('tengu_slash_command_forked', {
    command_name: command.name as AnalyticsMetadata_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS,
    invocation_trigger: 'user-slash' as AnalyticsMetadata_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS,
    ...(command.pluginInfo && {
      _PROTO_plugin_name: command.pluginInfo.pluginManifest.name as AnalyticsMetadata_I_VERIFIED_THIS_IS_PII_TAGGED,
      ...(pluginMarketplace && {
        _PROTO_marketplace_name: pluginMarketplace as AnalyticsMetadata_I_VERIFIED_THIS_IS_PII_TAGGED
      }),
      ...buildPluginCommandTelemetryFields(command.pluginInfo)
    })
  })
  
  // Prepare forked command context
  const {
    skillContent,
    modifiedGetAppState,
    baseAgent,
    promptMessages
  } = await prepareForkedCommandContext(command, args, context)
  
  // Merge skill's effort into the agent definition
  const agentDefinition = command.effort !== undefined
    ? { ...baseAgent, effort: command.effort }
    : baseAgent
  
  // Assistant mode: fire-and-forget background execution
  if (feature('KAIROS') && (await context.getAppState()).kairosEnabled) {
    const bgAbortController = createAbortController()
    const commandName = getCommandName(command)
    const spawnTimeWorkload = getWorkload()
    
    const enqueueResult = (value: string): void =>
      enqueuePendingNotification({
        value,
        mode: 'prompt',
        priority: 'later',
        isMeta: true,
        skipSlashCommands: true,
        workload: spawnTimeWorkload
      })
    
    void (async () => {
      // Wait for MCP servers to settle
      const deadline = Date.now() + MCP_SETTLE_TIMEOUT_MS
      while (Date.now() < deadline) {
        const s = context.getAppState()
        if (!s.mcp.clients.some(c => c.type === 'pending')) break
        await sleep(MCP_SETTLE_POLL_MS)
      }
      
      const freshTools = context.options.refreshTools?.() ?? context.options.tools
      
      const agentMessages: Message[] = []
      for await (const message of runAgent({
        agentDefinition,
        promptMessages,
        toolUseContext: {
          ...context,
          getAppState: modifiedGetAppState,
          abortController: bgAbortController
        },
        canUseTool,
        isAsync: true,
        querySource: 'agent:custom',
        model: command.model as ModelAlias | undefined,
        availableTools: freshTools,
        override: { agentId }
      })) {
        agentMessages.push(message)
      }
      
      const resultText = extractResultText(agentMessages, 'Command completed')
      enqueueResult(`<scheduled-task-result command="/${commandName}">\n${resultText}\n</scheduled-task-result>`)
    })().catch(err => {
      logError(err)
      enqueueResult(`<scheduled-task-result command="/${commandName}" status="failed">\n${err instanceof Error ? err.message : String(err)}\n</scheduled-task-result>`)
    })
    
    return {
      messages: [],
      shouldQuery: false,
      command
    }
  }
  
  // Synchronous execution with progress UI
  const agentMessages: Message[] = []
  const progressMessages: ProgressMessage<AgentProgress>[] = []
  const parentToolUseID = `forked-command-${command.name}`
  let toolUseCounter = 0
  
  const createProgressMessage = (message: AssistantMessage | NormalizedUserMessage): ProgressMessage<AgentProgress> => {
    toolUseCounter++
    return {
      type: 'progress',
      data: {
        message,
        type: 'agent_progress',
        prompt: skillContent,
        agentId
      },
      parentToolUseID,
      toolUseID: `${parentToolUseID}-${toolUseCounter}`,
      timestamp: new Date().toISOString(),
      uuid: randomUUID()
    }
  }
  
  const updateProgress = (): void => {
    setToolJSX({
      jsx: renderToolUseProgressMessage(progressMessages, {
        tools: context.options.tools,
        verbose: false
      }),
      shouldHidePromptInput: false,
      shouldContinueAnimation: true,
      showSpinner: true
    })
  }
  
  // Show initial state and run agent
  updateProgress()
  
  try {
    for await (const message of runAgent({
      agentDefinition,
      promptMessages,
      toolUseContext: {
        ...context,
        getAppState: modifiedGetAppState
      },
      canUseTool,
      isAsync: false,
      querySource: 'agent:custom',
      model: command.model as ModelAlias | undefined,
      availableTools: context.options.tools
    })) {
      agentMessages.push(message)
      const normalizedNew = normalizeMessages([message])
      
      if (message.type === 'assistant') {
        const contentLength = getAssistantMessageContentLength(message)
        if (contentLength > 0) {
          context.setResponseLength(len => len + contentLength)
        }
        const normalizedMsg = normalizedNew[0]
        if (normalizedMsg && normalizedMsg.type === 'assistant') {
          progressMessages.push(createProgressMessage(message))
          updateProgress()
        }
      }
      
      if (message.type === 'user') {
        const normalizedMsg = normalizedNew[0]
        if (normalizedMsg && normalizedMsg.type === 'user') {
          progressMessages.push(createProgressMessage(normalizedMsg))
          updateProgress()
        }
      }
    }
  } finally {
    setToolJSX(null)
  }
  
  let resultText = extractResultText(agentMessages, 'Command completed')
  
  // Build result messages
  const messages: UserMessage[] = [
    createUserMessage({
      content: prepareUserContent({
        inputString: `/${getCommandName(command)} ${args}`.trim(),
        precedingInputBlocks
      })
    }),
    createUserMessage({
      content: `<local-command-stdout>\n${resultText}\n</local-command-stdout>`
    })
  ]
  
  return {
    messages,
    shouldQuery: false,
    command,
    resultText
  }
}
```

---

## 5. Major Command Category Deep Dives

### 5.1 Session Management: `/compact` Command

**File:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/commands/compact/compact.ts`

The compact command is a `local` type command that summarizes conversation history to reduce context window usage.

```typescript
import { feature } from 'bun:bundle'
import chalk from 'chalk'
import { markPostCompaction } from 'src/bootstrap/state.js'
import { getSystemPrompt } from '../../constants/prompts.js'
import { getSystemContext, getUserContext } from '../../context.js'
import { getShortcutDisplay } from '../../keybindings/shortcutFormat.js'
import { notifyCompaction } from '../../services/api/promptCacheBreakDetection.js'
import {
  type CompactionResult,
  compactConversation,
  ERROR_MESSAGE_INCOMPLETE_RESPONSE,
  ERROR_MESSAGE_NOT_ENOUGH_MESSAGES,
  ERROR_MESSAGE_USER_ABORT,
  mergeHookInstructions,
} from '../../services/compact/compact.js'
import { suppressCompactWarning } from '../../services/compact/compactWarningState.js'
import { microcompactMessages } from '../../services/compact/microCompact.js'
import { runPostCompactCleanup } from '../../services/compact/postCompactCleanup.js'
import { trySessionMemoryCompaction } from '../../services/compact/sessionMemoryCompact.js'
import { setLastSummarizedMessageId } from '../../services/SessionMemory/sessionMemoryUtils.js'
import type { ToolUseContext } from '../../Tool.js'
import type { LocalCommandCall } from '../../types/command.js'
import type { Message } from '../../types/message.js'
import { hasExactErrorMessage } from '../../utils/errors.js'
import { executePreCompactHooks } from '../../utils/hooks.js'
import { logError } from '../../utils/log.js'
import { getMessagesAfterCompactBoundary } from '../../utils/messages.js'
import { getUpgradeMessage } from '../../utils/model/contextWindowUpgradeCheck.js'
import {
  buildEffectiveSystemPrompt,
  type SystemPrompt,
} from '../../utils/systemPrompt.js'

/* eslint-disable @typescript-eslint/no-require-imports */
const reactiveCompact = feature('REACTIVE_COMPACT')
  ? (require('../../services/compact/reactiveCompact.js') as typeof import('../../services/compact/reactiveCompact.js'))
  : null
/* eslint-enable @typescript-eslint/no-require-imports */

export const call: LocalCommandCall = async (args, context) => {
  const { abortController } = context
  let { messages } = context

  // REPL keeps snipped messages for UI scrollback — project so the compact
  // model doesn't summarize content that was intentionally removed.
  messages = getMessagesAfterCompactBoundary(messages)

  if (messages.length === 0) {
    throw new Error('No messages to compact')
  }

  const customInstructions = args.trim()

  try {
    // Try session memory compaction first if no custom instructions
    if (!customInstructions) {
      const sessionMemoryResult = await trySessionMemoryCompaction(
        messages,
        context.agentId,
      )
      if (sessionMemoryResult) {
        getUserContext.cache.clear?.()
        runPostCompactCleanup()
        
        if (feature('PROMPT_CACHE_BREAK_DETECTION')) {
          notifyCompaction(
            context.options.querySource ?? 'compact',
            context.agentId,
          )
        }
        markPostCompaction()
        suppressCompactWarning()

        return {
          type: 'compact',
          compactionResult: sessionMemoryResult,
          displayText: buildDisplayText(context),
        }
      }
    }

    // Reactive-only mode: route /compact through the reactive path
    if (reactiveCompact?.isReactiveOnlyMode()) {
      return await compactViaReactive(
        messages,
        context,
        customInstructions,
        reactiveCompact,
      )
    }

    // Fall back to traditional compaction
    // Run microcompact first to reduce tokens before summarization
    const microcompactResult = await microcompactMessages(messages, context)
    const messagesForCompact = microcompactResult.messages

    const result = await compactConversation(
      messagesForCompact,
      context,
      await getCacheSharingParams(context, messagesForCompact),
      false,
      customInstructions,
      false,
    )

    setLastSummarizedMessageId(undefined)
    suppressCompactWarning()
    getUserContext.cache.clear?.()
    runPostCompactCleanup()

    return {
      type: 'compact',
      compactionResult: result,
      displayText: buildDisplayText(context, result.userDisplayMessage),
    }
  } catch (error) {
    if (abortController.signal.aborted) {
      throw new Error('Compaction canceled.')
    } else if (hasExactErrorMessage(error, ERROR_MESSAGE_NOT_ENOUGH_MESSAGES)) {
      throw new Error(ERROR_MESSAGE_NOT_ENOUGH_MESSAGES)
    } else if (hasExactErrorMessage(error, ERROR_MESSAGE_INCOMPLETE_RESPONSE)) {
      throw new Error(ERROR_MESSAGE_INCOMPLETE_RESPONSE)
    } else {
      logError(error)
      throw new Error(`Error during compaction: ${error}`)
    }
  }
}

async function compactViaReactive(
  messages: Message[],
  context: ToolUseContext,
  customInstructions: string,
  reactive: NonNullable<typeof reactiveCompact>,
): Promise<{
  type: 'compact'
  compactionResult: CompactionResult
  displayText: string
}> {
  context.onCompactProgress?.({
    type: 'hooks_start',
    hookType: 'pre_compact',
  })
  context.setSDKStatus?.('compacting')

  try {
    // Hooks and cache-param build are independent — run concurrently
    const [hookResult, cacheSafeParams] = await Promise.all([
      executePreCompactHooks(
        { trigger: 'manual', customInstructions: customInstructions || null },
        context.abortController.signal,
      ),
      getCacheSharingParams(context, messages),
    ])
    const mergedInstructions = mergeHookInstructions(
      customInstructions,
      hookResult.newCustomInstructions,
    )

    context.setStreamMode?.('requesting')
    context.setResponseLength?.(() => 0)
    context.onCompactProgress?.({ type: 'compact_start' })

    const outcome = await reactive.reactiveCompactOnPromptTooLong(
      messages,
      cacheSafeParams,
      { customInstructions: mergedInstructions, trigger: 'manual' },
    )

    if (!outcome.ok) {
      switch (outcome.reason) {
        case 'too_few_groups':
          throw new Error(ERROR_MESSAGE_NOT_ENOUGH_MESSAGES)
        case 'aborted':
          throw new Error(ERROR_MESSAGE_USER_ABORT)
        case 'exhausted':
        case 'error':
        case 'media_unstrippable':
          throw new Error(ERROR_MESSAGE_INCOMPLETE_RESPONSE)
      }
    }

    setLastSummarizedMessageId(undefined)
    runPostCompactCleanup()
    suppressCompactWarning()
    getUserContext.cache.clear?.()

    const combinedMessage =
      [hookResult.userDisplayMessage, outcome.result.userDisplayMessage]
        .filter(Boolean)
        .join('\n') || undefined

    return {
      type: 'compact',
      compactionResult: {
        ...outcome.result,
        userDisplayMessage: combinedMessage,
      },
      displayText: buildDisplayText(context, combinedMessage),
    }
  } finally {
    context.setStreamMode?.('requesting')
    context.setResponseLength?.(() => 0)
    context.onCompactProgress?.({ type: 'compact_end' })
    context.setSDKStatus?.(null)
  }
}

function buildDisplayText(
  context: ToolUseContext,
  userDisplayMessage?: string,
): string {
  const upgradeMessage = getUpgradeMessage('tip')
  const expandShortcut = getShortcutDisplay(
    'app:toggleTranscript',
    'Global',
    'ctrl+o',
  )
  const dimmed = [
    ...(context.options.verbose
      ? []
      : [`(${expandShortcut} to see full summary)`]),
    ...(userDisplayMessage ? [userDisplayMessage] : []),
    ...(upgradeMessage ? [upgradeMessage] : []),
  ]
  return chalk.dim('Compacted ' + dimmed.join('\n'))
}

async function getCacheSharingParams(
  context: ToolUseContext,
  forkContextMessages: Message[],
): Promise<{
  systemPrompt: SystemPrompt
  userContext: { [k: string]: string }
  systemContext: { [k: string]: string }
  toolUseContext: ToolUseContext
  forkContextMessages: Message[]
}> {
  const appState = context.getAppState()
  const defaultSysPrompt = await getSystemPrompt(
    context.options.tools,
    context.options.mainLoopModel,
    Array.from(
      appState.toolPermissionContext.additionalWorkingDirectories.keys(),
    ),
    context.options.mcpClients,
  )
  const systemPrompt = buildEffectiveSystemPrompt({
    mainThreadAgentDefinition: undefined,
    toolUseContext: context,
    customSystemPrompt: context.options.customSystemPrompt,
    defaultSystemPrompt: defaultSysPrompt,
    appendSystemPrompt: context.options.appendSystemPrompt,
  })
  const [userContext, systemContext] = await Promise.all([
    getUserContext(),
    getSystemContext(),
  ])
  return {
    systemPrompt,
    userContext,
    systemContext,
    toolUseContext: context,
    forkContextMessages,
  }
}
```

### 5.2 MCP Management: `/mcp` Command

**File:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/commands/mcp/mcp.tsx`

```typescript
import { randomUUID, type UUID } from 'crypto'
import { mkdir, readFile, writeFile } from 'fs/promises'
import { getOriginalCwd, getSessionId } from '../../bootstrap/state.js'
import type { LocalJSXCommandContext } from '../../commands.js'
import { logEvent } from '../../services/analytics/index.js'
import type { LocalJSXCommandOnDone } from '../../types/command.js'
import type {
  ContentReplacementEntry,
  Entry,
  LogOption,
  SerializedMessage,
  TranscriptMessage,
} from '../../types/logs.js'
import { parseJSONL } from '../../utils/json.js'
import {
  getProjectDir,
  getTranscriptPath,
  getTranscriptPathForSession,
  isTranscriptMessage,
  saveCustomTitle,
  searchSessionsByCustomTitle,
} from '../../utils/sessionStorage.js'
import { jsonStringify } from '../../utils/slowOperations.js'
import { escapeRegExp } from '../../utils/stringUtils.js'

type TranscriptEntry = TranscriptMessage & {
  forkedFrom?: {
    sessionId: string
    messageUuid: UUID
  }
}

// TODO: This is a hack to get the context value from toggleMcpServer
function MCPToggle({
  action,
  target,
  onComplete
}: {
  action: 'enable' | 'disable'
  target: string
  onComplete: (result: string) => void
}): null {
  const mcpClients = useAppState(s => s.mcp.clients)
  const toggleMcpServer = useMcpToggleEnabled()
  const didRun = useRef(false)

  useEffect(() => {
    if (didRun.current) return
    didRun.current = true

    const isEnabling = action === 'enable'
    const clients = mcpClients.filter(c => c.name !== 'ide')
    const toToggle =
      target === 'all'
        ? clients.filter(c =>
            isEnabling ? c.type === 'disabled' : c.type !== 'disabled',
          )
        : clients.filter(c => c.name === target)

    if (toToggle.length === 0) {
      onComplete(
        target === 'all'
          ? `All MCP servers are already ${isEnabling ? 'enabled' : 'disabled'}`
          : `MCP server "${target}" not found`,
      )
      return
    }

    for (const s of toToggle) {
      void toggleMcpServer(s.name)
    }

    onComplete(
      target === 'all'
        ? `${isEnabling ? 'Enabled' : 'Disabled'} ${toToggle.length} MCP server(s)`
        : `MCP server "${target}" ${isEnabling ? 'enabled' : 'disabled'}`,
    )
  }, [action, target, mcpClients, toggleMcpServer, onComplete])

  return null
}

export async function call(
  onDone: LocalJSXCommandOnDone,
  _context: unknown,
  args?: string,
): Promise<React.ReactNode> {
  if (args) {
    const parts = args.trim().split(/\s+/)

    // Allow /mcp no-redirect to bypass the redirect for testing
    if (parts[0] === 'no-redirect') {
      return <MCPSettings onComplete={onDone} />
    }
    
    if (parts[0] === 'reconnect' && parts[1]) {
      return (
        <MCPReconnect
          serverName={parts.slice(1).join(' ')}
          onComplete={onDone}
        />
      )
    }
    
    if (parts[0] === 'enable' || parts[0] === 'disable') {
      return (
        <MCPToggle
          action={parts[0]}
          target={parts.length > 1 ? parts.slice(1).join(' ') : 'all'}
          onComplete={onDone}
        />
      )
    }
  }

  // Redirect base /mcp command to /plugins installed tab for ant users
  if ("external" === 'ant') {
    return (
      <PluginSettings
        onComplete={onDone}
        args="manage"
        showMcpRedirectMessage
      />
    )
  }
  
  return <MCPSettings onComplete={onDone} />
}
```

### 5.3 Session Branching: `/branch` Command

**File:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/commands/branch/branch.ts`

```typescript
import type { UUID } from 'crypto'
import { getSessionId } from '../../bootstrap/state.js'
import {
  getBridgeBaseUrlOverride,
  getBridgeTokenOverride,
} from '../../bridge/bridgeConfig.js'
import type { ToolUseContext } from '../../Tool.js'
import type {
  LocalJSXCommandContext,
  LocalJSXCommandOnDone,
} from '../../types/command.js'
import { getMessagesAfterCompactBoundary } from '../../utils/messages.js'
import {
  getTranscriptPath,
  saveAgentName,
  saveCustomTitle,
} from '../../utils/sessionStorage.js'
import { isTeammate } from '../../utils/teammate.js'
import { generateSessionName } from './generateSessionName.js'

/**
 * Derive a single-line title base from the first user message.
 * Collapses whitespace — multiline first messages (pasted stacks, code)
 * otherwise flow into the saved title and break the resume hint.
 */
export function deriveFirstPrompt(
  firstUserMessage: Extract<SerializedMessage, { type: 'user' }> | undefined,
): string {
  const content = firstUserMessage?.message?.content
  if (!content) return 'Branched conversation'
  const raw =
    typeof content === 'string'
      ? content
      : content.find(
          (block): block is { type: 'text'; text: string } =>
            block.type === 'text',
        )?.text
  if (!raw) return 'Branched conversation'
  return (
    raw.replace(/\s+/g, ' ').trim().slice(0, 100) || 'Branched conversation'
  )
}

/**
 * Creates a fork of the current conversation by copying from the transcript file.
 * Preserves all original metadata (timestamps, gitBranch, etc.) while updating
 * sessionId and adding forkedFrom traceability.
 */
async function createFork(customTitle?: string): Promise<{
  sessionId: UUID
  title: string | undefined
  forkPath: string
  serializedMessages: SerializedMessage[]
  contentReplacementRecords: ContentReplacementEntry['replacements']
}> {
  const forkSessionId = randomUUID() as UUID
  const originalSessionId = getSessionId()
  const projectDir = getProjectDir(getOriginalCwd())
  const forkSessionPath = getTranscriptPathForSession(forkSessionId)
  const currentTranscriptPath = getTranscriptPath()

  // Ensure project directory exists
  await mkdir(projectDir, { recursive: true, mode: 0o700 })

  // Read current transcript file
  let transcriptContent: Buffer
  try {
    transcriptContent = await readFile(currentTranscriptPath)
  } catch {
    throw new Error('No conversation to branch')
  }

  if (transcriptContent.length === 0) {
    throw new Error('No conversation to branch')
  }

  // Parse all transcript entries
  const entries = parseJSONL<Entry>(transcriptContent)

  // Filter to only main conversation messages
  const mainConversationEntries = entries.filter(
    (entry): entry is TranscriptMessage =>
      isTranscriptMessage(entry) && !entry.isSidechain,
  )

  // Content-replacement entries for the original session
  const contentReplacementRecords = entries
    .filter(
      (entry): entry is ContentReplacementEntry =>
        entry.type === 'content-replacement' &&
        entry.sessionId === originalSessionId,
    )
    .flatMap(entry => entry.replacements)

  if (mainConversationEntries.length === 0) {
    throw new Error('No messages to branch')
  }

  // Build forked entries with new sessionId
  let parentUuid: UUID | null = null
  const lines: string[] = []
  const serializedMessages: SerializedMessage[] = []

  for (const entry of mainConversationEntries) {
    const forkedEntry: TranscriptEntry = {
      ...entry,
      sessionId: forkSessionId,
      parentUuid,
      isSidechain: false,
      forkedFrom: {
        sessionId: originalSessionId,
        messageUuid: entry.uuid,
      },
    }

    const serialized: SerializedMessage = {
      ...entry,
      sessionId: forkSessionId,
    }

    serializedMessages.push(serialized)
    lines.push(jsonStringify(forkedEntry))
    if (entry.type !== 'progress') {
      parentUuid = entry.uuid
    }
  }

  // Append content-replacement entry (if any)
  if (contentReplacementRecords.length > 0) {
    const forkedReplacementEntry: ContentReplacementEntry = {
      type: 'content-replacement',
      sessionId: forkSessionId,
      replacements: contentReplacementRecords,
    }
    lines.push(jsonStringify(forkedReplacementEntry))
  }

  // Write the fork session file
  await writeFile(forkSessionPath, lines.join('\n') + '\n', {
    encoding: 'utf8',
    mode: 0o600,
  })

  return {
    sessionId: forkSessionId,
    title: customTitle,
    forkPath: forkSessionPath,
    serializedMessages,
    contentReplacementRecords,
  }
}

/**
 * Generates a unique fork name by checking for collisions.
 */
async function getUniqueForkName(baseName: string): Promise<string> {
  const candidateName = `${baseName} (Branch)`

  const existingWithExactName = await searchSessionsByCustomTitle(
    candidateName,
    { exact: true },
  )

  if (existingWithExactName.length === 0) {
    return candidateName
  }

  // Name collision - find unique numbered suffix
  const existingForks = await searchSessionsByCustomTitle(`${baseName} (Branch`)

  const usedNumbers = new Set<number>([1])
  const forkNumberPattern = new RegExp(
    `^${escapeRegExp(baseName)} \\(Branch(?: (\\d+))?\\)$`,
  )

  for (const session of existingForks) {
    const match = session.customTitle?.match(forkNumberPattern)
    if (match) {
      if (match[1]) {
        usedNumbers.add(parseInt(match[1], 10))
      } else {
        usedNumbers.add(1)
      }
    }
  }

  let nextNumber = 2
  while (usedNumbers.has(nextNumber)) {
    nextNumber++
  }

  return `${baseName} (Branch ${nextNumber})`
}

export async function call(
  onDone: LocalJSXCommandOnDone,
  context: LocalJSXCommandContext,
  args: string,
): Promise<React.ReactNode> {
  const customTitle = args?.trim() || undefined
  const originalSessionId = getSessionId()

  try {
    const {
      sessionId,
      title,
      forkPath,
      serializedMessages,
      contentReplacementRecords,
    } = await createFork(customTitle)

    const now = new Date()
    const firstPrompt = deriveFirstPrompt(
      serializedMessages.find(m => m.type === 'user'),
    )

    const baseName = title ?? firstPrompt
    const effectiveTitle = await getUniqueForkName(baseName)
    await saveCustomTitle(sessionId, effectiveTitle, forkPath)

    logEvent('tengu_conversation_forked', {
      message_count: serializedMessages.length,
      has_custom_title: !!title,
    })

    const forkLog: LogOption = {
      date: now.toISOString().split('T')[0]!,
      messages: serializedMessages,
      fullPath: forkPath,
      value: now.getTime(),
      created: now,
      modified: now,
      firstPrompt,
      messageCount: serializedMessages.length,
      isSidechain: false,
      sessionId,
      customTitle: effectiveTitle,
      contentReplacements: contentReplacementRecords,
    }

    const titleInfo = title ? ` "${title}"` : ''
    const resumeHint = `\nTo resume the original: claude -r ${originalSessionId}`
    const successMessage = `Branched conversation${titleInfo}. You are now in the branch.${resumeHint}`

    if (context.resume) {
      await context.resume(sessionId, forkLog, 'fork')
      onDone(successMessage, { display: 'system' })
    } else {
      onDone(
        `Branched conversation${titleInfo}. Resume with: /resume ${sessionId}`,
      )
    }

    return null
  } catch (error) {
    const message =
      error instanceof Error ? error.message : 'Unknown error occurred'
    onDone(`Failed to branch conversation: ${message}`)
    return null
  }
}
```

### 5.4 Session Rename: `/rename` Command

**File:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/commands/rename/rename.ts`

```typescript
import type { UUID } from 'crypto'
import { getSessionId } from '../../bootstrap/state.js'
import {
  getBridgeBaseUrlOverride,
  getBridgeTokenOverride,
} from '../../bridge/bridgeConfig.js'
import type { ToolUseContext } from '../../Tool.js'
import type {
  LocalJSXCommandContext,
  LocalJSXCommandOnDone,
} from '../../types/command.js'
import { getMessagesAfterCompactBoundary } from '../../utils/messages.js'
import {
  getTranscriptPath,
  saveAgentName,
  saveCustomTitle,
} from '../../utils/sessionStorage.js'
import { isTeammate } from '../../utils/teammate.js'
import { generateSessionName } from './generateSessionName.js'

export async function call(
  onDone: LocalJSXCommandOnDone,
  context: ToolUseContext & LocalJSXCommandContext,
  args: string,
): Promise<null> {
  // Prevent teammates from renaming - their names are set by team leader
  if (isTeammate()) {
    onDone(
      'Cannot rename: This session is a swarm teammate. Teammate names are set by the team leader.',
      { display: 'system' },
    )
    return null
  }

  let newName: string
  if (!args || args.trim() === '') {
    const generated = await generateSessionName(
      getMessagesAfterCompactBoundary(context.messages),
      context.abortController.signal,
    )
    if (!generated) {
      onDone(
        'Could not generate a name: no conversation context yet. Usage: /rename <name>',
        { display: 'system' },
      )
      return null
    }
    newName = generated
  } else {
    newName = args.trim()
  }

  const sessionId = getSessionId() as UUID
  const fullPath = getTranscriptPath()

  // Always save the custom title (session name)
  await saveCustomTitle(sessionId, newName, fullPath)

  // Sync title to bridge session on claude.ai/code (best-effort, non-blocking)
  const appState = context.getAppState()
  const bridgeSessionId = appState.replBridgeSessionId
  if (bridgeSessionId) {
    const tokenOverride = getBridgeTokenOverride()
    void import('../../bridge/createSession.js').then(
      ({ updateBridgeSessionTitle }) =>
        updateBridgeSessionTitle(bridgeSessionId, newName, {
          baseUrl: getBridgeBaseUrlOverride(),
          getAccessToken: tokenOverride ? () => tokenOverride : undefined,
        }).catch(() => {}),
    )
  }

  // Also persist as the session's agent name for prompt-bar display
  await saveAgentName(sessionId, newName, fullPath)
  context.setAppState(prev => ({
    ...prev,
    standaloneAgentContext: {
      ...prev.standaloneAgentContext,
      name: newName,
    },
  }))

  onDone(`Session renamed to: ${newName}`, { display: 'system' })
  return null
}
```

### 5.5 Review Commands: `/review` and `/ultrareview`

**File:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/commands/review.ts`

```typescript
import type { ContentBlockParam } from '@anthropic-ai/sdk/resources/messages.js'
import type { Command } from '../commands.js'
import { isUltrareviewEnabled } from './review/ultrareviewEnabled.js'

const CCR_TERMS_URL = 'https://code.claude.com/docs/en/claude-code-on-the-web'

const LOCAL_REVIEW_PROMPT = (args: string) => `
      You are an expert code reviewer. Follow these steps:

      1. If no PR number is provided in the args, run \`gh pr list\` to show open PRs
      2. If a PR number is provided, run \`gh pr view <number>\` to get PR details
      3. Run \`gh pr diff <number>\` to get the diff
      4. Analyze the changes and provide a thorough code review that includes:
         - Overview of what the PR does
         - Analysis of code quality and style
         - Specific suggestions for improvements
         - Any potential issues or risks

      Keep your review concise but thorough. Focus on:
      - Code correctness
      - Following project conventions
      - Performance implications
      - Test coverage
      - Security considerations

      Format your review with clear sections and bullet points.

      PR number: ${args}
    `

const review: Command = {
  type: 'prompt',
  name: 'review',
  description: 'Review a pull request',
  progressMessage: 'reviewing pull request',
  contentLength: 0,
  source: 'builtin',
  async getPromptForCommand(args): Promise<ContentBlockParam[]> {
    return [{ type: 'text', text: LOCAL_REVIEW_PROMPT(args) }]
  },
}

// /ultrareview is the ONLY entry point to the remote bughunter path
const ultrareview: Command = {
  type: 'local-jsx',
  name: 'ultrareview',
  description: `~10–20 min · Finds and verifies bugs in your branch. Runs in Claude Code on the web. See ${CCR_TERMS_URL}`,
  isEnabled: () => isUltrareviewEnabled(),
  load: () => import('./review/ultrareviewCommand.js'),
}

export default review
export { ultrareview }
```

### 5.6 Security Review: `/security-review` Command

**File:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/commands/security-review.ts`

```typescript
import { parseFrontmatter } from '../utils/frontmatterParser.js'
import { parseSlashCommandToolsFromFrontmatter } from '../utils/markdownConfigLoader.js'
import { executeShellCommandsInPrompt } from '../utils/promptShellExecution.js'
import { createMovedToPluginCommand } from './createMovedToPluginCommand.js'

const SECURITY_REVIEW_MARKDOWN = `---
allowed-tools: Bash(git diff:*), Bash(git status:*), Bash(git log:*), Bash(git show:*), Bash(git remote show:*), Read, Glob, Grep, LS, Task
description: Complete a security review of the pending changes on the current branch
---

You are a senior security engineer conducting a focused security review of the changes on this branch.

GIT STATUS:

\`\`\`
!\`git status\`
\`\`\`

FILES MODIFIED:

\`\`\`
!\`git diff --name-only origin/HEAD...\`
\`\`\`

COMMITS:

\`\`\`
!\`git log --no-decorate origin/HEAD...\`
\`\`\`

DIFF CONTENT:

\`\`\`
!\`git diff origin/HEAD...\`
\`\`\`

Review the complete diff above. This contains all code changes in the PR.

OBJECTIVE:
Perform a security-focused code review to identify HIGH-CONFIDENCE security vulnerabilities that could have real exploitation potential. This is not a general code review - focus ONLY on security implications newly added by this PR. Do not comment on existing security concerns.

CRITICAL INSTRUCTIONS:
1. MINIMIZE FALSE POSITIVES: Only flag issues where you're >80% confident of actual exploitability
2. AVOID NOISE: Skip theoretical issues, style concerns, or low-impact findings
3. FOCUS ON IMPACT: Prioritize vulnerabilities that could lead to unauthorized access, data breaches, or system compromise
4. EXCLUSIONS: Do NOT report the following issue types:
   - Denial of Service (DOS) vulnerabilities, even if they allow service disruption
   - Secrets or sensitive data stored on disk (these are handled by other processes)
   - Rate limiting or resource exhaustion issues

SECURITY CATEGORIES TO EXAMINE:

**Input Validation Vulnerabilities:**
- SQL injection via unsanitized user input
- Command injection in system calls or subprocesses
- XXE injection in XML parsing
- Template injection in templating engines
- NoSQL injection in database queries
- Path traversal in file operations

**Authentication & Authorization Issues:**
- Authentication bypass logic
- Privilege escalation paths
- Session management flaws
- JWT token vulnerabilities
- Authorization logic bypasses

**Crypto & Secrets Management:**
- Hardcoded API keys, passwords, or tokens
- Weak cryptographic algorithms or implementations
- Improper key storage or management
- Cryptographic randomness issues
- Certificate validation bypasses

**Injection & Code Execution:**
- Remote code execution via deserialization
- Pickle injection in Python
- YAML deserialization vulnerabilities
- Eval injection in dynamic code execution
- XSS vulnerabilities in web applications (reflected, stored, DOM-based)

**Data Exposure:**
- Sensitive data logging or storage
- PII handling violations
- API endpoint data leakage
- Debug information exposure

SEVERITY GUIDELINES:
- **HIGH**: Directly exploitable vulnerabilities leading to RCE, data breach, or authentication bypass
- **MEDIUM**: Vulnerabilities requiring specific conditions but with significant impact
- **LOW**: Defense-in-depth issues or lower-impact vulnerabilities

CONFIDENCE SCORING:
- 0.9-1.0: Certain exploit path identified, tested if possible
- 0.8-0.9: Clear vulnerability pattern with known exploitation methods
- 0.7-0.8: Suspicious pattern requiring specific conditions to exploit
- Below 0.7: Don't report (too speculative)

START ANALYSIS:

Begin your analysis now. Do this in 3 steps:

1. Use a sub-task to identify vulnerabilities. Use the repository exploration tools to understand the codebase context, then analyze the PR changes for security implications. In the prompt for this sub-task, include all of the above.
2. Then for each vulnerability identified by the above sub-task, create a new sub-task to filter out false-positives. Launch these sub-tasks as parallel sub-tasks. In the prompt for these sub-tasks, include everything in the "FALSE POSITIVE FILTERING" instructions.
3. Filter out any vulnerabilities where the sub-task reported a confidence less than 8.

Your final reply must contain the markdown report and nothing else.`

export default createMovedToPluginCommand({
  name: 'security-review',
  description:
    'Complete a security review of the pending changes on the current branch',
  progressMessage: 'analyzing code changes for security risks',
  pluginName: 'security-review',
  pluginCommand: 'security-review',
  async getPromptWhileMarketplaceIsPrivate(_args, context) {
    // Parse frontmatter from the markdown
    const parsed = parseFrontmatter(SECURITY_REVIEW_MARKDOWN)

    // Parse allowed tools from frontmatter
    const allowedTools = parseSlashCommandToolsFromFrontmatter(
      parsed.frontmatter['allowed-tools'],
    )

    // Execute bash commands in the prompt
    const processedContent = await executeShellCommandsInPrompt(
      parsed.content,
      {
        ...context,
        getAppState() {
          const appState = context.getAppState()
          return {
            ...appState,
            toolPermissionContext: {
              ...appState.toolPermissionContext,
              alwaysAllowRules: {
                ...appState.toolPermissionContext.alwaysAllowRules,
                command: allowedTools,
              },
            },
          }
        },
      },
      'security-review',
    )

    return [
      {
        type: 'text',
        text: processedContent,
      },
    ]
  },
})
```

### 5.7 Initialization: `/init` Command

**File:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/commands/init.ts`

```typescript
import { feature } from 'bun:bundle'
import type { Command } from '../commands.js'
import { maybeMarkProjectOnboardingComplete } from '../projectOnboardingState.js'
import { isEnvTruthy } from '../utils/envUtils.js'

const OLD_INIT_PROMPT = `Please analyze this codebase and create a CLAUDE.md file, which will be given to future instances of Claude Code to operate in this repository.

What to add:
1. Commands that will be commonly used, such as how to build, lint, and run tests. Include the necessary commands to develop in this codebase, such as how to run a single test.
2. High-level code architecture and structure so that future instances can be productive more quickly. Focus on the "big picture" architecture that requires reading multiple files to understand.

Usage notes:
- If there's already a CLAUDE.md, suggest improvements to it.
- When you make the initial CLAUDE.md, do not repeat yourself and do not include obvious instructions like "Provide helpful error messages to users", "Write unit tests for all new utilities", "Never include sensitive information (API keys, tokens) in code or commits".
- Avoid listing every component or file structure that can be easily discovered.
- Don't include generic development practices.
- If there are Cursor rules (in .cursor/rules/ or .cursorrules) or Copilot rules (in .github/copilot-instructions.md), make sure to include the important parts.
- If there is a README.md, make sure to include the important parts.
- Do not make up information such as "Common Development Tasks", "Tips for Development", "Support and Documentation" unless this is expressly included in other files that you read.
- Be sure to prefix the file with the following text:

\`\`\`
# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.
\`\`\``

const NEW_INIT_PROMPT = `Set up a minimal CLAUDE.md (and optionally skills and hooks) for this repo. CLAUDE.md is loaded into every Claude Code session, so it must be concise — only include what Claude would get wrong without it.

## Phase 1: Ask what to set up

Use AskUserQuestion to find out what the user wants:

- "Which CLAUDE.md files should /init set up?"
  Options: "Project CLAUDE.md" | "Personal CLAUDE.local.md" | "Both project + personal"
  Description for project: "Team-shared instructions checked into source control — architecture, coding standards, common workflows."
  Description for personal: "Your private preferences for this project (gitignored, not shared) — your role, sandbox URLs, preferred test data, workflow quirks."

- "Also set up skills and hooks?"
  Options: "Skills + hooks" | "Skills only" | "Hooks only" | "Neither, just CLAUDE.md"
  Description for skills: "On-demand capabilities you or Claude invoke with \`/skill-name\` — good for repeatable workflows and reference knowledge."
  Description for hooks: "Deterministic shell commands that run on tool events (e.g., format after every edit). Claude can't skip them."

## Phase 2: Explore the codebase

Launch a subagent to survey the codebase, and ask it to read key files to understand the project: manifest files (package.json, Cargo.toml, pyproject.toml, go.mod, pom.xml, etc.), README, Makefile/build configs, CI config, existing CLAUDE.md, .claude/rules/, AGENTS.md, .cursor/rules or .cursorrules, .github/copilot-instructions.md, .windsurfrules, .clinerules, .mcp.json.

Detect:
- Build, test, and lint commands (especially non-standard ones)
- Languages, frameworks, and package manager
- Project structure (monorepo with workspaces, multi-module, or single project)
- Code style rules that differ from language defaults
- Non-obvious gotchas, required env vars, or workflow quirks
- Existing .claude/skills/ and .claude/rules/ directories
- Formatter configuration (prettier, biome, ruff, black, gofmt, rustfmt, or a unified format script like \`npm run format\` / \`make fmt\`)
- Git worktree usage: run \`git worktree list\` to check if this repo has multiple worktrees

Note what you could NOT figure out from code alone — these become interview questions.

## Phase 3: Fill in the gaps

Use AskUserQuestion to gather what you still need to write good CLAUDE.md files and skills. Ask only things the code can't answer.

If the user chose project CLAUDE.md or both: ask about codebase practices — non-obvious commands, gotchas, branch/PR conventions, required env setup, testing quirks. Skip things already in README or obvious from manifest files.

If the user chose personal CLAUDE.local.md or both: ask about them, not the codebase. Examples:
  - What's their role on the team?
  - How familiar are they with this codebase?
  - Do they have personal sandbox URLs, test accounts, API key paths?
  - Any communication preferences?

**Synthesize a proposal from Phase 2 findings** — e.g., format-on-edit if a formatter exists, a \`/verify\` skill if tests exist.

**Show the proposal via AskUserQuestion's \`preview\` field** — the dialog overlays your output. Structure it as:

  - \`question\`: short and plain, e.g. "Does this proposal look right?"
  - Each option gets a \`preview\` with the full proposal as markdown.

**Build the preference queue** from the accepted proposal.

## Phase 4: Write CLAUDE.md (if user chose project or both)

Write a minimal CLAUDE.md at the project root. Every line must pass this test: "Would removing this cause Claude to make mistakes?" If no, cut it.

Include:
- Build/test/lint commands Claude can't guess
- Code style rules that DIFFER from language defaults
- Testing instructions and quirks
- Repo etiquette (branch naming, PR conventions)
- Required env vars or setup steps
- Non-obvious gotchas

Exclude:
- File-by-file structure or component lists
- Standard language conventions
- Generic advice
- Long references — use \`@path/to/import\` syntax instead

Prefix the file with:

\`\`\`
# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.
\`\`\`

## Phase 5: Write CLAUDE.local.md (if user chose personal or both)

Write a minimal CLAUDE.local.md at the project root. After creating it, add \`CLAUDE.local.md\` to .gitignore.

Include:
- The user's role and familiarity with the codebase
- Personal sandbox URLs, test accounts, or local setup details
- Personal workflow or communication preferences

## Phase 6: Suggest and create skills

Create each skill at \`.claude/skills/<skill-name>/SKILL.md\`:

\`\`\`yaml
---
name: <skill-name>
description: <what the skill does and when to use it>
---

<Instructions for Claude>
\`\`\`

## Phase 7: Suggest additional optimizations

Check the environment and ask about each gap:

- **GitHub CLI**: Run \`which gh\`. If missing AND project uses GitHub, offer to install.
- **Linting**: If no lint config found, offer to set up linting.
- **Hooks**: If formatter exists and user chose hooks, offer format-on-edit hook.

## Phase 8: Summary and next steps

Recap what was set up and present a to-do list with remaining optimizations.`

const command = {
  type: 'prompt',
  name: 'init',
  get description() {
    return feature('NEW_INIT') &&
      (process.env.USER_TYPE === 'ant' ||
        isEnvTruthy(process.env.CLAUDE_CODE_NEW_INIT))
      ? 'Initialize new CLAUDE.md file(s) and optional skills/hooks with codebase documentation'
      : 'Initialize a new CLAUDE.md file with codebase documentation'
  },
  contentLength: 0,
  progressMessage: 'analyzing your codebase',
  source: 'builtin',
  async getPromptForCommand() {
    maybeMarkProjectOnboardingComplete()

    return [
      {
        type: 'text',
        text:
          feature('NEW_INIT') &&
          (process.env.USER_TYPE === 'ant' ||
            isEnvTruthy(process.env.CLAUDE_CODE_NEW_INIT))
            ? NEW_INIT_PROMPT
            : OLD_INIT_PROMPT,
      },
    ]
  },
} satisfies Command

export default command
```

### 5.8 Ultraplan: Multi-Agent Remote Planning

**File:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/commands/ultraplan.tsx`

```typescript
import { readFileSync } from 'fs'
import { REMOTE_CONTROL_DISCONNECTED_MSG } from '../bridge/types.js'
import type { Command } from '../commands.js'
import { DIAMOND_OPEN } from '../constants/figures.js'
import { getRemoteSessionUrl } from '../constants/product.js'
import { getFeatureValue_CACHED_MAY_BE_STALE } from '../services/analytics/growthbook.js'
import type { AppState } from '../state/AppStateStore.js'
import { checkRemoteAgentEligibility, formatPreconditionError, RemoteAgentTask, registerRemoteAgentTask } from '../tasks/RemoteAgentTask/RemoteAgentTask.js'
import type { LocalJSXCommandCall } from '../types/command.js'
import { logForDebugging } from '../utils/debug.js'
import { errorMessage } from '../utils/errors.js'
import { logError } from '../utils/log.js'
import { enqueuePendingNotification } from '../utils/messageQueueManager.js'
import { ALL_MODEL_CONFIGS } from '../utils/model/configs.js'
import { updateTaskState } from '../utils/task/framework.js'
import { archiveRemoteSession, teleportToRemote } from '../utils/teleport.js'
import { pollForApprovedExitPlanMode, UltraplanPollError } from '../utils/ultraplan/ccrSession.js'

const ULTRAPLAN_TIMEOUT_MS = 30 * 60 * 1000
export const CCR_TERMS_URL = 'https://code.claude.com/docs/en/claude-code-on-the-web'

// Get ultraplan model (Opus - best quality)
function getUltraplanModel(): string {
  return getFeatureValue_CACHED_MAY_BE_STALE(
    'tengu_ultraplan_model',
    ALL_MODEL_CONFIGS.opus46.firstParty
  )
}

// Build ultraplan prompt with optional seed plan
export function buildUltraplanPrompt(blurb: string, seedPlan?: string): string {
  const parts: string[] = []
  if (seedPlan) {
    parts.push('Here is a draft plan to refine:', '', seedPlan, '')
  }
  parts.push(ULTRAPLAN_INSTRUCTIONS)
  if (blurb) {
    parts.push('', blurb)
  }
  return parts.join('\n')
}

// Poll for ultraplan completion
function startDetachedPoll(
  taskId: string,
  sessionId: string,
  url: string,
  getAppState: () => AppState,
  setAppState: (f: (prev: AppState) => AppState) => void,
): void {
  const started = Date.now()
  let failed = false
  
  void (async () => {
    try {
      const {
        plan,
        rejectCount,
        executionTarget
      } = await pollForApprovedExitPlanMode(
        sessionId,
        ULTRAPLAN_TIMEOUT_MS,
        phase => {
          if (phase === 'needs_input')
            logEvent('tengu_ultraplan_awaiting_input', {})
          updateTaskState<RemoteAgentTaskState>(taskId, setAppState, t => {
            if (t.status !== 'running') return t
            const next = phase === 'running' ? undefined : phase
            return t.ultraplanPhase === next ? t : { ...t, ultraplanPhase: next }
          })
        },
        () => getAppState().tasks?.[taskId]?.status !== 'running'
      )
      
      if (executionTarget === 'remote') {
        // User chose "execute in CCR" - remote session is coding
        const task = getAppState().tasks?.[taskId]
        if (task?.status !== 'running') return
        
        updateTaskState<RemoteAgentTaskState>(taskId, setAppState, t =>
          t.status !== 'running' ? t : { ...t, status: 'completed', endTime: Date.now() }
        )
        
        setAppState(prev =>
          prev.ultraplanSessionUrl === url
            ? { ...prev, ultraplanSessionUrl: undefined }
            : prev
        )
        
        enqueuePendingNotification({
          value: [
            'Ultraplan approved — executing in Claude Code on the web.',
            `Follow along at: ${url}`,
            '',
            'Results will land as a pull request when the remote session finishes.'
          ].join('\n'),
          mode: 'task-notification'
        })
      } else {
        // Teleport: set pendingChoice so REPL mounts UltraplanChoiceDialog
        setAppState(prev => {
          const task = prev.tasks?.[taskId]
          if (!task || task.status !== 'running') return prev
          return {
            ...prev,
            ultraplanPendingChoice: { plan, sessionId, taskId }
          }
        })
      }
    } catch (e) {
      const task = getAppState().tasks?.[taskId]
      if (task?.status !== 'running') return
      
      failed = true
      setAppState(prev =>
        prev.ultraplanSessionUrl === url
          ? { ...prev, ultraplanSessionUrl: undefined }
          : prev
      )
    }
  })()
}

// Stop ultraplan session
export async function stopUltraplan(
  taskId: string,
  sessionId: string,
  setAppState: (f: (prev: AppState) => AppState) => void,
): Promise<void> {
  await RemoteAgentTask.kill(taskId, setAppState)
  
  setAppState(prev =>
    prev.ultraplanSessionUrl || prev.ultraplanPendingChoice || prev.ultraplanLaunching
      ? {
          ...prev,
          ultraplanSessionUrl: undefined,
          ultraplanPendingChoice: undefined,
          ultraplanLaunching: undefined
        }
      : prev
  )
  
  const url = getRemoteSessionUrl(sessionId, process.env.SESSION_INGRESS_URL)
  enqueuePendingNotification({
    value: `Ultraplan stopped.\n\nSession: ${url}`,
    mode: 'task-notification'
  })
}

// Launch ultraplan
export async function launchUltraplan(opts: {
  blurb: string
  seedPlan?: string
  getAppState: () => AppState
  setAppState: (f: (prev: AppState) => AppState) => void
  signal: AbortSignal
  disconnectedBridge?: boolean
  onSessionReady?: (msg: string) => void
}): Promise<string> {
  const { ultraplanSessionUrl: active, ultraplanLaunching } = getAppState()
  
  if (active || ultraplanLaunching) {
    return buildAlreadyActiveMessage(active)
  }
  
  if (!blurb && !seedPlan) {
    return [
      'Usage: /ultraplan \\<prompt\\>, or include "ultraplan" anywhere in your prompt',
      '',
      'Advanced multi-agent plan mode with our most powerful model (Opus).',
      'Runs in Claude Code on the web. When the plan is ready,',
      'you can execute it in the web session or send it back here.',
      'Terminal stays free while the remote plans.',
      'Requires /login.',
      '',
      `Terms: ${CCR_TERMS_URL}`
    ].join('\n')
  }

  // Set synchronously before detached flow
  setAppState(prev =>
    prev.ultraplanLaunching ? prev : { ...prev, ultraplanLaunching: true }
  )
  
  void launchDetached({
    blurb,
    seedPlan,
    getAppState,
    setAppState,
    signal,
    onSessionReady
  })
  
  return buildLaunchMessage(disconnectedBridge)
}
```

### 5.9 Cost Command: `/cost`

**File:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/commands/cost/cost.ts`

```typescript
import { formatTotalCost } from '../../cost-tracker.js'
import { currentLimits } from '../../services/claudeAiLimits.js'
import type { LocalCommandCall } from '../../types/command.js'
import { isClaudeAISubscriber } from '../../utils/auth.js'

export const call: LocalCommandCall = async () => {
  if (isClaudeAISubscriber()) {
    let value: string

    if (currentLimits.isUsingOverage) {
      value =
        'You are currently using your overages to power your Claude Code usage. We will automatically switch you back to your subscription rate limits when they reset'
    } else {
      value =
        'You are currently using your subscription to power your Claude Code usage'
    }

    if (process.env.USER_TYPE === 'ant') {
      value += `\n\n[ANT-ONLY] Showing cost anyway:\n ${formatTotalCost()}`
    }
    return { type: 'text', value }
  }
  return { type: 'text', value: formatTotalCost() }
}
```

### 5.10 Files Command: `/files`

**File:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/commands/files/files.ts`

```typescript
import { relative } from 'path'
import type { ToolUseContext } from '../../Tool.js'
import type { LocalCommandResult } from '../../types/command.js'
import { getCwd } from '../../utils/cwd.js'
import { cacheKeys } from '../../utils/fileStateCache.js'

export async function call(
  _args: string,
  context: ToolUseContext,
): Promise<LocalCommandResult> {
  const files = context.readFileState
    ? cacheKeys(context.readFileState)
    : []

  if (files.length === 0) {
    return { type: 'text' as const, value: 'No files in context' }
  }

  const fileList = files.map(file => relative(getCwd(), file)).join('\n')
  return {
    type: 'text' as const,
    value: `Files in context:\n${fileList}`
  }
}
```

### 5.11 Help Command: `/help`

**File:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/commands/help/help.tsx`

```typescript
import * as React from 'react'
import { HelpV2 } from '../../components/HelpV2/HelpV2.js'
import type { LocalJSXCommandCall } from '../../types/command.js'

export const call: LocalJSXCommandCall = async (
  onDone,
  { options: { commands } },
) => {
  return <HelpV2 commands={commands} onClose={onDone} />
}
```

### 5.12 Rewind Command: `/rewind`

**File:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/commands/rewind/rewind.ts`

```typescript
import type { LocalCommandResult } from '../../commands.js'
import type { ToolUseContext } from '../../Tool.js'

export async function call(
  _args: string,
  context: ToolUseContext,
): Promise<LocalCommandResult> {
  if (context.openMessageSelector) {
    context.openMessageSelector()
  }
  // Return a skip message to not append any messages
  return { type: 'skip' }
}
```

### 5.13 Clear Command: `/clear`

**File:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/commands/clear/clear.ts`

```typescript
import type { LocalCommandCall } from '../../types/command.js'
import { clearConversation } from './conversation.js'

export const call: LocalCommandCall = async (_, context) => {
  await clearConversation(context)
  return { type: 'text', value: '' }
}
```

### 5.14 Insights Command: `/insights`

**File:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/commands/insights.ts` (partial - 115KB file)

```typescript
import { execFileSync } from 'child_process'
import { diffLines } from 'diff'
import { constants as fsConstants } from 'fs'
import {
  copyFile,
  mkdir,
  mkdtemp,
  readdir,
  readFile,
  rm,
  unlink,
  writeFile,
} from 'fs/promises'
import { tmpdir } from 'os'
import { extname, join } from 'path'
import type { Command } from '../commands.js'
import { queryWithModel } from '../services/api/claude.js'
import {
  AGENT_TOOL_NAME,
  LEGACY_AGENT_TOOL_NAME,
} from '../tools/AgentTool/constants.js'
import type { LogOption } from '../types/logs.js'
import { getClaudeConfigHomeDir } from '../utils/envUtils.js'
import { toError } from '../utils/errors.js'
import { execFileNoThrow } from '../utils/execFileNoThrow.js'
import { logError } from '../utils/log.js'
import { extractTextContent } from '../utils/messages.js'
import { getDefaultOpusModel } from '../utils/model/providers.js'
import {
  getProjectsDir,
  getSessionFilesWithMtime,
  getSessionIdFromLog,
  loadAllLogsFromSessionFile,
} from '../utils/sessionStorage.js'
import { jsonParse, jsonStringify } from '../utils/slowOperations.js'
import { countCharInString } from '../utils/stringUtils.js'
import { asSystemPrompt } from '../utils/systemPromptType.js'
import { escapeXmlAttr as escapeHtml } from '../utils/xml.js'

// Model for facet extraction and summarization (Opus - best quality)
function getAnalysisModel(): string {
  return getDefaultOpusModel()
}

// Model for narrative insights (Opus - best quality)
function getInsightsModel(): string {
  return getDefaultOpusModel()
}

// Session metadata type
type SessionMeta = {
  session_id: string
  project_path: string
  start_time: string
  duration_minutes: number
  user_message_count: number
  assistant_message_count: number
  tool_counts: Record<string, number>
  languages: Record<string, number>
  git_commits: number
  git_pushes: number
  input_tokens: number
  output_tokens: number
  first_prompt: string
  summary?: string
  user_interruptions: number
  user_response_times: number[]
  tool_errors: number
  tool_error_categories: Record<string, number>
  uses_task_agent: boolean
  uses_mcp: boolean
  uses_web_search: boolean
  uses_web_fetch: boolean
  lines_added: number
  lines_removed: number
  files_modified: number
  message_hours: number[]
  user_message_timestamps: string[]
}

// Session facets type
type SessionFacets = {
  session_id: string
  underlying_goal: string
  goal_categories: Record<string, number>
  outcome: string
  user_satisfaction_counts: Record<string, number>
  claude_helpfulness: string
  session_type: string
  friction_counts: Record<string, number>
  friction_detail: string
  primary_success: string
  brief_summary: string
  user_instructions_to_claude?: string[]
}

// Constants for language detection
const EXTENSION_TO_LANGUAGE: Record<string, string> = {
  '.ts': 'TypeScript',
  '.tsx': 'TypeScript',
  '.js': 'JavaScript',
  '.jsx': 'JavaScript',
  '.py': 'Python',
  '.rb': 'Ruby',
  '.go': 'Go',
  '.rs': 'Rust',
  '.java': 'Java',
  '.md': 'Markdown',
  '.json': 'JSON',
  '.yaml': 'YAML',
  '.yml': 'YAML',
  '.sh': 'Shell',
  '.css': 'CSS',
  '.html': 'HTML',
}

// Label map for cleaning up category names
const LABEL_MAP: Record<string, string> = {
  debug_investigate: 'Debug/Investigate',
  implement_feature: 'Implement Feature',
  fix_bug: 'Fix Bug',
  write_script_tool: 'Write Script/Tool',
  refactor_code: 'Refactor Code',
  configure_system: 'Configure System',
  create_pr_commit: 'Create PR/Commit',
  analyze_data: 'Analyze Data',
  understand_codebase: 'Understand Codebase',
  write_tests: 'Write Tests',
  write_docs: 'Write Docs',
  deploy_infra: 'Deploy/Infra',
  warmup_minimal: 'Cache Warmup',
  frustrated: 'Frustrated',
  dissatisfied: 'Dissatisfied',
  likely_satisfied: 'Likely Satisfied',
  satisfied: 'Satisfied',
  happy: 'Happy',
  unsure: 'Unsure',
  neutral: 'Neutral',
  delighted: 'Delighted',
  single_task: 'Single Task',
  multi_task: 'Multi Task',
  iterative_refinement: 'Iterative Refinement',
  exploration: 'Exploration',
  quick_question: 'Quick Question',
  fully_achieved: 'Fully Achieved',
  mostly_achieved: 'Mostly Achieved',
  partially_achieved: 'Partially Achieved',
  not_achieved: 'Not Achieved',
  unclear_from_transcript: 'Unclear',
}

// Facet extraction prompt
const FACET_EXTRACTION_PROMPT = `Analyze this Claude Code session and extract structured facets.

CRITICAL GUIDELINES:

1. **goal_categories**: Count ONLY what the USER explicitly asked for.
   - DO NOT count Claude's autonomous codebase exploration
   - DO NOT count work Claude decided to do on its own
   - ONLY count when user says "can you...", "please...", "I need...", "let's..."

2. **user_satisfaction_counts**: Base ONLY on explicit user signals.
   - "Yay!", "great!", "perfect!" → happy
   - "thanks", "looks good", "that works" → satisfied
   - "ok, now let's..." (continuing without complaint) → likely_satisfied
   - "that's not right", "try again" → dissatisfied
   - "this is broken", "I give up" → frustrated

3. **friction_counts**: Be specific about what went wrong.
   - misunderstood_request: Claude interpreted incorrectly
   - wrong_approach: Right goal, wrong solution method
   - buggy_code: Code didn't work correctly
   - user_rejected_action: User said no/stop to a tool call
   - excessive_changes: Over-engineered or changed too much

SESSION:
`

// Extract tool stats from session log
function extractToolStats(log: LogOption): {
  toolCounts: Record<string, number>
  languages: Record<string, number>
  gitCommits: number
  gitPushes: number
  inputTokens: number
  outputTokens: number
  userInterruptions: number
  userResponseTimes: number[]
  toolErrors: number
  toolErrorCategories: Record<string, number>
  usesTaskAgent: boolean
  usesMcp: boolean
  usesWebSearch: boolean
  usesWebFetch: boolean
  linesAdded: number
  linesRemoved: number
  filesModified: Set<string>
  messageHours: number[]
  userMessageTimestamps: string[]
} {
  const toolCounts: Record<string, number> = {}
  const languages: Record<string, number> = {}
  let gitCommits = 0
  let gitPushes = 0
  let inputTokens = 0
  let outputTokens = 0
  let userInterruptions = 0
  const userResponseTimes: number[] = []
  let toolErrors = 0
  const toolErrorCategories: Record<string, number> = {}
  let usesTaskAgent = false
  let linesAdded = 0
  let linesRemoved = 0
  const filesModified = new Set<string>()
  const messageHours: number[] = []
  const userMessageTimestamps: string[] = []
  let usesMcp = false
  let usesWebSearch = false
  let usesWebFetch = false
  let lastAssistantTimestamp: string | null = null

  for (const msg of log.messages) {
    const msgTimestamp = (msg as { timestamp?: string }).timestamp

    if (msg.type === 'assistant' && msg.message) {
      if (msgTimestamp) {
        lastAssistantTimestamp = msgTimestamp
      }

      const usage = (msg.message as { usage?: { input_tokens?: number; output_tokens?: number } }).usage
      if (usage) {
        inputTokens += usage.input_tokens || 0
        outputTokens += usage.output_tokens || 0
      }

      const content = msg.message.content
      if (Array.isArray(content)) {
        for (const block of content) {
          if (block.type === 'tool_use' && 'name' in block) {
            const toolName = block.name as string
            toolCounts[toolName] = (toolCounts[toolName] || 0) + 1

            if (toolName === AGENT_TOOL_NAME || toolName === LEGACY_AGENT_TOOL_NAME)
              usesTaskAgent = true
            if (toolName.startsWith('mcp__')) usesMcp = true
            if (toolName === 'WebSearch') usesWebSearch = true
            if (toolName === 'WebFetch') usesWebFetch = true

            const input = (block as { input?: Record<string, unknown> }).input
            if (input) {
              const filePath = (input.file_path as string) || ''
              if (filePath) {
                const lang = getLanguageFromPath(filePath)
                if (lang) {
                  languages[lang] = (languages[lang] || 0) + 1
                }
                if (toolName === 'Edit' || toolName === 'Write') {
                  filesModified.add(filePath)
                }
              }

              if (toolName === 'Edit') {
                const oldString = (input.old_string as string) || ''
                const newString = (input.new_string as string) || ''
                for (const change of diffLines(oldString, newString)) {
                  if (change.added) linesAdded += change.count || 0
                  if (change.removed) linesRemoved += change.count || 0
                }
              }

              if (toolName === 'Write') {
                const writeContent = (input.content as string) || ''
                if (writeContent) {
                  linesAdded += countCharInString(writeContent, '\n') + 1
                }
              }

              const command = (input.command as string) || ''
              if (command.includes('git commit')) gitCommits++
              if (command.includes('git push')) gitPushes++
            }
          }
        }
      }
    }

    // Process user messages
    if (msg.type === 'user' && msg.message) {
      const content = msg.message.content
      let isHumanMessage = false
      
      if (typeof content === 'string' && content.trim()) {
        isHumanMessage = true
      } else if (Array.isArray(content)) {
        for (const block of content) {
          if (block.type === 'text' && 'text' in block) {
            isHumanMessage = true
            break
          }
        }
      }

      if (isHumanMessage) {
        if (msgTimestamp) {
          try {
            const msgDate = new Date(msgTimestamp)
            const hour = msgDate.getHours()
            messageHours.push(hour)
            userMessageTimestamps.push(msgTimestamp)
          } catch {
            // Skip invalid timestamps
          }
        }

        if (lastAssistantTimestamp && msgTimestamp) {
          const assistantTime = new Date(lastAssistantTimestamp).getTime()
          const userTime = new Date(msgTimestamp).getTime()
          const responseTimeSec = (userTime - assistantTime) / 1000
          if (responseTimeSec > 2 && responseTimeSec < 3600) {
            userResponseTimes.push(responseTimeSec)
          }
        }
      }

      // Process tool results for error tracking
      if (Array.isArray(content)) {
        for (const block of content) {
          if (block.type === 'tool_result' && 'content' in block) {
            const isError = (block as { is_error?: boolean }).is_error
            if (isError) {
              toolErrors++
              const resultContent = (block as { content?: string }).content
              let category = 'Other'
              if (typeof resultContent === 'string') {
                const lowerContent = resultContent.toLowerCase()
                if (lowerContent.includes('exit code')) category = 'Command Failed'
                else if (lowerContent.includes('rejected')) category = 'User Rejected'
                else if (lowerContent.includes('string to replace not found')) category = 'Edit Failed'
                else if (lowerContent.includes('modified since read')) category = 'File Changed'
                else if (lowerContent.includes('exceeds maximum')) category = 'File Too Large'
                else if (lowerContent.includes('file not found')) category = 'File Not Found'
              }
              toolErrorCategories[category] = (toolErrorCategories[category] || 0) + 1
            }
          }
        }
      }

      // Check for interruptions
      if (typeof content === 'string') {
        if (content.includes('[Request interrupted by user')) {
          userInterruptions++
        }
      } else if (Array.isArray(content)) {
        for (const block of content) {
          if (block.type === 'text' && 'text' in block && (block.text as string).includes('[Request interrupted by user')) {
            userInterruptions++
            break
          }
        }
      }
    }
  }

  return {
    toolCounts,
    languages,
    gitCommits,
    gitPushes,
    inputTokens,
    outputTokens,
    userInterruptions,
    userResponseTimes,
    toolErrors,
    toolErrorCategories,
    usesTaskAgent,
    usesMcp,
    usesWebSearch,
    usesWebFetch,
    linesAdded,
    linesRemoved,
    filesModified,
    messageHours,
    userMessageTimestamps
  }
}
```

---

## 6. Representative Command Implementations

### 6.1 Login Command Pattern

```typescript
// commands/login/index.ts
import type { Command } from '../../commands.js'
import { hasAnthropicApiKeyAuth } from '../../utils/auth.js'
import { isEnvTruthy } from '../../utils/envUtils.js'

export default () =>
  ({
    type: 'local-jsx',
    name: 'login',
    description: hasAnthropicApiKeyAuth()
      ? 'Switch Anthropic accounts'
      : 'Sign in with your Anthropic account',
    isEnabled: () => !isEnvTruthy(process.env.DISABLE_LOGIN_COMMAND),
    load: () => import('./login.js'),
  }) satisfies Command
```

### 6.2 Memory Command Pattern

```typescript
// commands/memory/index.ts
import type { Command } from '../../commands.js'

const memory: Command = {
  type: 'local-jsx',
  name: 'memory',
  description: 'Edit Claude memory files',
  load: () => import('./memory.js'),
}

export default memory
```

### 6.3 Plugin Command Pattern

```typescript
// commands/plugin/index.tsx
import type { Command } from '../../commands.js'

const plugin = {
  type: 'local-jsx',
  name: 'plugin',
  aliases: ['plugins', 'marketplace'],
  description: 'Manage Claude Code plugins',
  immediate: true,
  load: () => import('./plugin.js')
} satisfies Command

export default plugin
```

### 6.4 Skills Command Pattern

```typescript
// commands/skills/index.ts
import type { Command } from '../../commands.js'

const skills = {
  type: 'local-jsx',
  name: 'skills',
  description: 'List available skills',
  load: () => import('./skills.js'),
} satisfies Command

export default skills
```

### 6.5 Status Command Pattern

```typescript
// commands/status/index.ts
import { getIsRemoteMode } from '../../bootstrap/state.js'
import type { Command } from '../../commands.js'

const session = {
  type: 'local-jsx',
  name: 'session',
  aliases: ['remote'],
  description: 'Show remote session URL and QR code',
  isEnabled: () => getIsRemoteMode(),
  get isHidden() {
    return !getIsRemoteMode()
  },
  load: () => import('./session.js'),
} satisfies Command

export default session
```

### 6.6 Usage Command Pattern

```typescript
// commands/usage/index.ts
import type { Command } from '../../commands.js'

export default {
  type: 'local-jsx',
  name: 'usage',
  description: 'Show plan usage limits',
  availability: ['claude-ai'],
  load: () => import('./usage.js'),
} satisfies Command
```

### 6.7 Exit Command Pattern

```typescript
// commands/exit/index.ts
import type { Command } from '../../commands.js'

const exit = {
  type: 'local-jsx',
  name: 'exit',
  aliases: ['quit'],
  description: 'Exit the REPL',
  immediate: true,
  load: () => import('./exit.js'),
} satisfies Command

export default exit
```

### 6.8 Plan Command Pattern

```typescript
// commands/plan/index.ts
import type { Command } from '../../commands.js'

const plan = {
  type: 'local-jsx',
  name: 'plan',
  description: 'Enable plan mode or view the current session plan',
  argumentHint: '[open|<description>]',
  load: () => import('./plan.js'),
} satisfies Command

export default plan
```

### 6.9 Add-Dir Command Pattern

```typescript
// commands/add-dir/index.ts
import type { Command } from '../../commands.js'

const addDir = {
  type: 'local-jsx',
  name: 'add-dir',
  description: 'Add a new working directory',
  argumentHint: '<path>',
  load: () => import('./add-dir.js'),
} satisfies Command

export default addDir
```

### 6.10 Doctor Command Pattern

```typescript
// commands/doctor/doctor.tsx
import React from 'react'
import { Doctor } from '../../screens/Doctor.js'
import type { LocalJSXCommandCall } from '../../types/command.js'

export const call: LocalJSXCommandCall = (onDone, _context, _args) => {
  return Promise.resolve(<Doctor onDone={onDone} />)
}
```

### 6.11 Stats Command Pattern

```typescript
// commands/stats/stats.tsx
import * as React from 'react'
import { Stats } from '../../components/Stats.js'
import type { LocalJSXCommandCall } from '../../types/command.js'

export const call: LocalJSXCommandCall = async onDone => {
  return <Stats onClose={onDone} />
}
```

### 6.12 Diff Command Pattern

```typescript
// commands/diff/diff.tsx
import * as React from 'react'
import type { LocalJSXCommandCall } from '../../types/command.js'

export const call: LocalJSXCommandCall = async (onDone, context) => {
  const { DiffDialog } = await import('../../components/diff/DiffDialog.js')
  return <DiffDialog messages={context.messages} onDone={onDone} />
}
```

### 6.13 Config Command Pattern

```typescript
// commands/config/config.tsx
import * as React from 'react'
import { Settings } from '../../components/Settings/Settings.js'
import type { LocalJSXCommandCall } from '../../types/command.js'

export const call: LocalJSXCommandCall = async (onDone, context) => {
  return <Settings onClose={onDone} context={context} defaultTab="Config" />
}
```

---

## 7. Plugin and Skill System Integration

### 7.1 Dynamic Skills Loading

```typescript
// From commands.ts
async function getSkills(cwd: string): Promise<{
  skillDirCommands: Command[]
  pluginSkills: Command[]
  bundledSkills: Command[]
  builtinPluginSkills: Command[]
}> {
  try {
    const [skillDirCommands, pluginSkills] = await Promise.all([
      getSkillDirCommands(cwd).catch(err => {
        logError(toError(err))
        logForDebugging(
          'Skill directory commands failed to load, continuing without them',
        )
        return []
      }),
      getPluginSkills().catch(err => {
        logError(toError(err))
        logForDebugging('Plugin skills failed to load, continuing without them')
        return []
      }),
    ])
    
    // Bundled skills are registered synchronously at startup
    const bundledSkills = getBundledSkills()
    // Built-in plugin skills come from enabled built-in plugins
    const builtinPluginSkills = getBuiltinPluginSkillCommands()
    
    logForDebugging(
      `getSkills returning: ${skillDirCommands.length} skill dir commands, ${pluginSkills.length} plugin skills, ${bundledSkills.length} bundled skills, ${builtinPluginSkills.length} builtin plugin skills`,
    )
    
    return {
      skillDirCommands,
      pluginSkills,
      bundledSkills,
      builtinPluginSkills,
    }
  } catch (err) {
    logError(toError(err))
    logForDebugging('Unexpected error in getSkills, returning empty')
    return {
      skillDirCommands: [],
      pluginSkills: [],
      bundledSkills: [],
      builtinPluginSkills: [],
    }
  }
}
```

### 7.2 Skill Tool Commands

```typescript
// SkillTool shows ALL prompt-based commands that the model can invoke
export const getSkillToolCommands = memoize(
  async (cwd: string): Promise<Command[]> => {
    const allCommands = await getCommands(cwd)
    return allCommands.filter(
      cmd =>
        cmd.type === 'prompt' &&
        !cmd.disableModelInvocation &&
        cmd.source !== 'builtin' &&
        // Always include skills from /skills/ dirs, bundled skills, and legacy /commands/ entries
        (cmd.loadedFrom === 'bundled' ||
          cmd.loadedFrom === 'skills' ||
          cmd.loadedFrom === 'commands_DEPRECATED' ||
          cmd.hasUserSpecifiedDescription ||
          cmd.whenToUse),
    )
  },
)

// Filters commands to include only skills
export const getSlashCommandToolSkills = memoize(
  async (cwd: string): Promise<Command[]> => {
    try {
      const allCommands = await getCommands(cwd)
      return allCommands.filter(
        cmd =>
          cmd.type === 'prompt' &&
          cmd.source !== 'builtin' &&
          (cmd.hasUserSpecifiedDescription || cmd.whenToUse) &&
          (cmd.loadedFrom === 'skills' ||
            cmd.loadedFrom === 'plugin' ||
            cmd.loadedFrom === 'bundled' ||
            cmd.disableModelInvocation),
      )
    }
  }
)
```

### 7.3 MCP Skill Commands

```typescript
// Filter AppState.mcp.commands to MCP-provided skills
export function getMcpSkillCommands(
  mcpCommands: readonly Command[],
): readonly Command[] {
  if (feature('MCP_SKILLS')) {
    return mcpCommands.filter(
      cmd =>
        cmd.type === 'prompt' &&
        cmd.loadedFrom === 'mcp' &&
        !cmd.disableModelInvocation,
    )
  }
  return []
}
```

### 7.4 Cache Management

```typescript
/**
 * Clears only the memoization caches for commands, WITHOUT clearing skill caches.
 * Use this when dynamic skills are added to invalidate cached command lists.
 */
export function clearCommandMemoizationCaches(): void {
  loadAllCommands.cache?.clear?.()
  getSkillToolCommands.cache?.clear?.()
  getSlashCommandToolSkills.cache?.clear?.()
  clearSkillIndexCache?.()
}

export function clearCommandsCache(): void {
  clearCommandMemoizationCaches()
  clearPluginCommandCache()
  clearPluginSkillsCache()
  clearSkillCaches()
}
```

---

## 8. Feature Flags and Conditional Commands

### 8.1 Feature-Gated Command Imports

```typescript
// Feature flags control which commands are available
const proactive =
  feature('PROACTIVE') || feature('KAIROS')
    ? require('./commands/proactive.js').default
    : null

const briefCommand =
  feature('KAIROS') || feature('KAIROS_BRIEF')
    ? require('./commands/brief.js').default
    : null

const assistantCommand = feature('KAIROS')
  ? require('./commands/assistant/index.js').default
  : null

const bridge = feature('BRIDGE_MODE')
  ? require('./commands/bridge/index.js').default
  : null

const remoteControlServerCommand =
  feature('DAEMON') && feature('BRIDGE_MODE')
    ? require('./commands/remoteControlServer/index.js').default
    : null

const voiceCommand = feature('VOICE_MODE')
  ? require('./commands/voice/index.js').default
  : null

const forceSnip = feature('HISTORY_SNIP')
  ? require('./commands/force-snip.js').default
  : null

const workflowsCmd = feature('WORKFLOW_SCRIPTS')
  ? (require('./commands/workflows/index.js') as typeof import('./commands/workflows/index.js')).default
  : null

const webCmd = feature('CCR_REMOTE_SETUP')
  ? (require('./commands/remote-setup/index.js') as typeof import('./commands/remote-setup/index.js')).default
  : null

const subscribePr = feature('KAIROS_GITHUB_WEBHOOKS')
  ? require('./commands/subscribe-pr.js').default
  : null

const ultraplan = feature('ULTRAPLAN')
  ? require('./commands/ultraplan.js').default
  : null

const torch = feature('TORCH')
  ? require('./commands/torch.js').default
  : null

const peersCmd = feature('UDS_INBOX')
  ? (require('./commands/peers/index.js') as typeof import('./commands/peers/index.js')).default
  : null

const forkCmd = feature('FORK_SUBAGENT')
  ? (require('./commands/fork/index.js') as typeof import('./commands/fork/index.js')).default
  : null

const buddy = feature('BUDDY')
  ? (require('./commands/buddy/index.js') as typeof import('./commands/buddy/index.js')).default
  : null
```

### 8.2 Command Availability

```typescript
export type CommandAvailability =
  // claude.ai OAuth subscriber
  | 'claude-ai'
  // Console API key user (direct api.anthropic.com)
  | 'console'

export function meetsAvailabilityRequirement(cmd: Command): boolean {
  if (!cmd.availability) return true
  for (const a of cmd.availability) {
    switch (a) {
      case 'claude-ai':
        if (isClaudeAISubscriber()) return true
        break
      case 'console':
        if (
          !isClaudeAISubscriber() &&
          !isUsing3PServices() &&
          isFirstPartyAnthropicBaseUrl()
        )
          return true
        break
    }
  }
  return false
}
```

### 8.3 Environment-Based Command Filtering

```typescript
// Internal-only commands eliminated from external build
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
  mockLimits,
  bridgeKick,
  version,
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
  agentsPlatform,
  autofixPr,
].filter(Boolean)

// Added to command array only for ant users
...(process.env.USER_TYPE === 'ant' && !process.env.IS_DEMO
  ? INTERNAL_ONLY_COMMANDS
  : [])
```

---

## Summary

The commands module is the central hub for all user-interactable commands in Claude Code. With 189 files and multiple command types (prompt, local, local-jsx), it provides:

1. **Centralized Registry**: Memoized command loading with dynamic skill/plugin integration
2. **Type-Safe Architecture**: Comprehensive TypeScript types for all command variants
3. **Feature Gating**: GrowthBook feature flags control command availability
4. **Auth-Based Filtering**: Commands can be gated by authentication type (claude-ai, console)
5. **Lazy Loading**: Heavy dependencies are loaded on-demand via dynamic imports
6. **Sub-agent Forking**: Prompt commands can fork to sub-agents with isolated context
7. **Plugin Integration**: Dynamic loading of skills, plugins, and MCP-provided commands
8. **Cache Management**: Sophisticated memoization with explicit cache invalidation

The command execution pipeline flows through `processSlashCommand` → `getMessagesForSlashCommand` → type-specific handlers, with support for background execution, progress reporting, and result queuing.

---

**Document Statistics:**
- Source files analyzed: 50+
- Command implementations documented: 15+
- Total lines: 5000+
