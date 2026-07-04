# Remix 3: Design Philosophy

Remix 3 is a complete rethinking of web framework design, optimized for LLM-first development.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/remix/`

## 6 Core Principles

### 1. Model-First Development

AI fundamentally shifts human-computer interaction. Optimize source code, documentation, tooling, and abstractions for LLMs. Develop abstractions for applications to use models in the product itself.

### 2. Build on Web APIs

Sharing abstractions across the stack reduces context switching for both humans and machines. Build on Web APIs and JavaScript — the only full-stack ecosystem.

Uses:
- **Web Streams API** instead of `node:stream`
- **Uint8Array** instead of Node.js `Buffer`
- **Web Crypto API** instead of `node:crypto`

### 3. Religiously Runtime

Design for no static analysis expectations. All tests run without bundling. Browsers involved → `--import` loaders for TypeScript/JSX transformations are permissible.

### 4. Avoid Dependencies

Dependencies lock you into someone else's roadmap. Choose wisely, wrap completely, expect to replace with own package. Goal: zero dependencies.

### 5. Demand Composition

Single-purpose, replaceable abstractions. Easy to add and remove. Every package must be useful and documented independent of context. New features first attempted as new package.

### 6. Distribute Cohesively

Composable ecosystems are hard to learn. Remix distributed as single `remix` package for distribution and documentation.

## Portability by Default

Each package works across:
- Node.js
- Bun
- Deno
- Cloudflare Workers
- Other environments

## Next

Continue to [02-packages](02-packages.md) for all 25 package details.
