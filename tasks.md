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

1. [x] /home/darkvoid/Boxxed/@dev/repo-expolorations/content-addressed-data
  - **Status:** COMPLETE - Has exploration.md, rust-revision.md, production-grade.md, 09-valtron-integration.md, and 15+ deep dive documents
  - **Location:** `./content-addressed-data/`
  - **Note:** Valtron integration for pinning service Lambda deployment COMPLETE

2. [x] /home/darkvoid/Boxxed/@dev/repo-expolorations/taubyte
  - **Status:** COMPLETE - Has exploration.md, rust-revision.md, production-grade.md, and 20+ deep dive documents
  - **Location:** `./taubyte/`
  - **Note:** WASM runtime fully covered (wazero)

3. [x] /home/darkvoid/Boxxed/@dev/repo-expolorations/dolthub
  - **Status:** COMPLETE - Has exploration.md, rust-revision.md, production-grade.md, 09-valtron-integration.md, and 6+ deep dive documents
  - **Location:** `./dolthub/`
  - **Note:** Valtron integration for serverless Dolt deployment COMPLETE

4. [x] /home/darkvoid/Boxxed/@dev/repo-expolorations/microgpt
  - **Status:** COMPLETE - Textbook-level deep dives: 00-zero-to-ml-engineer.md, 01-autograd-backpropagation-deep-dive.md, 02-transformer-architecture-deep-dive.md, 03-training-loop-adam-deep-dive.md, 04-inference-sampling-deep-dive.md, production-grade.md, rust-revision.md, 09-valtron-integration.md
  - **Location:** `./microgpt/`
  - **Note:** Valtron integration for model inference on Lambda COMPLETE

5. [x] /home/darkvoid/Boxxed/@dev/repo-expolorations/alchemy/fragment
  - **Status:** COMPLETE - exploration.md, 8 deep dives (00-08), rust-revision.md, production-grade.md, 08-valtron-integration.md
  - **Location:** `./alchemy/fragment/`
  - **Note:** ALL template requirements met including valtron Lambda integration

6. [x] /home/darkvoid/Boxxed/@formulas/src.rust/src.deployAnywhere (alchemy)
  - **Status:** COMPLETE - Comprehensive deep dives covering all core architecture, providers, state management, and Valtron integration
  - **Location:** `./alchemy/`
  - **Documents:**
    - `exploration.md` - Overview of Alchemy IaC framework
    - `00-zero-to-deploy-engineer.md` - First principles textbook
    - `01-distilled-api-specs-deep-dive.md` - Git submodule spec cloning
    - `02-provider-integration-deep-dive.md` - Cloudflare/AWS/GCP provider patterns
    - `03-core-architecture-deep-dive.md` - Resource system, Scope, Apply engine
    - `03-resource-lifecycle-deep-dive.md` - Create/update/delete lifecycle
    - `04-state-management-deep-dive.md` - StateStore implementations, serde
    - `05-valtron-integration.md` - Valtron replication patterns
    - `rust-revision.md` - Rust translation guide
    - `production-grade.md` - Deployment, scaling, monitoring
  - **Note:** All providers (Cloudflare Worker/D1/KV/R2, AWS Lambda/S3/DynamoDB, GCP) covered with Valtron replication patterns

7. [x] /home/darkvoid/Boxxed/@formulas/src.rust/src.cloudflare/workerd
  - **Status:** COMPLETE - Comprehensive deep dives with 7 documents
  - **Location:** `./cloudflare/workerd/`
  - **Documents:**
    - `exploration.md` - High-level overview of workerd runtime
    - `00-zero-to-runtime-engineer.md` - First principles textbook
    - `01-isolate-architecture-deep-dive.md` - V8 isolate multi-tenancy
    - `02-actor-model-deep-dive.md` - Durable Objects actor implementation
    - `03-wasm-runtime-deep-dive.md` - WebAssembly module support
    - `04-web-api-compatibility-deep-dive.md` - Fetch, Streams, Service Worker APIs
    - `05-capnp-rpc-deep-dive.md` - Capability-based RPC system
    - `06-event-loop-async-deep-dive.md` - Async event loop architecture
    - `07-valtron-integration.md` - Valtron replication patterns
    - `rust-revision.md` - Rust translation guide
    - `production-grade.md` - Production deployment guide
  - **Note:** Full C++ runtime architecture covered with Valtron integration for Lambda deployment

