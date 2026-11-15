---
title: taste-skill package exploration
source_location: /Users/darkvoid/GitSources/src.aidesign/src.leonnix/taste-skill
repository: Git checkout, local pin f6b7657
explored_at: 2026-11-15
version: 2.5.1
license: MIT
files: 1663
documents: 6
languages: Markdown, JavaScript, HyperText Markup Language (HTML), Cascading Style Sheets (CSS), Shell
---

# Specification: taste-skill package exploration

## 1. Source

| Property | Value |
|---|---|
| Location | `/Users/darkvoid/GitSources/src.aidesign/src.leonnix/taste-skill` |
| Kind | Git checkout with its own history |
| Pin | `f6b7657` (short local commit identifier) |
| Tracked files | 1,663 |
| Version | `2.5.1` (`SKILL.md:3`, `CHANGELOG.md:7`) |
| Licence | MIT (`LICENSE`, `README.md:19`) |
| Skill root | `skills/taste-skill/` |
| Node.js requirement | 22 or later (`SKILL.md:28`) |
| Browser requirement | Chrome, Edge, or Chromium. ffmpeg is optional (`SKILL.md:28`) |

Taste Skill is an Agent Skill for web design. It is a text skill plus a kit of
scripts. The kit has a checker, a finish gate, a page starter, photograph
search, a copy linter, and eleven hero demonstrations. A Stop hook makes the
gate a condition for finishing a build.

The tracked files divide like this:

| Path | Files | Role |
|---|---:|---|
| `skills/taste-skill` | 1,631 | The skill: text, references, scripts, assets, library |
| `assets` | 13 | Repository wordmark, badges, sponsor images |
| `examples/v2` | 5 | Example outputs |
| `tests` | 3 | Skill tests: frontmatter, kit cleanliness, text encoding |
| `.github` | 2 | Funding file and workflow |
| Root files | 6 | `README.md`, `LICENSE`, `install.sh`, `CHANGELOG.md`, `.gitignore`, `.gitattributes` |
| Plugin manifests | 3 | `.claude-plugin`, `.codex-plugin`, `.agents` |

The total is 1,663 tracked files.

The skill folder is 39 MB on disk. The `assets` folder is 37 MB of that total.
The tracked files are the same files, so the sizes are measured on the checkout,
not estimated from the file count. The assets are large binary files: images,
fonts, and page models.

## 2. Scope

This package covers the skill as a system. It covers these areas:

- The `SKILL.md` contract: the nine house rules, the workflow, and the stop
  conditions.
- The reference library: 22 Markdown files that hold the design knowledge.
- The kit: `assets/starter.html`, the CSS system, the motion layer, and the
  eleven hero demonstrations.
- The scripts: `start.mjs`, `consistency.mjs`, `gate.mjs`, `shoot.mjs`,
  `copy-lint.mjs`, `photos.mjs`, `effects.mjs`, `icons.mjs`, `moments.mjs`,
  `perf.mjs`.
- The finish gate as a Stop hook, its rounds, its waivers, and its state files.
- The copy layer and its linter.
- The library data: `palettes.json` and `type.json`.
- The plugin manifests for Claude Code, Codex, and Antigravity.
- The three tests and the install script.

It does not evaluate the visual quality of a generated page. It does not run the
checker against a live page. It does not start a browser.

## 3. Documents

The package uses the standard six-document layout.

| File | Role | Words | Snippets |
|---|---|---:|---:|
| `markdown/README.md` | Index, thesis, map, reading order | no minimum | 3 or more |
| `markdown/00-overview.md` | The product, the skill contract, the kit | 3,000 to 5,000 | 10 or more |
| `markdown/01-architecture.md` | Skill layout, scripts, assets, install paths | 3,000 to 5,000 | 10 or more |
| `markdown/02-core-mechanism.md` | The checker and the finish gate | 3,000 to 5,000 | 10 or more |
| `markdown/03-workflow-and-quality.md` | One build from brief to `GATE: PASS` | 3,000 to 5,000 | 10 or more |
| `markdown/04-limits-and-comparison.md` | Limits, false positives, tastecode comparison | 3,000 to 5,000 | 10 or more |

Every deep dive has at least two Mermaid diagrams, at least one `**Aha:**`
insight, and a citation header on every code block. An earlier draft exists at
`markdown/00-overview.md`. The writer must verify every fact in it.

## 4. Constraints

- The source is read only. No skill file is modified.
- No build, checker, gate, browser, or script is run.
- No network request is made. The photograph search is not run.
- Prose uses Aerospace and Defence Simplified Technical English, issue 100
  (ASD-STE100). One idea per sentence. Thirty words maximum. Expand every
  abbreviation at first use in each file.
- Prose avoids marketing language, em dashes, "simply", "just", and "obviously".
- A value that the source does not define is recorded as not defined.
- The pin is `f6b7657`. Runtime behaviour is marked as not verified.

## 5. Research questions

