# MoqDev (src.MoqDev) Task Tracker

## Projects: Documented (1)

| Project | Status | Documents | Lines |
|---------|--------|-----------|-------|
| **moqdev** (combined ecosystem doc) | ✅ DONE | 12 docs + spec | 3,404 |

## Projects: Top-level dirs NOT in moqdev docs

### Binding Projects — HIGH PRIORITY
- [ ] `moq-go` (v0.2.15) — Go bindings, FFI via cgo
- [ ] `moq-go-ffi` — Go FFI bindings mirror
- [ ] `moq-swift` (v0.3.0) — Swift bindings, Package.swift
- [ ] `moq-swift-ffi` — Swift FFI bindings mirror
- [ ] `libmoq` (v0.3.1) — C FFI staticlib

### Application Projects — HIGH PRIORITY
- [ ] `moqbs` — OBS Studio fork with MoQ support (C/CMake + Swift + Metal + D3D11)
- [ ] `obs` — OBS Studio plugin for MoQ publishing
- [ ] `hang.live` — Live streaming web app (SolidJS/Vite 8 + Cloudflare Workers + Tauri)
- [ ] `moq.dev` — Project website (Astro framework)
- [ ] `doc.moq.dev` — Built documentation site (VitePress static assets)

### Test/Infrastructure — HIGH PRIORITY
- [ ] `smoke` — Cross-language smoke tests (7 languages: C, Go, JS-native, JS-browser, Kotlin, Python, Swift)
- [ ] `drafts` — IETF MoQ protocol specification drafts (XML)
- [ ] `gst` — GStreamer plugin for MoQ (deprecated)
- [ ] `web` — Frontend examples (Vite 5/6/7, Vue-MoQ template)

### Individual Crate Deep Dives — HIGH PRIORITY

The moqdev docs cover all 28 crates at overview level. Deep-dive docs exist for:
- ✅ moq-net (02-moq-net.md)
- ✅ moq-relay (03-moq-relay.md)
- ✅ hang (04-hang-media.md)
- ✅ moq-mux (05-moq-mux.md)
- ✅ web-transport (06-web-transport.md)
- ✅ kio (07-kio.md)
- ✅ Applications overview (08-applications.md) — moq-cli, moq-audio, moq-boy, moq-token

**NOT individually documented (ALL HIGH PRIORITY):**
- [ ] `moq-cli` — CLI tool deep dive
- [ ] `moq-ffi` — UniFFI bindings deep dive
- [ ] `moq-gst` — GStreamer plugin deep dive
- [ ] `moq-native` — QUIC/WebTransport client/server helpers
- [ ] `moq-token` — JWT token implementation deep dive
- [ ] `moq-token-cli` — Token CLI deep dive
- [ ] `moq-loc` — LOC frame encoding deep dive
- [ ] `moq-msf` — MSF catalog types deep dive
- [ ] `moq-video` — Video codec placeholder
- [ ] `moq-audio` — Opus codec deep dive
- [ ] `moq-boy` — Game Boy emulator deep dive
- [ ] `qmux` — QMux draft-01 protocol deep dive
- [ ] `web-transport-proto` — Core WebTransport protocol deep dive
- [ ] `web-transport-quinn` — Quinn backend deep dive
- [ ] `web-transport-iroh` — Iroh backend deep dive
- [ ] `web-transport-quiche` — QUICHE backend deep dive
- [ ] `web-transport-noq` — noq backend deep dive
- [ ] `web-transport-wasm` — WASM browser deep dive
- [ ] `web-transport-ffi` — C FFI deep dive
- [ ] `web-transport-node` — Node.js bindings deep dive
- [ ] `web-transport-trait` — Trait definitions deep dive

---

## Grandfather Review Discrepancies — moqdev

Waiting on review agent `ab335e4ddf2f2f309` to complete.