8. [x] /home/darkvoid/Boxxed/@formulas/src.rust/src.cloudflare/trie-hard
  - **Status:** COMPLETE - Textbook-level deep dives with 6 documents
  - **Location:** `./cloudflare/trie-hard/`
  - **Documents:**
    - `exploration.md` - Overview of trie data structure and trie-hard
    - `00-zero-to-trie-engineer.md` - First principles trie textbook
    - `01-trie-structure-deep-dive.md` - Trie data structure details
    - `02-wasm-integration-deep-dive.md` - WASM integration patterns
    - `03-performance-optimization-deep-dive.md` - Bitmask indexing optimization
    - `04-concurrency-patterns-deep-dive.md` - Concurrent access patterns
    - `05-valtron-integration.md` - Valtron replication patterns
    - `rust-revision.md` - Rust translation (already Rust)
    - `production-grade.md` - Production deployment at 30M req/s
  - **Note:** Already Rust implementation - Valtron integration for Lambda deployment complete

9. [x] /home/darkvoid/Boxxed/@formulas/src.rust/src.boxer
  - **Status:** COMPLETE - Comprehensive deep dives with 6 documents
  - **Location:** `./boxer/`
  - **Documents:**
    - `exploration.md` - Overview of boxer macro system
    - `00-zero-to-boxer-engineer.md` - First principles textbook
    - `01-boxing-patterns-deep-dive.md` - Boxing pattern mechanics
    - `02-wasm-compatibility-deep-dive.md` - WASM integration
    - `03-performance-deep-dive.md` - Performance optimization
    - `04-macro-system-deep-dive.md` - Rust macro internals
    - `05-valtron-integration.md` - Valtron replication patterns
    - `rust-revision.md` - Rust translation guide
    - `production-grade.md` - Production deployment guide
  - **Note:** Macro system fully covered with Valtron integration

10. [x] /home/darkvoid/Boxxed/@formulas/src.rust/src.BuildTooling (pavex)
  - **Status:** COMPLETE - Comprehensive deep dives with 6 documents
  - **Location:** `./pavex/`
  - **Documents:**
    - `exploration.md` - Overview of pavex build system
    - `00-zero-to-build-engineer.md` - First principles textbook
    - `01-macro-codegen-deep-dive.md` - Macro code generation
    - `02-dependency-resolution-deep-dive.md` - Dependency graph resolution
    - `03-incremental-builds-deep-dive.md` - Incremental compilation
    - `04-framework-integration-deep-dive.md` - Framework integration
    - `05-valtron-integration.md` - Valtron replication patterns
    - `rust-revision.md` - Rust translation guide
    - `production-grade.md` - Production deployment guide
  - **Note:** Build system architecture covered with Valtron integration

11. [x] /home/darkvoid/Boxxed/@formulas/src.rust/src.Concurrency
  - **Status:** COMPLETE - Comprehensive deep dives with 6 documents
  - **Location:** `./concurrency/`
  - **Documents:**
    - `exploration.md` - Overview of concurrency patterns
    - `00-zero-to-concurrency-engineer.md` - First principles textbook
    - `01-thread-model-deep-dive.md` - Threading models
    - `02-async-model-deep-dive.md` - Async/await internals
    - `03-actor-model-deep-dive.md` - Actor model implementation
    - `04-sync-primitives-deep-dive.md` - Synchronization primitives
    - `05-valtron-integration.md` - Valtron replication patterns
    - `rust-revision.md` - Rust translation guide
    - `production-grade.md` - Production deployment guide
  - **Note:** Concurrency patterns fully covered with Valtron integration

