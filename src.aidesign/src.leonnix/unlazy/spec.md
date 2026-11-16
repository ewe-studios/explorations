---
title: unlazy package exploration
source_location: /Users/darkvoid/GitSources/src.aidesign/src.leonnix/unlazy
repository: Git checkout, local pin 1667149
explored_at: 2026-11-16
version: 2.1.0 (unreleased source target)
license: MIT
files: 37
documents: 6
languages: JavaScript (ECMAScript modules), Markdown, YAML Ain't Markup Language (YAML), JavaScript Object Notation (JSON), HyperText Markup Language (HTML)
---

# Specification: unlazy package exploration

## 1. Source

| Property | Value |
|---|---|
| Location | `/Users/darkvoid/GitSources/src.aidesign/src.leonnix/unlazy` |
| Kind | Git checkout |
| Pin | `1667149` (short local commit identifier) |
| Full pin | `16671491f6679ad9378f52604d3bc2415b4120c7` |
| Commit date | 2026-09-03 17:31:08 +0800 |
| Commit subject | `fix: bind gate evidence and harden Windows identity` |
| Tracked files | 37, excluding `.git` and `node_modules` |
| Total tracked lines | 9,181 |
| Licence | MIT (`LICENSE`) |
| Stated version | `2.1.0`, described as the unreleased current source target (`README.md:15`, `package.json:3`) |
| Runtime | Node.js 16 or newer, no third-party runtime packages (`package.json:11`, `scripts/lib/gates.mjs:2`) |

`unlazy` is an agent skill. It changes what an autonomous agent must do before it may
report that substantial work is complete. The agent writes a gate ledger first. Each gate
states one observable outcome. A runnable gate carries a `CHECK:` command and an `EXPECT:`
string. A checker runs the command and records evidence. A completed gate counts as met
only when the process exits zero, the expectation matches, and the stored evidence binds
the current gate definition. The optional Stop hook blocks the agent from ending a coding
session while the resolved ledger still has unmet gates or unfinished dispatch waves.

## 2. Scope

The exploration reads the source at the pinned commit. It explains the instruction layer,
the strict ledger format, the checker and its evidence model, the approval boundary, the
shared file-reading and file-writing primitives, the lease and dispatch coordination, the
Stop hook and installer, the quality linter, the templates, the test suite, and the
continuous integration matrix.

It does not run the checker, the hook, the installer, the linter or the tests. It does not
approve or execute any `CHECK:` command. It does not judge whether the method improves
agent output. Where the source states a value, the exploration quotes the value and the
line. Where the source does not state a value, the exploration says so.

## 3. Document set

| File | Subject |
|---|---|
| `markdown/README.md` | Thesis, map, document table, reading order, non-claims |
| `markdown/00-overview.md` | The problem, the three modes, one gate from write to report |
| `markdown/01-architecture.md` | Instruction layer, module graph, data files, trust boundaries |
| `markdown/02-core-mechanism.md` | Parser, digest, evidence, approval, execution, evidence writeback |
| `markdown/03-workflow-and-quality.md` | Author, lint, approve, run, reverify, orchestrate, dispatch, report |
| `markdown/04-limits-and-comparison.md` | Limits, costs, threat model, research posture, sibling comparison |

Every deep dive has 3,000 to 5,000 words, at least ten byte-exact snippets, at least two
Mermaid diagrams and at least one **Aha:** insight. Every snippet has a `path:line` header
and quotes the source byte for byte. Every document expands each abbreviation at first use.

## 4. Inventory

| Path | Files | Lines | Role |
|---|---:|---:|---|
| `SKILL.md` | 1 | 104 | Core instructions and mode routing |
| `references/` | 6 | 568 | Gate format, method, orchestration, dispatch, parallel, token economy |
| `templates/` | 3 | 218 | Plan, leaf ledger and branch ledger templates |
| `agents/` | 1 | 6 | Skill interface metadata for one host |
| `scripts/` | 5 | 1,764 | Checker, linter, dispatch recorder, installer, Stop hook |
| `scripts/lib/` | 5 | 1,462 | Shared parser and state, dispatch state, supervisor, process tree, regex worker |
| `tests/` | 7 | 4,395 | 173 behavioural tests and 21 structural self-checks |
| `README.md` | 1 | 244 | Install, quick start, contract, security, research |
| `SECURITY.md` | 1 | 72 | Command, approval, hook and lease threat model |
| `CONTRIBUTING.md` | 1 | 66 | Ground rules and required regression coverage |
| `CHANGELOG.md` | 1 | 114 | Unreleased 2.1.0 change set, 2.0.0 and 1.0.0 milestones |
| `research/` | 1 | 81 | Validation protocol and historical limitations |
| `.github/workflows/` | 1 | 50 | Three operating systems by three Node versions, plus one pinned lane |
| Root | 3 | 41 | `package.json`, `LICENSE`, `.gitignore` |

The largest source files are `scripts/gate-check.mjs` (960 lines), `scripts/lib/gates.mjs`
(953), `scripts/lib/dispatch.mjs` (293), `scripts/gate-lint.mjs` (245), `scripts/stop-hook.mjs`
(211) and `scripts/install-hooks.mjs` (205). The largest test file is
`tests/hardening-tests.mjs` (1,423 lines, 51 tests).

## 5. Constants

