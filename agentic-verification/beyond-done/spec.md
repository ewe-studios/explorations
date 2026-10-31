---
source_location: /Users/darkvoid/GitSources/src.Workos/beyond-done (commit 51a1e36698f5b53acd5deb4acc237b941c9a06ef)
repository: git@github.com:workos/beyond-done.git
explored_at: 2026-10-30
documentation_goal: Explain the "beyond done" idea, the principles behind it, and exactly how the workshop's proof lever, receipt, judge, and bug game work.
---

# Spec: Beyond Done Documentation

## 1. Source

| Property | Value |
|----------|-------|
| Location | /Users/darkvoid/GitSources/src.Workos/beyond-done, commit `51a1e36` |
| Language | TypeScript (Astro site, Slidev deck, React glossary, a small CDP command-line interface (CLI) in `lab/`) |
| Version | 0.1.0 (private) |
| License | Private repository (WorkOS). Built from `nicknisi/template-workshop` |
| Main files studied | `lab/snack.ts` (the "lever" CLI), `lab/proof.ts`, `lab/judge.ts`, `src/content/lessons/*.md`, `src/*.ts(x)` |

## 2. What Beyond Done Is

Beyond Done is a 60-minute AI-engineering workshop by Nick Nisi and Zack Proser (WorkOS). It teaches JavaScript and TypeScript developers to build a verification loop for an artificial intelligence (AI) coding agent. The one idea: an agent says "done"; give it a way to show the receipt. Attendees use a small command-line lever (`pnpm snack`) that drives one browser-local waitlist app over the Chrome DevTools Protocol (CDP), write one proof that defines "done", hand both to their own agent, and leave with a plan for the same lever in their own app.

## 3. Documentation Goal

1. Know the "beyond done" mental model: a completion claim, observed facts (`prove`), semantic judgment (`judge`), and the stop rule.
2. Know every command of the `pnpm snack` lever, its flags, exit codes, and the JSON it prints.
3. Know the proof and receipt: revision hash, receipt fields, and exit codes 0 (PASS), 1 (FAIL), 2 (can't tell).
4. Know the Jev judge: the `noul` question type, the two probabilities, and the 0.9 / 0.2 thresholds.
5. Know the five injected bugs and the "break it quietly" game, including the stale-receipt boss.
6. Know the seven lessons, the waitlist app, the glossary and chat, the Slidev deck, and the build/deploy scripts.
7. Know the strengths, weaknesses and risks of the approach, ranked.

## 4. Documentation Structure

```
agentic-verification/beyond-done/
├── exploration.md
├── spec.md
├── build.py
├── styles.css
├── markdown/   (README.md, 00 to 13)
└── html/
```

## 5. Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | markdown/README.md | DONE |
| 1 | 00-overview.md | DONE |
| 2 | 01-the-concept.md | DONE |
| 3 | 02-the-lever-snack-cli.md | DONE |
| 4 | 03-the-proof-and-receipt.md | DONE |
| 5 | 04-semantic-judgment-jev.md | DONE |
| 6 | 05-break-it-quietly.md | DONE |
| 7 | 06-lessons.md | DONE |
| 8 | 07-the-lab-app.md | DONE |
| 9 | 08-site-glossary-chat.md | DONE |
| 10 | 09-slides-and-deck.md | DONE |
| 11 | 10-scripts-build-deploy.md | DONE |
| 12 | 11-facilitation-docs.md | DONE |
| 13 | 12-critical-assessment.md | DONE |
| 14 | 13-glossary.md | DONE |
| 15 | exploration.md | DONE |
| 16 | Grandfather review | DONE |
| 17 | Generate HyperText Markup Language (HTML) | DONE |

## 6. Build System

Run `python3 -I build.py .` inside this directory. It writes `html/`. Run `python3 -I build.py` in the parent `agentic-verification` to build the collection index that links both projects. No dependencies beyond the Python standard library. This copy of `build.py` escapes raw angle brackets and processes inline markup inside table cells.

## 7. Quality Requirements (Iron Rules)

1. Detailed sections with code snippets.
2. Teach key facts, principles and ideas quickly.
3. Clear articulation.
4. Mermaid diagrams (minimum 2 per document).
5. Good visual assets.
6. Generated HyperText Markup Language (HTML).
7. Cross-references.
8. Source path references.
9. Aha moments.
10. Navigation: index plus prev/next buttons.

Also: ASD-STE100 Simplified Technical English. Abbreviations are spelled out where used. The code wins over the docs.

## 8. Expected Outcome

A reader can explain the "beyond done" concept, decide what to copy into their own verification loop, and find the code for any claim.

## 9. Resume Point

All documents are written. To continue: re-run the citation check on any changed chapter, then rebuild the HyperText Markup Language (HTML).
