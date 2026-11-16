---
title: shanshui package exploration
source_location: /Users/darkvoid/GitSources/src.aidesign/src.leonnix/shanshui
repository: Git checkout, local pin b5357c0
explored_at: 2026-11-16
version: 1.0.0
license: MIT
files: 146
documents: 6
languages: JavaScript, Graphics Library Shading Language (GLSL), HyperText Markup Language (HTML), Markdown, JSON
---

# Specification: shanshui package exploration

## 1. Source

| Property | Value |
|---|---|
| Location | `/Users/darkvoid/GitSources/src.aidesign/src.leonnix/shanshui` |
| Kind | Git checkout with its own history |
| Pin | `b5357c0` (short local commit identifier) |
| Tracked files | 146 |
| Version | `1.0.0` (`package.json:3`) |
| Licence | MIT (`LICENSE:1`) |
| Runtime | Browser with WebGL 2 and Canvas2D |
| Build tool | None. The page loads ES modules directly |
| Server | `tools/serve.mjs`, port 8080 by default |
| Capture tool | Puppeteer Core `24.43.1`; WebSocket library `8.22.0` (`package-lock.json:689` and `925`) |
| Scripts | `serve`, `still`, `render` (`package.json:6-10`) |
| Entry page | `index.html` loads `/src/main.js` as a module (`index.html:10`) |
| Entry module | `src/main.js` exposes `window.__api` for tools and starts a viewer |
| Output | One 60 second film, ten paintings, and one soundtrack, all generated |

Shanshui means "mountains and water". The package paints ten Chinese ink-wash landscapes in code.
No image, font or audio sample is loaded at runtime. WebGL 2 shaders and Canvas2D brushwork
produce every cliff, pine, mist bank, crane, ripple and snowflake. JavaScript synthesizes the
soundtrack sample by sample. One clock drives the picture and the sound.

The film is 60 seconds long at 60 frames per second and 1920 by 1080 pixels. It holds ten
paintings, two bars of music each at 80 beats per minute. Every seam is centred on a bar line.
The ten paintings are ordered for the film, not by scene number: bamboo valley, cranes over the
falls, rope bridges, floating peaks, red cascade, autumn ruins, golden river, stone bridge, moon
over the lake, and snow temple.

The tracked files divide like this:

| Path | Files | Role |
|---|---:|---|
| `src` | 127 | Engine, element library, ten scenes, transitions, lab scenes and entry modules |
| `tools` | 8 | Server, still capture, render daemon, film renderer, audio, forensics and helpers |
| `docs` | 5 | Four guides and one contact sheet image |
| Root files | 6 | `README.md`, `LICENSE`, `index.html`, `package.json`, lock file and ignore file |

The `src` tree holds 24,027 JavaScript lines. The `tools` tree holds 1,140 JavaScript module
lines. The code divides like this:

| Folder | Files | JavaScript lines | Role |
|---|---:|---:|---|
| `src` root | 3 | 452 | Entry, film timeline and shot list, shared clock |
| `src/engine` | 4 | 983 | WebGL helpers, shared GLSL, seeded random numbers, compositor |
| `src/lib` | 15 | 8,624 | Brush, ink effects and the element generators |
| `src/scenes` | 61 | 12,176 | Ten scenes, each with its own helpers |
| `src/transitions` | 1 | 334 | Ten seam programs |
| `src/lab` | 43 | 1,458 | Small test scenes for single elements |

The scene folders hold ten main scene modules, one registry module and 50 helper modules. The element library holds
twelve top-level generators and three fauna helpers. The four guides are `ENGINE.md` for the
application programming interface (API), `STYLE.md` for palette and painting rules, `FILM.md` for
the cut, and `LIBRARY.md` for the element index.

## 2. Scope

This package covers shanshui as a system. It covers these areas:

- The visual idea: ink on warm rice paper, three depth planes, a value ladder, edges, trees,
  water and a sparing red accent.
