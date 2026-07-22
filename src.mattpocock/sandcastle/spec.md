---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.mattpocock/sandcastle
language: TypeScript
version: 0.12.0
author: Matt Pocock (mattpocock)
license: MIT
repository: https://github.com/mattpocock/sandcastle
explored_at: 2026-07-22
---

# Sandcastle — Documentation Spec

## What the project is

**@ai-hero/sandcastle** is a TypeScript library for orchestrating AI coding agents in isolated sandbox environments. It provides a single `run()` API that sandboxes an agent (Claude Code, Codex, Pi, Cursor, OpenCode, Copilot) inside a configurable container or cloud VM, manages git worktrees and branch strategies, captures agent sessions for resume/fork, and merges commits back. Provider-agnostic: ships with Docker, Podman, Vercel, and no-sandbox providers, plus factory functions for building custom bind-mount or isolated providers.

## Documentation goal

A reader should understand:
1. What Sandcastle is and why it exists (agent orchestration, not agent implementation)
2. The sandbox provider architecture (bind-mount vs isolated, handle contract)
3. The three branch strategies (head, merge-to-head, branch) and when to use each
4. How `run()`, `createSandbox()`, `createWorktree()`, and `interactive()` differ
5. The prompt system (resolution, `{{KEY}}` substitution, `` !`command` `` expansion, built-in args)
6. The agent provider system (claudeCode, codex, pi, cursor, opencode, copilot factories)
7. The orchestration flow (sandbox lifecycle, iteration loop, completion signals)
8. Session capture, resume, and fork mechanics
9. Structured output extraction with retry
10. The template system and `sandcastle init` workflow
11. The CLI commands and flags
12. Error handling, abort signals, and shutdown lifecycle
13. How to build custom sandbox providers
14. Testing strategy and CI/CD pipeline

## Documentation structure

```
src.mattpocock/sandcastle/
  sandcastle/
    exploration.md
    spec.md
    markdown/
      README.md
      00-overview.md
      01-architecture.md
      02-sandbox-providers.md
      03-agent-integration.md
      04-orchestration-workflow.md
      05-prompt-system.md
      06-cli-interface.md
      07-utilities.md
      08-cross-cutting.md
    html/
      index.html
      *.html
```

## Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | exploration.md | TODO |
| 1 | spec.md | TODO |
| 2 | README.md (index) | TODO |
| 3 | 00-overview.md | TODO |
| 4 | 01-architecture.md | TODO |
| 5 | 02-sandbox-providers.md | TODO |
| 6 | 03-agent-integration.md | TODO |
| 7 | 04-orchestration-workflow.md | TODO |
| 8 | 05-prompt-system.md | TODO |
| 9 | 06-cli-interface.md | TODO |
| 10 | 07-utilities.md | TODO |
| 11 | 08-cross-cutting.md | TODO |
| 12 | HTML generation + verification | TODO |

## Build system

```bash
cd src.mattpocock/sandcastle && python3 build.py sandcastle
```

Python 3.12+ stdlib only. Idempotent.

## Quality requirements

All 10 Iron Rules from the Documentation Directive apply:
1. Detailed sections with actual source code snippets
2. Teach key facts quickly — first sentence = thesis
3. Clear articulation — one idea per sentence
4. Minimum 2 mermaid diagrams per document
5. Good visual assets (tables, mermaid, code blocks)
6. Generated HTML builds correctly
7. Cross-references between documents
8. Source path references (file:line)
9. At least 1 Aha moment per document
10. Navigation: index + prev/next buttons

Plus grandfather review: names match source, numbers match source, flows match source, nothing missing.

## Expected outcome

A reader can use Sandcastle to orchestrate AI agents, understand every option and type, build custom providers, and debug issues — all without reading the source code.

## Resume point

If interrupted, continue from the task list above. Each document builds on the previous. After all markdown, run `python3 build.py sandcastle` and verify HTML.
