# Output Styles Module — Deep-Dive Exploration

**Module:** `outputStyles/`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/outputStyles/`  
**Files:** 1 TypeScript file  
**Created:** 2026-04-07

---

## 1. Module Overview

The `outputStyles/` module implements **custom output style loading** — enabling users to define personalized output formats for Claude Code's responses via markdown configuration files. Output styles can modify how code blocks, tool outputs, and assistant messages are rendered in the terminal.

### Core Responsibilities

1. **Style Discovery** — Multi-source style loading:
   - Project styles: `.claude/output-styles/*.md`
   - User styles: `~/.claude/output-styles/*.md`
   - Plugin styles: Loaded via plugin system

2. **Frontmatter Parsing** — Style metadata extraction:
   - Name and description
   - `keep-coding-instructions` flag
   - `force-for-plugin` flag (plugin styles only)

3. **Style Application** — Output transformation:
   - Custom rendering rules
   - Instruction preservation options
   - Override semantics (project > user)

### Key Design Patterns

- **Memoized Loading**: `memoize()` for efficient repeated access
- **Frontmatter Configuration**: YAML-like metadata in markdown
- **Source Priority**: Project styles override user styles
- **Cache Management**: Explicit cache clearing for hot reload

---

## 2. File Inventory

| File | Lines | Description |
|------|-------|-------------|
| `loadOutputStylesDir.ts` | ~99 | Output style directory loading |

**Total:** ~99 lines

---

## 3. Key Exports

```typescript
// Load styles from output-styles directories
export const getOutputStyleDirStyles: (
  cwd: string
) => Promise<OutputStyleConfig[]>

// Clear all output style caches
export function clearOutputStyleCaches(): void
```

### Output Style Config Type

```typescript
export type OutputStyleConfig = {
  name: string
  description: string
  prompt: string  // Full markdown content
  source: SettingSource
  keepCodingInstructions?: boolean
}
```

---

## 4. Line-by-Line Analysis

### 4.1 Style Loading Function (`loadOutputStylesDir.ts` lines 26-92)

```typescript
export const getOutputStyleDirStyles = memoize(
  async (cwd: string): Promise<OutputStyleConfig[]> => {
    try {
      const markdownFiles = await loadMarkdownFilesForSubdir(
        'output-styles',
        cwd,
      )

      const styles = markdownFiles
        .map(({ filePath, frontmatter, content, source }) => {
          try {
            const fileName = basename(filePath)
            const styleName = fileName.replace(/\.md$/, '')

            // Get style configuration from frontmatter
            const name = (frontmatter['name'] || styleName) as string
            const description =
              coerceDescriptionToString(
                frontmatter['description'],
                styleName,
              ) ??
              extractDescriptionFromMarkdown(
                content,
                `Custom ${styleName} output style`,
              )

            // Parse keep-coding-instructions flag
            const keepCodingInstructionsRaw =
              frontmatter['keep-coding-instructions']
            const keepCodingInstructions =
              keepCodingInstructionsRaw === true ||
              keepCodingInstructionsRaw === 'true'
                ? true
                : keepCodingInstructionsRaw === false ||
                    keepCodingInstructionsRaw === 'false'
                  ? false
                  : undefined

            // Warn if force-for-plugin is set on non-plugin output style
            if (frontmatter['force-for-plugin'] !== undefined) {
              logForDebugging(
                `Output style "${name}" has force-for-plugin set, but this option only applies to plugin output styles. Ignoring.`,
                { level: 'warn' },
              )
            }

            return {
              name,
              description,
              prompt: content.trim(),
              source,
              keepCodingInstructions,
            }
          } catch (error) {
            logError(error)
            return null
          }
        })
        .filter(style => style !== null)

      return styles
    } catch (error) {
      logError(error)
      return []
    }
  },
)
```

**Memoization**: `memoize()` caches result per `cwd` for efficient repeated calls.

**Name Fallback**: Uses filename (without `.md`) if `name` frontmatter not provided.

**Description Fallback**: Extracts from markdown content if not in frontmatter.

### 4.2 Keep Coding Instructions Parsing (`loadOutputStylesDir.ts` lines 53-62)

```typescript
// Parse keep-coding-instructions flag (supports both boolean and string values)
const keepCodingInstructionsRaw =
  frontmatter['keep-coding-instructions']
const keepCodingInstructions =
  keepCodingInstructionsRaw === true ||
  keepCodingInstructionsRaw === 'true'
    ? true
    : keepCodingInstructionsRaw === false ||
        keepCodingInstructionsRaw === 'false'
      ? false
      : undefined
```

**Flexible Parsing**: Accepts YAML boolean (`true`/`false`) or string (`'true'`/`'false'`).

**Undefined Default**: If not specified, value is `undefined` (not `false`).

### 4.3 Force-for-Plugin Warning (`loadOutputStylesDir.ts` lines 65-70)

```typescript
// Warn if force-for-plugin is set on non-plugin output style
if (frontmatter['force-for-plugin'] !== undefined) {
  logForDebugging(
    `Output style "${name}" has force-for-plugin set, but this option only applies to plugin output styles. Ignoring.`,
    { level: 'warn' },
  )
}
```

**Plugin-Only Flag**: `force-for-plugin` is ignored for non-plugin styles.

**Warning Log**: Debug log helps users understand flag is being ignored.

### 4.4 Cache Clearing (`loadOutputStylesDir.ts` lines 94-98)

```typescript
export function clearOutputStyleCaches(): void {
  getOutputStyleDirStyles.cache?.clear?.()
  loadMarkdownFilesForSubdir.cache?.clear?.()
  clearPluginOutputStyleCache()
}
```

**Triple Cache Clear**:
1. Main style loader cache
2. Markdown file loader cache
3. Plugin output style cache

**Usage**: Called when styles are added/modified to force reload.

---

## 5. Integration Points

### 5.1 With `utils/markdownConfigLoader.js`

| Component | Integration |
|-----------|-------------|
| `loadOutputStylesDir.ts` | Uses `loadMarkdownFilesForSubdir()`, `extractDescriptionFromMarkdown()` |

### 5.2 With `utils/frontmatterParser.js`

| Component | Integration |
|-----------|-------------|
| `loadOutputStylesDir.ts` | Uses `coerceDescriptionToString()` |

### 5.3 With `utils/plugins/loadPluginOutputStyles.js`

| Component | Integration |
|-----------|-------------|
| `clearOutputStyleCaches()` | Uses `clearPluginOutputStyleCache()` |

### 5.4 With `constants/outputStyles.js`

| Component | Integration |
|-----------|-------------|
| `loadOutputStylesDir.ts` | Uses `OutputStyleConfig` type |

---

## 6. Data Flow

### 6.1 Style Loading Flow

```
Startup or style refresh
    │
    ▼
getOutputStyleDirStyles(cwd)
    │
    ├──► loadMarkdownFilesForSubdir('output-styles', cwd)
    │    │
    │    ├──► ~/.claude/output-styles/*.md
    │    └──► .claude/output-styles/*.md
    │
    ├──► For each markdown file:
    │    ├──► Parse frontmatter
    │    ├──► Extract name, description
    │    ├──► Parse keep-coding-instructions
    │    └──► Return OutputStyleConfig
    │
    ▼
Return OutputStyleConfig[] (memoized)
```

### 6.2 Style Override Flow

```
User styles loaded
    │
    └──► [{name: 'compact', ...}]
    
Project styles loaded
    │
    └──► [{name: 'compact', ...}]
    
Merge with project override
    │
    └──► Project 'compact' replaces user 'compact'
```

**Override Semantics**: Project styles take precedence over user styles.

---

## 7. Key Patterns

### 7.1 Directory Structure

```
~/.claude/output-styles/
├── compact.md       # User compact style
└── detailed.md      # User detailed style

.claude/output-styles/
├── compact.md       # Project compact style (overrides user)
└── team-style.md    # Project-specific style
```

### 7.2 Frontmatter Format

```markdown
---
name: Display Name
description: One-line description
keep-coding-instructions: true  # or false, 'true', 'false'
---

# Style prompt content

This defines how output should be formatted...
```

### 7.3 Memoization Pattern

```typescript
export const getOutputStyleDirStyles = memoize(
  async (cwd: string): Promise<OutputStyleConfig[]> => {
    // ... loading logic
  },
)

// Cache cleared on style changes
export function clearOutputStyleCaches(): void {
  getOutputStyleDirStyles.cache?.clear?.()
  // ...
}
```

---

## 8. Example Style Files

### 8.1 Compact Style

```markdown
---
name: Compact
description: Minimal output with abbreviated explanations
keep-coding-instructions: false
---

# Compact Output Style

- Show code blocks without additional explanation
- Omit "I've..." phrasing
- Use terse responses
- Skip pleasantries
```

### 8.2 Detailed Style

```markdown
---
name: Detailed
description: Verbose explanations with step-by-step reasoning
keep-coding-instructions: true
---

# Detailed Output Style

- Explain reasoning before code
- Include comments in code blocks
- Summarize changes after
- Keep coding instructions visible
```

### 8.3 Team Style

```markdown
---
name: Team Standard
description: Company-mandated output format
keep-coding-instructions: true
---

# Team Output Standard

- Always include function signatures
- Use JSDoc comments
- Follow team naming conventions
- Include error handling examples
```

---

## 9. Summary

The `outputStyles/` module provides **custom output format loading**:

1. **Multi-Source Loading** — User and project style directories
2. **Frontmatter Configuration** — Name, description, flags
3. **Memoized Access** — Efficient repeated calls
4. **Cache Management** — Explicit clearing for hot reload

**Key Design Decisions**:
- **Memoized loading** for performance
- **Frontmatter metadata** for style configuration
- **Project override** semantics
- **Flexible boolean parsing** for YAML compatibility

**Style Structure**:
- Filename becomes style identifier
- Frontmatter provides metadata
- Content defines rendering rules
- `keep-coding-instructions` controls instruction preservation

---

**Last Updated:** 2026-04-07  
**Status:** Complete — 1 of 1 files analyzed