12. [x] /home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.dragonflydb
  - **Status:** COMPLETE - Comprehensive deep dives with 8 documents
  - **Location:** `./src.db/src.dragonflydb/`
  - **Documents:**
    - `exploration.md` - Overview of DragonflyDB architecture
    - `00-zero-to-db-engineer.md` - In-memory fundamentals, shared-nothing architecture
    - `01-storage-engine-deep-dive.md` - Dashtable, DenseSet, memory efficiency
    - `02-query-execution-deep-dive.md` - VLL transaction framework, command processing
    - `03-consensus-replication-deep-dive.md` - Replication protocol, consistency models
    - `rust-revision.md` - Valtron-based Rust translation
    - `production-grade.md` - Kubernetes, Terraform, monitoring
    - `04-valtron-integration.md` - Edge cache patterns for Lambda
  - **Note:** 25X throughput vs Redis (3.8M QPS), 30% more memory efficient, shared-nothing architecture with VLL for strict serializability

---

## Pending Explorations

### src.db Remaining Databases

1. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.deltalake
  - **Status:** Pending - Only exploration.md exists
  - **Location:** `./src.db/src.deltalake/`
  - **Needed:** Deep dives into Delta Lake protocol, ACID transactions, time travel

2. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.ArrowAndDBs
  - **Status:** Pending - Only exploration.md exists
  - **Location:** `./src.db/src.ArrowAndDBs/`
  - **Needed:** Columnar storage, Apache Arrow format, query optimization

3. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.gimli-rs
  - **Status:** Pending - Only exploration.md exists
  - **Location:** `./src.db/src.gimli-rs/`
  - **Needed:** DWARF debugging format, binary analysis

4. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.GoatPlatform
  - **Status:** Pending - Only exploration.md exists
  - **Location:** `./src.db/src.GoatPlatform/`
  - **Needed:** Platform architecture, plugin system

5. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.OrbitingHail
  - **Status:** Pending - Only exploration.md exists
  - **Location:** `./src.db/src.OrbitingHail/`
  - **Needed:** Distributed systems, consensus protocols

6. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.TigerBeetle
  - **Status:** Pending - Only exploration.md exists
  - **Location:** `./src.db/src.TigerBeetle/`
  - **Needed:** Financial ledger, ACID guarantees, two-phase commit

7. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.Neodatabase
  - **Status:** Pending - Only exploration.md exists
  - **Location:** `./src.db/src.Neodatabase/`
  - **Needed:** Graph database, Cypher query language

8. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.libsql (Turso)
  - **Status:** COMPLETE - See Completed Explorations section

### Other Pending

9. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.bubbletea
  1. It needs to be very deep and detailed
  2. It should be created in a `./bubbletea` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we build a resilient system like this for an inexperienced software engineer from first principles to expert level covering all topics related to this project.
  7. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)

10. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.Containers/colima
  1. It needs to be very deep and detailed
  2. It should be created in a `./colima` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we build a resilient system like this for an inexperienced software engineer from first principles to expert level covering all topics related to this project.
  7. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)

11. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.cloudflare/telescope
  1. It needs to be very deep and detailed
  2. Create this under cloudflare (in this repo) as `./cloudflare/telescope`
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we build a resilient system like this for an inexperienced software engineer from first principles to expert level covering all topics related to this project.
  7. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)

12. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.cloudflare/quiche
  1. It needs to be very deep and detailed
  2. Create this under cloudflare (in this repo) as `./cloudflare/quiche`
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we build a resilient system like this for an inexperienced software engineer from first principles to expert level covering all topics related to this project.
  7. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)

13. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.cloudflare/partykit
  1. It needs to be very deep and detailed
  2. Create this under cloudflare (in this repo) as `./cloudflare/partykit`
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we build a resilient system like this for an inexperienced software engineer from first principles to expert level covering all topics related to this project.
  7. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)

14. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.cloudflare (core subdirectories)
  1. It needs to be very deep and detailed
  2. Focus on these directories:
    a. agents
    b. ai
    c. ai-search-snippet
    d. api-schemas
    e. capnweb
    f. cloudflared
    g. containers
    h. daemonize
  3. Create this under cloudflare (in this repo) as `./cloudflare/[subdirectory-name]`
  4. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  5. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  6. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  7. How do we build a resilient system like this for an inexperienced software engineer from first principles to expert level covering all topics related to this project.
  8. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  9. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

15. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AIResearch
  1. It needs to be very deep and detailed
  2. It should be created in a `./llamacpp/AIResearch` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we build a resilient system like this for an inexperienced software engineer from first principles to expert level covering all topics related to this project.

17. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.aws/aws-lambda-web-adapter
  1. It needs to be very deep and detailed
  2. It should be created in a `./aws/aws-lambda-web-adapter` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind

18. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.aws/aws-lambda-rust-runtime
  1. It needs to be very deep and detailed
  2. It should be created in a `./aws/aws-lambda-rust-runtime` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. A detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

19. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.caddy/caddy
  1. It needs to be very deep and detailed
  2. It should be created in a `./caddy/caddy` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. A detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

20. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.CodingIDE/fresh
  1. It needs to be very deep and detailed
  2. Include:
    a. /home/darkvoid/Boxxed/@formulas/src.rust/src.CodingIDE/fresh-plugins
    b. /home/darkvoid/Boxxed/@formulas/src.rust/src.CodingIDE/fresh-plugins-registry
  3. It should be created in a `./CodingIDE/fresh` directory and any sub-directory should exist under this directory and nowhere else
  4. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  5. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  6. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  7. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  8. A detailed first principle explanation for someone who has not done this before on related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to text editor engineer by the time i am done, skip nothing, cover everything.

21. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.cbor
  1. It needs to be very deep and detailed
  2. It should be created in a `./cbor` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. A detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

22. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.CodingIDE/rockies
  1. It needs to be very deep and detailed
  2. It should be created in a `./CodingIDE/rockies` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. A detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

23. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.driftingspace
  1. It needs to be very deep and detailed
  2. It should be created in a `./driftingspace` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. A detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

24. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.Zero
  1. It needs to be very deep and detailed
  2. It should be created in a `./Zero` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. A detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

25. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.wildcard-ai
  1. It needs to be very deep and detailed
  2. It should be created in a `./wildcard-ai` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. A detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

26. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.turbopuffer
  1. It needs to be very deep and detailed
  2. It should be created in a `./turbopuffer` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

27. [ ] /home/darkvoid/Boxxed/@formulas/src.WebEditors
  1. It needs to be very deep and detailed
  2. It should be created in a `./WebEditors` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

28. [ ] /home/darkvoid/Boxxed/@formulas/src.superfly
  1. It needs to be very deep and detailed
  2. It should be created in a `./superfly` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

29. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.extism
  1. It needs to be very deep and detailed
  2. It should be created in a `./extism` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

30. [ ] /home/darkvoid/Boxxed/@formulas/src.zeromicro
  1. It needs to be very deep and detailed
  2. It should be created in a `./zeromicro` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

31. [ ] /home/darkvoid/Boxxed/@formulas/src.hyperflask
  1. It needs to be very deep and detailed
  2. It should be created in a `./hyperflask` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

32. [ ] /home/darkvoid/Boxxed/@formulas/src.kobweb
  1. It needs to be very deep and detailed
  2. It should be created in a `./kobweb` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

33. [ ] /home/darkvoid/Boxxed/@formulas/src.localfirst
  1. It needs to be very deep and detailed
  2. It should be created in a `./localfirst` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

34. [ ] /home/darkvoid/Boxxed/@formulas/src.MendableAI
  1. It needs to be very deep and detailed
  2. It should be created in a `./MendableAI` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

35. [ ] /home/darkvoid/Boxxed/@formulas/src.nordcraftengine
  1. It needs to be very deep and detailed
  2. It should be created in a `./nordcraftengine` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

36. [ ] /home/darkvoid/Boxxed/@formulas/src.opencontainer
  1. It needs to be very deep and detailed
  2. It should be created in a `./opencontainer` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

37. [ ] /home/darkvoid/Boxxed/@formulas/src.OpenDevin
  1. It needs to be very deep and detailed
  2. It should be created in a `./OpenDevin` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

38. [ ] /home/darkvoid/Boxxed/@formulas/src.OpenMCP
  1. It needs to be very deep and detailed
  2. It should be created in a `./OpenMCP` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

39. [ ] /home/darkvoid/Boxxed/@formulas/src.rivet-dev
  1. It needs to be very deep and detailed
  2. It should be created in a `./rivet-dev` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

40. [ ] /home/darkvoid/Boxxed/@formulas/src.Peer2Peer
  1. It needs to be very deep and detailed
  2. It should be created in a `./Peer2Peer` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

