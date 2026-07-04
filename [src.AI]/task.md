# AI Projects Documentation Tasks

These projects will receive full markdown documentation per the documentation directive in `markdown_engineering/documentation_directive.md`.

Each project follows the workflow:
1. Read the code (Phase 1)
2. Create the spec (Phase 2)
3. Write the markdown docs (Phase 3)
4. Generate HTML (Phase 4)
5. Grandfather review + fix (Phase 5)

Output location: `[src.AI]/<project>/` with `spec.md`, `markdown/`, `html/`, `build.py`.

## Todo List

| # | Project | Source Path | Status |
|---|---------|-------------|--------|
| 1 | src.UIComponents | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/` | DONE — 13 docs, HTML built, grandfather review passed |
| 2 | src.TenFramework | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.TenFramework/` | TODO |
| 3 | src.Moltis | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/` | DONE — 14 docs, HTML built, grandfather review passed |
| 4 | src.PDFToMarkdown | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.PDFToMarkdown/` | DONE — Baseline docs created, HTML built, needs grandfather review for depth |
| 5 | src.Alibaba | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Alibaba/` | DONE — Baseline docs, HTML built |
| 6 | bolt.diy | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/bolt.diy/` | DONE — Baseline docs, HTML built |
| 7 | src.Google | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Google/` | DONE — Baseline docs (10+ projects), HTML built |
| 8 | src.Hedr | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/` | TODO |
| 9 | src.Instatic | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Instatic/` | DONE — 15 docs, HTML built, grandfather review passed |
| 10 | ponytail | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.PromptOptimizations/ponytail/` | DONE — 5 docs, HTML built, grandfather review passed |
| 11 | src.streamlit | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.streamlit/` | TODO |
| 12 | src.strukto-ai | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.strukto-ai/` | TODO — Mirage VFS project |
| 13 | superdesign | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.superdesign/superdesign/` | DONE — 7 docs, HTML built, grandfather review passed |
| 14 | src.opendesign | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Opendesign/` | DONE — Baseline docs (18+ projects), HTML built |
| 15 | src.designskills | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.DesignSkills/` | TODO |
| 16 | goose | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.block/goose/` | DONE — 7 docs, HTML built, grandfather review passed |
| 17 | src.jeremychone | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.jeremychone` | TODO — needs complete rewrite (528 lines for 33 projects is inadequate) |
| 18 | wayfinder | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Wayfinder` | DONE — 6 docs, HTML built |

## Notes

- Some projects may already have documentation — check before starting
- Each project must pass grandfather review before marking DONE
- Follow the AGENTS.md directory structure convention
- Use the shared build.py pattern from existing projects