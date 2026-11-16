---
title: needle-drop package exploration
source_location: /Users/darkvoid/GitSources/src.aidesign/src.leonnix/needle-drop
repository: Git checkout, local pin 8982f09
explored_at: 2026-11-16
version: none stated
license: not stated
files: 100
documents: 6
languages: Python, JavaScript, Shell, PowerShell, JavaScript Object Notation (JSON), Markdown
---

# Specification: needle-drop package exploration

## 1. Source

| Property | Value |
|---|---|
| Location | `/Users/darkvoid/GitSources/src.aidesign/src.leonnix/needle-drop` |
| Kind | Git checkout with its own history |
| Pin | `8982f09` (short local commit identifier) |
| Full pin | `8982f09c1502b5f6c01b1aa7c1eee23e5139677b` |
| Commit date | 2026-11-09 21:04:18 +0100 |
| Commit subject | `NEEDLE DROP: 30 s cinematic short built from headless Blender scripts` |
| Tracked files | 100 |
| Licence | Not stated. No root licence file exists. |
| Version | No version field exists. There is no package manifest. |
| Runtime | Blender 5.2.2, headless, with the Blender Python module (`bpy`) |
| Render engine | EEVEE, the Blender real-time render engine, with raytracing enabled |
| Image | 2560 by 1440 pixels, 24 frames per second, 720 frames |
| Delivered frames | Frames 1 to 708. Frames 709 to 720 are black and are added at encode. |
| Audio | 48 kilohertz stereo, delivered at about -14 loudness units relative to full scale (LUFS) |
| Other programs | ffmpeg and ffprobe for audio and video; Node.js for one mix script |
| Asset downloads | Poly Haven models, textures and a high dynamic range image (HDRI), all Creative Commons Zero (CC0) |

Needle Drop is a 30 second film. It is built from three Blender scenes and a set of Python
scripts. The film has one visual idea: a record appears at four scales. A canyon is a record
groove. A turntable is the machine that plays it. A desert becomes a 2.4 kilometre record. A
mountain-sized tonearm lowers a needle onto the desert.

The film uses six shots:

| Shot | Name | Frames | Set | Length |
|---|---|---:|---|---:|
| S1 | The Groove | 1 to 120 | A | 5.0 seconds |
| S2 | The Machine | 121 to 288 | B | 7.0 seconds |
| S3 | The Desert | 289 to 432 | C | 6.0 seconds |
| S4 | The Drop | 433 to 516 | C | 3.5 seconds |
| S5 | The Record | 517 to 624 | C | 4.5 seconds |
| S6 | The Needle | 625 to 708 | C | 3.5 seconds |

The film has no hand keyframes on the camera. Each set script computes a camera path in
NumPy. The script then writes one linear key per frame into the `.blend` file. Set B also
writes a second file, `blend/tonearm_asset.blend`. Set C appends that file and scales the
tonearm by 8000, so the same asset is a desk-sized arm in S2 and a mountain-sized arm in S6.

The project has four layers:

| Path | Files | Lines | Role |
|---|---:|---:|---|
| `scenes/set_a` | 7 | 1,956 | Groove canyon builder, camera design, surface field and checks |
| `scenes/set_b` | 12 | 2,326 | Turntable and tonearm builder, camera rig, handoff data and probes |
| `scenes/set_c` | 16 | 3,915 | Desert record builder, terrain, props, effects, camera and look |
| `tools` | 14 | 787 | Shared library, marble field, safe runner, render, master and encode scripts |
| `audio/scripts` | 28 | 2,091 | Measurement, music edit, cue build, master and verification scripts |
| `data` | 5 | not code | Beat grid, timing events, handoff values and Set C layout |
| `docs` | 3 | not code | Production bible, builder brief and concept list |
| `stills` | 10 | not code | Ten frames from the released film |
| Root files | 3 | not code | `README.md`, `.gitignore` and one more file |

The code total is 11,075 lines across Python, JavaScript, Shell and PowerShell files. The
`scenes` tree holds 8,197 lines. The `tools` tree holds 787 lines. The `audio/scripts` tree
holds 2,091 lines.

