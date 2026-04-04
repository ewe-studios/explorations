# Todo

You are to spawn exploration agents where each must singularly generate deep and detailed explorations of each of the project, create a directory. Once the exploration is done, spawn the rust agent to create the version of that type of project in rust.

**Important:**

1. Some explorations have already started before, were paused, review whats there, update what's necessary, continue where you left off. Remember do it one by one.
2. Do not duplicate code and copy files over, dont be stupid.
3. Run the agents on the list one by one, only multi-task within each item.
4. Nothing is completed, till you do a systematic review of each part of the directory or project to confirm completeness.
5. We are to do detailed exploration, not just one single exploration.md file, do deep dives into each sub project, sub-module, make it super detailed
6. Trigger up to 3 agents to parallelize the work on the items.
7. When user says `./[provided-parent-directory-name-above]` they mean the directory (named after the base directory of the exploration e.g /alex/alex.workers, then the directory is alex.workers), dont mess that up, and the directory is supposed to be in this repo.

---

### Template Section

1. [ ] The [exploration-name] exploration is too light, we need to make it more detailed, going into:
  1. How each part of the project works
  2. How it accesses the WebGPU layer
  3. How it builds that into TypeScript types and implementation
  4. What it will take to replicate similar in Rust.

2. [ ] [project directory]
  1. It needs to be very deep and detailed
  2. It should be created in a `./[provided-parent-directory-name-above]` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how the WASM web rendering works, what graphics and SVG/vector algorithms
  5. How do we replicate the C++ implementation of rendering, animation, vector graphics, optimizations in the project in Rust, what we need to keep in mind
  6. How do we build a resilient storage system like this for an inexperienced software engineer

3. [ ] [project directory]
  1. It needs to be very deep and detailed
  2. It should be created in a `./[provided-parent-directory-name-above]` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we build a resilient system like this for an inexperienced software engineer from first principles to expert level covering all topics related to this project.

---

## In Progress (Exploration started, needs deep-dives / rust-revision / production-grade)

### Partially Complete (zero-to + exploration done, need deep-dives + rust-revision + production-grade)

1. [x] **htmx** - `./htmx` (has: 00-zero-to + 01-exploration + 5 deep-dives + rust-revision + production-grade) - **COMPLETE**
   - Source: `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/HTMX`
   - Documents: 9 total (zero-to, exploration, 5 deep-dives, rust-revision, production-grade)

2. [x] **extism** - `./extism` (has: 00-zero-to + 01-exploration + 4 deep-dives + rust-revision + production-grade) - **COMPLETE**
   - Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.extism`
   - Documents: 8 total (zero-to, exploration, 4 deep-dives, rust-revision, production-grade)

3. [x] **zeromicro** - `./zeromicro` (has: 00-zero-to + 01-exploration + 4 deep-dives + rust-revision + production-grade) - **COMPLETE**
   - Source: `/home/darkvoid/Boxxed/@formulas/src.zeromicro`
   - Documents: 8 total (zero-to, exploration, 4 deep-dives, rust-revision, production-grade)

4. [x] **p2p/hyperswarm** - `./p2p` (has: 00-zero-to + 01-exploration + 2 deep-dives + rust-revision + production-grade) - **COMPLETE**
   - Source: `/home/darkvoid/Boxxed/@formulas/src.Peer2Peer`
   - Documents: 6 total (zero-to, exploration, 2 deep-dives, rust-revision, production-grade)

5. [x] **duckdb** - `./duckdb` (has: 00-zero-to + 01-storage-engine + 02-object-storage + 03-query-execution + 04-compression + rust-revision + production-grade) - **COMPLETE**
   - Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.ArrowAndDBs/src.duckdb/duckdb`
   - Documents: 7 total (zero-to, 4 deep-dives, rust-revision, production-grade)

