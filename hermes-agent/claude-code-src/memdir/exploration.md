# Memdir Module — Deep-Dive Exploration

**Module:** `memdir/`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/memdir/`  
**Files:** 8 TypeScript files  
**Created:** 2026-04-07

---

## 1. Module Overview

The `memdir/` module implements Claude Code's **persistent memory system** — a file-based knowledge storage that persists across sessions, enabling Claude to remember user preferences, project context, feedback, and external system references. This is the infrastructure for "auto memory" and "team memory" features.

### Core Responsibilities

1. **Memory Directory Management** — Path resolution and directory creation:
   - Auto memory path computation (per-project)
   - Team memory path (shared across team members)
   - Daily log paths for assistant mode
   - Override support via env vars and settings.json

2. **Memory Type System** — Four-type taxonomy:
   - **user**: User role, preferences, knowledge
   - **feedback**: Guidance on how to approach work
   - **project**: Ongoing work, goals, incidents
   - **reference**: Pointers to external systems

3. **Prompt Building** — Memory system instructions:
   - How to save memories (frontmatter format)
   - What NOT to save (exclusions)
   - When to access memories
   - Trusting recall (verify before recommending)

4. **Assistant Mode** — Daily log workflow:
   - Append-only date-named log files
   - Nightly distillation to MEMORY.md
   - SkipIndex mode for streamlined logging

5. **Memory Loading** — Entrypoint loading with truncation:
   - Line cap (200 lines)
   - Byte cap (25KB)
   - Truncation warnings

6. **Team Memory Integration** — Private + team directories:
   - Combined prompt for both directories
   - Scope tags (private/team) in type descriptions
   - Feature-gated via TEAMMEM flag

### Key Design Patterns

- **Path Validation**: Security-hardened path normalization (null bytes, UNC, drive-root)
- **Memoization**: `getAutoMemPath()` memoized on projectRoot to avoid repeated computation
- **Feature Gating**: Auto memory, team memory, assistant mode all feature-gated
- **Truncation Safety**: Dual caps (lines + bytes) prevent memory bloat
- **Tilde Expansion**: User-friendly `~/` paths in settings.json
- **Dependency Injection**: Scratchpad dir passed in to avoid circular deps

---

## 2. File Inventory

| File | Lines | Description |
|------|-------|-------------|
| `findRelevantMemories.ts` | — | Memory search/retrieval (not read) |
| `memdir.ts` | ~508 | Main prompt building, memory loading |
| `memoryAge.ts` | — | Memory age calculation (not read) |
| `memoryScan.ts` | — | Memory scanning logic (not read) |
| `memoryTypes.ts` | ~272 | Type taxonomy, frontmatter, guidance |
| `paths.ts` | ~279 | Path resolution, validation, overrides |
| `teamMemPaths.ts` | — | Team memory path computation (feature-gated) |
| `teamMemPrompts.ts` | — | Team memory prompt building (feature-gated) |

**Note**: Files marked "—" are feature-gated behind `TEAMMEM` or not read in this exploration.

**Total:** ~1,059 lines in 4 core files (excluding feature-gated team memory files)

---

## 3. Key Exports

### Path Resolution (`paths.ts`)

```typescript
// Check if auto-memory is enabled
export function isAutoMemoryEnabled(): boolean

// Check if extract-mode is active (background agent)
export function isExtractModeActive(): boolean

// Get base directory for memory storage
export function getMemoryBaseDir(): string

// Get auto-memory directory path (memoized)
export const getAutoMemPath: () => string

// Get daily log file path (assistant mode)
export function getAutoMemDailyLogPath(date?: Date): string

// Get MEMORY.md entrypoint path
export function getAutoMemEntrypoint(): string

// Check if path is within auto-memory directory
export function isAutoMemPath(absolutePath: string): boolean

// Check if path override is set
export function hasAutoMemPathOverride(): boolean
```

### Memory Types (`memoryTypes.ts`)

```typescript
// Four memory types
export const MEMORY_TYPES = ['user', 'feedback', 'project', 'reference'] as const
export type MemoryType = (typeof MEMORY_TYPES)[number]

// Parse memory type from frontmatter
export function parseMemoryType(raw: unknown): MemoryType | undefined

// Type descriptions for combined mode (private + team)
export const TYPES_SECTION_COMBINED: readonly string[]