The three set scripts have different shapes:

| Set | Entry | Modules | What it does |
|---|---|---|---|
| A | `scenes/set_a/build.py` | `design.py`, `assets_a.py`, `check_path.py`, `fetch_assets.py`, `lumstats.py`, `time_test.py` | Builds a 30 metre deep V-groove field, a cut-gem spire, a lint bundle and dust. It writes the camera from a pure NumPy path. |
| B | `scenes/set_b/build.py` | None that it imports; one file builds all | Builds a studio cove, platter, vinyl, label, controls, strobe ring and tonearm. It writes the tonearm asset and the handoff file. |
| C | `scenes/set_c/build.py` | `c_common.py`, `c_terrain.py`, `c_look.py`, `c_cam.py`, `c_props.py`, `c_fx.py`, `c_nodes.py` | Builds a desert record, outer dunes, mesas, trees, boulders, effects and a giant tonearm. It writes the camera and the record morph. |

The project has no test suite. It has probe and diagnostic scripts instead. About 20 of the
100 files are one-purpose probes, comparison checks or quality-fix scripts. Examples are
`scenes/set_b/ab_cove.py`, `scenes/set_b/mbfix_check.py`, `scenes/set_c/probe_arm.py` and
`scenes/set_c/qafix_patch.py`. These files record how a human and a language model measured
and repaired the film.

The exploration is a static reading. It does not build a scene, render a frame, run a
diagnostic, download an asset, play the audio or encode a film.

## 2. Scope

This package covers Needle Drop as a system. It covers these areas:

- The film idea: one record at four scales, and the six shots that show it.
- The production plan: the production bible, the builder brief that overrides it, and the
  `data/` files that hold the numbers of record.
- The three set builders: their entry points, module boundaries, geometry and materials.
- The shared look library: the palette, the marble field, the fade group and the shared
  material helpers.
- The camera system: pure NumPy paths, piecewise cubic interpolation, the set A vault, the Set
  C cut handling and the Set B rig.
- The match cut: the `S2_end` handoff values and how Set C rebuilds the same marble field.
- The drop: the front wall, the spring-overshoot morph, the kick rings and the record build.
- The giant tonearm: the asset handoff, the 8000 times scale, the cheat table and the touchdown.
- The render path: the safe runner, the chunked render script, resolution and sample overrides.
- The finishing path: the master script, the grain, the frame-count check and the verification.
- The audio path: analysis, music edit, cue sheet, delivery master and verification.
- The limits: missing source audio, absent render output, unstated licence and static-only
  evidence.

It does not run Needle Drop. It does not start Blender, build a `.blend` file, render a frame,
mix audio or encode a film. Every runtime number comes from source text or from a data file.
The exploration marks such numbers as not verified when no output exists.

## 3. Documents

The package uses the standard six-document layout.

| File | Role | Words | Snippets |
|---|---|---:|---:|
| `markdown/README.md` | Index, thesis, map, reading order, non-claims | no minimum | 3 or more |
| `markdown/00-overview.md` | The film, the six shots, the three sets and the parts | 3,000 to 5,000 | 10 or more |
| `markdown/01-architecture.md` | Processes, data flow, state ownership and storage | 3,000 to 5,000 | 10 or more |
| `markdown/02-core-mechanism.md` | One frame from beat grid to graded pixel, traced end to end | 3,000 to 5,000 | 10 or more |
| `markdown/03-workflow-and-quality.md` | One shot from first build to accepted frame, then the film | 3,000 to 5,000 | 10 or more |
| `markdown/04-limits-and-comparison.md` | Limits, rejected choices, failure modes and comparisons | 3,000 to 5,000 | 10 or more |

Every deep dive has at least two Mermaid diagrams, at least one `**Aha:**` insight, and a
citation header on every code block. The citation header gives the source path relative to the
needle-drop root and the line range.

The central mechanism for document 02 is the drop morph in `scenes/set_c/c_terrain.py` and
`scenes/set_c/build.py`. The mechanism turns a marble dune field into a record groove field.
It uses a front radius, a spring-overshoot curve, three kick rings, a view cull and a static
copy of the disc. The same document traces the frame from `data/timing.json` through the
camera, the morph, the effects, the EEVEE render, the compositor and the encode.

