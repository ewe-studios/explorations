---
title: Astro Compiler Documentation
---

# Astro Compiler Documentation

Complete documentation for the Astro compiler's Go/TypeScript and Rust implementations. Covers parsing, transformation, code generation, N-API bindings, and the full .astro syntax specification.

## Foundation

- [Overview](./00-overview.md) — What the compiler is, dual-implementation architecture, pipeline
- [Syntax Specification](./08-syntax-spec.md) — Complete .astro file format specification

## Parser Deep Dives

- [Go Parser](./01-go-parser.md) — Tokenizer, HTML5 insertion modes, frontmatter handling, expression parsing
- [Rust Parser](./03-rust-parser.md) — oxc-based parsing, AstroRoot AST, scanner pass

## Code Generation

- [Go Printer](./02-go-printer.md) — render1, attribute printing, metadata, import hoisting
- [Rust Code Generation](./06-code-generation.md) — AstroCodegen, TypeScript stripping, sourcemaps

## Integration

- [N-API Bindings](./04-napi-bindings.md) — compileAstro, parseAstro, sync/async API, error handling
- [Compiler API](./07-compiler-api.md) — TypeScript API surface, types.d.ts, transform/parse/compile

## Transforms and Cross-Cutting

- [AST Transforms](./05-ast-transforms.md) — CSS scoping, directive resolution, component metadata, whitespace
- [Cross-Cutting Concerns](./09-cross-cutting.md) — Sourcemaps, whitespace strategies, performance, error handling

## Source Repositories

- Go/TypeScript: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Astro/compiler/`
- Rust: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Astro/compiler-rs/`
- GitHub: https://github.com/withastro/compiler