// Type descriptions for individual-only mode
export const TYPES_SECTION_INDIVIDUAL: readonly string[]

// What NOT to save section
export const WHAT_NOT_TO_SAVE_SECTION: readonly string[]

// When to access memories section
export const WHEN_TO_ACCESS_SECTION: readonly string[]

// Trusting recall section
export const TRUSTING_RECALL_SECTION: readonly string[]

// Frontmatter example
export const MEMORY_FRONTMATTER_EXAMPLE: readonly string[]

// Memory drift caveat
export const MEMORY_DRIFT_CAVEAT: string
```

### Prompt Building (`memdir.ts`)

```typescript
// Entrypoint constants
export const ENTRYPOINT_NAME = 'MEMORY.md'
export const MAX_ENTRYPOINT_LINES = 200
export const MAX_ENTRYPOINT_BYTES = 25_000

// Truncate entrypoint content
export function truncateEntrypointContent(raw: string): EntrypointTruncation

// Ensure memory directory exists
export async function ensureMemoryDirExists(memoryDir: string): Promise<void>

// Build memory lines (without content)
export function buildMemoryLines(
  displayName: string,
  memoryDir: string,
  extraGuidelines?: string[],
  skipIndex?: boolean,
): string[]

// Build memory prompt (with MEMORY.md content)
export function buildMemoryPrompt(params: {
  displayName: string
  memoryDir: string
  extraGuidelines?: string[]
}): string

// Load memory prompt for system prompt
export async function loadMemoryPrompt(): Promise<string | null>
```

### Path Override Types (`paths.ts`)

```typescript
// Env var override (Cowork SDK)
function getAutoMemPathOverride(): string | undefined

// Settings.json override (user-friendly, supports ~/)
function getAutoMemPathSetting(): string | undefined
```

---

## 4. Line-by-Line Analysis

### 4.1 Auto Memory Enablement (`paths.ts`)

**Enablement Logic (lines 22-55):**

```typescript
/**
 * Whether auto-memory features are enabled (memdir, agent memory, past session search).
 * Enabled by default. Priority chain (first defined wins):
 *   1. CLAUDE_CODE_DISABLE_AUTO_MEMORY env var (1/true → OFF, 0/false → ON)
 *   2. CLAUDE_CODE_SIMPLE (--bare) → OFF
 *   3. CCR without persistent storage → OFF (no CLAUDE_CODE_REMOTE_MEMORY_DIR)
 *   4. autoMemoryEnabled in settings.json (supports project-level opt-out)
 *   5. Default: enabled
 */
export function isAutoMemoryEnabled(): boolean {
  const envVal = process.env.CLAUDE_CODE_DISABLE_AUTO_MEMORY
  if (isEnvTruthy(envVal)) {
    return false
  }
  if (isEnvDefinedFalsy(envVal)) {
    return true
  }
  // --bare / SIMPLE: prompts.ts already drops memory section from system prompt
  if (isEnvTruthy(process.env.CLAUDE_CODE_SIMPLE)) {
    return false
  }
  if (
    isEnvTruthy(process.env.CLAUDE_CODE_REMOTE) &&
    !process.env.CLAUDE_CODE_REMOTE_MEMORY_DIR
  ) {
    return false
  }
  const settings = getInitialSettings()
  if (settings.autoMemoryEnabled !== undefined) {
    return settings.autoMemoryEnabled
  }
  return true
}
```

**Priority Chain**: Env var → SIMPLE mode → CCR storage → settings.json → default enabled.

**SIMPLE Mode**: `--bare` flag disables memory — prompts.ts drops memory section from system prompt.

**Extract Mode Gate (lines 68-77):**

```typescript
export function isExtractModeActive(): boolean {
  if (!getFeatureValue_CACHED_MAY_BE_STALE('tengu_passport_quail', false)) {
    return false
  }
  return (
    !getIsNonInteractiveSession() ||
    getFeatureValue_CACHED_MAY_BE_STALE('tengu_slate_thimble', false)
  )
}
```

**Purpose**: Background agent that extracts memories at end of session. Only active in interactive sessions or with explicit gate.

### 4.2 Path Validation (`paths.ts`)

**Security-Hardened Validation (lines 109-150):**

```typescript
function validateMemoryPath(
  raw: string | undefined,
  expandTilde: boolean,
): string | undefined {
  if (!raw) return undefined
  
  let candidate = raw
  // Tilde expansion for settings.json (user-friendly)
  if (expandTilde && (candidate.startsWith('~/') || candidate.startsWith('~\\'))) {
    const rest = candidate.slice(2)
    // Reject trivial remainders that would expand to $HOME or ancestor
    const restNorm = normalize(rest || '.')
    if (restNorm === '.' || restNorm === '..') {
      return undefined  // ~/ and ~/* would match $HOME or parent
    }
    candidate = join(homedir(), rest)
  }
  
  const normalized = normalize(candidate).replace(/[/\\]+$/, '')
  
  // SECURITY: Reject dangerous paths
  if (
    !isAbsolute(normalized) ||           // Must be absolute
    normalized.length < 3 ||             // Not root/near-root
    /^[A-Za-z]:$/.test(normalized) ||    // Not Windows drive-root
    normalized.startsWith('\\\\') ||      // Not UNC paths
    normalized.startsWith('//') ||        // Not UNC (Unix form)
    normalized.includes('\0')            // No null bytes
  ) {
    return undefined
  }
  
  return (normalized + sep).normalize('NFC')  // Exactly one trailing separator
}
```

**Security Considerations**:
- **Relative paths**: Would be interpreted relative to CWD (attacker-controlled)
- **Root/near-root**: `/` → "" after strip; `/a` too short
- **Drive-root**: `C:` survives strip, becomes dangerous
- **UNC paths**: Network paths — opaque trust boundary
- **Null bytes**: Survives normalize(), can truncate in syscalls

**Tilde Expansion Security**: `~/`, `~/., `~/..` NOT expanded — would make isAutoMemPath() match all of $HOME.

