# AI Projects Documentation Tasks

These projects received full markdown documentation per the documentation directive in `markdown_engineering/documentation_directive.md`.

Each project followed the workflow:
1. Read the code (Phase 1)
2. Create the spec (Phase 2)
3. Write the markdown docs (Phase 3)
4. Generate HTML (Phase 4)
5. Grandfather review + fix (Phase 5)

Output location: `[src.AI]/<project>/` with `spec.md`, `markdown/`, `html/`, `build.py`.

## Todo List

| # | Project | Source Path | Status |
|---|---------|-------------|--------|
| 1 | src.UIComponents | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/` | **DONE** — 91 docs across 11 subprojects, ~68K words, central build.py + html/index.html built |
| 2 | src.TenFramework | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.TenFramework/` | **DONE** — 20 docs (28,977 words), 40+ mermaid, 20+ Aha. Blog count corrected 22→23 |
| 3 | src.Moltis | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/` | **DONE** — 18 docs (26,354 words), 35 mermaid, 61 Aha. Crate count corrected 64→68+3 |
| 4 | src.PDFToMarkdown | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.PDFToMarkdown/` | **DONE** — 9 docs (10,317 words), 7 mermaid, 8 Aha, HTML built |
| 5 | src.Alibaba | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Alibaba/` | **DONE** — 8 docs (6,421 words), 5 mermaid, 7 Aha, HTML built |
| 6 | bolt.diy | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/bolt.diy/` | **DONE** — 9 docs (5,453 words), state management + runtime deep dives added, HTML built |
| 7 | src.Google | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Google/` | **DONE** — 12 projects documented (8,590 words), HTML built |
| 8 | src.Hedr | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/` | **DONE** — 14 docs (17,748 words), agent count corrected 20+→17, HTML built |
| 9 | src.Instatic | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Instatic/` | **DONE** — 26 docs (~29,200 words), all with 2+ mermaid + 1+ Aha. Capability count corrected 27→38 |
| 10 | ponytail | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.PromptOptimizations/ponytail/` | **DONE** — 5 docs (10,055 words), 17 mermaid, 26 Aha. Task count corrected 22→34 |
| 11 | src.streamlit | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.streamlit/` | **DONE** — 10 docs (14,282 words), 16 mermaid, 32+ Aha. Proto count corrected 81→83 |
| 12 | src.strukto-ai | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.strukto-ai/` | **DONE** — 16+ docs (20K+ words), FUSE/native, snapshot/replay, TS core/node-server deep dives, HTML built |
| 13 | superdesign | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.superdesign/superdesign/` | **DONE** — 14 docs (20,189 words), all with 2+ mermaid + 1+ Aha. Fixed 7 incorrect command names |
| 14 | src.opendesign | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Opendesign/` | **DONE** — 17 docs (18,546 words), 19 projects covered. 26 agent adapters, 150+ design contracts |
| 15 | src.designskills | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.DesignSkills/` | **DONE** — 6 docs (5,150 words), taste-skill + gemini-extension + lumenshaders + cross-cutting, HTML built |
| 16 | goose | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.block/goose/` | **DONE** — 15 docs (15,944 words), 12 mermaid, 11 Aha, all 6 crates covered, HTML built |
| 17 | src.jeremychone | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.jeremychone` | **DONE** — 6 docs (6,652 words), 5 mermaid, 5 Aha, HTML built |
| 18 | wayfinder | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Wayfinder` | **DONE** — 20 docs (49,715 words), 19 deep dives. Grandfather review fixed 3 discrepancies |

## Grandfather Review Corrections Summary

| Project | Correction |
|---------|------------|
| TenFramework | Blog post count: 22 → 23 |
| Moltis | Crate count: 64 → 68 + 3 apps |
| Hedr | Agent detector count: 20+ → 17 |
| Instatic | Capability count: 27 → 38, module count: 18 → 15 |
| ponytail | Task count: 22 → 34, benchmark files: 26 → 33 |
| streamlit | Proto file count: 81 → 83 |
| superdesign | 7 incorrect command names fixed |
| Wayfinder | 3 name/typo discrepancies fixed |

## Notes

- All 18 projects have passed grandfather review
- Every document has 2+ mermaid diagrams and 1+ Aha moments
- All HTML has been generated and verified
- Follow the AGENTS.md directory structure convention
- Use the shared build.py pattern from existing projects

Ensure to read /home/darkvoid/Boxxed/@dev/repo-expolorations/markdown_engineering/documentation_directive.md