- The engine: a 2.5D camera, flat cards at depth, three layer kinds, premultiplied alpha,
  reflections, wind sway, bakes and the shared GLSL library.
- The painting library: brush, ink effects, pine, rock, peaks, atmosphere, water, architecture,
  fauna, flora and the seal.
- The ten scenes: composition, layers, camera move, animation and per-scene helpers.
- The transitions: bloom, mist, cloud rise, spray, wind, scroll, sun match, dusk, snow and
  evaporate.
- The clock: 80 beats per minute, four beats per bar, two bars per painting, 20 bars in 60
  seconds, and named sound cues.
- The sound: a guzheng, xiao flute, singing bowl, wind and water, synthesized from samples.
- The tools: live server, warm render daemon, still and comparison tool, contact sheets, film
  renderer, frame forensics and documentation generator.
- The film path: sub-frame motion blur, GPU YUV conversion, WebSocket streaming and ffmpeg.

It does not run shanshui. It does not start the server, open a browser, render a frame, play the
film, generate the soundtrack or build a scene. All runtime numbers come from the project
documents and are marked as not verified.

## 3. Documents

The package uses the standard six-document layout.

| File | Role | Words | Snippets |
|---|---|---:|---:|
| `markdown/README.md` | Index, thesis, map, reading order | no minimum | 3 or more |
| `markdown/00-overview.md` | The ten paintings, the film, the parts and one frame | 3,000 to 5,000 | 10 or more |
| `markdown/01-architecture.md` | Layers, module contracts, data flow and ownership | 3,000 to 5,000 | 10 or more |
| `markdown/02-core-mechanism.md` | From a time value to ink on paper, traced end to end | 3,000 to 5,000 | 10 or more |
| `markdown/03-workflow-and-quality.md` | One painting from reference to accepted still, then the film | 3,000 to 5,000 | 10 or more |
| `markdown/04-limits-and-comparison.md` | Limits, failure modes, inconsistencies and comparisons | 3,000 to 5,000 | 10 or more |

Every deep dive has at least two Mermaid diagrams, at least one `**Aha:**` insight, and a
citation header on every code block. The citation header gives the source path relative to the
shanshui root and the line range.

## 4. Constraints

- The source is read only. No shanshui file is modified.
- No server, browser, still tool, daemon, film render, audio synthesis or scene build is run.
- No network request is made. No dependency is installed.
- Prose uses Aerospace and Defence Simplified Technical English, issue 100 (ASD-STE100). One idea
  per sentence. Thirty words maximum. Expand every abbreviation at first use in each file.
- Prose avoids marketing language, em dashes, "simply", "just", and "obviously".
- A value that the source does not define is recorded as not defined.
- The pin is `b5357c0`. Runtime behaviour is marked as not verified.
- A quoted line must match the source byte for byte.
- The exploration does not repeat the build-process product names.

## 5. Research questions

1. What does a "living painting" require, and how does one clock keep picture and sound
   together?
2. How does the 2.5D camera turn flat cards at depth into parallax, and why is the design space
   fixed at 1920 by 1080?
3. How does the scene contract work: build once, return layers, then animate as a pure function
   of time?
4. How does the engine composite bakes, shader cards and Canvas2D cards with premultiplied
   alpha?
5. How do the brush and ink-effect libraries make Canvas2D marks read as ink soaked into paper?
6. How do the element generators build a pine, a rock, a peak, mist, water, architecture, a
   crane, bamboo and the seal?
7. How do the ten scenes divide one reference composition into depth planes and moving parts?
8. How do the ten transitions carry a seam, and how do scene time and camera offsets overlap?
9. How does the film render path produce motion blur, convert to YUV on the graphics processing
   unit (GPU), stream frames and encode with ffmpeg?
10. How does the still tool compare a render with a reference image, and what does the warm
    daemon change?
