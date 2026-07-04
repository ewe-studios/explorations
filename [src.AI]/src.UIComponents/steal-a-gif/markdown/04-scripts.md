# Scripts & Tooling

Three automation scripts handle E2E testing, thumbnail regeneration, and OG card generation. All run with Node.js (CommonJS), not browser-based.

Source: `scripts/` directory — 5 files, ~200 lines total.

## E2E Test: `e2e.cjs`

Source: `scripts/e2e.cjs` — ~80 lines.

A smoke test that drives the full editor flow via Puppeteer:

```bash
npm run dev          # terminal 1: serve on :5173
node scripts/e2e.cjs # terminal 2: run test
```

### What It Tests

1. **Load a sample** — clicks the "Ghost" button, then a sample tile
2. **Wait for crop canvas** — waits for `.cropcanvas` to appear (up to 20s)
3. **Draw a crop box** — synthesizes mouse drag on the canvas center
4. **Verify "Next · Trim" disabled** → enabled (box is required)
5. **Step through wizard** — clicks Next·Trim, Next·Export, Make GIF
6. **Wait for outputs** — polls for `.outcard` elements (up to 40s)
7. **Verify results** — asserts outputs exist, no errors

### What It Reports

```
Next·Trim disabled  before box: true  after box: false
make-gif outcome: outputs:2
--- relevant logs ---
[steal-a-gif] accurate decode failed; using preview ...
[steal-a-gif] window 1200–2400ms · 45 decoded → 12 distinct frames
```

The e2e script also prints console logs, so you can verify the memory fix (cropped frame dimensions in the log).

### Viewport

```js
await page.setViewport({ width: 390, height: 800 });
```

Tests mobile viewport by default — the most constrained layout.

## Scroll Test: `scroll-test.cjs`

A simpler script that verifies scrolling behavior in the app (used during development to test the crop canvas interaction on scrollable pages).

## Thumbnail Extraction: `extract-thumbnails.cjs`

Source: `scripts/extract-thumbnails.cjs` — ~60 lines.

Regenerates sample-tile thumbnails for every video in `public/samples/`:

```bash
npm run dev
node scripts/extract-thumbnails.cjs
```

### How It Works

1. Lists all `.mp4`, `.mov`, `.m4v` files in `public/samples/`
2. For each file, creates a `<video>` element in Puppeteer
3. Seeks to `currentTime = 0.05` (first frame)
4. Draws to a canvas, scales to max 360px edge
5. Writes `<name>.thumb.png`

### Square Thumbnails

```bash
SQUARE=xhs-smelly.MP4 node scripts/extract-thumbnails.cjs
```

When `SQUARE=<file>` is set, also writes `<name>.before.png` — a center-cropped 256×256 square used for the intro before/after demo.

## OG Card Generation: `og-card.cjs`

Source: `scripts/og-card.cjs` — ~100 lines.

Generates social share cards (Open Graph / Twitter Card) using `@resvg/resvg-js`:

```bash
node scripts/og-card.cjs
```

Outputs:
- `public/og.png` (1200×630) — Standard OG card
- `public/og-square.png` (1200×1200) — Square variant

### How It Works

1. **Fetches fonts** — Baloo 2 and Nunito variable TTFs from Google Fonts (cached in `scripts/.fonts/`)
2. **Loads sample art** — `xhs-smelly.before.png` (before) and `example-export.gif` (after) as data URIs
3. **Renders SVG** — Constructs an SVG template with brand tokens
4. **Converts to PNG** — Uses Resvg for fast, server-side rendering

Aha: The OG card uses `@resvg/resvg-js` instead of a headless browser. This means no Chrome/Puppeteer needed — just an SVG template and a renderer. The before/after demo reuses the same sample art as the in-app intro, so the card always matches what the site shows.

### Brand Tokens

The card mirrors `src/styles.css` colors, fonts, and layout to ensure the social share image looks identical to the app.

## Puppeteer Helper: `lib.cjs`

Source: `scripts/lib.cjs` — shared launch utility.

Provides the `launch()` function and `URL` constant used by all scripts:

```js
const { launch, URL } = require("./lib.cjs");
```

Launches a headless Chrome instance pointed at the dev server.

## Cross-References

- [Overview](00-overview.md) — Package summary
- [Pipeline Deep Dive](01-pipeline.md) — What the E2E test exercises
- [UI Components](02-ui.md) — What the E2E test drives
