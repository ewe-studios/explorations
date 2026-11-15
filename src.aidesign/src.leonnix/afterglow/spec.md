---
title: afterglow package exploration
source_location: /Users/darkvoid/GitSources/src.aidesign/src.leonnix/afterglow
repository: Git checkout, local pin ced1038
explored_at: 2026-11-16
version: 0.1.0
license: MIT for code and generated data; composition, film, audio and stills all rights reserved; third-party assets under their own licences
files: 351
documents: 6
languages: JavaScript, Python, Graphics Library Shading Language (GLSL), HyperText Markup Language (HTML), Cascading Style Sheets (CSS), PowerShell, Markdown
---

# Specification: afterglow package exploration

## 1. Source

| Property | Value |
|---|---|
| Location | `/Users/darkvoid/GitSources/src.aidesign/src.leonnix/afterglow` |
| Kind | Git checkout with its own history |
| Pin | `ced1038` (short local commit identifier) |
| Tracked files | 351 |
| Version | `0.1.0` (`package.json:3`) |
| Manifest licence field | `MIT` (`package.json:6`). The field covers the code only; see the licence row |
| Licence | Split (`LICENSE:1`, `README.md:220-231`). MIT covers the code, the documentation and the generated data. The musical composition and every file that encodes its notes, the performance tracks, the film, the audio renders and the stills are © 2026 Leonxlnx, all rights reserved. Third-party assets keep their own licences: Salamander Grand Piano V3 under CC BY 3.0, Detmold impulse response under CC BY 4.0, MakeHuman and MPFB assets under CC0 1.0, meshoptimizer under MIT |
| Runtime | Node.js 22 or later (`package.json:8`) |
| Renderer | three.js 0.186.1 on WebGL2 (`package.json:15`, `README.md:5`) |
| Browser driver | puppeteer-core 25.15.0 (`package.json:14`) |
| Capture browser | Google Chrome, headless, on Windows 11 in the recorded run (`README.md:130`) |
| Encoder | FFmpeg 8 with libx264 and libmp3lame (`README.md:178`) |
| Audio tooling | Python 3.11 and a virtual environment (`README.md:178`, `requirements.txt`) |
| Film duration | 230.95 s, 13,857 frames at 60 frames per second (`README.md:16`, `README.md:120`) |
| Design record | `docs/DESIGN.md`; module records `docs/PIANO.md`, `docs/WORLD.md`, `docs/PERFORMANCE.md`, `docs/RENDERING.md`, `docs/INTEGRATION.md`, `docs/SHOTLIST.md`; the brief and the credits |

Afterglow is an offline-rendered film. A pianist in a suit plays a concert grand
piano that stands in a shallow mountain lake at sunset. The film is 3 minutes and
51 seconds long. The music, the performance, the piano, the pianist, the sky, the
mountains and the water are all made for the film. The renderer is three.js,
driven frame by frame by Node.js in a headless Chrome page. No frame is captured
from a live session.

The tracked files divide like this:

| Path | Files | Role |
|---|---:|---|
| `scene` | 63 | The film as a web page: render core, world, piano, pianist, camera, integration |
| `tools` | 86 | Offline generators and checks: piano geometry, pianist build in Blender, comparison sheets |
| `music` | 114 | Score domain-specific language (DSL), composition candidates, final score, sampler, mix |
| `perf` | 26 | Performance solver, its libraries, and quality-control (QC) code |
| `docs` | 21 | Eleven Markdown documents and ten stills |
| `render` | 15 | Eight Node.js drivers and seven shared driver libraries |
| `research` | 9 | Research notes behind the look and the build decisions |
| `assets` | 5 | Pianist binary glTF (GLB), rig, pack report, inspection report, readme |
| `sandbox` | 4 | Three helper scripts and one composition sketch that the score checks import |
| `analysis` | 1 | Statistics measured on the reference images |
| Root files | 7 | `README.md`, `LICENSE`, `package.json`, `package-lock.json`, `requirements.txt`, `.gitignore`, `.gitattributes` |