| Constant | Value | Line |
|---|---|---|
| Node floor | 16 or newer | `package.json:11` |
| Captured output ceiling | 1,048,576 bytes, on raw and canonical forms | `scripts/lib/gates.mjs:14` |
| Automatic evidence cap | 900 characters | `scripts/lib/gates.mjs:15` |
| Stable-file default ceiling | 8,388,608 bytes | `scripts/lib/gates.mjs:22` |
| Lease record ceiling | 65,536 bytes | `scripts/lib/gates.mjs:23` |
| Gate ledger ceiling | 8,388,608 bytes | `scripts/gate-check.mjs:68` |
| Approval record ceiling | 262,144 bytes | `scripts/gate-check.mjs:67` |
| Default check timeout | 120 seconds | `scripts/gate-check.mjs:72` |
| Check timeout range | 1 to 86,400 seconds | `scripts/gate-check.mjs:134` |
| Job range | 1 to 64, default 1 | `scripts/gate-check.mjs:140-147` |
| Regular-expression match budget | 250 milliseconds after worker start | `scripts/gate-check.mjs:69` |
| Regular-expression startup budget | 5,000 milliseconds | `scripts/gate-check.mjs:70` |
| Regular-expression worker limit | 4 | `scripts/gate-check.mjs:71` |
| Definition digest schema | `unlazy.gate-definition`, version 1 | `scripts/lib/gates.mjs:448-454` |
| Evidence prefix | `automatic-evidence=v1` | `scripts/lib/gates.mjs:461` |
| Scope identifier shape | `[A-Za-z0-9][A-Za-z0-9._-]{0,63}` | `scripts/lib/gates.mjs:247` |
| Stop-hook release | 6 consecutive blocks without semantic progress | `scripts/stop-hook.mjs:12` |
| Stop-hook handler timeout | 20 seconds | `scripts/install-hooks.mjs:198` |
| Default lease lock timeout | 30,000 milliseconds | `scripts/lib/gates.mjs:767` |
| Windows cleanup helper timeout | 1,000 milliseconds | `scripts/lib/process-tree.mjs:7` |
| Dispatch handle limit | 256 characters | `scripts/lib/dispatch.mjs:31` |
| Dispatch abandonment reason limit | 500 characters | `scripts/lib/dispatch.mjs:40` |
| Dispatch state schema and ceiling | 1, 8,388,608 bytes | `scripts/lib/dispatch.mjs:8-9` |
| Continuous integration matrix | 3 operating systems by Node 16, 20, 24 | `.github/workflows/test.yml:17-19` |

## 6. Research questions

| # | Question |
|---|---|
| 1 | What exact state turns an unchecked gate into a met gate, and what can demote it again? |
| 2 | How does the checker keep an inherited `CHECK:` line from executing before a human approves it? |
| 3 | How does the parser make malformed ledgers fail closed instead of becoming a completion certificate? |
| 4 | How do scope resolution, ownership leases and dispatch waves share one completion decision? |
| 5 | What does automatic evidence prove, and what does it deliberately not prove? |
| 6 | How do the Windows file-identity and process-tree mechanisms keep the same guarantees as the Unix paths? |
| 7 | What does the project state about its own limits, research and historical claims? |

## 7. Constraints

- Aerospace and Defence Simplified Technical English (ASD-STE100) applies to every sentence.
- The source is read-only. No file in the source tree is changed.
- No network access. No dependency is installed. No checker run, hook run or test run occurs.
- One package is in flight. Two subagents work on this package: one writer and one fresh reader.
- A fresh-reader review runs before the package is done. The writer fixes every reported gap that
  the source supports.

## 8. Task table

| # | Task | State |
|---|---|---|
| 1 | Write `spec.md` | DONE |
| 2 | Write the six documents | DONE (6 documents, 21,003 prose words) |
| 3 | Validate snippets against the source | DONE (64 blocks, 0 mismatches) |
| 4 | Validate Simplified Technical English | DONE (0 long sentences in 6 files) |
| 5 | Validate links | DONE (21 links, 0 broken) |
| 6 | Fresh-reader review and repair | DONE (10 findings, all repaired) |
| 7 | Build HyperText Markup Language (HTML) | DONE (8 pages and one stylesheet) |
| 8 | Update the container index and spec | DONE (18 of 19 in `index.md`, `spec.md` and the container README) |
| 9 | Commit and push | IN PROGRESS (commit pending) |

## 9. Resume point

Read the task table. The first `TODO` row is the next action. A task is complete only when its
check passes and the result is recorded in the verification table.

## 10. Verification record

| Check | Result |
|---|---|
| Documents | 6 files, 21,003 prose words: README 2,000; 00-overview 4,072; 01-architecture 3,552; 02-core-mechanism 3,761; 03-workflow-and-quality 3,967; 04-limits-and-comparison 3,651 |
| Snippets | 64 blocks (README 4; 00 14; 01 12; 02 11; 03 12; 04 11), 0 mismatches against the pinned source |
| Diagrams and insights | 12 Mermaid diagrams (2 per file), 6 **Aha:** insights (1 per file) |
| Simplified Technical English | 0 long sentences in 6 files |
| Links | 21 checked, 0 broken |
| Fresh-reader review | A separate reader agent with no writing context and no source access reported 10 gaps, including a stray diagram edge and a four-state versus five-state count mismatch. All 10 were repaired |
| Build | DONE: 8 HTML pages and one stylesheet in `html/` |
| Commit | Pending |
