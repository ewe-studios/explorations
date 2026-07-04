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
| 1 | src.UIComponents | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/` | NEEDS REVIEW — 91 docs across 11 components, mermaid injection needed |
| 2 | src.TenFramework | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.TenFramework/` | **DONE** — 4.7M LOC framework documented (TEN + VAD + turn detection + portal), HTML built |
| 3 | src.Moltis | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/` | NEEDS REVIEW — 13 docs, content exists, needs mermaids + depth |
| 4 | src.PDFToMarkdown | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.PDFToMarkdown/` | **DONE** — 8 deep docs (10,317 words), HTML built |
| 5 | src.Alibaba | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Alibaba/` | **DONE** — 7 deep docs, HTML built |
| 6 | bolt.diy | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/bolt.diy/` | **DONE** — 6 deep docs, HTML built |
| 7 | src.Google | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Google/` | **DONE** — 12 projects documented, HTML built |
| 8 | src.Hedr | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/` | **DONE** — Terminal workspace manager (20+ agent detectors, PTY management), HTML built |
| 9 | src.Instatic | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Instatic/` | NEEDS REVIEW — 15 docs, content exists |
| 10 | ponytail | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.PromptOptimizations/ponytail/` | NEEDS REVIEW — 4 docs |
| 11 | src.streamlit | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.streamlit/` | **DONE** — Python↔JS protobuf bridge, rerun model, DeltaGenerator, SessionState, HTML built |
| 12 | src.strukto-ai | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.strukto-ai/` | **DONE** — Mirage VFS (50+ backends, in-process shell, 4-layer architecture), HTML built |
| 13 | superdesign | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.superdesign/superdesign/` | NEEDS REVIEW — 6 docs |
| 14 | src.opendesign | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Opendesign/` | **DONE** — 19 projects documented, HTML built |
| 15 | src.designskills | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.DesignSkills/` | **DONE** — taste-skill, lumenshaders, gemini-extension documented, HTML built |
| 16 | goose | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.block/goose/` | **DONE** — 14 deep docs (15,944 words) covering all 6 crates, HTML built |
| 17 | src.jeremychone | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.jeremychone` | **DONE** — 5 deep docs (aiprog, aipack, udiffx, genai + overview), HTML built |
| 18 | wayfinder | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Wayfinder` | **DONE** — RAC ecosystem documented (7 projects), HTML built |

## Notes

- Some projects may already have documentation — check before starting
- Each project must pass grandfather review before marking DONE
- Follow the AGENTS.md directory structure convention
- Use the shared build.py pattern from existing projects


Ensure to read /home/darkvoid/Boxxed/@dev/repo-expolorations/markdown_engineering/documentation_directive.md
