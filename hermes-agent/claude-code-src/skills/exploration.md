# Skills Module — Deep-Dive Exploration

**Module:** `skills/`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/skills/`  
**Files:** 20 TypeScript files (3 core files + 17 bundled skills)  
**Created:** 2026-04-07

---

## 1. Module Overview

The `skills/` module implements **Claude Code skills system** — a framework for creating reusable, parameterized workflows that can be invoked via `/skill-name` commands. Skills are markdown-based definitions with frontmatter configuration that enable automation of repeatable processes.

### Core Responsibilities

1. **Skill Loading** — Multi-source skill discovery:
   - `/skills/` directory format (skill-name/SKILL.md)
   - Legacy `/commands/` directory format
   - Bundled skills (compiled into CLI)
   - MCP server-provided skills
   - Plugin skills

2. **Frontmatter Parsing** — Skill configuration extraction:
   - Name, description, when_to_use
   - allowed-tools, argument-hint, arguments
   - model, context (inline/fork), agent
   - hooks, effort, shell configuration
   - paths for skill scoping

3. **Skill Execution** — Prompt preparation:
   - Argument substitution (${arg_name})
   - Shell command execution (!`...`)
   - CLAUDE_SKILL_DIR and CLAUDE_SESSION_ID substitution
   - Base directory prefix for file operations

4. **Bundled Skills** — Built-in workflow automation:
   - /remember (memory review and promotion)
   - /skillify (capture session as skill)
   - /simplify, /debug, /verify, /stuck
   - /keybindings, /lorem-ipsum, /batch
   - Feature-gated skills: /dream, /hunter, /loop

### Key Design Patterns

- **Directory Format**: skill-name/SKILL.md structure
- **Frontmatter Configuration**: YAML-like metadata for skill behavior
- **Argument Substitution**: Template-style parameter injection
- **Security Hardening**: Path traversal prevention, O_NOFOLLOW for file writes
- **Idempotent Extraction**: Bundled skill files extracted on first invocation

---

## 2. File Inventory

| File | Lines | Description |
|------|-------|-------------|
| `bundledSkills.ts` | ~220 | Bundled skill registration API |
| `loadSkillsDir.ts` | ~850+ | Skill directory loading and parsing |
| `mcpSkillBuilders.ts` | ~45 | MCP skill builder registry (cycle breaker) |
| `bundled/index.ts` | ~80 | Bundled skill initializer |
| `bundled/remember.ts` | ~83 | Memory review skill |
| `bundled/skillify.ts` | ~198 | Session-to-skill capture |
| `bundled/updateConfig.ts` | ~430 | Config update skill |
| `bundled/keybindings.ts` | ~260 | Keybinding management skill |
| `bundled/simplify.ts` | ~110 | Simplification skill |
| `bundled/debug.ts` | ~105 | Debug skill |
| `bundled/verify.ts` | ~23 | Verification skill |
| `bundled/loremIpsum.ts` | ~110 | Lorem ipsum generator |
| `bundled/batch.ts` | ~180 | Batch processing skill |
| `bundled/stuck.ts` | ~105 | Unstuck skill |
| `bundled/claudeApi.ts` | ~155 | Claude API skill |
| `bundled/scheduleRemoteAgents.ts` | ~470 | Remote agent scheduling |

**Total:** ~3000+ lines across 20 files

---

## 3. Key Exports

### Bundled Skill Registration (`bundledSkills.ts`)

```typescript
// Definition for a bundled skill
export type BundledSkillDefinition = {
  name: string
  description: string
  aliases?: string[]
  whenToUse?: string
  argumentHint?: string
  allowedTools?: string[]
  model?: string
  disableModelInvocation?: boolean
  userInvocable?: boolean
  isEnabled?: () => boolean
  hooks?: HooksSettings
  context?: 'inline' | 'fork'
  agent?: string
  files?: Record<string, string>  // Reference files to extract
  getPromptForCommand: (
    args: string,
    context: ToolUseContext,
  ) => Promise<ContentBlockParam[]>
}

export function registerBundledSkill(definition: BundledSkillDefinition): void
export function getBundledSkills(): Command[]
export function getBundledSkillExtractDir(skillName: string): string
```

### Skill Loading (`loadSkillsDir.ts`)

```typescript
// Setting source for skills
export type LoadedFrom =
  | 'commands_DEPRECATED'
  | 'skills'
  | 'plugin'
  | 'managed'
  | 'bundled'
  | 'mcp'

export function getSkillsPath(
  source: SettingSource | 'plugin',
  dir: 'skills' | 'commands',
): string

export function estimateSkillFrontmatterTokens(skill: Command): number

