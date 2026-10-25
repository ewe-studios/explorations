# ai-iso-skill -- Project Spec

## Source Codebase

**Location:** `/Users/darkvoid/GitSources/src.Designer/ai-iso-skill`
**Language:** The skill is a Markdown instruction file. Its output is one HyperText Markup Language (HTML) file that holds inline JavaScript (JS), inline Cascading Style Sheets (CSS), and inline Scalable Vector Graphics (SVG). No build step and no dependencies.
**Author:** Tolga Cohce (`MrBongoC` on GitHub)
**License:** Massachusetts Institute of Technology (MIT) License
**Repository:** https://github.com/MrBongoC/ai-iso-skill
**Version:** 1.0.0 (in `.claude-plugin/plugin.json`)

**Scale:** Small. The whole project is nine text files plus two animated Graphics Interchange Format (GIF) images:

```
ai-iso-skill/
├── README.md                       49 lines
├── LICENSE                         21 lines
├── .gitignore                       2 lines
├── .claude-plugin/
│   ├── plugin.json                 12 lines
│   └── marketplace.json            14 lines
├── skills/
│   └── iso-figure/
│       └── SKILL.md               116 lines   ← the entire skill
├── examples/
│   ├── pocket-calculator.html     144 lines   ← worked output #1
│   └── desk-synth.html            193 lines   ← worked output #2
└── docs/
    ├── pocket-calculator.gif
    └── desk-synth.gif
```

## What the Project Is

`iso-figure` is a Claude skill. The user names an object. The skill has Claude produce one
self-contained HTML file: a hairline isometric line drawing on a numbered plate, with parts
that work when pressed. No libraries, no canvas, no three-dimensional (3D) engine. The whole
projection is about fifteen lines of JavaScript.

The central idea is stated in the skill file: **the object must do something real.** A figure
that only animates is decoration. A figure where pressing a part produces output — typed text,
a number, a note, a sound, a changed display — is the point.

## Documentation Structure

```
designer/ai-iso-skill/
├── spec.md                            ← THIS FILE (project tracker)
├── markdown/                          ← Source documentation
│   ├── README.md                      ← Index / table of contents (terminal page)
│   ├── 00-purpose-trigger.md          ← What the skill is, when it fires, the claims checked
│   ├── 01-projection.md               ← The projection kernel: P, D, plane, TOP, FRONT, SIDE, rect, box
│   ├── 02-assembly.md                 ← How one HTML file is assembled: skeleton, scene, paint order, frame
│   ├── 03-plate-and-parts.md          ← The numbered plate, pressable parts, state/render, input patterns
│   ├── 04-palette-theming.md          ← CSS custom properties, dark/light sets, two greys + one live colour
│   ├── 05-request-to-figure.md        ← The nine-step procedure from prompt to verified file
│   └── 06-plugin-packaging.md         ← .claude-plugin layout, the skills folder, install paths
└── html/                              ← Rendered HTML (build.py output)
    ├── index.html
    ├── styles.css
    └── *.html
```

## Iron Rules (every numbered document; README exempt from rules 2-5)

1. H1 is the first line: `# NN -- Title: subtitle` (NN zero-padded). No YAML frontmatter.
2. At least two Mermaid diagrams. Each Mermaid fenced block starts with
   `%%{init: {"flowchart": {"htmlLabels": false}}}%%` (use `"sequence"` instead of `"flowchart"`
   for sequence diagrams). With `htmlLabels:false`, quote every node label and never put an
   unquoted `(` or `)` inside a label.
3. At least three real code snippets (JS, HTML, SVG, JSON, YAML), each with a source file path
   plus an approximate line number.
4. Exactly one `## The Aha Moment` section that opens with a bolded `**Aha:**` line, then a
   short paragraph.
5. At least two cross-references to other documents in this series (relative links).
6. Footer is one line: `Continue to [NN-slug.md](NN-slug.md).`, chaining
   00 -> 01 -> 02 -> 03 -> 04 -> 05 -> 06 -> README. README has no forward footer. No orphan pages.
7. All prose in ASD-STE100 Simplified Technical English: short sentences, one idea per sentence,
   every abbreviation spelled out at first use.
8. Teach the key idea in the first paragraph, then go deeper.

## Build Command

```
cd /Users/darkvoid/Dev/explorations && python3 build.py designer/ai-iso-skill
```

The build reads every `*.md` file under `markdown/`, converts each to HTML, and writes the
result to `designer/ai-iso-skill/html/`. The build script is
`/Users/darkvoid/Dev/explorations/build.py` (Python standard library only; do not edit it).
Do not run git; do not commit.

## Source Files Read in Full

- `README.md` — the project README (claims, install, usage).
- `skills/iso-figure/SKILL.md` — the entire skill: frontmatter, kernel, CSS, procedure, input patterns, look, don'ts.
- `examples/pocket-calculator.html` — worked output: calculator figure.
- `examples/desk-synth.html` — worked output: synthesizer figure.
- `.claude-plugin/plugin.json` and `.claude-plugin/marketplace.json` — plugin packaging.
- `LICENSE` and `.gitignore` — license and ignored files.