**Env Var Override (lines 161-166):**

```typescript
function getAutoMemPathOverride(): string | undefined {
  return validateMemoryPath(
    process.env.CLAUDE_COWORK_MEMORY_PATH_OVERRIDE,
    false,  // No tilde expansion for env var (programmatically set)
  )
}
```

**Settings.json Override (lines 179-186):**

```typescript
function getAutoMemPathSetting(): string | undefined {
  const dir =
    getSettingsForSource('policySettings')?.autoMemoryDirectory ??
    getSettingsForSource('flagSettings')?.autoMemoryDirectory ??
    getSettingsForSource('localSettings')?.autoMemoryDirectory ??
    getSettingsForSource('userSettings')?.autoMemoryDirectory
  return validateMemoryPath(dir, true)  // Tilde expansion for user convenience
}
```

**Priority**: policy → flag → local → user settings.

**Security Note**: `projectSettings` (.claude/settings.json in repo) intentionally EXCLUDED — malicious repo could set `autoMemoryDirectory: "~/.ssh"`.

### 4.3 Auto Memory Path Memoization (lines 223-235)

```typescript
/**
 * Returns the auto-memory directory path.
 *
 * Resolution order:
 *   1. CLAUDE_COWORK_MEMORY_PATH_OVERRIDE env var
 *   2. autoMemoryDirectory in settings.json
 *   3. <memoryBase>/projects/<sanitized-git-root>/memory/
 *
 * Memoized: render-path callers fire per tool-use message per Messages re-render;
 * each miss costs getSettingsForSource × 4 → parseSettingsFile (realpathSync + readFileSync).
 * Keyed on projectRoot so tests that change its mock mid-block recompute.
 */
export const getAutoMemPath = memoize(
  (): string => {
    const override = getAutoMemPathOverride() ?? getAutoMemPathSetting()
    if (override) {
      return override
    }
    const projectsDir = join(getMemoryBaseDir(), 'projects')
    return (
      join(projectsDir, sanitizePath(getAutoMemBase()), AUTO_MEM_DIRNAME) + sep
    ).normalize('NFC')
  },
  () => getProjectRoot(),  // Memo key
)
```

**Why Memoize**: Called frequently during rendering — each miss costs 4x settings file reads.

**Git Root Fallback (lines 203-205):**

```typescript
function getAutoMemBase(): string {
  return findCanonicalGitRoot(getProjectRoot()) ?? getProjectRoot()
}
```

**Why**: All worktrees of same repo share one auto-memory directory.

### 4.4 Daily Log Path (lines 246-251)