export function parseSkillFrontmatterFields(
  frontmatter: FrontmatterData,
  markdownContent: string,
  resolvedName: string,
  descriptionFallbackLabel: 'Skill' | 'Custom command',
): {
  displayName: string | undefined
  description: string
  hasUserSpecifiedDescription: boolean
  allowedTools: string[]
  argumentHint: string | undefined
  argumentNames: string[]
  whenToUse: string | undefined
  model: ReturnType<typeof parseUserSpecifiedModel> | undefined
  disableModelInvocation: boolean
  userInvocable: boolean
  hooks: HooksSettings | undefined
  executionContext: 'fork' | undefined
  agent: string | undefined
  effort: EffortValue | undefined
  shell: FrontmatterShell | undefined
}

export function createSkillCommand({...}): Command
```

---

## 4. Line-by-Line Analysis

### 4.1 Bundled Skill Registration (`bundledSkills.ts` lines 53-100)

```typescript
export function registerBundledSkill(definition: BundledSkillDefinition): void {
  const { files } = definition

  let skillRoot: string | undefined
  let getPromptForCommand = definition.getPromptForCommand

  // Extract reference files on first invocation
  if (files && Object.keys(files).length > 0) {
    skillRoot = getBundledSkillExtractDir(definition.name)
    let extractionPromise: Promise<string | null> | undefined
    const inner = definition.getPromptForCommand
    getPromptForCommand = async (args, ctx) => {
      extractionPromise ??= extractBundledSkillFiles(definition.name, files)
      const extractedDir = await extractionPromise
      const blocks = await inner(args, ctx)
      if (extractedDir === null) return blocks
      return prependBaseDir(blocks, extractedDir)
    }
  }

  const command: Command = {
    type: 'prompt',
    name: definition.name,
    description: definition.description,
    aliases: definition.aliases,
    hasUserSpecifiedDescription: true,
    allowedTools: definition.allowedTools ?? [],
    // ... rest of command definition
    getPromptForCommand,
  }
  bundledSkills.push(command)
}
```

**Lazy File Extraction**: Files extracted on first invocation, not at registration. Promise memoization prevents race conditions.

### 4.2 Security: Safe File Writes (`bundledSkills.ts` lines 176-193)

```typescript
const O_NOFOLLOW = fsConstants.O_NOFOLLOW ?? 0
// On Windows, use string flags — numeric O_EXCL can produce EINVAL
const SAFE_WRITE_FLAGS =
  process.platform === 'win32'
    ? 'wx'
    : fsConstants.O_WRONLY |
      fsConstants.O_CREAT |
      fsConstants.O_EXCL |
      O_NOFOLLOW

