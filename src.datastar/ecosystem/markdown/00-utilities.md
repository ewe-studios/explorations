# Ecosystem -- Utilities & Tools

## klump

**Path**: `src.datastar/klump/`  
**Language**: Rust  
**Dependencies**: fjall 3.0.1, scru128

A minimal utility crate that provides a thin wrapper around fjall + scru128 for simple ordered key-value operations. Likely a prototype or extraction of the storage primitives used by xs.

## m2h

**Path**: `src.datastar/m2h/`  
**Version**: 0.2.0  
**Repository**: https://github.com/cablehead/m2h  
**Language**: Rust

Convert Markdown to HTML with syntax highlighting. A standalone binary/library that combines pulldown-cmark with syntect. Used as the markdown rendering engine for http-nu's `html md` command and for building documentation sites.

### Key features:
- CommonMark parsing
- Syntax highlighting for fenced code blocks
- Single binary output
- Streaming or batch conversion

## patrol

**Path**: `src.datastar/patrol/`  
**Language**: Likely Nushell scripts

A monitoring/watcher utility. Based on the name and ecosystem context, likely watches for file system changes or process health and reports to xs via frames.

## stacks

**Path**: `src.datastar/stacks/`  
**Language**: Rust (18 source files)

A stack-based tooling application. In the context of the xs ecosystem, "stacks" likely refers to a layered configuration or environment management tool — managing collections of related services/configs as named stacks.

## win_uds

**Path**: `src.datastar/win_uds/`  
**Version**: 0.2.2  
**Repository**: https://github.com/kouhe3/win_uds  
**Language**: Rust  
**License**: Unlicense

Windows Unix Domain Socket compatibility layer. Provides synchronous `UnixStream`/`UnixListener` types (built on socket2) plus async variants (`AsyncStream`/`AsyncListener`) implementing `futures_io::{AsyncRead, AsyncWrite}`. Use `tokio_util::compat` to adapt for tokio.

### Key types:
- `UnixStream` — Synchronous UDS stream for Windows
- `UnixListener` — Synchronous UDS listener for Windows
- `AsyncStream` — Async UDS stream (futures_io traits)
- `AsyncListener` — Async UDS listener (futures_io traits)

Used by xs's and http-nu's listener modules for cross-platform Unix socket support.

## syntect-SyntaxSet-with-nushell

**Path**: `src.datastar/syntect-SyntaxSet-with-nushell/`  
**Crate name**: stacks-syntaxset  
**Language**: Rust  
**Dependencies**: syntect 5.2.0

Builds a custom syntect `SyntaxSet` that includes Nushell syntax definitions. Used at compile time by http-nu and m2h to provide Nushell syntax highlighting in code blocks.
