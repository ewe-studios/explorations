# Awesome Opus 5.5 Video -- Project Spec

## Source Codebase

**Location:** `/Users/darkvoid/GitSources/src.Designer/awesome-opus-5.5-video`
**Upstream data source:** `zhuyansen/jasonzhu.ai` (public repository; the daily sync reads `scripts/opus-prompts/build-repo.mjs` and `src/content/opus-prompts/cases.json` from it)
**Language:** Markdown (README and category lists), JSON (structured data), YAML (GitHub Actions workflow)
**Curator:** JasonZhu.AI (https://jasonzhu.ai)
**Online companion:** https://jasonzhu.ai/en/prompts/claude-opus-5-5 (English) and https://jasonzhu.ai/zh/prompts/claude-opus-5-5 (Chinese)
**Tagline:** A curated catalog of videos, motion graphics, 3D scenes and games made with the Claude 5.5 model family

**Scale:** One small repository. `cases.json` is a 1.8 MB pretty-printed JSON
file (65,071 lines) that holds 2,094 works. `cases/` holds 16 Markdown files:
eight per-category lists, each in English and Chinese. Two README files
(English and Chinese) render the highlights. `.github/workflows/sync.yml` is a
GitHub Action that regenerates the whole repository from the upstream
`jasonzhu.ai` source every day. There is no build step inside the project
itself: the Markdown and JSON are the finished artifacts.

## What This Project Is

This is an "awesome list". An awesome list is a curated collection of links,
one topic per list. This one collects X (formerly Twitter) posts. Each post is
a video, a motion-graphics piece, a 3D scene, or a game. The creator made the
work with the Claude 5.5 model family: Opus 5.5, Sonnet 5.5, Haiku 5.5, or
Fable 5.5. Each post has 5,000 or more views.

The catalog has two surfaces:

1. Human-readable Markdown. The two README files show a highlight table per
   category. The `cases/` files list every work in a category as a table row.
2. Machine-readable data. `cases.json` holds every work as a structured record
   with a fixed schema, so a script can query, count, or re-render the catalog.

The online companion at jasonzhu.ai adds what the Markdown cannot: it plays the
videos and shows the full prompt text for the 515 works that have one.

## Verified Aggregate Counts

These numbers were checked against `cases.json` with `jq` on 2026-10-25.

| Measure | Value |
|---|---|
| Total works | 2,094 |
| Works with a prompt | 515 |
| Full prompts | 235 |
| Brief prompts | 280 |
| Categories | 8 |
| Model mentions (flattened `models` arrays) | 2,168 |
| Multi-model works | 72 |
| Minimum views | 5,061 |
| Maximum views | 20,097,168 |
| `posted_at` range | 2026-09-22 to 2026-10-23 |

Category counts: art3d 316, comparison 376, education 176, game 373, motion
282, product 221, production 152, stories 198. Sum: 2,094.

Model mentions: opus-5.5 1956, sonnet-5.5 116, haiku-5.5 19, fable-5.5 77.
Sum: 2,168. The 72 multi-model works account for the 74 extra mentions (70
works list two models, 2 works list three).

## Documentation Structure

```
designer/awesome-opus-5.5-video/
├── spec.md                          <- THIS FILE (project tracker)
├── markdown/                        <- Source documentation
│   ├── README.md                    <- Index / table of contents (terminal page)
│   ├── 00-overview.md               <- What the catalog is, purpose, audience, the two surfaces
│   ├── 01-inclusion-criteria.md     <- The 5,000-views threshold, native video, creator's post, exclusions
│   ├── 02-model-family.md           <- Opus / Sonnet / Haiku / Fable coverage and counts
│   ├── 03-schema.md                 <- cases.json field-by-field (top-level + 18 case fields)
│   ├── 04-categories.md             <- The eight categories and how cases/ is organized
│   ├── 05-prompts.md                <- Full vs brief prompts, the four sources, the verbatim rule
│   ├── 06-bilingual-mirror.md       <- The English / Chinese mirror
│   ├── 07-maintenance.md            <- The daily sync pipeline and the 500-work guard
│   └── 08-contributing.md           <- How to submit a work, correct it, or remove it
└── html/                            <- Rendered HTML (build.py output)
    ├── index.html
    ├── styles.css
    └── *.html
```

## Tasks

### Phase 1: Core Documentation (Markdown)

| # | Task | Status | File |
|---|------|--------|------|
| 1 | Overview: purpose, audience, the two surfaces, key numbers | DONE | `00-overview.md` |
| 2 | Inclusion criteria: 5,000 views, native video, model attribution | DONE | `01-inclusion-criteria.md` |
| 3 | Model family: the four models and their coverage | DONE | `02-model-family.md` |
| 4 | Schema: cases.json field-by-field | DONE | `03-schema.md` |
| 5 | Categories: the eight categories and cases/ layout | DONE | `04-categories.md` |
| 6 | Prompts: full vs brief, sources, verbatim rule | DONE | `05-prompts.md` |
| 7 | Bilingual mirror: English / Chinese files | DONE | `06-bilingual-mirror.md` |
| 8 | Maintenance: the sync workflow and guard | DONE | `07-maintenance.md` |
| 9 | Contributing: submit, correct, remove | DONE | `08-contributing.md` |
| 10 | README index | DONE | `README.md` |

### Phase 2: HTML Rendering

| # | Task | Status | File |
|---|------|--------|------|
| 11 | Generate all HTML pages | DONE | `html/*.html` |
| 12 | Index page with navigation | DONE | `html/index.html` |
| 13 | Shared CSS | DONE | `html/styles.css` |

### Phase 3: Verification

| # | Task | Status |
|---|------|--------|
| 14 | Names match source | DONE |
| 15 | Numbers match source | DONE |
| 16 | Flows match source | DONE |
| 17 | Nothing missing from the public surface | DONE |

## Build System

**Script:** `/Users/darkvoid/Dev/explorations/build.py`
**Dependencies:** None (Python 3 standard library only)

**Usage:**
```bash
cd /Users/darkvoid/Dev/explorations
python3 build.py designer/awesome-opus-5.5-video
```

The build reads `markdown/*.md`, sorts the files alphabetically, and writes
`html/*.html`. `build.py` already has a `descriptions` entry for
`awesome-opus-5.5-video` (line 527), so the index page renders with the
correct one-line description.

## Quality Requirements (Iron Rules)

Every numbered document obeys these rules. The README is exempt from rules 2-5.

1. H1 is the first line: `# NN -- Title: subtitle` (NN zero-padded). No YAML frontmatter.
2. At least two Mermaid diagrams per document. Each Mermaid fenced block starts
   with `%%{init: {"flowchart": {"htmlLabels": false}}}%%` (use `"sequence"` for
   sequence diagrams). With `htmlLabels:false`, quote every node label and
   never put an unquoted `(` or `)` inside a label.
3. At least three real code snippets (JSON / JSON5 / YAML / Markdown) per
   document, each with a source file path and approximate line number or a
   precise field name.
4. Exactly one `## The Aha Moment` section that opens with a bolded `**Aha:**`
   line, then a short paragraph.
5. At least two cross-references to other documents in this series (relative
   links like `./03-schema.md`).
6. Footer is one line: `Continue to [NN-slug.md](NN-slug.md).`, chaining
   00 to 08 to README. README has no forward footer. No orphan pages.
7. All prose in ASD-STE100 Simplified Technical English: short sentences, one
   idea per sentence, every abbreviation spelled out at first use.
8. Teach the key idea in the first paragraph, then go deeper.

## Depth Requirement

The catalog is small but dense. Every field of `cases.json` must be explained
with a real value. Every category must be named with its slug, English title,
Chinese title, and count. The inclusion rule and the prompt-sourcing rule must
be quoted from the source. A reader must be able to reproduce every count in
this spec with `jq` and never need to open the source repository.

## Verification

The source is the grandfather. Every name, number, and flow in this
documentation was checked against the files in
`/Users/darkvoid/GitSources/src.Designer/awesome-opus-5.5-video/`. Aggregate
counts were computed with `jq` against `cases.json`. Field names were read from
the top-level object and from representative entries. The build must succeed
with zero errors before the documentation is considered complete.
