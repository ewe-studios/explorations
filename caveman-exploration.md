# Exploration: Caveman — Lithic Token Compression for AI Agents

**Source**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AIResearch/caveman`
**Author**: Julius Brussee (GitHub: JuliusBrussee/caveman)
**License**: MIT

---

## 1. What Is Caveman?

Caveman is a prompt-engineering skill/plugin for AI coding agents (Claude Code, Codex, Gemini CLI, Cursor, Windsurf, Cline, Copilot, and 40+ others) that forces the LLM to communicate in compressed, caveman-like speech — cutting **~75% of output tokens** while retaining 100% technical accuracy.

The core insight: most LLM output tokens are filler — articles, hedging, pleasantries, sycophantic padding. Removing these produces identical technical substance in a fraction of the tokens, resulting in faster responses, lower cost, and improved readability.

This is backed by research: the March 2026 paper *"Brevity Constraints Reverse Performance Hierarchies in Language Models"* (arXiv:2604.00025) found that constraining large models to brief responses improved accuracy by 26 percentage points on certain benchmarks.

### The Pitch

```
Normal Claude (69 tokens):
"The reason your React component is re-rendering is likely because you're
creating a new object reference on each render cycle. When you pass an inline
object as a prop, React's shallow comparison sees it as a different object
every time, which triggers a re-render. I'd recommend using useMemo to
memoize the object."

Caveman Claude (19 tokens):
"New object ref each render. Inline object prop = new ref = re-render.
Wrap in `useMemo`."
```

Same fix. 75% less word.

---

## 2. Architecture Overview

```
caveman/
├── caveman/                    # Plugin bundle (contains packaged SKILL.md)
│   └── SKILL.md                # Canonical caveman mode skill (the core prompt)
├── skills/                     # Individual skills (source of truth)
│   ├── caveman/SKILL.md        # Main caveman mode
│   ├── caveman-commit/SKILL.md # Terse commit message generator
│   ├── caveman-review/SKILL.md # One-line code review comments
│   └── compress/               # Input token compression tool
│       ├── SKILL.md
│       └── scripts/            # Python CLI for compression
│           ├── __main__.py
│           ├── cli.py
│           ├── compress.py
│           ├── detect.py
│           └── validate.py
├── plugins/                    # Claude Code plugin packaging
│   └── caveman/
│       ├── assets/
│       └── skills/
├── hooks/                      # Session lifecycle hooks (JS)
│   ├── caveman-activate.js     # SessionStart: activate caveman mode
│   ├── caveman-mode-tracker.js # UserPromptSubmit: track intensity level
│   ├── caveman-statusline.sh   # Statusline badge (bash)
│   ├── caveman-statusline.ps1  # Statusline badge (PowerShell)
│   ├── install.sh / install.ps1
│   └── uninstall.sh / uninstall.ps1
├── commands/                   # Claude Code slash command definitions
│   ├── caveman.toml
│   ├── caveman-commit.toml
│   └── caveman-review.toml
├── rules/                      # Agent activation rules
│   └── caveman-activate.md
├── benchmarks/                 # Token benchmarking harness
│   ├── prompts.json
│   ├── run.py
│   └── results/
├── evals/                      # Three-arm evaluation harness
│   ├── llm_run.py              # Generates snapshots via Claude CLI
│   ├── measure.py              # Offline token counting from snapshots
│   ├── plot.py                 # Visualization
│   ├── prompts/
│   └── snapshots/
├── tests/
│   ├── test_hooks.py
│   └── verify_repo.py
├── docs/
│   └── index.html              # Marketing landing page
├── caveman.skill               # Packaged skill bundle (zip/binary)
├── CLAUDE.md                   # Agent config redirect
├── AGENTS.md                   # Agent configuration
├── GEMINI.md                   # Gemini CLI context
└── README.md
```

The architecture is **skill-based**: the entire system is driven by SKILL.md files — structured prompts loaded into the LLM's context that define behavior rules.

---

## 3. Core Mechanism: The Caveman Skill

The heart of caveman is `skills/caveman/SKILL.md` — a ~65-line prompt that instructs the LLM how to communicate. It is not code — it is a behavioral constraint injected into the system prompt.

### Compression Rules

**Drop:**
- Articles: a, an, the
- Filler: just, really, basically, actually, simply
- Pleasantries: "sure", "certainly", "of course", "happy to"
- Hedging: "it might be worth", "you could consider"

**Keep:**
- All technical terms (exact)
- Code blocks (unchanged)
- Error messages (quoted exact)

**Pattern:** `[thing] [action] [reason]. [next step].`

### Intensity Levels

| Level | Description | Example |
|-------|-------------|---------|
| **Lite** | Drop filler, keep grammar. Professional but no fluff | "Your component re-renders because you create a new object reference each render." |
| **Full** (default) | Drop articles, fragments OK, short synonyms | "New object ref each render. Inline object prop = new ref = re-render." |
| **Ultra** | Abbreviate everything, arrows for causality, telegraphic | "Inline obj prop → new ref → re-render. `useMemo`." |
| **Wenyan-Lite** | Semi-classical Chinese, grammar intact | "組件頻重繪，以每繪新生對象參照故。以 useMemo 包之。" |
| **Wenyan-Full** | Full 文言文, maximum classical terseness | "物出新參照，致重繪。useMemo Wrap之。" |
| **Wenyan-Ultra** | Extreme classical Chinese compression | "新參照→重繪。useMemo Wrap。" |

### Auto-Clarity Escapes

The skill is smart enough to drop caveman mode for:
- Security warnings
- Irreversible action confirmations
- Multi-step sequences where fragments risk misreading
- Situations where the user asks for clarification

This is critical — it means the compression is context-aware, not a blind text filter.

---

## 4. Skills Breakdown

### 4.1 caveman-commit

Generates terse commit messages following Conventional Commits format.

**Rules:**
- Subject line: `<type>(<scope>): <imperative summary>` ≤50 chars
- Body only when "why" isn't obvious from the subject
- Never includes: "This commit does X", AI attribution, emoji (unless project convention)
- Imperative mood: "add", "fix", "remove" — not "added", "adds"

**Example:**
```
feat(api): add GET /users/:id/profile

