---
title: src.leonnix container exploration
source_location: /Users/darkvoid/GitSources/src.aidesign/src.leonnix
repository: "none; a container of nineteen Git checkouts"
explored_at: 2026-11-15
subprojects: 19
license: mixed, recorded per subproject
languages:
  - TypeScript
  - JavaScript
  - Python
  - Graphics Library Shading Language (GLSL)
  - HyperText Markup Language (HTML)
  - Cascading Style Sheets (CSS)
  - Shell
  - Markdown
---

# Specification: the src.leonnix container

## 1. Source

The source is `/Users/darkvoid/Dev/explorations/../GitSources/src.aidesign/src.leonnix`,
which resolves to `/Users/darkvoid/GitSources/src.aidesign/src.leonnix`.

This path is not one package. It is a container of nineteen independent Git
checkouts. The container has no `README.md`, no package manifest, and no Git
repository of its own. Each child directory has its own `.git` directory and its
own history.

All nineteen checkouts were clean on 2026-11-15. I ran
`git rev-parse --short=7 HEAD` and `git ls-files | wc -l` in each child
directory. Every pin and every count matched the inventory that the task
supplied. The total is 8,704 tracked files.

| Project | Pin | Files | Purpose | Licence |
|---|---|---:|---|---|
| afterglow | ced1038 | 351 | Offline three.js piano film | MIT for code; music and film excluded |
| athenaion | 9b8bd23 | 220 | Procedural ancient Greek world | MIT |
| casa-ocaso | fcc3379 | 193 | Real-time WebGPU house scene | MIT |
| cinetic | f4bbfc8 | 964 | Agent skill for motion films | MIT |
| cook | b23cac6 | 12 | Agent skill for long visual iteration | MIT |
| earshot | 4ac3d2a | 287 | Audio analysis tool for agents | MIT |
| feed-backrooms | 81cdb6a | 94 | Backrooms horror game | Not stated |
| first-bloom | 713525b | 52 | Painterly three.js film | Not stated; manifest says ISC |
| golden-temple | 97fd9ec | 121 | three.js temple film | MIT |
| halo-above-the-clouds | 1d08819 | 55 | Real-time scene and drone film | MIT |
| needle-drop | 8982f09 | 100 | Blender Python film | Not stated |
| procedural-alpine-lake | b84c254 | 79 | Python and GLSL lake film | Not stated |
| room-at-dawn | aab8be2 | 48 | Photoreal bedroom scene and film | MIT |
| shanshui | b5357c0 | 146 | Ink-wash landscape film | MIT |
| taste-skill | f6b7657 | 1,663 | Agent skill for web design | MIT |
| tastecode | c216616 | 3,519 | Local design and coding tool | Apache-2.0 |
| unlazy | 1667149 | 37 | Agent skill for completion gates | MIT |
| unsung-vale | 9f61b0d | 239 | Browser role-playing game | MIT |
| valle-morta | 64471ca | 524 | Browser shooter | MIT for code; audio third-party |

The projects have no shared build command. Each project has its own scripts.
Some projects have no package manifest at all. A pin is a local commit
identifier. It is not a release tag and not a proof of upstream state.

## 2. Scope

This exploration gives **every one of the nineteen projects its own package**.
Each package lives in its own subdirectory of this exploration, has its own
`spec.md`, its own `markdown/` tree, and its own generated HyperText Markup
Language (HTML) pages.

The container level supplies only two items:

- `markdown/README.md`, the index. It gives the thesis, the four groups, the
  nineteen-row table, the reading order, and the shared non-claims.
- This file, the specification. It gives the pins, the document plan, the order
  of work, and the verification rules.

Every project receives the same six-document exploration. Section 3 gives the
fixed layout.

The exploration does not run a project. It does not build, render, test, or
launch one. It does not evaluate visual quality. Section 8 states the limits.

## 3. Depth: every project is detailed

Depth does not follow size. Every one of the nineteen projects receives the same
full exploration: six documents, and a deep dive for each part of the machine.
A twelve-file skill still gets the overview, the architecture, the core
mechanism, the workflow and quality, and the limits. The difference between a
large project and a small one is how many code paths those documents cover, not
whether the documents exist.

