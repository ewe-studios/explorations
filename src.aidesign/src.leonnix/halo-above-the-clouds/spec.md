---
title: halo-above-the-clouds package exploration
source_location: /Users/darkvoid/GitSources/src.aidesign/src.leonnix/halo-above-the-clouds
repository: Git checkout, local pin 1d08819
explored_at: 2026-11-16
version: 1.0.0
license: MIT
files: 55
documents: 6
languages: JavaScript, OpenGL Shading Language (GLSL), HyperText Markup Language (HTML), Cascading Style Sheets (CSS), JavaScript Object Notation (JSON), PowerShell
---

# Specification: halo-above-the-clouds package exploration

## 1. Source

| Property | Value |
|---|---|
| Location | `/Users/darkvoid/GitSources/src.aidesign/src.leonnix/halo-above-the-clouds` |
| Kind | Git checkout with its own history |
| Pin | `1d08819` (short local commit identifier) |
| Full pin | `1d08819aea2381cc3738da6fe19f33931fcfb409` |
| Commit date | 2026-11-02 13:02:41 +0100 |
| Commit subject | `Fix README facts, skip the matte without data, key fixes` |
| Tracked files | 55, excluding `.git` and `node_modules` |
| Commits | 2 |
| Licence | MIT License, Copyright 2026 Leon Lin |
| Version | 1.0.0 private package |
| Runtime | Node.js 22.12 or newer, a browser with WebGL2 |
| Dependencies | `three` 0.187.0, `puppeteer-core` 25.16.0, `sharp` 0.35.5, `vite` 8.3.5 |

`halo-above-the-clouds` is a real-time Three.js scene and a 45 second drone film. A concrete ring
restaurant stands between snowy dolomite pinnacles above a sea of clouds. The same code base
serves two outputs. A browser opens `index.html` and draws the scene live. A headless Chrome
process renders 1,350 frames and encodes the film.

The tree holds 9,780 lines in 55 tracked files. The source is 12 JavaScript scene modules
(4,345 lines), five GLSL modules (3,421 lines), 27 tool scripts (4,858 lines), and seven root
files. The four `docs/` files are JPEG images for the README page.

The repository has two commits. The first adds the scene, the film and the tools. The second
fixes the README facts, skips the matte when no data exists, and repairs key handling.

## 2. Scope

The exploration reads the source at the pinned commit. It explains the renderer bootstrap, the
configuration, the terrain level pipeline, the far-field terrain key, the sky bake, the cloud
raymarch, the ring structure, the lights, the camera timeline and views, the post chain, the film
render driver, the graphics processing unit (GPU) lock, the film checker, and the data fetch.

It does not run the scene, render a frame, encode the film or download data. It does not judge the
visual result. Where the source states a value, the exploration quotes the value and the line.
Where the source does not state a value, the exploration says so.

## 3. Document set

| File | Subject |
|---|---|
| `markdown/README.md` | Thesis, map, document table, reading order, non-claims |
| `markdown/00-overview.md` | The place, the scene, the film and one frame |
| `markdown/01-architecture.md` | Scene modules, GLSL modules, tools, data flow and render loop |
| `markdown/02-core-mechanism.md` | Terrain levels, cloud march, ring geometry, sky, lights and post |
| `markdown/03-workflow-and-quality.md` | Data fetch, bake, look development, timeline, render, check and encode |
| `markdown/04-limits-and-comparison.md` | Limits, costs, hardware bounds and sibling comparison |

Every deep dive has 3,000 to 5,000 words, at least ten byte-exact snippets, at least two Mermaid
diagrams and at least one **Aha:** insight. Every snippet has a `path:line` header and quotes the
source byte for byte. Every document expands each abbreviation at first use.

## 4. Inventory

| Path | Files | Lines | Role |
|---|---:|---:|---|
| `src/*.js` | 12 | 4,345 | Bootstrap, config, timeline, views, post, sky, clouds, terrain, structure, lights, texture, matte |
| `src/glsl/` | 5 | 3,421 | Common noise and atmosphere, far terrain, rock, water and structure shaders |
| `tools/` | 27 | 4,858 | Terrain build, data fetch, sky bake, light bake, matte, render, GPU lock, checker and helpers |
| Root | 7 | 157 | `index.html`, `main` module, `package.json`, lock file, Vite config, README, licence |
| `docs/` | 4 | binary | Four JPEG stills for the README |

The scene modules are `main.js` (246 lines), `config.js` (99), `timeline.js` (266), `views.js` (39),
`post.js` (211), `sky.js` (85), `lights.js` (352), `clouds.js` (1,353), `terrain.js` (474),
`structure.js` (985), `tex.js` (25) and `matte.js` (210).

The GLSL modules hold the shader strings. `common.js` holds the noise and atmosphere library,
`far.js` the far terrain and aerial perspective, `rock.js` the rock surface, `water.js` the lake and
`structure.js` the ring and bridge surfaces.