async function safeWriteFile(p: string, content: string): Promise<void> {
  const fh = await open(p, SAFE_WRITE_FLAGS, 0o600)
  try {
    await fh.writeFile(content, 'utf8')
  } finally {
    await fh.close()
  }
}
```

**Security Design**: "The per-process nonce in getBundledSkillsRoot() is the primary defense against pre-created symlinks/dirs. Explicit 0o700/0o600 modes keep the nonce subtree owner-only even on umask=0, so an attacker who learns the nonce via inotify on the predictable parent still can't write into it."

### 4.3 Path Validation (`bundledSkills.ts` lines 196-206)

```typescript
function resolveSkillFilePath(baseDir: string, relPath: string): string {
  const normalized = normalize(relPath)
  if (
    isAbsolute(normalized) ||
    normalized.split(pathSep).includes('..') ||
    normalized.split('/').includes('..')
  ) {
    throw new Error(`bundled skill file path escapes skill dir: ${relPath}`)
  }
  return join(baseDir, normalized)
}
```

**Traversal Prevention**: Rejects absolute paths, `..` components in both path separators.

### 4.4 Skill Frontmatter Parsing (`loadSkillsDir.ts` lines 185-265)

```typescript
export function parseSkillFrontmatterFields(
  frontmatter: FrontmatterData,
  markdownContent: string,
  resolvedName: string,
  descriptionFallbackLabel: 'Skill' | 'Custom command',
): {...} {
  const validatedDescription = coerceDescriptionToString(
    frontmatter.description,
    resolvedName,
  )
  const description =
    validatedDescription ??
    extractDescriptionFromMarkdown(markdownContent, descriptionFallbackLabel)

  const userInvocable =
    frontmatter['user-invocable'] === undefined
      ? true
      : parseBooleanFrontmatter(frontmatter['user-invocable'])

  const model =
    frontmatter.model === 'inherit'
      ? undefined
      : frontmatter.model
        ? parseUserSpecifiedModel(frontmatter.model as string)
        : undefined

  // ... effort, hooks, executionContext, agent, shell parsing
}
```

**Description Fallback**: Uses frontmatter description, or extracts from markdown content.

### 4.5 Skill Command Creation (`loadSkillsDir.ts` lines 270-401)

```typescript
export function createSkillCommand({...}): Command {
  return {
    type: 'prompt',
    name: skillName,
    description,
    hasUserSpecifiedDescription,
    allowedTools,
    argumentHint,
    argNames: argumentNames.length > 0 ? argumentNames : undefined,
    whenToUse,
    model,
    disableModelInvocation,
    userInvocable,
    context: executionContext,
    agent,
    contentLength: markdownContent.length,
    isHidden: !userInvocable,
    progressMessage: 'running',
    userFacingName(): string {
      return displayName || skillName
    },
    source,
    loadedFrom,
    hooks,
    skillRoot: baseDir,
    async getPromptForCommand(args, toolUseContext) {
      let finalContent = baseDir
        ? `Base directory for this skill: ${baseDir}\n\n${markdownContent}`
        : markdownContent

      // Argument substitution
      finalContent = substituteArguments(
        finalContent,
        args,
        true,
        argumentNames,
      )

      // CLAUDE_SKILL_DIR substitution for bash injection
      if (baseDir) {
        const skillDir =
          process.platform === 'win32' ? baseDir.replace(/\\/g, '/') : baseDir
        finalContent = finalContent.replace(/\$\{CLAUDE_SKILL_DIR\}/g, skillDir)
      }

      // CLAUDE_SESSION_ID substitution
      finalContent = finalContent.replace(
        /\$\{CLAUDE_SESSION_ID\}/g,
        getSessionId(),
      )

      // Security: MCP skills are remote and untrusted — never execute inline
      if (loadedFrom !== 'mcp') {
        finalContent = await executeShellCommandsInPrompt(
          finalContent,
          {...toolUseContext},
          `/${skillName}`,
          shell,
        )
      }

      return [{ type: 'text', text: finalContent }]
    },
  } satisfies Command
}
```

**Security Gate**: "Security: MCP skills are remote and untrusted — never execute inline shell commands (!`...`) from their markdown body."

### 4.6 Skills Directory Loading (`loadSkillsDir.ts` lines 407-480)

```typescript
async function loadSkillsFromSkillsDir(
  basePath: string,
  source: SettingSource,
): Promise<SkillWithPath[]> {
  const fs = getFsImplementation()

  let entries
  try {
    entries = await fs.readdir(basePath)
  } catch (e: unknown) {
    if (!isFsInaccessible(e)) logError(e)
    return []
  }

  const results = await Promise.all(
    entries.map(async (entry): Promise<SkillWithPath | null> => {
      try {
        // Only support directory format: skill-name/SKILL.md
        if (!entry.isDirectory() && !entry.isSymbolicLink()) {
          return null  // Single .md files NOT supported in /skills/
        }

        const skillDirPath = join(basePath, entry.name)
        const skillFilePath = join(skillDirPath, 'SKILL.md')

        let content: string
        try {
          content = await fs.readFile(skillFilePath, { encoding: 'utf-8' })
        } catch (e: unknown) {
          if (!isENOENT(e)) {
            logForDebugging(`[skills] failed to read ${skillFilePath}: ${e}`, {
              level: 'warn',
            })
          }
          return null
        }

        const { frontmatter, content: markdownContent } = parseFrontmatter(
          content,
          skillFilePath,
        )

        const skillName = entry.name
        const parsed = parseSkillFrontmatterFields(...)
        const paths = parseSkillPaths(frontmatter)

        return {
          skill: createSkillCommand({
            ...parsed,
            skillName,
            markdownContent,
            source,
            baseDir: skillDirPath,
            loadedFrom: 'skills',
            paths,
          }),
          filePath: skillFilePath,
        }
      } catch (error) {
        logError(error)
        return null
      }
    }),
  )

  return results.filter((r): r is SkillWithPath => r !== null)
}
```

**Directory Format Only**: `/skills/` only supports `skill-name/SKILL.md` format, not single `.md` files.

### 4.7 Remember Skill (`bundled/remember.ts`)

```typescript
export function registerRememberSkill(): void {
  if (process.env.USER_TYPE !== 'ant') {
    return  // Anthropic employees only
  }

  registerBundledSkill({
    name: 'remember',
    description:
      'Review auto-memory entries and propose promotions to CLAUDE.md, CLAUDE.local.md, or shared memory.',
    whenToUse:
      'Use when the user wants to review, organize, or promote their auto-memory entries.',
    userInvocable: true,
    isEnabled: () => isAutoMemoryEnabled(),
    async getPromptForCommand(args) {
      let prompt = SKILL_PROMPT
      if (args) {
        prompt += `\n## Additional context from user\n\n${args}`
      }
      return [{ type: 'text', text: prompt }]
    },
  })
}
```

**Memory Classification**: The skill classifies entries into CLAUDE.md, CLAUDE.local.md, team memory, or stay in auto-memory.

### 4.8 Skillify Skill (`bundled/skillify.ts`)

```typescript
export function registerSkillifySkill(): void {
  if (process.env.USER_TYPE !== 'ant') {
    return  // Anthropic employees only
  }

  registerBundledSkill({
    name: 'skillify',
    description:
      "Capture this session's repeatable process into a skill. Call at end of the process you want to capture with an optional description.",
    allowedTools: [
      'Read', 'Write', 'Edit', 'Glob', 'Grep',
      'AskUserQuestion',
      'Bash(mkdir:*)',
    ],
    userInvocable: true,
    disableModelInvocation: true,  // Uses AskUserQuestion internally
    argumentHint: '[description of the process you want to capture]',
    async getPromptForCommand(args, context) {
      const sessionMemory =
        (await getSessionMemoryContent()) ?? 'No session memory available.'
      const userMessages = extractUserMessages(
        getMessagesAfterCompactBoundary(context.messages),
      )

      const userDescriptionBlock = args
        ? `The user described this process as: "${args}"`
        : ''

      const prompt = SKILLIFY_PROMPT.replace(...)
      return [{ type: 'text', text: prompt }]
    },
  })
}
```

**Session Capture**: Uses session memory and user messages to interview user and generate SKILL.md.

---

## 5. Integration Points

### 5.1 With `utils/frontmatterParser.js`

| Component | Integration |
|-----------|-------------|
| `loadSkillsDir.ts` | Uses `parseFrontmatter()`, `parseBooleanFrontmatter()` |

### 5.2 With `utils/markdownConfigLoader.js`

| Component | Integration |
|-----------|-------------|
| `loadSkillsDir.ts` | Uses `loadMarkdownFilesForSubdir()`, `extractDescriptionFromMarkdown()` |

### 5.3 With `utils/settings/types.js`

| Component | Integration |
|-----------|-------------|
| `bundledSkills.ts` | Uses `HooksSchema`, `HooksSettings` |

### 5.4 With `utils/argumentSubstitution.js`

| Component | Integration |
|-----------|-------------|
| `loadSkillsDir.ts` | Uses `parseArgumentNames()`, `substituteArguments()` |

### 5.5 With `utils/promptShellExecution.js`

| Component | Integration |
|-----------|-------------|
| `loadSkillsDir.ts` | Uses `executeShellCommandsInPrompt()` |

### 5.6 With `utils/permissions/filesystem.js`

| Component | Integration |
|-----------|-------------|
| `bundledSkills.ts` | Uses `getBundledSkillsRoot()` |

---

## 6. Data Flow

### 6.1 Skill Loading Flow

```
Startup
    │
    ▼
