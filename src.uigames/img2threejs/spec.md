---
source_location: /Users/darkvoid/GitSources/src.uigames/img2threejs
repository: git@github.com:img2threejs/img2threejs.git
explored_at: 2026-11-15
commit: 0c0c49bc168e648dbaf9c8c080474de0b3a941b6
commit_date: 2026-11-05
version: 2.0.1 (skill), 0.2.0 (Node package)
license: Apache-2.0
languages: Python, TypeScript, JavaScript, Markdown, YAML
---

# Spec: img2threejs, an image-to-Three.js skill for agents

## 1. Source

| Property | Value |
|----------|-------|
| Snapshot location | `/Users/darkvoid/GitSources/src.uigames/img2threejs` |
| Remote | `git@github.com:img2threejs/img2threejs.git` |
| Commit | `0c0c49bc168e648dbaf9c8c080474de0b3a941b6` |
| Commit date | 2026-11-05 |
| Skill version | `2.0.1` (release tag) |
| Node package version | `0.2.0` (`package.json`) |
| License | Apache-2.0 |
| Primary languages | Python 3.10+ standard library, TypeScript, JavaScript, Markdown |
| Source of truth | The source at the commit above. Every claim names a file and, where useful, a line. |

Every document cites the same snapshot. The hash in the front matter above is the one that the
grandfather review used.

## 2. Scope

This exploration explains how img2threejs turns one reference image into a TypeScript
Three.js factory. It covers the two halves of the product: the agent-facing skill contract and
the deterministic Python forge. It covers the installer, the five forge stages, the gate system,
the reference library, the optional integrations, the test and release machinery, and the limits
that the project states about itself.

It does not cover the hosted demo gallery, the paid rendering services that plugins can call, or
the Three.js library itself.

## 3. Method

1. Read `README.md`, `CLAUDE.md`, `CONTRIBUTING.md`, `SKILL.md`, and `docs/ARCHITECTURE.md`.
2. Map every directory under `forge/`, `grimoire/`, `integrations/`, `scripts/`, and `bin/`.
3. Read each stage from its entry script to its gates. Follow one object and one character
   through the pipeline.
4. Write one document for each layer. Quote the source byte for byte and cite the file and line
   range.
5. Review every document against the pinned source, then with a fresh reader.

## 4. Documents

| Document | Subject |
|----------|---------|
| `00-overview.md` | The two halves, the thesis, the vocabulary, and the reading guide |
| `01-repository-anatomy.md` | The directory map, the product split, and the dependency rules |
| `02-installer-cli.md` | The Node installer, host detection, ref pinning, and exit codes |
| `03-skill-contract.md` | `SKILL.md`: routing, the state gate, the loop, and the budgets |
| `04-forge-orchestration.md` | `state.py`, `next.py`, `workflow_state.py`, and pipeline routing |
| `05-stage-intake.md` | Image probe, GLB probe, detail inventory, landmarks, and PBR evidence |
| `06-stage-spec.md` | Pre-spec assessment, spec authoring, strict-quality validation, and proportions |
| `07-stage-build.md` | Pass orchestration, the factory generator, emission, and geometry gates |
| `08-stage-review.md` | Comparison sheets, interior difference, render scoring, and self-correction |
| `09-stage-rig-and-character.md` | Rig spec, geodesic skinning, rig payload, hair, and characters |
| `10-grimoire.md` | The reference library: rubrics, gates reference, and build patterns |
| `11-integrations.md` | Vision, mesh3d, the GLB character pipeline, and optional tools |
| `12-materials-and-rendering.md` | Material reference pipeline, PBR, render profile, and the render bridge |
| `13-testing-and-ci.md` | The 107 test modules, the CI scripts, and the workflows |
| `14-release-and-operations.md` | Versioning, release gates, docs quality, and maintenance |
| `15-limits-and-findings.md` | Lab findings, the roadmap, and the limits the project states |

## 5. Constraints

- Aerospace and Defence Simplified Technical English (ASD-STE100) applies to every sentence.
  Expand each abbreviation at first use in each file.
- Every document has at least two Mermaid diagrams, at least ten byte-exact code snippets, at
  least one Aha insight, and `path:line` citations.
- A snippet shows the source as it is. It does not paraphrase and it does not join several files
  into one block.
- The documents state when the repository does not define a value or a behaviour. They do not
  fill a gap with a guess.

## 6. Research questions

