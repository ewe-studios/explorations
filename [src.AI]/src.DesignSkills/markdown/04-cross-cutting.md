---
title: DesignSkills Cross-Cutting — Anti-Slop Principles, Design Tokens, and Quality Gates
---

# DesignSkills Cross-Cutting — Anti-Slop Principles, Design Tokens, and Quality Gates

**All three DesignSkills projects share a common philosophy: reject generic AI output in favor of deliberate, opinionated design choices.** Whether it's taste-skill's SKILL.md rules, gemini-extension's per-zone customization, or LumenShaders' genome system, the unifying thread is "specificity over generality."

## The Anti-Slop Design Token System

Across all three projects, design decisions are encoded as specific values rather than defaults:

| Token | taste-skill | gemini-extension | LumenShaders |
|-------|-------------|------------------|--------------|
| Color | HSL scale with named palettes | Custom background URLs + per-zone darkness | 4-color palette (`u_c1`-`u_c4`) |
| Spacing | 4/8/16/24/32/48/64px scale | Glass blur at 24px default | `u_scale` controls pattern size |
| Typography | Specific font size ratios | System font stack | N/A (visual art) |
| Contrast | Accessible minimum ratios | Darkness 0.0-0.8 per zone | `u_exposure`, `u_contrast` |

Aha: The genome system in LumenShaders is the ultimate expression of this principle — a single number encodes an entire visual configuration. Share the genome, reproduce the exact visual. This is the design token concept taken to its logical extreme: one number instead of 35 parameters.

## Quality Gates

Each project has a quality enforcement mechanism:

- **taste-skill**: The `output-skill` SKILL.md reviews generated output against taste principles
- **gemini-extension**: The `content.js` guard (`if (!chrome.runtime?.id) return;`) prevents broken UI injections
- **LumenShaders**: The two-stage shader boot ensures pixels appear within 1 second, even if the full shader takes longer to compile

## Shared Patterns

| Pattern | Where | Why |
|---------|-------|-----|
| Progressive enhancement | LumenShaders (base → full shader), gemini-extension (CSS before JS) | Show something useful immediately |
| Configurable defaults | All three | Sensible defaults that users can override |
| Parameter encoding | taste-skill (SKILL.md files), LumenShaders (genome) | Compress complex configurations into shareable units |
| Per-zone control | gemini-extension (4 darkness zones) | Granular customization without complexity |

## Cross-References

- [taste-skill](01-taste-skill.md) — Anti-slop agent skills
- [Gemini UI Redesign](02-gemini-extension.md) — Browser extension
- [LumenShaders](03-lumenshaders.md) — Shader art studio

Next: Review the [task.md status](../../../task.md) to see what needs work across all AI projects.
