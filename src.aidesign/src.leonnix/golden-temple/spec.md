---
title: golden-temple package exploration
source_location: /Users/darkvoid/GitSources/src.aidesign/src.leonnix/golden-temple
repository: Git checkout, local pin 97fd9ec
explored_at: 2026-11-16
version: 1.0.0
license: MIT
files: 121
documents: 6
languages: JavaScript, Graphics Library Shading Language (GLSL), HyperText Markup Language (HTML), Cascading Style Sheets (CSS), Markdown, JavaScript Object Notation (JSON)
---

# Specification: golden-temple package exploration

## 1. Source

| Property | Value |
|---|---|
| Location | `/Users/darkvoid/GitSources/src.aidesign/src.leonnix/golden-temple` |
| Kind | Git checkout with its own history |
| Pin | `97fd9ec` (short local commit identifier) |
| Tracked files | 121 |
| Version | `1.0.0` (`package.json:3`) |
| Licence | MIT (`LICENSE:1`) |
| Runtime | Browser with WebGL 2, driven by three.js 0.186.1 |
| Build tool | Vite 8.3.4 for the interactive page; no build step for the film |
| Server | `vite --port 5173 --strictPort` |
| Capture tool | Puppeteer Core 25.13.0; it uses an installed Chrome or Edge |
| Encoder | ffmpeg, an external program on the path |
| Scripts | `dev`, `build`, `snap`, `sheet`, `render`, `fetch-music`, `score`, `camcheck` and `flicker` (`package.json:30-40`) |
| Entry page | `index.html` loads `/src/main.js` as a module (`index.html:23`) |
| Entry module | `src/main.js` reads the query string and selects one of three modes |
| Output | One 70 second film at 1920 by 1080 pixels and 60 frames per second |

Golden Temple means a Japanese temple complex at the golden hour. The package builds the
complex in code. It loads no three-dimensional model, texture file, high dynamic range image
(HDRI) or photograph at runtime. JavaScript creates the buildings, roof tiles, bracket sets,
gold fittings, statues, trees, lotus, koi, water, terrain, clouds and sky. Graphics Library
Shading Language (GLSL) code supplies every surface detail. A few textures are drawn onto
canvases at load time.

The world uses metres. Positive X is east, positive Y is up and positive Z is south, so north
is negative Z. The courtyard ground is at Y equals zero. The site plan holds the hall at
(10, 0, -22), the pagoda at (-32, 0, -8), the gate at (10, 0, 46) and the pond outline. The sun
stands 6.8 degrees above the horizon in the west-south-west.

The film has ten shots of seven seconds each, for 70 seconds in total. Each shot opens on one
detailed object and moves back to show the complex around it. Every frame is a set of sub-frames.
Each sub-frame has a different camera position, sun position and shutter time. The average
gives anti-aliasing, motion blur, soft sun shadows and clean ambient occlusion.

The tracked files divide like this:

| Path | Files | Role |
|---|---:|---|
| `src` | 87 | Engine, site plan, shots, world modules and camera overrides |
| `src/core` | 9 | Sky, fog, lighting, post pipeline, materials, geometry, shaders, random numbers |
| `src/world` | 66 | Nine world modules and their helper files |
| `src/shotkeys` | 9 | One index plus eight module camera files; one file is empty |
| `tools` | 11 | Stills, contact sheets, film renderer, music, splice and motion checks |
| `docs` | 14 | One architecture guide and thirteen film stills |
| Root files | 9 | Page shell, Vite config, manifest, lock file, licence, credits, readme and ignore files |

The `src` tree holds 31,299 JavaScript lines. The `tools` tree holds 612 module lines. The code
divides like this:

| Folder | Files | JavaScript lines | Role |
|---|---:|---:|---|
| `src` root | 3 | 790 | Entry module, site plan and shot list |
| `src/core` | 9 | 1,880 | Renderer, sky, fog, light, post, materials, geometry and random numbers |
| `src/world` | 66 | 28,327 | Nine world modules and their helper files |
| `src/shotkeys` | 9 | 302 | Module-owned camera overrides for the hero shots |

The nine world modules divide like this:

