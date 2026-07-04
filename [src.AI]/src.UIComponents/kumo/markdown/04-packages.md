# Kumo: Package Map

Kumo is a design system monorepo with multiple sub-packages.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/kumo/`

## Sub-Packages

| Package | Purpose |
|---------|---------|
| `kumo` | Main component library |
| `kumo-docs-astro` | Documentation site (Astro) |
| `kumo-figma` | Figma integration (color/token sync) |
| `kumo-screenshot-worker` | Screenshot generation worker |

## CI System

| Directory | Purpose |
|-----------|---------|
| `ci/reporters/` | CI reporting (npm release, visual regression, docs preview) |
| `ci/scripts/` | CI scripts (release PRs, changeset validation, doc reports) |
| `ci/utils/` | GitHub API, git operations, PR reporter |
| `ci/visual-regression/` | Visual regression test configuration |

## Lint System

| Rule | Purpose |
|------|---------|
| `enforce-variant-standard.js` | Enforces consistent Tailwind variant ordering |
| `no-cross-package-imports.js` | Prevents cross-package import violations |
| `no-primitive-colors.js` | Prevents use of raw/primitive color values |
| `no-tailwind-dark-variant.js` | Enforces dark mode via theme tokens, not dark: classes |
| `lint-astro-colors.js` | Lints color usage in Astro files |
| `kumo-plugin.js` | Main ESLint plugin bundling all rules |

## Cross-References

- [Overview](00-overview.md) — Architecture summary
- [Theming](01-theming.md) — Theme system and tokens
- [AI Registry](02-ai-registry.md) — AI-assisted generation
- [Tooling](03-tooling.md) — CI and build tooling
