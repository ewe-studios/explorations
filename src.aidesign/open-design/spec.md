---
source_location: /Users/darkvoid/GitSources/src.aidesign/open-design
repository: none, not a Git checkout
explored_at: 2026-11-15
version: 0.27.2
license: Apache License 2.0
languages: TypeScript, JavaScript, Markdown, Cascading Style Sheets (CSS), HyperText Markup Language (HTML), JavaScript Object Notation (JSON), YAML, Shell, Dockerfile
---

# Specification: OpenDesign package exploration

## 1. Source

The source is `/Users/darkvoid/GitSources/src.aidesign/open-design`.

The source is a directory snapshot dated 2026-11-15. It is not a Git checkout. The directory has no `.git` entry. Therefore this exploration cannot give a commit hash, a branch name, or a remote Uniform Resource Locator (URL).

The package name is `open-design`. The package version is `0.27.2`. The version comes from `package.json:3`. The license is Apache License 2.0. The license comes from `package.json:8` and the `LICENSE` file.

The package is a private monorepo. The package manager is pnpm version 10.33.2. The module system is ECMAScript modules. The Node.js engine range is `~24`.

The source contains no build output in the read paths. It contains application source, content catalogs, tests, deployment files, and documentation.

## 2. Scope

This exploration covers the package as a product and as a system. It covers these areas:

- The web application in `apps/web`.
- The desktop application in `apps/desktop`.
- The daemon in `apps/daemon`.
- The runtime registry and the agent spawn path.
- The prompt composer and its two implementations.
- The artifact manifest and the preview decision.
- The content catalogs in `skills`, `design-templates`, `design-systems`, `craft`, and `prompt-templates`.
- The plugin runtime in `packages/plugin-runtime`.
- The plugin specification and the official plugin catalog.
- The browser clipper, the deployment files, and the Helm chart.
- The end-to-end test layout in `e2e`.

This exploration does not cover these areas in depth:

- The hosted cloud service.
- The release pipeline and code signing.
- The marketplace server.
- The external agent CLIs themselves.
- The quality of generated designs.

## 3. Method

The method had six steps.

1. Read the top-level instruction and product files. These files were `README.md`, `AGENTS.md`, `CLAUDE.md`, `CONTEXT.md`, `QUICKSTART.md`, `package.json`, and the `docs/` index.
2. Map the top-level directories and count their entries.
3. Read the daemon routes for project creation and run creation.
4. Trace one run from the request to the prompt, the spawn, the artifact, and the preview.
5. Trace one plugin from `open-design.json` to manifest parsing, context resolution, validation, and prompt injection.
6. Compare document claims with measured file counts and source behavior.

All code snippets in the deep dives were checked against the source with `/tmp/i2t-fix.py`. The checker compares each fenced block with the cited source range. A mismatch is a failure.

## 4. Documents

| Document | Purpose |
|---|---|
| `markdown/README.md` | Index, thesis, reading order, and non-claims |
| `markdown/00-overview.md` | Product and repository overview |
| `markdown/01-architecture.md` | Process, storage, runtime, and preview architecture |
| `markdown/02-core-mechanism.md` | Prompt composition and plugin resolution |
| `markdown/03-workflow-and-quality.md` | One end-to-end journey and the quality gates |
| `markdown/04-limits-and-comparison.md` | Limits, failure modes, and comparison |

The parent exploration index is the `index.html` one directory above this package. This specification names it once. The other documents do not link outside the package tree.

## 5. Constraints

- The source is read only. No source file was modified.
- No build ran.
- No test ran.
- No package was installed.
- No network request was made to a product service.
- No agent Command Line Interface (CLI) was executed.
- The desktop application was not launched.
- The browser clipper was not loaded.
- Counts are counts of files or directories in the snapshot. A count is not a claim about runtime registration.
- A claim that cannot be checked against a file is marked as not verified.

## 6. Research questions

The exploration answers these questions.

