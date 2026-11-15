---
title: earshot package exploration
source_location: /Users/darkvoid/GitSources/src.aidesign/src.leonnix/earshot
repository: Git checkout, local pin 4ac3d2a
explored_at: 2026-11-16
version: 0.4.1
license: MIT for code and documents; models and evaluation data have their own terms
files: 287
documents: 6
languages: Python, JavaScript, Markdown, JSON
---

# Specification: earshot package exploration

## 1. Source

| Property | Value |
|---|---|
| Location | `/Users/darkvoid/GitSources/src.aidesign/src.leonnix/earshot` |
| Kind | Git checkout with its own history |
| Pin | `4ac3d2a` (short local commit identifier) |
| Tracked files | 287 |
| Version | `0.4.1` (`pyproject.toml:7`, `earshot/__init__.py:15`) |
| Licence | MIT (`LICENSE:1`, `pyproject.toml:11`) |
| Models | CLAP `laion/larger_clap_general` under Apache-2.0; AST `MIT/ast-finetuned-audioset-10-10-0.4593` under BSD-3-Clause (`README.md:427-430`) |
| Evaluation data | Licensed recordings and third-party films. Not redistributed. Only derived numbers are committed (`eval/README.md:20-38`) |
| Python | 3.10 or later (`pyproject.toml:10`) |
| Main dependencies | numpy, scipy, soundfile, soxr, librosa, pyloudnorm, scikit-learn, pillow, matplotlib, torch, transformers (`pyproject.toml:24-35`) |
| Optional dependencies | `mcp` for the Model Context Protocol server; `requests` for the audio large language model critique; `pytest` for development (`pyproject.toml:43-46`) |
| Entry point | `earshot = "earshot.cli:main"` (`pyproject.toml:49`) |

Earshot gives coding agents "ears". An agent can write code that makes a film, a game or a product
demo, but it cannot hear the result. Earshot turns sound into evidence that an agent can read:
calibrated text-to-audio match scores, the best moment inside a long recording, similarity to real
reference recordings, a real-versus-synthesized judgement, a second-by-second listening transcript,
an annotated image, and a launch-video scorecard. It also makes sounds from code (`earshot forge`),
translates a recording into code (`earshot fit`), and renders a bank of sound effects for a film
(`earshot sfx`, `earshot kit`). It runs on the central processing unit (CPU) and offers a command
line interface, a Python application programming interface (API), and a Model Context Protocol
(MCP) server.

The tracked files divide like this:

| Path | Files | Role |
|---|---:|---|
| `earshot` | 115 | The Python package: analysis, forging, fitting, bank, tools |
| `docs` | 14 | Design, validation, limits, integration and sound-design records |
| `eval` | 76 | Evaluation scripts, raw numbers and written results |
| `skills` | 34 | Two agent skills with scripts, templates and prompts |
| `tests` | 42 | Pytest suite, fast and slow |
| Root files | 6 | `README.md`, `CHANGELOG.md`, `LICENSE`, `pyproject.toml`, `.gitignore`, `.gitattributes` |

The package folder divides into 24 root Python modules, 4 model wrappers, 4 data files and the
`forge` folder. `forge` holds 14 root files, 33 recipe modules, 19 seed files, 9 fit files, 4 bank
files and 4 kit presets. The `eval` folder divides into 30 forge files, 15 semantics files, 13
timeline files, 10 realness files, 6 launch files and 2 root files. The `skills` folder holds the
`earshot` skill (1 file) and the `real-sound` skill (33 files). The `docs` folder holds 11 root
documents, 1 history file and 2 images.

## 2. Scope

This package covers earshot as a system. It covers these areas:

- The core: audio loading and resampling, objective features, video frame and cut access, report
  formatting, configuration, and the model wrappers.
- The listening operations: `describe`, `match`, `rank`, `find`, `compare`, `ab`, `realness`,
  `timeline`, `sheet`, `critique` and `profile`.
