---
source_location: /Users/darkvoid/GitSources/src.cursor/plugins/pstack (commit 4dd1d05818d19792e7cd6e818f1667f95f7e1dbe, subdirectory of cursor/plugins)
repository: git@github.com:cursor/plugins.git
explored_at: 2026-10-30
documentation_goal: Explain pstack's philosophy, its 24 principles, the poteto-mode entry skill and its playbooks, every skill, the two subagents, and the statechart formal-verification tool.
---

# Spec: pstack Documentation

## 1. Source

| Property | Value |
|----------|-------|
| Location | /Users/darkvoid/GitSources/src.cursor/plugins/pstack, commit `4dd1d05` (the `pstack` folder of the `cursor/plugins` monorepo) |
| Language | Markdown skills and TypeScript tooling (the `statechart` and `poteto-mode` skills ship TypeScript scripts) |
| Version | 0.15.36 |
| License | MIT |
| Author | Lauren Tan (poteto) |
| Main files studied | `skills/poteto-mode/SKILL.md`, the 24 `skills/principle-*` skills, `skills/statechart/scripts/*`, `agents/*.md`, `docs/guide/*.md` |

## 2. What pstack Is

pstack is a Cursor and Claude Code plugin by Lauren Tan (poteto). It packages about 60 skills, two subagents, 24 principles, and a statechart/TLA+ formal-verification tool. The core idea is "if you want to go fast, go deep first": write less but higher-quality code, and get "fearless parallelism". The entry point is the `poteto-mode` skill, which matches a task to one of 23 playbooks and routes to the other skills as steps fire.

## 3. Documentation Goal

1. Know the philosophy: go deep first, write less but better, verify against the real artifact.
2. Know all 24 principles, grouped, with the exact rule each states.
3. Know `poteto-mode`: the 23 playbooks, the todo-list mechanism, model routing, and "unslopped replies".
4. Know every skill in the catalog and when to use it.
5. Know the two subagents (`poteto-agent`, `comment-sicko`) and how they wrap the mode skill.
6. Know the statechart tool: the pipeline stages, the intermediate representation, each pass, and each backend.
7. Know the guide's first-task walkthrough, the benny automation pack, and model routing/setup.
8. Know the strengths, weaknesses and risks, ranked.

## 4. Documentation Structure

```
agentic-verification/pstack/
├── exploration.md
├── spec.md
├── build.py
├── styles.css
├── markdown/   (README.md, 00 to 14)
└── html/
```

## 5. Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | markdown/README.md | DONE |
| 1 | 00-overview.md | DONE |
| 2 | 01-philosophy-and-principles.md | DONE |
| 3 | 02-poteto-mode-and-playbooks.md | DONE |
| 4 | 03-skill-catalog.md | DONE |
| 5 | 04-understanding-and-evidence.md | DONE |
| 6 | 05-verification-skills.md | DONE |
| 7 | 06-the-statechart-tool.md | DONE |
| 8 | 07-parallelism-delegation-subagents.md | DONE |
| 9 | 08-writing-and-correctness.md | DONE |
| 10 | 09-personalization-and-setup.md | DONE |
| 11 | 10-the-guide.md | DONE |
| 12 | 11-benny-automation.md | DONE |
| 13 | 12-model-routing.md | DONE |
| 14 | 13-critical-assessment.md | DONE |
| 15 | 14-glossary.md | DONE |
| 16 | exploration.md | DONE |
| 17 | Grandfather review | DONE |
| 18 | Generate HyperText Markup Language (HTML) | DONE |

## 6. Build System

Run `python3 -I build.py .` inside this directory. It writes `html/`. Run `python3 -I build.py` in the parent `agentic-verification` to build the collection index that links both projects. No dependencies beyond the Python standard library. This copy of `build.py` escapes raw angle brackets and processes inline markup inside table cells.

## 7. Quality Requirements (Iron Rules)

1. Detailed sections with code snippets.
2. Teach key facts, principles and ideas quickly.
3. Clear articulation.
4. Mermaid diagrams (minimum 2 per document).
5. Good visual assets.
6. Generated HyperText Markup Language (HTML).
7. Cross-references.
8. Source path references.
9. Aha moments.
10. Navigation: index plus prev/next buttons.

Also: ASD-STE100 Simplified Technical English. Abbreviations are spelled out where used. The code wins over the docs.

## 8. Expected Outcome

A reader can explain pstack's philosophy, pick the right skill or playbook for a task, and trace how the statechart tool verifies a TLA+ model.

## 9. Resume Point

All documents are written. To continue: re-run the citation check on any changed chapter, then rebuild the HyperText Markup Language (HTML).