| Level | Documents | Words per deep dive | Snippets per deep dive |
|---|---|---:|---:|
| Every package | README plus five deep dives (6) | 3,000 to 5,000 | 10 or more |

The file names are fixed. A writer must not invent a new layout.

| File | Role |
|---|---|
| `markdown/README.md` | Index, thesis, Mermaid map, document list, reading order, non-claims |
| `markdown/00-overview.md` | What the project is, who uses it, and what it ships |
| `markdown/01-architecture.md` | Processes, data flow, state ownership, and storage |
| `markdown/02-core-mechanism.md` | The one mechanism that carries the central idea, traced end to end |
| `markdown/03-workflow-and-quality.md` | One real workflow from start to finish, and every check in it |
| `markdown/04-limits-and-comparison.md` | Limits, rejected choices, failure modes, and comparisons |

Every deep dive has at least two Mermaid diagrams, at least one `**Aha:**`
insight, and a citation header on every code block. The header gives the source
path relative to the subproject root and the line range. The README has the
thesis, one Mermaid map of the package, the document list, and the non-claims.

For a small project, the five deep dives divide the small surface completely.
For a large project, the same five documents divide a large surface. No document
is a summary of another document.

## 4. Order of work

| # | Subproject | Group | Files | Documents |
|---:|---|---|---:|---:|
| 1 | tastecode | Tooling | 3,519 | 6 |
| 2 | taste-skill | Agent skills | 1,663 | 6 |
| 3 | cinetic | Agent skills | 964 | 6 |
| 4 | valle-morta | Real-time worlds | 524 | 6 |
| 5 | afterglow | Offline films | 351 | 6 |
| 6 | earshot | Tooling | 287 | 6 |
| 7 | unsung-vale | Real-time worlds | 239 | 6 |
| 8 | athenaion | Real-time worlds | 220 | 6 |
| 9 | casa-ocaso | Real-time worlds | 193 | 6 |
| 10 | shanshui | Offline films | 146 | 6 |
| 11 | golden-temple | Offline films | 121 | 6 |
| 12 | needle-drop | Offline films | 100 | 6 |
| 13 | feed-backrooms | Real-time worlds | 94 | 6 |
| 14 | procedural-alpine-lake | Offline films | 79 | 6 |
| 15 | halo-above-the-clouds | Films and worlds | 55 | 6 |
| 16 | first-bloom | Offline films | 52 | 6 |
| 17 | room-at-dawn | Films and worlds | 48 | 6 |
| 18 | unlazy | Agent skills | 37 | 6 |
| 19 | cook | Agent skills | 12 | 6 |

The order is the two poles first, then the rest from largest to smallest. The
poles carry the design argument of the whole collection. The small projects
reuse the vocabulary that the poles establish.

## 5. Per-package workflow

A package is complete only when all seven steps are done.

1. Read the project: manifest, entry points, scripts, tests, and documents.
2. Write `spec.md` in the package directory. It gives the pin, the file count,
   the licence, the source map, the research questions, and the task table.
3. Spawn the writer agent for that one package. The brief names the six-file
   layout, the word and snippet minimums, the citation rule, and the source
   path. The writer is the only agent that writes prose.
4. Validate mechanically: byte-exact snippet check, sentence length check,
   relative link check, and Mermaid and `Aha:` count.
5. Spawn the fresh-reader agent. It has no memory of the writing. It reads the
   package documents with no tool use and no web access. It reports every place
   where it loses the thread or finds the depth too thin.
6. Repair every reported gap. Where the source cannot settle a finding, record
   it as unexplained instead of guessing.
7. Build the HTML, commit with an `ADD:` prefix and no Artificial Intelligence
   (AI) attribution, and push. Update the container README row from "Planned" to
   a live link.

Only one package is in flight at a time. A new package does not start until the
current one is committed and pushed.

## 6. Constraints

- The source is read only. No project file is modified.
- No project build, test, render, or server run is executed.
- Code snippets are copied byte for byte from the source, with the cited line
  range. A reformatted snippet is a defect.
