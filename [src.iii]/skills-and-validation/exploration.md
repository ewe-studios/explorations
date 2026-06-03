---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/skills-and-validation
repository: git@github.com:iii-hq/skills-and-validation
explored_at: 2026-06-03T00:00:00Z
language: Rust
---

# Project Exploration: Skills & Validation — Doc Render & Validate

## Overview

Skills & Validation is a **Rust workspace that renders and validates iii worker documentation** against project voice, structure, and Diataxis rules. Authors write short markdown partials under `<worker>/docs/`; the tool renders them into `README.md` and `skill.md`, then verifies prose quality with Vale and semantic correctness with AI (Anthropic).

```
┌─────────────────────────────────────────────────────┐
│              Author writes partials                  │
│  <worker>/docs/00-overview.md                       │
│  <worker>/docs/01-api.md                            │
│  <worker>/docs/02-examples.md                       │
├─────────────────────────────────────────────────────┤
│              iii-skill-render                        │
│  Renders partials → README.md + skill.md            │
├─────────────────────────────────────────────────────┤
│              iii-skill-check                         │
│  ┌────────────────┐   ┌──────────────────────────┐  │
│  │  Vale (prose)  │   │  AI/Anthropic (semantic) │  │
│  │  → style rules │   │  → factuality, voice     │  │
│  └────────────────┘   └──────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

## Repository

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/skills-and-validation`
- **Remote:** `git@github.com:iii-hq/skills-and-validation`
- **Primary Language:** Rust
- **License:** Apache-2.0 (inferred)

## Directory Structure

```
skills-and-validation/
├── Cargo.toml                      # Workspace definition
├── .skill-check.yaml               # Config file
├── action.yml                      # 30KB — composite GitHub Action
├── content/                        # Project rules and styles
│   ├── Vale styles/                # Vale linting rules
│   ├── skill bundles/              # Reusable skill bundles
│   └── .vale.ini                   # Vale configuration
├── templates/                      # Templates
│   ├── .skill-check.yaml.template  # Config template
│   └── example worker/             # Example worker docs
├── fixtures/                       # Test fixtures
│   └── (intentionally broken workers for testing)
├── scripts/                        # Shared scripts
│   └── (used by GitHub Action and pre-commit hook)
└── crates/
    ├── iii-skill-core/             # ── Shared library ──
    │   ├── Cargo.toml
    │   └── src/                    # render, structure, vale, ai, config, bundle
    ├── iii-skill-render/           # ── Render-only binary ──
    │   ├── Cargo.toml
    │   └── src/
    └── iii-skill-check/            # ── Verify + render binary ──
        ├── Cargo.toml
        └── src/
```

## Crate Breakdown

### 1. iii-skill-core (library)

**Location:** `crates/iii-skill-core/`

Shared library used by both binaries. Contains modules for:

| Module | Purpose |
|--------|---------|
| `render` | Markdown rendering logic |
| `structure` | Document structure analysis |
| `vale` | Vale integration for prose linting |
| `ai` | Anthropic AI integration for semantic checks |
| `config` | `.skill-check.yaml` parsing |
| `bundle` | Skill bundle management |

### 2. iii-skill-render (binary)

**Location:** `crates/iii-skill-render/`

Render-only binary. Takes worker markdown partials and renders them into final documents (README.md, skill.md). No network dependencies — pure local rendering.

### 3. iii-skill-check (binary)

**Location:** `crates/iii-skill-check/`

Verification binary that runs both Vale (prose) and AI (semantic) checks. Requires network access for AI validation.

## GitHub Action

**Location:** `action.yml` (30KB)

Composite GitHub Action that runs the skill check pipeline.

### Inputs

| Input | Description |
|-------|-------------|
| `version` | Tool version |
| `config-path` | Path to `.skill-check.yaml` |
| `workers-glob` | Glob pattern for worker directories |
| `docs-glob` | Glob pattern for documentation files |
| `layers` | Validation layers to run |
| `vale-version` | Vale version to use |
| `anthropic-api-key` | Anthropic API key for AI validation |
| `write` | Whether to write rendered output |
| `scope` | Validation scope |

### Usage Modes

| Mode | Description |
|------|-------------|
| **worker mode** | Validates worker directories (`<worker>/docs/`) |
| **docs mode** | Validates standalone `.md`/`.mdx` files (Mintlify/Fumadocs) |

## Vale Integration

**Location:** `content/Vale styles/`, `content/.vale.ini`

Vale is a prose linter. The project includes custom Vale style rules for:
- Project voice consistency
- Diataxis structure compliance
- Grammar and style rules

## AI Validation

**Location:** `crates/iii-skill-core/src/ai/`

Uses Anthropic API to validate documentation semantically:
- Factual accuracy against source code
- Voice consistency
- Structural completeness

## Configuration

**Location:** `.skill-check.yaml`

Configures validation rules, layers, and scope:

```yaml
# .skill-check.yaml
workers: "workers/*"
docs: "docs/**/*.md"
layers:
  - vale
  - ai
  - structure
```

## Key Insights

1. **Two-pass validation catches different bugs.** Vale catches style issues (tone, grammar, forbidden words); AI catches semantic issues (wrong API descriptions, missing features, outdated numbers). Together they cover both the "how it's written" and "what it says" dimensions.

2. **Partials → rendered documents is a content pipeline.** Authors write small, focused markdown files under `<worker>/docs/`; the tool assembles them into coherent README.md and skill.md files. This is similar to how docs-as-code systems (MkDocs, Docusaurus) assemble pages from source files.

3. **Network-free rendering is a feature.** `iii-skill-render` has no network dependencies — it can render documents offline. Only `iii-skill-check` (the validation step) requires network access for AI validation.

4. **Supports both worker and docs modes.** The tool validates both iii worker documentation (`<worker>/docs/`) and standalone documentation sites (Mintlify/Fumadocs), making it reusable beyond the iii ecosystem.

5. **Fixtures are intentionally broken.** The `fixtures/` directory contains workers with known documentation issues — these serve as test cases to ensure the validation catches real problems.

## Open Questions

1. **Diataxis rules.** What specific Diataxis structure rules are enforced? How does the tool detect whether a document is a tutorial, how-to, reference, or explanation?

2. **AI prompt design.** What prompts are sent to Anthropic for semantic validation? How does the tool prevent false positives from the AI validator?

3. **Bundle format.** What is a "skill bundle" and how are they structured?

4. **Pre-commit hook integration.** How does the pre-commit hook invoke the tool, and what checks does it run vs. the full GitHub Action?

## Related Explorations

- [Workers](../workers/exploration.md) — iii worker modules (the primary consumers of this tool)
- [iii Engine](../iii/exploration.md) — The iii engine
- [Spec Forge](../spec-forge/exploration.md) — UI spec generation worker

## Next Steps

1. Create `rust-revision.md` for idiomatic Rust patterns
2. Deep-dive into the AI validation prompts
3. Document the Diataxis rule set
4. Explore the skill bundle format