1. What does the package ship, and what does it expect the user to supply?
2. Which process owns state, and where does state live?
3. How does the daemon choose and start an agent CLI?
4. How is the system prompt composed, and which inputs change it?
5. How does a plugin manifest become prompt context?
6. How does a generated file become a preview?
7. Which controls check quality, and when do they run?
8. Where do the README and `docs/` disagree with the code or the file tree?
9. What are the important limits of the local-first design?

## 7. Deliverables

The deliverables are seven files in the exploration directory:

- `spec.md`.
- `markdown/README.md`.
- `markdown/00-overview.md`.
- `markdown/01-architecture.md`.
- `markdown/02-core-mechanism.md`.
- `markdown/03-workflow-and-quality.md`.
- `markdown/04-limits-and-comparison.md`.

Each deep dive has these properties:

- Between 3,000 and 5,000 words.
- At least two Mermaid diagrams.
- At least ten byte-exact code snippets with a citation header.
- At least one `**Aha:**` insight.

## 8. Task table

| Task | Status |
|---|---|
| Read the top-level instruction files | DONE |
| Map the top-level directories | DONE |
| Read the product documentation index | DONE |
| Trace the web and desktop entry paths | DONE |
| Trace project creation and run creation | DONE |
| Trace prompt composition | DONE |
| Trace runtime detection and spawn | DONE |
| Trace artifact creation and preview selection | DONE |
| Trace one plugin from manifest to prompt | DONE |
| Read the quality controls and test layout | DONE |
| Measure catalog counts | DONE |
| Write the six documents | DONE |
| Run the snippet checker on every markdown file | DONE |
| Count words, Mermaid fences, and Aha callouts | DONE |
| Check every relative link | DONE |
| Run the fresh-reader review | DONE |
| Record the corrections and limits | DONE |

## 9. Resume Point

A later reader can resume from these points.

- The runtime registry starts at `apps/daemon/src/runtimes/registry.ts:40`.
- The prompt fork starts at `apps/daemon/src/prompts/system.ts:942`.
- The run prompt call is at `apps/daemon/src/server.ts:11418`.
- The spawn invariant starts at `apps/daemon/src/runtimes/agent-process.ts:94`.
- The artifact manifest contract starts at `apps/daemon/src/artifacts/manifest.ts:22`.
- The preview decision starts at `apps/web/src/components/file-viewer-render-mode.ts:104`.
- The plugin parser starts at `packages/plugin-runtime/src/parsers/manifest.ts:25`.
- The plugin context resolver starts at `packages/plugin-runtime/src/resolve.ts:61`.
- The validator starts at `packages/plugin-runtime/src/validate.ts:43`.

## 10. Corrections and limits found during research

This section records every disagreement between a document and the snapshot. The snapshot is the authority.

### 10.1 Design-system package count

The README says 151 design-system packages at `README.md:238` and `README.md:404`. `design-systems/README.md:7` also says 151 packages.

The snapshot has 153 directories under `design-systems/`. One directory is `_schema`, which has no package manifest. The other 152 directories have `manifest.json`, `DESIGN.md`, and `tokens.css`. Therefore the package count is 152 under the same rule that the catalog uses for a package.

Evidence:

```text
# README.md:238
- 🧠 **Brand-grade by default.** Every render reads the active package's `DESIGN.md` as the core brand contract. 151 design-system packages ship with the repo; legacy packages may be `DESIGN.md`-only, while newer packages can add `manifest.json`, `tokens.css`, components, assets, and provenance. Drop a folder in, the picker finds it.
```

```text
# design-systems/README.md:7
The bundled catalog currently contains **151 packages**. Every bundled package
```

The count in the plugin description is also lower. `plugins/open-design/.claude-plugin/plugin.json` says 150 `DESIGN.md` systems. The snapshot has 152.

### 10.2 Official plugin and example counts

The README says 277 official plugins plus 183 examples at `README.md:437`. The category table at `README.md:446` says 183 examples.

