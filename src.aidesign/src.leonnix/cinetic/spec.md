---
title: cinetic package exploration
source_location: /Users/darkvoid/GitSources/src.aidesign/src.leonnix/cinetic
repository: Git checkout, local pin f4bbfc8
explored_at: 2026-11-15
version: 1.5.1
license: MIT
files: 964
documents: 6
languages: Markdown, JavaScript, TypeScript, Python, Shell, HyperText Markup Language (HTML), Cascading Style Sheets (CSS)
---

# Specification: cinetic package exploration

## 1. Source

| Property | Value |
|---|---|
| Location | `/Users/darkvoid/GitSources/src.aidesign/src.leonnix/cinetic` |
| Kind | Git checkout with its own history |
| Pin | `f4bbfc8` (short local commit identifier) |
| Tracked files | 964 |
| Version | `1.5.1` (`.claude-plugin/plugin.json:5`, `CHANGELOG.md:27`) |
| Licence | MIT (`LICENSE`). The skill copy repeats it at `skills/cinetic/LICENSE.txt` |
| Skill root | `skills/cinetic/` |
| Node.js | 22 or later (`README.md:189`) |
| ffmpeg | 6 or later with libx264, plus ffprobe (`README.md:190`) |
| Python | 3.11 or later with numpy, scipy, soundfile, pyloudnorm, opencv-python, librosa (`README.md:191`) |
| Sound tool | earshot 0.4 or later, in its own virtual environment (`README.md:192`) |
| Engines | Remotion 4.0.529 by default; HyperFrames 0.8.79 as the HTML mirror (`skills/cinetic/SKILL.md:19`) |

cinetic (cinema plus kinetic) is an Agent Skill that makes a coding agent direct and
render a short motion film from code. The two main products are the launch film and
the update video. It also covers product videos, feature loops, logo stings, and UI
walkthroughs. The skill does not supply animation components. It supplies a
production system: a script contract, a scene bank, a direction deck, a music
pipeline, a sound-effect kit, gates, and a review pass.

The tracked files divide like this:

| Path | Files | Role |
|---|---:|---|
| `skills/cinetic/assets` | 510 | Starters, templates, critics, directions, libraries, sound material |
| `skills/cinetic/scripts` | 334 | Python, Node.js, and Shell tools, plus their tests |
| `skills/cinetic/references` | 93 | Thirty reference documents and three device and library indexes |
| `skills/cinetic/SKILL.md` | 1 | The skill contract |
| `skills/cinetic/LICENSE.txt` | 1 | Licence copy |
| `docs` | 15 | Setup, evaluation, brand files |
| `.claude-plugin` | 2 | Plugin and marketplace manifests |
| `.github` | 2 | Workflow files |
| Root files | 5 | `README.md`, `CHANGELOG.md`, `LICENSE`, `.gitignore`, `.gitattributes` |
| `evals/evals.json` | 1 | Seven evaluation briefs |

The asset folder divides into 328 files in `assets/remotion-starter`, 157 in
`assets/templates`, 13 in `assets/hyperframes-starter`, 6 critics, 2 direction
data files, 1 library data file, 1 music data file, and 2 root files. The
reference folder divides into 30 root documents, 27 device files, 27 library
files, and 9 motion-library files. The script folder divides into 55 root
scripts, 14 audio scripts, and 265 test files.

## 2. Scope

This package covers the skill as a production system. It covers these areas:

- The `SKILL.md` contract: the eleven standards, the hard bans, the time plan,
  the eight-step path, the format table, and the craft rules.
- The direction system: `scripts/deck.py`, the direction assets, and the idea
  documents.
- The scaffold and template system: `scripts/new-film.sh`, `scripts/new-film.mjs`,
  and the eight template folders.
- The scene bank and the deal: `assets/remotion-starter/src/bank.ts`,
  `src/deal.ts`, `src/dealt.ts`, and `scripts/film-diff.mjs`.
- The copy contract: `scripts/copy-check.py` and its rules.
- The picture checks: `scripts/lint-film.mjs`, `scripts/grid-check.ts`,
  `scripts/forensics.py`, and `scripts/look-audit.py`.
- The sound pipeline: `scripts/audio/` (fourteen files), `scripts/music.mjs`,
  `scripts/check-sync.py`, and earshot.
