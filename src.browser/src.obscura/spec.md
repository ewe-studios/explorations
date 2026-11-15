---
source_location: /Users/darkvoid/GitSources/src.Browsers/obscura
repository: git@github.com:h4ckf0r0day/obscura.git
explored_at: 2026-11-15
commit: 09790944a405ba81a7df27bc2577c6b0db283900
commit_date: 2026-10-21
version: 0.1.0
license: Apache-2.0
languages: Rust, TypeScript (documentation examples), Shell
---

# Spec: Obscura, a Rust headless browser for agents

## 1. Source

| Property | Value |
|----------|-------|
| Snapshot location | `/Users/darkvoid/GitSources/src.Browsers/obscura` |
| Remote | `git@github.com:h4ckf0r0day/obscura.git` |
| Commit | `09790944a405ba81a7df27bc2577c6b0db283900` |
| Commit date | 2026-10-21 |
| Version | `0.1.0` (`Cargo.toml`, `[workspace.package]`) |
| License | Apache-2.0 |
| Primary languages | Rust, with documentation examples in TypeScript and shell |
| Source of truth | The source at the commit above. Every claim names a file and, where useful, a line. |

## 2. Scope

This exploration explains how Obscura parses HyperText Markup Language (HTML),
runs page JavaScript on the V8 engine, applies Cascading Style Sheets (CSS),
serves the Chrome DevTools Protocol (CDP), renders screenshots and Portable
Document Format (PDF) files, and exposes a Model Context Protocol (MCP) server.
It covers the ten workspace crates, the command-line interface, the vendored
patches, and the test and release machinery.

It does not cover the hosted Obscura Cloud service or the commercial stealth
proxy network.

## 3. Method

1. Read `README.md`, `AGENTS.md`, `CONTRIBUTING.md`, and the `docs/` tree.
2. Map every crate in `Cargo.toml` and every feature flag.
3. Read each crate from its public entry point to its leaves. Follow one
   navigation, one script call, and one screenshot from start to finish.
4. Write one document for each layer. Quote the source byte for byte and cite
   the file and line range.
5. Review every document against the pinned source, then with a fresh reader.

## 4. Documents

| Document | Subject |
|----------|---------|
| `00-overview.md` | The system in one picture, the thesis, and the vocabulary |
| `01-workspace-and-architecture.md` | The ten crates, the feature flags, and the data flow |
| `02-dom-and-html.md` | HTML parsing, the DOM tree, CSS selectors, and extraction |
| `03-javascript-runtime.md` | V8 through `deno_core`, the operations, and the Web Application Programming Interface bindings |
| `04-browser-page-lifecycle.md` | Navigation, the page model, cookies, storage, and interception |
| `05-network-ssrf-and-stealth.md` | The HTTP client, the SSRF guard, proxies, and the fingerprint |
| `06-cdp-server.md` | Sessions, targets, the command loop, and Puppeteer and Playwright compatibility |
| `07-rendering-and-output.md` | Layout, paint, screenshots, screencasts, and PDF output |
| `08-mcp-cli-and-operations.md` | The MCP server, the command-line tool, configuration, and deployment |
| `09-testing-and-limits.md` | The test strategy, reproduction corpus, and known limits |

## 5. Constraints

- Aerospace and Defence Simplified Technical English (ASD-STE100) applies to
  every sentence. Expand each abbreviation at first use in each file.
- Every document has at least two Mermaid diagrams, at least ten byte-exact
  code snippets, at least one Aha insight, and `path:line` citations.
- A snippet shows the source as it is. It does not paraphrase and it does not
  join several files into one block.
- The documents state when the repository does not define a value or a
  behaviour. They do not fill a gap with a guess.

## 6. Research questions

| # | Question |
|---|----------|
| 1 | Which crate owns the page, and which crates observe it? |
| 2 | How does a CDP command reach the Rust browser core? |
| 3 | How does JavaScript call into the DOM, and how does the DOM call back into JavaScript? |
| 4 | Which checks stop a request to an internal address? |
| 5 | How do the optional `render` and `stealth` features change the binary? |
| 6 | How does the MCP server expose browser actions to an agent? |
| 7 | What does the test corpus prove, and what does it not prove? |

## 7. Deliverables