## 4. Constraints

- The source is read only. No needle-drop file is modified.
- No Blender build, render, probe, asset download, audio mix or encode is run.
- No network request is made. No dependency is installed.
- Prose uses Aerospace and Defence Simplified Technical English, issue 100 (ASD-STE100). One
  idea per sentence. Thirty words maximum. Expand every abbreviation at first use in each file.
- Prose avoids marketing language, em dashes, "simply", "just", and "obviously".
- A value that the source does not define is recorded as not defined.
- The pin is `8982f09`. Runtime behaviour is marked as not verified.
- A quoted line must match the source byte for byte.
- The exploration does not repeat the build-process product names.

## 5. Research questions

1. What does the film show, and how does one visual idea survive four scales and six shots?
2. How do the production bible and the builder brief disagree, and which one governs the code?
3. How does `data/timing.json` become the time base for three scenes, the audio and the edit?
4. How does Set A build a 30 metre deep groove field, and how does its camera reach the vault?
5. How does Set B build the tonearm, and why does it write a second `.blend` file?
6. How does the `S2_end` handoff make a 0.074 metre record point continue into a 1.2 kilometre
   desert disc?
7. How does Set C morph dunes into record grooves, and how does the front radius select which
   vertices move?
8. How does the same tonearm become a mountain, and which values are cheats?
9. How does the camera system avoid a moving cut, and why does Set C add half-frame keys?
10. How does `tools/brun.sh` keep a render from crashing the laptop or colliding with another
    Blender job?
11. How does the audio chain measure the music, snap the event frames and master the delivery?
12. Where does the static reading stop, and which claims depend on a render that this
    exploration did not perform?

## 6. Source map for the writer

| Area | Start point |
|---|---|
| Project statement and released files | `README.md` |
| Production plan | `docs/bible.md` |
| Builder brief and override list | `docs/BUILDERS.md` |
| Concept list | `docs/concepts.json` |
| Event frames and shot ranges | `data/timing.json` |
| Beat grid and kicks | `data/beats.json` |
| Eight-band audio energy | `data/bands.json` |
| Set A to Set B handoff | `data/handoff.json` |
| Set C layout output | `data/layout_c.json` |
| Shared palette and marble | `tools/nd_lib.py` |
| Shared render and material helpers | `tools/common.py` |
| Safe Blender runner | `tools/brun.sh`, `tools/foreign.ps1` |
| Frame renderer | `tools/render_frames.py` |
| Chunked render loop | `tools/render_seq.sh` |
| Master and smear pass | `tools/master.sh` |
| Encode and grain | `tools/assemble.sh`, `tools/encode.sh` |
| Encode verification | `tools/verify_encode.sh` |
| Contact sheets | `tools/contact.sh` |
| Reference mix | `tools/mix.js` |
| Set A build | `scenes/set_a/build.py` |
| Set A camera and field | `scenes/set_a/design.py` |
| Set A assets | `scenes/set_a/assets_a.py`, `fetch_assets.py` |
| Set A path checks | `scenes/set_a/check_path.py` |
| Set B build | `scenes/set_b/build.py` |
| Set B probes and checks | `scenes/set_b/*.py`, except `build.py` |
| Set C entry | `scenes/set_c/build.py` |
| Set C shared constants | `scenes/set_c/c_common.py` |
| Set C terrain and morph | `scenes/set_c/c_terrain.py` |
| Set C camera | `scenes/set_c/c_cam.py` |
| Set C look | `scenes/set_c/c_look.py` |
| Set C props and tonearm | `scenes/set_c/c_props.py` |
| Set C effects | `scenes/set_c/c_fx.py` |
| Set C node helper | `scenes/set_c/c_nodes.py` |
| Audio chain | `audio/scripts/build_audio.sh` |
| Audio analysis | `audio/scripts/audio_analyze.py` |
| Music edit | `audio/scripts/edit_music.py` |
| Cue build | `audio/scripts/build_cues.py`, `audio/cues.json` |
| Delivery master | `audio/scripts/master_linear.py` |
| Audio checks | `audio/scripts/verify_mix.py`, `measure.py`, `hero_check.py` |
| Credits and source links | `audio/CREDITS.txt` |