11. How is the soundtrack synthesized, and how does it follow the 80 beat clock?
12. What does the project state it cannot do, and where does the static reading stop?

## 6. Source map for the writer

| Area | Start point |
|---|---|
| Entry page | `index.html` |
| Entry module and capture API | `src/main.js` |
| Clock, bars and sound cues | `src/timeline.js` |
| Film shot list and overlays | `src/film.js` |
| Compositor | `src/engine/engine.js` |
| WebGL helpers | `src/engine/gl.js` |
| Shared GLSL | `src/engine/glsl.js` |
| Seeded random numbers and noise | `src/engine/rng.js` |
| Brush vocabulary | `src/lib/brush.js` |
| Ink pass | `src/lib/inkfx.js` |
| Pine generator | `src/lib/pine.js` |
| Rock generator | `src/lib/rock.js` |
| Peak generator | `src/lib/peaks.js` |
| Mist and clouds | `src/lib/atmos.js` |
| Water and waterfalls | `src/lib/water.js` |
| Architecture | `src/lib/architecture.js` |
| Fauna | `src/lib/fauna.js`, `src/lib/fauna/` |
| Flora | `src/lib/flora.js` |
| Seal | `src/lib/seal.js` |
| Leaf gust overlay | `src/lib/gust.js` |
| Scene registry | `src/scenes/index.js` |
| Scenes | `src/scenes/s01-cranes-falls.js` to `src/scenes/s10-autumn-ruins.js` |
| Scene helpers | `src/scenes/s01/` to `src/scenes/s10/` |
| Transitions | `src/transitions/index.js` |
| Lab scenes | `src/lab/` |
| Live server | `tools/serve.mjs` |
| Shared tool helpers | `tools/lib.mjs` |
| Still and comparison tool | `tools/still.mjs` |
| Warm render daemon | `tools/daemon.mjs` |
| Film renderer | `tools/render.mjs` |
| Soundtrack | `tools/audio.mjs` |
| Frame forensics | `tools/forensics.mjs` |
| Library documenter | `tools/libdoc.mjs` |
| Project statement | `README.md` |
| Guides | `docs/ENGINE.md`, `docs/STYLE.md`, `docs/FILM.md`, `docs/LIBRARY.md` |

## 7. Task table

| Task | State |
|---|---|
| Verify pin, file count, licence, version, dependencies | DONE |
| Map source, tools, docs and absent outputs | DONE |
| Write `spec.md` | DONE |
| Write the six documents | DONE |
| Snippet check, sentence check, link check | DONE |
| Fresh-reader review | DONE (two rounds, 43 and 11 findings) |
| Repair reported gaps | DONE |
| Build HyperText Markup Language (HTML) | DONE (8 pages plus style sheet) |
| Commit and push | DONE (`cfa74bb`, pushed) |

## 8. Resume point

A later reader starts at `README.md` for the film description and the ten-painting table, then at
`src/timeline.js` for the clock. The frame path starts at `index.html`, moves through
`src/main.js`, `src/film.js` and `src/engine/engine.js`, and ends in `src/engine/gl.js`. The
painting path starts at `src/lib/brush.js`, continues in `src/lib/inkfx.js`, and then enters the
element generators. The scene path starts at `src/scenes/index.js` and `src/scenes/s01/`. The
film output path starts at `tools/render.mjs` and ends at `tools/audio.mjs` and ffmpeg.

If the checkout moves from `b5357c0`, run the snippet checker again.

## 9. Limits

- Static reading only. No frame was rendered and no film was viewed.
- The reference paintings `ref/01.webp` to `ref/10.webp` are not in the repository. The README
  states this. Visual agreement is not checked here.
- The film is not in the repository. The README links to release assets.
- The soundtrack is not in the repository. The `out/` folder is ignored.
- No browser was opened. WebGL 2 behaviour, shader compilation and timings are not verified.
- The scene helpers, bakes and textures were not decoded or measured.
- The tools were not run. Their outputs are not checked.
- The build process is not reproducible here. The project has no build step.
- The comparison tool needs reference images that this checkout does not hold.

