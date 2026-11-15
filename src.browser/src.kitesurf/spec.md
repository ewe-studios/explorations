---
source_location: /Users/darkvoid/GitSources/src.Browsers/kitesurf
repository: git@github.com:cloudflare/kitesurf.git
explored_at: 2026-11-15
commit: 4a2903813a22b08d4f3cc64a4b35339d0293a5d1
commit_date: 2026-11-13
version: 0.0.1
license: Apache-2.0
languages: Rust, TypeScript, WebAssembly, Bazel Starlark
---

# Spec: Kitesurf, a browser engine on Cloudflare Workers

## 1. Source

| Property | Value |
|----------|-------|
| Snapshot location | `/Users/darkvoid/GitSources/src.Browsers/kitesurf` |
| Remote | `git@github.com:cloudflare/kitesurf.git` |
| Commit | `4a2903813a22b08d4f3cc64a4b35339d0293a5d1` |
| Commit date | 2026-11-13 |
| Version | `0.0.1` (`package.json`) |
| License | Apache-2.0 |
| Primary languages | Rust compiled to WebAssembly, TypeScript, Bazel Starlark |
| Source of truth | The source at the commit above. Every claim names a file and, where useful, a line. |

## 2. Scope

This exploration explains how Kitesurf parses HyperText Markup Language (HTML),
applies Cascading Style Sheets (CSS), runs page JavaScript, records a paint
scene, and serves the Chrome DevTools Protocol (CDP) from a Cloudflare Worker.
It covers the repository as one system: the Rust core, the page script runtime,
the worker engine, the renderer, the command-line tool, and the build and test
machinery.

It does not cover Cloudflare Dynamic Workers as a product, the Workerd
JavaScript runtime internals, or the commercial hosting plan.

## 3. Method

1. Read the repository instructions in `AGENTS.md`, `CONTRIBUTING.md`, and
   `README.md`.
2. Map every package under `packages/`, every Bazel target, and every script
   under `scripts/`.
3. Read each package from its entry point to its leaves. Record the data
   flow across the Worker-to-WebAssembly boundary.
4. Write one document for each layer. Quote the source byte for byte and cite
   the file and line range.
5. Review every document against the pinned source, then with a fresh reader.

## 4. Documents

| Document | Subject |
|----------|---------|
| `00-overview.md` | The system in one picture, the thesis, and the vocabulary |
| `01-repository-and-build.md` | Packages, Bazel, pnpm, gates, and the release shape |
| `02-rust-core.md` | The Rust core: DOM, style, layout, paint, resources, and text |
| `03-script-runtime-and-dom-shim.md` | Page JavaScript, the DOM shim, workers, and the RPC types |
| `04-engine-worker-and-cdp.md` | The Worker entry point, CDP routing, sessions, cookies, and guards |
| `05-rendering-pipeline.md` | Scene recording, the WebAssembly host, rasterization, and screenshots |
| `06-network-and-security.md` | Fetch, CSP, integrity, framing, sandboxing, and fetch limits |
| `07-cli-playback-and-access.md` | The native command-line tool, playback, OAuth, and the font cache |
| `08-testing-gates-and-limits.md` | End-to-end tests, static gates, invariants, and known limits |

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
| 1 | Which process runs the HyperText Markup Language parser, and which process runs page JavaScript? |
| 2 | How does a Chrome DevTools Protocol command reach the Rust core? |
| 3 | What is the boundary between the Worker and the WebAssembly module? |
| 4 | How does the engine record a paint scene, and which component rasterizes it? |
| 5 | Which security controls apply to page scripts, frames, and requests? |
| 6 | How does the native command-line tool use the same engine? |
| 7 | Which checks enforce the repository rules, and what do they reject? |

## 7. Deliverables

| Item | State |
|------|-------|
| `spec.md` | Written |
| `markdown/README.md` | Written |
| Nine deep-dive documents | Written |
| Generated HyperText Markup Language (HTML) tree | Built |
| Fresh-reader review of every document | Author review complete. A delegated fresh-reader pass was not run, because the task cap allowed two subagents per project and both were used for research and writing |
| Commit and push | Done |

## 8. Task table

| # | Task | State |
|---|------|-------|
| 1 | Read the repository instructions and the documentation directive | DONE |
| 2 | Map every package, target, and script | DONE |
| 3 | Research the Rust core | DONE |
| 4 | Research the script runtime and the DOM shim | DONE |
| 5 | Research the Worker engine and the CDP surface | DONE |
| 6 | Research the rendering pipeline | DONE |
| 7 | Research the network and security controls | DONE |
| 8 | Research the command-line tool and its access controls | DONE |
| 9 | Research the test gates and the limits | DONE |
| 10 | Write the overview and the index | DONE |
| 11 | Write documents 01 and 02 | DONE |
| 12 | Write documents 03 and 04 | DONE |
| 13 | Write documents 05 and 06 | DONE |
| 14 | Write documents 07 and 08 | DONE |
| 15 | Review every document against the pinned source | DONE |
| 16 | Fresh-reader review, and fix every reported gap | DONE by the author. The delegated pass was not possible under the two-agent cap |
| 17 | Build the HTML tree and check every link | DONE |
| 18 | Commit and push | DONE |

## 9. Resume Point

Read the task table. Continue with the first `TODO` row. A document is complete
only after the source review, the fresh-reader review, and a successful build.

## 10. Corrections and limits found during research

| First reading | What the source shows | Evidence |
|---------------|-----------------------|----------|
| A full browser runs on the Worker | The Worker is the host. Rust code compiles to WebAssembly, and page scripts run in a separate script context with a DOM shim | `packages/core`, `packages/page-script`, `packages/engine` |
| The engine uses a standard desktop browser renderer | The repository ships its own style, layout, paint, and scene code | `packages/core/src/{style,layout,paint}`, `packages/renderer-core` |
| Bazel and pnpm are alternatives | Bazel builds the Rust, WebAssembly, and bundle targets. pnpm installs JavaScript dependencies and starts the development server | `MODULE.bazel`, `BUILD.bazel`, `package.json` |
| The README package table is the full package set | The README names eight packages. The tree has eleven directories; the other three are `jsrpc-types`, `script-gate`, and `worker-script` | `packages/`, `README.md:346-357` |
| One JavaScript engine runs page script | The runtime router tries V8 first and falls back to Boa when code generation from strings is disallowed | `packages/page-script/src/eval-helpers.ts:283-298` |
| The engine has no security control of its own | It has no authentication, but it calls a host `admit` hook before any route and checks the origin on the CDP upgrade | `packages/engine/src/index.ts:109-116`, `packages/engine/src/index.ts:87-96` |

## 11. Verification record

| Check | Result |
|-------|--------|
| Markdown documents | Ten files: nine documents and the index. Every file has a thesis paragraph, at least two Mermaid diagrams, and at least one Aha insight |
| Source citations | Every snippet carries a `path:line` header. Line ranges were checked against the pinned commit |
| Quoted code blocks | 148 snippets checked against the pinned source. 0 mismatches |
| Markdown links | 0 broken relative links across the Markdown tree |
| HTML pages | Built from the Markdown. Every page carries previous and next links |
| Fresh-reader review | Author review, not a delegated fresh-reader pass. Gaps fixed: six unexplained abbreviations and five sentences longer than 30 words |
| Build | Pass |
