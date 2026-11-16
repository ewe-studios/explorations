---
title: casa-ocaso package exploration
source_location: /Users/darkvoid/GitSources/src.aidesign/src.leonnix/casa-ocaso
repository: Git checkout, local pin fcc3379
explored_at: 2026-11-16
version: 1.0.0
license: MIT for code; Creative Commons Zero (CC0) for third-party panoramas and textures
files: 193
documents: 6
languages: JavaScript, Graphics Library Shading Language (GLSL) through Three Shading Language (TSL), Python, PowerShell, HyperText Markup Language (HTML), Cascading Style Sheets (CSS), JSON
---

# Specification: casa-ocaso package exploration

## 1. Source

| Property | Value |
|---|---|
| Location | `/Users/darkvoid/GitSources/src.aidesign/src.leonnix/casa-ocaso` |
| Kind | Git checkout with its own history |
| Pin | `fcc3379` (short local commit identifier) |
| Tracked files | 193 |
| Version | `1.0.0` (`package.json:4`) |
| Licence | Code MIT (`LICENSE:1`); panoramas and textures CC0; film music CC BY 4.0 and not in the repository |
| Runtime | Browser, WebGPU through three.js `0.187.0` (`package-lock.json`) |
| Build tool | Vite `8.3.5`; capture tool Playwright `1.64.1` (`package-lock.json`) |
| Scripts | `dev`, `build`, `preview` (`package.json:6-10`) |
| Entry page | `index.html` loads `/src/main.js` as a module (`index.html:27`) |
| Entry module | `src/main.js` selects interactive or capture mode from the query string |
| Scene | One cliffside living room, eight camera shots and six outside views |

Casa Ocaso is a real-time recreation of a cliffside living room at golden hour. The room has a
curved glass wall, a plaster core with two linear fireplaces, a curved boucle sofa, a travertine
coffee table, a round wool rug, a lounge chair, a dining terrace, a cliff and Mediterranean
planting. The browser draws the room with the WebGPU renderer of three.js and its node material
system, Three Shading Language (TSL). The outside is a set of six real high dynamic range (HDR)
panoramas. Each panorama is rotated so its sun matches a reference image. The renderer restores
the clipped sun with a measured halo. A pre-baked light probe grid supplies the bounce light.

The project states that an artificial intelligence coding tool built the room from five reference
renders (`README.md:3-5`). The exploration records this as a project statement. It does not verify
the process and does not repeat the product name.

The tracked files divide like this:

| Path | Files | Role |
|---|---:|---|
| `src` | 31 | Application code, one entry module and one layout module |
| `tools` | 30 | Capture, bake, profile, reference comparison and film tools |
| `public` | 122 | Six view asset sets, six models, 44 texture maps |
| `docs` | 3 | Hero image, table detail and one reference image |
| Root files | 7 | `README.md`, `LICENSE`, `index.html`, `package.json`, lock file, Vite config, ignore file |

The `src` tree holds 9,531 JavaScript lines. The `tools` tree holds 408 `.mjs` lines, 3,795
Python lines and 123 PowerShell lines. The `public` tree is about 147 mebibytes: 86 mebibytes of
view assets, 44 mebibytes of textures and 17 mebibytes of models. The code divides like this:

| Folder | Files | JavaScript lines | Role |
|---|---:|---:|---|
| root | 2 | 604 | Config with every room dimension, and the entry module |
| `core` | 1 | 218 | WebGPU renderer and the two render pipelines |
| `scene` | 5 | 1,955 | Architecture, cliff, limestone, materials and shader utility |
| `scene/furniture` | 10 | 3,298 | Sofa, lounge, tables, rug, fireplace, curtains, decor and dining |
| `scene/plants` | 2 | 1,628 | Olive toolkit, potted olive, maquis and exterior planting |
| `world` | 5 | 1,058 | Backdrop, islands, lighting, reflections and view table |
| `camera` | 2 | 297 | First-person controller and the shot list |
| `geo` | 1 | 244 | Curved wall, annulus and revolve helpers |
| `audio` | 1 | 156 | Procedural sea, breeze and fire sound |
| `ui` | 2 | 73 | Overlay script and its style sheet |

## 2. Scope

This package covers casa-ocaso as a system. It covers these areas:

- The room plan: one config file holds the curved glass, roof, oculus, core, fireplaces, terrace,
  seating, dining, decor and cliff positions.
