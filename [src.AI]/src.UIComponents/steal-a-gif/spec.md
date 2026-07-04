---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/steal-a-gif/
language: TypeScript/React
explored_at: 2026-07-04
license: MIT
---

# spec.md -- steal-a-gif Documentation

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/steal-a-gif/`
- **Language:** TypeScript/React 19
- **Source files:** 14 (8 pipeline, 2 UI, 1 App, 1 entry, 1 style, 1 type declaration, 1 vite-env)
- **Script files:** 5 (+ 1 README)
- **Total files in repo:** ~35 (including public samples, config)
- **License:** MIT
- **GitHub:** https://github.com/vorpus/steal-a-gif

## What steal-a-gif Is

steal-a-gif is a browser-only web application that extracts clean, transparent GIFs from screen recordings of animated stickers. The user drops a video clip (from iMessage, TikTok, XHS, or anywhere), draws a crop box around the animation, trims to one loop cycle, and exports two GIFs: a full-size version and a Slack-emoji-sized version (128px, 128KB limit). Everything runs client-side -- nothing is uploaded to a server. The pipeline has six stages: decode, auto-crop, background keying, loop detection, background removal, and encode.

## Documentation Goal

A reader should understand:

1. The 6-stage pipeline architecture and how data flows between stages
2. How video decoding works with WebCodecs, mp4box demuxing, and slow-play fallback
3. How loop detection uses fingerprint comparison and period scoring
4. How background removal uses flood-fill keying (not AI matting) for solid-color sticker backgrounds
5. How GIF encoding with palette quantization and size constraints works
6. How the React UI (CropCanvas, LoopSelector) drives the pipeline
7. How the e2e, thumbnail, and OG-card scripts work

## Documentation Structure

```
steal-a-gif/
  spec.md
  markdown/
    README.md               -- Index / table of contents
    00-overview.md          -- What it is, 6-stage pipeline, quick start
    01-pipeline.md          -- Deep dive into all 6 pipeline stages
    02-ui-components.md     -- CropCanvas, LoopSelector, editors, landing page
    03-types.md             -- TypeScript types, data flow, constants
    04-scripts.md           -- E2E, thumbnails, OG card, scroll test
  html/
    index.html
    *.html
  build.py
```

## Tasks

| # | Document | Status |
|---|----------|--------|
| 1 | spec.md | DONE |
| 2 | README.md | DONE |
| 3 | 00-overview.md | DONE |
| 4 | 01-pipeline.md | DONE |
| 5 | 02-ui-components.md | DONE |
| 6 | 03-types.md | DONE |
| 7 | 04-scripts.md | DONE |
| 8 | HTML build | TODO |

## Build System

```bash
cd "[src.AI]/src.UIComponents/steal-a-gif" && python3 build.py
```

Zero dependencies. Python 3.12+ stdlib only. Converts markdown to HTML with Mermaid diagrams, dark/light theme, and prev/next navigation.

## Quality Requirements

Follow the documentation directive at `markdown_engineering/documentation_directive.md`: grandfather review (names match, numbers match, flows match, nothing missing), minimum 2 mermaid diagrams per document, at least 1 Aha moment per document, source path references, cross-references, code snippets.

## Expected Outcome

A reader can understand the full steal-a-gif pipeline, use the API, and replicate the architecture without reading source code.

## Resume Point

All markdown files written. Run `python3 build.py` to generate HTML.