6. [x] **smithy** - `./smithy` (has: 00-zero-to + 01-exploration + 02-model-system + 03-code-generation + 04-protocol-generation + 05-aws-sdk + rust-revision + production-grade) - **COMPLETE**
   - Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.deployAnywhere/smithy`
   - Subdirs: smithy-examples, smithy-python, smithy-rs, smithy-typescript
   - Documents: 8 total (zero-to, exploration, 5 deep-dives, rust-revision, production-grade)

7. [x] **pheonixLiveView** - `./pheonixLiveView` (has: 00-zero-to + 01-exploration + 02-liveview-lifecycle + 03-heex-templating + 04-websocket-protocol + 05-pubsub-broadcast + 06-forms-validation + 07-live-components + 08-javascript-hooks + rust-revision + production-grade) - **COMPLETE**
   - Source: `/home/darkvoid/Boxxed/@formulas/src.pheonixLiveView/phoenix`
   - Documents: 10 total (zero-to, exploration, 7 deep-dives, rust-revision, production-grade)

8. [x] **rivet-dev** - `./rivet-dev` (has: 00-zero-to + 01-exploration + 02-actor-lifecycle + 03-storage-drivers + 04-realtime-patterns + 05-distribution-scaling + rust-revision + production-grade) - **COMPLETE**
   - Source: `/home/darkvoid/Boxxed/@formulas/src.rivet-dev/rivetkit`
   - Documents: 8 total (zero-to, exploration, 4 deep-dives, rust-revision, production-grade)

9. [x] **basecamp/kamal** - `./basecamp/kamal` (has: 00-zero-to + 01-exploration + 02-deployment-workflow + 03-proxy-internals + 04-secrets-management + 05-asset-handling + rust-revision + production-grade) - **COMPLETE**
   - Source: `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/kamal`
   - Subdirs: kamal-proxy, kamal-site, kamal-skiff
   - Documents: 8 total (zero-to, exploration, 4 deep-dives, rust-revision, production-grade)

10. [x] **basecamp/once** - `./basecamp/once` (has: 00-zero-to + 01-exploration + 02-tui-dashboard + 03-docker-orchestration + 04-backup-restore + rust-revision + production-grade) - **COMPLETE**
    - Source: `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/once`
    - Documents: 7 total (zero-to, exploration, 3 deep-dives, rust-revision, production-grade)

11. [x] **basecamp/gh-signoff** - `./basecamp/gh-signoff` (has: 00-zero-to + 01-exploration + 02-github-api + 03-branch-protection + 04-team-workflows + rust-revision + production-grade) - **COMPLETE**
    - Source: `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/gh-signoff`
    - Documents: 7 total (zero-to, exploration, 3 deep-dives, rust-revision, production-grade)

### Light explorations (exploration.md only, need zero-to + deep-dives + rust-revision + production-grade)

12. [x] **superfly** - `./superfly` (has: 00-zero-to + 5 deep-dives + rust-revision + production-grade) - **COMPLETE**
    - Source: `/home/darkvoid/Boxxed/@formulas/src.superfly`
    - Documents: 8 total (zero-to, 5 deep-dives, rust-revision, production-grade)

13. [x] **hono** - `./hono` (has: 00-zero-to + 5 deep-dives + rust-revision + production-grade) - **COMPLETE**
    - Source: `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/hono`
    - Documents: 8 total (zero-to, 5 deep-dives, rust-revision, production-grade)

14. [x] **trpc** - `./trpc` (has: 00-zero-to + 5 deep-dives + rust-revision + production-grade) - **COMPLETE**
    - Source: `/home/darkvoid/Boxxed/@formulas/src.trpc`
    - Documents: 8 total (zero-to, 5 deep-dives, rust-revision, production-grade)

15. [x] **Kobweb** - `./Kobweb` (has: 00-zero-to + 5 deep-dives + rust-revision + production-grade) - **COMPLETE**
    - Source: `/home/darkvoid/Boxxed/@formulas/src.kobweb`
    - Documents: 8 total (zero-to, 5 deep-dives, rust-revision, production-grade)

---

## Not Started (Empty directories or no exploration yet)

1. [x] **nordcraftengine** - `./nordcraftengine` (has: 00-zero-to + 5 deep-dives + rust-revision + production-grade) - **COMPLETE**
   - Source: `/home/darkvoid/Boxxed/@formulas/src.nordcraftengine`
   - Documents: 8 total (zero-to, 5 deep-dives, rust-revision, production-grade)

2. [x] **joy** - `./joy` (has: 00-zero-to + 5 deep-dives + rust-revision + production-grade) - **COMPLETE**
   - Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.Joy`
   - Documents: 8 total (zero-to, 5 deep-dives, rust-revision, production-grade)