| Module | Files | Lines | Contents |
|---|---:|---:|---|
| `terrain` | 11 | 3,800 | Ground, raked gravel, moss, paving, pond bed, hills, backdrop forest and cover |
| `pond` | 10 | 4,439 | Planar-reflection water, stone edge, island, lotus, koi and floating debris |
| `hall` | 12 | 3,646 | Two-tier hall exterior, karahafu porch, podium, columns, brackets and gold |
| `interior` | 11 | 3,478 | Amida Buddha, throne, mandorla, altar, canopies, ceiling, shafts and smoke |
| `pagoda` | 3 | 1,711 | Five-storey pagoda, side-local generation and parts |
| `bridge` | 1 | 1,517 | Arched vermilion bridge, deck, arch, railings, giboshi and piers |
| `buildings` | 7 | 3,220 | Pavilion, corridor, gate, bell tower and side halls |
| `props` | 6 | 2,650 | Stone lanterns, hanging lanterns, bells, burner, basin and stepping stones |
| `vegetation` | 5 | 3,866 | Pines, maples, cherries, shrubs, ferns, bark, leaf atlases and drift |

The `src/world` count includes one test-only file, `interior-testshell.js`. The interior module
loads it only when the query string holds `ishell=1`.

## 2. Scope

This package covers Golden Temple as a system. It covers these areas:

- The visual idea: a low sun, a warm stone court, vermilion lacquer, gold leaf and a mirror pond.
- The engine: sky, aerial perspective, ambient volumes, image-based light, shadow fitting and the
  post pipeline.
- The materials: the `patch()` hook system and the shared procedural materials.
- The world modules: nine `build(ctx)` modules, their contracts, their helpers and their data
  exchange.
- The shots: ten camera definitions, camera overrides, easing, smoothing and focus.
- The frame: sub-frame sampling, the stability rule, anti-aliasing, motion blur, shadows and
  ambient occlusion.
- The film path: headless Chrome, raw pixel transfer, ffmpeg, the music edit and the splice tool.
- The quality tools: camera motion, brightness flicker, motion-compensated shimmer and contact
  sheets.
- The limits: absent assets, runtime-only values, stale comments and tool assumptions.

It does not run Golden Temple. It does not start Vite, open a browser, build the world, compile
a shader, render a frame, encode a film, fetch music or measure a metric. All runtime numbers
come from the project documents and are marked as not verified.

## 3. Documents

The package uses the standard six-document layout.

| File | Role | Words | Snippets |
|---|---|---:|---:|
| `markdown/README.md` | Index, thesis, map, reading order | no minimum | 3 or more |
| `markdown/00-overview.md` | The site, the ten shots, the parts and one frame | 3,000 to 5,000 | 10 or more |
| `markdown/01-architecture.md` | Modules, engine layers, contracts, data flow and ownership | 3,000 to 5,000 | 10 or more |
| `markdown/02-core-mechanism.md` | From query string to a graded pixel, traced end to end | 3,000 to 5,000 | 10 or more |
| `markdown/03-workflow-and-quality.md` | One still from first view to accepted frame, then the film | 3,000 to 5,000 | 10 or more |
| `markdown/04-limits-and-comparison.md` | Limits, failure modes, inconsistencies and comparisons | 3,000 to 5,000 | 10 or more |

Every deep dive has at least two Mermaid diagrams, at least one `**Aha:**` insight, and a
citation header on every code block. The citation header gives the source path relative to the
golden-temple root and the line range.

## 4. Constraints

- The source is read only. No golden-temple file is modified.
- No server, browser, still capture, world build, shader compile, frame render, film encode or
  music download is run.
- No network request is made. No dependency is installed.
- Prose uses Aerospace and Defence Simplified Technical English, issue 100 (ASD-STE100). One idea
  per sentence. Thirty words maximum. Expand every abbreviation at first use in each file.
- Prose avoids marketing language, em dashes, "simply", "just", and "obviously".
- A value that the source does not define is recorded as not defined.
- The pin is `97fd9ec`. Runtime behaviour is marked as not verified.
- A quoted line must match the source byte for byte.
- The exploration does not repeat the build-process product names.

## 5. Research questions

1. What does a "photoreal" scene without assets require, and how does one site plan keep nine
   independent modules in agreement?
2. How does the startup sequence patch shaders, build modules, collect anchors, bake ambient
   volumes, compile programs and capture light?
3. How does the `build(ctx)` contract work, and what may a module return and read from other
   modules?
4. How does the `patch()` system inject GLSL into three.js physical materials, and how does the
   program cache stay small?
5. How does one frame become eight sub-frames, and why does the sampling depend only on the
   sub-frame index?
6. How does the post pipeline use HDR targets, multi-sample anti-aliasing, ambient occlusion,
   depth of field, bloom and a mixed tone mapper?
7. How does the shot system interpolate position, direction, focus, lens and easing, and how do
   module-owned overrides replace base keys?
8. How do the nine world modules build their geometry, and how do they exchange fields and
   anchors?
9. How does the film path move raw pixels from the graphics processing unit (GPU) to ffmpeg
   without an intermediate file?
10. How do the quality tools measure camera motion and temporal flicker, and what does each
    metric isolate?
