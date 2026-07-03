# steal-a-gif: Screen Recording to GIF

Recover a clean, downloadable GIF from a screen recording of a hard-to-download animation.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/steal-a-gif/`

## Pipeline (6 stages)

```
Video Upload → Decode → Auto-Crop → BG Remove/Key → Loop Detect → Encode GIF
```

| Stage | File | Purpose |
|-------|------|---------|
| Decode | `pipeline/decode.ts` | Decode video frames via browser Video API |
| Auto-Crop | `pipeline/autoCrop.ts` | Detect content area, crop margins |
| BG Remove | `pipeline/removeBackground.ts` | Remove background |
| BG Key | `pipeline/bgKey.ts` | Chroma key background |
| Loop Detect | `pipeline/loopDetect.ts` | Find optimal loop points |
| Encode | `pipeline/encodeGif.ts` | Encode to GIF via gifenc |

## UI Components

| Component | File | Purpose |
|-----------|------|---------|
| `CropCanvas` | `src/ui/CropCanvas.tsx` | Interactive crop selection |
| `LoopSelector` | `src/ui/LoopSelector.tsx` | Loop point selection |

## Scripts

| Script | Purpose |
|--------|---------|
| `e2e.cjs` | End-to-end test |
| `extract-thumbnails.cjs` | Extract video thumbnails |
| `og-card.cjs` | Generate OG card images |
| `scroll-test.cjs` | Scroll performance test |
