---
title: first-bloom package exploration
source_location: /Users/darkvoid/GitSources/src.aidesign/src.leonnix/first-bloom
repository: Git checkout, local pin 713525b
explored_at: 2026-11-16
version: 1.0.0
license: ISC declared in package.json; media licences in CREDITS.md
files: 52
documents: 6
languages: JavaScript, OpenGL Shading Language (GLSL), HyperText Markup Language (HTML), Cascading Style Sheets (CSS), JavaScript Object Notation (JSON), Shell
---

# Specification: first-bloom package exploration

## 1. Source

| Property | Value |
|---|---|
| Location | `/Users/darkvoid/GitSources/src.aidesign/src.leonnix/first-bloom` |
| Kind | Git checkout with one commit |
| Pin | `713525b` (short local commit identifier) |
| Full pin | `713525b52d8541fdcb4e0750cd257e6a24a5d54e` |
| Commit date | 2026-11-09 21:18:50 +0100 |
| Commit subject | `FIRST BLOOM: a 30-second painterly 3D short in three.js` |
| Tracked files | 52, excluding `.git` and `node_modules` |
| Commits | 1 |
| Licence | `package.json` declares ISC. The tree has no licence file. `CREDITS.md` lists the music and sound-effect licences |
| Version | 1.0.0 |
| Runtime | Node.js 20 or newer, Google Chrome, `ffmpeg` on the command path |
| Dependencies | `three` 0.187.0 and `puppeteer-core` 25.16.0 |

`first-bloom` is a 30 second painterly nature film. A frost-covered pasque-flower bud opens on a
frozen meadow. The spring that it releases runs across a mountain basin, thaws a creek and breaks a
frozen waterfall into a double rainbow. From above, the basin is itself a six-petalled flower.

The tree holds 10,445 lines in `src`, 1,694 lines in `audio`, and 1,619 lines in the 19 root files.
The three `docs/` files are JPEG images. The world data and the audio files are not in the tree.
`bake.mjs` builds the data. `audio/fetch_audio.mjs` downloads the library audio from its sources.

The repository has one commit. It adds the film, the render tools, the bake, the audio edit and the
developer guide.

## 2. Scope

The exploration reads the source at the pinned commit. It explains the engine and its subframe
accumulation, the post chain, the world layout and terrain bake, the bloom arrival field, the sky,
the vegetation, the trees, the hero flower, the water and waterfall, the effects and birds, the
camera timeline, the audio cue sheet, the offline render driver and the quality checks.

It does not run the bake, the render, the audio mix or the encode. It does not judge the visual or
musical result. Where the source states a value, the exploration quotes the value and the line.
Where the source does not state a value, the exploration says so.

## 3. Document set

| File | Subject |
|---|---|
| `markdown/README.md` | Thesis, map, document table, reading order, non-claims |
| `markdown/00-overview.md` | The story, the six camera shots, one output frame and the post chain |
| `markdown/01-architecture.md` | Engine, world modules, bake, tools, audio and data flow |
| `markdown/02-core-mechanism.md` | Arrival field, bloom wave, terrain, sky, plants, hero flower, water, rainbow |
| `markdown/03-workflow-and-quality.md` | Bake, look development, camera, render, resume, checker, audio and encode |
| `markdown/04-limits-and-comparison.md` | Limits, costs, hardware bounds and sibling comparison |

Every deep dive has 3,000 to 5,000 words, at least ten byte-exact snippets, at least two Mermaid
diagrams and at least one **Aha:** insight. Every snippet has a `path:line` header and quotes the
source byte for byte. Every document expands each abbreviation at first use.

## 4. Inventory

| Path | Files | Lines | Role |
|---|---:|---:|---|
| `src/engine.js` and `src/main.js` | 2 | 616 | Render engine, subframe loop and page entry |
| `src/glsl/noise.js` | 1 | 44 | Shared noise shader text |
| `src/world/*.js` | 14 | 9,548 | Layout, data, terrain, sky, vegetation, trees, hero flower, water, effects, timeline and helpers |
| `src/worlds/*.js` | 2 | 237 | Film assembly and benchmark scene |
| `audio/` | 11 | 1,694 | Cue sheets, manifests, download tool and music edit |
| Root | 19 | 1,619 | Bake, render, mix, review, server, page, manifest, concept, guide and credits |
| `docs/` | 3 | binary | Contact sheet and two posters |

The large world modules are `engine.js` (593 lines), `fx.js` (1,608), `hero.js` (1,555), `water.js`
(1,528), `vegetation.js` (1,236), `trees.js` (1,061), `sky.js` (1,002) and `terrain.js` (287).
`layout.js` holds the pure geometry functions, and `timeline.js` holds the six camera rigs.

The audio cue sheet holds 46 cues. The music edit is a 30 second cut of "The Light Between Moments"
by MoonpetalMedia. The sound effects are CC0 or Mixkit-licensed recordings listed in `CREDITS.md`.

## 5. Constants

