---
title: room-at-dawn package exploration
source_location: /Users/darkvoid/GitSources/src.aidesign/src.leonnix/room-at-dawn
repository: Git checkout, local pin aab8be2
explored_at: 2026-11-16
version: 1.0.0
license: MIT
files: 48
documents: 6
languages: JavaScript, Graphics Library Shading Language (GLSL), HyperText Markup Language (HTML), Cascading Style Sheets (CSS), JavaScript Object Notation (JSON), PowerShell
---

# Specification: room-at-dawn package exploration

## 1. Source

| Property | Value |
|---|---|
| Location | `/Users/darkvoid/GitSources/src.aidesign/src.leonnix/room-at-dawn` |
| Kind | Git checkout |
| Pin | `aab8be2` (short local commit identifier) |
| Full pin | `aab8be218aa051bbb2e22b82916962619c951747` |
| Commit date | 2026-11-12 15:18:13 +0100 |
| Commit subject | `Flatter duvet, room set dressing, sound, sturdier film renderer` |
| Tracked files | 48, excluding `.git` and `node_modules` |
| Licence | MIT for code. Sound and music have their own licences in `CREDITS.md` |
| Version | 1.0.0 |
| Runtime | Node.js, Vite 8, Google Chrome or Microsoft Edge, `ffmpeg` for the film |
| Dependencies | `three` 0.187.0, `three-mesh-bvh` 0.9.18, `n8ao` 2.0.1, `puppeteer-core` 25.17.0 |

`room-at-dawn` is a photoreal bedroom at first light, built completely from code. No model file, no
image texture and no high-dynamic-range image (HDRI) is in the tree. The room is a live first-person
walk in three.js. The same scene also renders a 45 second film at 3,840 by 2,160 pixels in high
dynamic range 10 (HDR10, perceptual quantizer over BT.2020, 10-bit HEVC).

The tree holds 10,086 lines of JavaScript under `src`, 268 lines of JavaScript tooling (`vite.config.js`
and the four `.mjs` tools), 14 lines of PowerShell, and seven JPEG frames in `docs/media`. The
lighting bake is a 13.06 megabyte binary at `public/bake/gi.bin`. Two recorded audio files sit in
`public/audio`.

## 2. Scope

The exploration reads the source at the pinned commit. It explains the room shell and its parts, the
procedural geometry and texture helpers, the seeded random number generator, the central processing
unit (CPU) path tracer and its workers, the per-vertex global illumination plumbing and its shader
patch, the box-projected reflection capture, the high dynamic range render pipeline, the film
timeline and its sub-frame accumulation, the live game and its adaptive resolution, the development
server bridge to `ffmpeg`, and the four tools.

It does not run the development server, the bake, the film render or the encode. It does not judge
the visual result. Where the source states a value, the exploration quotes the value and the line.
Where the source does not state a value, the exploration says so.

## 3. Document set

| File | Subject |
|---|---|
| `markdown/README.md` | Thesis, map, document table, reading order, non-claims |
| `markdown/00-overview.md` | The room, the film, the two run modes, one frame and one bake |
| `markdown/01-architecture.md` | Modules, processes, the scene graph contract, the bake and render paths |
| `markdown/02-core-mechanism.md` | The bake: the portal, the two passes, the sample graph, the buffer format |
| `markdown/03-workflow-and-quality.md` | Write a part, bake it, check it, render stills, render the film, resume, encode |
| `markdown/04-limits-and-comparison.md` | Limits, costs, hardware bounds and sibling comparison |

Every deep dive has 3,000 to 5,000 words, at least ten byte-exact snippets, at least two Mermaid
diagrams and at least one **Aha:** insight. Every snippet has a `path:line` header and quotes the
source byte for byte. Every document expands each abbreviation at first use.

## 4. Inventory

| Path | Files | Lines | Role |
|---|---:|---:|---|
| `src/core/` | 4 | 556 | Seeded random number generator, geometry helpers, texture helpers, baked-light plumbing |
| `src/bake/` | 2 | 494 | Bake driver on the main thread and the path-tracing worker |
| `src/scene/` | 5 | 762 | Layout constants, assembly, room shell, sky, material library |
| `src/scene/parts/` | 7 | 7,489 | Bed, fixtures, desk, plants, clutter, decor, exterior |
| `src/render/` | 4 | 528 | Pipeline, film, look, named cameras |
| `src/game/` | 1 | 115 | First-person walk and the attract loop |
| `src/main.js` | 1 | 142 | Boot, bake load, reflection capture, public application programming interface (API) |
| `tools/` | 5 | 214 | Shot, film, profile, evaluate, PowerShell system check |
| Root | 8 | 1,530 | `vite.config.js` with the `ffmpeg` bridge, page, manifest, lock file, guide, credits, licence, ignore list |
| `docs/` | 8 | 71 | The architecture guide and seven JPEG frames |
| `public/` | 3 | binary | The bake file and two audio files |

The part modules are `desk.js` (1,660 lines), `fixtures.js` (1,462), `bed.js` (1,141),
`exterior.js` (912), `clutter.js` (845), `decor.js` (845) and `plants.js` (624). Their load order is
fixed: exterior 5, fixtures 10, bed 20, desk 30, plants 40, clutter 60, decor 70.