## 7. Task table

| Task | State |
|---|---|
| Verify pin, file count, licence and absence of a manifest | DONE |
| Map scenes, tools, data, audio and documents | DONE |
| Write `spec.md` | DONE |
| Write the six documents | DONE |
| Snippet check, sentence check, link check | DONE |
| Fresh-reader review | DONE |
| Repair reported gaps | DONE |
| Build HyperText Markup Language (HTML) | DONE |
| Commit and push | DONE |

## 8. Resume point

A later reader starts at `README.md` for the film and the released files, then at
`docs/BUILDERS.md` for the rules that override the bible. The time base is
`data/timing.json`. The visual path starts at `scenes/set_a/build.py`, moves through
`scenes/set_b/build.py` and `data/handoff.json`, and ends at `scenes/set_c/build.py` and
`scenes/set_c/c_terrain.py`. The frame path starts at `tools/brun.sh`, moves through
`tools/render_frames.py` and the set cameras, and ends in the EEVEE compositor. The sound path
starts at `audio/scripts/audio_analyze.py`, moves through `edit_music.py`, `build_cues.py` and
`master_linear.py`, and ends at `audio/mix.wav`.

If the checkout moves from `8982f09`, run the snippet checker again.

## 9. Limits

- Static reading only. No frame was rendered, no film was viewed and no audio was measured.
- The film, the stills' source renders and the `.blend` files are not in the repository.
  `.gitignore` ignores `*.blend`, `*.mp4`, `renders/` and `logs/`.
- The source audio is not in the repository. The Mixkit licence forbids redistribution.
  `audio/music_edit_map.json` and `audio/cues.json` hold the edit decisions, not the samples.
- The downloaded Poly Haven assets are not in the repository. The build scripts fetch them.
- No Blender was opened. Geometry Node behaviour, shader compilation, EEVEE settings and
  timings are not verified.
- The NumPy paths were read, not executed. Numeric output comes from constants in the source.
- The audio scripts were read, not executed. Loudness, peak and cue alignment are not verified.
- The build is not reproducible here. It needs Blender 5.2.2, ffmpeg, Node.js and the network.
- The released files on GitHub were not downloaded or inspected.

## 10. Corrections and limits found during research

The inventory matched: pin `8982f09`, 100 tracked files, no licence file and no manifest.
The following findings qualify the first reading.

1. **The README says "everything is code", but the scenes load external assets.** The README
   claims "No hand modelling". The Set C builder calls `c_props.trees`, `c_props.boulders` and
   `c_props.load_model`, and those functions download and place Poly Haven models. Set B loads
   the `studio_small_09` HDRI. The correct claim is that no bespoke model is hand built. The
   film still depends on CC0 models and textures.
2. **The marble is one field construction, not one fixed mapping.** The README says "One
   marble, everywhere". `tools/nd_lib.py` gives one `build_marble_field` function. Each set
   maps it differently. Set A calls it with `scale=0.05` and `warp=1.3` (`build.py:277`). Set B
   uses record coordinates at scale 12 and band frequency 4.4. Set C uses the handoff mapping
   at scale 0.001578855 and frequency 4.4. The shared part is the node construction and the
   colour stops, not the numbers.
3. **The bible is not the build specification.** `docs/BUILDERS.md:2` states that it overrides
   the bible wherever the two disagree. The bible plans `scripts/probe.py`,
   `scripts/safe_launch.ps1`, `scripts/render_queue.ps1` and `scripts/assemble.ps1`. Those four
   files do not exist. The real tools are `tools/brun.sh`, `tools/foreign.ps1`,
   `tools/render_seq.sh`, `tools/render_frames.py`, `tools/master.sh` and `tools/assemble.sh`.
4. **`data/timing.json` is the data of record for time, and some events differ from the
   bible.** `lint_pass` is frame 49 at 2.0 seconds; the bible used 1.9 seconds and frame 46.
   `fade_in_end` is frame 7; the bible used frame 6. `shadow_enters` is frame 613; the bible
   used 615. The note in `timing.json` says the audio stage snapped events to detected beats.
   The docs must use the JSON values and state the divergence.
