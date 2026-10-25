# Designer exploration: anatomy

## Source Codebase

Location: `/Users/darkvoid/GitSources/src.Designer/anatomy`

## What anatomy Is

`anatomy` is a Claude Agent Skill (MIT license). It is a kit for technical exploded-view figures of real objects. A builder turns a concept into a machine, an instrument or a piece of furniture. The figure is isometric, interactive and crafted. The object is the explanation.

The project ships:

- `SKILL.md` (the skill definition, about 33 KB), `README.md` and `CHANGELOG.md`.
- `kit/` — the geometry modules: `iso-kit` (core), `lathe` (round parts), `tube` (pipes and cables), `audit` (geometry proof), `gl` and `gl-shared` (WebGL), `turn*` (3D turning groups), `canvas-painter`, `react/` and `iso.css`.
- `references/` — ten craft and API documents.
- `scripts/` — the capture, drive, verify, publish and print toolchain plus 45 test files.
- `agents/` and `evals/` — agent interface and eval fixtures.
- `examples/` — nine worked figures.
- `tests/fixtures/` — small reference images.

## Documentation Goal

Teach the whole project so a fresh reader never needs to open the source. Cover the purpose, the pipeline, every kit module, the references, the worked examples, and the capture/verify/publish toolchain with real source-level detail.

## Documentation Structure

```
designer/anatomy/
  spec.md
  markdown/
    README.md          index, no forward footer
    00-overview.md     purpose, trigger, sizes, what ships
    01-pipeline.md     the ten-step workflow and the toolchain map
    02-isometric-core.md   iso-kit: camera math and four-tone shading
    03-lathe.md        round parts on any axis
    04-tube.md         pipes, hoses and cables
    05-audit.md        signed distance fields and geometry proof
    06-webgl.md        shaders inside the drawing
    07-turn.md         three-dimensional turning groups (beta)
    08-react.md        server-safe drawing and client hooks
    09-references.md   the ten reference documents
    10-capture-verify.md   capture, drive, lines and turn checks
    11-publish-print.md    publish, print and sheet toolchain
    12-examples.md     the nine worked figures
    13-tests-evals.md  the test suite and evals
```

Footer chain: `00 -> 01 -> 02 -> 03 -> 04 -> 05 -> 06 -> 07 -> 08 -> 09 -> 10 -> 11 -> 12 -> 13 -> README`.

## Iron Rules

1. H1 first line `# NN -- Title: subtitle` (NN zero-padded). No YAML frontmatter.
2. At least two Mermaid diagrams per numbered doc. Each fenced block starts with `%%{init: {"flowchart": {"htmlLabels": false}}}%%`; use the key `"sequence"` for sequence diagrams. With `htmlLabels: false`, quote every node label as `A["text"]` and never put an unquoted `(` or `)` inside a label.
3. At least three real code snippets per numbered doc, each with its source file path and approximate line number.
4. Exactly one `## The Aha Moment` section that opens with a bolded `**Aha:**` line, then a short paragraph.
5. At least two cross-references to other docs in the series, as relative links like `./04-tube.md`.
6. Footer is one line: `Continue to [NN-slug.md](NN-slug.md).` The chain runs 00 through 13 and ends at README. README has no forward footer. No orphan pages.
7. All prose uses ASD-STE100 Simplified Technical English. Short sentences, one idea per sentence, abbreviations spelled out at first use.
8. Teach the key idea in the first paragraph, then go deeper.

## Build

```
cd /Users/darkvoid/Dev/explorations && python3 build.py designer/anatomy
```

The build writes HTML to `designer/anatomy/html/`. Do not edit `build.py`.
