---
title: athenaion package exploration
source_location: /Users/darkvoid/GitSources/src.aidesign/src.leonnix/athenaion
repository: Git checkout, local pin 9b8bd23
explored_at: 2026-11-16
version: 0.1.0
license: MIT
files: 220
documents: 6
languages: JavaScript, Graphics Library Shading Language (GLSL), HyperText Markup Language (HTML), Markdown, JSON
---

# Specification: athenaion package exploration

## 1. Source

| Property | Value |
|---|---|
| Location | `/Users/darkvoid/GitSources/src.aidesign/src.leonnix/athenaion` |
| Kind | Git checkout with its own history |
| Pin | `9b8bd23` (short local commit identifier) |
| Tracked files | 220 |
| Version | `0.1.0` (`package.json:4`) |
| Licence | MIT (`LICENSE:1`) |
| Runtime | Browser, WebGL 2 through three.js `0.186.1` (`package.json:12`) |
| Dependencies | `three` 0.186.1, `puppeteer-core` 25.14.0, `pngjs` 7.0.0 (`package.json:9-13`) |
| Scripts | `serve` starts `tools/serve.mjs`; `render` starts `tools/render.mjs` (`package.json:5-8`) |
| Entry page | `index.html` loads `/src/main.js` as a module (`index.html:27`) |
| Entry module | `src/main.js` selects interactive, still or test mode (`src/main.js:12-14`) |
| Module order | 15 world modules in `src/world/index.js` (`MODULE_ORDER`) |

Athenaion is a procedural ancient Greek world. The world has an acropolis, a large Doric temple, colossal
statues, an aqueduct, a terraced city, a lagoon, a river, waterfalls, vegetation and a mountain range. It
uses no image, model or audio assets. Every mesh, texture, shader, tree, statue and waterfall is built in
code from a seed. The project renders nine fixed shots and one 54 second film.

The project states that an AI coding agent built the world over about two days, with 55 subagents and
about 45 hours of wall clock time (`README.md:3,30-34`). The exploration records this as a project
statement. It does not verify the process.

The tracked files divide like this:

| Path | Files | Role |
|---|---:|---|
| `src` | 176 | 168 JavaScript modules, 4 `.mjs` helper scripts, 4 `READY` markers |
| `tools` | 11 | Server, still render, video render, session, snapshot and statistics tools |
| `media` | 12 | Nine shot images, an overview image, a comparison image and a teaser animation |
| `refs` | 9 | The nine reference images |
| `docs` | 4 | Art bible, composition, module contract and build prompt |
| Root files | 7 | `README.md`, `LICENSE`, `index.html`, `package.json`, lock file, ignore files |
| `generated` | 1 | `terrain-far-v11.bin`, a 23 mebibyte baked far-terrain grid |

The `src` tree holds 43,797 JavaScript lines and 313 `.mjs` lines. The `tools` tree holds 810 lines.
The source divides into these folders:

| Folder | Files | JavaScript lines | Role |
|---|---:|---:|---|
| `core` | 12 | 2,381 | Renderer, pipeline, lighting, sky, atmosphere, materials, quality, random numbers |
| `world` | 5 | 1,274 | Master plan, heightfield, shots, look settings, module order |
| `modules` | 15 | 5,761 | One build module per feature: terrain, water, temple, acropolis, city, waterfalls and others |
| `arch` | 13 | 3,612 | Architecture kit: orders, columns, entablature, temple, tholos, ornaments, ruins |
| `sculpt` | 13 | 3,347 | Signed distance sculpting, surface mesher, statue library, head and figure |
| `terrain` | 15 | 2,212 | Heightfield mesh, level of detail, worker, erosion, far field |
| `acropolis` | 19 | 5,184 | Plateau walls, stair, terraces, buildings, garden, arcade |
| `aqueducts` | 4 | 1,385 | Arch arcade, channel, spillways |
| `city` | 13 | 3,969 | Houses, streets, harbour, landmarks |
| `water` | 10 | 2,823 | Sea, lagoon, river, waterfall sheets, foam, reflections |
| `vegetation` | 8 | 2,472 | Scatter, tree species, LOD, wind |
| `flora` | 9 | 1,852 | Grass, flowers, ivy, reeds |
| `props` | 9 | 2,623 | Banners, braziers, urns, owl, shield and small objects |
| `ruins` | 14 | 2,025 | Broken columns, fallen blocks, the fallen head |
| `materials` | 7 | 1,972 | Marble, stone, metal, procedural texture |
| `sets` | 3 | 483 | Set dressing groups |
| `video` | 2 | 130 | Film sequence and frame player |
| `test` | 1 | 45 | Neutral test scene |
| `bake` | 3 | 119 | Head, statue and terrain bake entry points |