- The vocabulary and the calibration: 322 descriptions in 16 categories, five prompt templates,
  the term-bias removal, the per-sound z score and the verdict bands.
- The realness detector: its features, its grouped training, its held-out evaluation and its tone
  guard.
- The timeline and the sheet: event detection, tags, sections, loudness, beat grid, sync to cuts
  and cue frames, and the annotated image.
- The launch scorecard: the reference profile from seven professional films, the metric ranges and
  the timestamped advice.
- Forge: physical synthesis in code, the recipe contract, the critic, the model-free tone checks,
  the search loop and the bank.
- Fit: modal, sine and spectral fitters, quality metrics, recipe code generation, takes and the
  replication benchmark.
- The sound bank and kits: 252 sounds rendered by identifier, the `sfx` aim and audit, and the
  `kit` placement against a music bed and picture cuts.
- The interfaces: the command line, the Python API, the MCP server, the two agent skills, and the
  optional Gemini critique.
- The quality record: `docs/VALIDATION.md`, `docs/LIMITS.md`, the evaluation folders, the
  changelog, and the test suite.

It does not run earshot. It does not load the models, analyse audio, render a sound, run an
evaluation, call a network service or listen to any file. All runtime numbers come from the
project documents and are marked as not verified.

## 3. Documents

The package uses the standard six-document layout.

| File | Role | Words | Snippets |
|---|---|---:|---:|
| `markdown/README.md` | Index, thesis, map, reading order | no minimum | 3 or more |
| `markdown/00-overview.md` | The problem, the operations, one listening pass, the shipped parts | 3,000 to 5,000 | 10 or more |
| `markdown/01-architecture.md` | Module graph, core, operations, forge, fit, bank, interfaces, data | 3,000 to 5,000 | 10 or more |
| `markdown/02-core-mechanism.md` | Bytes to evidence, and a recipe to sound, traced end to end | 3,000 to 5,000 | 10 or more |
| `markdown/03-workflow-and-quality.md` | One listening request and one forge run, with every check | 3,000 to 5,000 | 10 or more |
| `markdown/04-limits-and-comparison.md` | Limits, failure modes, untested domains, comparison | 3,000 to 5,000 | 10 or more |

Every deep dive has at least two Mermaid diagrams, at least one `**Aha:**` insight, and a citation
header on every code block. The citation header gives the source path relative to the earshot root
and the line range.

## 4. Constraints

- The source is read only. No earshot file is modified.
- No command, analysis, model load, sound render, fit, evaluation or test is run.
- No network request is made. No model is downloaded.
- Prose uses Aerospace and Defence Simplified Technical English, issue 100 (ASD-STE100). One idea
  per sentence. Thirty words maximum. Expand every abbreviation at first use in each file.
- Prose avoids marketing language, em dashes, "simply", "just", and "obviously".
- A value that the source does not define is recorded as not defined.
- The pin is `4ac3d2a`. Runtime behaviour is marked as not verified.
- A quoted line must match the source byte for byte.

## 5. Research questions

1. What problem does an agent have with sound, and how does earshot turn sound into evidence?
2. How does one file move from bytes to a calibrated verdict: windowing, embedding, vocabulary,
   term bias, z score and verdict band?
3. How do the two models divide the work, and where do objective measurements replace a model?
4. How does `find` search a long recording, and how does it avoid a trim that starts after the
   attack?
5. How does the realness detector separate real recordings from code-synthesized sound, and what
   do its bands and its tone guard mean?
6. How does `timeline` build a listening transcript and measure sync to picture cuts and cue
   frames?
7. How does `critique` build a reference profile from seven films and turn a measurement into a
   timestamped fix?
8. How does a forge recipe become a waveform, and how does the critic steer the search?
9. How does fit invert a recording into a recipe, and which properties survive the round trip?
10. How do the bank, `sfx` and `kit` place a sound at a picture moment without a model?
11. How do the command line, the Python API, the MCP server and the skills share one contract?
12. Which quality gates exist, what do they prove, and what does the project say they cannot prove?

