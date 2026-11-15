---
title: tastecode package exploration
source_location: /Users/darkvoid/GitSources/src.aidesign/src.leonnix/tastecode
repository: Git checkout, local pin c216616
explored_at: 2026-11-15
version: 0.1.1
license: Apache License 2.0
files: 3519
tier: A
languages: TypeScript, JavaScript, Markdown, Cascading Style Sheets (CSS), HyperText Markup Language (HTML), Structured Query Language (SQL)
---

# Specification: tastecode package exploration

## 1. Source

| Property | Value |
|---|---|
| Location | `/Users/darkvoid/GitSources/src.aidesign/src.leonnix/tastecode` |
| Kind | Git checkout with its own history |
| Pin | `c216616` (short local commit identifier) |
| Tracked files | 3,519 |
| Version | `0.1.1` (`README.md:9`). `package.json` has no version field |
| Licence | Apache License 2.0 (`LICENSE`). `package.json` has no licence field |
| Package manager | pnpm `11.8.0` (`package.json:5`) |
| Node.js engine | `>=22.18.0` (`package.json:7`) |
| Module system | ECMAScript modules (`package.json:4`) |

TasteCode is a local desktop workspace for Artificial Intelligence (AI) coding
agents. It adds a Design Mode. The product has four clients or hosts: a desktop
application, a web client, a phone client, and a core server. The core server
owns state and orchestration. Every client is a thin renderer over a typed
WebSocket protocol.

The workspace has four applications and twelve packages. The tracked-file count
per workspace is:

| Path | Files | Role |
|---|---:|---|
| `apps/web` | 786 | Web client |
| `apps/server` | 254 | Core server, adapters, event log, checkpoints |
| `apps/desktop` | 140 | Electron desktop host |
| `apps/mobile` | 80 | Expo phone client |
| `packages/adapter-codex` | 672 | Codex app-server adapter |
| `packages/design-agent` | 548 | Design Mode phase pipeline |
| `packages/adapter-claude-code` | 51 | Claude Code adapter |
| `packages/adapter-opencode` | 36 | OpenCode adapter |
| `packages/adapter-grok` | 35 | Grok adapter |
| `packages/adapter-acp` | 29 | Agent Client Protocol adapter |
| `packages/adapter-cursor` | 28 | Cursor adapter |
| `packages/proc` | 27 | Child-process helpers |
| `packages/adapter-antigravity` | 22 | Antigravity adapter |
| `packages/contracts` | 17 | Shared typed contracts |
| `packages/default-skills` | 9 | Bundled skill pins and patches |
| `packages/adapter-pi` | 6 | Pi adapter |

The total above is 2,740 files. The remaining 779 files are top-level
configuration, documentation, plans, rules, tools, and patches.

## 2. Scope

This package covers TasteCode as a system. It covers these areas:

- The server as the owner of state, sessions, checkpoints, and the event log.
- The typed protocol and the client state machine.
- The adapter layer and provider capability negotiation.
- The Design Mode phase pipeline in `packages/design-agent`.
- The terminal, worktree, and browser-preview mechanisms.
- The default skills and their bundled pins.
- The release, licensing, and verification tooling.

It does not cover the hosted GitHub release binaries, the phone build pipeline,
or the visual quality of generated designs.

## 3. Documents

| File | Role | Words | Snippets |
|---|---|---:|---:|
| `markdown/README.md` | Index, thesis, map, reading order | no minimum | 3 or more |
| `markdown/00-overview.md` | Product, workspaces, vocabulary | 3,000 to 5,000 | 10 or more |
| `markdown/01-architecture.md` | Processes, protocol, state, storage | 3,000 to 5,000 | 10 or more |
| `markdown/02-core-mechanism.md` | Design Mode phase pipeline | 3,000 to 5,000 | 10 or more |
| `markdown/03-workflow-and-quality.md` | One design run, checkpoints, checks | 3,000 to 5,000 | 10 or more |
| `markdown/04-limits-and-comparison.md` | Limits, rejections, skill comparison | 3,000 to 5,000 | 10 or more |

Every deep dive has at least two Mermaid diagrams and at least one `**Aha:**`
insight. Every code block has a citation header with the relative source path
and the line range. A snippet is copied byte for byte.

An earlier draft exists at `markdown/00-overview.md`. It was written before the
tier plan. The writer may reuse its facts but must bring it to the rules above.

## 4. Constraints

- The source is read only. No file in the checkout is modified.
- No build, test, server, adapter, or desktop application is run.
- No network request is made to a provider service.
- Prose uses Aerospace and Defence Simplified Technical English, issue 100
  (ASD-STE100). One idea per sentence. Thirty words maximum. Expand every
  abbreviation at first use in each file.
- Prose avoids marketing language, em dashes, "simply", "just", and "obviously".
- A value that the source does not define is recorded as not defined.
- The pin is `c216616`. A claim about runtime behaviour is marked as not
  verified because no runtime was observed.