5. **The delivered film is 708 frames, not 720.** `timing.json` gives
   `"rendered_frames": [1, 708]`. `docs/BUILDERS.md` says frames 709 to 720 are black and are
   added at encode. The README says "720 frames". Both are correct for different products.
6. **The loudness target is -14.3 LUFS, not -14.0.** `README.md` and the bible state -14 LUFS.
   `audio/scripts/master_linear.py:89` sets `TARGET_I = -14.3` and explains that the margin
   reduces limiting at the hits.
7. **The delivery mix comes from `master_linear.py`, not `tools/mix.js`.** `build_audio.sh`
   calls `tools/mix.js` to write `audio/_work/mixjs_reference.wav` and marks it "REFERENCE
   ONLY". The delivery file is `audio/mix.wav`, written by `master_linear.py` with a static
   gain and a latency-compensated peak limiter.
8. **Set C splits the record into three objects.** The live `RecordDisc` with the morph
   modifier is visible to frame 516. A `RecordStatic` copy is visible from frame 517. A
   `RecordPatch` sector covers the close view and is visible to frame 541. The split keeps the
   per-frame Geometry Node cost off 3.6 million vertices after the morph completes.
9. **The giant tonearm is the Set B asset, scaled 8000 times.** `c_props.tonearm` appends
   `blend/tonearm_asset.blend`, renames the collection, rotates it 90 degrees about Z and sets
   scale to `ARM_LEN / 0.235`, where `ARM_LEN` is 1880 metres. A cheat table moves parts to
   keep the silhouette correct at that scale.
10. **The Set C camera adds half-frame keys at the cut.** `c_cam.bake_camera` keys every
    integer frame from 289 to 708. It also keys `CUT - 0.01` and `CUT + 0.01`, where `CUT` is
    432.5. The extra keys stop motion blur from mixing the top-down S3 frame with the low S4
    frame.
11. **The safe runner is Windows-specific.** `tools/brun.sh` calls PowerShell through
    `cygpath`, uses `taskkill`, and reads free memory through a Windows command. The README
    describes the behaviour but not the platform dependence. The script cannot run on macOS or
    Linux without changes.
12. **No licence is stated.** The container study records `needle-drop` as not stated. The
    README gives credits for assets and music but no licence for the code or the film.
13. **Some source claims are design intent, not code.** The bible gives vertex budgets,
    sample counts and timings as targets. The builder brief restates some as maxima. The code
    contains the final values. A reader must not treat a bible number as a measured result.
14. **The film is not reproducible from the checkout alone.** The audio, the Poly Haven
    assets and the rendered frames are absent. The build needs a network connection, Blender
    5.2.2, ffmpeg and Node.js.

## 11. Verification record

The record fills after the writer and the fresh-reader review.

| Check | Target | Result |
|---|---|---|
| Six documents present | README plus five deep dives | DONE: 6 documents, 108 snippets, 0 bad |
| Words per deep dive | 3,000 to 5,000 | DONE: 4,427, 3,953, 4,998, 4,132 and 4,885 |
| Byte-exact snippets | 10 or more per deep dive, 0 bad | DONE: 15, 17, 33, 16 and 22; README has 5 |
| Mermaid diagrams | 2 or more per deep dive | DONE: 11 across the package |
| `**Aha:**` insights | 1 or more per deep dive | DONE: 7 across the package |
| Sentence length | 0 sentences over 30 words | DONE: 0 long sentences |
| Links | 0 broken relative links | DONE: 24 checked, 0 broken |
| Fresh-reader review | Gaps repaired or recorded | DONE: 116 findings; 115 repaired, 1 rejected as wrong |
| Build | HTML pages generated | DONE: 6 pages plus index and specification |
| Commit and push | No Artificial Intelligence (AI) attribution | DONE: `ADD:` commit, no attribution |

Measured prose words: README 1,727; 00 4,427; 01 3,953; 02 4,998; 03 4,132; 04 4,885.
Total 24,122 words across the six documents.
