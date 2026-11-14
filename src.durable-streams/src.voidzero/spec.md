---
source_location: /Users/darkvoid/GitSources/src.voidzero
repository: seven independent repositories in one directory (see section 1)
explored_at: 2026-11-14
commits: void f254d1ec477b6b2af2c02fa427bd9d196316ee07; vite-plus 794a96acd52e12216b3fe4eb02026637962cd315; vite-task 95d8da2fc1f06cfe8d176b30b608ed540efab066; setup-vp ee6805d1c428a397d5c7d3fa4111f8a7e5af2136; void-fbtee 6784d295baf4af878f33a73d76ec725f2f27e3a9; chrome-capture 87c3096cb8d0186cb6cb68d8b5ed00af87934bd4; lite-youtube-embed 674a402883c668066f940f08e757a11528299ef8
language: Rust, TypeScript, JavaScript, Shell
---

# Spec: VoidZero, the unified web toolchain and the Void framework

## 1. Source

The directory `/Users/darkvoid/GitSources/src.voidzero` is not a Git repository.
It holds seven independent Git repositories. The exploration pins each one at
its head commit.

| Directory | Remote | Commit | Date | Version | License |
|-----------|--------|--------|------|---------|---------|
| `void` | `git@github.com:voidzero-dev/void.git` | `f254d1ec477b6b2af2c02fa427bd9d196316ee07` | 2026-11-11 | `void` 1.7.0 (npm package) | MIT, copyright Cloudflare, Inc. |
| `vite-plus` | `git@github.com:voidzero-dev/vite-plus.git` | `794a96acd52e12216b3fe4eb02026637962cd315` | 2026-11-11 | `vite-plus` 1.6.0 (npm package) | MIT, copyright VoidZero Inc. |
| `vite-task` | `git@github.com:voidzero-dev/vite-task.git` | `95d8da2fc1f06cfe8d176b30b608ed540efab066` | 2026-11-10 | `@voidzero-dev/vite-task-client` 0.2.0 | MIT, copyright VoidZero Inc. |
| `setup-vp` | `git@github.com:voidzero-dev/setup-vp.git` | `ee6805d1c428a397d5c7d3fa4111f8a7e5af2136` | 2026-11-09 | `setup-vp` 1.22.0 (private action) | MIT |
| `void-fbtee` | `git@github.com:joeblew999/void-fbtee.git` | `6784d295baf4af878f33a73d76ec725f2f27e3a9` | 2026-11-12 | `void-fbtee` 0.4.0 (npm package) | MIT |
| `chrome-capture` | `git@github.com:paulirish/chrome-capture.git` | `87c3096cb8d0186cb6cb68d8b5ed00af87934bd4` | 2026-11-10 | `chrome-capture` 0.1.2 | Apache-2.0 |
| `lite-youtube-embed` | `git@github.com:paulirish/lite-youtube-embed.git` | `674a402883c668066f940f08e757a11528299ef8` | 2025-11-10 | `lite-youtube-embed` 0.3.4 | Apache-2.0 |

The source of truth is the code at the commits above. Every claim in the
exploration names a file and, where useful, a line number. A citation has the
form `path:line` or `path:line-line`. All paths are relative to
`/Users/darkvoid/GitSources/src.voidzero` unless the text names one repository
as the root. A path that starts with a repository name, such as
`vite-plus/crates/vp_command`, names a path from the snapshot root.

The snapshot has a property that the documents must state and must not hide.
Only four of the seven repositories belong to the `voidzero-dev` organization.
`void-fbtee` belongs to `joeblew999`. `chrome-capture` and `lite-youtube-embed`
belong to `paulirish`. The directory is a working collection of related tools,
not a single product monorepo.

## 2. What the Project Is

VoidZero is an organization that builds one unified toolchain for web
development, and the projects in this snapshot show the toolchain and the
framework that uses it.

- **Vite+** (`vite-plus`) is the unified toolchain. One dependency supplies the
  JavaScript runtime, package management, project creation, the development
  server, checking, testing, building, packing, and monorepo task caching. The
  command is `vp`. The command is split in two parts. A Node.js script loads a
  TypeScript entry point, which handles the global commands. Every other
  command goes to a Rust core through Native API (NAPI) bindings. A separate
  Rust global command-line binary resolves and starts a local installation. The
  Rust workspace has fifteen crates, and the TypeScript side has four packages.