## 5. Research questions

1. Which process owns state, and how does a client recover after a gap?
2. How does one request become a model turn, a tool call, and a checkpoint?
3. How does the adapter layer hide provider differences without lying about
   capabilities?
4. How does Design Mode turn a brief into a sequence of phases with checks?
5. What does the project reject, and why?
6. Where does TasteCode agree with the `taste-skill` package, and where does it
   differ?

## 6. Source map for the writer

| Area | Start point |
|---|---|
| Product claim | `README.md:1` |
| Shape and protocol | `docs/ARCHITECTURE.md:9` |
| Design Mode | `docs/DESIGN-AGENT.md` |
| Providers | `docs/PROVIDERS.md` |
| Features | `docs/FEATURES.md` |
| Vision | `docs/VISION.md` |
| Phase pipeline | `packages/design-agent/src/workflow.ts` |
| Phase prompt steps | `packages/design-agent/src/*-phase.ts` |
| Typed protocol | `packages/contracts/src/protocol.ts` |
| Server entry | `apps/server/src/cli.ts` |
| Adapter registry | `apps/server/src/adapters.ts` |
| Checkpoints | `apps/server/src/checkpoint.ts` |
| Default skills | `packages/default-skills/skills.json` |
| Rules | `rules/` |
| Plans | `plans/README.md` |
| Verification | `docs/verification/` |

## 7. Task table

| Task | State |
|---|---|
| Verify pin, file count, licence, version | DONE |
| Map workspaces and packages | DONE |
| Write `spec.md` | DONE |
| Write `markdown/README.md` | DONE |
| Write `markdown/00-overview.md` | DONE |
| Write `markdown/01-architecture.md` | DONE |
| Write `markdown/02-core-mechanism.md` | DONE |
| Write `markdown/03-workflow-and-quality.md` | DONE |
| Write `markdown/04-limits-and-comparison.md` | DONE |
| Snippet check, sentence check, link check | DONE; results in section 10 |
| Fresh-reader review | DONE; gaps repaired |
| Repair reported gaps | DONE |
| Build HyperText Markup Language (HTML) | DONE |
| Commit and push | DONE |

## 8. Resume point

A later reader starts at `packages/design-agent/src/workflow.ts` for the phase
pipeline and at `apps/server/src/adapters.ts` for provider selection. The
protocol starts at `packages/contracts/src/protocol.ts`.

If the checkout moves from `c216616`, run the snippet checker again. A changed
file moves line ranges.

## 9. Limits

- Static reading only. No run was observed.
- The public beta is `0.1.1`. The README says to expect bugs and changes.
- The provider plan says `main` ships Codex, Claude Code, and Grok. Other
  adapters exist in the tree as nightly work. A reader must not assume that
  every adapter ships.
- The bundled default skills pin older versions than the container checkouts.
  The container has `taste-skill` 2.5.1 and `cinetic` at `f4bbfc8`. The bundled
  manifest pins `taste-skill` 2.5.0 and `cinetic` at `ecfa6605`.
- Visual quality is not evaluated. No design was generated.

## 10. Verification record

The snippet checker is `/tmp/i2t-fix.py`. It compares each fenced block with
the cited source range. The sentence checker is `/tmp/ste.py`.

| File | Words | Snippets | Bad | Mermaid | Aha |
|---|---:|---:|---:|---:|---:|
| `markdown/README.md` | 1,036 | 3 | 0 | 1 | 1 |
| `markdown/00-overview.md` | 3,282 | 13 | 0 | 2 | 1 |
| `markdown/01-architecture.md` | 3,345 | 14 | 0 | 2 | 1 |
| `markdown/02-core-mechanism.md` | 3,981 | 17 | 0 | 2 | 1 |
| `markdown/03-workflow-and-quality.md` | 3,745 | 16 | 0 | 2 | 1 |
| `markdown/04-limits-and-comparison.md` | 3,912 | 14 | 0 | 2 | 1 |
| **Total** | **19,301** | **77** | **0** | **11** | **6** |

The fresh-reader review found 22 classes of gap. The writer verified each
against the source and repaired the real ones. The writer rejected three
findings with evidence: the Repair phase reuses the running preview, a normal
start does reject a provider without image input, and the source scan limit is
32,000,000 bytes. One contradiction in `docs/DESIGN-AGENT.md:604-606` is
recorded as a document defect, because the start guard contradicts it.

The link check found one broken target in the whole `src.aidesign` tree, inside
the `open-design` package. It is a literal Application Programming Interface
(API) path inside a code block, not a page link.

Unverified: runtime behaviour of every kind. The state change that makes the
late image-skip path reachable is not stated. Whether a published installer was
built by the current packaging scripts is not stated. The intended effect of the
novelty field is not stated.