The snapshot has 479 `open-design.json` files under `plugins/_official`. The `examples` category has 201 manifests. The non-example categories have 278 manifests.

The difference is one manifest in the official count. The example count is 18 higher than the README claim.

Evidence:

```text
# README.md:437
**277 official plugins plus 183 remixable reference examples** live in [`plugins/_official/`](plugins/_official/). Each entry is a portable plugin directory anchored by `open-design.json` plus the payload required by its type: for example `SKILL.md` for agent workflows, `template.json` for media templates, or `DESIGN.md` for design-system entries. Jump straight to a category:
```

Measured manifest counts by category:

| Category | Manifests |
|---|---|
| `atoms` | 13 |
| `design-systems` | 143 |
| `examples` | 201 |
| `image-templates` | 45 |
| `scenarios` | 14 |
| `video-templates` | 63 |

The total is 479. The non-example total is 278. The README total is 277 plus 183, which is 460. The README total is 19 lower than the snapshot total.

### 10.3 Prompt-template count

The README says 93 ready-to-replicate prompts at `README.md:208`.

The snapshot has 106 JSON files under `prompt-templates`. The count includes the catalog files, not only the prompt records. Therefore the count is not a direct contradiction of the number of prompts. It does show that the README number is not the file count.

Evidence:

```text
# README.md:208
**93 ready-to-replicate prompts** live in [`prompt-templates/`](prompt-templates/) — preview thumbnails, full prompt body, target model, aspect ratio, and source attribution. One click drops a brief into the composer.
```

Status: the prompt count is not verified. The file count is 106 JSON files.

### 10.4 Deck template count

The README says 15 deck templates and 36 themes at `README.md:189` and `README.md:393`.

The snapshot has 36 theme CSS files under `design-templates/html-ppt/assets/themes`. The theme count matches.

The snapshot has 47 directories that start with `html-ppt-`, plus the master `html-ppt` directory. The README says 15 deck templates. The relation between the 15 claim and the 47 directories is not explained in the README.

Evidence:

```text
# README.md:189
<sub><b>Swiss International-style deck</b> — grid-anchored, monochrome accents. One of <b>15 deck templates</b> and <b>36 themes</b> under <code>design-templates/html-ppt-*/</code>.</sub>
```

Status: the 15 value is not reconciled. The 36 theme value matches the snapshot.

### 10.5 Skill count in plugin descriptions

The plugin description says 139 skills at `.claude-plugin/marketplace.json:13`. The plugin manifest description says 139 skills plus 150 `DESIGN.md` systems.

The snapshot has 169 `SKILL.md` files under `skills/`. The daemon registry can exclude some entries, apply workspace visibility, and deduplicate identifiers. Therefore the registered count can be lower than 169. The exact registered count was not measured because that needs a live daemon.

Status: the 139 value is not verified. The 150 design-system value is lower than the 152 package count.

### 10.6 Runtime count and distinct executables

The README says 27 runtime definitions across 26 distinct coding-agent CLI executables at `README.md:537` and `README.md:559`.

The snapshot has 27 entries in `SHIPPED_AGENT_DEFS` at `apps/daemon/src/runtimes/registry.ts:40`. The definition count is confirmed.

The distinct executable count was not confirmed. Some definitions share a primary binary name. For example, `apps/daemon/src/runtimes/defs/opencode.ts:111` uses `bin: 'opencode-cli'`. `apps/daemon/src/runtimes/defs/byok-opencode.ts` also uses `bin: 'opencode-cli'`. That pair is consistent with the README note that `byok-opencode` shares the OpenCode executable.

A full distinct-binary count needs every definition to be read and every fallback binary to be considered. That count was not completed.

Status: 27 definitions confirmed. 26 distinct executables not verified.

### 10.7 HyperFrames and Seedance counts

The README says 11 HyperFrames templates and 39 Seedance prompts at `README.md:229`.

