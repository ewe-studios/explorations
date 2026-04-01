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

## Priority Explorations (Next Up)

### Priority 1 - Database & Storage Systems
1. [ ] **duckdb** - `/home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.ArrowAndDBs/src.duckdb/duckdb`
   - Target: `./duckdb`
   - **Focus Areas:**
     - File storage efficiency into object storage (S3, GCP)
     - Large file reading optimizations
     - Processing algorithms and approaches
     - All optimization tricks, algorithms, and approaches fully detailed
   - Missing: Full exploration + rust-revision + production-grade

### Priority 2 - Deployment & Infrastructure
2. [ ] **cloudflare (remaining subdirs)** - `/home/darkvoid/Boxxed/@formulas/src.rust/src.cloudflare`
   - Target: `./cloudflare/[subdirectory-name]`
   - **Missing:** agents, ai, ai-search-snippet, api-schemas, capnweb, cloudflared, containers, daemonize
   - Each needs: Full exploration + rust-revision + production-grade

3. [ ] **basecamp/kamal** - `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/kamal`
   - Target: `./basecamp/kamal`
   - Subdirs: kamal-proxy, kamal-site, kamal-skiff
   - Missing: Full exploration + rust-revision + production-grade

4. [ ] **smithy** - `/home/darkvoid/Boxxed/@formulas/src.rust/src.deployAnywhere/smithy`
   - Target: `./smithy`
   - Subdirs: smithy-examples, smithy-python, smithy-rs, smithy-typescript
   - Missing: Full exploration + rust-revision + production-grade

### Priority 3 - LiveView & Real-time Systems
5. [ ] **pheonixLiveView** - `/home/darkvoid/Boxxed/@formulas/src.pheonixLiveView`
   - Target: `./pheonixLiveView`
   - Missing: Full exploration + rust-revision + production-grade

6. [ ] **HTMX** - `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/HTMX`
   - Target: `./htmlx`
   - Missing: Full exploration + rust-revision + production-grade

### Priority 4 - Platform & Runtime Systems
7. [ ] **rivet-dev** - `/home/darkvoid/Boxxed/@formulas/src.rivet-dev`
   - Target: `./rivet-dev`
   - Missing: Full exploration + rust-revision + production-grade

8. [ ] **nordcraftengine** - `/home/darkvoid/Boxxed/@formulas/src.nordcraftengine`
   - Target: `./nordcraftengine`
   - Missing: Full exploration + rust-revision + production-grade

9. [ ] **backtrace** - `/home/darkvoid/Boxxed/@formulas/src.rust/src.backtrace-labs`
   - Target: `./backtrace`
   - Missing: Full exploration + rust-revision + production-grade

10. [ ] **extism** - `/home/darkvoid/Boxxed/@formulas/src.rust/src.extism`
    - Target: `./extism`
    - Missing: Full exploration + rust-revision + production-grade

11. [ ] **zeromicro** - `/home/darkvoid/Boxxed/@formulas/src.zeromicro`
    - Target: `./zeromicro`
    - Missing: Full exploration + rust-revision + production-grade

### Priority 5 - UI & Communication Frameworks
12. [ ] **Peer2Peer** - `/home/darkvoid/Boxxed/@formulas/src.Peer2Peer`
    - Target: `./Peer2Peer`
    - Missing: Full exploration + rust-revision + production-grade

13. [ ] **basecamp/once** - `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/once`
    - Target: `./basecamp/once`
    - Missing: Full exploration + rust-revision + production-grade

14. [ ] **basecamp/gh-signoff** - `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/gh-signoff`
    - Target: `./basecamp/gh-signoff`
    - Missing: Full exploration + rust-revision + production-grade

## Remaining Explorations (Lower Priority)

1. [ ] **cloudflare/partykit** - `/home/darkvoid/Boxxed/@formulas/src.rust/src.cloudflare/partykit`
   - Target: `./cloudflare/partykit`
   - Missing: Full exploration + rust-revision + production-grade