- **Vite Task** (`vite-task`) is the task runner inside Vite+. It has a
  dependency-aware scheduler with a persistent and remote cache. It contains
  `fspy`, a file-system proxy that traces the files that a task reads and
  writes, so that a cache key describes the real inputs rather than a declared
  guess.
- **setup-vp** (`setup-vp`) installs `vp` in GitHub Actions, GitLab CI/CD, and
  Azure Pipelines.
- **Void** (`void`) is a full-stack framework that builds with Vite and deploys
  to Cloudflare Workers. It adds file-based server and page routing, typed
  database, storage, and AI access, migrations, resource provisioning, and a
  one-command deployment. It also carries a self-hosted team platform.
- **void-fbtee** (`void-fbtee`) is an internationalization add-on for Void. It
  uses `fbtee` for strings and the browser `Intl` Application Programming
  Interface (API) for numbers, dates, and lists.
- **chrome-capture** (`chrome-capture`) records a Chrome tab to an AV1 `.mp4`
  file with the Chrome DevTools Protocol (CDP) `Page.startScreenRecording`
  command, with no `ffmpeg` dependency.
- **lite-youtube-embed** (`lite-youtube-embed`) is a custom HyperText Markup
  Language (HTML) element that renders a YouTube player with a small facade,
  so the page loads much faster than an embedded `iframe`.

## 3. Documentation Goal

A reader must be able to:

1. State what VoidZero builds, and how the seven repositories relate.
2. Map every repository and name the package or crate that holds each concern.
3. Trace one `vp` command from the shell to the TypeScript handler or the
   Rust core.
4. Explain how Vite+ resolves a Node.js version, installs packages, and runs
   the development server.
5. Explain how Vite Task builds a task graph, selects tasks, computes a cache
   key, and restores an artifact.
6. Explain how `fspy` observes file access on Linux and on Windows.
7. Trace one Void request from the Cloudflare Worker entry point to a page, a
   server route, or a resource binding.
8. Explain how Void provisions a resource, applies a migration, and deploys.
9. State what the self-hosted Void Platform adds.
10. Explain how `setup-vp`, `void-fbtee`, `chrome-capture`, and
    `lite-youtube-embed` work.
11. Compare Vite+ with pnpm, Nx, and Turborepo, and compare Void with Next.js
    on Cloudflare Workers.
12. State the limits and the open questions that the source does not answer.

## 4. Documentation Structure

```
src.durable-streams/src.voidzero/
├── spec.md
├── markdown/
│   ├── README.md                              ← index, reading order, and thesis
│   ├── 00-overview.md                         ← what VoidZero is, the seven repositories, the thesis
│   ├── 01-repository-map.md                   ← every repository, package, crate, and pin
│   ├── 02-vite-plus-toolchain.md              ← the vp binary, command dispatch, and the crates
│   ├── 03-vite-plus-package-management.md     ← vp_pm_cli, installs, lockfile, supply chain
│   ├── 04-vite-plus-js-runtime.md             ← vp_js_runtime, Node.js versions, env, shims
│   ├── 05-vite-plus-dev-test-build.md         ← dev, check, test, build, pack, fmt, lint, config
│   ├── 06-vite-task-scheduler.md              ← vt_plan, vt_graph, vt_select, workspace, concurrency
│   ├── 07-vite-task-caching.md                ← materialized artifacts, local and remote cache, keys
│   ├── 08-fspy-file-system-proxy.md           ← why the runner traces system calls, and how
│   ├── 09-vite-task-client-and-ipc.md         ← vt_client, napi, IPC, pty, shell, tui
│   ├── 10-setup-vp-ci.md                      ← GitHub, GitLab, Azure setup and caching
│   ├── 11-void-framework-routing.md           ← file-based server and page routing, loaders, actions
│   ├── 12-void-render-adapters.md             ← React, Vue, Solid, Svelte, Markdown, SSR, SSG, islands
│   ├── 13-void-resources-and-bindings.md      ← database, KV, storage, queues, state, sockets, auth, AI
│   ├── 14-void-cli-and-deployment.md          ← the CLI, deploy-core, deploy-cloudflare, migrations
│   ├── 15-void-platform.md                    ← the self-hosted team platform and its packages
│   ├── 16-void-fbtee.md                       ← the internationalization add-on
│   ├── 17-browser-tools.md                    ← chrome-capture and lite-youtube-embed
│   ├── 18-comparisons.md                      ← pnpm, Nx, Turborepo, Bazel, Next.js on Workers
│   └── 19-glossary.md                         ← every term, spelled out
└── html/                                      ← generated
```

The twenty-one Markdown files map to the reader goals as follows.