## 2. Scope

This package covers athenaion as a system. It covers these areas:

- The master plan: one layout file holds every fixed position, dimension and heading.
- The heightfield: analytic landform, water signed distance field, terraces, stair profile, far grid.
- The module contract: `build(engine)`, optional `test(engine)`, `engine.add`, `onUpdate`, seeds,
  footprints, slots and the shared registry.
- The architecture kit: Doric, Ionic and Corinthian orders, fluting, entablature, roofs, tholos,
  propylon, stoa, balustrade and ruin fragments.
- Sculpture: signed distance functions, smooth union, the surface net mesher, the statue library,
  the colossal figures, the pediment groups and the fallen head.
- The render pipeline: sub-frame accumulation, jitter, motion blur, depth of field, bloom, AgX tone
  map, local tone mapping, hue secondaries, auto exposure, auto levels and the lens effects.
- The nine shots and the look system: per-shot camera, sun angle, exposure, grading overrides.
- The film: eight segments, camera moves in metres and degrees, cross-dissolves, motion blur.
- The tools: static server, headless render, video render, persistent session, snapshot, colour
  statistics, saturation statistics, sample and bake.
- The quality record: art bible, composition document, contract, prompt, reference comparisons and
  exposure percentiles.

It does not run athenaion. It does not start the server, open a browser, render a pixel, build a mesh,
run a bake, measure a colour or play the film. All runtime numbers come from the project documents and
are marked as not verified.

## 3. Documents

The package uses the standard six-document layout.

| File | Role | Words | Snippets |
|---|---|---:|---:|
| `markdown/README.md` | Index, thesis, map, reading order | no minimum | 3 or more |
| `markdown/00-overview.md` | The problem, the world, the parts, one render pass | 3,000 to 5,000 | 10 or more |
| `markdown/01-architecture.md` | Layer graph, core, world, modules, kits, tools, data | 3,000 to 5,000 | 10 or more |
| `markdown/02-core-mechanism.md` | From seed to pixel, traced end to end | 3,000 to 5,000 | 10 or more |
| `markdown/03-workflow-and-quality.md` | One shot render and one film render, with every check | 3,000 to 5,000 | 10 or more |
| `markdown/04-limits-and-comparison.md` | Limits, rejected choices, failure modes, comparison | 3,000 to 5,000 | 10 or more |

Every deep dive has at least two Mermaid diagrams, at least one `**Aha:**` insight, and a citation
header on every code block. The citation header gives the source path relative to the athenaion root
and the line range.

## 4. Constraints

- The source is read only. No athenaion file is modified.
- No server, browser, render, bake, session, snapshot or statistics command is run.
- No network request is made. No dependency is installed.
- Prose uses Aerospace and Defence Simplified Technical English, issue 100 (ASD-STE100). One idea
  per sentence. Thirty words maximum. Expand every abbreviation at first use in each file.
- Prose avoids marketing language, em dashes, "simply", "just", and "obviously".
- A value that the source does not define is recorded as not defined.
- The pin is `9b8bd23`. Runtime behaviour is marked as not verified.
- A quoted line must match the source byte for byte.
- The exploration does not repeat the build-process product names. It describes the process only as
  the project records it.

## 5. Research questions

1. What problem does a fully procedural world solve, and which fixed data does the plan hold?
2. How does the heightfield combine an analytic landform, a water field, terraces and a stair profile
   into one function that every module calls?
3. How does one module join the world: which members does the engine give it, what must it publish,
   and how do failures stay local?
4. How do footprints and slots let producers and consumers meet without importing each other?
5. How does the architecture kit turn an order, a column count and a size into a temple, a tholos or
   a stoa with correct proportion?
6. How does the signed distance sculpting pipeline turn a figure into a surface, and how is the same
   library used for a 21 metre colossus and a 20 centimetre ornament?
7. How does one still render move from camera to accumulated high dynamic range image to graded
   pixel, and what does each sub-frame change?
8. How do auto exposure and auto levels keep nine different framings in one narrow luminance band?
9. How does the film reuse the shot cameras, and how do camera moves, shutter time and cross-dissolves
   build motion blur?
10. How do the reference images, colour statistics and side-by-side comparisons drive iteration?
11. What does the quality preset change, and how does the project keep an integrated graphics
    processing unit usable?
12. What does the project say it cannot do, and where does the static reading stop?

## 6. Source map for the writer