The shipped bake records 302 lit meshes, 1,157,291 unique sample points and 1,742,198 occluder
triangles. It was baked at 160 samples per point in 1,808 seconds.

## 5. Constants

| Constant | Value | Line |
|---|---|---|
| Room size | 4.8 by 4.0 by 2.5 metres, wall 0.32 thick | `src/scene/layout.js:3` |
| Window opening | x 0.55 to 2.45, y 0.86 to 2.26 | `src/scene/layout.js:4` |
| Scene seed | 20,261,112 | `src/scene/index.js:13` |
| Bake depth | 9 path vertices | `src/scene/index.js:33` |
| Bake job size | 3,000 sample points | `src/bake/baker.js:118` |
| Bake default samples | 128 per point, 6 workers | `src/bake/baker.js:18` |
| Bake first pass | one fifth of the target, 12 to 48 samples | `src/bake/baker.js:182` |
| Bake smoothing | default one edge-aware iteration | `src/bake/baker.js:18` |
| Worker environment map | 128 by 96 cells, 40 shots per cell | `src/bake/worker.js:188-190` |
| Envelope magic | `0x47494231` (`GIB1`) | `src/core/gi.js:160` |
| Bake horizon | pixels at z below -0.33 take the exterior scale | `src/core/gi.js:96` |
| Live exposure | 19 | `src/render/look.js:5` |
| Exterior share, SDR and HDR10 | 0.21 and 0.55 | `src/render/look.js:6-7` |
| Bloom levels | 6 | `src/render/pipeline.js:216` |
| Live multisample count | 4 | `src/render/pipeline.js:199` |
| Film size and rate | 3,840 by 2,160 at 30 frames per second | `src/render/film.js:109` |
| Film length | 45.0 seconds, seven shots | `src/render/film.js:13-36` |
| Film default samples | 16 per frame, scaled by shot | `src/render/film.js:109` |
| Shutter | 180 degrees, half a frame | `src/main.js:90` |
| Live resolution cap | 1,600 pixels wide, scale floor 0.5 | `src/game/game.js:13,108` |
| HDR10 peak | 1,000 nits, 203 nits reference white | `src/render/look.js:13`, `src/render/pipeline.js:167` |

The shot table holds seven entries: wake 8.5 seconds, sill 7.0, leaves 5.5, duvet 6.0, macro 5.5,
desk 5.0 and room 7.5. Each shot stores two camera keys with position, target, field of view, focus
point and aperture radius. The sub-frame count per shot ranges from 32 to 96.

Two exposure controls coexist. `LOOK.exposure` is 19 for the live view. Each shot has its own `exp`
multiplier from 0.62 to 1.5. `applyFilmCamera` multiplies the two. The exploration states both
values and does not merge them.

The bake file records 160 samples per point. The default in the driver is 128. The README example
uses 256. The exploration quotes each number with its context.

## 6. Research questions

| # | Question |
|---|---|
| 1 | How does a scene with no light objects still look lit? |
| 2 | How does the bake find light in a sealed room with one opening? |
| 3 | Why is irradiance stored per vertex, and what does the second attribute buy? |
| 4 | How do the two bake passes and the sample graph remove noise without a de-noiser library? |
| 5 | How does one geometry build serve both a live walk and a 4K high-dynamic-range film? |
| 6 | How do seeded randomness and fixed load order keep the baked file valid across runs? |
| 7 | What does the project state about cost, hardware and its own limits? |

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
| 2 | Write the six documents | DONE (6 documents, 20,314 prose words) |
| 3 | Validate snippets against the source | DONE (80 blocks, 0 mismatches) |
| 4 | Validate Simplified Technical English | DONE (0 long sentences in 6 files) |
| 5 | Validate links | DONE (5 links, 0 broken) |
| 6 | Fresh-reader review and repair | DONE (61 findings: 59 repaired, 2 rejected with the text clarified) |
| 7 | Build HyperText Markup Language (HTML) | DONE (8 pages and one stylesheet) |
| 8 | Update the container index and spec | DONE (17 of 19 in `index.md`, `spec.md` and the container README) |
| 9 | Commit and push | DONE (`a30c8e7`) |

## 9. Resume point

Read the task table. The first `TODO` row is the next action. A task is complete only when its
check passes and the result is recorded in the verification table.

## 10. Verification record

| Check | Result |
|---|---|
| Documents | 6 files, 20,314 prose words: README 1,564; 00-overview 3,347; 01-architecture 3,328; 02-core-mechanism 3,763; 03-workflow-and-quality 3,316; 04-limits-and-comparison 4,996. A strict count that drops table rows gives 19,530 |
| Snippets | 80 blocks, 0 mismatches against the pinned source |
| Diagrams and insights | 11 Mermaid diagrams, 6 **Aha:** insights |
| Simplified Technical English | 0 long sentences in 6 files |
| Links | 5 checked, 0 broken |
| Fresh-reader review | 61 findings adjudicated: 1 blocker, 13 major, 47 minor. 59 repaired, 2 rejected with the text clarified |
| Build | DONE: 8 HTML pages and one stylesheet in `html/` |
| Commit | DONE: `a30c8e7`, pushed to `origin/master` |