```typescript
export function getAutoMemDailyLogPath(date: Date = new Date()): string {
  const yyyy = date.getFullYear().toString()
  const mm = (date.getMonth() + 1).toString().padStart(2, '0')
  const dd = date.getDate().toString().padStart(2, '0')
  return join(getAutoMemPath(), 'logs', yyyy, mm, `${yyyy}-${mm}-${dd}.md`)
}
```

**Shape**: `<autoMemPath>/logs/YYYY/MM/YYYY-MM-DD.md`

**Purpose**: Assistant mode appends to daily log; nightly `/dream` skill distills to MEMORY.md.

### 4.5 Entrypoint Truncation (`memdir.ts`)

**Dual Caps (lines 34-47):**

```typescript
export const ENTRYPOINT_NAME = 'MEMORY.md'
export const MAX_ENTRYPOINT_LINES = 200
// ~125 chars/line at 200 lines. At p97 today; catches long-line indexes that
// slip past the line cap (p100 observed: 197KB under 200 lines).
export const MAX_ENTRYPOINT_BYTES = 25_000

export type EntrypointTruncation = {
  content: string
  lineCount: number
  byteCount: number
  wasLineTruncated: boolean
  wasByteTruncated: boolean
}
```

**Truncation Logic (lines 57-103):**

```typescript
export function truncateEntrypointContent(raw: string): EntrypointTruncation {
  const trimmed = raw.trim()
  const contentLines = trimmed.split('\n')
  const lineCount = contentLines.length
  const byteCount = trimmed.length

  const wasLineTruncated = lineCount > MAX_ENTRYPOINT_LINES
  // Check original byte count — long lines are the failure mode
  const wasByteTruncated = byteCount > MAX_ENTRYPOINT_BYTES

  if (!wasLineTruncated && !wasByteTruncated) {
    return { content: trimmed, lineCount, byteCount, wasLineTruncated, wasByteTruncated }
  }

  let truncated = wasLineTruncated
    ? contentLines.slice(0, MAX_ENTRYPOINT_LINES).join('\n')
    : trimmed

  if (truncated.length > MAX_ENTRYPOINT_BYTES) {
    const cutAt = truncated.lastIndexOf('\n', MAX_ENTRYPOINT_BYTES)
    truncated = truncated.slice(0, cutAt > 0 ? cutAt : MAX_ENTRYPOINT_BYTES)
  }

  const reason =
    wasByteTruncated && !wasLineTruncated
      ? `${formatFileSize(byteCount)} (limit: ${formatFileSize(MAX_ENTRYPOINT_BYTES)}) — index entries are too long`
      : wasLineTruncated && !wasByteTruncated
        ? `${lineCount} lines (limit: ${MAX_ENTRYPOINT_LINES})`
        : `${lineCount} lines and ${formatFileSize(byteCount)}`

  return {
    content:
      truncated +
      `\n\n> WARNING: ${ENTRYPOINT_NAME} is ${reason}. Only part of it was loaded. Keep index entries to one line under ~200 chars; move detail into topic files.`,
    lineCount,
    byteCount,
    wasLineTruncated,
    wasByteTruncated,
  }
}
```

**Why Byte-Truncate at Last Newline**: Avoids cutting mid-line, produces cleaner truncation.

**Warning Message**: Tells user which cap fired and suggests keeping index entries under ~200 chars.

### 4.6 Memory Directory Creation (lines 129-147)

```typescript
export async function ensureMemoryDirExists(memoryDir: string): Promise<void> {
  const fs = getFsImplementation()
  try {
    await fs.mkdir(memoryDir)
  } catch (e) {
    // fs.mkdir already handles EEXIST internally. Real problems (EACCES/EPERM/EROFS)
    // are logged for debugging. Prompt building continues; model's Write will
    // surface real perm errors.
    const code =
      e instanceof Error && 'code' in e && typeof e.code === 'string'
        ? e.code
        : undefined
    logForDebugging(
      `ensureMemoryDirExists failed for ${memoryDir}: ${code ?? String(e)}`,
      { level: 'debug' },
    )
  }
}
```

**Idempotent**: `fs.mkdir` is recursive and swallows EEXIST.

**Non-Blocking**: Error logged but prompt building continues — model's Write will fail if perms are wrong.

### 4.7 Memory Line Building (`memdir.ts`)

**Build Memory Lines (lines 199-266):**

