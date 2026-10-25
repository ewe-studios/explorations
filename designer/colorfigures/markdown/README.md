# Colorfigures -- Documentation Index

Colorfigures is a Claude skill. It draws a hand-inked, flat-coloured, interactive isometric figure of an everyday object as one self-contained HyperText Markup Language (HTML) page. A reader drags to turn the object all the way round. A See-through switch reveals the real working mechanism inside, with labelled parts that light up while they work. These documents explain the whole project from the source files in `/Users/darkvoid/GitSources/src.Designer/colorfigures/`.

## The documents

| Number | Title | What you learn |
|--------|-------|----------------|
| 00 | [Overview](00-overview.md) | Purpose, trigger, pipeline, the three looks, and the workflow |
| 01 | [Projection](01-projection.md) | Coordinates, the true isometric default, the free camera, and depth sorting |
| 02 | [Shading](02-shading.md) | Flat light, mid and dark bands, and the blue cast shadow |
| 03 | [Ink and paper](03-ink-paper.md) | The silhouette ink, the hand-drawn wobble, and the paper and palette styling |
| 04 | [Geometry](04-geometry.md) | The shape kit and the rules for drawing without a depth buffer |
| 05 | [Interaction](05-interaction.md) | Drag to rotate, buttons, animations, the keypad, and zoom |
| 06 | [See-through](06-seethrough.md) | The ghost case and the labelled working parts |
| 07 | [Examples and build](07-examples-build.md) | The three reference figures, the build script, and the verify workflow |

## How to read

Start at [00-overview.md](00-overview.md) and follow the footer at the end of each page. Each numbered page ends with a `Continue to` link. The chain runs 00, 01, 02, 03, 04, 05, 06, 07, then back to this index.

## The source

| File | Role |
|------|------|
| `SKILL.md` | The instructions the Claude model follows |
| `assets/core.js` | The engine: camera, shapes, shading, ink, labels, page shell |
| `assets/examples/` | Three complete reference figures: toaster, desk lamp, claw machine |
| `scripts/build.py` | Inlines the engine and adds zoom and the phone layout |

Colorfigures builds on the iso-glow skill, which builds on iso-figure. See the sibling exploration at [../../iso-glow/](../../iso-glow/).