2. [ ] **cloudflare (core subdirs)** - `/home/darkvoid/Boxxed/@formulas/src.rust/src.cloudflare`
   - Focus: agents, ai, ai-search-snippet, api-schemas, capnweb, cloudflared, containers, daemonize
   - Target: `./cloudflare/[subdirectory-name]`
   - Missing: 8 sub-directory explorations

3. [ ] **llamacpp/AIResearch** - `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AIResearch`
   - Target: `./llamacpp/AIResearch`
   - Missing: Full exploration + rust-revision + production-grade

4. [ ] **aws/aws-lambda-web-adapter** - `/home/darkvoid/Boxxed/@formulas/src.rust/src.aws/aws-lambda-web-adapter`
   - Target: `./aws/aws-lambda-web-adapter`
   - Missing: Full exploration + rust-revision + production-grade

5. [ ] **aws/aws-lambda-rust-runtime** - `/home/darkvoid/Boxxed/@formulas/src.rust/src.aws/aws-lambda-rust-runtime`
   - Target: `./aws/aws-lambda-rust-runtime`
   - Missing: Full exploration + rust-revision + production-grade

6. [ ] **CodingIDE/rockies** - `/home/darkvoid/Boxxed/@formulas/src.rust/src.CodingIDE/rockies`
   - Target: `./CodingIDE/rockies`
   - Missing: Full exploration + rust-revision + production-grade

7. [ ] **driftingspace** - `/home/darkvoid/Boxxed/@formulas/src.rust/src.driftingspace`
   - Target: `./driftingspace`
   - Missing: Full exploration + rust-revision + production-grade

8. [ ] **Zero** - `/home/darkvoid/Boxxed/@formulas/src.rust/src.Zero`
   - Target: `./Zero`
   - Missing: Full exploration + rust-revision + production-grade

9. [ ] **wildcard-ai** - `/home/darkvoid/Boxxed/@formulas/src.rust/src.wildcard-ai`
   - Target: `./wildcard-ai`
   - Missing: Full exploration + rust-revision + production-grade

10. [ ] **turbopuffer** - `/home/darkvoid/Boxxed/@formulas/src.rust/src.turbopuffer`
    - Target: `./turbopuffer`
    - Missing: Full exploration + rust-revision + production-grade

11. [ ] **WebEditors** - `/home/darkvoid/Boxxed/@formulas/src.WebEditors`
    - Target: `./WebEditors`
    - Missing: Full exploration + rust-revision + production-grade

12. [ ] **superfly** - `/home/darkvoid/Boxxed/@formulas/src.superfly`
    - Target: `./superfly`
    - Missing: Full exploration + rust-revision + production-grade

13. [ ] **extism** - `/home/darkvoid/Boxxed/@formulas/src.rust/src.extism`
    - Target: `./extism`
    - Missing: Full exploration + rust-revision + production-grade

14. [ ] **zeromicro** - `/home/darkvoid/Boxxed/@formulas/src.zeromicro`
    - Target: `./zeromicro`
    - Missing: Full exploration + rust-revision + production-grade

15. [ ] **hyperflask** - `/home/darkvoid/Boxxed/@formulas/src.hyperflask`
    - Target: `./hyperflask`
    - Missing: Full exploration + rust-revision + production-grade

16. [ ] **kobweb** - `/home/darkvoid/Boxxed/@formulas/src.kobweb`
    - Target: `./kobweb`
    - Missing: Full exploration + rust-revision + production-grade

17. [ ] **localfirst** - `/home/darkvoid/Boxxed/@formulas/src.localfirst`
    - Target: `./localfirst`
    - Missing: Full exploration + rust-revision + production-grade

18. [ ] **MendableAI** - `/home/darkvoid/Boxxed/@formulas/src.MendableAI`
    - Target: `./MendableAI`
    - Missing: Full exploration + rust-revision + production-grade