The scene folder divides like this: 14 render-core files, 11 piano geometry
files, 7 camera files, 9 world files, 4 pianist files, 4 piano root files, 3
placeholders, 2 light-transformation-table files, 2 integration files, 1
example, 1 performance-track reader, 1 piano look-development file, 1 vendored
file, and 3 root files (`index.html`, `film.config.js`, `main.js`).

The music folder holds 76 candidate files: 28 for the groove candidate, 26 for
the melody candidate, and 22 for the arranger candidate. It also holds 19 final
score files, 12 root files (7 Python and 5 JSON or Markdown), 5 mix presets, and
2 impulse-response files.

The tools folder is mostly asset generation. 49 files build the piano, 33 build
the pianist at the command line, in Blender and in checks, 2 are frozen copies,
and 2 are root tools.

## 2. Scope

This package covers the film as a system. It covers these areas:

- The music: the score DSL, the note-by-note composition, the validators, and the
  final score data.
- The sound: the offline sampler, the damper and pedal model, the synthetic hall,
  the mix and the mastering chain.
- The performance: the solver that turns score onsets into fingering, hand pose,
  finger inverse kinematics (IK), arm IK, body motion, pedal motion, key motion,
  hammer motion, damper motion, and water ripples.
- The scene: the accumulation renderer, the thin lens, the depth-of-field (DOF)
  gap fill, the final pass, the film light-transformation table (LUT), the
  procedural piano, the pianist asset, the sky, the clouds, the terrain, the
  boulders, the water, the camera edit and the integration checks.
- The render drivers: still frames, chunked film render, resume, determinism,
  snapshot, encode, verify and the GPU lock.
- The edit: 26 shots, every cut anchored to a bar or beat, the focus hand track,
  and the sun arc.
- The quality record: the module documents, the measured numbers, the known
  limitations, the credits and the licence scope.

It does not run the film. It does not render a frame, sample audio, solve the
performance, launch Blender, start the preview server, or encode a file. It does
not listen to the soundtrack or watch the release film. No network request is
made.

## 3. Documents

The package uses the standard six-document layout. Every deep dive explains one
part of the machine, and no deep dive repeats another.

| File | Role | Words | Snippets |
|---|---|---:|---:|
| `markdown/README.md` | Index, thesis, map, document table, reading order, non-claims | no cap | 3 or more |
| `markdown/00-overview.md` | The film, the user, what ships, the two output chains, the numbers, the licence | 3,000 to 5,000 | 10 or more |
| `markdown/01-architecture.md` | The six layers, the module slots, the data contracts, state ownership, the file map | 3,000 to 5,000 | 10 or more |
| `markdown/02-core-mechanism.md` | One frame from the edit to FFV1, and the score-to-audio and score-to-motion split, end to end | 3,000 to 5,000 | 10 or more |
| `markdown/03-workflow-and-quality.md` | A clean rebuild from the pin: audio, solver, self test, still, render, resume, verify, encode; every gate | 3,000 to 5,000 | 10 or more |
| `markdown/04-limits-and-comparison.md` | Limits, render-budget cuts, failure modes, licence boundary, comparison with other projects | 3,000 to 5,000 | 10 or more |

Every deep dive has at least two Mermaid diagrams, at least one `**Aha:**`
insight, and a citation header on every code block. The header gives the source
path relative to the afterglow root and the line range.

## 4. Constraints

- The source is read only. No project file is modified.
- No render, sampler run, solver run, Blender build, browser launch, encode, or
  preview server is executed.
- No network request is made. The release film, the soundtrack, the samples and
  the reference images are not downloaded.
- Prose uses Aerospace and Defence Simplified Technical English, issue 100
  (ASD-STE100). One idea per sentence. Thirty words maximum. Every abbreviation
  is expanded at first use in each file.
- Prose avoids marketing language, em dashes, "simply", "just", and "obviously".
- A value that the source does not define is recorded as not defined.
- The pin is `ced1038`. Runtime behaviour is marked as not verified.
- A quoted line must match the source byte for byte.
- The exploration repeats the split licence. It does not call the whole
  repository MIT.

