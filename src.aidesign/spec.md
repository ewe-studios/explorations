---
source_location: /Users/darkvoid/GitSources/src.aidesign
repository: none (a container of separate projects)
explored_at: 2026-11-15
packages: 34
license: mixed, recorded per package
languages: TypeScript, JavaScript, Python, Markdown, Cascading Style Sheets (CSS), HyperText Markup Language (HTML)
---

# Spec: the src.aidesign package set

## 1. Source

| Property | Value |
|----------|-------|
| Snapshot location | `/Users/darkvoid/GitSources/src.aidesign` |
| Container type | A plain directory. It is not a Git repository |
| Sub-packages | 34 projects. 28 are Git checkouts. Six are plain directories |
| Snapshot date | 2026-11-15 |
| Remote | None at the container level. Each Git checkout keeps its own remote |
| Source of truth | Each project's own tree at the commit or date recorded in its `spec.md` |

Two entries are Zip archives, `colorfigures.zip` and `iso-glow.zip`. They are copies of the
plain directories beside them. The exploration reads the directories.

The count is 34 projects. An earlier working note said 37. That figure counted the two dot
entries, `.DS_Store`, and the two Zip files. This spec uses the corrected count.

## 2. Scope

This exploration covers every project in the container. Each project explains what it is, how it
is built, the mechanism that carries its idea, and the limits that its own files state.

It does not run the projects, render their output, or publish their packages. It reads the source
and the documents at the pinned revision.

## 3. Depth: every package is detailed

Every package receives the full exploration. Depth does not follow size. A nine-file skill still
gets its architecture, its mechanism, its workflow, and its limits, because a reader who opens
that package must be able to learn how it works from the package alone.

| Level | Documents per package | Words per deep dive | Snippets per deep dive |
|-------|-----------------------|--------------------|------------------------|
| Every package | 6 (README plus five deep dives) | 3,000 to 5,000 | 10 or more |

The five deep dives are always `00-overview`, `01-architecture`, `02-core-mechanism`,
`03-workflow-and-quality`, and `04-limits-and-comparison`. A small project divides a small
surface into those five parts. It does not receive a shorter list.

Every deep dive has at least two Mermaid diagrams, at least one **Aha:** insight, and ten or more
byte-exact snippets with a `path:line` citation header. The README has the thesis, one Mermaid
map, the document table, the reading order, and the non-claims. Each document expands every
abbreviation at first use.

The size bands in section 4 now set only the order of work. They do not set depth.

## 4. Inventory and order

The work runs in this order, one package at a time. A package is complete, committed, and pushed
before the next one starts. The size bands are A = 1,000 files or more, B = 300 to 999, C = 100 to
299, and D = under 100. A band sets the place in the queue only. Every row receives six documents.

| # | Package | Files | Size band | Git | Purpose |
|---|---------|-------|------|-----|---------|
| 1 | `open-design` | 14,292 | A | no | A design environment built around plugins and skills |
| 2 | `src.leonnix` | 8,704 | A | no | A container of nineteen design, film, and skill checkouts |
| 3 | `impeccable` | 4,558 | A | yes | The design vocabulary for coding agents, shipped as a plugin |
| 4 | `openui` | 3,872 | A | yes | The open standard for generative user interfaces |
| 5 | `motion` | 3,570 | A | no | An animation library and its tooling |
| 6 | `Libraries.dev` | 1,478 | A | yes | Seven React user-interface libraries for agents |
| 7 | `motion-canvas` | 1,059 | A | yes | A programmatic animation canvas |
| 8 | `open-codesign` | 1,014 | A | yes | An open co-design tool |
| 9 | `styleseed` | 905 | B | yes | Repeatable design judgment for coding agents |
| 10 | `AutoDesign` | 736 | B | yes | Automated design generation |
| 11 | `transitions.dev` | 480 | B | yes | A transition skill with named demonstrations |
| 12 | `html-anything` | 379 | B | yes | An HTML generation workspace |
| 13 | `anatomy-figures` | 333 | B | yes | Themed anatomical figures |
| 14 | `two.js` | 308 | B | yes | A two-dimensional drawing library |
| 15 | `html-video` | 280 | C | yes | Video built from HTML |
| 16 | `breakscale` | 255 | C | yes | A scale and layout tool |
| 17 | `openui-trip-planner` | 197 | C | yes | A trip planner built on OpenUI |
| 18 | `src.Designer` | 174 | C | no | A design application tree |
| 19 | `anatomy` | 147 | C | yes | An anatomical drawing skill |
| 20 | `openui-f1-dashboard` | 144 | C | yes | A Formula 1 dashboard built on OpenUI |
| 21 | `asset-forge` | 99 | D | yes | A three-dimensional and pixel-art asset workshop |
| 22 | `open-intelligent-ui` | 83 | D | yes | An implementation of an intelligent user-interface pattern |
| 23 | `isometric-cities` | 71 | D | yes | Isometric pixel-art maps from real neighbourhoods |
| 24 | `thinking-orbs` | 69 | D | yes | Loading indicators for agent interfaces |
| 25 | `img-fx` | 69 | D | yes | Image effects for React |
| 26 | `appless` | 61 | D | yes | An operating-system native application pattern |
| 27 | `pixel_character_generator` | 59 | D | yes | A 64 by 64 retro pixel character generator |
| 28 | `metal-fx` | 57 | D | yes | Metal effects for React |
| 29 | `motion-canvas-exporter` | 46 | D | yes | An exporter for Motion Canvas |
| 30 | `awesome-opus-5.5-video` | 21 | D | yes | A curated video list |
| 31 | `ola` | 20 | D | yes | A small package named Ola |
| 32 | `iso-glow` | 14 | D | no | An isometric glow skill |
| 33 | `ai-iso-skill` | 10 | D | yes | An isometric figure skill |
| 34 | `colorfigures` | 9 | D | no | A colour-figure skill |