- The architecture: floor, glass, roof, core, back wall, column and planters.
- The materials: plaster, boucle, linen, knit, travertine, ceramic, bronze and limestone.
- The render pipeline: the ultra path with a normal and velocity pre-pass, ground-truth ambient
  occlusion (GTAO), temporal resolution anti-aliasing (TRAA) and sharpening; the high path with
  multi-render-target (MRT) geometry, post ambient occlusion, temporal anti-aliasing upscaling
  (TAAU) and dynamic resolution.
- The outside: panorama loading, sun measurement, halo reconstruction, procedural open sea,
  islands and fog banks.
- The light: a directional sun with virtual shadow maps (VSM), a hemisphere fill, a light probe
  grid with two bounces, baked atlas files and atlas blending during view changes.
- The physical scene: signed distance field (SDF) assets built in Python, loaded as GLB, with
  baked vertex ambient occlusion and a detail render layer.
- The views: six panorama sets, per-view grade, sun, exposure, cove and fill; the 2.8 second
  crossfade.
- The cameras and controls: drag, wheel, keyboard, eight shots, a guided tour and a capture
  application programming interface (API).
- The tools: headless capture, GI bake, profiling, colour sampling, tonal tracing, reference
  comparison, film render, merge, finish and encode.
- The film: an offline 4K path defined by scripts; the film and its music are not in the repository.

It does not run casa-ocaso. It does not start Vite, open a browser, render a pixel, bake a probe,
build a model or play the film. All runtime numbers come from the project documents and are marked
as not verified.

## 3. Documents

The package uses the standard six-document layout.

| File | Role | Words | Snippets |
|---|---|---:|---:|
| `markdown/README.md` | Index, thesis, map, reading order | no minimum | 3 or more |
| `markdown/00-overview.md` | The room, the views, the parts and one frame | 3,000 to 5,000 | 10 or more |
| `markdown/01-architecture.md` | Layers, module contracts, data flow and ownership | 3,000 to 5,000 | 10 or more |
| `markdown/02-core-mechanism.md` | From panorama to light to pixel, traced end to end | 3,000 to 5,000 | 10 or more |
| `markdown/03-workflow-and-quality.md` | One reference match and one film, with every check | 3,000 to 5,000 | 10 or more |
| `markdown/04-limits-and-comparison.md` | Limits, failure modes, inconsistencies and comparisons | 3,000 to 5,000 | 10 or more |

Every deep dive has at least two Mermaid diagrams, at least one `**Aha:**` insight, and a citation
header on every code block. The citation header gives the source path relative to the casa-ocaso
root and the line range.

## 4. Constraints

- The source is read only. No casa-ocaso file is modified.
- No server, browser, capture, bake, profile, model build or film command is run.
- No network request is made. No dependency is installed.
- Prose uses Aerospace and Defence Simplified Technical English, issue 100 (ASD-STE100). One idea
  per sentence. Thirty words maximum. Expand every abbreviation at first use in each file.
- Prose avoids marketing language, em dashes, "simply", "just", and "obviously".
- A value that the source does not define is recorded as not defined.
- The pin is `fcc3379`. Runtime behaviour is marked as not verified.
- A quoted line must match the source byte for byte.
- The exploration does not repeat the build-process product names.

## 5. Research questions

1. What problem does a reference-driven room solve, and how does one config file control every
   curved surface?
2. How does the ultra pipeline differ from the high pipeline, and what does each pass add?
3. How does a photograph become a light source: sun detection, rotation, cube faces, halo table,
   sun colour and sun irradiance?
4. How does the backdrop shader turn six cube faces into a panorama, restore the clipped sun, and
   replace the lower half with open sea for the sea-level view?
5. How does the room receive bounce light: probe grid, two bounces, baked atlas, blend and
   interior reflection probe?
6. How do the materials match the reference images: palette, relief, fabric lighting, limestone
   and the travertine floor?
7. How is stone, fabric and furniture geometry built offline as signed distance fields and loaded
   with baked ambient occlusion?
8. How does the furniture assembly isolate a failed module, and how does the detail layer save
   probe and reflection cost?
9. How does a view change blend the panorama, the sun, the grade and the light probe data?
10. How do the camera, the shots, the tour and the capture API share one scene?
11. How do the tools measure a render against a reference and produce the film?
12. What does the project state it cannot do, and where does the static reading stop?

## 6. Source map for the writer

