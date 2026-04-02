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
8. First fix my markdown numbering in the tasks list and ensure to mark what is done (ignore the template section)

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

## Completed Explorations

### llamacpp/MillionCo

1. [x] **ami-releases** - Complete (4 docs: exploration, zero-to, rust-revision, production-grade)
2. [x] **Sink** - Core exploration complete (4 docs), missing: rust-revision, production-grade
3. [x] **cal.com** - Complete (8 docs: exploration, zero-to, 4 deep-dives, rust-revision, production-grade)
4. [x] **expect** - Core exploration complete (7 docs), missing: production-grade
5. [x] **companion** - Core exploration complete (6 docs), missing: rust-revision, production-grade

### llamacpp/AIResearch

6. [x] **AIResearch** - Complete (8 docs: exploration, zero-to, 3 deep-dives, valtron-integration, rust-revision, production-grade)

### Database & Storage

7. [x] **turso** - Complete (14 docs: libsql exploration, client explorations for rs/ts/go/c, embedded-replicas, cli, pg_turso, rusqlite, multi-cloud-sync, agentfs, ewe-platform, rust-revision, blog)
8. [x] **DragonflyDB** (src.db/src.dragonflydb) - Complete (8 docs: exploration, zero-to, 3 deep-dives, valtron-integration, rust-revision, production-grade)
9. [x] **spacetimedb** - Complete (14 docs: exploration, deep-dives, rust-revision, production-grade)

### Infrastructure & Networking

10. [x] **cloudflare/partykit** - Complete (9 docs: exploration, zero-to, 4 deep-dives, valtron-integration, rust-revision, production-grade)
11. [x] **process-compose** (src.process-compose) - Complete (2 docs: exploration, rust-revision)

### Application Frameworks

12. [x] **wildcard-ai** - Complete (12 docs)
13. [x] **Zero** - Complete (11 docs)

---

## In Progress (Exploration started, needs deep-dives / rust-revision / production-grade)

### Core explorations done (2 docs each, need deep-dives + rust-revision + production-grade)

1. [ ] **htmx** - `./htmx` (has: zero-to + exploration)
   - Source: `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/HTMX`
   - Missing: Deep-dives, rust-revision, production-grade

2. [ ] **extism** - `./extism` (has: zero-to + exploration)
   - Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.extism`
   - Missing: Deep-dives, rust-revision, production-grade

3. [ ] **zeromicro** - `./zeromicro` (has: zero-to + exploration)
   - Source: `/home/darkvoid/Boxxed/@formulas/src.zeromicro`
   - Missing: Deep-dives, rust-revision, production-grade

4. [ ] **p2p/hyperswarm** - `./p2p` (has: zero-to + exploration)
   - Source: `/home/darkvoid/Boxxed/@formulas/src.Peer2Peer`
   - Missing: Deep-dives, rust-revision, production-grade

5. [ ] **duckdb** - `./duckdb` (has: zero-to + storage-engine deep-dive)
   - Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.ArrowAndDBs/src.duckdb/duckdb`
   - Missing: More deep-dives, rust-revision, production-grade
   - **Focus Areas:**
     - File storage efficiency into object storage (S3, GCP)
     - Large file reading optimizations
     - Processing algorithms and approaches
     - All optimization tricks, algorithms, and approaches fully detailed