11. How does the music edit align two musical hits with the cuts at 28 and 42 seconds?
12. Where does the static reading stop, and which claims depend on a render that this exploration
    did not perform?

## 6. Source map for the writer

| Area | Start point |
|---|---|
| Project statement | `README.md` |
| Architecture guide | `docs/ARCHITECTURE.md` |
| Entry page | `index.html` |
| Entry module, query string, build order, frame loop | `src/main.js` |
| Site plan | `src/layout.js` |
| Shot list and camera evaluator | `src/shots.js` |
| Module camera overrides | `src/shotkeys/index.js` and `src/shotkeys/<module>.js` |
| Environment constants and global uniforms | `src/core/env.js` |
| Seeded random numbers and noise | `src/core/rng.js` |
| Shared GLSL noise library | `src/core/glsl.js` |
| Geometry helpers and geometry merger | `src/core/geom.js` |
| Sky model | `src/core/sky.js` |
| Aerial perspective and ambient volumes | `src/core/fog.js` |
| Sun, shadow fitting and environment capture | `src/core/lighting.js` |
| Post pipeline | `src/core/post.js` |
| Material patch system and procedural materials | `src/core/materials.js` |
| Terrain module | `src/world/terrain.js` and `src/world/terrain-*.js` |
| Pond module | `src/world/pond.js` and `src/world/pond-*.js` |
| Hall module | `src/world/hall.js` and `src/world/hall-*.js` |
| Interior module | `src/world/interior.js` and `src/world/interior-*.js` |
| Pagoda module | `src/world/pagoda.js`, `pagoda-parts.js`, `pagoda-mat.js` |
| Bridge module | `src/world/bridge.js` |
| Buildings module | `src/world/buildings.js` and `src/world/buildings-*.js` |
| Props module | `src/world/props.js` and `src/world/props-*.js` |
| Vegetation module | `src/world/vegetation.js` and `src/world/vegetation-*.js` |
| Still capture tool | `tools/snap.mjs` |
| Contact sheet tool | `tools/sheet.mjs` |
| Film renderer | `tools/render.mjs` |
| Browser detection and GPU flags | `tools/lib/chrome.mjs` |
| Music download and checksum | `tools/fetch-music.mjs` |
| Music edit and mux | `tools/score.mjs` |
| Segment splice | `tools/splice.mjs` |
| Camera motion check | `tools/camcheck.mjs` |
| Flicker metrics | `tools/lumaflicker.mjs`, `tools/mcflicker.mjs`, `tools/flickercheck.mjs` |
| Stills used in the readme | `docs/images/` |

## 7. Task table

| Task | State |
|---|---|
| Verify pin, file count, licence, version, dependencies | DONE |
| Map source, tools, docs and absent outputs | DONE |
| Write `spec.md` | DONE |
| Write the six documents | TODO |
| Snippet check, sentence check, link check | TODO |
| Fresh-reader review | TODO |
| Repair reported gaps | TODO |
| Build HyperText Markup Language (HTML) | TODO |
| Commit and push | TODO |

## 8. Resume point

A later reader starts at `README.md` for the film description and the tool commands, then at
`docs/ARCHITECTURE.md` for the frame pipeline. The frame path starts at `index.html`, moves through
`src/main.js`, `src/shots.js` and `src/core/post.js`, and ends in `src/world/*` shaders. The world
path starts at `src/layout.js`, moves through `src/world/terrain.js` and the other eight modules,
and ends in `src/shotkeys/`. The film path starts at `tools/render.mjs` and ends at
`tools/score.mjs` and ffmpeg.

If the checkout moves from `97fd9ec`, run the snippet checker again.

## 9. Limits

- Static reading only. No frame was rendered, no film was viewed and no metric was measured.
- The film is not in the repository. `.gitignore` ignores `*.mp4` and other media files.
- The music is not in the repository. `tools/fetch-music.mjs` downloads it from the network.
- The reference images are not in the repository. `.gitignore` ignores `ref/`.
- The render output folder `out/` is ignored. The checkout holds no render input or render output.
- No browser was opened. WebGL 2 behaviour, shader compilation, worker behaviour and timings
  are not verified.
- The texture atlases and terrain masks were not drawn or measured.
- The tools were not run. Their outputs are not checked.
- The build process is not reproducible here. The page needs a browser and a graphics card.

## 10. Corrections and limits found during research

The inventory matched: pin `97fd9ec`, 121 tracked files, version 1.0.0 and MIT. The following
findings qualify the first reading.

1. **The line count is higher than the rounded claim.** `README.md:15` says "About 31,000 lines
   of JavaScript". The `src` tree holds 31,299 lines and the `tools` tree holds 612, for 31,911
   lines in total. The rounded claim is accurate for `src` alone.
