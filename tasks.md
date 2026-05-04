# Todo - Remaining Exploration Tasks

## CRITICAL: Depth Requirement

**ALL explorations must be deep and thorough.** A 200-line markdown for a large, multi-file project is unacceptable. The purpose of exploration is to **teach** — a reader should understand the project deeply without reading source code. Read every file. Document every significant function, type, algorithm, and data structure. Include actual code snippets with file paths. Length is not a constraint — write as much as needed.

## Sequential Task List

Complete these in order. Do NOT skip grandfather review. Do NOT produce shallow summaries.

### Phase 1: src.datastar — Rewrite from scratch

- [x] Initial exploration (shallow — needs full rewrite)
- [x] **1.1:** Read every source file in `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.datastar/datastar/` — document every package, every type, every function, every algorithm
- [x] **1.2:** Rewrite `./src.datastar/datastar/` documentation — 15 documents covering engine, signals, genRx compiler, 3 plugin types, all 23 plugins, DOM morphing, SSE streaming, utilities, Rust equivalents, production patterns, web tooling
- [x] **1.3:** Grandfather review — verified names, numbers (line counts, fetch defaults, plugin counts, event names), flows, coverage
- [x] **1.4:** Applied grandfather review fixes (line counts off-by-1, missing None=0 in ReactiveFlags)
- [x] **1.5:** Rebuilt HTML with build.py, all 16 pages render
- [ ] create a index.html for all the projects in ./src.datastar so users can navigate from root to 
- [ ] all the others.
- [ ] **1.6:** Commit

### Phase 2: src.orbitinghail — Rewrite from scratch

- [x] Initial exploration (shallow — needs full rewrite)
- [ ] **2.1:** Read every source file in `/home/darkvoid/Boxxed/@formulas/src.rust/src.orbitinghail/` — document every crate, every type, every algorithm
- [ ] **2.2:** Rewrite `./src.orbitinghail/orbitinghail/` documentation — as many documents and as much content as needed. Output goes inside `./src.orbitinghail/orbitinghail/` with its own `spec.md`, `markdown/`, `html/`.
- [ ] **2.3:** Grandfather review — verify every name, number, and flow against source
- [ ] create a index.html for all the projects in ./src.datastar so users can navigate from root to 
- [ ] all the others.
- [ ] **2.4:** Apply all grandfather review fixes
- [ ] **2.5:** Rebuild HTML with build.py, verify all pages render
- [ ] **2.6:** Commit

### Phase 3: src.ui — Rewrite from scratch + cover all 11 sub-projects

- [x] Initial exploration (shallow — needs full rewrite)
- [ ] **3.1:** Read every source file in `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ui/` — all 12 sub-projects
- [ ] **3.2:** Rewrite `./src.ui/ui/` documentation — as many documents and as much content as needed. Output goes inside `./src.ui/ui/` with its own `spec.md`, `markdown/`, `html/`.
- [ ] **3.3:** Grandfather review — verify every name, number, and flow against source
- [ ] create a index.html for all the projects in ./src.datastar so users can navigate from root to 
- [ ] all the others.
- [ ] **3.4:** Apply all grandfather review fixes
- [ ] **3.5:** Rebuild HTML with build.py, verify all pages render
- [ ] **3.6:** Commit


### Phase 4: src.datstar — Rewrite other directories projects in datastar

- [ ] **1.1:** Read each project and create an exploration for it from `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.datastar` (excluding data-star and data-star-rust  - already done) — document every package, every type, every function, every algorithm
- [ ] **1.2:** Write to `./src.datastar/[projectdir_name]/` documentation — as many documents and as much content as needed to fully teach the project. Output goes inside `./src.datastar/[projectdir_name]/` with its own `spec.md`, `markdown/`, `html/`.
- [ ] create a index.html for all the projects in ./src.datastar so users can navigate from root to 
- [ ] all the others.
- [ ] **1.3:** Grandfather review — verify every name, number, and flow against source
- [ ] **1.4:** Apply all grandfather review fixes
- [ ] **1.5:** Rebuild HTML with build.py, verify all pages render
- [ ] **1.6:** Commit