- The render and delivery path: `scripts/render.sh`, `scripts/frames.mjs`,
  `scripts/deliver.sh`, and `scripts/av-audit.py`.
- The review loop and the six critic files.
- The evaluation record in `docs/evaluation.md` and `evals/evals.json`.

It does not render a film. It does not run a checker against a real film. It does
not fetch a music track, a photograph, or a logo. It does not call earshot. No
network request is made.

## 3. Documents

The package uses the standard six-document layout.

| File | Role | Words | Snippets |
|---|---|---:|---:|
| `markdown/README.md` | Index, thesis, map, reading order | no minimum | 3 or more |
| `markdown/00-overview.md` | What cinetic is, the user, the shipped parts, the evaluation | 3,000 to 5,000 | 10 or more |
| `markdown/01-architecture.md` | Skill layout, process boundaries, engines, state, storage | 3,000 to 5,000 | 10 or more |
| `markdown/02-core-mechanism.md` | The timeline, the deal, and the gates traced end to end | 3,000 to 5,000 | 10 or more |
| `markdown/03-workflow-and-quality.md` | One film from brief to master, with every check | 3,000 to 5,000 | 10 or more |
| `markdown/04-limits-and-comparison.md` | Limits, rejected choices, failure modes, comparison to taste-skill | 3,000 to 5,000 | 10 or more |

Every deep dive has at least two Mermaid diagrams, at least one `**Aha:**`
insight, and a citation header on every code block. The citation header gives the
source path relative to the cinetic root and the line range.

## 4. Constraints

- The source is read only. No skill file is modified.
- No render, gate, checker, installer, fetch, or test is run.
- No network request is made.
- Prose uses Aerospace and Defence Simplified Technical English, issue 100
  (ASD-STE100). One idea per sentence. Thirty words maximum. Expand every
  abbreviation at first use in each file.
- Prose avoids marketing language, em dashes, "simply", "just", and "obviously".
- A value that the source does not define is recorded as not defined.
- The pin is `f4bbfc8`. Runtime behaviour is marked as not verified.
- A quoted line must match the source byte for byte.

## 5. Research questions

1. What does cinetic ship that a plain "make a launch video" prompt does not?
2. How does the skill turn a creative judgement into a measurable gate?
3. How does one timeline file become the single source of timing for picture,
   sound, and checks?
4. How does the deal make each film differ from the three before it?
5. Why is the music chosen before the picture, and what does that order change?
6. What can the gates measure, and what can they not measure?
7. Where does cinetic agree with taste-skill, and where does it differ?

## 6. Source map for the writer

| Area | Start point |
|---|---|
| Skill contract | `skills/cinetic/SKILL.md:1` |
| The eleven standards | `skills/cinetic/SKILL.md:18` |
| Hard bans | `skills/cinetic/SKILL.md:81` |
| Time plan | `skills/cinetic/SKILL.md:102` |
| The eight-step path | `skills/cinetic/SKILL.md:115` |
| Format table | `skills/cinetic/SKILL.md:183` |
| Craft rules | `skills/cinetic/SKILL.md:199` |
| Direction deck | `skills/cinetic/scripts/deck.py` |
| Direction data | `skills/cinetic/assets/directions/directions.json` |
| Scaffold (shell) | `skills/cinetic/scripts/new-film.sh` |
| Scaffold (Node.js) | `skills/cinetic/scripts/new-film.mjs` |
| Template list | `skills/cinetic/references/templates.md` |
| Template folders | `skills/cinetic/assets/templates/` |
| Scene bank | `skills/cinetic/assets/templates/launch-simple/src/bank.ts` |
| Deal logic | `skills/cinetic/assets/templates/launch-simple/src/deal.ts` |
| Dealt scene data | `skills/cinetic/assets/templates/launch-simple/src/dealt.ts` |
| Film history | `skills/cinetic/scripts/film-diff.mjs` |
| Copy contract | `skills/cinetic/scripts/copy-check.py` |
| Copy rules | `skills/cinetic/references/copy-and-type.md` |
| Code lint | `skills/cinetic/scripts/lint-film.mjs` |
| Grid and spine check | `skills/cinetic/scripts/grid-check.ts` |
| Frame forensics | `skills/cinetic/scripts/forensics.py` |
| Look audit | `skills/cinetic/scripts/look-audit.py` |
| Render driver | `skills/cinetic/scripts/render.sh` |
| Frame capture | `skills/cinetic/scripts/frames.mjs` |
| Sound kit | `skills/cinetic/scripts/audio/kit.py` |
| Sound plan | `skills/cinetic/scripts/audio/plan.py` |
| Score and master | `skills/cinetic/scripts/audio/score.py` |
| Sound catalogue | `skills/cinetic/scripts/audio/sfx-catalogue.json` |
| Music choice | `skills/cinetic/scripts/music.mjs` |
| Music analysis | `skills/cinetic/scripts/audio/track.py` |
| Sync check | `skills/cinetic/scripts/check-sync.py` |
| Audio and video audit | `skills/cinetic/scripts/av-audit.py` |
| Delivery | `skills/cinetic/scripts/deliver.sh` |
| Review loop | `skills/cinetic/references/review-loop.md` |
| Critics | `skills/cinetic/assets/critics/director.md` |
| All gates | `skills/cinetic/references/checks.md` |
| Evaluation record | `docs/evaluation.md` |
| Evaluation briefs | `evals/evals.json` |
| Setup | `docs/setup.md` |
| Plugin version | `.claude-plugin/plugin.json:5` |
| Changelog | `CHANGELOG.md:27` |