2. **The triangle count is a runtime value.** `README.md:15` states 24.4 million triangles. The
   value comes from `window.__info.tris`, which `src/main.js:311-315` computes after the world is
   built. No static file states the count. This exploration did not build the world, so the value
   is not verified.
3. **"Every shot opens on a macro close-up" is a design intent, not a lens rule.**
   `README.md:17` makes the claim. The first key of each shot has focus distances of 1.05, 0.6,
   0.9, 1.35, 1.3, 4.0, 2.8, 1.0, 3.0 and 1.1 metres. Shot 6 opens at 4 metres with a 28
   millimetre lens, and shot 9 opens 3 metres from a lotus pad. They are close-ups, but not all
   are macro.
4. **Not every surface uses a patched physical material.** `README.md:13` says "every surface is
   procedural GLSL injected into three.js's physically based materials". The sky
   (`src/core/sky.js`), the light shafts and dust (`src/world/interior-fx.js`) and the incense
   smoke (`src/world/props-mat.js`) use custom `ShaderMaterial` objects. They do not use `patch()`.
   The sky sets `fog: false` on its material and uses its own atmosphere model. The pond water
   (`src/world/pond-water.js`) is a patched physical material, and its shader calls `applyAerial`
   directly.
5. **The silent render and the scored film are two products.** `tools/render.mjs` writes a video
   stream only. `tools/score.mjs` joins two music sections and muxes the audio onto a copy of the
   video. The release film has music, but a fresh render does not.
6. **The film tools assume a 70 second cut.** `src/shots.js:226` derives the duration from the
   shot count. `tools/render.mjs:103` uses a literal 70 seconds when `--to` is absent.
   `tools/score.mjs:34` sets `FILM = 70.0`. `docs/ARCHITECTURE.md:269` states that several
   tools assume the 70 second cut.
7. **Eight module override files cover nine shots.** `src/shotkeys/` holds nine files: one index
   and eight module files. `terrain.js` exports an empty object. Seven module files replace nine
   shot definitions. `docs/ARCHITECTURE.md:224` describes the nine owned shots correctly.
8. **The shadow map comment is stale.** `src/core/lighting.js:29` says "the 4k shadow map is
   spent where the camera looks". The default parameter is 4096, but `src/main.js:62` passes
   8192 at final quality and 2048 at preview quality. `README.md:161` and
   `docs/ARCHITECTURE.md:29` give the correct values.
9. **The sub-frame count depends on the mode.** `README.md:18` says each film frame averages
   eight sub-frames. `src/main.js:30` sets eight in render mode, four in still mode and one in
   preview mode. The claim applies to the film only.
10. **The camera is clamped at a cut, but the animation clock is not.** In render mode,
    `src/main.js:379` clamps the camera time to the current shot. `setTime()` at
    `src/main.js:169-175` receives the unclamped film time plus the shutter offset. The rule
    "motion blur never spans a cut" therefore applies to the camera path, not to module
    animation time.
11. **Anchors need the props module.** `src/main.js:113-123` collects anchors from every module,
    then asks `props.placeAnchors()` to place lanterns and bells. If the query string selects
    modules without `props`, the anchors are collected but no lantern or bell is placed.
12. **The music file has a checksum guard.** `tools/fetch-music.mjs:18` records the SHA-256 of
    the file used to time the edit. A different file raises a warning, because the cut points may
    miss the musical hits.
13. **The build is not an offline film build.** `npm run build` runs `vite build` for the browser
    page. The film has no bundling step and no build output. It runs through the dev server and
    headless Chrome.
14. **Preview and final quality have different settings.** Preview uses a 2048 squared shadow
    map, 10 ambient-occlusion samples and no depth of field. Final uses an 8192 squared map, 16
    samples, depth of field and a scene-captured environment. The readme gives the same split.

## 11. Verification record

The record fills after the writer and the fresh-reader review.

| Check | Target | Result |
|---|---|---|
| Six documents present | README plus five deep dives | Pending |
| Words per deep dive | 3,000 to 5,000 | Pending |
| Byte-exact snippets | 10 or more per deep dive, 0 bad | Pending |
| Mermaid diagrams | 2 or more per deep dive | Pending |
| `**Aha:**` insights | 1 or more per deep dive | Pending |
| Sentence length | 0 sentences over 30 words | Pending |
| Links | 0 broken relative links | Pending |
| Fresh-reader review | Gaps repaired or recorded | Pending |
| Build | HTML pages generated | Pending |
| Commit and push | No Artificial Intelligence (AI) attribution | Pending |
