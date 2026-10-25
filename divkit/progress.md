# DivKit Exploration — Progress Tracker (deepening pass)

## Goal

Deepen the DivKit exploration so every area is covered at source level, not
summarized: rendering, HTML hydration, web communication, live sync, partial
updates, diffing, and the native-rendering optimizations that make DivKit
production-ready and fast. Then pull through the GitHub issues and PRs that
resolved bugs, improved performance, optimized bundle size, and improved server
delivery. Finally document the sandbox (visual editor) and how it renders the
same card on Android and iOS natively.

## Source

- `divkit`: `/Users/darkvoid/GitSources/src.sdui/divkit`
- `divkit-ios`: `/Users/darkvoid/GitSources/src.sdui/divkit-ios`
- `divkit-ios-facade`: `/Users/darkvoid/GitSources/src.sdui/divkit-ios-facade`
- Docs: `/Users/darkvoid/Dev/explorations/divkit/markdown/` (00–21 exist)
- Build: `cd /Users/darkvoid/Dev/explorations && python3 build.py divkit`

## Iron rules (every new doc)

1. At least 2 Mermaid diagrams; at least 3 verbatim code snippets; at least
   1 Aha moment; at least 2 cross-references.
2. Footer `Continue to [next].md`; no orphan pages.
3. ASD-STE100 Simplified Technical English: short sentences, spell out every
   abbreviation at first use, one idea per sentence.
4. Every name, number, and flow grounded in source with a file path.

## Area breakdown

| # | Area | Agent | New doc | Status |
|---|------|-------|---------|--------|
| A | Web rendering, SSR/hydration, communication | Agent A | `22-web-rendering-hydration.md` | verified |
| B | Live sync + sandbox (visual editor ↔ native demo) | Agent B | `23-live-sync-sandbox.md` | verified |
| C | Native rendering optimization + production tricks | Agent C | `24-native-rendering-performance.md` | verified |
| D | Issues, PRs, changelogs, bundle size, server delivery | Agent D | `25-issues-prs-delivery.md` | verified |
| E | Rendering measure/layout depth: Android LayoutParams/Gravity + Web CSS compilation | Wave 2 agent | `26-platform-geometry-compilation.md` | verified |
| F | Verification of new docs (3 agents × ~3 docs) | Wave 3 | — | done |

## New document chain (footer order)

```
21-rendering-and-patch.md  → 22-web-rendering-hydration.md
22-web-rendering-hydration.md → 23-live-sync-sandbox.md
23-live-sync-sandbox.md → 24-native-rendering-performance.md
24-native-rendering-performance.md → 25-issues-prs-delivery.md
25-issues-prs-delivery.md → 26-platform-geometry-compilation.md
26-platform-geometry-compilation.md → README.md
```

## Wave 1 (research + draft, 4 agents in parallel)

- Agent A → `22-web-rendering-hydration.md`
- Agent B → `23-live-sync-sandbox.md`
- Agent C → `24-native-rendering-performance.md`
- Agent D → `25-issues-prs-delivery.md`

## Wave 2 (gap fill)

Distribute remaining areas after Wave 1 reports: rendering measure/layout
internals, diffing specifics, Compose/Flutter rendering, anything Wave 1 could
not fully ground.

## Wave 3 (verification)

Three agents, each verifying a set of docs against source (names, numbers,
flows, public API coverage) and correcting in place.

Result (independent, not self-verified): all docs sound after correction.
Corrections applied: 11×3, 12×1, 13×2, 14×3, 21×4, 22×3, 23×2, 24×1, 25×14
(7 factual + 2 quote completions + 5 link conversions), 26×1. No issue/PR
numbers invented. Footer chain and iron rules intact on every doc.

## Notes

- `gh` CLI has an invalid `GITHUB_TOKEN`; use `git log` + local CHANGELOG files
  for the issues/PRs pass (Agent D).
- Live sync exists: Android demo `div/editor/` (WebSocket, neovisionaries) and
  iOS `DivKitPlayground/WebPreview/` (WebPreviewSocket). The web `visual-editor`
  is the sandbox front end.
