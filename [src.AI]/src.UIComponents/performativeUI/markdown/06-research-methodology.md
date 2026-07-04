# Research Methodology

Components are sourced from real companies. The `research/` directory documents the source companies and implementation details.

## Research Files

| File | Content |
|------|---------|
| `00_source_companies.md` | List of companies using these patterns |
| `01_typewriter_hero.md` | Typewriter hero implementations from real sites |
| `02_logo_walls.md` | Logo wall patterns from real sites |
| `03_node_graph_backgrounds.md` | Node graph backgrounds from real sites |
| `04_ascii_hero_art.md` | ASCII art heroes from real sites |
| `05_aiified_ui_elements.md` | AI-themed UI elements from real sites |
| `06_cursor_effects.md` | Cursor effect patterns from real sites |

Aha: Each component is reverse-engineered from a real company's landing page, not invented from scratch. This means the components are battle-tested patterns, not theoretical designs. The research files document which companies use which patterns and how they implement them.

## Docs Site Architecture

The docs site (`docs/`) has:

| File | Purpose |
|------|---------|
| `App.tsx` (198 lines) | Router with component pages |
| `main.tsx` (14 lines) | Entry point |
| `pages/Home.tsx` (119 lines) | Landing page with component grid |
| `lib/meta.tsx` (2281 lines) | Component metadata (props, descriptions, examples) |
| `lib/ComponentPage.tsx` (121 lines) | Component preview page |
| `lib/PropsTable.tsx` (36 lines) | Props documentation table |
| `lib/CodeBlock.tsx` (22 lines) | Code display |
| `lib/Attribution.tsx` (33 lines) | Source attribution |

Aha: `meta.tsx` is 2,281 lines — the largest file in the project. It contains metadata for all 32 components: props tables, descriptions, usage examples, and attribution information. This is essentially the component catalog's database.

## Cross-References

- [Overview](00-overview.md) — All components
- [Build System](05-build-system.md) — Vite configs