```typescript
export function buildMemoryLines(
  displayName: string,
  memoryDir: string,
  extraGuidelines?: string[],
  skipIndex = false,
): string[] {
  const howToSave = skipIndex
    ? [
        // SkipIndex mode: write directly to topic files
        '## How to save memories',
        '',
        'Write each memory to its own file (e.g., `user_role.md`, `feedback_testing.md`) using this frontmatter format:',
        '',
        ...MEMORY_FRONTMATTER_EXAMPLE,
        // No index guidance
      ]
    : [
        // Standard mode: write to file + add to index
        '## How to save memories',
        '',
        'Saving a memory is a two-step process:',
        '',
        '**Step 1** — write the memory to its own file...',
        '',
        `**Step 2** — add a pointer to that file in \`${ENTRYPOINT_NAME}\`. \`${ENTRYPOINT_NAME}\` is an index, not a memory — each entry should be one line, under ~150 characters...`,
        '',
        `- \`${ENTRYPOINT_NAME}\` is always loaded into your conversation context — lines after ${MAX_ENTRYPOINT_LINES} will be truncated...`,
      ]

  const lines: string[] = [
    `# ${displayName}`,
    '',
    `You have a persistent, file-based memory system at \`${memoryDir}\`. ${DIR_EXISTS_GUIDANCE}`,
    '',
    "You should build up this memory system over time...",
    '',
    'If the user explicitly asks you to remember something, save it immediately...',
    '',
    ...TYPES_SECTION_INDIVIDUAL,
    ...WHAT_NOT_TO_SAVE_SECTION,
    '',
    ...howToSave,
    '',
    ...WHEN_TO_ACCESS_SECTION,
    '',
    ...TRUSTING_RECALL_SECTION,
    '',
    '## Memory and other forms of persistence',
    '- When to use or update a plan instead of memory...',
    '- When to use or update tasks instead of memory...',
    '',
    ...(extraGuidelines ?? []),
    '',
  ]

  lines.push(...buildSearchingPastContextSection(memoryDir))
  return lines
}
```

**SkipIndex Mode**: For assistant daily-log mode — no index maintenance, just append to log files.

**DIR_EXISTS_GUIDANCE (lines 116-119):**

```typescript
export const DIR_EXISTS_GUIDANCE =
  'This directory already exists — write to it directly with the Write tool (do not run mkdir or check for its existence).'