| Document | Repository | Main sources |
|----------|------------|--------------|
| 00 | all | the seven READMEs, the package manifests, `vite-plus/docs/guide/why.md` |
| 01 | all | `pnpm-workspace.yaml`, `Cargo.toml`, `package.json`, and the directory trees |
| 02 | vite-plus | `crates/vp_command`, `crates/vp_global_cli`, `crates/vp_cli_help`, `packages/cli` |
| 03 | vite-plus | `crates/vp_pm_cli`, `crates/vp_installer`, `crates/vp_toolchain` |
| 04 | vite-plus | `crates/vp_js_runtime`, `crates/vp_setup`, `crates/vp_trampoline`, `docs/guide/env.md` |
| 05 | vite-plus | `packages/core`, `crates/vp_static_config`, the command documents |
| 06 | vite-task | `crates/vt_plan`, `crates/vt_graph`, `crates/vt_select`, `crates/vt_workspace` |
| 07 | vite-task | `crates/vt_materialized_artifact`, `crates/vt_remote_cache`, `crates/vt_graph_ser` |
| 08 | vite-task | the `fspy` crate family, including the preload, shared-memory, and seccomp crates |
| 09 | vite-task | `crates/vt_client`, `vt_client_napi`, `vt_ipc_shared`, `vt_ipc_str`, `vt_pty_terminal`, `vt_shell`, `vt_tui` |
| 10 | setup-vp | `action.yml`, `src`, `gitlab`, `azure`, `dist` |
| 11 | void | `packages/void/src/pages`, `src/inference`, `src/application-routing.ts` |
| 12 | void | `packages/{react,vue,solid,svelte,md}`, `packages/edge`, `packages/isr` |
| 13 | void | `packages/void/src/{database-providers,durable,migrations,inference}`, `docs/guide` |
| 14 | void | `packages/void/src/cli`, `packages/deploy-core`, `packages/deploy-cloudflare` |
| 15 | void | `packages/platform`, `platform/packages/*`, `platform/meta` |
| 16 | void-fbtee | `packages/void-fbtee`, `apps`, `e2e` |
| 17 | chrome-capture, lite-youtube-embed | `src`, the package manifests, the README technology notes |
| 18 | all | the documents above, plus the external tool documentation that the text names |
| 19 | all | every document above |

## 5. Tasks

| Phase | Task | Status |
|-------|------|--------|
| 1 | Read the instruction set, the directive, and the seven READMEs | DONE |
| 2 | Map every repository, workspace, package, and crate | DONE |
| 3 | Research the Vite+ binary, command dispatch, and help | DONE |
| 4 | Research Vite+ package management and the installer | DONE |
| 5 | Research the Vite+ JavaScript runtime and environment handling | DONE |
| 6 | Research the Vite+ development, check, test, and build pipeline | DONE |
| 7 | Research the Vite Task scheduler and workspace model | DONE |
| 8 | Research the Vite Task cache, artifacts, and remote cache | DONE |
| 9 | Research the `fspy` file-system proxy on each platform | DONE |
| 10 | Research the Vite Task client, IPC, pseudo-terminal, and shell | DONE |
| 11 | Research `setup-vp` for the three Continuous Integration (CI) systems | DONE |
| 12 | Research Void routing, pages, inference, and type generation | DONE |
| 13 | Research the Void render adapters, SSR, SSG, and islands | DONE |
| 14 | Research the Void resources, storage, queues, and bindings | DONE |
| 15 | Research the Void CLI, deployment, and migrations | DONE |
| 16 | Research the self-hosted Void Platform | DONE |
| 17 | Research `void-fbtee`, `chrome-capture`, and `lite-youtube-embed` | DONE |
| 18 | Write README and documents 00 to 05 | DONE |
| 19 | Write documents 06 to 11 | DONE |
| 20 | Write documents 12 to 17 | DONE |
| 21 | Write documents 18 and 19, and the parent index entry | DONE |
| 22 | Grandfather review against the pinned sources for every document | DONE |
| 23 | Fresh-reader review for every document, and fix every reported gap | DONE |
| 24 | Build the HyperText Markup Language (HTML) tree and check every link | DONE |

## 6. Build System

```bash
cd /Users/darkvoid/Dev/explorations/src.durable-streams
python3 -I build.py .
```

The parent build discovers every direct child directory that has a `markdown`
tree. It writes the matching `html` tree, copies the parent `styles.css`, and
adds the page to the parent index. No build change is needed after
`src.voidzero/markdown/` exists. The parent `index.md` gets one row and one link
for this exploration.