The tools include `terrain.mjs` (the level build), `farmid.mjs`, `farsummit.mjs`, `fardem.mjs`,
`farkey.mjs`, `skybake.mjs`, `lights.mjs`, `noise*.mjs`, `matte_*.mjs`, `render.mjs`, `gpulock.mjs`,
`shot.mjs`, `filmcheck.mjs`, `stack.mjs`, `sheet.mjs`, `gradetune.mjs`, `loudness.mjs`,
`fetch-data.mjs` and `encode.ps1`.

## 5. Constants

| Constant | Value | Line |
|---|---|---|
| Deck height | 2,180 metres | `src/config.js:5` |
| Lake height | 564 metres | `src/config.js:6` |
| Ring radii | outer 55 metres, inner 42.6 metres | `src/config.js:17-18` |
| Cloud deck | base 1,500 metres, top 1,780 metres | `src/config.js:29-32` |
| Hero view azimuth | 210 degrees | `src/config.js:7` |
| Sun azimuth and elevation | 214 degrees and -4.0 degrees | `src/config.js:22-23` |
| Earth radius | 6,371,000 metres | `src/config.js:13` |
| Film length | 45.0 seconds | `src/timeline.js:207` and `:215` |
| Film frame rate | 30 frames per second, 1,350 frames | `src/timeline.js:23` |
| Film shots | 11 free-flight shots | `src/timeline.js:147-209` |
| Picture fade | 0.6 seconds in, 1.8 seconds out | `src/timeline.js:264` |
| Terrain levels | L0 320 kilometres at 1,025 cells, L1 50 kilometres at 1,601, L2 8 kilometres at 1,281, L3 1.6 kilometres at 1,025, L4 0.39 metres | `tools/terrain.mjs:14-19` |
| Far polar map | 240 kilometres | `tools/fardem.mjs:1-12` |
| Mid-level map | plus or minus 12 kilometres at 15.625 metres | `tools/farmid.mjs:1-10` |
| Cloud march | 300 steps, half resolution for the film | `src/clouds.js:1210`, `src/config.js:97` |
| Lights | about 117,000 village and road lights | `README.md:62` |
| Post grade | exposure 0.69, shoulder 0.6, toe 0.06, clarity 0.22 | `src/post.js:26-28` |
| Bloom | seven mip levels | `src/post.js:153` |

## 6. Research questions

| # | Question |
|---|---|
| 1 | How does the scene build a mountain range that spans 320 kilometres without a mesh for every cell? |
| 2 | How does the cloud marcher test depth against terrain and the ring, and how does it fade into the sky? |
| 3 | How is the sky baked from two photographic panoramas into a small log-encoded texture? |
| 4 | How does the ring restaurant sit on a pinnacle and cast light into the cloud deck? |
| 5 | How does one timeline drive both the live scene and the offline film? |
| 6 | Why does the offline renderer need a machine-wide GPU lock? |
| 7 | What does the project cost in time, memory and data, and what hardware does it require? |

## 7. Constraints

- Aerospace and Defence Simplified Technical English (ASD-STE100) applies to every sentence.
- The source is read-only. No file in the source tree is changed.
- No network access. No dependency is installed. No render, bake or encode runs.
- One package is in flight. Two subagents work on this package: one writer and one fresh reader.
- A fresh-reader review runs before the package is done. The writer fixes every reported gap that
  the source supports.

## 8. Task table

| # | Task | State |
|---|---|---|
| 1 | Write `spec.md` | DONE |
| 2 | Write the six documents | DONE (6 documents, 21,430 prose words) |
| 3 | Validate snippets against the source | DONE (74 blocks, 0 mismatches) |
| 4 | Validate Simplified Technical English | DONE (0 long sentences in 6 files) |
| 5 | Validate links | DONE (10 links, 0 broken) |
| 6 | Fresh-reader review and repair | DONE (35 findings: 31 fixed, 4 clarified) |
| 7 | Build HyperText Markup Language (HTML) | DONE (8 pages and one stylesheet) |
| 8 | Update the container index and spec | IN PROGRESS (15 of 19 nested projects) |
| 9 | Commit and push | TODO |

## 9. Resume point

Read the task table. The first `TODO` row is the next action. A task is complete only when its
check passes and the result is recorded in the verification table.

## 10. Verification record

| Check | Result |
|---|---|
| Documents | 6 files, 21,430 prose words: README 1,447; 00-overview 3,213; 01-architecture 3,546; 02-core-mechanism 3,887; 03-workflow-and-quality 4,993; 04-limits-and-comparison 4,344 |
| Snippets | 74 blocks, 0 mismatches against the pinned source |
| Diagrams and insights | 12 Mermaid diagrams, 9 **Aha:** insights |
| Simplified Technical English | 0 long sentences in 6 files |
| Links | 10 checked, 0 broken |
| Fresh-reader review | 35 findings adjudicated: 31 repaired, 4 clarified from the source |
| Build | 8 HyperText Markup Language pages and one stylesheet, generated by `src.aidesign/build.py` |
| Commit | Pending |
