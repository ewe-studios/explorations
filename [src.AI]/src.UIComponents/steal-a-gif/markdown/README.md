# steal-a-gif Documentation

Recover a clean, downloadable GIF from a screen recording of a hard-to-download animation. Entirely in the browser — no server, no FFmpeg, no upload.

| Attribute | Value |
|-----------|-------|
| Source files | ~35 (src/ + scripts/) |
| Pipeline | 6 stages (decode → dedupe → loop → crop → bg → encode) |
| React | 19 |
| Deploy | GitHub Pages |

## Navigation

| # | Document | Purpose |
|---|----------|---------|
| 00 | [Overview](00-overview.md) | What it is, pipeline architecture, quick start |
| 01 | [Pipeline Deep Dive](01-pipeline.md) — All 6 stages with code, algorithms, and aha moments |
| 02 | [UI Components](02-ui.md) | CropCanvas, LoopSelector, 3-step wizard |
| 03 | [Types & Data Flow](03-types.md) | Frame, Rect, LoopRange, Prepared, ExtractResult |
| 04 | [Scripts](04-scripts.md) | E2E tests, thumbnail extraction, OG card generation |