```

**Purpose**: Prevents wasted turns on `ls`/`mkdir -p` — harness ensures directory exists.

### 4.8 Assistant Daily Log Prompt (lines 327-370)

```typescript
function buildAssistantDailyLogPrompt(skipIndex = false): string {
  const memoryDir = getAutoMemPath()
  // Describe as pattern, not today's literal path (prompt cached across midnight)
  const logPathPattern = join(memoryDir, 'logs', 'YYYY', 'MM', 'YYYY-MM-DD.md')

  const lines: string[] = [
    '# auto memory',
    '',
    `You have a persistent, file-based memory system found at: \`${memoryDir}\``,
    '',
    "This session is long-lived. As you work, record anything worth remembering by **appending** to today's daily log file:",
    '',
    `\`${logPathPattern}\``,
    '',
    "Substitute today's date (from `currentDate` in your context) for `YYYY-MM-DD`...",
    '',
    'Write each entry as a short timestamped bullet. Create the file on first write...',
    '',
    '## What to log',
    '- User corrections and preferences...',
    '- Facts about the user, their role, or their goals',
    '- Project context not derivable from code...',
    '- Pointers to external systems...',
    '- Anything the user explicitly asks you to remember',
    '',
    ...WHAT_NOT_TO_SAVE_SECTION,
    '',
    ...(skipIndex ? [] : [`## ${ENTRYPOINT_NAME}`, ...]),
    ...buildSearchingPastContextSection(memoryDir),
  ]

  return lines.join('\n')
}
```

**Why Pattern Not Literal Path**: Prompt cached by `systemPromptSection('memory')` — NOT invalidated on date change. Model derives date from `currentDate` attachment.

**Append-Only**: "Do not rewrite or reorganize the log — it is append-only."

### 4.9 Memory Loading (`memdir.ts`)

**Load Memory Prompt (lines 419-507):**

```typescript
export async function loadMemoryPrompt(): Promise<string | null> {
  const autoEnabled = isAutoMemoryEnabled()

  const skipIndex = getFeatureValue_CACHED_MAY_BE_STALE('tengu_moth_copse', false)

  // KAIROS daily-log mode takes precedence
  if (feature('KAIROS') && autoEnabled && getKairosActive()) {
    logMemoryDirCounts(getAutoMemPath(), { memory_type: 'auto' })
    return buildAssistantDailyLogPrompt(skipIndex)
  }

  // Cowork extra guidelines from env var
  const coworkExtraGuidelines = process.env.CLAUDE_COWORK_MEMORY_EXTRA_GUIDELINES
  const extraGuidelines = coworkExtraGuidelines?.trim().length > 0 ? [coworkExtraGuidelines] : undefined

  if (feature('TEAMMEM')) {
    if (teamMemPaths!.isTeamMemoryEnabled()) {
      const autoDir = getAutoMemPath()
      const teamDir = teamMemPaths!.getTeamMemPath()
      await ensureMemoryDirExists(teamDir)
      logMemoryDirCounts(autoDir, { memory_type: 'auto' })
      logMemoryDirCounts(teamDir, { memory_type: 'team' })
      return teamMemPrompts!.buildCombinedMemoryPrompt(extraGuidelines, skipIndex)
    }
  }

  if (autoEnabled) {
    const autoDir = getAutoMemPath()
    await ensureMemoryDirExists(autoDir)
    logMemoryDirCounts(autoDir, { memory_type: 'auto' })
    return buildMemoryLines('auto memory', autoDir, extraGuidelines, skipIndex).join('\n')
  }

  logEvent('tengu_memdir_disabled', {
    disabled_by_env_var: isEnvTruthy(process.env.CLAUDE_CODE_DISABLE_AUTO_MEMORY),
    disabled_by_setting: getInitialSettings().autoMemoryEnabled === false,
  })
  
  if (getFeatureValue_CACHED_MAY_BE_STALE('tengu_herring_clock', false)) {
    logEvent('tengu_team_memdir_disabled', {})
  }
  return null
}
```

**Dispatch Logic**:
1. KAIROS + auto + active → daily log prompt
2. TEAMMEM + enabled → combined private + team prompt
3. Auto enabled → standard auto memory prompt
4. Disabled → return null, log telemetry

### 4.10 Memory Type Taxonomy (`memoryTypes.ts`)

**Four Types (lines 14-21):**

```typescript
export const MEMORY_TYPES = ['user', 'feedback', 'project', 'reference'] as const
export type MemoryType = (typeof MEMORY_TYPES)[number]

export function parseMemoryType(raw: unknown): MemoryType | undefined {
  if (typeof raw !== 'string') return undefined
  return MEMORY_TYPES.find(t => t === raw)
}
```

**Exclusion Guidance (lines 183-195):**

```typescript
export const WHAT_NOT_TO_SAVE_SECTION: readonly string[] = [
  '## What NOT to save in memory',
  '',
  '- Code patterns, conventions, architecture — derivable from current project state.',
  '- Git history, recent changes — `git log` / `git blame` are authoritative.',
  '- Debugging solutions or fix recipes — the fix is in the code.',
  '- Anything already documented in CLAUDE.md files.',
  '- Ephemeral task details: in-progress work, temporary state.',
  '',
  'These exclusions apply even when the user explicitly asks you to save.',
]
```

**Explicit-Save Gate**: Even if user asks to save PR list or activity summary, ask what was *surprising* or *non-obvious*.

**Trusting Recall Guidance (lines 240-256):**

```typescript
export const TRUSTING_RECALL_SECTION: readonly string[] = [
  '## Before recommending from memory',
  '',
  'A memory that names a specific function, file, or flag is a claim that it existed *when the memory was written*.',
  '',
  '- If the memory names a file path: check the file exists.',
  '- If the memory names a function or flag: grep for it.',
  '- If the user is about to act on your recommendation (not just asking about history), verify first.',
  '',
  '"The memory says X exists" is not the same as "X exists now."',
  '',
  'A memory that summarizes repo state is frozen in time. Prefer `git log` or reading code over recalling snapshot.',
]
```

**Eval-Validated**: This section went 0/3 → 3/3 when moved to appendSystemPrompt (position matters).

**Memory Drift Caveat (lines 201-202):**

```typescript
export const MEMORY_DRIFT_CAVEAT =
  '- Memory records can become stale over time. Use memory as context for what was true at a given point in time. Before answering, verify against current state. If conflict, trust current observation — update or remove stale memory.'
