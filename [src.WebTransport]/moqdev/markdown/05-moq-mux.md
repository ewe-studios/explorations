---
title: moq-mux — Media Muxers and Demuxers
---

# moq-mux — Media Muxers and Demuxers

moq-mux provides encoding/decoding for multiple codec formats and container formats.

## Supported Codecs

| Codec | Parse | Mux |
|-------|-------|-----|
| H.264 | ✅ | ✅ |
| H.265 | ✅ | ✅ |
| AV1 | ✅ | ✅ |
| AAC | ✅ | ✅ |
| Opus | ✅ | ✅ |

Source: `moq/rs/moq-mux/src/codec/` — Codec implementations.

## Supported Containers

| Container | Parse | Mux | Module |
|-----------|-------|-----|--------|
| fMP4 (CMAF) | ✅ | ✅ | `fmp4/` |
| HLS (M3U8) | ✅ | — | `hls/` |
| Legacy | ✅ | ✅ | `legacy/` |
| LOC | ✅ | ✅ | `loc/` |
| MKV/WebM | ✅ | ✅ | `mkv/` |

> **Note:** 5 container modules. MP4 and WebM are NOT standalone containers — fMP4 handles CMAF/fMP4, and WebM is handled inside the `mkv/` module (MKV/WebM share the Matroska format).

## Catalog Support

moq-mux supports two catalog types:
- **hang** — WebCodecs catalog
- **msf** — MSF catalog

Source: `moq/rs/moq-mux/src/catalog/` — Catalog implementations.

## Related Documents

- [hang](../markdown/04-hang-media.md) — Media encoding
- [Data Flow](../markdown/09-data-flow.md) — Media streaming