41. [ ] /home/darkvoid/Boxxed/@formulas/src.pheonixLiveView
  1. It needs to be very deep and detailed
  2. It should be created in a `./pheonixLiveView` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

42. [ ] /home/darkvoid/Boxxed/@formulas/src.trpc
  1. It needs to be very deep and detailed
  2. It should be created in a `./trpc` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

43. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.deployAnywhere/smithy
  1. It needs to be very deep and detailed
  2. It should be created in a `./smithy` directory and any sub-directory should exist under this directory and nowhere else
  3. Include the following additional directories under this exploration:
    a. /home/darkvoid/Boxxed/@formulas/src.rust/src.deployAnywhere/smithy-examples
    b. /home/darkvoid/Boxxed/@formulas/src.rust/src.deployAnywhere/smithy-python
    c. /home/darkvoid/Boxxed/@formulas/src.rust/src.deployAnywhere/smithy-rs
    d. /home/darkvoid/Boxxed/@formulas/src.rust/src.deployAnywhere/smithy-typescript
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

42. [ ] /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/hono
  1. It needs to be very deep and detailed
  2. It should be created in a `./hono` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.
  
45. [ ] /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/shoelace
  1. It needs to be very deep and detailed
  2. It should be created in a `./shoelace` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.
  
46. [ ] /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/HTMX
  1. It needs to be very deep and detailed
  2. It should be created in a `./htmlx` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

47. [ ] /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.11ty
  1. It needs to be very deep and detailed
  2. It should be created in a `./11ty` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

48. [ ] /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/once
  1. It needs to be very deep and detailed
  2. It should be created in a `./basecamp/once` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.
  
48. [ ] /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/kamal
  1. It needs to be very deep and detailed
  2. It should be created in a `./basecamp/kamal` directory and any sub-directory should exist under this directory and nowhere else
  2.1: Include the following directories under this:
    a. /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/kamal-proxy
    b. /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/kamal-site
    c. /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/kamal-skiff
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

48. [ ] /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/gh-signoff
  1. It needs to be very deep and detailed
  2. It should be created in a `./basecamp/gh-signoff` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, and how it helps to execute your own  CI/CD locally.
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.

48. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.Joy
  1. It needs to be very deep and detailed
  2. It should be created in a `./joy` directory and any sub-directory should exist under this directory and nowhere else
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, and how it helps to execute your own  CI/CD locally.
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.
  

49. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.ArrowAndDBs/src.duckdb/duckdb
  1. It needs to be very deep and detailed.
  2. It should be created in a `./duckdb` directory and any sub-directory should exist under this directory and nowhere else. Include other directories under /home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.ArrowAndDBs/src.duckdb within this.
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, and how it helps to execute your own  CI/CD locally.
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.
  
50. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.backtrace-labs
  1. It needs to be very deep and detailed.
  2. It should be created in a `./backtrace` directory and any sub-directory should exist under this directory and nowhere else. Include other directories under /home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.ArrowAndDBs/src.duckdb within this.
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, and how it helps to execute your own  CI/CD locally.
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.
  
51. [ ] /home/darkvoid/Boxxed/@formulas/src.rust/src.SpacetimeDB/SpacetimeDB
  1. It needs to be very deep and detailed.
  2. It should be created in a `./spacetimedb` directory and any sub-directory should exist under this directory and nowhere else. Include other directories under /home/darkvoid/Boxxed/@formulas/src.rust/src.SpacetimeDB/ within this.
  3. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
  4. Do a good cover and add fundamental explainers for the algorithms, concepts used, and how it helps to execute your own  CI/CD locally.
  5. How do we replicate the implementation in the project in Rust, what we need to keep in mind
  6. How do we replicate this without the aws-lambda-rust-runtime, what http API must we follow, what data and response must we create to be 1 to 1 compatible with the runtime, how do we correctly expose the API to be callable from lambda, be very detailed, cover all areas for production level replication in rust without async/tokio with valtron from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron (see /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators and /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/10-simple-http-client-enhancements)
  7. Each must have a detailed first principle explanation for someone who has not done ML, AI related logic, code or project before, step by step guide, explanations with detailed and clear answers so detailed that is close to a mini textbook to move me from zero to ML engineer by the time i am done, skip nothing, cover everything.
  
---