## 10. Corrections and limits found during research

The inventory matched: pin `b5357c0`, 146 tracked files, version 1.0.0, MIT. The following
findings qualify the first reading.

1. **The film is not in the repository.** `README.md:9` says the film is attached to the
   releases. `git ls-files` holds no `.mp4` file and no `video/` folder. The render path is
   defined by `tools/render.mjs`. The output does not ship.
2. **The reference paintings are not in the repository.** `README.md:64` and
   `docs/ENGINE.md:3` refer to `ref/01.webp` to `ref/10.webp`. The folder is absent. The
   comparison options need images that the reader must supply.
3. **The soundtrack is not in the repository.** `README.md:41` tells the reader to run
   `tools/audio.mjs` and write `out/audio/soundtrack.wav`. No `.wav` file is tracked and `out/`
   is ignored (`.gitignore:2`).
4. **All render output is absent.** `out/` is ignored. The contact sheet `docs/stills.jpg` is the
   only picture in the checkout. It is documentation, not a render input.
5. **One transition duration differs between the guide and the timeline.** `docs/FILM.md:17`
   gives the wind wipe as 1.6 seconds. `src/timeline.js:24` sets the autumn ruins transition to
   2.0 seconds with the `wind` type. The timeline is the executable value.
6. **The spray transition centres differ between the guide and the timeline.** `docs/FILM.md:37`
   assumes the s09 waterfall at (330, 560) and the s08 cascade at (700, 420).
   `src/timeline.js:23` passes `c: [308, 672]` and `c2: [640, 270]`. The timeline is the
   executable value.
7. **The sun-match centres differ between the guide and the timeline.** `docs/FILM.md:38`
   assumes s02 at (1520, 330) and s05 at (1700, 345). `src/timeline.js:26` passes
   `c: [1349, 308]` and `c2: [1742, 353]`.
8. **The no-assets claim has one qualification.** The runtime loads no image, font or audio
   sample (`docs/ENGINE.md:3-4`). The README displays `docs/stills.jpg`, and the comparison tools
   read `ref/` images. Neither is a render input.
9. **There is no build step.** `package.json` has no `build` script. `index.html` loads the
   modules directly. Vite is absent.
10. **The dependencies do not include a browser.** `puppeteer-core` connects to an installed
    Chrome or Edge. The README states this at `README.md:45`. `npm install` installs the client
    only.
11. **The Node version is a README statement.** `README.md:45` states Node 20 or later.
    `package.json` has no `engines` field, so the version is not enforced.
12. **The film has no tracked output or duration proof.** The 60 second duration comes from
    `src/timeline.js:9`. No encoded file verifies it.

## 11. Verification record

The record fills after the writer and the fresh-reader review.

| Check | Target | Result |
|---|---|---|
| Six documents present | README plus five deep dives | Pass |
| Words per deep dive | 3,000 to 5,000 | 4,997 / 4,998 / 4,997 / 4,926 / 4,999 |
| Byte-exact snippets | 10 or more per deep dive, 0 bad | 12 / 10 / 10 / 11 / 11, 0 bad |
| Mermaid diagrams | 2 or more per deep dive | 2 / 3 / 2 / 2 / 2, plus 1 in README |
| `**Aha:**` insights | 1 or more per deep dive | 1 in each deep dive |
| Sentence length | 0 sentences over 30 words | 0 |
| Links | 0 broken relative links | 22 checked, 0 broken |
| Fresh-reader review | Gaps repaired or recorded | 43 findings in round one, 11 in round two, all repaired |
| Build | HTML pages generated | Pass (8 pages plus style sheet) |
| Commit and push | No Artificial Intelligence (AI) attribution | Pass (`cfa74bb`, pushed) |