## 5. Research questions

1. How does one written score produce both a soundtrack and a moving pianist?
2. How does one output frame become an average of N sub-frames, and which pass
   owns motion blur, depth of field, anti-aliasing, glare, tone mapping, grade
   and dither?
3. How does an offline renderer stay deterministic across sessions, chunk
   boundaries and render order?
4. How does the render pipeline survive a 21.8 hour run, and how does it resume?
5. How does the encoder prove that sound and picture match the score?
6. What did the render budget change, and what does the project state as a limit?

## 6. Source map for the writer

| Area | Start point |
|---|---|
| Project statement, numbers, limits, licence | `README.md` |
| Design contracts and coordinate frames | `docs/DESIGN.md` |
| Render page and public interface | `scene/index.html`, `scene/src/main.js` |
| Film constants | `scene/src/core/constants.js` |
| Film configuration and presets | `scene/src/film.config.js` |
| Accumulation renderer | `scene/src/core/accum.js` |
| Final pass and grade | `scene/src/core/final.js` |
| Sample tables and random numbers | `scene/src/core/rng.js` |
| Thin lens and DOF | `scene/src/core/lens.js`, `scene/src/core/glsl.js` |
| Frame input and output | `scene/src/core/frameio.js` |
| Light-transformation table | `scene/src/core/lut.js`, `scene/src/core/luts/` |
| Camera director and lenses | `scene/src/core/camera.js`, `scene/src/camera/` |
| The edit and the cut logic | `scene/src/camera/shots.js`, `scene/src/camera/director.js`, `scene/src/camera/score.js` |
| Hand track for focus | `scene/src/camera/hands.js`, `scene/src/camera/handtrack.json` |
| World root | `scene/src/world/index.js` |
| Sky and clouds | `scene/src/world/sky.js`, `scene/src/world/cloudmap.js`, `scene/src/world/skytable.js` |
| Terrain and boulders | `scene/src/world/terrain.js`, `scene/src/world/boulders.js` |
| Water | `scene/src/world/water.js` |
| Piano | `scene/src/piano/index.js`, `scene/src/piano/layout.js`, `scene/src/piano/materials.js`, `scene/src/piano/geom/` |
| Pianist | `scene/src/pianist/index.js`, `scene/src/pianist/pianist.js`, `scene/src/pianist/tailoring.js` |
| Integration checks and N boost | `scene/src/integration/index.js`, `scene/src/integration/motion_n.json` |
| Performance tracks in the page | `scene/src/perf/tracks.js` |
| Score DSL and composition | `music/dsl.py`, `music/afterglow_melody.py`, `music/afterglow_arr.py`, `music/afterglow_score.py`, `music/afterglow_checks.py` |
| Score library and timing | `music/afterglow_lib.py` |
| Sampler | `music/sampler.py`, `music/render_audio.py`, `music/mix_meter.py` |
| Final score data | `music/final/score.json`, `music/final/score.mid` |
| Mix presets | `music/presets/` |
| Performance solver | `perf/solve.mjs`, `perf/lib/` |
| Performance QC | `perf/qc/`, `perf/qc.js` |
| Still renderer | `render/still.mjs` |
| Film plan, render and resume | `render/film.mjs`, `render/render.mjs` |
| Frame sources and chunk manifests | `render/lib/frames.mjs`, `render/lib/mov.mjs` |
| Browser session | `render/lib/chrome.mjs`, `render/lib/session.mjs`, `render/lib/server.mjs` |
| GPU lock | `render/lib/gpulock.mjs` |
| Encode and verify | `render/encode.mjs` |
| Determinism check | `render/determinism.mjs` |
| Self test | `render/selftest.mjs` |
| Snapshot | `render/snapshot.mjs` |
| Preview server | `render/serve.mjs` |
| Pianist build | `tools/pianist/` |
| Piano generators and checks | `tools/piano/` |
| Reference comparison | `tools/compare.py`, `tools/analyze_refs.py` |
| Module records | `docs/PIANO.md`, `docs/WORLD.md`, `docs/PERFORMANCE.md`, `docs/RENDERING.md`, `docs/INTEGRATION.md`, `docs/SHOTLIST.md` |
| Credits and licence table | `docs/CREDITS.md`, `LICENSE` |

