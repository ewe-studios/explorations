# ISO-GLOW -- Deep Documentation Index

ISO-GLOW is a Claude skill that draws interactive isometric line-art figures with glow lighting. The whole skill rests on one idea: draw a dark hairline isometric object, then colour only what is live in one accent colour and make that colour glow. This index indexes a nine-document exploration of the source, from the inherited projection kernel to the three.js 3D mode and the Lumen solid-lit style.

## The documents

- [00 -- Overview](00-overview.md) — what the skill is, the source layout, and the two fast paths that assemble a page.
- [01 -- The iso-figure base](01-iso-figure-base.md) — the projection kernel, the plane transforms, and the figure contract inherited from iso-figure.
- [02 -- Glow lighting](02-glow.md) — the tokens, the three glow filters, and the class vocabulary that lights only what is live.
- [03 -- The palette picker](03-palette-picker.md) — the ember and city palettes, the accent choice, and the light-theme accent handling.
- [04 -- The gallery shell](04-gallery-shell.md) — the page shell, the shared api object, and the mount lifecycle.
- [05 -- 3D mode](05-3d-mode.md) — the three.js shell, the three line tiers, and the crease-extraction algorithm.
- [06 -- 3D figures](06-3d-figures.md) — the registry-driven figures, raycast clicks, flashes, and canvas screens.
- [07 -- Lumen style](07-lumen.md) — the solid lit look, the RW shading kit, and the gradient picker.
- [08 -- Anatomy builds](08-anatomy-builds.md) — exploded, sectioned, labelled real objects built at real scale.

## How to read this exploration

Each numbered document teaches one layer. Read them in order: the base first, then the glow, then the shells and the 3D and Lumen modes. Every document carries at least two diagrams, three code snippets with file paths and line numbers, and a section called "The Aha Moment" that names the one insight the layer exists to teach.

The source of truth is the skill folder at `/Users/darkvoid/GitSources/src.Designer/iso-glow`. The snippets in these documents are copied from that folder; the line numbers point into it. A fresh reader should not need to open the source, because every significant function and data structure is documented here.

## The key terms

- **Isometric** — a parallel projection where the three axes meet at equal angles; the viewer sees a top face and two side faces.
- **Hairline** — a stroke that stays one pixel wide at any zoom, drawn without shading.
- **Accent** — the single colour token (`--accent`) that marks what is live.
- **Live** — the state that produces real output: a pressed key, a lit screen, a floor with its lights on.
- **Fig N** — the textbook-plate label that names each figure.
- **Lumen** — the solid, lit, filled alternative to the hairline look.