| Area | Start point |
|---|---|
| Entry page | `index.html` |
| Entry module | `src/main.js` |
| Module order and build loop | `src/world/index.js` |
| Master plan | `src/world/layout.js` |
| Cameras | `src/world/shots.js` |
| Per-shot look | `src/world/looks.js` |
| Heightfield | `src/world/heightfield.js` |
| Engine | `src/core/engine.js` |
| Render pipeline | `src/core/pipeline.js` |
| Quality presets | `src/core/quality.js` |
| Lighting and sun | `src/core/lighting.js` |
| Sky and clouds | `src/core/sky.js` |
| Atmosphere | `src/core/atmosphere.js` |
| Random numbers | `src/core/rng.js` |
| Footprints | `src/core/footprints.js` |
| Materials core | `src/core/materials.js`, `src/core/glsl.js` |
| Terrain mesh | `src/terrain/`, `src/modules/terrain.js` |
| Water and falls | `src/water/`, `src/modules/water.js`, `src/modules/waterfalls.js` |
| Architecture kit | `src/arch/` |
| Temple | `src/arch/temple.js`, `src/modules/temple.js` |
| Acropolis | `src/acropolis/`, `src/modules/acropolis.js` |
| Aqueducts | `src/aqueducts/`, `src/modules/aqueducts.js` |
| Sculpture | `src/sculpt/`, `src/modules/statues.js` |
| Vegetation | `src/vegetation/`, `src/modules/vegetation.js` |
| Flora | `src/flora/`, `src/modules/flora.js` |
| City | `src/city/`, `src/modules/city.js` |
| Ruins | `src/ruins/`, `src/modules/ruins.js` |
| Props | `src/props/`, `src/modules/banners.js` |
| Clouds and birds | `src/modules/clouds.js`, `src/modules/birds.js` |
| Set dressing | `src/sets/`, `src/modules/sets.js` |
| Bake | `src/bake/`, `src/sculpt/bakeutil.js`, `tools/bake.mjs` |
| Video | `src/video/sequence.js`, `src/video/player.js`, `tools/render-video.mjs` |
| Still render tool | `tools/render.mjs` |
| Server | `tools/serve.mjs` |
| Session tool | `tools/session.mjs`, `tools/snap.mjs` |
| Colour statistics | `tools/colorstats.mjs`, `tools/satstats.mjs` |
| Module contract | `docs/CONTRACT.md` |
| Art direction | `docs/ART_BIBLE.md` |
| Composition | `docs/COMPOSITION.md` |
| Build prompt | `docs/PROMPT.md` |

## 7. Task table

| Task | State |
|---|---|
| Verify pin, file count, licence, version, dependencies | DONE |
| Map core, world, modules, kits, tools, data | DONE |
| Write `spec.md` | DONE |
| Write the six documents | DONE (6 documents, 24,075 words, 78 snippets) |
| Snippet check, sentence check, link check | DONE (0 bad snippets, 0 long sentences, 0 broken links) |
| Fresh-reader review | DONE (25 findings reported) |
| Repair reported gaps | DONE (all verified findings repaired) |
| Build HyperText Markup Language (HTML) | DONE (8 pages generated) |
| Commit and push | DONE (no Artificial Intelligence (AI) attribution) |

## 8. Resume point

A later reader starts at `README.md` for the problem and the world, then at `docs/CONTRACT.md` for the
module contract. The world path starts at `src/world/layout.js`, continues in `src/world/heightfield.js`
and ends in `src/world/index.js`. The render path starts at `src/main.js`, moves through
`src/core/engine.js` and `src/core/pipeline.js`, and ends in `tools/render.mjs`. The geometry path
starts at `src/arch/index.js` for architecture and at `src/sculpt/mesher.js` for sculpture. The film
path starts at `src/video/sequence.js` and `src/video/player.js`.

If the checkout moves from `9b8bd23`, run the snippet checker again.

## 9. Limits

- Static reading only. No pixel was rendered and no frame was viewed.
- The reference images and the media images are not compared here. Their numbers come from the code and
  the documents.
- No browser was opened. The WebGL behaviour, the shader compiles and the timings are not verified.
- The far terrain grid in `generated/terrain-far-v11.bin` is binary and was not decoded.
- The bake tools were not run. Their outputs are not checked.
- The build process is a project statement. It is not reproducible here.
- The render harness names Windows browser paths. The checkout was read on macOS, so the harness paths
  do not match this machine.

## 10. Corrections and limits found during research

The inventory matched: pin `9b8bd23`, 220 tracked files, version 0.1.0, MIT. The following findings
qualify the first reading.