The order is size band first, then size. The largest packages teach the shared vocabulary that the
small packages reuse. Every row receives six documents, as section 3 states.

## 5. Constraints

- Aerospace and Defence Simplified Technical English (ASD-STE100) applies to every sentence.
- Each document cites its source with `path:line` and quotes code byte for byte.
- A document states when its source does not define a value. It does not guess.
- One package is in flight at a time. At most four subagents work on that one package. A new
  package does not start until the current one is committed and pushed.
- The fresh-reader review runs for each package. A fresh agent reads the package documents with no
  memory of the writing, no tools beyond reading, and no web access. The writer fixes every
  reported gap before the package is done.

## 6. Research questions

| # | Question |
|---|----------|
| 1 | What does each project do, and for which user? |
| 2 | Which projects are agent skills, which are libraries, and which are applications? |
| 3 | What mechanism carries each project's central idea? |
| 4 | How does each project ship: plugin, package, static site, or source tree? |
| 5 | Which projects share a lineage, a dependency, or a duplicated idea? |
| 6 | What does each project state about its own limits? |

## 7. Deliverables

| Item | State |
|------|-------|
| Shared `build.py`, `styles.css`, and index | Done |
| 34 package directories with `spec.md` and `markdown/` | In progress |
| Generated HyperText Markup Language (HTML) tree | Rebuilt after each package |
| Fresh-reader review per package | In progress |
| Commit and push per package | In progress |

## 8. Task table

| # | Package | State |
|---|---------|-------|
| 1 | `open-design` | DONE (6 documents, 67 snippets, fresh-reader review repaired) |
| 2 | `src.leonnix` | IN PROGRESS (container index and 19-subproject plan written; 3 of 19 nested projects published: tastecode, taste-skill, cinetic) |
| 3 | `impeccable` | TODO |
| 4 | `openui` | TODO |
| 5 | `motion` | TODO |
| 6 | `Libraries.dev` | TODO |
| 7 | `motion-canvas` | TODO |
| 8 | `open-codesign` | TODO |
| 9 | `styleseed` | TODO |
| 10 | `AutoDesign` | TODO |
| 11 | `transitions.dev` | TODO |
| 12 | `html-anything` | TODO |
| 13 | `anatomy-figures` | TODO |
| 14 | `two.js` | TODO |
| 15 | `html-video` | TODO |
| 16 | `breakscale` | TODO |
| 17 | `openui-trip-planner` | TODO |
| 18 | `src.Designer` | TODO |
| 19 | `anatomy` | TODO |
| 20 | `openui-f1-dashboard` | TODO |
| 21 | `asset-forge` | TODO |
| 22 | `open-intelligent-ui` | TODO |
| 23 | `isometric-cities` | TODO |
| 24 | `thinking-orbs` | TODO |
| 25 | `img-fx` | TODO |
| 26 | `appless` | TODO |
| 27 | `pixel_character_generator` | TODO |
| 28 | `metal-fx` | TODO |
| 29 | `motion-canvas-exporter` | TODO |
| 30 | `awesome-opus-5.5-video` | TODO |
| 31 | `ola` | TODO |
| 32 | `iso-glow` | TODO |
| 33 | `ai-iso-skill` | TODO |
| 34 | `colorfigures` | TODO |

## 9. Resume Point

Read the task table. Continue with the first `TODO` row. A package is complete only after the
source review, the fresh-reader review, a successful build, and a commit and push.

## 10. Corrections and limits found during research

| First reading | What the source shows | Evidence |
|---------------|-----------------------|----------|
| The container is one project | It holds 34 separate projects with no shared root repository | Directory listing |
| All projects are Git checkouts | 28 are Git checkouts. Six are plain directories | `git rev-parse` per package |
| All projects are design skills | They include skills, libraries, applications, demos, and curated lists | Each `README.md` |

## 11. Verification record

| Check | Result |
|-------|--------|
| Package directories | Pending |
| Source citations | Pending |
| Quoted code blocks | Pending |
| Markdown links | Pending |
| HTML pages | Pending |
| Fresh-reader review | Pending |
| Build | Pending |
