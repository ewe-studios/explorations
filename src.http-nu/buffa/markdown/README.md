# buffa — Pure-Rust Protocol Buffers

Protocol Buffers implementation in Rust (v0.5.2, MSRV 1.85). Full conformance test pass, zero-copy views, two-pass serialization, editions-first design.

## Source

- **Package:** `buffa` v0.5.2
- **Language:** Rust
- **166 source files across 8 workspace crates**

## Documentation

| Document | Description |
|----------|-------------|
| [00-overview](markdown/00-overview.md) | Workspace structure, key advantages over prost, dependencies |
| [01-core-runtime](markdown/01-core-runtime.md) | Message trait, wire encoding, SizeCache, editions, unknown fields |
| [02-views-zero-copy](markdown/02-views-zero-copy.md) | Zero-copy views, OwnedView, ViewEncode, ViewReborrow |
| [03-codegen-build](markdown/03-codegen-build.md) | Code generation pipeline, protoc plugin, build.rs integration |

## Key Features

- **Two-pass serialization** — O(n) via SizeCache, not O(depth^2) like prost
- **Zero-copy views** — `MessageView<'a>` borrows directly from wire bytes
- **Editions-first** — proto2/proto3 as feature presets, single code path
- **no_std + alloc** — core runtime works without std
- **Unknown field preservation** — round-trip fidelity for proxies
- **Open enums** — `EnumValue<E>` preserves unknown values from the wire