## 7. Task table

| Task | State |
|---|---|
| Verify pin, file count, licence, version | DONE |
| Map skill folders, scripts, assets, references | DONE |
| Write `spec.md` | DONE |
| Write the six documents | DONE |
| Snippet check, sentence check, link check | DONE; results in section 11 |
| Fresh-reader review | DONE; 15 material contradictions and 40 depth gaps reported |
| Repair reported gaps | DONE; every verified item repaired or recorded |
| Build HyperText Markup Language (HTML) | DONE |
| Commit and push | DONE |

## 8. Resume point

A later reader starts at `skills/cinetic/SKILL.md` for the contract, at
`skills/cinetic/scripts/new-film.sh` for the scaffold, and at
`skills/cinetic/scripts/render.sh` for the render path. The gate list is
`skills/cinetic/references/checks.md`. The sound path starts at
`skills/cinetic/scripts/audio/plan.py`.

If the checkout moves from `f4bbfc8`, run the snippet checker again.

## 9. Limits

- Static reading only. No film, gate, render, or fetch was run.
- The evaluation results in `docs/evaluation.md` are the project's own report.
  The exploration repeats them and does not confirm them.
- The seven evaluation briefs are documents. Their graded outcomes are not
  reproduced here.
- The gates depend on a working Remotion or HyperFrames project, ffmpeg, Python
  packages, Chromium, and earshot. None of these was installed or run.
- The skill makes network requests for music, photographs, and logos in normal
  use. No such request was made here.
- The 964 tracked files include 265 test files. A file count is not a measure of
  the skill text.

## 10. Corrections and limits found during research

The inventory matched: pin `f4bbfc8`, 964 tracked files, MIT licence, version
1.5.1. The following findings qualify the first reading.

1. **The skill is not the whole repository.** 939 of the 964 tracked files are
   under `skills/cinetic/`. The root files, `docs`, `evals`, and the plugin
   manifests make up the rest.
2. **Most script files are tests.** 265 of the 334 script files are under
   `skills/cinetic/scripts/test/`. The working tools are 55 root scripts and 14
   audio scripts.
3. **Two engines share one contract.** Remotion 4.0.529 is the default.
   HyperFrames 0.8.79 mirrors the tokens and the timeline contract. A reader must
   not assume that every script supports both.
4. **The version has an uncommitted section.** `CHANGELOG.md:9` starts an
   `[Unreleased]` section above `1.5.1`. A reader must not read the unreleased
   notes as part of the tagged version.
5. **The evaluation is not a win record.** `docs/evaluation.md:5` states that the
   neutral blind judge was split: 7 wins in 23 comparisons. It also states that a
   cinetic run costs about three times the time and tokens of the baseline. The
   exploration must repeat both facts.
6. **Three of the seven briefs were never judged.** `docs/evaluation.md:15`
   states that briefs five, six, and seven were added after the judged rounds.