| Area | Start point |
|---|---|
| Entry page | `index.html` |
| Entry module | `src/main.js` |
| Room plan | `src/config.js` |
| Renderer and grade | `src/core/renderer.js` |
| Views and sun colour | `src/world/views.js` |
| Panorama shader | `src/world/backdrop.js` |
| Islands and haze | `src/world/islands.js` |
| Lighting and probe bake | `src/world/lighting.js` |
| Interior reflection | `src/world/reflections.js` |
| Architecture | `src/scene/architecture.js`, `src/geo/arch.js` |
| Materials | `src/scene/materials.js`, `src/scene/limestone.js`, `src/scene/tsl-utils.js` |
| Cliff | `src/scene/cliff.js` |
| Furniture assembly | `src/scene/furniture/index.js` |
| Sofa | `src/scene/furniture/seating.js`, `tools/assets/sofa.py` |
| Lounge chair and side table | `src/scene/furniture/lounge.js`, `tools/assets/lounge.py` |
| Coffee table | `src/scene/furniture/coffee-table.js` |
| Rug | `src/scene/furniture/rug.js` |
| Fireplace | `src/scene/furniture/fireplace.js` |
| Curtains | `src/scene/furniture/curtains.js` |
| Decor | `src/scene/furniture/decor.js`, `tools/assets/decor_stones.py` |
| Dining set | `src/scene/furniture/dining.js`, `tools/assets/dining_chair.py` |
| Textiles | `src/scene/furniture/textiles.js`, `tools/assets/throw.py` |
| Plants | `src/scene/plants/olive.js`, `src/scene/plants/exterior.js` |
| Camera and shots | `src/camera/controller.js`, `src/camera/shots.js` |
| UI and audio | `src/ui/ui.js`, `src/audio/ambience.js` |
| Capture tools | `tools/shoot.mjs`, `tools/gpu.ps1`, `tools/profile.mjs`, `tools/diag.mjs` |
| GI bake | `tools/bake_gi.mjs` |
| Reference checks | `tools/compare.py`, `tools/trace.py`, `tools/sample_colors.py` |
| Asset tools | `tools/assets/sdf.py`, `tools/assets/*.py`, `tools/process_hdri.py` |
| Texture tools | `tools/prepare_textures.py`, `tools/bake_boucle_loops.py`, `tools/gen_jacquard.py`, `tools/decimate_glb.py` |
| Film tools | `tools/render_video.mjs`, `tools/render_chunks.ps1`, `tools/video_merge.py`, `tools/video_finish.py`, `tools/video_encode.ps1` |
| Project statement | `README.md` |

## 7. Task table

| Task | State |
|---|---|
| Verify pin, file count, licence, version, dependencies | DONE |
| Map source, tools, public assets and documents | DONE |
| Write `spec.md` | DONE |
| Write the six documents | DONE (27,652 words, 65 snippets) |
| Snippet check, sentence check, link check | DONE (65 snippets, 0 bad; 0 long sentences; 29 links, 0 broken) |
| Fresh-reader review | DONE (25 findings) |
| Repair reported gaps | DONE (25 repaired; atlas, probe lookup, texture sets, tone-map order and floor reflection rewritten) |
| Build HyperText Markup Language (HTML) | DONE |
| Commit and push | DONE |

## 8. Resume point

A later reader starts at `README.md` for the project statement, then at `src/config.js` for the
room plan. The render path starts at `index.html`, moves through `src/main.js`,
`src/core/renderer.js` and `src/world/lighting.js`, and ends in `src/world/backdrop.js`. The
material path starts at `src/scene/materials.js`, continues in `src/scene/architecture.js`, and
ends in `src/scene/furniture/index.js`. The asset path starts at `tools/assets/sdf.py` and
`tools/process_hdri.py`. The measurement path starts at `tools/shoot.mjs` and ends in
`tools/trace.py`.

If the checkout moves from `fcc3379`, run the snippet checker again.

## 9. Limits

- Static reading only. No pixel was rendered and no frame was viewed.
- The reference images are not in the repository. Only `docs/shot_ref1.jpg` is available; the
  other reference images are absent, so no visual match is checked here.
- The film is absent. The `video/` directory in the project README is not tracked.
- No browser was opened. WebGPU behaviour, shader compilation and timings are not verified.
- The light probe atlas, cube faces, HDR files, GLB models and texture maps were not decoded.
- The asset, bake and film tools were not run. Their outputs are not checked.
- The build process is a project statement. It is not reproducible here.
- The capture tools select a Windows graphics path and use PowerShell wrappers. The checkout was
  read on macOS, so the tools do not match this machine.