## 6. Source map for the writer

| Area | Start point |
|---|---|
| Package entry | `earshot/__init__.py`, `earshot/__main__.py`, `earshot/cli.py` |
| Configuration | `earshot/config.py` |
| Audio input | `earshot/audio.py` |
| Objective features | `earshot/features.py` |
| Video input | `earshot/video.py` |
| Report format | `earshot/report.py` |
| Python API | `earshot/api.py`, `earshot/_callable.py` |
| Model wrappers | `earshot/models/clap.py`, `earshot/models/ast.py`, `earshot/models/hub.py` |
| Vocabulary | `earshot/data/vocab.json`, `earshot/data/vocab_stats.json`, `earshot/vocab.py` |
| Describe and match | `earshot/describe.py`, `earshot/match.py` |
| Rank, find and compare | `earshot/search.py`, `earshot/compare.py` |
| Realness | `earshot/realness.py`, `earshot/data/realness_model.json` |
| Listening transcript | `earshot/timeline.py` |
| Annotated image | `earshot/sheet.py` |
| Reference profile and scorecard | `earshot/profile.py`, `earshot/critique.py`, `earshot/data/launch_profile.json` |
| Forge core | `earshot/forge/`, `earshot/forge/recipes/`, `earshot/forge/seeds/` |
| Forge bank and kits | `earshot/forge/bank/`, `earshot/forge/kits/` |
| Fit | `earshot/forge/fit/` |
| Static gallery | `earshot/gallery.py` |
| MCP server | `earshot/mcp_server.py` |
| Audio language model | `earshot/llm.py` |
| Install check | `earshot/doctor.py` |
| Agent skills | `skills/earshot/SKILL.md`, `skills/real-sound/` |
| Design and ownership | `docs/DESIGN.md` |
| Validation | `docs/VALIDATION.md`, `eval/README.md` |
| Limits | `docs/LIMITS.md` |
| Forge record | `docs/FORGE_V2.md` |
| Fit record | `docs/FIT.md` |
| Bank record | `docs/SFX.md` |
| Integration record | `docs/INTEGRATION.md` |
| Tests | `tests/` |

## 7. Task table

| Task | State |
|---|---|
| Verify pin, file count, licence, version, runtime | DONE |
| Map package, operations, forge, fit, bank, interfaces, data | DONE |
| Write `spec.md` | DONE |
| Write the six documents | DONE |
| Snippet check, sentence check, link check | DONE: 66 snippets, 0 bad; 0 long sentences; 36 links, 0 broken |
| Fresh-reader review | DONE: two reviews |
| Repair reported gaps | DONE |
| Build HyperText Markup Language (HTML) | DONE |
| Commit and push | DONE |

## 8. Resume point

A later reader starts at `README.md` for the problem and the command map, then at
`docs/DESIGN.md` for the contracts. The listening path starts at `earshot/audio.py`, moves through
`earshot/models/clap.py` and `earshot/vocab.py`, and ends in `earshot/match.py`. The timeline path
starts at `earshot/timeline.py` and continues in `earshot/sheet.py`. The forge path starts at
`earshot/forge/` and its recipe modules. The fit path starts at `earshot/forge/fit/`. The
interface path starts at `earshot/cli.py` and `earshot/mcp_server.py`.

If the checkout moves from `4ac3d2a`, run the snippet checker again.

## 9. Limits

- Static reading only. No audio file was loaded, analysed or listened to.
- No model was downloaded or run. Every model number comes from the project documents.
- The evaluation data is absent. The numbers in the results files cannot be recomputed here.
- The visual results are not viewed. The README describes its own images.
- The forge and fit results are not rendered. Their measured quality comes from the documents.
- The command line and MCP server are not started. Their contracts come from the code.
- The project was developed on one Windows laptop with a Ryzen 7 260 CPU. Other hardware may differ.