| Constant | Value | Line |
|---|---|---|
| Film length and rate | 30.0 seconds at 30 frames per second, 900 frames | `README.md:5`, `src/world/timeline.js:12-24` |
| Master size | 3,840 by 2,160 pixels, 16:9 | `README.md:5`, `render.mjs:4` |
| Camera shots | 6 rigs: macro, fpv, canopy, reveal, falls, aerial | `src/world/timeline.js:26-33` |
| Story beats | 11 beats in the concept document | `CONCEPT.json` |
| Bloom clock | 0 at the 6.0 second hit, 7,500 at 30.0 seconds | `src/world/timeline.js:41-53` |
| Bloom width | 2.5 plus 0.035 times the distance | `src/world/timeline.js:55` |
| Basin and petal radii | basin 1,500 metres, petal tip 1,150, heart 85 | `src/world/layout.js:51-53` |
| Waterfall | radius 1,720 metres, declared drop 150 metres | `src/world/layout.js:58` |
| Waterfall lip | height 236 metres | `src/world/layout.js:225` |
| Lake and grove | lake radius 165 metres, grove radius 262 | `src/world/layout.js:59-60` |
| Terrain grid | 2,048 by 2,048 cells over 16 kilometres, warped | `src/world/layout.js:274` |
| Falls patch | 1,024 by 1,024 cells over 760 metres | `bake.mjs:27-30` |
| Engine defaults | 8 subframes, 4 multisamples, 2 pixel tiles | `src/engine.js:371` |
| Final render | 16 subframes per frame | `render.mjs:4` |
| Post order | paint, depth of field, overlay, bloom, grade, finish | `src/engine.js:503-592` |
| Audio target | 30 seconds, -14 loudness units full scale | `audio/cues.mjs:8-10` |
| Music level | -19.5 decibels full scale root mean square | `audio/cues.mjs:10` |
| Audio cues | 46 entries | `audio/cues.mjs` |

The source gives several subframe counts for different stages. The engine default is 8. The final
render command uses 16. The developer guide states 8 to 16. The concept document asks for 4 to 8 on
the flight shots. The README states 10 to 32. The exploration quotes each count with its context and
does not merge them into one value.

The waterfall values also conflict. `layout.js` declares a drop of 150 metres. The same file builds
a lip at height 236 metres. The developer guide gives a base near 20 metres. The exploration states
each value and the disagreement. It does not choose one.

## 6. Research questions

| # | Question |
|---|---|
| 1 | How does one flower bud become a basin-wide bloom in one continuous wave? |
| 2 | How does the bake turn terrain and water into an arrival-time field? |
| 3 | How does the engine turn jittered subframes into anti-aliasing and true motion blur? |
| 4 | Why does the post chain paint before it blurs, and how does each pass read depth? |
| 5 | How do six camera rigs carry eleven story beats on a music-locked timeline? |
| 6 | How are the creek reflection, waterfall and double rainbow made without a reflection pass? |
| 7 | What does the film cost in time, memory and machine load, and what limits does it state? |

## 7. Constraints

- Aerospace and Defence Simplified Technical English (ASD-STE100) applies to every sentence.
- The source is read-only. No file in the source tree is changed.
- No network access. No dependency is installed. No bake, render, mix or encode runs.
- One package is in flight. Two subagents work on this package: one writer and one fresh reader.
- A fresh-reader review runs before the package is done. The writer fixes every reported gap that
  the source supports.

## 8. Task table

| # | Task | State |
|---|---|---|
| 1 | Write `spec.md` | DONE |
| 2 | Write the six documents | DONE (6 documents, 19,873 prose words) |
| 3 | Validate snippets against the source | DONE (88 blocks, 0 mismatches) |
| 4 | Validate Simplified Technical English | DONE (0 long sentences in 6 files) |
| 5 | Validate links | DONE (10 links, 0 broken) |
| 6 | Fresh-reader review and repair | DONE (34 findings: 31 fixed, 2 partial, 1 reader error) |
| 7 | Build HyperText Markup Language (HTML) | DONE (8 pages and one stylesheet) |
| 8 | Update the container index and spec | DONE (16 of 19 in `index.md`, `spec.md` and the container README) |
| 9 | Commit and push | DONE (`9a8526b`) |

## 9. Resume point

Read the task table. The first `TODO` row is the next action. A task is complete only when its
check passes and the result is recorded in the verification table.

## 10. Verification record

| Check | Result |
|---|---|
| Documents | 6 files, 19,873 prose words: README 1,895; 00-overview 3,534; 01-architecture 3,147; 02-core-mechanism 4,847; 03-workflow-and-quality 3,175; 04-limits-and-comparison 3,275 |
| Snippets | 88 blocks, 0 mismatches against the pinned source |
| Diagrams and insights | 11 Mermaid diagrams, 9 **Aha:** insights |
| Simplified Technical English | 0 long sentences in 6 files |
| Links | 10 checked, 0 broken |
| Fresh-reader review | 34 findings adjudicated: 31 repaired, 2 partly repaired, 1 reader error |
| Build | DONE: 8 HTML pages and one stylesheet in `html/` |
| Commit | DONE: `9a8526b`, pushed to `origin/master` |
