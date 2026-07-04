---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.TenFramework/
language: C/C++, Rust, Go, Python, TypeScript
explored_at: 2026-07-04
license: Apache-2.0
---

# spec.md — TEN Framework Documentation

## Source

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.TenFramework/`
- **Language:** C/C++ (core runtime), Rust (CLI + validation), Go/Python/Node.js (bindings)
- **LOC:** ~4.7M lines across 4 sub-projects
- **License:** Apache 2.0

## What TEN Framework Is

TEN (Transformational Extension Network) is an open-source framework for building real-time multi-modal conversational AI agents. It provides a graph-based message-passing architecture where extensions (STT, LLM, TTS, tools) connect as nodes in a directed graph. The core runtime is C for low-latency execution; the package manager and graph editor are Rust; extensions are written in Python, Go, C++, or Node.js. Includes 95 pre-built extensions, 30 agent examples, a high-performance VAD library, and a Transformer-based turn-detection model.

## Sub-Projects

1. **ten-framework** — Core runtime + 95 extensions + TMAN CLI + agent examples
2. **ten-vad** — High-performance voice activity detection (~306KB, 50x smaller than Silero)
3. **ten-turn-detection** — Qwen2.5-7B-based semantic turn detection
4. **portal** — Documentation website (Next.js + Fumadocs)

## Documentation Structure

```
src.TenFramework/
├── spec.md, build.py, markdown/, html/
```