## 10. Corrections and limits found during research

The inventory matched: pin `4ac3d2a`, 287 tracked files, version 0.4.1, MIT code. The following
findings qualify the first reading.

1. **The validation record counts 13 MCP tools, but the pin registers 16.**
   `docs/VALIDATION.md:102` says 13 tools. `earshot/mcp_server.py:529-533` lists 13 and
   `earshot/mcp_server.py:607` adds `sfx_list`, `sfx_render` and `kit_render`. The validation
   record predates the three bank tools.
2. **The held-out library is 125 files in one place and 126 in another.**
   `README.md:389` and `docs/VALIDATION.md:5,16` say 125. `docs/DESIGN.md` and `eval/README.md:25`
   say 126 rejected recordings. The source does not reconcile the two counts.
3. **The bank count is a sum of four files.** `forge_v2.json` holds 183 entries,
   `forge04.json` holds 51, `fit.json` holds 15 and `pluck.json` holds 3. The sum is
   `183 + 51 + 15 + 3 = 252` (`README.md:198`).
4. **The package version and the launch film tag differ.** The package is `0.4.1`. The launch
   film is a `v0.4.0` release asset (`README.md:5`). A release tag is not a package version.
5. **The vocabulary size is 322 descriptions in 16 categories.** The number is the sum of the 16
   category lists in `earshot/data/vocab.json`. Five prompt templates wrap a description
   (`earshot/data/vocab.json`, `earshot/vocab.py`).
6. **The test count is a release record.** `docs/VALIDATION.md:119` says 128 passed and 14
   skipped in the fast run. The test files contain many parametrised cases, so the count is not the
   number of `def test_` functions.
7. **The evaluation data is not in the repository.** The scripts read licensed recordings and
   third-party films from environment variables or a gitignored `eval/local_paths.json`
   (`eval/README.md:20-38`). Only derived numbers are committed.
8. **The two models cover different bands.** CLAP hears up to 14 kHz. The AudioSet tagger hears up
   to 8 kHz (`docs/LIMITS.md:48`).
9. **The bf16 path depends on the CPU.** bf16 runs only on CPUs with native bf16 support.
   Elsewhere the package runs fp32 (`docs/LIMITS.md:58-60`).
10. **The optional language model critique was never called live.** Every path was tested against
    a fake Hypertext Transfer Protocol (HTTP) layer (`docs/VALIDATION.md:105-107`).
11. **The project describes a multi-agent build process.** `docs/DESIGN.md:26-38` assigns one owner
    per file. The exploration records this as a project statement and does not verify the process.

## 11. Verification record

The record fills after the writer and the fresh-reader review.

| Check | Target | Result |
|---|---|---|
| Six documents present | README plus five deep dives | Pass: README and five deep dives |
| Words per deep dive | 3,000 to 5,000 | Pass: 4,734, 4,743, 5,000, 5,000 and 5,000 |
| Byte-exact snippets | 10 or more per deep dive, 0 bad | Pass: 11, 16, 13, 12 and 10; 66 total; 0 bad |
| Mermaid diagrams | 2 or more per deep dive | Pass: 2, 3, 2, 2 and 2; 1 in README |
| `**Aha:**` insights | 1 or more per deep dive | Pass: 1, 1, 2, 1 and 1; 1 in README |
| Sentence length | 0 sentences over 30 words | Pass: 0 long sentences in all six files |
| Links | 0 broken relative links | Pass: 36 links checked, 0 broken |
| Fresh-reader review | Gaps repaired or recorded | Pass: two reviews; each gap repaired, verified against the source, or recorded as a limit |
| Build | HTML pages generated | Pass: `python3 -I build.py` made the six documents, the index and the spec |
| Commit and push | No Artificial Intelligence (AI) attribution | Pass: commit and push recorded |