- Prose uses Aerospace and Defence Simplified Technical English, issue 100
  (ASD-STE100). One idea per sentence. Thirty words maximum. Every abbreviation
  is expanded at first use in each file.
- Prose avoids marketing language, em dashes, "simply", "just", and "obviously".
- A document states when its source does not define a value. It does not guess.
- Each package repeats the non-claims in its own README, because a reader can
  open one package without the container index.

## 7. Research questions

The container answers one question: what connects nineteen projects that look
unrelated? Each package answers the same five questions in its own terms.

| # | Question |
|---|---|
| 1 | What does the project make, and for which user? |
| 2 | How does a run start, and which process owns state? |
| 3 | Where does determinism come from: seed, fixed step, or a measured gate? |
| 4 | What does the project check before it trusts a model report? |
| 5 | What does the project say it cannot do? |

The two poles add one question each. For tastecode: how does a local server turn
design judgement into phases, checks, and an append-only event log? For
taste-skill: how does a Stop hook turn a browser measurement into a condition
for finishing?

## 8. Limits of the container study

- No runtime behaviour was observed. Every runtime claim comes from source text.
- A licence field is recorded as the files state it. Four projects state no
  licence. Two projects have a split licence. Section 10 records both cases.
- A short commit identifier can collide or move upstream. A later reader must
  record a new pin.
- File counts can include generated, vendored, or fixture files.
- The container index does not compare visual quality. Quality claims need
  output, and no output was produced.
- The bundled copies of skills inside tastecode are older than the container
  checkouts. Section 10 records the versions.

## 9. Task table

| # | Subproject | State |
|---:|---|---|
| 1 | tastecode | DONE (6 documents, 77 snippets, fresh-reader review repaired) |
| 2 | taste-skill | DONE (6 documents, 66 snippets, fresh-reader review repaired) |
| 3 | cinetic | DONE (6 documents, 63 snippets, fresh-reader review repaired) |
| 4 | valle-morta | DONE (6 documents, 76 snippets, fresh-reader review repaired) |
| 5 | afterglow | DONE (6 documents, 69 snippets, fresh-reader review repaired) |
| 6 | earshot | DONE (6 documents, 66 snippets, fresh-reader review repaired) |
| 7 | unsung-vale | DONE (6 documents, 90 snippets, fresh-reader review repaired) |
| 8 | athenaion | DONE (6 documents, 78 snippets, fresh-reader review repaired) |
| 9 | casa-ocaso | DONE (6 documents, 65 snippets, fresh-reader review repaired) |
| 10 | shanshui | DONE (6 documents, 59 snippets, fresh-reader review repaired) |
| 11 | golden-temple | DONE (6 documents, 71 snippets, fresh-reader review repaired) |
| 12 | needle-drop | DONE (6 documents, 108 snippets, fresh-reader review repaired) |
| 13 | feed-backrooms | DONE (6 documents, 84 snippets, fresh-reader review repaired) |
| 14 | procedural-alpine-lake | DONE (6 documents, 69 snippets, fresh-reader review repaired) |
| 15 | halo-above-the-clouds | DONE (6 documents, 74 snippets, fresh-reader review repaired) |
| 16 | first-bloom | DONE (6 documents, 88 snippets, fresh-reader review repaired) |
| 17 | room-at-dawn | DONE (6 documents, 80 snippets, fresh-reader review repaired) |
| 18 | unlazy | TODO |
| 19 | cook | TODO |

Container index: written. Container specification: this file.

## 10. Corrections and limits found during research

The inventory was correct at the time of reading. All nineteen pins and all
nineteen file counts matched. The following findings correct or qualify other
assumptions.

1. **The container is not a package.** There is no container manifest, README, or
   Git repository. The word "package" in the task means a group of checkouts.
2. **The licence field is unreliable in two projects.** `first-bloom` has no
   licence file and its manifest says ISC. Its README names licences for assets.
   `valle-morta` has an MIT licence file, but its manifest says ISC. The licence
   file and the README agree, so the manifest field is stale.
3. **Four projects state no licence.** `feed-backrooms`, `first-bloom`,
   `needle-drop`, and `procedural-alpine-lake` have no root licence file. The
   exploration records them as not stated. It does not infer a licence.