## 7. Task table

| Task | State |
|---|---|
| Verify pin, file count, licence, version, runtime | DONE |
| Map source, render drivers, music, performance, scene, documents | DONE |
| Write `spec.md` | DONE |
| Write the six documents | DONE |
| Snippet check, sentence check, link check | DONE: 69 snippets, 0 bad; 0 long sentences; 82 links, 0 broken |
| Fresh-reader review | DONE: one independent pass, 206 findings |
| Repair reported gaps | DONE: two repair rounds |
| Build HyperText Markup Language (HTML) | DONE |
| Commit and push | DONE |

## 8. Resume point

A later reader starts at `README.md` for the film and its numbers, then at
`docs/DESIGN.md` for the contracts. The render path starts at `scene/src/main.js`
and continues through `scene/src/core/accum.js` and `scene/src/core/final.js`.
The driver path starts at `render/film.mjs`, continues through `render/render.mjs`
and the libraries under `render/lib/`, and ends at `render/encode.mjs`. The music
path starts at `music/dsl.py` and `music/afterglow_score.py`. The performance
path starts at `perf/solve.mjs`.

If the checkout moves from `ced1038`, run the snippet checker again.

## 9. Limits

- Static reading only. No frame, audio file, performance track, or film was
  produced. The release film is not in the repository.
- The visual quality is not evaluated. The README states its own visual limits.
- The audio is not listened to. The measured loudness and peak values come from
  the project documents, not from a new measurement.
- The render times, sub-frame counts and file sizes come from the project
  documents. No measurement was repeated.
- The performance solver code changed after the released run. The README states
  that a fresh solve is not the film's run (`README.md:199`).
- Reproducibility is limited to one Windows machine and one integrated GPU. The
  project states that other hardware gives slightly different pixels.
- The released film, the soundtrack, the samples, the reference images and the
  performance tracks are external assets or release files. The exploration reads
  only the tracked files.
- The film licence is not MIT. Only the code and the listed generated data are
  MIT.

## 10. Corrections and limits found during research

The inventory matched: pin `ced1038`, 351 tracked files, version 0.1.0, Node.js
22 or later. The following findings qualify the first reading.

1. **The manifest licence field is narrow.** `package.json:6` says MIT. The
   `LICENSE` file and `README.md:220-231` state a split scope. The composition,
   every file that encodes its notes, the performance tracks, the film, the
   audio renders and the stills are all rights reserved. A reader must not treat
   the repository as MIT.
2. **The film is not in the repository.** `build/` is excluded
   (`README.md:242`). The checkout holds the source, the score data, the pianist
   asset and the documents. The film exists only as a release download.
3. **Two large binary assets use Git LFS.** The pianist GLB and the Detmold
   impulse response are LFS files (`README.md:187-193`). The released mix does
   not use the impulse response.
4. **The sun arc in the constants file is not the film's arc.**
   `DEFAULT_SUN_KEYS` starts at +0.6 degrees
   (`scene/src/core/constants.js:12`). The edit overrides it and starts at -0.8
   degrees (`scene/src/camera/shots.js:52-56`). The README's -0.8 to -4.5 range
   follows the edit.
5. **The edit's N is not the release's N.** The edit sets 32 for most shots and
   48 for three shots. The final freeze lowered eight close-ups from 48 to 32
   and one shot from 64 to 32 (`scene/src/camera/shots.js:30-31`). The release
   render then halves N after film time 1:50 (`README.md:100`). The released
   sub-frame count is 360,368, not the full-N 473,760.
