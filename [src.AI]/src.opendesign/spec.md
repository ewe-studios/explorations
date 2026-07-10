---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Opendesign/
language: TypeScript, Go, Python
explored_at: 2026-07-10
---

# spec.md — src.Opendesign Projects

## Source

- Location: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Opendesign/`
- Files: 19,045+ total across 19 projects
- Languages: TypeScript (primary), Go (looper), Python (manim)
- License: MIT (most projects)

## The nexu Ecosystem

The **nexu ecosystem** is a suite of agent-native tools for design, development, and knowledge management. The platform centers on a local-first runtime model where AI coding agents are auto-detected on PATH, and local daemons coordinate between web interfaces, desktop shells, and cloud services.

### Projects

| Project | Language | Description |
|---------|----------|-------------|
| open-design | TypeScript | Agent-native design workspace — 26+ agent adapters, 150+ design systems, sandboxed iframe preview |
| nexu | TypeScript | Desktop client — Hono controller + React web + Electron shell + OpenClaw runtime orchestration |
| design | TypeScript | React component library + design tokens — 50+ primitives, 15 patterns, Storybook, theming |
| html-anything | TypeScript | Agentic HTML editor — 75 skills, 9 surfaces, SSE streaming, agent auto-detection |
| html-video | TypeScript | HTML→MP4 via pluggable engines — HyperFrames, Remotion adapters, 21 templates, AI soundtrack |
| looper | Go | Autonomous AI dev team — Planner→Reviewer→Fixer→Worker, git worktrees, GitHub/Forgejo |
| monet | TypeScript | Electron desktop scaffold — Hono controller + Next.js + Drizzle ORM + SQLite |
| agentic-design | Docs/Specs | Next-gen vision: chat-first, agent-native product lifecycle (PRD→pixel→production) |
| nexu-site | TypeScript | Landing page — Astro static site |
| open-design-bot-sandbox | TypeScript | Cloudflare Worker + D1 + R2 bot sandbox |
| open-design-contributor-card | TypeScript | Contributor card — Cloudflare Worker, D1, migrations |
| roast-skill | Config | Personality test skill for agents |
| sbti-skill | Config | SBTI personality test skill for agents |
| harness-engineering-guide | Docs | Engineering guide with changelogs, skills, site |
| manim | Python | 3Blue1Brown's math animation engine |
| src.hyprframes | Multi | HyperFrames video production ecosystem — 18 subprojects |
| src.motion-canvas | TypeScript | Motion Canvas generator-based animation + live preview |

## Tasks

| # | Task | Status |
|---|------|--------|
| 1 | spec.md | DONE |
| 2 | README.md (index) | DONE |
| 3 | 00-overview.md (update) | DONE |
| 4 | 01-architecture.md — Full ecosystem architecture | DONE |
| 5 | 02-nexu-platform.md — Nexu controller, web, desktop, slimclaw | DONE |
| 6 | 03-open-design.md — Daemon, web, contracts, brands engine | DONE |
| 7 | 04-agent-detection.md — Agent detection, registry, spawning | DONE |
| 8 | 05-design-systems.md — Design systems registry, brand engine | DONE |
| 9 | 06-design-tokens.md — Token system, theming, CSS variables | DONE |
| 10 | 07-component-library.md — Primitives, patterns, Storybook | DONE |
| 11 | 08-plugin-system.md — Plugin runtime, registry, protocol | DONE |
| 12 | 09-skillhub.md — Skill catalog, install, npm-runner, scanning | DONE |
| 13 | 10-html-anything.md — Skills, surfaces, SSE, iframe preview | DONE |
| 14 | 11-html-video.md — Engine adapters, templates, soundtrack | DONE |
| 15 | 12-looper.md — Agent roles, worktrees, loops, network mode | DONE |
| 16 | 13-monet.md — Desktop scaffold, sessions, live artifacts | DONE |
| 17 | 14-data-flow.md — End-to-end data flows | DONE |
| 18 | 15-developer-tooling.md — CLI, testing, CI, deployment | DONE |
| 19 | 16-smaller-projects.md — Bots, skills, manim, hyprframes | DONE |

## Documentation Structure

```
src.opendesign/
  markdown/
    README.md              ← Index / table of contents
    00-overview.md         ← Ecosystem overview
    01-architecture.md     ← Full dependency graph, layer diagram, module map
    02-nexu-platform.md    ← Nexu deep dive
    03-open-design.md      ← Open Design deep dive
    04-agent-detection.md  ← Agent detection system
    05-design-systems.md   ← Design systems registry
    06-design-tokens.md    ← Design token system
    07-component-library.md ← Component library
    08-plugin-system.md    ← Plugin system
    09-skillhub.md         ← SkillHub service
    10-html-anything.md    ← HTML anything
    11-html-video.md       ← HTML video
    12-looper.md           ← Looper AI dev team
    13-monet.md            ← Monet desktop scaffold
    14-data-flow.md        ← End-to-end data flows
    15-developer-tooling.md ← Developer tooling
    16-smaller-projects.md ← Smaller projects
  html/
    index.html             ← Auto-generated
    *.html                 ← Generated from markdown
  build.py                 ← Shared build script
```

## Build System

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations/[src.AI]/src.opendesign
python3 build.py src.opendesign   # Generate HTML
```

- Script: `build.py` — Python 3.12+ stdlib only, no dependencies
- Converts markdown to HTML with tables, code blocks, mermaid diagrams
- Embeds dark/light theme toggle with localStorage persistence
- Generates prev/next navigation between pages
- Idempotent (safe to re-run)

## Quality Requirements

All 10 Iron Rules from documentation_directive.md:
1. Detailed sections with code snippets
2. Teach key facts quickly (insight density)
3. Clear articulation (one idea per sentence)
4. Minimum 2 mermaid diagrams per document
5. Good visual assets (tables, ASCII, code blocks)
6. Generated HTML with navigation
7. Cross-references between documents
8. Source path references (file:line)
9. At least 1 Aha moment per document
10. Navigation: index + prev/next buttons

## Expected Outcome

After reading this documentation, a reader should be able to:
- Understand the full nexu ecosystem architecture and how 19 projects interconnect
- Explain the agent detection mechanism across 26+ adapters
- Navigate and contribute to any of the 19 projects
- Understand the design token system, component library, and theming
- Build, test, and deploy any project in the ecosystem
- Understand the plugin system and skill architecture

## Resume Point

If interrupted, check spec.md tasks table. Resume from the first TODO task. Re-read already-completed documents before continuing. Run `python3 build.py src.opendesign` after writing each batch of documents to verify HTML generation.