3. [x] **backtrace** - `./backtrace` (has: 00-zero-to + 5 deep-dives + rust-revision + production-grade) - **COMPLETE**
   - Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.backtrace-labs`
   - Documents: 8 total (zero-to, 5 deep-dives, rust-revision, production-grade)

4. [x] **opencontainer** - `./opencontainer` (has: 00-zero-to + 5 deep-dives + rust-revision + production-grade) - **COMPLETE**
   - Source: `/home/darkvoid/Boxxed/@formulas/src.opencontainer`
   - Documents: 8 total (zero-to, 5 deep-dives, rust-revision, production-grade)

5. [ ] **OpenDevin** - `./OpenDevin` - **SKIPPED: Source directory does not exist**
   - Source: `/home/darkvoid/Boxxed/@formulas/src.OpenDevin` - DOES NOT EXIST

6. [ ] **OpenMCP** - `./OpenMCP` - **SKIPPED: Source directory does not exist**
   - Source: `/home/darkvoid/Boxxed/@formulas/src.OpenMCP` - DOES NOT EXIST

7. [ ] **hyperflask** - `./hyperflask`
   - Source: `/home/darkvoid/Boxxed/@formulas/src.hyperflask`
   - Missing: Full exploration + rust-revision + production-grade

8. [ ] **WebEditors** - `./WebEditors`
   - Source: `/home/darkvoid/Boxxed/@formulas/src.WebEditors`
   - Missing: Full exploration + rust-revision + production-grade

---

## Already Started - Specific Projects (Need Full Treatment)

1. [ ] **HKUSD/OpenSpace** - `./HKUSD/OpenSpace`
   - Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.HKUSD/OpenSpace`
   - Missing: Full exploration (zero-to, exploration, deep-dives, rust-revision, production-grade)

2. [ ] **AIResearch/pretext** - `./AIResearch/pretext`
   - Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AIResearch/pretext`
   - Missing: Full exploration with ewe_platform foundation_wasm crate integration details
   - Special focus: Text rendering, measuring across browsers/OS, internationalization

---

## Completed Explorations

### llamacpp

- [x] **ClaudOpen** - `./llamacpp/ClaudOpen` (COMPLETE - 47 documents across 5 subdirectories)
  - claw-code, fff.nvim, free-code, openclaude, claude-code-system-prompts
- [x] **TabbyML** - `./llamacpp/TabbyML` (7 docs: exploration, zero-to, 3 deep-dives, rust-revision, production-grade)
- [x] **MillionCo/ami-releases** - Complete (4 docs)
- [x] **MillionCo/Sink** - Core exploration complete (4 docs)
- [x] **MillionCo/cal.com** - Complete (8 docs)
- [x] **MillionCo/expect** - Core exploration complete (7 docs)
- [x] **MillionCo/companion** - Core exploration complete (6 docs)
- [x] **AIResearch** - Complete (8 docs)

### Database & Storage

- [x] **turso** - Complete (14 docs)
- [x] **DragonflyDB** - Complete (8 docs)
- [x] **spacetimedb** - Complete (14 docs)

### Infrastructure & Networking

- [x] **cloudflare/partykit** - Complete (9 docs)
- [x] **process-compose** - Complete (2 docs)
- [x] **cloudflare** (main) - Complete (workerd, trie-hard, quiche, boringtun, lol-html, workers-rs deep-dives)

### Application Frameworks

- [x] **wildcard-ai** - Complete (12 docs)
- [x] **Zero** - Complete (11 docs)
- [x] **driftingspace** - Complete (10 docs: exploration, zero-to, 5 deep-dives, valtron-integration, rust-revision, production-grade)

### Other Complete

- [x] **AppOSS** - Complete (10 documents)
- [x] **fframes** - Complete (13 documents)
- [x] **basecamp** (main exploration) - Complete
- [x] **aws/aws-lambda-rust-runtime** - Complete (7 docs: exploration, zero-to, 3 deep-dives, valtron-integration, rust-revision, production-grade)
- [x] **aws/aws-lambda-web-adapter** - Complete (7 docs: exploration, 4 deep-dives, valtron-integration, rust-revision, production-grade)
- [x] **CodingIDE/rockies** - Complete (8 docs: exploration, zero-to, 5 deep-dives, valtron-integration, rust-revision, production-grade)
- [x] **turbopuffer** - Complete (9 docs: exploration, 3 deep-dives, sdk-comparison, performance-optimizations, storage-system-guide, rust-revision, production-grade, blog posts)

---

## Cloudflare Remaining Subdirs (Not Started)

These subdirectories under `/home/darkvoid/Boxxed/@formulas/src.rust/src.cloudflare` need exploration:

1. [ ] **agents**
2. [ ] **ai**
3. [ ] **ai-search-snippet**
4. [ ] **api-schemas**
5. [ ] **capnweb**
6. [ ] **cloudflared**
7. [ ] **containers**
8. [ ] **daemonize**

---