```

### 4.11 Searching Past Context (`memdir.ts`)

**Build Section (lines 375-407):**

```typescript
export function buildSearchingPastContextSection(autoMemDir: string): string[] {
  if (!getFeatureValue_CACHED_MAY_BE_STALE('tengu_coral_fern', false)) {
    return []
  }
  const projectDir = getProjectDir(getOriginalCwd())
  const embedded = hasEmbeddedSearchTools() || isReplModeEnabled()
  
  const memSearch = embedded
    ? `grep -rn "<search term>" ${autoMemDir} --include="*.md"`
    : `${GREP_TOOL_NAME} with pattern="<search term>" path="${autoMemDir}" glob="*.md"`
  const transcriptSearch = embedded
    ? `grep -rn "<search term>" ${projectDir}/ --include="*.jsonl"`
    : `${GREP_TOOL_NAME} with pattern="<search term>" path="${projectDir}/" glob="*.jsonl"`
  
  return [
    '## Searching past context',
    '',
    'When looking for past context:',
    '1. Search topic files in your memory directory:',
    '```',
    memSearch,
    '```',
    '2. Session transcript logs (last resort — large files, slow):',
    '```',
    transcriptSearch,
    '```',
    'Use narrow search terms (error messages, file paths, function names).',
    '',
  ]
}
```

**Feature Gated**: `tengu_coral_fern` gate controls whether this section appears.

**Embedded vs Tool**: Uses shell grep form when embedded tools available (ant-native, REPL mode).

---

## 5. Integration Points

### 5.1 With `utils/settings/settings.js`

| Component | Integration |
|-----------|-------------|
| `paths.ts` | Reads `autoMemoryDirectory`, `autoMemoryEnabled` from settings |

### 5.2 With `utils/git.js`

| Component | Integration |
|-----------|-------------|
| `paths.ts` | Uses `findCanonicalGitRoot()` for worktree sharing |

### 5.3 With `bootstrap/state.js`

| Component | Integration |
|-----------|-------------|
| `paths.ts` | Uses `getProjectRoot()`, `getIsNonInteractiveSession()` |
| `memdir.ts` | Uses `getKairosActive()`, `getOriginalCwd()` |

### 5.4 With `services/analytics/growthbook.js`

| Component | Integration |
|-----------|-------------|
| `paths.ts` | Checks `tengu_scratch`, `tengu_passport_quail`, `tengu_slate_thimble` |
| `memdir.ts` | Checks `tengu_moth_copse`, `tengu_coral_fern`, `tengu_herring_clock` |

### 5.5 With `tools/GrepTool/`

| Component | Integration |
|-----------|-------------|
| `memdir.ts` | References `GREP_TOOL_NAME` in search guidance |

---

## 6. Data Flow

### 6.1 Auto Memory Path Resolution

```
getAutoMemPath() called
         │
         ▼
  Check memoization cache (keyed on projectRoot)
         │
         ▼
  Check CLAUDE_COWORK_MEMORY_PATH_OVERRIDE
         │
         ▼
  Check settings.json autoMemoryDirectory
         │
         ▼
  Compute default: <memoryBase>/projects/<sanitized-git-root>/memory/
         │
         ▼
  Validate path (security checks)
         │
         ▼
  Return normalized path with trailing separator
         │
         ▼
  Cache result for future calls
```

### 6.2 Memory Prompt Loading

```
Session start
         │
         ▼
  loadMemoryPrompt()
         │
         ├──► isAutoMemoryEnabled()
         │    ├──► Env var check
         │    ├──► SIMPLE mode check
         │    ├──► CCR storage check
         │    └──► Settings.json check
         │
         ▼
  Check KAIROS + active
         │
         ├──► Yes: buildAssistantDailyLogPrompt()
         │
         ▼
  Check TEAMMEM + enabled
         │
         ├──► Yes: buildCombinedMemoryPrompt()
         │
         ▼
  Auto enabled
         │
         ├──► Yes: buildMemoryLines()
         │
         ▼
  Disabled: return null, log telemetry