1. What makes this skill different from a prompt that says "make it look good"?
2. How does a text skill enforce a rule that a model cannot ignore?
3. What does the checker measure, and what can it not measure?
4. How does the gate decide PASS, FAIL, OPEN, SETUP, ERROR, and OFF?
5. How does the skill avoid trapping the agent in a loop?
6. How does the copy linter separate a real fault from a false positive?
7. Where does this skill agree with `tastecode`, and where does it differ?

## 6. Source map for the writer

| Area | Start point |
|---|---|
| Skill contract | `skills/taste-skill/SKILL.md:1` |
| House rules | `skills/taste-skill/SKILL.md:16` |
| Gate reference | `skills/taste-skill/references/gate.md` |
| Checker | `skills/taste-skill/scripts/consistency.mjs` |
| Finish gate | `skills/taste-skill/scripts/gate.mjs` |
| Starter | `skills/taste-skill/scripts/start.mjs` |
| Page capture | `skills/taste-skill/scripts/shoot.mjs` |
| Copy linter | `skills/taste-skill/scripts/copy-lint.mjs` |
| Photograph search | `skills/taste-skill/scripts/photos.mjs` |
| Layout check | `skills/taste-skill/assets/layout/check.mjs` |
| Motion layer | `skills/taste-skill/assets/motion/` |
| Page types | `skills/taste-skill/assets/pages/` |
| Palette data | `skills/taste-skill/library/palettes.json` |
| Type data | `skills/taste-skill/library/type.json` |
| Plugin manifest | `.claude-plugin/marketplace.json` |
| Codex manifest | `.codex-plugin/plugin.json` |
| Changelog | `CHANGELOG.md:7` |
| Tests | `tests/frontmatter.mjs`, `tests/kit-clean.mjs`, `tests/text-encoding.mjs` |
| Install | `install.sh` |

## 7. Task table

| Task | State |
|---|---|
| Verify pin, file count, licence, version | DONE |
| Map skill folders and scripts | DONE |
| Write `spec.md` | DONE |
| Write the six documents | DONE |
| Snippet check, sentence check, link check | DONE; results in section 10 |
| Fresh-reader review | DONE; gaps repaired |
| Repair reported gaps | DONE |
| Build HyperText Markup Language (HTML) | DONE |
| Commit and push | DONE |

## 8. Resume point

A later reader starts at `skills/taste-skill/scripts/gate.mjs` for the finish
gate and at `skills/taste-skill/scripts/consistency.mjs` for the checker. The
gate's written contract is `skills/taste-skill/references/gate.md`.

If the checkout moves from `f6b7657`, run the snippet checker again.

## 9. Limits

- Static reading only. No checker, gate, browser, or build was run.
- The checker's false-positive rate is not measured here. The changelog states
  that six false positives were fixed in 2.5.1.
- A `GATE: PASS` result is not evaluated. It is a claim by the source.
- The skill's visual results are not evaluated.
- The gate depends on Claude Code, Codex, or Antigravity hooks. The hook
  behaviour of those products is not tested here.
- The 39 MB skill folder and the 37 MB asset folder are measured on disk. A
  file count is a different measure. A reader must not compare them as the same.

## 10. Verification record

The snippet checker is `/tmp/i2t-fix.py`. The sentence checker is `/tmp/ste.py`.

| File | Words | Snippets | Bad | Mermaid | Aha |
|---|---:|---:|---:|---:|---:|
| `markdown/README.md` | 1,002 | 4 | 0 | 1 | 0 |
| `markdown/00-overview.md` | 3,694 | 10 | 0 | 2 | 2 |
| `markdown/01-architecture.md` | 3,900 | 15 | 0 | 3 | 2 |
| `markdown/02-core-mechanism.md` | 4,707 | 13 | 0 | 2 | 1 |
| `markdown/03-workflow-and-quality.md` | 4,996 | 14 | 0 | 2 | 1 |
| `markdown/04-limits-and-comparison.md` | 4,988 | 10 | 0 | 3 | 3 |
| **Total** | **23,287** | **66** | **0** | **13** | **9** |

The fresh-reader review found 30 classes of gap. The writer verified each
against the source and repaired the real ones. The writer rejected one term,
"plate", because the source has no such field. The writer corrected two line
citations from the coordinator's brief after it checked the source.

The review found these source defects:

- The `SKILL.md` version is inside the description at line 4. There is no
  `version:` field. A citation to line 3 is wrong.
- `gate.mjs` has eight verdict strings, not six or seven.
- `gate.mjs` never runs `perf.mjs`. Performance is a separate report.
- The 10-minute host hook limit exists only in a code comment at
  `gate.mjs:114`. It is not a measured limit.
- `ORDER` is not the complete fault set. Module checks are appended after the
  three splice calls (`consistency.mjs:10196`).

Unverified: host Stop hook behaviour, installer results, continuous integration
results, the checker false-positive rate, and the visual quality of a passing
page. No script, checker, gate, browser, test, or install was run.