4. **afterglow has a split licence.** The MIT licence covers the code. It does
   not cover the musical composition, the film, the audio, or the stills. A
   reader must not treat the whole repository as MIT.
5. **tastecode bundles older skill copies than the container holds.** The
   default-skill manifest pins taste-skill at version 2.5.0 and cinetic at
   commit `ecfa6605`. The container has taste-skill 2.5.1 and cinetic at
   `f4bbfc8`. The cook pin is `b23cac6`, which equals the container checkout.
6. **tastecode `main` ships three providers.** The provider plan states that
   `main` ships Codex, Claude Code, and Grok only. Other adapters exist in the
   source tree and are described as nightly work. A reader must not assume that
   every adapter in the tree ships on `main`.
7. **No runtime claim is verified.** I did not run a build, a test, a render, or
   a server. Statements about output quality come from project documentation and
   from source structure, not from observed runs.
8. **Some projects have no package manifest.** `needle-drop` is a Blender Python
   project. `cook` and `unlazy` are skill folders. Their file counts are small
   because they contain text and scripts, not application code.
9. **Two projects belong to two groups.** `halo-above-the-clouds` and
   `room-at-dawn` each contain a real-time scene and an offline film. The index
   marks both groups for these projects.
10. **The earlier plan was too flat.** A first draft gave the whole container six
    documents and treated the container as one package. That draft did not cover
    the seventeen smaller projects in detail. The current plan gives each
    project its own package, as the task requires.

## 11. Verification record

The record fills as each package lands. A row is complete only after the build
and the push.

| # | Subproject | Documents | Snippets | Bad | Words | Mermaid | Aha | Links | Fresh reader |
|---:|---|---:|---:|---:|---:|---:|---:|---|---|
| 1 | tastecode | 6 | 77 | 0 | 19,331 | 11 | 6 | 0 broken | Done |
| 2 | taste-skill | 6 | 66 | 0 | 23,287 | 13 | 9 | 0 broken | Done |
| 3 | cinetic | 6 | 63 | 0 | 27,165 | 11 | 6 | 0 broken | Done |
| 4 | valle-morta | 6 | 76 | 0 | 27,226 | 11 | 6 | 0 broken | Done |
| 5 | afterglow | 6 | 69 | 0 | 28,271 | 11 | 7 | 0 broken | Done |
| 6 | earshot | 6 | 66 | 0 | 27,313 | 12 | 7 | 0 broken | Done |
| 7 | unsung-vale | 6 | 90 | 0 | 28,705 | 13 | 6 | 0 broken | Done |
| 8 | athenaion | 6 | 78 | 0 | 24,075 | 11 | 5 | 0 broken | Done |
| 9 | casa-ocaso | 6 | 65 | 0 | 27,652 | 11 | 10 | 0 broken | Done |
| 10 | shanshui | 6 | 59 | 0 | 27,986 | 12 | 5 | 0 broken | Done |
| 11 | golden-temple | 6 | 71 | 0 | 29,624 | 11 | 7 | 0 broken | Done |
| 12 | needle-drop | 6 | 108 | 0 | 24,122 | 11 | 7 | 0 broken | Done |
| 13 | feed-backrooms | 6 | 84 | 0 | 25,152 | 12 | 7 | 0 broken | Done |
| 14 | procedural-alpine-lake | 6 | 69 | 0 | 26,546 | 12 | 9 | 0 broken | Done |
| 15 | halo-above-the-clouds | 6 | 74 | 0 | 21,430 | 12 | 9 | 0 broken | Done |
| 16 | first-bloom | 6 | 88 | 0 | 19,873 | 11 | 9 | 0 broken | Done |
| 17 | room-at-dawn | 6 | 80 | 0 | 20,314 | 11 | 6 | 0 broken | Done |
| 18 | unlazy | pending | | | | | | | |
| 19 | cook | pending | | | | | | | |

Container checks:

| Check | Result |
|---|---|
| Container index `markdown/README.md` | Written |
| Container specification | This file |
| Nineteen subproject directories with `markdown/` | Present |
| Nested-project build discovery | Done: recursive discovery and relative root link |
| HTML pages | Rebuilt after each package |