6. [ ] **smithy** - `./smithy` (has: zero-to + exploration)
   - Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.deployAnywhere/smithy`
   - Subdirs: smithy-examples, smithy-python, smithy-rs, smithy-typescript
   - Missing: Deep-dives, rust-revision, production-grade

7. [ ] **pheonixLiveView** - `./pheonixLiveView` (has: zero-to + exploration)
   - Source: `/home/darkvoid/Boxxed/@formulas/src.pheonixLiveView`
   - Missing: Deep-dives, rust-revision, production-grade

8. [ ] **rivet-dev** - `./rivet-dev` (has: zero-to + exploration)
   - Source: `/home/darkvoid/Boxxed/@formulas/src.rivet-dev`
   - Missing: Deep-dives, rust-revision, production-grade

9. [ ] **basecamp/kamal** - `./basecamp/kamal` (has: zero-to + exploration)
   - Source: `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/kamal`
   - Subdirs: kamal-proxy, kamal-site, kamal-skiff
   - Missing: Deep-dives, rust-revision, production-grade

10. [ ] **basecamp/once** - `./basecamp/once` (has: zero-to + exploration)
    - Source: `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/once`
    - Missing: Deep-dives, rust-revision, production-grade

11. [ ] **basecamp/gh-signoff** - `./basecamp/gh-signoff` (has: zero-to + exploration)
    - Source: `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/gh-signoff`
    - Missing: Deep-dives, rust-revision, production-grade

### Completed Explorations

#### AppOSS - Complete

12. [x] **AppOSS** - `./AppOSS` (COMPLETE - 10 documents)
    - Source: `/home/darkvoid/Boxxed/@formulas/src.AppOSS`
    - Documents created:
      1. `00-zero-to-apposs.md` - Beginner's guide from zero to understanding AppOSS (898 lines)
      2. `exploration.md` - Updated main exploration with references (1582 lines)
      3. `rust-revision.md` - Complete Rust translation guide (1057 lines)
      4. `production-grade.md` - Production readiness and scaling guide (1202 lines)
      5. `storage-system-guide.md` - Storage system guide for beginners (738 lines)
      6. `deep-dives/graphics-rendering-deep-dive.md` - Vector rendering, GPU pipeline (1056 lines)
      7. `deep-dives/wasm-web-rendering-deep-dive.md` - WASM architecture, CanvasKit (738 lines)
      8. `deep-dives/vector-graphics-algorithms.md` - Path tessellation, Bezier curves (932 lines)
      9. `examples/vector-graphics-examples.md` - Practical code examples (492 lines)
    - Total: ~8,700 lines of comprehensive documentation

---

### Light explorations (1 doc each, need full treatment)

13. [ ] **superfly** - `./superfly` (has: exploration.md only)
    - Source: `/home/darkvoid/Boxxed/@formulas/src.superfly`
    - Missing: Zero-to, deep-dives, rust-revision, production-grade

14. [ ] **hono** - `./hono` (has: exploration.md only)
    - Source: `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/hono`
    - Missing: Zero-to, deep-dives, rust-revision, production-grade

15. [ ] **shoelace** - `./shoelace` (has: exploration.md only)
    - Source: `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/shoelace`
    - Missing: Zero-to, deep-dives, rust-revision, production-grade

16. [ ] **11ty** - `./11ty` (has: exploration.md only)
    - Source: `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.11ty`
    - Missing: Zero-to, deep-dives, rust-revision, production-grade

17. [ ] **MendableAI** - `./MendableAI` (has: 1 doc)
    - Source: `/home/darkvoid/Boxxed/@formulas/src.MendableAI`
    - Missing: Zero-to, deep-dives, rust-revision, production-grade

18. [ ] **localfirst** - `./localfirst` (has: 1 doc)
    - Source: `/home/darkvoid/Boxxed/@formulas/src.localfirst`
    - Missing: Zero-to, deep-dives, rust-revision, production-grade

19. [ ] **trpc** - `./trpc` (has: 1 doc)
    - Source: `/home/darkvoid/Boxxed/@formulas/src.trpc`
    - Missing: Zero-to, deep-dives, rust-revision, production-grade

20. [ ] **Kobweb** - `./Kobweb` (has: 1 doc)
    - Source: `/home/darkvoid/Boxxed/@formulas/src.kobweb`
    - Missing: Zero-to, deep-dives, rust-revision, production-grade

---

## Not Started

1. [ ] **ClaudOpen** - `./llamacpp/ClaudOpen`
   - Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen`
   - Missing: Full exploration + rust-revision + production-grade
   1. It needs to be very deep and detailed
   2. It should be created in a `./llamacpp/ClaudOpen` directory and any sub-directory should exist under this directory and nowhere else
   3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
   4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how the WASM web rendering works, what graphics and SVG/vector algorithms
   5. How do we replicate the implementation for each around of rendering, animation, organisations, workflows, functionality, optimizations in the project in Rust, what we need to keep in mind
   6. How do we build a resilient storage system like this for an inexperienced software engineer

2. [x] **TabbyML** - `./llamacpp/TabbyML`
   - Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.TabbyML`
   - **Complete** (6 docs: exploration, zero-to, 3 deep-dives, rust-revision, production-grade)

3. [ ] **cloudflare (remaining subdirs)** - `./cloudflare/[subdirectory-name]`
   - Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.cloudflare`
   - Missing: agents, ai, ai-search-snippet, api-schemas, capnweb, cloudflared, containers, daemonize
   - Each needs: Full exploration + rust-revision + production-grade

4. [ ] **nordcraftengine** - `./nordcraftengine`
   - Source: `/home/darkvoid/Boxxed/@formulas/src.nordcraftengine`
   - Missing: Full exploration + rust-revision + production-grade

5. [ ] **aws/aws-lambda-web-adapter** - `./aws/aws-lambda-web-adapter`
   - Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.aws/aws-lambda-web-adapter`
   - Missing: Full exploration + rust-revision + production-grade

6. [ ] **aws/aws-lambda-rust-runtime** - `./aws/aws-lambda-rust-runtime`
   - Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.aws/aws-lambda-rust-runtime`
   - Missing: Full exploration + rust-revision + production-grade

7. [ ] **CodingIDE/rockies** - `./CodingIDE/rockies`
   - Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.CodingIDE/rockies`
   - Missing: Full exploration + rust-revision + production-grade

8. [ ] **driftingspace** - `./driftingspace`
   - Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.driftingspace`
   - Missing: Full exploration + rust-revision + production-grade

9. [ ] **turbopuffer** - `./turbopuffer`
   - Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.turbopuffer`
   - Missing: Full exploration + rust-revision + production-grade

10. [ ] **WebEditors** - `./WebEditors`
    - Source: `/home/darkvoid/Boxxed/@formulas/src.WebEditors`
    - Missing: Full exploration + rust-revision + production-grade

11. [ ] **opencontainer** - `./opencontainer`
    - Source: `/home/darkvoid/Boxxed/@formulas/src.opencontainer`
    - Missing: Full exploration + rust-revision + production-grade

12. [ ] **OpenDevin** - `./OpenDevin`
    - Source: `/home/darkvoid/Boxxed/@formulas/src.OpenDevin`
    - Missing: Full exploration + rust-revision + production-grade

13. [ ] **OpenMCP** - `./OpenMCP`
    - Source: `/home/darkvoid/Boxxed/@formulas/src.OpenMCP`
    - Missing: Full exploration + rust-revision + production-grade

14. [ ] **hyperflask** - `./hyperflask`
    - Source: `/home/darkvoid/Boxxed/@formulas/src.hyperflask`
    - Missing: Full exploration + rust-revision + production-grade

15. [ ] **joy** - `./joy`
    - Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.Joy`
    - Missing: Full exploration + rust-revision + production-grade

16. [ ] **backtrace** - `./backtrace`
    - Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.backtrace-labs`
    - Missing: Full exploration + rust-revision + production-grade

---