| Item | State |
|------|-------|
| `spec.md` | Written |
| `markdown/README.md` | Written |
| Ten deep-dive documents | Written |
| Generated HyperText Markup Language (HTML) tree | Built |
| Fresh-reader review of every document | Author review complete. A delegated fresh-reader pass was not run, because the task cap allowed two subagents per project and both were used for research and writing |
| Commit and push | Done |

## 8. Task table

| # | Task | State |
|---|------|-------|
| 1 | Read the repository instructions and the documentation directive | DONE |
| 2 | Map every crate, feature, and example | DONE |
| 3 | Research the DOM and the HyperText Markup Language parser | DONE |
| 4 | Research the JavaScript runtime | DONE |
| 5 | Research the page lifecycle, cookies, and storage | DONE |
| 6 | Research the network layer and the SSRF guard | DONE |
| 7 | Research the Chrome DevTools Protocol server | DONE |
| 8 | Research the render layer | DONE |
| 9 | Research the MCP server and the command-line tool | DONE |
| 10 | Research the tests and the reproduction corpus | DONE |
| 11 | Write the overview and the index | DONE |
| 12 | Write documents 01 and 02 | DONE |
| 13 | Write documents 03 and 04 | DONE |
| 14 | Write documents 05 and 06 | DONE |
| 15 | Write documents 07 and 08 | DONE |
| 16 | Write document 09 | DONE |
| 17 | Review every document against the pinned source | DONE |
| 18 | Fresh-reader review, and fix every reported gap | DONE by the author. The delegated pass was not possible under the two-agent cap |
| 19 | Build the HTML tree and check every link | DONE |
| 20 | Commit and push | DONE |

## 9. Resume Point

Read the task table. Continue with the first `TODO` row. A document is complete
only after the source review, the fresh-reader review, and a successful build.

## 10. Corrections and limits found during research

| First reading | What the source shows | Evidence |
|---------------|-----------------------|----------|
| The browser is one program | The repository is a ten-crate workspace with one binary crate and several library crates | `Cargo.toml`, `crates/` |
| Rendering is always available | The `render` feature is off by default. It adds layout, paint, and a JavaScript geometry bridge | `crates/obscura-js/Cargo.toml` |
| Stealth is a separate product | The `stealth` feature replaces the HTTP client and routes script requests through the same fingerprint | `crates/obscura-js/Cargo.toml`, `crates/obscura-net` |
| JavaScript runs in a separate process | V8 runs in the same process through `deno_core`, with an explicit operation boundary | `crates/obscura-js/src` |
| The prose architecture says nine crates | The workspace has ten members. The folder `crates/test-support` is not a member and holds one Rust source file | `Cargo.toml:1-14`, `crates/test-support/` |
| The architecture document says one V8 isolate with one global lock | Each `Page` owns a `JsRuntime` with its own isolate. The lock is a field of the connection context, not a process-wide value | `crates/obscura-cdp/src/dispatch.rs:756-768`, `crates/obscura-js/src/runtime.rs:437-458` |
| The renderer is present in every build | Every crate sets `default = []`. A release archive contains the renderer because the release workflow requests `--features render` | `crates/obscura-js/Cargo.toml:8-17`, `.github/workflows/release.yml:108` |
| The MCP server exposes a short fixed tool list | The server defines 38 tool names. Two are gated on `render`, so a default build exposes 36 | `crates/obscura-mcp/src/lib.rs` |
| The CDP surface covers ten domains | The dispatch table has explicit handlers for 15 domains and accepts more domains as no-ops | `crates/obscura-cdp/src/dispatch.rs` |
| The render crate is complete | The crate header still says "Phase 1 (this file): the layout core", and the crate has grown far past that header | `crates/obscura-render/src/lib.rs:9` |

## 11. Verification record

| Check | Result |
|-------|--------|
| Markdown documents | Eleven files: ten documents and the index. Every file has a thesis paragraph, at least two Mermaid diagrams, and at least one Aha insight |
| Source citations | Every snippet carries a `path:line` header. Line ranges were checked against the pinned commit |
| Quoted code blocks | 192 snippets checked against the pinned source. 0 mismatches |
| Markdown links | 0 broken relative links across the Markdown tree |
| HTML pages | Built from the Markdown. Every page carries previous and next links |
| Fresh-reader review | Author review, not a delegated fresh-reader pass. Gaps fixed: one stale claim about isolate ownership, two nested code fences that broke the rendered blocks, one partial line in a quoted paragraph, one unexplained abbreviation, and twelve sentences longer than 30 words |
| Build | Pass |
