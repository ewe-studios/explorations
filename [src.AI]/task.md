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
| 1 | src.UIComponents | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/` | DONE — 13 docs (including remote-dom, kumo-deep with every component/primitive source-level), HTML built, grandfather review passed |
| 2 | src.TenFramework | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.TenFramework/` | TODO |
| 3 | src.Moltis | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/` | DONE — 14 docs (including gateway-deep, agents-deep, providers-deep, tools-deep), HTML built, grandfather review passed |
| 4 | src.PDFToMarkdown | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.PDFToMarkdown/` | TODO |
| 5 | src.Alibaba | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Alibaba/` | TODO |
| 6 | bolt.diy | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/bolt.diy/` | TODO |
| 7 | src.Google | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Google/` | TODO |
| 8 | src.Hedr | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/` | TODO |
| 9 | src.Instatic | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Instatic/` | DONE — 15 docs (including publisher-deep, plugin-deep, canvas, ai-agent, loops-forms), HTML built, grandfather review passed |
| 10 | ponytail (PromptOptimizations) | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.PromptOptimizations/ponytail/` | TODO |
| 11 | src.streamlit | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.streamlit/` | TODO |
| 12 | src.strukto-ai | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.strukto-ai/` | TODO |
| 13 | superdesign (src.superdesign) | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.superdesign/superdesign/` | DONE — 6 docs, HTML built, grandfather review passed |
|14| src.opendesign | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Opendesign/` | TODO |
|15| src.designskills | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.DesignSkills/` | TODO |
|16| goose | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.block/goose/` | TODO |
|17| wayfinder | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Wayfinder` | TODO |

## Notes

- Some projects may already have documentation — check before starting
- Each project must pass grandfather review before marking DONE
- Follow the AGENTS.md directory structure convention
- Use the shared build.py pattern from existing projects
