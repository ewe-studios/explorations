---
title: feed-backrooms package exploration
source_location: /Users/darkvoid/GitSources/src.aidesign/src.leonnix/feed-backrooms
repository: Git checkout, local pin 81cdb6a
explored_at: 2026-11-16
version: 0.1.0
license: not stated for the code; assets are Creative Commons Zero (CC0) or Mixkit
files: 94
documents: 6
languages: JavaScript, HyperText Markup Language (HTML), Cascading Style Sheets (CSS), JavaScript Object Notation (JSON), Markdown
---

# Specification: feed-backrooms package exploration

## 1. Source

| Property | Value |
|---|---|
| Location | `/Users/darkvoid/GitSources/src.aidesign/src.leonnix/feed-backrooms` |
| Kind | Git checkout with its own history |
| Pin | `81cdb6a` (short local commit identifier) |
| Full pin | `81cdb6a4ac254a2783bd88853a3e3bf96d19cb2a` |
| Commit date | 2026-11-10 02:33:19 +0100 |
| Commit subject | `Add Vercel Analytics` |
| Tracked files | 94, excluding `.git` and `node_modules` |
| Commits | 5 |
| Licence | No root licence file. `README.md` states that textures are CC0 and sound files are CC0 or Mixkit Sound Effects Free License |
| Version | `0.1.0`, `private` |
| Runtime | Node.js with Vite 8; the game runs in a WebGL2 browser |
| Engine | three.js 0.187 |
| Deployment | Vercel, static output in `dist` |

`FEED` is a first-person horror game set in the Backrooms. The player is the monster. Eight
people enter the building. The player must catch all eight before hunger runs out. The game is
built from nine JavaScript modules, one HTML page and 26 texture images and 38 sound files.

The repository has five commits. The first commit holds the game. The other four add phone
support, the Vercel build settings, a link in the README and Vercel Analytics.

## 2. Scope

The exploration reads the source at the pinned commit. It explains the level generator, the baked
light grid, the shared material patch, the prey state machine, the procedural people, the monster
arms, the post-processing stack, the audio system and the trailer recorder.

It does not run the game, build the bundle, deploy the site, or play the trailer. It does not
judge visual quality. Where the source states a value, the exploration quotes the value and the
line. Where the source does not state a value, the exploration says so.

## 3. Document set

| File | Subject |
|---|---|
| `markdown/README.md` | Thesis, map, document table, reading order, non-claims |
| `markdown/00-overview.md` | The game, the loop, the level, the people and one frame |
| `markdown/01-architecture.md` | The nine modules, the scene graph and the shared state |
| `markdown/02-core-mechanism.md` | Level generation, light baking and one simulation tick |
| `markdown/03-workflow-and-quality.md` | Development, build, deploy, trailer recording and checks |
| `markdown/04-limits-and-comparison.md` | Limits, unused assets, determinism and sibling comparison |

Every deep dive has 3,000 to 5,000 words, at least ten byte-exact snippets, at least two Mermaid
diagrams and at least one **Aha:** insight. Every snippet has a `path:line` header and quotes the
source byte for byte. Every document expands each abbreviation at first use.

## 4. Inventory

| Path | Files | Lines | Role |
|---|---:|---:|---|
| `src/` | 9 | 2,699 | Game modules |
| `tools/record.js` | 1 | 437 | Trailer autopilot, capture and offline audio mix |
| `index.html` | 1 | 134 | Page, styles and HUD |
| `public/assets/textures/` | 28 | 0 | 26 texture images and 2 records |
| `public/assets/sfx/` | 40 | 0 | 38 recordings and 2 records |
| `media/` | 4 | 0 | Trailer and three stills |
| `public/` icons | 4 | 0 | Web-app manifest and three icons |
| Root | 5 | 152 | `package.json`, `package-lock.json`, `vite.config.js`, `vercel.json`, `.gitignore` |