```

### 6.3 Entrypoint Loading

```
buildMemoryPrompt() or loadMemoryPrompt()
         │
         ▼
  Read MEMORY.md (sync)
         │
         ▼
  truncateEntrypointContent(raw)
         │
         ├──► Check line cap (200)
         ├──► Check byte cap (25KB)
         ├──► Truncate at last newline if byte cap
         └──► Append warning if truncated
         │
         ▼
  Log telemetry (counts, truncation)
         │
         ▼
  Append to prompt lines
```

---

## 7. Key Patterns

### 7.1 Path Validation Security

```
Absolute + length ≥ 3 + no drive-root + no UNC + no null bytes = Safe path
```

**Defense in Depth**: Multiple independent checks, any failure rejects path.

### 7.2 Memoization Strategy

```typescript
export const getAutoMemPath = memoize(
  (): string => { /* compute */ },
  () => getProjectRoot(),  // Cache key
)
```

**Why ProjectRoot**: Path is per-project; changing project should invalidate cache.

### 7.3 Dual Caps (Lines + Bytes)

```
Line cap (200) catches typical bloat
Byte cap (25KB) catches long-line edge cases
```

**Why Both**: Line cap alone doesn't catch 197KB in 200 lines (observed p100).

### 7.4 Feature Gate Layering

```
Master gate (isAutoMemoryEnabled)
    └──► Mode gates (KAIROS, TEAMMEM)
        └──► Feature gates (skipIndex, coral_fern)
```

### 7.5 Append-Only Daily Log

```
Assistant mode: append to YYYY-MM-DD.md
Nightly: /dream distills to MEMORY.md + topic files
```

**Why**: Reduces write complexity — no index maintenance during session.

---

## 8. Environment Variables

| Variable | Purpose | Values |
|----------|---------|-------|
| `CLAUDE_CODE_DISABLE_AUTO_MEMORY` | Disable auto memory | `1`/`true` = off |
| `CLAUDE_CODE_SIMPLE` | Bare mode (disables memory) | `1`/`true` = bare |
| `CLAUDE_CODE_REMOTE` | Remote session mode | — |
| `CLAUDE_CODE_REMOTE_MEMORY_DIR` | Remote memory storage dir | Path |
| `CLAUDE_COWORK_MEMORY_PATH_OVERRIDE` | Cowork SDK override | Path |
| `CLAUDE_COWORK_MEMORY_EXTRA_GUIDELINES` | Extra guidelines text | String |

---

## 9. Feature Gates

| Gate | Purpose |
|------|---------|
| `tengu_passport_quail` | Extract mode enablement |
| `tengu_slate_thimble` | Extract in non-interactive sessions |
| `tengu_moth_copse` | SkipIndex mode (no index maintenance) |
| `tengu_coral_fern` | Searching past context section |
| `tengu_herring_clock` | Team memory cohort tracking |
| `KAIROS` | Assistant mode |
| `TEAMMEM` | Team memory feature |

---

## 10. Telemetry Events

| Event | Location | Fields |
|-------|----------|--------|
| `tengu_memdir_loaded` | `memdir.ts` | `content_length`, `line_count`, `was_truncated`, `memory_type` |
| `tengu_memdir_disabled` | `memdir.ts` | `disabled_by_env_var`, `disabled_by_setting` |
| `tengu_team_memdir_disabled` | `memdir.ts` | — |

---

## 11. Summary

The `memdir/` module implements Claude Code's **persistent memory infrastructure**:

1. **Path Resolution** — Secure, memoized path computation with override support
2. **Type Taxonomy** — Four-type system (user/feedback/project/reference)
3. **Prompt Building** — Comprehensive instructions for saving/accessing memories
4. **Assistant Mode** — Append-only daily logs with nightly distillation
5. **Entrypoint Loading** — Dual-capped truncation (lines + bytes)
6. **Team Memory** — Private + team directory integration

**Key Security Decisions**:
- **Path validation**: Absolute, no UNC, no null bytes, no drive-root
- **Tilde expansion**: Only for settings.json, not env vars
- **Project exclusion**: projectSettings can't set sensitive paths

**Key Architectural Decisions**:
- **Memoization**: Avoids repeated settings file reads
- **Dual caps**: Catches both typical and edge-case bloat
- **Feature layering**: Master gate → mode gates → feature gates
- **Append-only logs**: Simplifies assistant mode writes

---

**Last Updated:** 2026-04-07  
**Status:** Complete — 4 core files analyzed (teamMemPaths.ts/teamMemPrompts.ts feature-gated)
