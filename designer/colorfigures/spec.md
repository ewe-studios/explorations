# Colorfigures Documentation -- Project Spec

## Source Codebase

**Location:** `/Users/darkvoid/GitSources/src.Designer/colorfigures/`
**Language:** JavaScript (JavaScript) plus one Python build script
**Version:** Unversioned (skill source, MIT license)
**Author:** April Zhu ([@Aprilzz423](https://x.com/Aprilzz423))
**License:** MIT (Massachusetts Institute of Technology) license

Files in the source project:

| File | Lines | Purpose |
|------|-------|---------|
| `SKILL.md` | 190 | The instructions the Claude model follows |
| `README.md` | 49 | Install, try, credits, license |
| `LICENSE` | 23 | MIT license text |
| `assets/core.js` | 641 | The engine: camera, shapes, shading, ink, labels, page shell |
| `assets/examples/toaster.html` | 406 | Smallest complete reference figure |
| `assets/examples/lamp.html` | 395 | Reference figure on a board with a jointed arm |
| `assets/examples/claw.html` | 974 | Large reference figure with glass, a game, and a keypad |
| `scripts/build.py` | 25 | Inlines the engine and adds zoom and the phone layout |

## What Colorfigures Is

Colorfigures is a Claude skill. It draws a hand-inked, flat-coloured, interactive isometric figure of an everyday object as one self-contained HTML page. The figure is a real three-dimensional body shaded in flat light, mid and dark bands. It has a heavy ink silhouette, a flat blue cast shadow, warm paper, and body-colour swatches. A reader drags to turn the object all the way round. A See-through switch fades the case to a ghost and shows the real working parts inside, with labelled parts that light up while they work. Every figure does something real: buttons press, doors open, items drop, parts move.

One figure is one HTML page. The page holds a small engine (`assets/core.js`) plus a figure script. The figure script models the object in three dimensions and draws it with the engine's shapes. The engine owns the camera, the shading, the ink, and the cast shadow. `scripts/build.py` makes the finished page: it inlines the engine and adds zoom, notices, the See-through and Labels switches, and the phone layout.

## Documentation Goal

The documents must let a reader understand, without opening the source:

1. What the skill is, when to use it, and what its output looks like.
2. The isometric projection and the free camera it adapts from the iso-glow skill.
3. The flat-light shading (light, mid, dark bands) and the cast shadow.
4. The ink silhouette and the hand-drawn wobble, the paper and palette styling.
5. The geometry kit: planes, lofts, extrusions, cylinders, rods, boxes, text, boards.
6. The interaction: drag to rotate, buttons, choices, animations, the game keypad, zoom.
7. The See-through mode and the labelled working parts.
8. The three reference figures and the build and verify workflow.

## Documentation Structure

```
colorfigures/
├── spec.md                         <- THIS FILE (project tracker)
├── markdown/                       <- Source documentation
│   ├── README.md                   <- Index / table of contents (terminal page)
│   ├── 00-overview.md              <- Purpose, trigger, pipeline, three looks, workflow
│   ├── 01-projection.md            <- Coordinates, isometric default, free camera, depth sort
│   ├── 02-shading.md               <- Flat light/mid/dark bands and the blue cast shadow
│   ├── 03-ink-paper.md             <- Silhouette ink, wobble filter, paper and palette styling
│   ├── 04-geometry.md              <- The shape kit and the no-depth-buffer drawing rules
│   ├── 05-interaction.md           <- Drag to rotate, buttons, animations, keypad, zoom
│   ├── 06-seethrough.md            <- See-through ghost and the labelled working parts
│   └── 07-examples-build.md        <- The three reference figures, build.py, verify workflow
└── html/                           <- Generated HTML (produced by the build)
    ├── index.html
    ├── styles.css
    └── *.html
```

## Tasks

| # | Task | Status | File |
|---|------|--------|------|
| 1 | Overview: purpose, trigger, pipeline, three looks, workflow | DONE | `00-overview.md` |
| 2 | Projection: coordinates, isometric default, free camera, depth sort | DONE | `01-projection.md` |
| 3 | Shading: flat bands and the blue cast shadow | DONE | `02-shading.md` |
| 4 | Ink and paper: silhouette, wobble, palette, paper | DONE | `03-ink-paper.md` |
| 5 | Geometry: the shape kit and the drawing rules | DONE | `04-geometry.md` |
| 6 | Interaction: drag, buttons, animations, keypad, zoom | DONE | `05-interaction.md` |
| 7 | See-through: ghost case and labelled parts | DONE | `06-seethrough.md` |
| 8 | Examples and build: three figures, build.py, verify | DONE | `07-examples-build.md` |
| 9 | README index | DONE | `README.md` |
| 10 | Build the HTML | PENDING | `python3 build.py designer/colorfigures` |

## Build System

**Script:** `/Users/darkvoid/Dev/explorations/build.py` (shared, standard library only)
**Dependencies:** None (Python 3 standard library only)

**Usage:**

```bash
cd /Users/darkvoid/Dev/explorations && python3 build.py designer/colorfigures
```

This produces `designer/colorfigures/html/`. The build:

- Converts each `markdown/*.md` file to HTML with headings, lists, tables, links, and code blocks.
- Renders Mermaid diagrams client-side from a content delivery network (CDN).
- Generates `html/index.html` and prev/next navigation in alphabetical slug order.
- Copies a shared `styles.css` on first run.
- Is idempotent: safe to run again.

Do not edit `build.py`.

## Quality Requirements (Iron Rules)

Every numbered document must meet all of these. The README is exempt from rules 2 to 5.

1. H1 is the first line: `# NN -- Title: subtitle` (NN zero-padded). No YAML frontmatter.
2. At least TWO Mermaid diagrams. At the top of each Mermaid fenced block put `%%{init: {"flowchart": {"htmlLabels": false}}}%%` (use `"sequence"` for sequence diagrams). With `htmlLabels:false`, quote every node label (for example `A["text"]`) and never put an unquoted `(` or `)` inside a label.
3. At least THREE real code snippets (JavaScript, HTML, SVG, JSON, YAML), each with a source file path and approximate line number.
4. Exactly one `## The Aha Moment` section that opens with a bolded `**Aha:**` line, then a short paragraph.
5. At least TWO cross-references to other documents in this same series (relative links like `./02-shading.md`). You may optionally link to the sibling `iso-glow` exploration at `../../iso-glow/`.
6. Footer is one line: `Continue to [NN-slug.md](NN-slug.md).`, chaining 00 then 01 then ... then last then README; README has no forward footer. No orphan pages.
7. All prose in ASD-STE100 Simplified Technical English: short sentences, one idea per sentence, every abbreviation spelled out at first use.
8. Teach the key idea in the first paragraph, then go deeper.

## Expected Outcome

A reader unfamiliar with Colorfigures can:

1. Read the overview and understand what the skill is and when to use it in five minutes.
2. Read the projection and shading documents and understand how the figure is drawn and lit.
3. Read the geometry and interaction documents and understand how a figure is modelled and made interactive.
4. Read the see-through document and understand how the working mechanism is revealed and labelled.
5. Read the examples and build document and reproduce the full workflow.
6. View the documentation as rendered HTML locally.

## Resume Point

All markdown is written. The next step is to run the build:

```bash
cd /Users/darkvoid/Dev/explorations && python3 build.py designer/colorfigures
```

Then fix any errors until the build succeeds, and mark task 10 as DONE.