7. **The hard bans are not absolute.** A supplied brand overrides a ban when the
   brand file records the exception. The source shows this with the Quire
   paper-cream case in `docs/evaluation.md`.
8. **The 1.5.1 changelog entry is short.** `CHANGELOG.md:27` lists four
   contributions and no metrics. The larger 1.5.0 entry holds the detailed
   claims.
9. **No runtime claim is verified.** No film, gate, render, sound pass, or fetch
   was run. Statements about output quality come from project documentation and
   source structure.
10. **The skill depends on a sibling project.** earshot is a separate checkout in
    this container. cinetic calls it but does not vendor it.
11. **The Python floor has two values.** `README.md:191` requires Python 3.11 or
    later. `README.md:205` and `skills/cinetic/scripts/py.mjs:10-11` accept 3.10
    or later. The source does not reconcile the two values.
12. **The copy deadline has two values.** `copy-check.py:2` says a stranger must
    know the product within 5 seconds. `copy-check.py:105` sets `WHAT_BY_S = 3.0`,
    and the check enforces 3 seconds.
13. **The interpreter diagnostics and the selector test different packages.**
    `frames.mjs:50-53` tests numpy, scipy, soundfile and cv2. `py.mjs` prefers an
    interpreter with numpy, scipy and cv2. `scripts/requirements.txt` lists more
    packages than either check.
14. **The frame rate has two values.** `SKILL.md:196` states 30 frames per second.
    The templates ship with `FPS = 60`, and the evaluation expects 60 when a brief
    gives no rate (`docs/evaluation.md:187`). The scaffold rewrites the template
    constant from `--fps`, whose default is 30. The source does not reconcile the
    contract and the evaluation.
15. **The second-round rule has two versions.** `SKILL.md:176` allows a second
    round only after a gate fail. `review-loop.md:73` also allows one after a fixed
    P0. The review guide is wider than the short path.
16. **The changelog count is off.** `CHANGELOG.md:27-31` says "Three community
    contributions" and lists four changes. The entry credits pull requests 4, 8 and
    3, and gives no metrics.
17. **The judged version is not the shipped version.** `docs/evaluation.md:124`
    and `docs/evaluation.md:189` state that rounds 6 and 7 tested 1.0.0 snapshots.
    The fixes from those rounds shipped after the judged snapshot. No judged round
    covers the release of 1.0.0 or version 1.5.1.
18. **The fourth history weight has no stated use.** `bank.ts:154` seeds the deal
    with the last three films. `bank.ts:158` holds four weights, and `bank.ts:168`
    can apply all four. The source does not say when the fourth value applies.
19. **The HyperFrames path is a mirror, not a peer.** The source states that
    HyperFrames mirrors the tokens and the timeline contract (`SKILL.md:13`). The
    starter has 13 files against 328 in the Remotion starter. Full feature parity
    is not established by the source.

## 11. Verification record

The record fills after the writer and the fresh-reader review.

| Check | Target | Result |
|---|---|---|
| Six documents present | README plus five deep dives | 6 |
| Words per deep dive | 3,000 to 5,000 | 4,983; 4,898; 4,998; 4,999; 4,995 |
| Byte-exact snippets | 10 or more per deep dive, 0 bad | 63; 0 bad |
| Mermaid diagrams | 2 or more per deep dive | 11 |
| `**Aha:**` insights | 1 or more per deep dive | 6 |
| Sentence length | 0 sentences over 30 words | 0 |
| Links | 0 broken relative links | 0 |
| Fresh-reader review | Gaps repaired or recorded | 15 material contradictions and 40 depth gaps; all verified items repaired |
| Build | HyperText Markup Language (HTML) pages generated | 9 pages |
| Commit and push | No Artificial Intelligence (AI) attribution | Done |

The writer corrected these items after the fresh-reader review: the
HyperFrames timeline, the own-scene model, the pinned-scene sentence, the
`GRID_BPM` wording, the ship-gate tension, the state-file counts, the
hard and soft devices, the type roles, the two lookbacks, the interpreter
selector, the bank entry terms, the gate vocabulary, the severity levels, the
reference frame, and the PARTIAL sound result. The writer rejected three reader
claims with source evidence: the actor count, the history weights and the
Remotion licence. The package records the source defects at section 10.