loadSkillsDir.ts module init
    │
    ├──► loadSkillsFromSkillsDir(~/.claude/skills)
    ├──► loadSkillsFromSkillsDir(.claude/skills)
    ├──► loadSkillsFromCommandsDir(~/.claude/commands)  [legacy]
    ├──► loadSkillsFromCommandsDir(.claude/commands)  [legacy]
    └──► getBundledSkills()
    │
    ▼
Merge all skills into Command[] registry
```

### 6.2 Skill Invocation Flow

```
User types /skill-name args
    │
    ▼
resolveKey() finds skill command
    │
    ▼
getPromptForCommand(args, context)
    │
    ├──► Add base directory prefix (if files extracted)
    ├──► Substitute arguments (${arg_name})
    ├──► Substitute CLAUDE_SKILL_DIR
    ├──► Substitute CLAUDE_SESSION_ID
    ├──► Execute shell commands (!`...`) [non-MCP only]
    │
    ▼
Return ContentBlockParam[] for model
```

### 6.3 Bundled Skill File Extraction

```
First invocation of bundled skill with files
    │
    ▼
getPromptForCommand() called
    │
    ▼
extractBundledSkillFiles() (memoized promise)
    │
    ├──► getBundledSkillExtractDir(skillName)
    ├──► writeSkillFiles() with O_NOFOLLOW|O_EXCL
    └──► Return extracted dir path
    │
    ▼