19. [ ] **nordcraftengine** - `/home/darkvoid/Boxxed/@formulas/src.nordcraftengine`
    - Target: `./nordcraftengine`
    - Missing: Full exploration + rust-revision + production-grade

20. [ ] **opencontainer** - `/home/darkvoid/Boxxed/@formulas/src.opencontainer`
    - Target: `./opencontainer`
    - Missing: Full exploration + rust-revision + production-grade

21. [ ] **OpenDevin** - `/home/darkvoid/Boxxed/@formulas/src.OpenDevin`
    - Target: `./OpenDevin`
    - Missing: Full exploration + rust-revision + production-grade

22. [ ] **OpenMCP** - `/home/darkvoid/Boxxed/@formulas/src.OpenMCP`
    - Target: `./OpenMCP`
    - Missing: Full exploration + rust-revision + production-grade

23. [ ] **rivet-dev** - `/home/darkvoid/Boxxed/@formulas/src.rivet-dev`
    - Target: `./rivet-dev`
    - Missing: Full exploration + rust-revision + production-grade

24. [ ] **Peer2Peer** - `/home/darkvoid/Boxxed/@formulas/src.Peer2Peer`
    - Target: `./Peer2Peer`
    - Missing: Full exploration + rust-revision + production-grade

25. [ ] **pheonixLiveView** - `/home/darkvoid/Boxxed/@formulas/src.pheonixLiveView`
    - Target: `./pheonixLiveView`
    - Missing: Full exploration + rust-revision + production-grade

26. [ ] **trpc** - `/home/darkvoid/Boxxed/@formulas/src.trpc`
    - Target: `./trpc`
    - Missing: Full exploration + rust-revision + production-grade

27. [ ] **smithy** - `/home/darkvoid/Boxxed/@formulas/src.rust/src.deployAnywhere/smithy`
    - Target: `./smithy`
    - Subdirs: smithy-examples, smithy-python, smithy-rs, smithy-typescript
    - Missing: Full exploration + rust-revision + production-grade

28. [ ] **hono** - `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/hono`
    - Target: `./hono`
    - Missing: Full exploration + rust-revision + production-grade

29. [ ] **shoelace** - `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/shoelace`
    - Target: `./shoelace`
    - Missing: Full exploration + rust-revision + production-grade

30. [ ] **HTMX** - `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/HTMX`
    - Target: `./htmlx`
    - Missing: Full exploration + rust-revision + production-grade

31. [ ] **11ty** - `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.11ty`
    - Target: `./11ty`
    - Missing: Full exploration + rust-revision + production-grade

32. [ ] **basecamp/once** - `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/once`
    - Target: `./basecamp/once`
    - Missing: Full exploration + rust-revision + production-grade

33. [ ] **basecamp/kamal** - `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/kamal`
    - Target: `./basecamp/kamal`
    - Subdirs: kamal-proxy, kamal-site, kamal-skiff
    - Missing: Full exploration + rust-revision + production-grade

34. [ ] **basecamp/gh-signoff** - `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/gh-signoff`
    - Target: `./basecamp/gh-signoff`
    - Missing: Full exploration + rust-revision + production-grade

35. [ ] **joy** - `/home/darkvoid/Boxxed/@formulas/src.rust/src.Joy`
    - Target: `./joy`
    - Missing: Full exploration + rust-revision + production-grade

36. [ ] **duckdb** - `/home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.ArrowAndDBs/src.duckdb/duckdb`
    - Target: `./duckdb`
    - Missing: Full exploration + rust-revision + production-grade

37. [ ] **backtrace** - `/home/darkvoid/Boxxed/@formulas/src.rust/src.backtrace-labs`
    - Target: `./backtrace`
    - Missing: Full exploration + rust-revision + production-grade

38. [ ] **spacetimedb** - `/home/darkvoid/Boxxed/@formulas/src.rust/src.SpacetimeDB/SpacetimeDB`
    - Target: `./spacetimedb`
    - Missing: Full exploration + rust-revision + production-grade

---