1. **The "57 waterfalls" figure is a runtime log value, not a constant.** `src/world/layout.js:154-179`
   lists 17 waterfall specifications. `src/modules/waterfalls.js:1` adds every
   `engine.registry.aqueductSpills` entry. The module logs the final count at
   `src/modules/waterfalls.js:293`. The figure 57 comes from one build run (`README.md:14`).
2. **The "149k trees" figure is a runtime scatter result, not a constant.** The scatter module scales
   the count by `quality.density` and by the `vegDensity` parameter
   (`src/vegetation/scatter.js:54`). A preview build and a high build do not hold the same count.
3. **"168 modules" counts JavaScript files only.** `git ls-files src` holds 168 `.js` files and 4
   `.mjs` helper scripts. The `.mjs` files are preview, visualisation and worker helpers, not world
   modules. The count in `README.md:34` is correct for `.js`.
4. **The film has eight segments, not nine.** The header comment in `src/video/sequence.js:1` says
   "nine segments". The list at `src/video/sequence.js:11-23` holds eight entries. The comment at
   `:20-21` records that shot 3 was dropped because it hung the graphics processing unit in the video
   path. Eight segments at 30 frames per second with seven cross-dissolves give 54.3 seconds, which
   matches the 54 second film in `README.md:33` after rounding.
5. **The podium height has two values.** `src/world/layout.js:28` says a 12 metre podium wall, and the
   same comment gives a stylobate near 67.5 metres above a 54 metre plateau, which is 13.5 metres.
   `docs/COMPOSITION.md` gives 13 metres. The source does not reconcile 12, 13 and 13.5.
6. **The render harness browser paths are Windows paths.** `tools/render.mjs:50-52` and
   `tools/render-video.mjs:37` search `C:/Program Files/...` for Chrome and Edge. The tool has no
   browser-path option, so a reader on another operating system must edit the fixed list. The project
   README names a Windows laptop (`README.md:3`).
7. **The film does not use every shot.** Shot 3 has a camera and a look, but it is absent from
   `SEQUENCE`. The film uses shots 1, 4, 2, 7, 6, 5, 8 and 9 in that order.
8. **The nine shots use different camera distances.** `docs/COMPOSITION.md:20-21` gives 210 to 240
   metres for shots 3, 8 and 9, about 100 to 110 metres for shots 1 and 6, and about 260 metres for
   shot 2. A reader must not assume one camera rig for all shots.
9. **The plan holds a default sun, but each shot changes it.** `src/world/layout.js` defines the
   default sun direction. `src/world/looks.js:54-75` gives every shot its own `sun` value through
   `sunRel`, and also its own sky, exposure and grade. The pipeline reads the per-shot values.
10. **The quality presets trade sample count against density.** `src/core/quality.js:4-27` gives
    preview 1 sample per pixel and density 0.45, high 12 samples and density 1.0, and ultra 48
    samples and density 1.0. The preview also disables ambient occlusion.
11. **The project records a multi-agent build.** `README.md:32` says 55 subagents, up to 12 in
    parallel, and about 147 agent-hours. `docs/PROMPT.md` holds the prompt. The exploration records
    this as a project statement and does not verify it.
12. **The film needs two external items that the repository does not hold.** The music is not in the
    repository (`README.md:81`). The encoder calls `ffmpeg` from the command path
    (`tools/render-video.mjs:147`). The baked grid names also carry their version, as
    `terrain-far-v11.bin` shows (`src/world/heightfield.js:15`), and `loadOrBuild` fetches the name
    without a further version check (`src/core/cache.js:63-66`).

## 11. Verification record

The record fills after the writer and the fresh-reader review.

| Check | Target | Result |
|---|---|---|
| Six documents present | README plus five deep dives | 6 present |
| Words per deep dive | 3,000 to 5,000 | 4,025 / 4,742 / 4,999 / 4,815 / 4,413 |
| Byte-exact snippets | 10 or more per deep dive, 0 bad | 13 / 16 / 16 / 14 / 15, 0 bad; 78 total |
| Mermaid diagrams | 2 or more per deep dive | 2 per deep dive; 10 deep-dive diagrams, 11 with the README |
| `**Aha:**` insights | 1 or more per deep dive | 1 per deep dive |
| Sentence length | 0 sentences over 30 words | 0 |
| Links | 0 broken relative links | 21 checked, 0 bad |
| Fresh-reader review | Gaps repaired or recorded | 25 findings checked against source and repaired |
| Build | HTML pages generated | 8 pages generated |
| Commit and push | No Artificial Intelligence (AI) attribution | DONE |