## 7. Quality Requirements

The ten Iron Rules from `markdown_engineering/documentation_directive.md` apply:

1. Detailed sections with code snippets from the source.
2. The thesis in the first paragraph.
3. One idea per sentence, in ASD Simplified Technical English. Spell out every
   abbreviation at first use in each file.
4. At least two Mermaid diagrams per document.
5. Tables, trees, and diagrams as visual assets.
6. Generated HTML with consistent navigation.
7. Cross-references between documents.
8. Source paths and line numbers for implementation claims.
9. At least one Aha moment per document.
10. An index and previous and next navigation on every page.

Each deep-dive document must hold at least ten code snippets. Every claim about
behaviour must cite one of the seven repositories at the commit above, or one of
their documents. A claim about an external system must name the external
document.

## 8. Expected Outcome

After reading, an engineer can explain how one `vp` binary replaces a runtime
manager, a package manager, a development server, a test runner, a bundler, and
a task runner. The engineer can state how Vite Task derives a cache key from
observed file access rather than from a declared list of inputs. The engineer
can explain how Void turns a file tree into routes, provisions the Cloudflare
resources that those routes name, and deploys the result. The engineer can also
state where the toolchain is stronger than pnpm, Nx, and Turborepo, and where it
is younger or narrower.

## 9. Resume Point

If work stops, read this file and the task table. The task table shows the last
completed phase. Continue with the first `TODO` row. Each document is complete
only after the grandfather review, the fresh-reader review, and a successful
HTML build.

## 10. Corrections and limits found during research

These facts correct a first reading of the snapshot. Every document must follow
them.

| First reading | What the source shows | Evidence |
|---------------|-----------------------|----------|
| The directory is one mono-repository | It holds seven independent Git repositories with three owners | `git remote -v` in each directory |
| Every repository belongs to VoidZero | Four belong to `voidzero-dev`, one to `joeblew999`, and two to `paulirish` | the same command |
| Vite+ is a Node.js package with a JavaScript entry point | The public `vp` command is a Node.js script that loads TypeScript code. The TypeScript code handles the global commands. Every other command goes to the Rust core through Node-API (NAPI) bindings. A separate Rust binary is the global command-line installation. | `vite-plus/packages/cli/bin/vp:1-8`, `vite-plus/packages/cli/src/bin.ts:1-9`, `vite-plus/crates/*` |
| Vite Task is a separate product | It is a repository of its own, and `vp run` uses it. The README names `vp run` as the entry point. | `vite-task/README.md:3`, `vite-plus/docs/guide/run.md` |
| Void is a VoidZero product by copyright | The license file names Cloudflare, Inc. as the copyright holder, and the remote is `voidzero-dev/void` | `void/LICENSE:1-3`, `void/README.md:1-3` |
| The cache key comes from a declared task input list | `fspy` traces the real file access of the process, including reads and writes, and the artifact is materialized from that record | `vite-task/crates/fspy`, `vite-task/docs/inputs.md` |
| Vite+ builds against the Vite Task head in this snapshot | Vite+ declares the Vite Task crates as Git dependencies at revision `cb1ea8d214b26a0f3ca2b2ef3b6bc471b18ffacb`, while the snapshot checkout is at `95d8da2fc1f06cfe8d176b30b608ed540efab066`. Documents 06 to 09 must state this difference before they describe behaviour. | `vite-plus/Cargo.toml:197`, `vite-plus/Cargo.toml:312-315` |

## 11. Verification record

The exploration is complete at the commits in section 1. This table records
the checks that ran on 2026-11-15, after the last edit.

| Check | Result |
|-------|--------|
| Markdown documents | 21: the README and documents 00 to 19, plus this spec |
| Source citations | 4,954 checked, 0 invalid |
| Citation form | Every citation uses `path:line` or `path:line-line`. No comma-joined range and no bare line number remains. |
| Quoted code blocks | 501, all byte-exact against the pinned source |
| Markdown links | 21 files in this exploration, 0 broken; 227 files in the parent tree, 0 broken |
| HyperText Markup Language (HTML) pages | 23 for this exploration (20 documents, README, spec, index), 0 broken; 253 pages in the parent tree, 0 broken |
| Grandfather review | Independent reviewers read documents 00 to 19 against the pinned source. Every finding was checked and corrected. |
| Fresh-reader review | An agent with no memory of the writing read every document. Every reported gap was fixed. |
| Build | `python3 -I build.py .` in `src.durable-streams` writes the tree with no error |
