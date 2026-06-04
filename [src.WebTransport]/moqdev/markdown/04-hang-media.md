---
title: hang — WebCodecs Media Encoding
---

# hang — WebCodecs Media Encoding

hang provides WebCodecs-compatible media encoding for MoQ broadcasts.

## Architecture

```
hang Media Layer:
┌────────────────────────────────────────┐
│  Catalog: JSON track with codec info   │
├────────────────────────────────────────┤
│  Tracks: Audio or video renditions     │
├────────────────────────────────────────┤
│  Containers: CMAF (fMP4) or Legacy     │
└────────────────────────────────────────┘
```

Source: `moq/rs/hang/src/` — Media encoding implementation.

## Codecs

| Codec | Support |
|-------|---------|
| H.264 | Catalog metadata (profile/level) |
| H.265 | Catalog metadata (profile/level) |
| AV1 | Catalog metadata (profile/level/bitdepth) |
| VP9 | Catalog metadata (profile/level/bitdepth/colorSpace/chromaSubsampling) |
| AAC | Catalog metadata (profile) |
| Opus | Audio codec |

> **Note:** hang defines catalog metadata structs (mimetype/profile information) and container frame types — it does NOT implement encoder/decoder logic. It has exactly 3 modules: `error`, `catalog`, `container`. The actual codec encoding/decoding happens elsewhere in the MoQ stack.

Source: `moq/rs/hang/src/codec/` — Codec implementations.

## Container Format

hang supports three container formats defined in `hang/src/catalog/container.rs`:
- **CMAF** (fMP4) — Common Media Application Format
- **Loc** — LOC container format
- **Legacy** — Simplified container for backward compatibility

Source: `moq/rs/hang/src/container/` — Container implementations.

**Aha:** hang's catalog is a JSON track containing codec information and metadata. This allows receivers to understand the media format before receiving any frames — they can set up decoders with the correct parameters before the first frame arrives.

## Related Documents

- [moq-mux](../markdown/05-moq-mux.md) — Media muxers
- [Data Flow](../markdown/09-data-flow.md) — Media streaming
