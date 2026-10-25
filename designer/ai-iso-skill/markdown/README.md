# ai-iso-skill -- Index

`iso-figure` is a Claude skill by Tolga Cohce (`MrBongoC`). The user names an object. The
skill has Claude produce one self-contained HyperText Markup Language (HTML) file: a hairline
isometric line drawing on a numbered plate, with parts that work when pressed. No libraries,
no canvas, no three-dimensional (3D) engine. About fifteen lines of JavaScript (JS) do the
projection.

The key idea comes first in the skill file: **the object must do something real.** A figure
that only animates is decoration. A figure where pressing a part produces output — typed text,
a number, a note, a sound, a changed display — is the point. Every other rule serves that one.

The documents are ordered as a course. Start at 00 and follow the *Continue to* footer, or jump
to any piece. Each document teaches its key idea in the first paragraph, shows the real code,
and names the source file it reads.

## The course

| # | Document | What it teaches |
|---|---|---|
| 00 | [Purpose and Trigger](./00-purpose-trigger.md) | What the skill is, when it fires, and the claims checked against the source |
| 01 | [Projection Math](./01-projection.md) | The kernel: `P`, `D`, `plane`, `TOP`, `FRONT`, `SIDE`, `rect`, `box` |
| 02 | [File Assembly](./02-assembly.md) | One HTML file: skeleton, scene, paint order back to front, frame |
| 03 | [Plate and Parts](./03-plate-and-parts.md) | The numbered plate, pressable parts, one `state`, one `render()`, input |
| 04 | [Palette and Theming](./04-palette-theming.md) | Cascading Style Sheets (CSS) custom properties, dark and light, two greys and one live colour |
| 05 | [Request to Figure](./05-request-to-figure.md) | The nine-step procedure from prompt to a verified file |
| 06 | [Plugin Packaging](./06-plugin-packaging.md) | `.claude-plugin`, the skills folder, and the three install paths |

## The source

Everything is checked against the source at
`/Users/darkvoid/GitSources/src.Designer/ai-iso-skill`:

```
ai-iso-skill/
├── README.md                    the project README: claims, install, usage
├── LICENSE                      Massachusetts Institute of Technology (MIT) License
├── .claude-plugin/
│   ├── plugin.json              plugin name, version, author, repository
│   └── marketplace.json         marketplace name, owner, plugin list
├── skills/
│   └── iso-figure/
│       └── SKILL.md             the entire skill: kernel, CSS, procedure, look, don'ts
├── examples/
│   ├── pocket-calculator.html   worked output: a calculator that really calculates
│   └── desk-synth.html          worked output: a synthesizer that really plays notes
└── docs/
    ├── pocket-calculator.gif    animated preview of the calculator
    └── desk-synth.gif           animated preview of the synthesizer
```

## What each file proves

`SKILL.md` is the specification. The two files under `examples/` are real outputs that obey
it. The calculator file shows typed text and arithmetic. The synthesizer file shows held
instrument keys, drag across keys, and live sound. Both files embed the same projection kernel
verbatim. The two files under `docs/` are the moving previews used by the README.

Start with [00-purpose-trigger.md](00-purpose-trigger.md).