The source files are `assets.js` (42 lines), `audio.js` (103), `level.js` (509), `main.js` (728),
`materials.js` (264), `player.js` (296), `post.js` (81), `prey.js` (629) and `util.js` (47).

## 5. Constants

| Constant | Value | Line |
|---|---|---|
| Cell size | 2 metres | `src/level.js:5` |
| Wall height | 3.0 metres | `src/level.js:6` |
| Macro block | 4 cells | `src/level.js:7` |
| Light-grid texels per cell | 2 | `src/level.js:8` |
| Macro grid | 12 by 12, giving 49 by 49 cells and 98 metres | `src/level.js:11`, `src/main.js:321` |
| Eye height | 2.14 metres | `src/player.js:26` |
| Field of view | 80 degrees | `src/main.js:36` |
| Walk and run speed | 3.5 and 7.2 metres per second | `src/player.js:211` |
| Hunger drain | 1/150 per second | `src/main.js:625` |
| Hunger per kill | 0.36 | `src/main.js:278` |
| Prey count | 8 | `src/main.js:65` |
| Roar cooldown | 11 seconds | `src/player.js:245` |
| Freeze radius | 14 metres | `src/prey.js:303` |
| Scent-trail life | 32 seconds | `src/main.js:168` |
| Blood particles | 260 | `src/main.js:95` |
| Render budget | 2.3, 1.1 or 0.55 megapixels | `src/main.js:49` |
| Trailer | 45 seconds at 60 ticks per second; every second frame kept | `tools/record.js:16`, `:406-420` |
| Trailer kills | 3 | `tools/record.js:18` |

## 6. Research questions

| # | Question |
|---|---|
| 1 | How does the game turn a folder of primitives into a playable horror scene? |
| 2 | How does the level generator guarantee that every room is reachable? |
| 3 | How does the baked light grid replace real-time lights? |
| 4 | How does one material patch serve walls, floors, people and claws? |
| 5 | Which state machine drives the eight people, and what makes each state end? |
| 6 | How does the trailer recorder film the real game and mix its sound offline? |
| 7 | What does the project not do, and which files does it not use? |

## 7. Constraints

- Aerospace and Defence Simplified Technical English (ASD-STE100) applies to every sentence.
- The source is read-only. No file in the source tree is changed.
- No network access. No dependency is installed. No build or test runs.
- One package is in flight. At most two subagents work on this package.
- A fresh-reader review runs before the package is done. The writer fixes every reported gap
  that the source supports.

## 8. Task table

| # | Task | State |
|---|---|---|
| 1 | Write `spec.md` | DONE |
| 2 | Write the six documents | DONE (25,152 words) |
| 3 | Validate snippets against the source | DONE (84 snippets, 0 bad) |
| 4 | Validate Simplified Technical English | DONE (0 long sentences) |
| 5 | Validate links | DONE (57 links, 0 broken) |
| 6 | Fresh-reader review and repair | DONE (85 findings, all repaired) |
| 7 | Build HyperText Markup Language (HTML) | DONE |
| 8 | Update the container index and spec | DONE |
| 9 | Commit and push | DONE |

## 9. Resume point

Read the task table. The first `TODO` row is the next action. A task is complete only when its
check passes and the result is recorded in the verification table.

## 10. Verification record

| Check | Result |
|---|---|
| Documents | 6 documents, 25,152 words. Deep dives: 00 4,554; 01 4,478; 02 4,992; 03 4,274; 04 4,858. README 1,996 |
| Snippets | 84 snippets, 84 byte-exact, 0 bad |
| Diagrams and insights | 12 Mermaid diagrams; 7 **Aha:** insights |
| Simplified Technical English | 0 long sentences |
| Links | 57 links, 0 broken |
| Fresh-reader review | 85 findings reported; all 85 repaired. Two further errors found during verification and repaired: the doorway diagram label and the direction of the flicker threshold |
| Build | HyperText Markup Language (HTML) built at `html/`, 8 pages plus `styles.css` |
| Commit | `ADD:` commit with no attribution |