## 10. Corrections and limits found during research

The inventory matched: pin `fcc3379`, 193 tracked files, version 1.0.0, code MIT. The following
findings qualify the first reading.

1. **The film is not in the repository.** `README.md:73-82` describes a 45 second flythrough in
   `video/` and four film scripts. `git ls-files` holds no `video/` directory and no `.mp4` file.
   The scripts define the path. The output does not ship.
2. **The reference images are not in the repository.** `README.md:68-69` tells a reader to put
   `refs/ref1.png` to `refs/ref5.png` in place. The repository holds only `docs/shot_ref1.jpg`,
   `docs/hero.jpg` and `docs/detail_table.jpg`. The project states that the originals are not part
   of the repository (`README.md:69`).
3. **The capture tools target a Windows graphics path.** `tools/shoot.mjs` and
   `tools/bake_gi.mjs` pass `--use-angle=d3d11` to Chrome. `tools/gpu.ps1` and
   `tools/render_chunks.ps1` are PowerShell. `tools/bake_gi.mjs` launches Chrome headless with
   those arguments. The source does not define a macOS or Metal path.
4. **One diagnostic tool names a page that is not tracked.** `tools/gpuinfo.mjs` navigates to
   `/tools-blank.html`. No such file is tracked. The navigation is wrapped in a catch, so the
   tool continues and reads the adapter limits from the browser.
5. **The `?q` option does not match the default.** `README.md:24` presents `?q=ultra` as the
   full-resolution path. `src/main.js:53` sets the interactive default to `high` and the capture
   default to `ultra`. The query option can select `ultra` in a normal session, but it is not the
   default.
6. **Two tone map defaults exist.** `src/core/renderer.js:34` sets the renderer tone map to AgX.
   `src/main.js:56` passes ACES as the pipeline tone map unless `?tm=` overrides it, and the
   pipeline disables the renderer's own output transform (`renderer.js:87`). The project README
   says ACES (`README.md:49`). The effective path is ACES unless the query changes it.
7. **The six panoramas are real, but one view is partly procedural.** The beach view keeps the
   photograph for sky and horizon and replaces the lower half with open sea rendered in
   `src/world/backdrop.js`. The README states this (`README.md:24-29`). The other five use the
   photograph.
8. **Each view ships 12 files.** The six view folders hold six cube faces, two HDR environments,
   one thumbnail, `meta.json`, `gi.bin` and `gi.json`. The view assets are 86 mebibytes of the
   147 mebibyte public tree.
9. **The GI atlas depth is not the probe count.** `public/views/beach-sunrise/gi.json` records
   `dims` as `[18,6,126]`, while the grid count in `src/world/lighting.js:17` is `[18,6,16]`.
   The atlas is a packed GPU layout. The source does not state the packing formula, so this
   exploration does not derive it.
10. **There is no automated test suite.** The quality tools are manual capture and comparison
    scripts. The source has no test runner and no assertion set for the image.
11. **The music is not in the repository.** `README.md:90-91` names "The Great Sea" by Scott
    Buckley under CC BY 4.0. The encode tool expects an audio file under `.tools/`, which is not
    tracked.
12. **The project records an artificial intelligence build process.** `README.md:3-4` states it.
    The exploration records this as a project statement, not as verified work.

## 11. Verification record

The record fills after the writer and the fresh-reader review.

| Check | Target | Result |
|---|---|---|
| Six documents present | README plus five deep dives | PASS (6 files) |
| Words per deep dive | 3,000 to 5,000 | PASS (5,000 / 4,999 / 4,999 / 4,999 / 4,997; 27,652 total) |
| Byte-exact snippets | 10 or more per deep dive, 0 bad | PASS (13 / 12 / 10 / 12 / 14; 65 total, 0 bad) |
| Mermaid diagrams | 2 or more per deep dive | PASS (2 / 3 / 2 / 2 / 2; README 1; 11 total) |
| `**Aha:**` insights | 1 or more per deep dive | PASS (2 / 1 / 3 / 2 / 2; README 1; 10 total) |
| Sentence length | 0 sentences over 30 words | PASS (0 across 6 files) |
| Links | 0 broken relative links | PASS (29 checked, 0 bad) |
| Fresh-reader review | Gaps repaired or recorded | PASS (25 findings, 25 repaired) |
| Build | HTML pages generated | PASS (index, README, five deep dives, spec) |
| Commit and push | No Artificial Intelligence (AI) attribution | PASS |
