---
source_location: /Users/darkvoid/GitSources/src.pr-agent/skills (commit 0cab8928751fe8344926e9fd6b8e468a6eae9873)
repository: git@github.com:The-PR-Agent/skills.git
explored_at: 2026-10-31
documentation_goal: Explain the pr-agent-maintainer skill: its ground rules, the verify-by-reverting method, the draft-and-adversarial-pass loop, and the twelve review steps.
---

# Spec: PR-Agent Skills Documentation

## 1. Source

| Property | Value |
|----------|-------|
| Location | /Users/darkvoid/GitSources/src.pr-agent/skills, commit `0cab892` |
| Language | Markdown (a single Claude Code and Cursor skill file) |
| Version | 1.0.0 |
| License | MIT |
| Author | The-PR-Agent community |
| Main file studied | `pr-agent-maintainer/SKILL.md` (the entire repository content) |

## 2. What the Skills Repository Is

`skills` is a companion repository to PR-Agent holding one skill, `pr-agent-maintainer`. The skill teaches an agent how to help a maintainer or reviewer triage, verify, reply to, and merge issues and pull requests on The-PR-Agent/pr-agent itself. It is a procedure, not a library: draft, verify by reverting, break the draft adversarially, then post only on the maintainer's word.

## 3. Documentation Goal

1. Know what a skill file is and what its frontmatter declares.
2. Know the ground rules: nothing goes out without the maintainer's word, and what approval actually covers.
3. Know the presentation format for drafts (table, action line, confidence from 0 to 10, delimiters, inline comments at `path:line`).
4. Know the verify-by-reverting method and why it beats reading.
5. Know all twelve review steps and the subagent sweep.
6. Know the project-direction rule (bug fixes until version 1, `do not merge yet`).

## 4. Documentation Structure

```
agentic-verification/pr-agent/skills/
├── exploration.md
├── spec.md
├── build.py
├── styles.css
├── markdown/   (README.md, 00 to 02)
└── html/
```

## 5. Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | markdown/README.md | DONE |
| 1 | 00-overview.md | DONE |
| 2 | 01-the-maintainer-skill.md | DONE |
| 3 | 02-glossary.md | DONE |
| 4 | exploration.md | DONE |
| 5 | Grandfather review | DONE |
| 6 | Generate HyperText Markup Language (HTML) | DONE |

## 6. Build System

Run `python3 -I build.py .` inside this directory. It writes `html/`. Run `python3 -I build.py` in the parent `pr-agent` group to build the group index that links `pr-agent` and `skills`. No dependencies beyond the Python standard library.

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

A reader can explain what the skill is for, run its verify-by-reverting check, present a draft in the required format, and know when to post versus wait for the maintainer.

## 9. Resume Point

All documents are written. To continue: re-run the citation check on any changed chapter, then rebuild the HyperText Markup Language (HTML).