| # | Question |
|---|----------|
| 1 | What are the two halves of the product, and why are they separate? |
| 2 | How does the installer choose a host, a repository reference, and an install path? |
| 3 | How does the skill contract route a request and enforce its state gate? |
| 4 | How does the forge keep a run resumable across sessions? |
| 5 | Which gates run on geometry, and which gates run on pixels? |
| 6 | How does the material pipeline turn a crop into a spec entry and an accepted render? |
| 7 | What do the tests cover, and what do they not cover? |
| 8 | Which limits does the project state about likeness, inputs, and cost? |

## 7. Deliverables

| Item | State |
|------|-------|
| `spec.md` | Written |
| `markdown/README.md` | Written |
| Fifteen deep-dive documents | Written, sixteen Markdown files in total |
| Generated HyperText Markup Language (HTML) tree | Built, 20 pages |
| Fresh-reader review of every document | Done. One fresh agent with no memory of the writing read all sixteen Markdown files with no web access and reported 25 cross-document contradictions and per-file depth gaps. Both writers then verified every finding against the source and repaired the real ones. Findings the source could not settle are marked as unexplained in the documents. |
| Commit and push | Done |

## 8. Task table

| # | Task | State |
|---|------|-------|
| 1 | Read the repository instructions and the documentation directive | DONE |
| 2 | Map every directory, script, and test | DONE |
| 3 | Research the installer and the skill contract | DONE |
| 4 | Research forge orchestration and the five stages | DONE |
| 5 | Research the review and rig stages | DONE |
| 6 | Research the grimoire and the integrations | DONE |
| 7 | Research the tests, CI, and release machinery | DONE |
| 8 | Write the overview, the anatomy, and the index | DONE |
| 9 | Write documents 02 to 06 | DONE |
| 10 | Write documents 07 to 11 | DONE |
| 11 | Write documents 12 to 15 | DONE |
| 12 | Review every document against the pinned source | DONE |
| 13 | Fresh-reader review, and fix every reported gap | DONE |
| 14 | Build the HTML tree and check every link | DONE |
| 15 | Commit and push | DONE |

## 9. Resume Point

Read the task table. Continue with the first `TODO` row. A document is complete only after the
source review, the fresh-reader review, and a successful build.

## 10. Corrections and limits found during research

| First reading | What the source shows | Evidence |
|---------------|-----------------------|----------|
| img2threejs is a model that generates Three.js code | The repository holds no model. It is a skill contract plus deterministic Python scripts. The host agent supplies the vision and the code generation | `SKILL.md`, `forge/` |
| The Python scripts render the image | The scripts validate, measure, gate, and package. The browser Three.js route is the rendering authority | `docs/ARCHITECTURE.md`, `grimoire/build/python_threejs_render_bridge.md` |
| The pipeline runs in one pass | It runs eight locked passes, and a pass unlocks only after review | `docs/ARCHITECTURE.md`, `forge/stage3_build/orchestrate_passes.py` |
| The skill is the Node package | The Node package installs the skill. The skill itself is the Markdown tree in the repository | `package.json`, `bin/img2threejs.mjs` |
| The pipeline always reaches a result | Routing can refuse below a confidence of 0.82 and return `request-input` | `forge/_shared/pipeline_routing.py` |
| SKILL.md lists seven passes | The code order has eight; `SKILL.md:70` shortens labels and omits surface-pass. The code list is authoritative | `SKILL.md:70`, `forge/stage3_build/orchestrate_passes.py:18-27` |
| The primitive list has twelve or fourteen kinds | `VALID_PRIMITIVES` has fifteen members | `forge/stage2_spec/validate_sculpt_spec.py:43-59` |
| Stage 5 has four Python modules | Stage 5 has twelve Python files; four are the core contract modules and `rig_gates.py` holds the twelve gates | `forge/stage5_rig/` |
| The test suite count is one number | Two different runs are recorded: 1,416 collected with 64 skips in the base suite, and 1,343 with 30 skips in the Stage R plan record | `13-testing-and-ci.md`, `15-limits-and-findings.md` |

## 11. Verification record

| Check | Result |
|-------|--------|
| Markdown documents | 16 deep-dive Markdown files plus `README.md` in `markdown/`, and `spec.md` |
| Source citations | Every quoted block carries a `path:line` header that resolves against the pinned source; the validator reported 0 missing paths |
| Quoted code blocks | 392 snippets, 0 mismatches |
| Markdown links | 0 broken after the build (424 local targets checked) |
| HTML pages | 20 generated: 16 deep dives, `README.html`, `spec.html`, inner `index.html`, and root `index.html` |
| Fresh-reader review | One delegated pass with no memory of the writing; findings repaired as recorded in section 7 |
| Build | Clean; no raw code-fence markers in the rendered pages |