prependBaseDir() adds "Base directory: <dir>" prefix
```

---

## 7. Key Patterns

### 7.1 Directory Format

```
~/.claude/skills/
├── skill-name/
│   └── SKILL.md
└── another-skill/
    └── SKILL.md
```

**Why**: Provides skillRoot directory for file operations, keeps related files together.

### 7.2 Frontmatter Configuration

```markdown
---
name: Display Name
description: One-line description
allowed-tools:
  - Read
  - Bash(git *)
when_to_use: Use when...
argument-hint: "<arg1> [arg2]"
arguments:
  - arg1
  - arg2
context: fork  # or omit for inline
model: claude-sonnet-4-6
disable-model-invocation: false
user-invocable: true
hooks:
  - matcher: "Write"
    hooks: [...]
---

# Skill content with ${arg1} substitution
```

### 7.3 Security Hardening

```typescript
// O_NOFOLLOW | O_EXCL prevents symlink attacks
const SAFE_WRITE_FLAGS =
  process.platform === 'win32'
    ? 'wx'
    : fsConstants.O_WRONLY |
      fsConstants.O_CREAT |
      fsConstants.O_EXCL |
      O_NOFOLLOW

// Path traversal prevention
if (normalized.split(pathSep).includes('..')) {
  throw new Error(`bundled skill file path escapes skill dir`)
}

// MCP skills: no shell execution
if (loadedFrom !== 'mcp') {
  finalContent = await executeShellCommandsInPrompt(...)
}
```

### 7.4 Argument Substitution

```typescript
// In skill markdown:
## Inputs
- `$pr_number`: PR number to process
- `$target_branch`: Target branch

// Usage:
/pr-process 123 main

// Becomes:
## Inputs
- `123`: PR number to process
- `main`: Target branch
```

---

## 8. Environment Variables

| Variable | Purpose | Values |
|----------|---------|--------|
| `USER_TYPE` | Employee gating for bundled skills | `'ant'` for Anthropic employees |
| `CLAUDE_SKILL_DIR` | Substituted at runtime | Skill's extraction directory |
| `CLAUDE_SESSION_ID` | Substituted at runtime | Current session ID |

---

## 9. Feature Gates

| Feature | Skills Affected | Purpose |
|---------|-----------------|---------|
| `KAIROS` | /dream | Dream skill for Kairos feature |
| `KAIROS_DREAM` | /dream | Alternative gate for dream skill |
| `REVIEW_ARTIFACT` | /hunter | Hunter skill for artifact review |
| `AGENT_TRIGGERS` | /loop | Loop skill for scheduled tasks |
| `AGENT_TRIGGERS_REMOTE` | /schedule-remote-agents | Remote agent scheduling |
| `BUILDING_CLAUDE_APPS` | /claude-api | Claude API skill |
| `RUN_SKILL_GENERATOR` | /run-skill-generator | Skill generator skill |

---

## 10. Bundled Skills Catalog

| Skill | Description | User Invocable |
|-------|-------------|----------------|
| `/remember` | Review auto-memory, propose promotions | Yes (Ant only) |
| `/skillify` | Capture session as reusable skill | Yes (Ant only) |
| `/update-config` | Update CLI configuration | Yes |
| `/keybindings` | Manage keybindings | Yes |
| `/simplify` | Simplify complex output | Yes |
| `/debug` | Debug skill | Yes |
| `/verify` | Verification skill | Yes |
| `/lorem-ipsum` | Generate lorem ipsum text | Yes |
| `/batch` | Batch processing | Yes |
| `/stuck` | Help when stuck | Yes |
| `/claude-api` | Claude API operations | Yes |
| `/schedule-remote-agents` | Schedule remote agents | Yes |
| `/dream` | Dream feature (feature-gated) | Yes |
| `/hunter` | Artifact hunter (feature-gated) | Yes |
| `/loop` | Loop/automation (feature-gated) | Yes |

---

## 11. Summary

The `skills/` module provides **reusable workflow automation**:

1. **Multi-Source Loading** — Skills from directories, bundled, MCP, plugins
2. **Frontmatter Configuration** — Rich metadata for skill behavior
3. **Argument Substitution** — Template-style parameterization
4. **Security Hardening** — Path validation, safe file writes, MCP isolation
5. **Bundled Skills** — 15+ built-in workflow automations

**Key Design Decisions**:
- **Directory format** enables skillRoot for file operations
- **Lazy extraction** for bundled skill files (on first invocation)
- **MCP isolation** — no shell execution for remote skills
- **Idempotent writes** — O_NOFOLLOW|O_EXCL prevents symlink attacks

---

**Last Updated:** 2026-04-07  
**Status:** Complete — All 20 files analyzed
