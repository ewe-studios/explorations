# What Is steal-a-gif?

Recover a clean, downloadable GIF from a screen recording of a hard-to-download animation.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/steal-a-gif/`

## Architecture

```
Video → Decode → Auto-Crop → BG Remove/Key → Loop Detect → Encode GIF
```

## Pipeline Stages

| Stage | File | Purpose |
|-------|------|---------|
| Decode | `pipeline/decode.ts` | Decode video frames |
| Auto-Crop | `pipeline/autoCrop.ts` | Detect and crop to content area |
| BG Remove | `pipeline/removeBackground.ts` | Remove background |
| BG Key | `pipeline/bgKey.ts` | Chroma key background |
| Loop Detect | `pipeline/loopDetect.ts` | Find optimal loop points |
| Encode | `pipeline/encodeGif.ts` | Encode to GIF with gifenc |

## UI Components

| Component | File | Purpose |
|-----------|------|---------|
| CropCanvas | `src/ui/CropCanvas.tsx` | Interactive crop selection |
| LoopSelector | `src/ui/LoopSelector.tsx` | Loop point selection |

## Scripts

| Script | Purpose |
|--------|---------|
| `e2e.cjs` | End-to-end test |
| `extract-thumbnails.cjs` | Extract video thumbnails |
| `og-card.cjs` | Generate OG card images |
