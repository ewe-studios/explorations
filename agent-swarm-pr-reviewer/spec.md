---
source_location: /Users/darkvoid/Dev/agent-swarm-guide.txt (a single 226-line design guide, not a code repository)
repository: N/A (not a git repository)
explored_at: 2026-11-01
documentation_goal: Distill a seven-part design guide for an automated pull request (PR) reviewer built from several artificial intelligence (AI) agents into a clear, correct, complete technical exploration — the pipeline, risk scoring, the specialists, the verifying coordinator, fail-closed handling, security, the scope bug, the build stack, and interview preparation.
---

# Spec: agent-swarm-pr-reviewer Documentation

## 1. Source

| Property | Value |
|----------|-------|
| Location | `/Users/darkvoid/Dev/agent-swarm-guide.txt`, 226 lines |
| Kind | A prose design guide for a company's internal automated PR reviewer (interview study aid), not a code repository |
| Language | N/A (no source code ships with the guide) |
| Version | N/A |
| License | N/A |
| Size | 226 lines across 7 parts |
| Source of truth | `agent-swarm-guide.txt` itself, cited line by line as `agent-swarm-guide.txt:NN` |

The guide states openly that "you don't need the code to follow it" (`agent-swarm-guide.txt:3`). There is no code to read. The exploration therefore treats the text as the grandfather (the root of truth), and every factual claim — every number, name, and flow — traces to a line in that text. Where the documents show code, the snippets are marked **illustrative**: they reconstruct, in TypeScript, the shape of the design the guide describes. No illustrative snippet invents a fact the guide does not state.

## 2. What agent-swarm-pr-reviewer Is

agent-swarm-pr-reviewer is the distilled design of an automated pull request (PR) reviewer built from several artificial intelligence (AI) agents. When a person opens or updates a pull request on GitHub, the system fetches the diff, filters it, scores its risk, fans out to parallel specialist agents, fans in to one verifying coordinator, and posts a single GitHub review with line comments and a verdict of approve, comment, or block. The guide is a study of that system, written for interview preparation, and it is explicitly honest: it says to present the system as one "studied from a friend's company," not one built (`agent-swarm-guide.txt:222`).

## 3. Documentation Goal

1. Know the problem the system solves: artificial intelligence speeds up code writing, so review becomes the bottleneck.
2. Know the seven-step pipeline: fetch, filter, score risk, prepare files, fan out, fan in, publish.
3. Know the risk tiers and their exact thresholds: trivial up to 30 lines, lite up to 100 lines, full over 100 lines or sensitive paths.
4. Know the fan-out: the five specialists (code quality, documentation, quality assurance, security, performance) and when each runs.
5. Know the fan-in coordinator: merge, verify against the file and line, drop nitpicks, correct severity, rank, cap at 3/6/10, and give a verdict.
6. Know the nine design decisions, especially "code controls the flow" and "a failed review must never look like a pass."
7. Know the security posture: untrusted pull request content, prompt injection, read-only agents, and permission denial.
8. Know the scope bug: why a reviewer with nothing to do invents work, and the fix that stops it before it starts.
9. Know the build stack and build order, and the interview question-and-answer set.
10. Know the strengths, weaknesses, and what a reader can copy, and what a reader should leave.

## 4. Documentation Structure

```
agent-swarm-pr-reviewer/
├── spec.md
├── exploration.md
├── build.py
├── styles.css
├── markdown/   (README.md, 00 to 12)
└── html/
```

## 5. Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | markdown/README.md | DONE |
| 1 | 00-overview.md | DONE |
| 2 | 01-the-original-guide.md (verbatim source) | DONE |
| 3 | 02-the-pipeline.md | DONE |
| 4 | 03-risk-scoring.md | DONE |
| 5 | 04-the-specialists.md | DONE |
| 6 | 05-the-coordinator.md | DONE |
| 7 | 06-fail-closed-and-failure-handling.md | DONE |
| 8 | 07-security-and-prompt-injection.md | DONE |
| 9 | 08-the-scope-bug.md | DONE |
| 10 | 09-the-stack-and-build-order.md | DONE |
| 11 | 10-interview-prep.md | DONE |
| 12 | 11-critical-assessment.md | DONE |
| 13 | 12-glossary.md | DONE |
| 14 | exploration.md | DONE |
| 15 | Grandfather review | DONE |
| 16 | Generate HyperText Markup Language (HTML) | DONE |

## 6. Build System

Run `python3 -I build.py .` inside this directory. It writes `html/`. No dependencies beyond the Python standard library. This copy of `build.py` carries an `agent-swarm-pr-reviewer` entry in its `descriptions` dictionary, escapes raw angle brackets, and processes inline markup inside table cells.

## 7. Quality Requirements (Iron Rules)

1. Detailed sections with code snippets (illustrative reconstructions, clearly marked).
2. Teach key facts, principles and ideas quickly.
3. Clear articulation.
4. Mermaid diagrams (minimum 2 per document).
5. Good visual assets.
6. Generated HyperText Markup Language (HTML).
7. Cross-references.
8. Source path references (to `agent-swarm-guide.txt:NN`).
9. Aha moments.
10. Navigation: index plus previous/next buttons.

Also: ASD-STE100 Simplified Technical English. Abbreviations are spelled out where used. The text wins over the distillation.

## 8. Expected Outcome

A reader can explain the whole system end to end — the pipeline, the risk tiers, the specialist roles, the coordinator's verifying step, the fail-closed rules, the security posture, and the scope bug — and can defend each claim with a line in the guide. A reader preparing for an interview can answer every question in Part 7 and use the two honest tips.

## 9. Resume Point

Complete. All thirteen markdown documents plus `exploration.md` are written, the grandfather review (names, numbers, flows, coverage) is done and all discrepancies fixed, and `html/` is generated with `python3 -I build.py .`.
