# ISO-GLOW — Exploration Spec

## Source location

- Project: `/Users/darkvoid/GitSources/src.Designer/iso-glow`
- Output: `/Users/darkvoid/Dev/explorations/designer/iso-glow/`

## What the project is

ISO-GLOW is a Claude skill (MIT licence, by April Zhu) that draws interactive isometric line-art figures with glow lighting. It builds on **iso-figure** by Tolga Cohce (github.com/MrBongoC/ai-iso-skill): the projection kernel, the "Fig N" plate, and the rule that every figure must do something real come from that base.

The look in one sentence: a monochrome hairline drawing on a dark panel, where one accent colour appears only on what is live, and live things glow.

ISO-GLOW adds four layers on top of the base, plus a second look:

1. **Glow lighting** — dark hairline drawing, one accent colour only on what is live.
2. **Palette picker** — the accent colour is chosen per viewer and remembered.
3. **Gallery shell** — one page holds several figures with sidebar navigation.
4. **three.js 3D mode** — the same look rebuilt in real 3D, with orbit and a true isometric view.
5. **Lumen style** — a second look: solid, lit, filled objects with geometric light.

## Document structure

```
designer/iso-glow/
├── spec.md                      (this file)
└── markdown/
    ├── README.md                index and terminal page
    ├── 00-overview.md           what the skill is, the build system, the fast paths
    ├── 01-iso-figure-base.md    the inherited projection kernel and figure contract
    ├── 02-glow.md               the glow filters, tokens and class vocabulary
    ├── 03-palette-picker.md     ember/city palettes and the accent picker
    ├── 04-gallery-shell.md      the page shell, api object and mount lifecycle
    ├── 05-3d-mode.md            the three.js shell, line tiers and crease extraction
    ├── 06-3d-figures.md         the registry-driven figures and raycast interaction
    ├── 07-lumen.md              the Lumen solid-lit style and gradient picker
    └── 08-anatomy-builds.md     exploded, sectioned, labelled real objects
```

Reading order chains 00 through 08 and ends at README. README is the terminal page and has no forward footer.

## Iron Rules

Every numbered document follows these rules. README is exempt from rules 2-5.

1. The H1 is the first line: `# NN -- Title: subtitle` (NN zero-padded). No YAML frontmatter.
2. At least two Mermaid diagrams. The first line of every Mermaid fence is `%%{init: {"flowchart": {"htmlLabels": false}}}%%`. With `htmlLabels:false`, quote every node label and never put an unquoted `(` or `)` inside a label.
3. At least three real code snippets, each with a source file path and approximate line number.
4. Exactly one `## The Aha Moment` section that opens with a bolded `**Aha:**` line, then a short paragraph.
5. At least two cross-references to other documents in this series (relative links such as `./02-glow.md`).
6. The footer is one line: `Continue to [NN-slug.md](NN-slug.md).`, chaining 00 to 01 through 08 to README. README has no forward footer. No orphan pages.
7. All prose in ASD-STE100 Simplified Technical English.
8. Teach the key idea in the first paragraph, then go deeper.

## Build command

```
cd /Users/darkvoid/Dev/explorations && python3 build.py designer/iso-glow
```

The build converts `markdown/*.md` into `designer/iso-glow/html/*.html`. `build.py` is shared and must not be edited. It already has an `iso-glow` index description.

## Source inventory

- `SKILL.md` — the skill: look, fast paths, figure contract, kernel, glow kit, palettes, procedure, lessons, 3D mode, Lumen, Don'ts.
- `references/anatomy-builds.md` — 10 rules for exploded, sectioned, labelled real-object builds.
- `references/lumen.md` — Lumen fast path, look rules, kit API, lessons.
- `assets/gallery.html` — the 2D page: tokens, filters, CSS vocabulary, kernel, api, navigation, palette picker.
- `assets/gallery-3d.html` — the 3D page: renderer, line tiers, crease extraction, figures, registry.
- `assets/examples/desk-computer.js` — a typing desk computer with a coiled cable and light pulse.
- `assets/examples/office-tower.js` — a 24-floor office tower with floor toggles and a light cascade.
- `assets/examples/lens-camera-3d.js` — the anatomy reference build: a traced 50 mm f/1.8 lens on a mirrorless body.
- `assets/lumen/kit.js` — the RW shading kit for Lumen.
- `assets/lumen/picker.js` — the Lumen gradient picker.
- `assets/lumen/examples/card-terminal.js`, `eth-blocks.js`, `vault-safe.js` — Lumen example figures.
- `LICENSE` — MIT, April Zhu and Tolga Cohce.
