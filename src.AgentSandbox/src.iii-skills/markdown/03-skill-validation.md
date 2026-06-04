---
title: Skill Validation — Three-Layer Quality Checks
---

# Skill Validation — Three-Layer Quality Checks

**Every skill is validated by the skills-and-validation system through three layers: structure, prose linting, and AI content review.**

## Validation Pipeline

```mermaid
flowchart TB
    subgraph L1["Layer 1: Structure (deterministic)"]
        S1["Required sections present"]
        S2["Frontmatter valid"]
        S3["Links resolve"]
    end

    subgraph L2["Layer 2: Vale (prose linting)"]
        V1["Diataxis rules"]
        V2["Terminology rules"]
        V3["Voice rules"]
    end

    subgraph L3["Layer 3: AI (Anthropic)"]
        A1["Content quality review"]
        A2["Diataxis pattern check"]
        A3["Voice consistency"]
    end

    L1 --> L2
    L2 --> L3
    L3 --> PASS
```

## Layer 1: Structure Checks

| Check | What it validates |
|-------|------------------|
| Required sections | `#` heading present, non-empty body |
| Frontmatter | `name:` and `description:` fields present and valid |
| Links | Internal links reference valid skills |

## Layer 2: Vale Prose Linting

Rules from the skills-and-validation `content/styles/` directory:

| Style | Rules | Purpose |
|-------|-------|---------|
| Diataxis | 12 rules | Correct doc type (how-to, reference, tutorial) |
| Terminology | 10 rules | Consistent terminology |

### Key Terminology Rules

| Rule | Enforces | Rejects |
|------|----------|---------|
| `SlopMarketing` | Technical terms | "revolutionary", "cutting-edge" |
| `SlopMagic` | Concrete descriptions | "magic", "seamlessly" |
| `BackendSoftware` | "backend" | "back-end" |
| `ForbiddenTerms` | Approved terms | Banned terminology |

## Layer 3: AI Content Review

Uses Anthropic's Claude to review:
- Content quality and completeness
- Diataxis pattern compliance
- Voice consistency with other skills
- Technical accuracy

## AI Pass Caching

Source: `skills-and-validation/crates/iii-skill-core/src/ai_cache.rs`

```mermaid
flowchart TD
    A[AI review request] --> B{Cache hit?}
    B -->|Yes + PASS| C[Return cached PASS]
    B -->|Yes + FAIL| D[Re-run AI review]
    B -->|No| E[Call Anthropic API]
    E --> F{Result?}
    F -->|PASS| G[Cache + return]
    F -->|FAIL| D
    D --> H[Return fresh result]
    C --> I[Return cached result]
```

**Aha:** The same skills-and-validation system that validates worker documentation also validates skills — ensuring consistency across all iii documentation.

## Validation Configuration

Source: `skills-and-validation/content/skills/iii-skill-authoring/`

The skill authoring guide provides:
- Quickstart for new skill creation
- Document structure guidelines
- Template for new skills
- Voice and tone guidelines
- LLM-only and human-only block usage
- Validation checklist

## What's Next

- [00 — Overview](00-overview.md) — Return to overview
- [01 — Skill Catalog](01-skill-catalog.md) — Return to catalog
- [02 — Skill Format](02-skill-format.md) — Return to format