Mobile client needs profile data without full user payload
to reduce LTE bandwidth on cold-launch screens.

Closes #128
```

### 4.2 caveman-review

One-line code review comments. Each finding is a single line with location, severity, problem, and fix.

**Format:** `L<line>: <severity> <problem>. <fix>.`

**Severity prefixes:**
- `🔴 bug:` — broken behavior, will cause incident
- `🟡 risk:` — works but fragile
- `🔵 nit:` — style, naming, micro-optimization
- `❓ q:` — genuine question

**Example:**
```
L42: 🔴 bug: user can be null after .find(). Add guard before .email.
L88-140: 🔵 nit: 50-line fn does 4 things. Extract validate/normalize/persist.
```

### 4.3 caveman-compress

Compresses natural language files (CLAUDE.md, memory files, notes) into caveman-speak to reduce **input** tokens. While the main caveman skill reduces output tokens, compress reduces what the LLM reads every session.

**Pipeline:**
1. Detect file type (prose vs code)
2. Call Claude to compress (removing filler, shortening prose)
3. Validate output (ensure code blocks unchanged, structure preserved)
4. Cherry-pick fix if errors (targeted, no full recompression)
5. Retry up to 2 times on failure
6. Backup original as `FILE.original.md`

**Results:** Average 46% input token reduction across tested files.

**Implementation:** Python CLI in `compress/scripts/`:
- `__main__.py` — entry point
- `cli.py` — argument parsing
- `compress.py` — Claude API compression logic
- `detect.py` — file type detection
- `validate.py` — output validation (code blocks preserved, structure intact)

**Critical preservation rules:**
- Code blocks (fenced and indented) — copied EXACTLY
- Inline code — never modified
- URLs, file paths, commands, technical terms
- Markdown structure (headings, bullets, tables, frontmatter)
- Dates, version numbers, numeric values

---

## 5. Hook System (Claude Code Integration)

Caveman auto-activates via Claude Code's hook system. Two hooks handle the lifecycle:

### 5.1 caveman-activate.js (SessionStart)

Runs on every session start:
1. **Writes flag file** at `~/.claude/.caveman-active` (statusline reads this)
2. **Emits caveman ruleset** as hidden SessionStart context — the rules from SKILL.md condensed to a single-line injection
3. **Detects missing statusline config** — if `~/.claude/settings.json` has no `statusLine`, emits a setup nudge

### 5.2 caveman-mode-tracker.js (UserPromptSubmit)

Listens on every user prompt:
- Parses `/caveman <level>` commands and writes the active mode to the flag file
- Detects deactivation phrases ("stop caveman", "normal mode") and removes the flag file
- Maps commands to modes: `/caveman-commit` → `commit`, `/caveman-review` → `review`, `/caveman lite` → `lite`, etc.

### 5.3 Statusline Integration

Shell scripts (`caveman-statusline.sh` / `.ps1`) read the flag file and output a badge:
- `[CAVEMAN]` — full mode
- `[CAVEMAN:ULTRA]` — ultra mode
- `[CAVEMAN:COMMIT]` — commit mode
- Nothing if no flag file

---

## 6. Benchmarking and Evaluation

### 6.1 Benchmark Harness (`benchmarks/`)

A Python script (`run.py`) that:
1. Loads 10 standard software engineering prompts from `prompts.json`
2. Runs each prompt through the Anthropic API in two modes: normal (system: "You are a helpful assistant") and caveman (system: SKILL.md content)
3. Runs multiple trials per prompt per mode
4. Computes median token counts, savings percentages
5. Outputs a markdown table and optionally updates README.md

**Benchmark categories:** debugging, bugfix, setup, explanation, refactor, architecture, code-review, devops, implementation.

**Results:** 22%–87% savings across prompts, 65% average.

### 6.2 Eval Harness (`evals/`)

A more rigorous three-arm evaluation:

| Arm | System Prompt | Purpose |
|-----|---------------|---------|
| `__baseline__` | None | Raw LLM verbosity |
| `__terse__` | "Answer concisely." | Control: how much does just asking for brevity help? |
| `terse+skill` | "Answer concisely." + SKILL.md | Treatment: what does the skill add on top of generic terseness? |

This is the honest measurement: comparing (3) vs (2) isolates the skill's contribution from the generic "be brief" effect. Previous benchmark versions compared skill vs verbose, which conflated the two effects.

**Tooling:**
- `llm_run.py` — calls Claude CLI (`claude -p`) for each prompt × arm, saves raw outputs to `snapshots/results.json`
- `measure.py` — reads snapshots, counts tokens with tiktoken (o200k_base, OpenAI approximation), reports median/mean/min/max/stdev
- `plot.py` — visualization

---

## 7. Multi-Agent Distribution

Caveman is designed for universal agent compatibility:

| Agent | Install Method | Auto-Activation |
|-------|---------------|-----------------|
| Claude Code | Plugin marketplace + hooks | SessionStart hook |
| Codex | Plugin install + `.codex/hooks.json` | Hook-based |
| Gemini CLI | Extension install + GEMINI.md context | Context file |
| Cursor | `npx skills add` | Manual (needs rules entry) |
| Windsurf | `npx skills add` | Manual |
| Cline | `npx skills add` | Manual |
| Copilot | `npx skills add` | Manual |
| Others (40+) | `npx skills add` | Manual |

The `caveman.skill` file at the repo root is a packaged zip bundle containing the skill for plugin distribution systems.

---

## 8. Design Principles

### 8.1 Prompt Engineering as a Product

Caveman demonstrates that prompt engineering can be packaged, distributed, and installed like software. The entire product is a ~65-line markdown file (SKILL.md) plus distribution infrastructure.

### 8.2 Output-Only Compression

Caveman only affects output tokens — thinking/reasoning tokens are untouched. This is important: it compresses the mouth, not the brain.

### 8.3 Context-Aware Safety

The Auto-Clarity escape hatch ensures that compressed communication doesn't compromise safety. Security warnings, destructive operation confirmations, and clarification requests revert to full verbosity.

### 8.4 Honest Evaluation

The three-arm eval design (baseline vs terse-control vs terse+skill) avoids the common trap of comparing compressed output against unnecessarily verbose baselines. It measures what the skill actually contributes.

### 8.5 Code Is Sacred

Code blocks, commit messages, and PR descriptions bypass the compression filter entirely. The skill recognizes that code compression is a different problem (minification, not communication).

---

## 9. Relevance to Our Platform

### Token Cost Reduction

For any service that wraps or proxies LLM calls (foundation_ai), caveman-style system prompt injection could reduce output token costs for non-code responses.

### Prompt Compression for Context

The caveman-compress approach (compressing memory/context files) is directly applicable to our agent system. If agent configuration files are loaded every session, compressing them saves input tokens at scale.

### Skill Distribution Model

The `skills/` + `hooks/` + `commands/` architecture is a clean pattern for packaging AI agent behaviors. Each skill is self-contained in a single SKILL.md with frontmatter metadata. This is worth studying as a model for our own agent skill system.

### Evaluation Methodology

The three-arm eval design is a template for evaluating any prompt engineering technique: always include a terse control arm to isolate the specific technique's contribution from generic compression effects.

### Hook-Based Lifecycle

The SessionStart + UserPromptSubmit hook pattern for automatic mode activation and tracking is a robust approach to agent state management without modifying the agent's core code.

---

## 10. Technical Notes

- **No runtime dependencies**: The core skill is pure prompt engineering — zero code at runtime
- **Tokenizer caveat**: Benchmarks use tiktoken o200k_base (OpenAI), which is an approximation of Claude's BPE. Ratios are still meaningful for comparison, absolute numbers are approximate
- **CI sync**: `caveman/SKILL.md`, `plugins/caveman/skills/caveman/SKILL.md`, `.cursor/skills/caveman/SKILL.md`, and `caveman.skill` are all auto-synced from `skills/caveman/SKILL.md` by CI after merge. Only edit the source.
- **The landing page** (`docs/index.html`) is a single-file site with a premium dark aesthetic, cursor-tracking glow, animated diff visualization, and a "bonk counter" easter egg