6. **The motion-blur boost table covers 132 frames.** The table holds boosts from
   48 to 128 sub-frames on frames where a fast hand would otherwise show
   separate copies (`scene/src/integration/motion_n.json`). The README states
   that the release halves these boosts after 1:50.
7. **The soundtrack is longer than the film.** The soundtrack is 227.5 s
   (`README.md:143`). The film places it after a 4.0 s lead-in
   (`scene/src/core/constants.js:5`), so the audio extends about 0.55 s past the
   230.95 s film. The encoder trims or pads the audio to the exact video duration
   (`render/encode.mjs:19-21`, `:104-112`). The tail is cut.
8. **There is no unit-test suite.** The quality gates are the self test, the
   determinism check, the performance QC, the integration checks and the encode
   verification. They are scripts, not a test framework.
9. **The README states the build method.** It describes a multi-agent build
   process (`README.md:149-156`). The exploration records this as a project
   statement. It does not verify the process or repeat the claim as its own.
10. **The film was made on one machine.** The recorded run used a Ryzen 7 260, a
    Radeon 780M integrated GPU, Windows 11 and headless Chrome 155
    (`README.md:125`). The project states that other hardware gives different
    pixels.
11. **The render estimate is not the recorded run.** The rebuild section projects
    about 13.4 hours on an idle GPU (`README.md:213`). The recorded run took 21.8
    hours of wall-clock time, about 9.7 hours of pure render time, with pauses
    for other GPU jobs (`README.md:122`).
12. **The score is frozen.** The README calls `music/final/` the frozen, approved
    version (`README.md:195`). A later edit of the composition invalidates the
    camera hand track and the motion-blur table.
13. **No runtime claim is verified.** No frame was rendered, no audio was
    sampled, no performance was solved, and no encode was run.

## 11. Verification record

The record holds the final checks after the writer, the fresh-reader review and the build.

| Check | Target | Result |
|---|---|---|
| Six documents present | README plus five deep dives | Pass: six files |
| Words per deep dive | 3,000 to 5,000 | Pass: 4,999 / 5,000 / 4,986 / 4,981 / 4,982 |
| Byte-exact snippets | 10 or more per deep dive, 0 bad | Pass: 10 / 10 / 18 / 10 / 16, 0 bad; README 5, 0 bad |
| Mermaid diagrams | 2 or more per deep dive | Pass: 2 / 2 / 2 / 2 / 2; README 1 |
| `**Aha:**` insights | 1 or more per deep dive | Pass: 1 / 1 / 1 / 1 / 2; README 1 |
| Abbreviation first use | Expanded in each file | Pass: all first prose uses expanded; source snippets unchanged |
| Sentence length | 0 sentences over 30 words | Pass: 0 in prose; source comments unchanged |
| Links | 0 broken relative links | Pass: 82 checked, 0 broken in all six files |
| Fresh-reader review | Gaps repaired or recorded | Pass: one independent pass raised 206 findings; repairs applied in two rounds |
| Values the source does not settle | Recorded as not defined, not guessed | Quality control (QC) joint tolerance; pedal sign convention; solver global pass threshold; timing reconciliation; `clipreview.py` absent from the pin |
| Build | HyperText Markup Language (HTML) pages generated | Pass: `build.py` rendered six pages |
| Commit and push | No Artificial Intelligence (AI) attribution | Pass: one commit, pushed to `master` |

The independent fresh-reader review read only the six documents. It raised 206 findings: 22 on the
README, 33 on the overview, 28 on the architecture, 38 on the core mechanism, 40 on the workflow,
33 on the limits, and 12 cross-document findings. The writer repaired them in two rounds. The
coordinator checked each factual correction that it sent against the pinned source, and
re-checked the repaired values afterwards. The
repairs added the shot sum, the frame-grid arithmetic, the anchor grammar, the solved-performance
worked mapping, the camera-state contract, the final depth-of-field composite, the plan-history
table, the zero-based note identifiers, the determinism configuration, the split licence, and the
record-versus-final probe values. Findings that the source cannot settle are recorded as open, not
guessed.