The snapshot does not expose those two counts as a single manifest or index that was found. A filename search for `seedance` returned fewer than 39 matches. The search method can miss records whose body mentions Seedance but whose filename does not.

Status: not verified.

### 10.8 README stack claims

The README says Next.js 16 App Router and React 18 at `README.md:545`. The snapshot confirms Next.js `16.2.6` and React `18.3.1` in `apps/web/package.json`.

The daemon uses Express `5.2.1` and better-sqlite3 `12.10.0`. The snapshot confirms these values in `apps/daemon/package.json`.

Status: verified.

### 10.9 Documentation paths and code paths

`docs/architecture.md` describes the component topology and the two execution profiles. The code confirms both profiles. `packages/contracts/src/execution-profile.ts` maps the `plain` stream format to the `text_artifact` profile. Every other stream format maps to the `filesystem` profile.

`docs/prompt-composition.md` warns that two prompt implementations share no composition floor. The code confirms the early fork at `apps/daemon/src/prompts/system.ts:942`.

Status: verified. The warning is accurate.

### 10.10 Limits of this research

- File counts can include helper files that are not catalog entries.
- Directory counts can include retired or experimental entries.
- The daemon registry applies visibility and deduplication rules. A file count can differ from a runtime count.
- A published release can differ from the snapshot.
- The hosted product can use code that is not in this snapshot.
- The external agent CLIs can change their flags after the snapshot date.
- No runtime behavior was observed. All runtime claims come from source reading.
- The fresh-reader review reported places where a first-time reader can lose the thread. The writer verified every finding against the source and repaired the real gaps in all six files. Findings that the source could not settle are marked as unexplained.

## 11. Verification record

The snippet checker is `/tmp/i2t-fix.py`. It compares each fenced block with the cited source range. The command is:

```text
python3 -I /tmp/i2t-fix.py markdown/FILE.md /Users/darkvoid/GitSources/src.aidesign/open-design
```

Results are recorded after the validation pass.

| File | Snippets | Bad | Words | Mermaid | Aha |
|---|---|---|---|---|---|
| `markdown/README.md` | 2 | 0 | 1799 | 2 | 0 |
| `markdown/00-overview.md` | 12 | 0 | 3424 | 2 | 1 |
| `markdown/01-architecture.md` | 14 | 0 | 4440 | 3 | 1 |
| `markdown/02-core-mechanism.md` | 12 | 0 | 4311 | 2 | 1 |
| `markdown/03-workflow-and-quality.md` | 15 | 0 | 4323 | 2 | 1 |
| `markdown/04-limits-and-comparison.md` | 12 | 0 | 3647 | 2 | 1 |
| **Total** | **67** | **0** | **21944** | **13** | **5** |

Validator results:

```text
markdown/README.md: 2 snippets, 0 bad
markdown/00-overview.md: 12 snippets, 0 bad
markdown/01-architecture.md: 14 snippets, 0 bad
markdown/02-core-mechanism.md: 12 snippets, 0 bad
markdown/03-workflow-and-quality.md: 15 snippets, 0 bad
markdown/04-limits-and-comparison.md: 12 snippets, 0 bad
TOTAL: 67 snippets, 0 bad
```

The six Markdown files are this index and the five deep dives. They hold 67 snippets. The checker reports 0 mismatches. They contain 13 Mermaid fences and 21,944 words. The link check found 16 relative links. It found 0 broken links.

The five deep-dive word counts are between 3,000 and 5,000. Each deep dive has at least two Mermaid diagrams, at least ten snippets, and one `**Aha:**` insight. The index has two diagrams and two snippets. Every relative link resolves on disk.

The final report gives the completed table. The report also gives the validator result for each file.

The fresh-reader review is a separate check. A fresh agent read the six files with the Read tool only and no web access. The agent reported its findings. The writer checked every finding against the source and repaired the real gaps in the six files. Findings that the source could not settle are marked as unexplained.
