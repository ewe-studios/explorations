---
title: 01 Repository map
---

# 01 Repository map

This document maps the core repository at commit `4a58441c50d1ce20d8803cfb1ab7c3285a5141d6`, version 2.3.31. Section 10 lists the sibling repositories and their own head commits. The core repository holds two kinds of code. The first kind is the Rust control plane, called the engine, with 76 of the 88 Cargo workspace members. The second kind is the client software development kits (SDKs) for TypeScript, Rust, Python, Swift, Go, and generated schemas. The central finding is that the workspace uses many small crates for a few source-stated boundaries. The remaining splits follow a consistent pattern that the source does not state as a rule (section 13). The map gives every engine crate and every TypeScript and Rust SDK package a row, so a reader can find the crate that owns a concern without a full-text search.

This map uses several short forms. The short form API means Application Programming Interface. HTTP means Hypertext Transfer Protocol. JSON means JavaScript Object Notation. SQL means Structured Query Language. SQLite is the name of the Structured Query Language database engine. TLS means Transport Layer Security. CLI means command line interface. SDK means software development kit. WASM means WebAssembly. NAPI means Node-API. RTT means round-trip time. OTLP means OpenTelemetry Protocol. URL means Uniform Resource Locator. OpenAPI means the OpenAPI Specification. AsyncAPI means the AsyncAPI Specification. NATS is the name of the publish-subscribe server. PyO3 is the Rust binding for Python. N-API means Node-API. VFS means virtual file system. SHA means Secure Hash Algorithm. FDB means FoundationDB. TS and TSX are TypeScript file names with and without JavaScript XML (JSX).

Some tables use a crate alias, such as `gas` for `gasoline`. Section 3.20 lists the aliases. The words Actor, runner, envoy, depot, bucket, and vbare appear below and in the glossary at the end of this document. A `CLAUDE.md` file is an instruction file for a coding agent. This map uses a `CLAUDE.md` file only for a design statement, and it checks that statement against the code or the manifest when it can.

All paths in this document are relative to `/Users/darkvoid/GitSources/src.Rivet/rivet` unless the text names the sibling snapshot. A citation has the form `path:line` or `path:line-line`. A citation of the form `path:line,line` names two separate lines in the same file. A comma always separates distinct line numbers. The source code is the authority. Where an internal document and the code disagree, this map follows the code and says so.

The sibling repositories under `/Users/darkvoid/GitSources/src.Rivet/` appear in section 10 of this map and in [17 The product family](17-the-product-family.md). That document is the deep dive. This map reads the top-level README, the package manifests, the package READMEs, and the named specification files of each sibling. It does not read the application source.

### What Rivet is

Rivet is an orchestrator for agentic workloads (`README.md:60`). It schedules Actors. An Actor is a long-lived process with durable state, a SQLite database, a queue, realtime connections, and a schedule. Each Actor has a key, and that key is the Actor's unique identifier (`README.md:60`). The queue holds work items for the Actor. A realtime connection is a live client link. The schedule starts the Actor at a set time. This map infers those three roles from the product list; the README names the features and does not trace them. Hosting an Actor is a role. The legacy implementation of that role is a runner. The current implementation is an envoy. An envoy reaches the engine through a tunnel. A tunnel is the Universal PubSub channel that carries request and response frames between a gateway and the envoy or runner (`docs-internal/engine/PEGBOARD_TUNNEL_RETRIES.md:5`). Depot is the storage subsystem that holds each Actor's SQLite pages, which are 4,096-byte units (section 3.7), in a log-structured form. The schema tool vbare generates the versioned wire types. A bucket is the object storage service. The product description calls the bucket the only external durable store (`README.md:68-69`). The source does not name the storage provider. In the open-source build, durable data lives in RocksDB or PostgreSQL. The bucket cold tier, which is the long-term object-storage copy that the enterprise build writes, stays empty in that build (`docs-internal/engine/oss-to-ee-upgrade.md:9-19`).

## 1. What this map covers and how to read it

The map answers five questions.

1. Which directories exist at the top of the repository, and what does each one hold?
2. Which crates make up the 88 Cargo workspace members, and what does each crate do?
3. Which packages make up the TypeScript and Rust SDK workspaces?
4. Where does each cross-cutting concern live: scheduling, durable execution, SQLite, storage, routing, authentication, telemetry, deployment, and tests?
5. How do the layers depend on each other, and why is the workspace split into many small crates?

Section 2 gives the directory tree. Section 3 gives the Cargo workspace in 17 groups. Section 4 gives the TypeScript workspace. Section 5 gives the Rust SDK workspace. Section 6 gives the other SDK directories. Section 7 gives the documentation. Section 8 gives the examples, integrations, scripts, and the shared TypeScript package. Section 9 gives the deployment assets. Section 10 gives the sibling repositories. Section 11 gives the diagrams. Section 12 gives the concern index. Section 13 gives the main conclusion about the shape of the workspace. This map calls that conclusion an **Aha**. An Aha is the one finding that explains the rest of the document.

### 1.1 The language of the tables

Each crate table has four columns:

| Column | Meaning |
|--------|---------|
| Crate | The exact `name` field from the crate `Cargo.toml`. |
| Path | The directory that holds the crate manifest. |
| Purpose | One sentence. The sentence comes from the manifest `description` when the manifest has one. A dagger (†) marks a sentence that this map inferred from the module names in `src/lib.rs`, the public interface, and the dependencies in `Cargo.toml`. |
| Main dependency | The dependency that most determines the crate's internal structure, chosen from the normal `[dependencies]` table. The column is illustrative, not a computed ranking. When the most useful dependency is external, the column names it with the prefix `external`. When the crate has only build dependencies, the column uses the build dependency and says so. |

The manifest gives a `description` for only 13 of the 76 packages. The other 63 purposes are inferences. The document marks every inferred sentence. An inference uses the module names in `src/lib.rs`, the public interface, and the dependencies in `Cargo.toml`. It does not use the directory name alone.

Three words in this map have more than one meaning. The map qualifies each use.

- **Worker.** The `rivet-workflow-worker` service hosts Gasoline workflows (`engine/packages/workflow-worker/src/lib.rs:5-11`). The legacy Actor host is a **runner**, and the current Actor host is an **envoy**. Neither is a worker. Cloudflare Workers is an external platform, and `@rivetkit/cloudflare-workers` is its integration.
- **Gateway.** The crates `pegboard-gateway`, `pegboard-gateway2`, and `pegboard-gateway3` terminate the Actor-host tunnel on the engine side (section 3.13). The request path is client to Guard to gateway to envoy or runner (section 3.11). The guard module `routing/pegboard_gateway` selects which gateway serves a request (section 3.11). The HTTP path `/gateway/{actor_id}/...` is an Actor route that the guard accepts (`docs-internal/engine/GUARD.md:85-87`). These are layers of one request path, not three products.
- **Envoy.** A Rivet envoy is the Actor-hosting process on the user side. The internal note `docs-internal/engine/actor-rescheduling.md:7` defines it. The note says: "Throughout this document 'envoy' means the actor-hosting process on the user side." A Rivet envoy is not the Cloud Native Computing Foundation (CNCF) Envoy proxy.

### 1.2 What the snapshot contains

The core repository holds these counts at commit `4a58441c50d1ce20d8803cfb1ab7c3285a5141d6`:

| Item | Count | Method |
|------|-------|--------|
| Cargo workspace members | 88 | Quoted entries in `Cargo.toml:3-92` |
| Engine packages | 76 | 66 under `engine/packages`, 10 under `engine/sdks/rust` |
| Rust files under `engine/` | 1,223 | `find engine -name '*.rs'` |
| Rust files under `rivetkit-rust/` | 171 | `find rivetkit-rust -name '*.rs'` |
| TypeScript and TSX files under `rivetkit-typescript/` | 573 | `find rivetkit-typescript -name '*.ts' -o -name '*.tsx'` |
| Example projects | 57 | Directories under `examples/` |
| Script directories | 22 | Directories under `scripts/` |
| Documentation files | 143 | `find docs -type f` |

The counts come from commands, not from a README. The count of 88 members includes 22 entries outside `engine/packages`: 10 under `engine/sdks/rust`, 8 RivetKit Rust crates, 2 TypeScript build targets, and 2 examples. Section 3.1 gives the arithmetic.

## 2. The top-level tree

The repository has 17 functional directories and several files at the top level. The tree below shows the functional directories only. Agent instruction directories such as `.claude`, `.agent`, `.agents`, `.amp`, and `.opencode` are present but are not part of the product.

```text
# Synthesized from ls -1A at the repository root; not copied from a file.
rivet/
├── engine/                 Rust control plane, 1223 Rust files
│   ├── packages/           66 crates
│   ├── sdks/               protocol schemas and language SDKs
│   ├── artifacts/          generated OpenAPI, config schema, error catalog
│   └── tests?              referenced by pnpm, absent in this commit
├── rivetkit-typescript/    TypeScript SDK, 18 packages, 573 TS/TSX files
├── rivetkit-rust/          Rust SDK, 8 packages, 171 Rust files
├── rivetkit-python/        PyO3 client, one package
├── rivetkit-swift/         Swift package
├── rivetkit-openapi/       OpenAPI 3.0.0 document, version 2.2.0
├── rivetkit-asyncapi/      AsyncAPI 3.0.0 document, version 2.3.31
├── rivetkit-json-schema/   registry configuration schema
├── frontend/               dashboard, inspector, and shared UI packages
├── docs/                   published documentation, 143 files
├── docs-internal/          design notes, 42 entries under engine/
├── examples/               57 example projects
├── integrations/           2 integration packages
├── scripts/                22 tooling directories
├── self-host/              compose files, workers, control plane samples
├── shared/                 one TypeScript package: virtual-websocket
├── docker/                 builder images and engine images
└── website?                absent; the site is a sibling repository
```

### 2.1 The top-level directories

The last column names a file or a directory that a reader can open first. It is a reading hint, not a program entry point. A path that ends with `/` is a directory.

| Directory | What it holds | Start here |
|-----------|---------------|-------------|
| `engine` | The Rust control plane. It holds 66 crates in `engine/packages`, 10 crates in `engine/sdks/rust`, versioned protocol schemas, TypeScript engine SDKs, a Go client, and the generated artifacts in `engine/artifacts`. | `engine/packages/engine/src/run_config.rs` |
| `rivetkit-typescript` | The TypeScript SDK. It holds the `rivetkit` package, framework integrations, the native addon, the WebAssembly build, and developer tools. | `rivetkit-typescript/packages/rivetkit` |
| `rivetkit-rust` | The Rust SDK. It holds the user-facing `rivetkit` crate, the runtime core, the client, the persistence and inspector protocols, and the engine process helper. | `rivetkit-rust/packages/rivetkit` |
| `rivetkit-python` | A Python client built with PyO3 and maturin. The package name is `rivetkit-client`, version 0.9.0-rc.1. | `rivetkit-python/client/pyproject.toml` |
| `rivetkit-swift` | A Swift package with the `RivetKitClient` and `RivetKitSwiftUI` products. | `rivetkit-swift/Package.swift` |
| `rivetkit-openapi` | One file, `openapi.json`. It declares OpenAPI 3.0.0, title "RivetKit API", version 2.2.0. | `rivetkit-openapi/openapi.json` |
| `rivetkit-asyncapi` | One file, `asyncapi.json`. It declares AsyncAPI 3.0.0, title "RivetKit WebSocket Protocol", version 2.3.31. | `rivetkit-asyncapi/asyncapi.json` |
| `rivetkit-json-schema` | One file, `registry-config.json`. It describes the object that a user passes to `setup()`, the function that builds the TypeScript Actor registry (section 6.5). | `rivetkit-json-schema/registry-config.json` |
| `frontend` | The dashboard and the inspector. The main application is `frontend/src`, built with Vite. | `frontend/package.json` |
| `docs` | Four documentation bundles, published by a separate website repository. | `docs/api/endpoints.json` |
| `docs-internal` | Design notes and implementation maps for the engine, the TypeScript SDK, and integrations. | `docs-internal/engine/` |
| `examples` | 57 runnable projects that cover frameworks, storage, realtime, workflows, and deployment targets. | each example has its own manifest |
| `integrations` | Two packages: `@rivet-dev/flue` and `@rivet-dev/workflow-world`. | `integrations/*/package.json` |
| `scripts` | 22 tooling directories for builds, releases, code generation, tests, and formatting. | `scripts/*` |
| `self-host` | Compose samples for development, worker samples for eleven platforms, and control plane samples for six platforms. | `self-host/compose/` |
| `shared` | One TypeScript package, `@rivetkit/virtual-websocket`. | `shared/typescript/virtual-websocket` |
| `docker` | Builder base images, cross-build images for seven targets, the engine image, and a compute hello world. | `docker/engine/Dockerfile` |

### 2.2 The `website` question

The repository does not contain a `website` directory. The `ls -1A` result at the top level has no such entry. The documentation site is a sibling repository at `/Users/darkvoid/GitSources/src.Rivet/website`, with head commit `10c8c186cc24d0aa54f15cedb9c2658fa4276483`. The `docs` directory in the core repository is the content that the website publishes. Section 10.8 gives the one-paragraph summary.

### 2.3 Directories that a workspace glob names but the commit does not contain

`pnpm-workspace.yaml:9-31` lists four paths that do not exist at this commit:

| Glob | State at this commit | Effect |
|------|----------------------|--------|
| `engine/tests/*` (line 17) | `engine/tests` is absent | The glob matches nothing. Document [16 Tests, benchmarks, and limits](16-tests-benchmarks-and-limits.md) covers where the tests live. |
| `engine/sdks/typescript/test-envoy` (line 14) | Absent | The glob matches nothing. |
| `engine/sdks/typescript/test-envoy-native` (line 15) | Absent | The glob matches nothing. |
| `scripts/vercel-examples` (line 30) | Absent | The glob matches nothing. |

The four missing paths are a real difference between the manifest and the file tree. This map does not treat the glob as evidence that a directory exists. Document [16 Tests, benchmarks, and limits](16-tests-benchmarks-and-limits.md) covers the test layout.

## 3. The engine Cargo workspace: 76 of 88 members

### 3.1 The arithmetic of 88

`Cargo.toml:3-92` lists the members. The list has 88 quoted entries. The entries divide as follows:

| Part | Count | Example |
|------|-------|---------|
| `engine/packages/*` | 66 | `engine/packages/pegboard` |
| `engine/sdks/rust/*` | 10 | `engine/sdks/rust/envoy-protocol` |
| `rivetkit-rust/packages/*` | 8 | `rivetkit-rust/packages/rivetkit-core` |
| `rivetkit-typescript/packages/*` | 2 | `rivetkit-typescript/packages/rivetkit-napi` |
| `examples/*` | 2 | `examples/chat-room-rust` |
| **Total** | **88** | |

This map calls the 76 packages under `engine/` the engine packages. That set is the 66 plus the 10. The two TypeScript entries in the workspace are native build targets: the Node-API (NAPI) addon and the WebAssembly (WASM) build. The two example entries are Rust examples. Sections 4.2, 5, and 8.1 describe the 12 members outside the engine set.

```toml
# Cargo.toml:3-10
members = [
  "engine/packages/auth",
  "engine/packages/auth-jwt",
  "engine/packages/auth-policy",
  "engine/packages/api-builder",
  "engine/packages/api-peer",
  "engine/packages/api-public",
  "engine/packages/api-public-openapi-gen",
```

```toml
# Cargo.toml:86-92
  "rivetkit-rust/packages/rivetkit-core",
  "rivetkit-rust/packages/shared-types",
  "rivetkit-typescript/packages/rivetkit-napi",
  "rivetkit-typescript/packages/rivetkit-wasm",
  "examples/chat-room-rust",
  "examples/hello-world-rust"
]
```

The workspace inherits one version for every workspace crate. `Cargo.toml:95-96` sets `version = "2.3.31"` and `edition = "2024"`. A crate writes `version.workspace = true` and `edition.workspace = true`. Every Cargo workspace crate uses the shared version. The version is not global across the repository. The Python client is 0.9.0-rc.1. The OpenAPI document `rivetkit-openapi/openapi.json` is 2.2.0. Two TypeScript packages, `@rivetkit/devtools` and `@rivetkit/mcp-hub`, are 2.2.1. The dashboard package is 2.0.21.

### 3.2 The 17 groups and their sizes

The grouping below is editorial. Each of the 76 crates appears in exactly one group. The groups follow the concern that the crate serves, not the directory that holds it.

| Group | Crates | Section |
|-------|--------|---------|
| Application Programming Interface (API) | 10 | 3.3 |
| Authentication and authorization | 3 | 3.4 |
| Cache | 3 | 3.5 |
| Configuration | 5 | 3.6 |
| Depot SQLite storage | 5 | 3.7 |
| Engine binary and process services | 8 | 3.8 |
| Epoxy replication | 2 | 3.9 |
| Gasoline durable execution | 3 | 3.10 |
| Guard proxy | 3 | 3.11 |
| Metrics | 2 | 3.12 |
| Pegboard scheduling | 9 | 3.13 |
| Runtime and shared infrastructure | 7 | 3.14 |
| Storage and wire protocols | 5 | 3.15 |
| Telemetry and logs | 2 | 3.16 |
| Tests | 4 | 3.17 |
| Utilities | 4 | 3.18 |
| Workflow worker | 1 | 3.19 |
| **Total** | **76** | |

### 3.3 API crates (10)

This group holds the API crates and the shared types and clients that they use. The public API serves users and the dashboard. The peer API serves envoys and runners inside the cluster. `rivet-types` defines the domain types. `rivet-data` holds generated data types. `namespace` holds namespace records. `rivet-api-full` is the generated client for the public HTTP API.

| Crate | Path | Purpose | Main dependency |
|-------|------|---------|-----------------|
| `rivet-api-builder` | `engine/packages/api-builder` | Builds axum routers with shared contexts, extractors, middleware, error responses, request identifiers, and metrics. † | external `axum` |
| `rivet-api-peer` | `engine/packages/api-peer` | Serves the internal API for Actor operations, depot inspection, envoys, namespaces, runner configurations, and runners. † | `depot` |
| `rivet-api-public` | `engine/packages/api-public` | Serves the public API for Actors, authentication, datacenters, envoys, health, metadata, namespaces, runner configurations, runners, and the embedded user interface. † | `gas` |
| `rivet-api-public-openapi-gen` | `engine/packages/api-public-openapi-gen` | Generates the public OpenAPI document in a build script. It has build dependencies only. † | `rivet-api-public` (build) |
| `rivet-api-types` | `engine/packages/api-types` | Defines the request and response types that the public and peer APIs share. † | `rivet-api-builder` |
| `rivet-api-util` | `engine/packages/api-util` | Provides error conversion and request helpers that the API crates share. † | `rivet-api-builder` |
| `rivet-types` | `engine/packages/types` | Defines the domain types for Actors, datacenters, envoys, keys, namespaces, runner configurations, and runners. † | `rivet-api-builder` |
| `rivet-api-full` | `engine/sdks/rust/api-full` | Holds the generated OpenAPI client with `apis` and `models` modules. The manifest says it is generated by OpenAPI Generator. | external `reqwest` |
| `rivet-data` | `engine/sdks/rust/data` | Holds generated data types and version constants for the other protocol crates. † | `rivet-runner-protocol` |
| `namespace` | `engine/packages/namespace` | Implements namespace records, keys, operations, and workflows on the durable store. † | `gas` |

A namespace is a named scope with its own identifier. Actors, runner configurations, and store keys carry a `namespace_id` (`engine/packages/types/src/namespaces.rs:5-10`). The `gas` column for `namespace` and `datacenter` reflects their Gasoline workflow registries (`engine/packages/namespace/src/lib.rs:9-15`; `engine/packages/datacenter/src/lib.rs:7-13`). For `rivet-api-public`, the source uses the Gasoline prelude in one utility module (`engine/packages/api-public/src/runner_configs/utils.rs:4`). The column is illustrative, as section 1.1 states.

The separation of `rivet-api-types` from `rivet-api-peer` has a source-stated reason. The internal note `docs-internal/engine/API.md:7` says that the shared request types cannot live in `rivet-api-peer`. The note gives the reason as a circular dependency: `rivet-api-peer` needs `pegboard`, and `pegboard` would need `rivet-api-peer` for the types.

The manifests show three real Cargo edges. `rivet-api-peer` lists `pegboard` at `engine/packages/api-peer/Cargo.toml:20`. `rivet-api-peer` also lists `rivet-api-types` at `:22`. `pegboard` lists `rivet-api-types` at `engine/packages/pegboard/Cargo.toml:26`. `pegboard` does not list `rivet-api-peer`. The note describes a cycle that the shared crate breaks before it becomes a Cargo edge. If the types lived in `rivet-api-peer`, then `pegboard` would need `rivet-api-peer` to use them, and the cycle would be real. `rivet-api-types` removes that need, so both sides use it. `rivet-api-types` depends on `rivet-api-builder` (`engine/packages/api-types/Cargo.toml:12`), and the peer crate depends on the types crate, not the reverse.

Three generators appear in this map. `utoipa` writes the engine OpenAPI document (section 6.3). Fern writes the TypeScript and Go clients (section 6.9). OpenAPI Generator writes the Rust client (`scripts/openapi/gen_rust.ts:9-10,40-50`). The `rivet-api-full` manifest names OpenAPI Generator, not one of the other two (`engine/sdks/rust/api-full/Cargo.toml:6`).

`rivet-api-public` also holds the dashboard mount point. Its `ui` module at `engine/packages/api-public/src/lib.rs:13` is the place where the built frontend assets are served. `BASE_URL` is the Vite public asset path prefix. The embedded build sets it to `/ui/`, which must match the mount path so the browser loads assets from the same prefix (`frontend/CLAUDE.md:7`). Section 4.3 covers the frontend build. An axum router maps request paths and methods to handlers. axum is the Rust HTTP framework.

### 3.4 Authentication and authorization crates (3)

| Crate | Path | Purpose | Main dependency |
|-------|------|---------|-----------------|
| `rivet-auth` | `engine/packages/auth` | Checks Actor and namespace authorization scopes against policy types. † | `rivet-auth-policy` |
| `rivet-auth-jwt` | `engine/packages/auth-jwt` | Issues, verifies, and persists JSON Web Tokens (JWTs), with claims, codecs, grants, key rings, planners, token identifiers, and workflows. † | `rivet-auth-policy` |
| `rivet-auth-policy` | `engine/packages/auth-policy` | Defines the policy types and the `authorize` checks that the other authentication crates call. † | `rivet-error` |

`rivet-auth-jwt` is one of the few auth crates with a feature flag in the dependency graph. `engine/packages/guard/Cargo.toml:46` enables its `service` feature. `engine/packages/engine/Cargo.toml` also enables `service`. The feature gates the `issuer`, `key_ring_cache`, `metrics`, `ops`, `storage_keys`, and `workflows` modules (`engine/packages/auth-jwt/src/lib.rs:12-23`). These modules issue tokens, cache key rings, read storage keys, and run authentication workflows. The `planner` module computes the JWT signing-key rotation schedule (`engine/packages/auth-jwt/src/planner.rs:11-30`). Document [13 Authentication and namespaces](13-auth-and-namespaces.md) covers the token flow.

### 3.5 Cache crates (3)

| Crate | Path | Purpose | Main dependency |
|-------|------|---------|-----------------|
| `rivet-cache` | `engine/packages/cache` | Provides the cache driver, keys, getter context, metrics, purge support, and request configuration, backed by `moka`. † | `rivet-cache-result` |
| `rivet-cache-purge` | `engine/packages/cache-purge` | Runs the workflow that purges expired cache entries. † | `rivet-cache` |
| `rivet-cache-result` | `engine/packages/cache-result` | Defines the result type that a cache getter returns. † | `rivet-util` |

### 3.6 Configuration crates (5)

| Crate | Path | Purpose | Main dependency |
|-------|------|---------|-----------------|
| `rivet-config` | `engine/packages/config` | Loads and validates process configuration, holds dynamic configuration and build metadata, and exposes the runtime protocol versions. † | external crate `config` (third-party, not a workspace crate) |
| `rivet-config-schema-gen` | `engine/packages/config-schema-gen` | Generates the configuration JSON Schema in a build script. It has build dependencies only. † | `rivet-config` (build) |
| `rivet-dynamic-config` | `engine/packages/dynamic-config` | Watches dynamic configuration and broadcasts changes to long-lived workers through a watch channel and the publish-subscribe layer. † | `rivet-config` |
| `rivet-env` | `engine/packages/env` | Reads environment variables into typed constants such as the environment name and process identifiers. † | external `lazy_static` |
| `rivet-build-meta` | `engine/packages/build-meta` | Stamps the git commit hash, build timestamp, package version, and compiled runtime protocol versions into the binary. † | `rivet-config` |

`rivet-config` is unusual because it has no dependency on another workspace crate. Its only direct dependency is the third-party `config` crate, which reads files and environment variables (`engine/packages/config/Cargo.toml:12`). The dependency direction is inward: other crates depend on `rivet-config`. `rivet-build-meta` depends on `rivet-config`, and the engine binary depends on both. The build metadata flows in one direction only. The next two code blocks show that flow.

```rust
// engine/packages/build-meta/src/lib.rs:1-9
//! Build-time metadata stamped into the binary by `build.rs`.
//!
//! This lives in its own crate because the git SHA and build timestamp change on every commit, and
//! `build.rs` declares them as `rerun-if-env-changed`. Anything that depends on this crate
//! therefore recompiles on every new commit, so only the binary should depend on it. Everything
//! else reads the values off `rivet_config::Config::build_meta` after [`publish`] stamps them.

/// Runtime identifier for the engine
pub const RUNTIME: &str = "engine";
```

The comment gives the reason for the separate crate. A commit changes the git hash and the timestamp. The build script declares those values with `rerun-if-env-changed`. Every crate that depends on `rivet-build-meta` therefore rebuilds on every commit. The crate exists so that only the `rivet-engine` binary and its exclusive dependency subtree pay that cost. An exclusive dependency is a crate that nothing except the binary and that subtree depends on. The rest of the tree reads the values through `rivet-config`.

```rust
// engine/packages/config/src/lib.rs:31-38
	/// Build metadata for this process, stamped once at startup by the binary that has it.
	///
	/// This does not come from the config file or the environment. It lives here so that reading
	/// build metadata does not require depending on the crate that carries it, which is rebuilt on
	/// every commit. Unlike `dynamic`, it is process-local and is never broadcast.
	build_meta: Arc<BuildMeta>,
	protocols: RwLock<Arc<RuntimeProtocols>>,
}
```

The `Config` struct holds `build_meta` as an `Arc<BuildMeta>` and `protocols` as a `RwLock<Arc<RuntimeProtocols>>`. The field is process-local. The binary stamps it once. No other process receives it over the network.

`engine/CLAUDE.md:70-77` states the second reason for the split:

```markdown
<!-- engine/CLAUDE.md:70-77 -->
## Build metadata

- `rivet-build-meta`'s build script re-stamps the git SHA and build timestamp on every commit, so everything downstream of it recompiles each commit. Only crates that nothing but the `rivet-engine` binary pulls in may depend on it.
- The binary reads the constants once and passes them into `rivet_config::Config::load`. Everywhere else reads them through `rivet_config::Config::build_meta()`.

## Runtime protocol version sync

- Compiled runtime protocol versions are assembled in `rivet_build_meta::compiled_runtime_protocols()` rather than the config crate, so a protocol bump does not recompile everything that depends on `rivet-config`.
```

The second heading in that file, Runtime protocol version sync, states a separate boundary. A runtime protocol version is a number that two processes exchange. If the number lived in `rivet-config`, then a protocol bump would change `rivet-config`, and every crate that depends on `rivet-config` would rebuild. The version lives in `rivet-build-meta` instead. Only the binary and the crates that only it pulls in depend on that crate. The cost is one more crate and one more indirection. The benefit is that a protocol bump rebuilds the binary and not the tree. Document [02 The control plane](02-control-plane-architecture.md) covers the service that consumes these versions.

### 3.7 Depot SQLite storage crates (5)

| Crate | Path | Purpose | Main dependency |
|-------|------|---------|-----------------|
| `depot` | `engine/packages/depot` | Implements per-Actor Structured Query Language (SQL) storage: the conveyer, log-structured transaction pages, compaction, garbage collection, recovery, inspection, and quota. † | `depot-client-types` |
| `rivet-depot-client` | `engine/packages/depot-client` | Provides the native SQLite virtual file system client that reads and writes database pages through the depot transport. The manifest description is "Depot-backed native SQLite VFS client". | `depot-client-types` |
| `depot-client-embedded` | `engine/packages/depot-client-embedded` | Implements the in-process depot transport, so a client can run without a network. The manifest description is "Embedded Depot transport for depot-client SQLite VFS". | `depot-client` |
| `rivet-depot-client-types` | `engine/packages/depot-client-types` | Defines the SQLite execution types and the shard constants that the client and the engine must agree on. The manifest description is "Shared SQLite execution types for depot client". | none (no dependency table) |
| `rivet-depot-protocol` | `engine/sdks/rust/depot-protocol` | Holds the generated, versioned depot wire protocol types. † | external `vbare` |

The conveyer is the depot submodule that holds the log-structured transaction, the page index, the read path, and the restore point. A restore point is a recorded database position that an operator can restore later (`engine/packages/depot/src/conveyer/restore_point.rs:1-26`). The source spells the name `conveyer`.

Six storage terms need a definition here.

1. A page is the 4,096-byte unit that SQLite reads and writes (`engine/packages/depot-client/src/vfs.rs:36`).
2. A storage shard is 64 pages, or 256 kibibytes (`engine/packages/depot-client-types/src/lib.rs:10-13`).
3. A staged commit segment is one shard-aligned block of dirty pages. It appears in a commit that is too large for one message. It may span 5 shards, or 320 pages (`engine/packages/depot-client/src/vfs.rs:122-139`; `engine/packages/depot-client-types/src/lib.rs:15-16`).
4. A PIDX row is a page-index row. It records the transaction that owns a page (`engine/packages/depot/src/recovery.rs:621-629`).
5. A write conflict range is a key range that UniversalDB records for conflict detection (`engine/packages/universaldb/src/transaction.rs:567-575`; `engine/packages/universaldb/src/options.rs:85`).
6. Truncate cleanup deletes every PIDX row above the new end of file after a commit shrinks the database (`engine/packages/depot-client-types/src/lib.rs:26-27`).

A staged commit works in four steps. The client VFS collects dirty pages in memory. It cuts them into shard-aligned segments and sends each segment to the engine. The engine buffers the segments for that transaction. The finalize call commits the whole set at once (`engine/packages/depot-client/src/vfs.rs:122-142`). A segment is a transport request, and the engine does not expose a staged page before finalize (`engine/packages/depot-client/src/vfs.rs:2692-2695`).

The shared constants in `rivet-depot-client-types` carry a source-stated reason. The comment says that the client cuts commit segments on shard boundaries and the engine rejects segments that are not cut on them. Two private copies could disagree, and every large commit would fail. The next block shows the constants.

```rust
// engine/packages/depot-client-types/src/lib.rs:10-33
/// Pages per storage shard. Shared rather than each side keeping its own copy: the client cuts
/// staged commit segments on these boundaries and the engine rejects segments that are not cut on
/// them, so two definitions could disagree and make every large commit fail.
pub const SHARD_SIZE: u32 = 64;

/// Shards one staged commit segment may span.
pub const COMMIT_SEGMENT_MAX_SHARDS: u32 = 5;

/// Dirty pages one commit may carry, staged or not.
///
/// A PIDX row is only about 64 bytes of key and value, but that is not what FoundationDB charges
/// against its 10 MB transaction limit. Each `set` also carries a write conflict range, and the
/// conflict ranges dominate: measured against a real cluster, the finalize transaction costs roughly
/// 170 bytes per page, so the hard ceiling is near 61,000 pages. A commit of 60,146 pages publishes
/// and one of 62,152 fails with a non-retryable `transaction_too_large`.
///
/// Half of that measured ceiling is the cap. The remaining headroom is not slack: truncate cleanup
/// deletes every above-EOF PIDX row in the same transaction and is not bounded by this constant at
/// all, so a commit that shrinks a large database pays for pages this number never counted.
///
/// Shared rather than duplicated so the client can refuse an oversized commit before staging any of
/// it. The engine still enforces it, since an old client is not bound by a new value here, but a
/// client that knows the number does not have to send 128 MiB to be told no.
pub const MAX_COMMIT_DIRTY_PAGES: usize = 32_768;
```

The numbers have a direct source. `SHARD_SIZE` is 64 pages. `COMMIT_SEGMENT_MAX_SHARDS` is 5. `MAX_COMMIT_DIRTY_PAGES` is 32,768 pages. The source comment gives the measured ceiling: 60,146 pages publishes, and 62,152 pages fails with `transaction_too_large` (`engine/packages/depot-client-types/src/lib.rs:20-24`).

The arithmetic has one subtlety. 32,768 pages times 4,096 bytes is 134,217,728 bytes, or 128 mebibytes. That value matches the comment at `engine/packages/depot-client-types/src/lib.rs:32`. The constant is exactly 2^15 pages, which is half of 65,536 pages. It is not half of the measured publish ceiling. Half of 60,146 pages is 30,073 pages, about 117.5 mebibytes. The constant sits 2,695 pages above that half. The comment calls the cap "half of that measured ceiling"; the code uses the round power of two instead. This map follows the code and treats the comment as an approximation. Section 3.15 shows that FoundationDB is not a driver in this tree. The comment describes the FoundationDB transaction model that the UniversalDB interface copies, and it does not name the measured cluster. This map does not claim that every UniversalDB driver enforces the 10 MB limit.

The `depot` crate re-exports the conveyer constants, the page index, the policy, the quota, the log-structured transaction module, and the UniversalDB binding. `engine/packages/depot/src/lib.rs:1-21` also shows that the `fault` module is gated behind the `test-faults` feature and the `takeover` module behind `debug_assertions`. The `fault` module injects storage faults at named checkpoints in tests (`engine/packages/depot/src/fault/mod.rs:1-15`). The `takeover` module pauses reconcile work for a test (`engine/packages/depot/src/takeover.rs:1-20`). Document [06 Actor SQLite and Depot](06-actor-sqlite-and-depot.md) traces the write path.

### 3.8 Engine binary and process service crates (8)

| Crate | Path | Purpose | Main dependency |
|-------|------|---------|-----------------|
| `rivet-engine` | `engine/packages/engine` | The binary: defines the command line interface (CLI) commands, the service list in `run_config`, and process utilities. † | `rivet-service-manager` |
| `rivet-bootstrap` | `engine/packages/bootstrap` | Runs one-shot startup work such as backfills across the datacenter, depot, epoxy, gasoline, and namespace layers. † | `depot` |
| `rivet-cli` | `engine/packages/cli` | Provides the command line interface that starts and manages the engine and its subprocesses. † | `rivetkit-engine-process` |
| `rivet-service-manager` | `engine/packages/service-manager` | Starts, supervises, and shuts down the services that make up one engine process. † | `rivet-config` |
| `rivet-version-management` | `engine/packages/version-management` | Tracks runtime protocol versions in the durable store and advances a version only when no older process remains. † | `rivet-config` |
| `rivet-profiling` | `engine/packages/profiling` | Starts and stops continuous profiling through Pyroscope and the publish-subscribe layer. † | `rivet-config` |
| `datacenter` | `engine/packages/datacenter` | Manages datacenter records and the workflows that register and expire them. † | `gas` |
| `rivet-term` | `engine/packages/term` | Formats terminal output, progress, prompts, and status. The manifest description is "Terminal output, formatting, and prompt helpers used by Rivet". | external `tabled` |

`rivet-engine` depends on almost every layer. Its manifest lists `rivet-api-peer`, `rivet-guard`, `rivet-workflow-worker`, `pegboard`, `pegboard-outbound`, `pegboard-runner`, `depot`, `depot-client`, `depot-client-embedded`, `universaldb`, `universalpubsub`, and the service crates. This fan-out is why the binary sits at the top of the dependency graph. The source limits the dependents of `rivet-build-meta` to crates that only the binary pulls in.

The service list is the clearest map of what one process runs. `engine/packages/engine/src/run_config.rs:5-35` shows the first five entries. Each `Service::new` call takes a name, a `ServiceKind`, a start function, and a boolean that enables graceful shutdown.

```rust
// engine/packages/engine/src/run_config.rs:5-35
	let services = vec![
		Service::new(
			"api_peer",
			ServiceKind::ApiPeer,
			|config, pools| Box::pin(rivet_api_peer::start(config, pools)),
			false,
		),
		Service::new(
			"guard",
			ServiceKind::ApiPublic,
			|config, pools| Box::pin(rivet_guard::start(config, pools)),
			true,
		),
		Service::new(
			"workflow_worker",
			ServiceKind::Standalone,
			|config, pools| Box::pin(rivet_workflow_worker::start(config, pools)),
			true,
		),
		Service::new(
			"pegboard_outbound",
			ServiceKind::Standalone,
			|config, pools| Box::pin(pegboard_outbound::start(config, pools)),
			true,
		),
		Service::new(
			"bootstrap",
			ServiceKind::Oneshot,
			|config, pools| Box::pin(rivet_bootstrap::start(config, pools)),
			false,
		),
```

The full list continues at `engine/packages/engine/src/run_config.rs:36-67` with five core services: `dynamic_config`, `version_management`, `epoxy_protocol_version`, `cache_purge`, and `profiling`. The `epoxy_protocol_version` service calls `epoxy::protocol_version::start` (`engine/packages/engine/src/run_config.rs:49-54`). Five `ServiceKind` values appear in this file: `ApiPeer`, `ApiPublic`, `Standalone`, `Oneshot`, and `Core`. The manager also defines `Cron(CronConfig)`, which no service in this file uses (`engine/packages/service-manager/src/lib.rs:246-348`). The boolean is `true` for `guard`, `workflow_worker`, and `pegboard_outbound`. It is `false` for `api_peer`, `bootstrap`, and all five core services.

`ServiceKind` selects how the manager runs a service (`engine/packages/service-manager/src/lib.rs:52-71`).

| Kind | Behavior | Services in this file |
|------|----------|-----------------------|
| `ApiPeer` | Long-running service. The manager restarts it after exit or crash (`engine/packages/service-manager/src/lib.rs:164-197`). | `api_peer` |
| `ApiPublic` | Long-running service, with the same restart rule. | `guard` |
| `Standalone` | Long-running service, with the same restart rule. | `workflow_worker`, `pegboard_outbound` |
| `Oneshot` | Runs until the first success. A crash is retried (`engine/packages/service-manager/src/lib.rs:207-237`). | `bootstrap` |
| `Core` | Long-running service. The source comment says "Run no matter what" (`engine/packages/service-manager/src/lib.rs:59-60`). | `dynamic_config`, `version_management`, `epoxy_protocol_version`, `cache_purge`, `profiling` |
| `Cron(CronConfig)` | Runs on a schedule (`engine/packages/service-manager/src/lib.rs:246-348`). | none in this file |

The `ApiPublic` label on `guard` records the public-facing role, not a different runtime. `ApiPeer` and `ApiPublic` both become `ServiceBehavior::Service` (`engine/packages/service-manager/src/lib.rs:68`).

The fourth argument is `requires_graceful_shutdown`. On a termination signal, the manager calls `abort()` on each service whose flag is false (`engine/packages/service-manager/src/lib.rs:383-391`). It keeps the flagged services running. A force-shutdown timer exits the process with code 1 if they linger (`engine/packages/service-manager/src/lib.rs:369-378`). After the loop, the manager shuts down UniversalDB and the termination-signal task (`engine/packages/service-manager/src/lib.rs:404-408`). The source does not state why a given service has a given flag.

Six components appear in the story of how a user obtains an engine. They have different roles.

| Component | Language | Role | Typical user |
|-----------|----------|------|--------------|
| `rivet-engine` | Rust binary (`engine/packages/engine`) | Runs the control plane and the service list above. | A self-hoster or the cloud platform |
| `rivet-cli` | Rust binary named `rivet` (`engine/packages/cli/Cargo.toml:2,10`) | Starts or reuses an engine for local development (`engine/packages/cli/src/commands/dev.rs:66-85`). | A developer who runs `rivet dev` |
| `rivetkit-engine-process` | Rust crate (`rivetkit-rust/packages/engine-process`) | Resolves, downloads when allowed, spawns, and reuses the engine subprocess (`rivetkit-rust/packages/engine-process/src/lib.rs:136,513`). | Called by `rivet-cli` and the native Rust host |
| `@rivetkit/engine-cli` | TypeScript npm package | Resolves the path of a prebuilt binary from the platform npm package. It does not download or spawn (`rivetkit-typescript/packages/engine-cli/index.js:3-13,47-64`). | A Node.js tool that needs the path |
| `@rivetkit/cli` | TypeScript npm package, bin `rivet` | The Rivet Cloud CLI (`rivetkit-typescript/packages/cli/package.json:2-10`). It is not the engine manager. | A Rivet Cloud user |
| `@rivetkit/engine-runner` | TypeScript process | An Actor-host process. It connects to an engine; it does not start one (`rivetkit-typescript/packages/engine-runner/src/mod.ts:22,752`). In this commit it is test-only (section 4.1). | The test runner |

`rivetkit-core` uses `rivetkit-engine-process` only under its native runtime feature (`rivetkit-rust/packages/rivetkit-core/Cargo.toml:20,70`). The TypeScript side obtains the path from `@rivetkit/engine-cli`. This map does not trace the TypeScript process launcher.

`rivet-version-management` illustrates the protocol gate. The source has two constants at the top of the module.

```rust
// engine/packages/version-management/src/lib.rs:10-11
const PROTOCOL_VERSION_ENTRY_EXPIRED_THRESHOLD_MS: i64 = 30000;
const PROTOCOL_VERSION_CHECK_INTERVAL: Duration = Duration::from_secs(15);
```

The gate is a state change in the durable store, not a wait for a release. Each protocol subspace holds entries. An entry records one live process, its protocol version, and the last time that process refreshed the entry. The gate chooses the oldest live version across all processes. It runs in this order:

1. `check_engine_admission` opens one serializable transaction. It reads `EngineVersionKey` and writes the current engine semver (`engine/packages/version-management/src/lib.rs:65-99`).
2. If the current semver is older than the stored value, the process bails unless configuration allows rollback (`engine/packages/version-management/src/lib.rs:71-88`).
3. `update_protocol_versions` reads each protocol subspace in ascending version order (`engine/packages/version-management/src/lib.rs:112-124`).
4. It deletes an entry whose timestamp is older than 30,000 milliseconds (`engine/packages/version-management/src/lib.rs:127-139`).
5. It stops at the first live entry. If the compiled version is older, it bails (`engine/packages/version-management/src/lib.rs:141-156`).
6. It sets the override to that live version (`engine/packages/version-management/src/lib.rs:158`). The process now speaks the oldest live version, not its compiled version.
7. It writes its own compiled version and the current timestamp (`engine/packages/version-management/src/lib.rs:162-167`).
8. The `version_management` service repeats steps 3 through 7 every 15 seconds and calls `Config::set_protocols` (`engine/packages/version-management/src/lib.rs:14-51`).

Step 6 is the "hold back" behavior that the test names `holds_back_to_the_oldest_live_version` (`engine/packages/version-management/tests/inline/protocol_sync.rs:143`). The process starts immediately and downgrades its protocol. It does not wait. When the older processes stop refreshing their entries, the 30,000-millisecond threshold expires them. The next 15-second check then raises the override to the compiled version. Document [02 The control plane](02-control-plane-architecture.md) covers the startup order.

### 3.9 Epoxy replication crates (2)

| Crate | Path | Purpose | Main dependency |
|-------|------|---------|-----------------|
| `epoxy` | `engine/packages/epoxy` | Implements the Epoxy replica, coordinator, backfill, HTTP routes, keys, metrics, and protocol version handling. † | `gas` |
| `epoxy-protocol` | `engine/sdks/rust/epoxy-protocol` | Holds the generated, versioned Epoxy wire protocol types. † | external `vbare` |

The `epoxy` crate registers three workflows. `engine/packages/epoxy/src/lib.rs:16-24` lists `backfill::Workflow`, `coordinator::Workflow`, and `replica::Workflow`. The crate depends on `epoxy-protocol` for the wire types. It depends on `gas` for the workflow runtime.

Epoxy is a geo-distributed, strongly consistent key-value store (`engine/packages/epoxy/README.md:3`). Its consensus protocol is single-decree Fast Paxos, run independently per key (`engine/packages/epoxy/README.md:3`). Single-decree means that the protocol agrees on one value per key, not a sequence of values. The fast path skips Phase 1 (Prepare) and sends Phase 2 (Accept) directly. Prepare asks replicas to promise a ballot, and Accept asks them to accept a value at that ballot (`engine/packages/epoxy/README.md:29-35`). A ballot is the proposal number that orders competing proposals. A key with prior in-flight state, meaning an earlier proposal that has not committed and may have an accepted value, falls back to the two-phase path (`engine/packages/epoxy/README.md:50`). EPaxos is a different protocol and only the ancestry of the old `PreAccept` name (`engine/packages/epoxy/README.md:27,42`).

The replicated unit is a keyed byte value. Each replica keeps its keys in the subspace `(RIVET, EPOXY_V2, REPLICA, replica_id)` (`engine/packages/epoxy/src/keys/mod.rs:13-15`). A writer calls `epoxy_propose` (`engine/packages/epoxy/src/ops/propose.rs:321`). The operation reads the cluster configuration and resolves the active quorum (`engine/packages/epoxy/src/ops/propose.rs:327-340`). It then selects a ballot and sends the proposal. The command is a set or a check-and-set (`engine/packages/epoxy/src/ops/propose.rs:29-33`).

A key is immutable by default: a writer sets it once, and the store offers an opt-in mutable overwrite mode for workloads that need updates (`engine/packages/epoxy/README.md:3`). A fresh key commits in 1 RTT because it skips the Prepare phase. A key with prior in-flight state uses Prepare and Accept, or 2 RTT (`engine/packages/epoxy/README.md:3`). The coordinator workflow drives a proposal. The replica workflow keeps replica state and the changelog. A committed immutable key can be read locally by any replica that holds it, with no network call (`engine/packages/epoxy/README.md:9`).

Two engine call sites use Epoxy. Actor key reservation stores the datacenter that owns an Actor key, so the key resolves to one home (`docs-internal/engine/ACTOR_KEY_RESERVATION.md:3-9`). The reservation sends a check-and-set proposal (`engine/packages/pegboard/src/workflows/actor2/keys.rs:221-230`). The JSON Web Token (JWT) key ring also sends a check-and-set proposal (`engine/packages/auth-jwt/src/ops/key_ring/compare_and_set.rs:63-69`). Document [08 Epoxy multi-region](08-epoxy-multi-region.md) covers the ballot and commit rules in full.

### 3.10 Gasoline durable execution crates (3)

| Crate | Path | Purpose | Main dependency |
|-------|------|---------|-----------------|
| `gasoline` | `engine/packages/gasoline` | The durable workflow engine: activities, builders, contexts, databases, history, listeners, messages, operations, registries, signals, stubs, workers, and workflows. † | `universaldb` |
| `gasoline-macros` | `engine/packages/gasoline-macros` | Provides the procedural macros that generate workflow and activity code. † | external `syn` |
| `gasoline-runtime` | `engine/packages/gasoline-runtime` | Registers and runs the workflows that the engine hosts. † | `gas` |

The module list in `engine/packages/gasoline/src/lib.rs:1-19` is the best summary of the crate. The modules `activity`, `history`, `operation`, `signal`, and `workflow` are public, among others. The modules `error`, `executable`, `stub`, and `worker` are private. The private modules hold the machinery that the public builder uses.

```rust
// engine/packages/gasoline/src/lib.rs:1-19
pub mod activity;
pub mod builder;
pub mod ctx;
pub mod db;
mod error;
mod executable;
pub mod history;
pub mod listen;
pub mod message;
pub mod metrics;
pub mod operation;
pub mod prelude;
pub mod pubsub_subjects;
pub mod registry;
pub mod signal;
mod stub;
pub mod utils;
mod worker;
pub mod workflow;
```

The durable-execution mechanism has five parts.

1. A workflow is a typed function with a name, an input, and an output (`engine/packages/gasoline/src/workflow.rs:11-19`).
2. An activity is a step with its own input and output (`engine/packages/gasoline/src/activity.rs:10-22`). The workflow context runs it and records the result (`engine/packages/gasoline/src/ctx/workflow.rs:562-600`). An activity may do non-deterministic work, so replay uses the recorded result and does not run it again.
3. A history record is an `Event`. It holds a coordinate, a version, and one `EventData` variant such as `Activity`, `Sleep`, or `SignalSend` (`engine/packages/gasoline/src/history/event.rs:10-19,42-53`). A coordinate is a dot-separated position in the event tree (`engine/packages/gasoline/src/history/location.rs:10-16`). The version is the workflow version that produced the event; `WorkflowCtx::v` selects a version (`engine/packages/gasoline/src/ctx/workflow.rs:134-137`). Replay compares the recorded step version with the context version. If the step version is lower, the comparison returns `WorkflowError::HistoryDiverged` (`engine/packages/gasoline/src/ctx/workflow.rs:139-151`).
4. Replay walks the history with a cursor. If an activity already has a recorded output, the context returns that output and does not run the activity again (`engine/packages/gasoline/src/ctx/workflow.rs:574-586`).
5. A registry maps workflow names to workflow types (`engine/packages/gasoline/src/registry.rs:14-17,51`). `merge` rejects a duplicate name (`engine/packages/gasoline/src/registry.rs:36-49`).

A signal is a named message that enters a running workflow (`engine/packages/gasoline/src/signal.rs:5-19`). The private `worker` module runs the workflows that the registry holds (`engine/packages/gasoline/src/worker.rs`). This module is inside the `gasoline` crate. It is not the `rivet-workflow-worker` process service in section 3.19. The division between public and private modules is a boundary. A user writes a workflow through the `builder` module, the `workflow` module, and the `activity` module. Replay uses `history`. The `stub` and `worker` modules stay private. This map reads that choice as room for the runtime to change without a public break. The source does not state the reason. Document [05 Gasoline durable execution](05-gasoline-durable-execution.md) covers the replay algorithm.

### 3.11 Guard proxy crates (3)

| Crate | Path | Purpose | Main dependency |
|-------|------|---------|-----------------|
| `rivet-guard` | `engine/packages/guard` | The reverse proxy that terminates Transport Layer Security (TLS), routes HTTP and WebSocket requests, caches routing decisions, and wakes Actors. † | `pegboard` |
| `rivet-guard-core` | `engine/packages/guard-core` | Provides the reusable proxy service, route table, certificate resolver, WebSocket handle, task group, and server loop. † | external `hyper` |
| `rivet-outbound-guard` | `engine/packages/outbound-guard` | Applies the outbound network policy to requests that leave the process. † | `rivet-config` |

`rivet-guard` has a large dependency fan-out. `engine/packages/guard/Cargo.toml:13-70` lists the workspace dependencies. The list includes `gas`, `namespace`, the pegboard crates, all three gateway crates, the API crates, the authentication crates, and the configuration and metrics crates. It also includes the two storage crates `universaldb` and `universalpubsub` (`:69-70`). The list contains all three gateway versions at the same time.

```toml
# engine/packages/guard/Cargo.toml:31-46
namespace.workspace = true
once_cell.workspace = true
pegboard-envoy.workspace = true
pegboard-gateway.workspace = true
pegboard-gateway2.workspace = true
pegboard-gateway3.workspace = true
pegboard-runner.workspace = true
pegboard.workspace = true
regex.workspace = true
reqwest.workspace = true
rivet-api-types.workspace = true
rivet-api-util.workspace = true
rivet-api-builder.workspace = true
rivet-api-public.workspace = true
rivet-auth.workspace = true
rivet-auth-jwt = { workspace = true, features = ["service"] }
```

The three gateway crates are not dead code. Guard depends on all three, and each serves a different path. The first gateway serves the path-based legacy route, and Guard selects it when the Actor record version is 1 (`engine/packages/guard/src/routing/pegboard_gateway/mod.rs:469-505,967-977`). The other two gateways serve version 2 Actors.

The envoy path calls `select_gateway` with the Actor envoy protocol version (`engine/packages/guard/src/routing/pegboard_gateway/mod.rs:748-754`). Gateway3 is used only when the selector returns `Decision::SampledIn` (`engine/packages/guard/src/routing/pegboard_gateway/gateway3_selector.rs:71-74`). The gateway3 mode is `Off`, `Opportunistic`, or `On` (`engine/packages/config/src/config/features.rs:47-54`). The percentage is the configuration key `features.guard_gateway_v3.percentage` (`engine/packages/config/src/config/features.rs:37-44`). The selector applies five tests in this order (`engine/packages/guard/src/routing/pegboard_gateway/gateway3_selector.rs:146-164`):

1. The gateway3 mode is not `Off`.
2. The request is not a WebSocket.
3. The envoy protocol version is present and is 7 through 8.
4. In `Opportunistic` mode, the request is Server-Sent Events (SSE) or has an unknown length. In `On` mode this test does not apply.
5. The namespace and Actor hash falls inside the configured percentage.

The sampling rule hashes the constant domain, the namespace identifier, a zero byte, and the Actor identifier with xxh3-128. The result modulo 10,000 is compared with the percentage times 100 (`engine/packages/guard/src/routing/pegboard_gateway/gateway3_selector.rs:231-246`). A WebSocket gets `Decision::WebSocket` (`engine/packages/guard/src/routing/pegboard_gateway/gateway3_selector.rs:148-149`). That decision is not `SampledIn`, so a WebSocket stays on gateway2. This rule keeps WebSocket hibernation on gateway2, because gateway2 has the hibernation task and gateway3 does not (section 3.13). Hibernation keeps the client WebSocket open while the Actor is unloaded. The task watches the client socket and wakes the Actor when a message arrives (`engine/packages/pegboard-gateway2/src/hibernation_task.rs:22-33,105-118`). The guard builds gateway3 or gateway2 at `engine/packages/guard/src/routing/pegboard_gateway/mod.rs:762-805`. The path-based legacy route builds the first gateway at `engine/packages/guard/src/routing/pegboard_gateway/mod.rs:967-977`. The source does not state a removal date for the older two, so this map treats all three as live in this commit.

Guard wakes a sleeping Actor with a workflow signal, not with a direct process start.

1. Guard reads the Actor route and sees the `sleeping` flag (`engine/packages/guard/src/routing/pegboard_gateway/mod.rs:531-538`).
2. It sends `actor2::Wake {}` to the Actor workflow (`engine/packages/guard/src/routing/pegboard_gateway/mod.rs:540-560`).
3. The legacy path sends `actor::Wake` with `AllocationOverride::DontSleep` instead (`engine/packages/guard/src/routing/pegboard_gateway/mod.rs:827-845`). Allocation is the act of placing an Actor on a host. `CrashPolicy` is the per-Actor rule that selects `Restart`, `Sleep`, or `Destroy` after a process exit (`engine/packages/types/src/actors.rs:42-47`). `DontSleep` forces an Actor with `CrashPolicy::Sleep` to pend instead of sleep (`engine/packages/pegboard/src/workflows/actor/runtime.rs:621-628`). To pend means that the Actor waits for an allocation, and the pending state records a timestamp (`engine/packages/pegboard/src/workflows/actor/runtime.rs:229-233`).
4. Guard then waits on the `Ready`, `Stopped`, `Failed`, and `DestroyStarted` subscriptions (`engine/packages/guard/src/routing/pegboard_gateway/mod.rs:526-529`). These are workflow event subscriptions that the Actor workflow emits.
5. If the Actor stops during the wait, Guard sends the wake signal again, up to eight times (`engine/packages/guard/src/routing/pegboard_gateway/mod.rs:626-634`).
6. The signal has a timeout from `guard().route_pegboard_wake_signal_timeout()` (`engine/packages/guard/src/routing/pegboard_gateway/mod.rs:540-560`).

The Actor workflow receives the signal. It schedules the Actor onto an envoy, and Guard waits for `Ready`. The legacy actor workflow allocates to a runner instead (`engine/packages/pegboard/src/workflows/actor/mod.rs:296,445`). Document [09 Guard routing](09-guard-routing.md) covers the full route table.

`rivet-guard-core` holds the generic proxy. Its re-export list at `engine/packages/guard-core/src/lib.rs:15-27` is the public surface. The proxy group is `CertResolverFn`, `CustomServeTrait`, `ProxyService`, `ProxyState`, `ResponseBody`, `ResponseBodyTerminal`, and `WebSocketHandle`. The route group is `CacheKeyFn`, `RouteConfig`, `RouteTarget`, `RoutingFn`, and `RoutingOutput`. The last group is `run_server`, `EndpointType`, and `GameGuardProtocol`. `EndpointType` selects hostname or path routing, and `GameGuardProtocol` selects HTTP or WebSocket (`engine/packages/guard-core/src/types.rs:3-19`). `ResponseBodyTerminal` is the terminal response-body type (`engine/packages/guard-core/src/response_body.rs:47`). `ProxyService` is the service value, and `ProxyState` is its shared state (`engine/packages/guard-core/src/proxy_service.rs:71,353`). The `GameGuardProtocol` type name is exact. It is not a typo for a generic guard protocol.

### 3.12 Metrics crates (2)

| Crate | Path | Purpose | Main dependency |
|-------|------|---------|-----------------|
| `rivet-metrics` | `engine/packages/metrics` | Provides the Prometheus metric helpers for Rivet crates. The manifest description is "Prometheus metric helpers for Rivet crates". | external `prometheus` |
| `rivet-metrics-server` | `engine/packages/metrics-server` | Serves metrics and OpenTelemetry exporters, including console, OTLP, and tracing providers. † | `rivet-config` |

`rivet-metrics` is a common dependency in the tree. Crates as different as `api-builder`, `gasoline`, `pegboard`, `tracing-utils`, and `universaldb` depend on it. A shared crate keeps the metric names and histogram layouts in one place. The source does not state that this is the reason for the shared crate. Document [14 Observability and operations](14-observability-and-operations.md) covers the metric names.

### 3.13 Pegboard scheduling crates (9)

| Crate | Path | Purpose | Main dependency |
|-------|------|---------|-----------------|
| `pegboard` | `engine/packages/pegboard` | Registers the Actor, runner, runner pool, serverless, and metrics workflows that schedule work. † | `gas` |
| `pegboard-envoy` | `engine/packages/pegboard-envoy` | Implements the envoy-side tasks: Actor lifecycle, Actor key-value state, SQLite page and remote SQLite tasks, connection control, hibernating requests, and ping. † | `gas` |
| `pegboard-gateway` | `engine/packages/pegboard-gateway` | The first gateway between an envoy tunnel and the engine, with keepalive, ping, metrics, and WebSocket tunnel tasks. † | `gas` |
| `pegboard-gateway2` | `engine/packages/pegboard-gateway2` | The second gateway; it adds a hibernation task to the same set. † | `gas` |
| `pegboard-gateway3` | `engine/packages/pegboard-gateway3` | The third gateway; it adds HTTP stream support and request metrics. † | `gas` |
| `pegboard-outbound` | `engine/packages/pegboard-outbound` | Runs outbound calls from the engine to envoys and records metrics. † | `pegboard` |
| `pegboard-runner` | `engine/packages/pegboard-runner` | Implements the runner-side connection, event demultiplexer, tunnel tasks, and ping. † | `gas` |
| `rivet-runner-protocol` | `engine/packages/runner-protocol` | Defines the versioned runner protocol and its compatibility helpers. † | `gas` |
| `rivet-envoy-client` | `engine/sdks/rust/envoy-client` | The client transport for envoy Actor hosts: connections, commands, events, HTTP, key-value, SQLite, tunnel, and WebSocket. The manifest description is "Envoy client transport for Rivet actor hosts". | `rivet-envoy-protocol` |

A runner pool is the set of serverless runners for one runner configuration. The pool workflow reads the desired runner state and starts the error-tracker and metadata-poller sidecar workflows (`engine/packages/pegboard/src/workflows/runner_pool.rs:27-45`).

The `pegboard` registry is the scheduling surface. `engine/packages/pegboard/src/lib.rs:14-34` registers 16 workflows. The next block gives the full list. The list covers the Actor workflow, two runner workflows, the runner pool workflows, the serverless workflows, the metrics aggregator, and several backfills.

```rust
// engine/packages/pegboard/src/lib.rs:14-34
pub fn registry() -> WorkflowResult<Registry> {
	use workflows::*;

	let mut registry = Registry::new();
	registry.register_workflow::<actor::Workflow>()?;
	registry.register_workflow::<actor::metrics::Workflow>()?;
	registry.register_workflow::<actor2::Workflow>()?;
	// registry.register_workflow::<actor2::metrics::Workflow>()?;
	registry.register_workflow::<actor_migration_fix_backfill::Workflow>()?;
	registry.register_workflow::<runner::Workflow>()?;
	registry.register_workflow::<runner2::Workflow>()?;
	registry.register_workflow::<runner_pool::Workflow>()?;
	registry.register_workflow::<runner_pool_error_tracker::Workflow>()?;
	registry.register_workflow::<runner_pool_metadata_poller::Workflow>()?;
	registry.register_workflow::<runner_pool_backfill::Workflow>()?;
	registry.register_workflow::<runner_pool2_backfill::Workflow>()?;
	registry.register_workflow::<serverless::receiver::Workflow>()?;
	registry.register_workflow::<serverless::conn::Workflow>()?;
	registry.register_workflow::<serverless::backfill::Workflow>()?;
	registry.register_workflow::<metrics_aggregator::Workflow>()?;
	registry.register_workflow::<actor_runner_name_selector_backfill::Workflow>()?;
```

The commented line is in the source. `actor2::metrics::Workflow` is not registered in this commit. The registry does not register it either. The list has two pairs that look duplicated. `actor`, `actor2`, `runner`, and `runner2` are workflow modules under `engine/packages/pegboard/src/workflows/`, not processes or gateway versions.

- `actor` and `actor2` are a migration pair, not a permanent split. New Actors start `actor2` directly (`engine/packages/pegboard/src/ops/actor/create.rs:113`). The old `actor` workflow stays registered for existing Actors. It dispatches `actor2` with `from_v1: true` when its lifecycle reports `migrate_to_v2` (`engine/packages/pegboard/src/workflows/actor/mod.rs:864-876`). `actor2` runs the SQLite v1-to-v2 migration once for that case (`engine/packages/pegboard/src/workflows/actor2/mod.rs:32,169-171`). The migration converts an existing Actor's SQLite database from the v1 format to the v2 format.
- `runner` and `runner2` are a protocol compatibility pair. The runner connection calls `is_mk2(protocol_version)` (`engine/packages/pegboard-runner/src/conn.rs:144`). A version greater than 3 starts `runner2` (`engine/packages/pegboard-runner/src/conn.rs:145-157`). A version of 3 or lower starts the legacy `runner` (`engine/packages/pegboard-runner/src/conn.rs:164-176`). The source comment calls `runner` legacy (`engine/packages/pegboard/src/workflows/runner.rs:136-137`).

The runner protocol has two generations. `engine/packages/runner-protocol/src/lib.rs:7-16` re-exports `generated::v3` as the latest first-generation protocol and `generated::v7` as `mk2`. The constants are `PROTOCOL_MK1_VERSION = 3` and `PROTOCOL_MK2_VERSION = 7`. The function `is_mk2(protocol_version)` returns true when the version is greater than 3.

```rust
// engine/packages/runner-protocol/src/lib.rs:7-16
// Re-export latest
pub use generated::v3::*;
pub use generated::v7 as mk2;

pub const PROTOCOL_MK1_VERSION: u16 = 3;
pub const PROTOCOL_MK2_VERSION: u16 = 7;

pub fn is_mk2(protocol_version: u16) -> bool {
	protocol_version > PROTOCOL_MK1_VERSION
}
```

The two paths exist in this commit. The legacy runner path uses `pegboard-runner` and `rivet-runner-protocol`. Its `engine/packages/pegboard-runner/src/ws_to_tunnel_task.rs:71-72` calls `is_mk2` and dispatches to `handle_message_mk2` when the connection version is greater than 3. The runner path therefore accepts both protocol generations. The newer envoy path uses `pegboard-envoy` and `rivet-envoy-protocol`, currently version 8. A statement about scheduling must say which path it describes. Document [04 Pegboard scheduling](04-pegboard-scheduling.md) does this.

The repository has several unrelated version numbers. This table puts them in one place.

| Protocol | Current version | Compatibility rule | Which component speaks it |
|----------|-----------------|--------------------|---------------------------|
| Runner, first generation | 3 (`engine/packages/runner-protocol/src/lib.rs:11`) | A version of 3 or lower uses the legacy handlers. | `pegboard-runner` and the legacy runner process |
| Runner, second generation | 7 (`engine/packages/runner-protocol/src/lib.rs:12`) | `is_mk2(v)` is true for every version greater than 3 (`engine/packages/runner-protocol/src/lib.rs:14-16`). | `pegboard-runner`, `runner2`, and `@rivetkit/engine-runner` (`rivetkit-typescript/packages/engine-runner/src/mod.ts:22`) |
| Envoy | 8 (`engine/sdks/rust/envoy-protocol/src/lib.rs:6`) | The serverless start payload requires the exact version, then retries once at version 7 (`engine/sdks/rust/envoy-client/src/handle.rs:777-797`). | `rivet-envoy-client`, `pegboard-envoy`, and the envoy host |
| Client | 4 (`rivetkit-rust/packages/client-protocol/src/lib.rs:8`) | The version is embedded in the BARE (Binary Application Record Encoding) payload (`rivetkit-rust/packages/client/src/protocol/codec.rs:43`). | `rivetkit-client` and Actor gateway clients |
| Actor persist | 4 (`rivetkit-rust/packages/actor-persist/src/lib.rs:7`) | The codec embeds the version. The source states no other compatibility rule. | The Actor host persistence layer |
| Inspector | 6 (`rivetkit-rust/packages/inspector-protocol/src/lib.rs:7`) | The codec embeds the version. The source states no other compatibility rule. | Inspector clients and the Actor host |
| UniversalDB commit | 2 (`engine/sdks/rust/universaldb-commit/src/lib.rs:6`) | The codec embeds the version. The source states no other compatibility rule. | UniversalDB and the engine |
| Universal PubSub | 3 (`engine/sdks/rust/ups-protocol/src/lib.rs:5`) | The codec embeds the version. The source states no other compatibility rule. | `universalpubsub` |
| Epoxy | 4 (`engine/sdks/rust/epoxy-protocol/src/lib.rs:2,7`) | Version 4 is required for read-state (`engine/sdks/rust/epoxy-protocol/src/lib.rs:7`). | Epoxy replicas |
| Depot | 1 (`engine/sdks/schemas/depot/`) | The codec embeds the version. The source states no other compatibility rule. | `depot-client` and `depot` |

Two terms in the table need a note. The serverless start payload is the first message that a serverless envoy sends. The first two bytes are a little-endian version, and that version must equal the current protocol version, 8 (`engine/sdks/rust/envoy-client/src/handle.rs:773-783`). If decoding at version 8 fails, the client retries the body at version 7 (`:785-795`). The source does not state why the fallback is 7. A read-state request asks a replica for its committed value, any accepted but uncommitted value, and its promised ballot, in the transaction that installs a promise (`engine/packages/epoxy/src/replica/messages/read_state.rs:7-33`). Version 4 adds the request, and conversion to version 3 rejects it (`engine/sdks/rust/epoxy-protocol/src/convert.rs:752-754`).

The 4-through-6 range needs an explanation. `is_mk2` is a threshold test, not an equality test (`engine/packages/runner-protocol/src/lib.rs:14-16`). Versions 4, 5, and 6 return true, although the current constant is 7. The schema directory holds `v1.bare` through `v7.bare` (`engine/sdks/schemas/runner-protocol/`). These are earlier schema versions of the second generation. A connection with version 4, 5, or 6 therefore starts `runner2`, not the legacy `runner` (`engine/packages/pegboard-runner/src/conn.rs:144-176`).

### 3.14 Runtime and shared infrastructure crates (7)

| Crate | Path | Purpose | Main dependency |
|-------|------|---------|-----------------|
| `rivet-runtime` | `engine/packages/runtime` | Configures the asynchronous runtime, tracing, metrics endpoints, and termination signals. † | `rivet-metrics` |
| `rivet-pools` | `engine/packages/pools` | Holds the shared connection pools for PostgreSQL, ClickHouse, NATS, and HTTP. † | `rivet-config` |
| `rivet-perf` | `engine/packages/perf` | Instruments performance with Prometheus and tracing macros. † | external `prometheus` |
| `rivet-tracing-utils` | `engine/packages/tracing-utils` | Adds metric fields to tracing spans. † | `rivet-metrics` |
| `rivet-error` | `engine/packages/error` | Provides structured error types and their serialization. The manifest description is "Structured error helpers for Rivet". | `rivet-error-macros` |
| `rivet-error-macros` | `engine/packages/error-macros` | Provides the derive macros for structured errors. The manifest description is "Derive macros for Rivet structured errors". | external `syn` |
| `rivet-actor-runtime-socket-protocol` | `engine/sdks/rust/actor-runtime-socket-protocol` | Holds versioned protocol types for the experimental Actor Runtime Socket. The manifest description is "Versioned protocol for the experimental Actor Runtime Socket". | external `vbare` |

`rivet-pools` holds long-lived client objects for external systems. It depends on `universaldb` and `universalpubsub` (`engine/packages/pools/Cargo.toml:32-33`), and it provides the NATS, ClickHouse, and PostgreSQL handles that the workflows use. The PostgreSQL and NATS pools serve the UniversalDB PostgreSQL driver and the Universal PubSub NATS driver. The crate is a boundary: a workflow asks the pools for a handle and does not open a connection itself.

### 3.15 Storage and wire protocol crates (5)

| Crate | Path | Purpose | Main dependency |
|-------|------|---------|-----------------|
| `universaldb` | `engine/packages/universaldb` | A transactional key-value layer with subspaces, key selectors, range options, throttling, retries, and pluggable drivers. † | external `foundationdb-tuple` |
| `universalpubsub` | `engine/packages/universalpubsub` | A publish-subscribe layer with drivers, chunking, subjects, and metrics. † | external `async-nats` |
| `rivet-universaldb-commit` | `engine/sdks/rust/universaldb-commit` | Holds the generated, versioned commit types for UniversalDB. † | external `vbare` |
| `rivet-ups-protocol` | `engine/sdks/rust/ups-protocol` | Holds the generated, versioned Universal PubSub protocol types. † | external `vbare` |
| `rivet-envoy-protocol` | `engine/sdks/rust/envoy-protocol` | Holds the generated, versioned Envoy protocol types, currently version 8, plus the HTTP stream window constant. The manifest description is "Versioned Envoy protocol types for Rivet actor hosts". | external `vbare` |

The storage stack needs one statement of where bytes live.

UniversalDB is a transactional key-value interface with pluggable drivers (`engine/packages/universaldb/src/driver/mod.rs:25-31`). The tree has two drivers: PostgreSQL and RocksDB (`engine/packages/universaldb/src/driver/mod.rs:14-18`). The configuration type `Database` selects one (`engine/packages/config/src/config/db/mod.rs:12-15`). The default is `FileSystem` (`engine/packages/config/src/config/db/mod.rs:17-19`). `FileSystem` is a configuration variant, not a third driver. It selects the RocksDB driver with the path `./data/db` (`engine/packages/config/src/config/db/mod.rs:29-33`). The pool maps `FileSystem` to `RocksDbDatabaseDriver` (`engine/packages/pools/src/db/udb.rs:57-60`). It maps `Postgres` to `PostgresDatabaseDriver` (`engine/packages/pools/src/db/udb.rs:49-55`).

FoundationDB is not a driver in this tree. The crate uses the `foundationdb-tuple` encoding and copies the FoundationDB API shape (`engine/packages/universaldb/src/lib.rs:19-28`). The phrase "FoundationDB transaction limit" in the depot comment describes the model that the API follows, not a live FoundationDB cluster. Depot writes its pages through UniversalDB. The pages therefore live in RocksDB or PostgreSQL. The bucket cold tier is Enterprise-only (`docs-internal/engine/oss-to-ee-upgrade.md:9-10`).

`universaldb` is the narrow waist of the storage layer. It re-exports `Database`, `DatabaseDriverHandle`, `KeySelector`, `RangeOption`, the throttle types, `RetryableTransaction`, `Transaction`, `Subspace`, `calculate_tx_retry_backoff`, and `foundationdb_tuple as tuple`. The last line re-exports the FoundationDB tuple type as `tuple`. The source comment on that line says `Re-export FDB types`.

```rust
// engine/packages/universaldb/src/lib.rs:19-28
pub use database::Database;
pub use driver::DatabaseDriverHandle;
pub use key_selector::KeySelector;
pub use range_option::RangeOption;
pub use throttle::{ThrottleCharge, ThrottleClass, ThrottleConfig, ThrottleDecision, ThrottleKind};
pub use transaction::{RetryableTransaction, Transaction};
pub use utils::{Subspace, calculate_tx_retry_backoff};

// Re-export FDB types
pub use foundationdb_tuple as tuple;
```

The protocol crates follow one versioning rule. Each generated module is named `vN`, the crate re-exports the latest `vN`, and a `PROTOCOL_VERSION` constant names the current version. `rivet-envoy-protocol` shows the same rule as the runner protocol. The crate re-exports `generated::v8::*` and a `PROTOCOL_VERSION` constant. It also defines the initial HTTP stream window.

```rust
// engine/sdks/rust/envoy-protocol/src/lib.rs:5-13
// Re-export latest
pub use generated::v8::*;

pub use generated::PROTOCOL_VERSION;

/// Initial unacknowledged body bytes allowed in either HTTP stream direction.
/// Receivers return credit with cumulative window-update messages as the body
/// is consumed by the next layer.
pub const HTTP_STREAM_INITIAL_WINDOW_BYTES: u64 = 1024 * 1024;
```

The window is 1,048,576 bytes, which is one mebibyte. A credit is a byte budget that the receiver grants to the sender. The sender stops when the credit is exhausted. A window-update message adds credit cumulatively. The constant is part of the protocol crate, not part of the proxy.

### 3.16 Telemetry and log crates (2)

| Crate | Path | Purpose | Main dependency |
|-------|------|---------|-----------------|
| `rivet-telemetry` | `engine/packages/telemetry` | Initializes Sentry and process telemetry from configuration. † | `rivet-config` |
| `rivet-logs` | `engine/packages/logs` | Configures tracing subscribers and log formatting for Unix and Windows. † | external `tracing-subscriber` |

The split is small but real. `rivet-telemetry` sends events to an external service. `rivet-logs` formats local output. A test that wants quiet logs can depend on `rivet-logs` without the telemetry client.

Six initialization steps run in this order. Four crates own most of them.

1. `rivet_runtime::run` builds the Tokio runtime (`engine/packages/runtime/src/lib.rs:25-29`).
2. Inside that runtime, `init_otel_providers` reads `RIVET_OTEL_ENABLED` and builds the OpenTelemetry tracer provider (`engine/packages/runtime/src/lib.rs:35-36`; `engine/packages/metrics-server/src/providers.rs:112-123`).
3. `init_tracing_subscriber` installs the tracing subscriber and adds the OpenTelemetry layer (`engine/packages/runtime/src/lib.rs:37`; `engine/packages/runtime/src/traces.rs:37-60`).
4. `main_inner` loads the configuration, then `rivet_telemetry::init` starts Sentry (`engine/packages/engine/src/main.rs:37-46`; `engine/packages/telemetry/src/lib.rs:7-23`).
5. The start command redirects logs to files when the configuration asks for it (`engine/packages/engine/src/commands/start.rs:27-31`; `engine/packages/logs/src/unix.rs:27-31`).
6. The service manager always adds a `metrics` service of kind `Core`. That service runs `rivet_metrics_server::run_standalone` (`engine/packages/service-manager/src/lib.rs:147-152`; `engine/packages/metrics-server/src/server.rs:10-40`). This entry is not in the `run_config.rs` list, because the manager adds it.

`rivet-metrics-server` owns the OpenTelemetry provider and the metrics endpoint. `rivet-telemetry` owns Sentry. `rivet-logs` owns file redirection. `rivet-runtime` owns the Tokio runtime and the tracing subscriber.

### 3.17 Test crates (4)

| Crate | Path | Purpose | Main dependency |
|-------|------|---------|-----------------|
| `rivet-test-deps` | `engine/packages/test-deps` | Starts the external dependencies that tests need and exposes them through a datacenter helper. † | `rivet-config` |
| `rivet-test-deps-docker` | `engine/packages/test-deps-docker` | Starts test databases, publish-subscribe services, and Toxiproxy in containers. † | `rivet-config` |
| `test-snapshot-gen` | `engine/packages/test-snapshot-gen` | Generates test snapshots with axum, epoxy, pegboard, and a temporary SQLite database. † | `pegboard` |
| `rivet-test-envoy` | `engine/sdks/rust/test-envoy` | A test envoy process and harness with programmable behaviors. The manifest description is "Rust test envoy process and harness for pegboard actor testing". | `rivet-envoy-protocol` |

Two helper crates need a definition. Toxiproxy is a network fault-injection proxy. It sits between a test and a service and adds latency or failures (`engine/packages/test-deps-docker/src/toxiproxy.rs:93`). A test snapshot is a copy of a UniversalDB state. The generator `test-snapshot-gen` starts a multi-replica UniversalDB test cluster, writes state through the normal interfaces, and checkpoints each replica's RocksDB into a snapshot directory (`engine/packages/test-snapshot-gen/src/main.rs:10-14`). The test cluster stores each replica in RocksDB, so a snapshot is a directory. The source does not compare this choice with the PostgreSQL driver. Integration and migration tests load that directory (`engine/packages/test-snapshot-gen/src/lib.rs:19-24`).

The test crates are workspace members, not dev-dependencies of one crate. More than one crate uses them. This map reads that use as the reason for the separate crates. The source does not state the reason. `gasoline` lists `rivet-test-deps` in its normal dependencies, which is unusual for production code.

The test files sit beside the crate under test. Rust integration tests are in `<crate>/tests/*.rs`, for example `engine/packages/universaldb/tests/` and `engine/packages/cache/tests/`. Rust unit tests are in `#[cfg(test)]` modules inside `src`, for example `engine/packages/runner-protocol/src/uuid_compat.rs`. TypeScript tests use `*.test.ts` under a package `tests/` or `src/` directory, for example `rivetkit-typescript/packages/workflow-engine/tests/`. This map does not list every test file. This map reports the normal-dependency edge at `engine/packages/gasoline/Cargo.toml` and does not claim it is a mistake. Document [16 Tests, benchmarks, and limits](16-tests-benchmarks-and-limits.md) covers the harness.

### 3.18 Utility crates (4)

| Crate | Path | Purpose | Main dependency |
|-------|------|---------|-----------------|
| `rivet-util` | `engine/packages/util` | Shared helpers for asynchronous counters, billing, checks, durations, formatting, futures, geography, mathematics, metrics, requests, serialization, sizes, sorting, throttling, timestamps, and Uniform Resource Locators (URLs). † | `rivet-config` |
| `rivet-util-id` | `engine/packages/util-id` | Identifier types with feature-gated UniversalDB key support and Utoipa OpenAPI schemas. The `universaldb` feature is on by default (`engine/packages/util-id/Cargo.toml:9-11`), and `utoipa` supplies the schema derive (`:17`). † | external `uuid` |
| `rivet-util-serde` | `engine/packages/util-serde` | Serde helpers shared by Rivet crates. The manifest description is "Serde helpers shared by Rivet crates". | external `serde` |
| `rivet-postgres-util` | `engine/packages/postgres-util` | PostgreSQL connection helpers with TLS roots. † | external `rustls` |

`rivet-util` has 17 public modules: `async_counter`, `billing`, `check`, `duration`, `format`, `future`, `geo`, `math`, `metric`, `metrics`, `req`, `serde`, `size`, `sort`, `throttle`, `timestamp`, and `url`. Two names are close but different. `metric` holds one billing constant, `KV_BILLABLE_CHUNK` (`engine/packages/util/src/metric.rs:1`). `metrics` holds the Prometheus histograms for observation, serialization, and deserialization (`engine/packages/util/src/metrics.rs:1-14`). The module list is the reason the crate is large. It is a set of small helpers, not one mechanism. A change to one helper forces a rebuild of every crate that depends on `rivet-util`, because Rust compiles the whole crate. This is a cost of the grouping. Section 13 discusses it.

### 3.19 Workflow worker crate (1)

| Crate | Path | Purpose | Main dependency |
|-------|------|---------|-----------------|
| `rivet-workflow-worker` | `engine/packages/workflow-worker` | Starts the workflow worker and merges the `pegboard`, `namespace`, `epoxy`, `gasoline-runtime`, `rivet-auth-jwt`, `datacenter`, and `depot` registries into one registry (`engine/packages/workflow-worker/src/lib.rs:14-23`). † | `gasoline-runtime` |

The registry merge is the join point of the durable systems. `engine/packages/workflow-worker/src/lib.rs:14-23` shows the call. Each subsystem contributes its workflows, and the worker runs them in one process.

```rust
// engine/packages/workflow-worker/src/lib.rs:14-23
pub fn registry(config: &rivet_config::Config) -> Result<Registry> {
	pegboard::registry()?
		.merge(namespace::registry()?)?
		.merge(epoxy::registry()?)?
		.merge(gasoline_runtime::registry()?)?
		.merge(rivet_auth_jwt::registry()?)?
		.merge(datacenter::registry()?)?
		.merge(depot::registry(config)?)
		.map_err(Into::into)
}
```

The `depot::registry` call takes the configuration, and the other registries do not. That difference is in the signature. Depot needs the configuration to build its workflow context. The map does not invent a deeper reason.

Four workflow systems appear in this map, plus the process that hosts one of them. They are different layers, not alternatives.

| System | Where it runs | Who uses it | Relation to the others |
|--------|---------------|-------------|------------------------|
| Gasoline (`engine/packages/gasoline`) | Rust, inside the `workflow_worker` service | `pegboard`, `namespace`, `depot`, `rivet-auth-jwt`, `datacenter`, and `epoxy` registries (`engine/packages/workflow-worker/src/lib.rs:14-23`) | The internal durable-execution engine for the control plane. |
| `@rivetkit/workflow-engine` | TypeScript, inside the TypeScript Actor host | The `rivetkit` package (`rivetkit-typescript/packages/rivetkit/package.json:223`; `rivetkit-typescript/packages/rivetkit/src/workflow/driver.ts:8`) | The user-facing TypeScript durable engine. It is separate from Gasoline. |
| `@rivet-dev/workflows` (sibling repository) | TypeScript, inside Actors through RivetKit | Application authors | A workflow package above RivetKit. It re-exports RivetKit and uses its public workflow-host interface (`workflows/README.md:3-30`, in the sibling checkout). |
| `@rivet-dev/workflow-world` (integration) | TypeScript, in a Vercel Workflow SDK application | Application authors on Vercel | An adapter that backs the Vercel Workflow SDK with Rivet Actors (`integrations/workflow-world/package.json:4`). |
| `rivet-workflow-worker` (host process, not an engine) | Rust process service | The engine | The process that merges and runs the Gasoline registries (`engine/packages/workflow-worker/src/lib.rs:14-23`). |

Two systems are easy to confuse. Gasoline is the engine's internal Rust engine. `@rivetkit/workflow-engine` is the SDK's TypeScript engine. A user workflow usually runs on the second; engine workflows run on the first.

### 3.20 Alias names and the dependency edge inventory

The workspace defines five short aliases in `Cargo.toml`. An alias is a dependency key that renames its package with `package = "..."`. The table lists each alias, the package, whether the package is a workspace member, the manifest count, and examples.

| Alias | Package | Kind | Manifests | Examples |
|-------|---------|------|-----------|----------|
| `gas` | `gasoline` | Workspace member | 31 | `pegboard`, `namespace`, `api-public`, `workflow-worker` |
| `util` | `rivet-util` | Workspace member | 0 | Defined at `Cargo.toml:617-619`; no manifest uses it in this commit. |
| `depot-client` | `rivet-depot-client` | Workspace member | 4 | `engine`, `depot-client-embedded`, `pegboard-envoy`, `rivetkit-napi` |
| `depot-client-types` | `rivet-depot-client-types` | Workspace member | 4 | `depot`, `depot-client`, `engine`, `rivetkit-core` |
| `async-nats` | `rivet-async-nats` | External package, version 0.46.0 (`Cargo.toml:185-187`) | 3 | `pools`, `universaldb`, `universalpubsub` |

The `async-nats` row is the one exception. The alias names an external package, not a workspace crate. The workspace pins it at version 0.46.0. Section 3.15 therefore lists it as an external dependency.

The `util` alias has no user in this commit. The workspace defines it, and no manifest names it. The source does not state why it exists. This map reports the definition and does not invent a purpose. A reader should use `rivet-util` to find the crate.

Three dependency keys look like aliases but are not. `epoxy-protocol`, `depot-client-embedded`, and `rivet-ups-protocol` name the same package as the key. They have no `package = "..."` line. A manifest writes `rivet-ups-protocol.workspace = true`; there is no `ups-protocol` key.

The alias rule has a cost. A reader who searches for `gas` in a manifest finds the workflow engine, not a gas utility. A reader who searches for `util` finds a definition with no users, because the crates use `rivet-util` instead. The workspace uses aliases in some manifests. The source does not state one rule for them. Some aliases remove the `rivet-` prefix, and one shortens a name. This map gives both names in the tables.

The observed internal dependency edges need four notes:

| Edge | Kind | Consequence |
|------|------|-------------|
| `rivet-api-peer` -> `pegboard`, `rivet-api-peer` -> `rivet-api-types`, `pegboard` -> `rivet-api-types` | Shared types | The types crate breaks the type-level cycle that `docs-internal/engine/API.md:7` describes. `pegboard` does not list `rivet-api-peer` in this commit. |
| `rivet-config` -> no workspace crate | Leaf | Many crates depend on it, and it depends on none. |
| `rivet-build-meta` -> `rivet-config` | One way | Build metadata flows to the binary and stops there. |
| `gasoline` -> `rivet-test-deps` | Normal dependency | A test helper is in the production graph of the workflow engine. |

The map reports the cycle note as source-stated and the last edge as observed in the manifest. It does not claim that the last edge is intended.

The fan-in count is the number of other workspace manifests that name the crate in a dependency table. The fan-out count is the number of `workspace = true` entries in the crate's own dependency tables, including dev-dependencies. An alias counts under its manifest key, not its package name.

| Crate | Fan-in | Note |
|-------|--------|------|
| `rivet-config` | 40 | The widest fan-in in the workspace. |
| `rivet-metrics` | 25 | `rivet-util` and `rivet-error` follow at 24. |
| `universaldb` | 22 | |
| `pegboard` | 14 | |
| `epoxy` | 10 | |

The Note column is not a dependency list. It gives the next-highest crates or another observation about the row.

| Crate | Fan-out | Note |
|-------|---------|------|
| `rivet-engine` | 84 | The engine manifest has 84 dependency-table entries, including external crates. |
| `rivet-guard` | 58 | |
| `pegboard` | 56 | |
| `api-public` | 35 | |

Section 13 uses these edges to explain the workspace shape.

## 4. The TypeScript workspace

The TypeScript workspace is a pnpm workspace. The root manifest is `package.json`, with name `@rivetkit/engine-workspace`, private, package manager `pnpm@10.13.1`, and Node.js version 20 or later. Turborepo runs the tasks. `turbo.json` defines them. The root `build` script runs two tasks, `build` and `build:embed` (`package.json:10`). The other root scripts run `npx turbo test` and `npx turbo check-types` (`package.json:11,13`).

The workspace is wider than `rivetkit-typescript/packages`. `pnpm-workspace.yaml:9-31` includes a glob named `engine` (line 10), the frontend, the examples, the integrations, the shared TypeScript package, and several script packages. The `engine` directory has no `package.json`, so that glob adds no package. This map covers the 18 packages under `rivetkit-typescript/packages` in section 4.1 and the frontend in section 4.3. The other globs appear in sections 6, 7, and 8.

### 4.1 The 18 packages under `rivetkit-typescript/packages`

| Package | Directory | Version | Purpose | Notes |
|---------|-----------|---------|---------|-------|
| `rivetkit` | `rivetkit` | 2.3.31 | The main SDK. The manifest description is "Lightweight libraries for building stateful actors on edge platforms". | The package name has no scope. |
| `@rivetkit/cli` | `cli` | 2.3.31 | The command line interface for Rivet Cloud. The manifest description is "Rivet Cloud CLI". | — |
| `@rivetkit/cloudflare-workers` | `cloudflare-workers` | 2.3.31 | The Cloudflare Workers integration. The manifest description is "Cloudflare Workers integration for RivetKit actors". | — |
| `@rivetkit/devtools` | `devtools` | 2.2.1 | Development tools for RivetKit. The manifest description is "RivetKit DevTools - A set of development tools for RivetKit". | Private. |
| `@rivetkit/effect` | `effect` | 2.3.31 | The Effect SDK for Actors. The manifest description is "Effect SDK for Rivet Actors". | — |
| `@rivetkit/engine-cli` | `engine-cli` | 2.3.31 | The engine binary distributed as platform-specific npm packages. The manifest description is "Rivet Engine binary distributed as platform-specific npm packages". | — |
| `@rivetkit/engine-runner` | `engine-runner` | 2.3.31 | The TypeScript runner process. The manifest has no description; the dependencies are `@rivetkit/virtual-websocket`, `@rivetkit/engine-runner-protocol`, `uuid`, `pino`, and `ws`. † | The purpose is inferred from the dependencies and the name. |
| `@rivetkit/engine-runner-protocol` | `engine-runner-protocol` | 2.3.31 | The TypeScript codec for the runner protocol. The manifest has no description; the dependency is `@rivetkit/bare-ts`. † | The purpose is inferred from the dependency and the name. |
| `@rivetkit/framework-base` | `framework-base` | 2.3.31 | Base utilities for client integrations. The manifest description is "Base framework utilities for RivetKit client integrations". | — |
| `@rivetkit/mcp-hub` | `mcp-hub` | 2.2.1 | A static documentation server for the Model Context Protocol (MCP). | Private. The README states the purpose. |
| `@rivetkit/next-js` | `next-js` | 2.3.31 | The Next.js integration. The manifest description is "Next.js integration for RivetKit actors and client". | — |
| `@rivetkit/react` | `react` | 2.3.31 | React hooks and components. The manifest description is "React hooks and components for RivetKit client applications". | — |
| `@rivetkit/rivetkit-napi` | `rivetkit-napi` | 2.3.31 | The native Node-API addon. The manifest description is "Native N-API addon for RivetKit providing envoy client and SQLite access". | A Cargo workspace member. |
| `@rivetkit/rivetkit-wasm` | `rivetkit-wasm` | 2.3.31 | The WebAssembly build. The manifest description is "WebAssembly bindings for RivetKit core on edge JavaScript runtimes". | A Cargo workspace member. |
| `@rivetkit/sql-loader` | `sql-loader` | 2.3.31 | The SQL file loader and migration utilities. The manifest description is "SQL file loader and migration utilities for RivetKit". | — |
| `@rivetkit/supabase` | `supabase` | 2.3.31 | The Supabase Edge Functions integration. The manifest description is "Supabase Edge Functions integration for RivetKit actors". | — |
| `@rivetkit/traces` | `traces` | 2.3.31 | Tracing storage and OpenTelemetry Protocol (OTLP) export. The manifest description is "Generic tracing storage and OTLP export for RivetKit". | — |
| `@rivetkit/workflow-engine` | `workflow-engine` | 2.3.31 | The durable workflow engine with reentrant execution. The manifest description is "Durable workflow engine with reentrant execution". | — |

The package count is 18. The directory list from `ls -1` has 18 names, and the manifest scan returns 18 `package.json` files.

Two TypeScript paths host Actors. A reader must distinguish them. Four terms need a definition first.

- A **fetch handler** is an object with a `fetch(request)` method that a hosting platform calls for each request (`rivetkit-typescript/packages/rivetkit/src/registry/index.ts:407-418`).
- **Serverful** means a long-running process that starts an envoy or binds a listener and stays alive. **Serverless** means the platform invokes the fetch handler per request. `registry.start()` chooses the envoy unless `RIVETKIT_RUNTIME_MODE=serverless` (`rivetkit-typescript/packages/rivetkit/src/registry/index.ts:951-981`).
- The **actor envoy** is the long-running TypeScript host process that serves the registry to the engine. `#startEnvoy` starts it (`rivetkit-typescript/packages/rivetkit/src/registry/index.ts:624-627`), and the runtime calls `runtime.serveRegistry` (`:660`).
- The **native runtime** is the host behind the `native-runtime` feature of `rivetkit-core`. It is loaded in Node.js through the Node-API addon. The WebAssembly build uses the separate `wasm-runtime` feature (`rivetkit-rust/packages/rivetkit-core/src/lib.rs:1-23`). Both runtimes implement `serveRegistry` (`rivetkit-typescript/packages/rivetkit/src/registry/napi-runtime.ts:329`; `rivetkit-typescript/packages/rivetkit/src/registry/wasm-runtime.ts:296`).

The `@rivetkit/workflow-engine` manifest says "reentrant execution" (`rivetkit-typescript/packages/workflow-engine/package.json:4`). The architecture note explains the mechanism. A workflow can stop after a crash or eviction and then resume, because recorded operations replay from history (`rivetkit-typescript/packages/workflow-engine/architecture.md:7-10`).

- The `rivetkit` package is the user-facing Actor host. `setup()` returns a `Registry` (`rivetkit-typescript/packages/rivetkit/src/registry/index.ts:1058-1062`). `registry.start()` starts an actor envoy for a standalone server (`:951-982`). `registry.serve()` returns a fetch handler for serverless deployments (`:407-419`). `registry.listen()` binds an HTTP listener (`:421-470`). The native runtime executes the Actor code.

The entry points form three modes:

| Mode | Entry point | Runtime | Process lifetime |
|------|-------------|---------|------------------|
| Serverful, envoy | `registry.start()` without `RIVETKIT_RUNTIME_MODE=serverless` | Native | Long-running; the registry starts an envoy (`rivetkit-typescript/packages/rivetkit/src/registry/index.ts:980-982`). |
| Serverful, application listener | `registry.listen({ application })` | Native only; the method throws on the WebAssembly runtime (`rivetkit-typescript/packages/rivetkit/src/registry/index.ts:445-450`). | Long-running; the registry serves the application and an envoy (`:451-470`). |
| Serverless | `registry.serve()` returns a fetch handler; `registry.start()` calls `listen()` when `RIVETKIT_RUNTIME_MODE=serverless` (`:964-979`). | Native or WebAssembly, selected by the `rivetkit-core` feature. | The platform calls the handler per request (`:407-419`). |
- `@rivetkit/engine-runner` is a runner process. Its `Runner` class (`rivetkit-typescript/packages/engine-runner/src/mod.ts:203`) connects to `<baseUrl>/runners/connect?protocol_version=7&...` (`:752`) and declares `PROTOCOL_VERSION = 7` (`:22`). Its `Tunnel` class (`rivetkit-typescript/packages/engine-runner/src/tunnel.ts:41`) carries gateway requests to the Actors on that runner. In this commit, only `@rivetkit/engine-test-runner` imports this package (`engine/sdks/typescript/test-runner/src/index.ts:2-3`). The TypeScript runner package is therefore test-only in this commit. Production TypeScript Actors use the `rivetkit` Registry envoy host. Diagram 11.2 shows the runner protocol path for completeness, not the production TypeScript path.

### 4.2 The native and WebAssembly build targets

Two TypeScript packages are also Cargo workspace members. They are the bridge between the TypeScript SDK and the Rust core.

`@rivetkit/rivetkit-napi` builds the Node-API addon. The `package.json` `napi.triples.additional` field lists seven target triples: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, and `x86_64-pc-windows-msvc`. The addon exposes the envoy client and SQLite access. The source manifest is `rivetkit-typescript/packages/rivetkit-napi/Cargo.toml`.

`@rivetkit/rivetkit-wasm` builds the WebAssembly target. It uses the `wasm-runtime` feature of `rivetkit-core`. The crate `rivetkit-core` makes the two runtimes mutually exclusive with a `compile_error!`.

```rust
// rivetkit-rust/packages/rivetkit-core/src/lib.rs:1-23
#[cfg(all(feature = "native-runtime", feature = "wasm-runtime"))]
compile_error!(
	"`native-runtime` and `wasm-runtime` are mutually exclusive. Enable exactly one rivetkit-core runtime."
);

pub mod actor;
#[cfg(feature = "native-runtime")]
mod development_process;
#[cfg(feature = "native-runtime")]
pub mod engine_process;
pub mod error;
pub mod inspector;
pub mod inspector_bundle;
pub mod metrics;
pub mod registry;
pub mod runtime;
pub(crate) mod serde_metrics;
pub mod serverless;
#[cfg(feature = "native-runtime")]
pub mod serverless_http;
#[cfg(feature = "native-runtime")]
mod services_process;
pub mod telemetry;
```

The `compile_error!` is a build-time guard, not a runtime check. A build that enables both features fails. This rule protects the two targets from a shared binary that would contain two runtimes. Section 5 covers the Rust crates in more detail.

### 4.3 The dashboard and the inspector

`frontend` is the dashboard application. Its manifest is `frontend/package.json`, name `@rivetkit/engine-frontend`, private, version 2.0.21. The application source is `frontend/src`, with the directories `app`, `components`, `queries`, `routes`, `stores`, `lib`, and `utils`. The build tool is Vite. The configuration files are `vite.config.ts`, `vite.inspector.config.ts`, `vite.inspector-ui.config.ts`, and `vite.mcp-inspector-ui.config.ts`.

The frontend has four directories under `frontend/packages` and two under `frontend/apps`. Only three of the four package directories have a `package.json`. Neither application directory has one.

| Directory | Package | Version | What it holds |
|-----------|---------|---------|---------------|
| `frontend/packages/components` | `@rivet-gg/components` | 2.2.1 | Shared components. Private. |
| `frontend/packages/icons` | `@rivet-gg/icons` | 2.2.1 | The icon set. Public. |
| `frontend/packages/shared-data` | `@rivetkit/shared-data` | 2.2.1 | Shared data definitions. Private. The source is `src/index.ts` and `src/deploy.ts`. |
| `frontend/packages/example-registry` | none | — | No `package.json`. It holds `src/_gen.ts` only. |
| `frontend/apps/inspector` | none | — | No `package.json`. The application is built by the root Vite configuration. |
| `frontend/apps/inspector-ui` | none | — | No `package.json`. The application is built by `vite.inspector-ui.config.ts`. |

The count of packages is three, not four. The two `apps` directories and `example-registry` have no manifest. This map infers their role from the directory names and the Vite configuration files. The inference is marked here and not presented as a source description.

Two different classifications apply to the frontend output. A build flavor is a runtime feature set; a delivery target is a container image.

- The build flavors are cloud, open-source, and enterprise. `frontend/CLAUDE.md:8` says that one build serves all three through `features.*`. A browser flag selects the flavor; the code does not change. The flag source is the build-time variable `VITE_FEATURE_FLAGS`, a comma-separated list (`frontend/src/lib/features.ts:3`). In development, the `localStorage` key `FEATURE_FLAGS` overrides that variable (`frontend/src/lib/features.ts:5-7`). An unset value means every flag is on, which is the cloud build. An empty value means every flag is off, which is the open-source flavor (`.claude/reference/feature-flags.md:17-20`). Three flags define the flavor difference. `auth` ships the login and register flow. `platform` enables the cloud stack: billing, projects, and multi-tenancy. `acl` makes the engine enforce token authentication on the public endpoint (`.claude/reference/feature-flags.md:27-29`). The enterprise flavor sets `acl` and leaves `auth` and `platform` off (`.claude/reference/feature-flags.md:42`).
- The delivery targets are the cloud dashboard, the inspector, and the Ladle component workbench. Each target has its own Dockerfile: `frontend/cloud.Dockerfile`, `frontend/inspector.Dockerfile`, and `frontend/ladle.Dockerfile`. Each also has a Railway template. Section 9.3 lists both sets.

`frontend/CLAUDE.md:7` states the build rule for the embedded dashboard. The embedded build sets `BASE_URL=/ui/`. The `ui` module in `rivet-api-public` mounts that build. The inspector and Ladle targets are separate deliveries; they are not flavors of the cloud dashboard.

## 5. The Rust SDK workspace

`rivetkit-rust` holds eight crates. All eight are Cargo workspace members. The crates divide into three layers. The first layer is `rivetkit`. The second layer is the runtime and transport: `rivetkit-core`, `rivetkit-client`, and `rivetkit-engine-process`. The third layer is the protocol types: `rivetkit-client-protocol`, `rivetkit-actor-persist`, and `rivetkit-inspector-protocol`. `rivetkit-shared-types` holds types that callers in all layers use.

### 5.1 The eight packages

| Crate | Path | Purpose | Main dependency |
|-------|------|---------|-----------------|
| `rivetkit` | `rivetkit-rust/packages/rivetkit` | The Rust SDK for Actors, actions, events, queues, and test harnesses. The manifest description is "Rust SDK for RivetKit actors, actions, events, queues, and test harnesses". | `rivetkit-core` |
| `rivetkit-core` | `rivetkit-rust/packages/rivetkit-core` | The core runtime primitives for Actor hosts. The manifest description is "Core runtime primitives for RivetKit actor hosts". | `rivetkit-actor-persist` |
| `rivetkit-client` | `rivetkit-rust/packages/client` | The Rust client. The manifest description is "Rust client for RivetKit - the Stateful Serverless Framework for building AI agents, realtime apps, and game servers". | `rivetkit-client-protocol` |
| `rivetkit-client-protocol` | `rivetkit-rust/packages/client-protocol` | The client protocol types. The manifest description is "Client protocol types for RivetKit". | external `vbare` |
| `rivetkit-actor-persist` | `rivetkit-rust/packages/actor-persist` | The versioned persistence protocol types. The manifest description is "Versioned persistence protocol types for RivetKit actors". | external `vbare` |
| `rivetkit-inspector-protocol` | `rivetkit-rust/packages/inspector-protocol` | The inspector protocol types. The manifest description is "Inspector protocol types for RivetKit". | external `vbare` |
| `rivetkit-engine-process` | `rivetkit-rust/packages/engine-process` | Resolves, starts, and reuses the `rivet-engine` subprocess. The manifest description is "Resolves, spawns, and reuses the rivet-engine subprocess for local hosts and the CLI". | external `reqwest` |
| `rivetkit-shared-types` | `rivetkit-rust/packages/shared-types` | Shared public types. The manifest description is "Shared public types for RivetKit crates". | external `serde` |

The three protocol constants are exact. `rivetkit-rust/packages/client-protocol/src/lib.rs:8` sets `PROTOCOL_VERSION` to 4. `rivetkit-rust/packages/actor-persist/src/lib.rs:7` sets `CURRENT_VERSION` to 4. `rivetkit-rust/packages/inspector-protocol/src/lib.rs:7` sets `PROTOCOL_VERSION` to 6. The constants are in three different crates.

`rivetkit-engine-process` resolves, starts, and reuses the engine subprocess. The default release endpoint is `https://releases.rivet.dev`. The environment variable `RIVETKIT_ENGINE_RELEASES_ENDPOINT` overrides it, as `rivetkit-rust/packages/engine-process/src/lib.rs:20-22` shows. The download timeout is 60 seconds.

### 5.2 The layering rule

The internal note `docs-internal/engine/bare-protocol-crates.md` states the layering rule. The `rivetkit` crate stays a thin typed wrapper over `rivetkit-core`. The protocol crates are separate. The note states the layout. This map infers the build benefit: a codec change rebuilds the codec and its clients, not the whole SDK.

The rule matches the manifests. `rivetkit` depends on `rivetkit-core` and the protocol crates. `rivetkit-core` does not depend on `rivetkit`. The client crate depends on `rivetkit-client-protocol`. A user who needs only the client does not compile the server runtime.

### 5.3 The workspace inheritance rule for new protocol crates

The same note gives the manifest rule for a new protocol crate under `rivetkit-rust/packages/`. The note is at `docs-internal/engine/bare-protocol-crates.md:7`. The crate sets `workspace = "../../../"` in its `[package]` table (`rivetkit-rust/packages/client-protocol/Cargo.toml:9`) and joins the root workspace members. It defines a vbare schema. The note says that the TypeScript codec generation follows `engine/packages/runner-protocol/build.rs` (`docs-internal/engine/bare-protocol-crates.md:17`). This map repeats the rule because a reader who adds a protocol crate needs it. The rule is source-stated, not inferred.

The schema files have a fixed path. Each crate keeps them under `schemas/vN.bare`, for example `rivetkit-rust/packages/client-protocol/schemas/v1.bare` through `v4.bare`, and `rivetkit-rust/packages/actor-persist/schemas/v1.bare` through `v4.bare`. The Rust build script reads the directory `<crate>/schemas` (`rivetkit-rust/packages/client-protocol/build.rs:9`) and calls `vbare_compiler::process_schemas_with_config` (`:16-17`).

The TypeScript output has a fixed path too. It is `rivetkit-typescript/packages/rivetkit/src/common/bare/generated/<protocol-name>/<version>.ts` (`rivetkit-rust/packages/client-protocol/build.rs:37-56`). The script removes the old directory before it writes (`:47`). The command for each schema is `node node_modules/@bare-ts/tools/dist/bin/cli.js compile --generator ts <schema-path> -o <output-path>` (`:58-65`). The call at line 19 passes `client-protocol` as the protocol name (`:19`).

## 6. The other SDK directories

The repository holds more SDK surfaces outside the two main workspaces. They are small in file count, and they define the public contract for four languages and three schema formats.

### 6.1 Python

`rivetkit-python/client` is a Python package named `rivetkit-client`, version 0.9.0-rc.1. It uses PyO3 and maturin. The Python source is under `src`, with the modules `simple` and `events`. The package has its own `Cargo.toml`, `pyproject.toml`, `requirements.txt`, and `tests` directory. The version is a release candidate and does not match the 2.3.31 workspace version. The map reports the difference and does not explain it; the source does not state a reason.

### 6.2 Swift

`rivetkit-swift` is a Swift Package Manager package. `rivetkit-swift/Package.swift` declares Swift tools version 6.0, the package name `rivetkit-swift`, and two library products: `RivetKitClient` and `RivetKitSwiftUI`. The platforms are iOS 17, macOS 14, tvOS 17, watchOS 10, and visionOS 1. The only external dependency is `SwiftCBOR`, version 0.6.0 or later, from `https://github.com/valpackett/SwiftCBOR.git`. The targets are `RivetKitClient`, `RivetKitSwiftUI`, and the test target `RivetKitClientTests`.

The package also holds `SPEC_SWIFTUI.md`, `README.tpl.md`, `Sources`, `Tests`, and `examples`. The `pnpm-workspace.yaml:26` includes `rivetkit-swift/examples/*/server`, so a Swift example can have a TypeScript server component.

### 6.3 OpenAPI

The repository holds two OpenAPI documents. They describe different APIs. The table gives the provenance of each one. The last column gives the evidence for the row. It answers one question: what in this commit supports the identification of the document and the stated generator? Where no evidence exists, the cell says so.

| Document | OpenAPI version | Title | Version | Generator | Evidence for this row |
|----------|-----------------|-------|---------|-----------|-----------------------|
| `engine/artifacts/openapi.json` | 3.1.0 | `rivet-api-public` | 2.3.31 | `utoipa` in the build script of `rivet-api-public-openapi-gen` | The build script writes this exact path with this title and version (`engine/packages/api-public-openapi-gen/build.rs:5,20,25`). |
| `rivetkit-openapi/openapi.json` | 3.0.0 | `RivetKit API` | 2.2.0 | Not verified in this commit | The file exists at this path with this title and version. The repository does not declare a generator for it, so the Generator cell says not verified. |

The engine document is generated from the Rust router types. `engine/packages/api-public-openapi-gen/build.rs:5` calls `rivet_api_public::router::ApiDoc::openapi()`. Line 20 sets the output directory to `engine/artifacts`, and line 25 writes `openapi.json`. The build dependency on `utoipa` is at `engine/packages/api-public-openapi-gen/Cargo.toml:12`. The file that the generator writes is the file in this commit: both declare the title `rivet-api-public` and version 2.3.31.

The `rivetkit-openapi` document is older. Its version is 2.2.0, and the repository version is 2.3.31. This map does not claim which tool produced it. The version difference is a fact in the file.

### 6.4 AsyncAPI

`rivetkit-asyncapi/asyncapi.json` is one file. It declares AsyncAPI 3.0.0, title "RivetKit WebSocket Protocol", and version 2.3.31. The version matches the repository. The document describes the WebSocket message protocol that the SDK and the engine exchange.

### 6.5 JSON Schema

`rivetkit-json-schema/registry-config.json` is one file. It describes the object that a user passes to the TypeScript `setup()` function. The schema is `type: object` with these properties: `use`, `maxIncomingMessageSize`, `maxOutgoingMessageSize`, `noWelcome`, `logging`, `endpoint`, `token`, `namespace`, `headers`, `staticDir`, `httpBasePath`, `httpPort`, and more.

The word "registry" has two meanings in this repository. A reader must not merge them.

| Name | Language | Meaning |
|------|----------|---------|
| `Registry` (SDK) | TypeScript | The Actor registry. `setup()` builds it (`rivetkit-typescript/packages/rivetkit/src/registry/index.ts:1058-1062`). Its configuration has a `use` field. |
| `use` | TypeScript | A record of Actor definitions, keyed by Actor name (`rivetkit-typescript/packages/rivetkit/src/registry/config/index.ts:79`). A user puts each Actor in this record to register it. |
| `Registry` (workflow) | Rust | The workflow handler lookup map in Gasoline (`engine/packages/gasoline/src/registry.rs:14-17`). It has no relation to the SDK `Registry`. |
| `register_workflow` | Rust | The method that adds one workflow to the Gasoline `Registry` (`engine/packages/gasoline/src/registry.rs:51-55`). |

The SDK `Registry` holds Actors. The Gasoline `Registry` holds workflows. Both names are correct in their own layer.

The default values are in the property descriptions, not in `default` keywords. The table gives the first three top-level defaults and the nested `logging.level` default.

| Property | Default | Source |
|----------|---------|--------|
| `maxIncomingMessageSize` | 65,536 bytes | `rivetkit-json-schema/registry-config.json:14` |
| `maxOutgoingMessageSize` | 1,048,576 bytes | `rivetkit-json-schema/registry-config.json:18` |
| `noWelcome` | `false` | `rivetkit-json-schema/registry-config.json:22` |
| `logging.level` | `"warn"` | `rivetkit-json-schema/registry-config.json:30` |

The schema has a top-level `required` array with one entry, `use` (`rivetkit-json-schema/registry-config.json:201-203`). Every other top-level property is optional. The top-level `configurePool` property configures serverless runners in the engine (`rivetkit-json-schema/registry-config.json:110-112`). It also has a `required` array, with one entry, `url` (`rivetkit-json-schema/registry-config.json:153-155`). The `endpoint` description states the URL authentication syntax: `https://namespace:token@api.rivet.dev`. The `namespace` description states the default `"default"`.

### 6.6 Go

`engine/sdks/go/api-full` holds a Go client with a `go.mod` file. It is a generated OpenAPI client. The Go module path is in `engine/sdks/go/api-full/go.mod`. This map does not open every generated Go file; the purpose comes from the path and the OpenAPI generator configuration.

### 6.7 The TypeScript engine SDKs

`engine/sdks/typescript` holds five directories with a `package.json`. `pnpm-workspace.yaml:12-16` names only `api-full`, `envoy-protocol`, `test-envoy`, `test-envoy-native`, and `test-runner`. Two directories, `runner` and `runner-protocol`, repeat the package names `@rivetkit/engine-runner` and `@rivetkit/engine-runner-protocol` from `rivetkit-typescript/packages` (section 4.1). The evidence shows that these two directories are stale copies.

| Directory | Package | Purpose |
|-----------|---------|---------|
| `api-full` | `@rivetkit/engine-api-full` (generated) | The TypeScript OpenAPI client. |
| `envoy-protocol` | `@rivetkit/engine-envoy-protocol` (generated) | The TypeScript codec for the Envoy protocol. |
| `runner` | `@rivetkit/engine-runner` | A stale copy of the runner process package. |
| `runner-protocol` | `@rivetkit/engine-runner-protocol` | A stale copy of the runner protocol codec. |
| `test-runner` | `@rivetkit/engine-test-runner` (test only) | A test runner package. |

Four facts support the stale-copy conclusion:

1. The copies have an older version. `engine/sdks/typescript/runner/package.json:3` and `engine/sdks/typescript/runner-protocol/package.json:3` say `2.2.1`. The `rivetkit-typescript/packages` copies say `2.3.31` (`rivetkit-typescript/packages/engine-runner/package.json:3`; `rivetkit-typescript/packages/engine-runner-protocol/package.json:3`).
2. The pnpm workspace does not include the copies. `pnpm-workspace.yaml:9-31` names `rivetkit-typescript/packages/*` at line 25 and the individual `engine/sdks/typescript` directories at lines 12-16. It does not name `engine/sdks/typescript/runner` or `engine/sdks/typescript/runner-protocol`.
3. The lockfile has no importer for the copies. A search of `pnpm-lock.yaml` for `engine/sdks/typescript/runner` returns no result.
4. The source files differ. The runner entry file `engine/sdks/typescript/runner/src/mod.ts` has 1,977 lines, and the workspace entry file `rivetkit-typescript/packages/engine-runner/src/mod.ts` has 1,939 lines. The protocol entry file `engine/sdks/typescript/runner-protocol/src/index.ts` has 2,515 lines, and the workspace entry file `rivetkit-typescript/packages/engine-runner-protocol/src/index.ts` has 2,189 lines. Both runner copies declare `PROTOCOL_VERSION = 7` (`engine/sdks/typescript/runner/src/mod.ts:22`; `rivetkit-typescript/packages/engine-runner/src/mod.ts:22`).

The build therefore uses the `rivetkit-typescript/packages` copies. The `engine/sdks/typescript` copies are not part of the pnpm workspace and are not installed.

The `pnpm-workspace.yaml` names `engine/sdks/typescript/test-envoy` and `engine/sdks/typescript/test-envoy-native` as well, but both directories are absent at this commit. Section 2.3 lists the absent globs.

### 6.8 The protocol schemas

`engine/sdks/schemas` holds the vbare schemas that generate the Rust and TypeScript protocol crates. The directories are `data`, `depot`, `envoy-protocol`, `epoxy-protocol`, `runner-protocol`, `universaldb-commit`, and `ups-protocol`. The `README.md` in that directory describes the generation workflow. The runner protocol schemas have versions 1 through 7, and the envoy protocol schemas have versions 1 through 8. The version numbers match the re-exports in sections 3.13 and 3.15.

### 6.9 The Fern configuration

`engine/sdks/api/fern` holds the Fern generator configuration for the HTTP API clients. `engine/sdks/api/fern/generators.yml:4` points Fern at `engine/artifacts/openapi.json`. The same file defines two generators. `fernapi/fern-typescript-node-sdk` writes `engine/sdks/typescript/api-full/src` (`:9-14`), and `fernapi/fern-go-sdk` writes `engine/sdks/go/api-full` (`:21-25`). The directory is small. Its role is generator input, not runtime code.

### 6.10 The contract artifacts and their consumers

Each public contract has one artifact, one generator, and at least one consumer. The table puts them in one place.

| Artifact | Format | Consumer | Generator |
|----------|--------|----------|-----------|
| `engine/artifacts/openapi.json` | OpenAPI 3.1.0 | Fern TypeScript and Go clients (`engine/sdks/api/fern/generators.yml:9-25`); OpenAPI Generator Rust client (`scripts/openapi/gen_rust.ts:9-10,40-50`) | `utoipa` in `engine/packages/api-public-openapi-gen/build.rs:5-25` |
| `rivetkit-openapi/openapi.json` | OpenAPI 3.0.0 | Not verified in this commit | Not verified in this commit |
| `rivetkit-asyncapi/asyncapi.json` | AsyncAPI 3.0.0 | The WebSocket reference in `docs` (`scripts/docs/gen-api-reference.mjs:12`; `docs/api/endpoints.json:87-88`) | Not verified in this commit |
| `rivetkit-json-schema/registry-config.json` | JSON Schema | Editors and users who validate `setup()` input | The TypeScript script `rivetkit-typescript/packages/rivetkit/scripts/registry-config-schema-gen.ts:3-29` |
| `engine/artifacts/config-schema.json` | JSON Schema | Engine configuration readers | `schemars` in `engine/packages/config-schema-gen/build.rs:4-37` |
| `engine/artifacts/errors/*.json` | JSON error catalog | Error reference and client generation | The `RivetError` derive macro writes one file at compile time (`engine/packages/error-macros/src/lib.rs:576-609`). |
| `<crate>/schemas/vN.bare` | BARE | Rust protocol crates and TypeScript codecs | `vbare_compiler` (`rivetkit-rust/packages/client-protocol/build.rs:16-17`) |

The error catalog has no separate generator command. The `RivetError` derive runs during compilation, and its write function runs inside the macro. The write function creates `engine/artifacts/errors/<group>.<code>.json` (`engine/packages/error-macros/src/lib.rs:590-595`). A build therefore writes the file for each declared error code.

The Swift client has no separate contract artifact. It encodes an action request as Concise Binary Object Representation (CBOR) and sends it over HTTP (`rivetkit-swift/Sources/RivetKitClient/ActorHandle.swift:21-37`). It decodes the response with the same codec (`:56-57`). The client names the encoding in the `x-rivet-encoding: cbor` header (`rivetkit-swift/Sources/RivetKitClient/Routing.swift:8`). CBOR is a relation between the Swift client and the HTTP action path, not a generated file.

## 7. Documentation and internal design notes

The repository holds two documentation trees. `docs` is the published documentation. `docs-internal` is the design record for people who change the code. The two trees serve different readers and use different rules.

### 7.1 The published documentation bundles

`docs` has four bundles and one root file. The counts come from `find docs/<bundle> -type f`.

| Bundle | Files | What it covers |
|--------|-------|----------------|
| `docs/actors` | 68 | The Actor model, actions, events, queues, state, SQLite, scheduling, and the client interfaces. |
| `docs/api` | 52 | The HTTP API reference, its parameters, and its authentication. |
| `docs/general` | 16 | Installation, self-hosting, concepts, and deployment. |
| `docs/integrations` | 6 | Framework integrations such as React, Next.js, and Cloudflare Workers. |
| `docs/CLAUDE.md` | 1 | Instructions for the documentation writer. |

The source of truth for the HTTP reference is `docs/api/endpoints.json`. The `$comment` field says: "Source of truth for the generated HTTP API reference. Run `node scripts/docs/gen-api-reference.mjs` after editing." The file names the engine OpenAPI artifact at `engine/artifacts/openapi.json` and defines the authentication header. The generated gateway and Actor pages come from this file. A writer must not edit the generated pages by hand.

The website repository publishes this directory. `docs` is a content source, not a standalone site. The core repository has no `website` directory, as section 2.2 states.

### 7.2 The internal design notes

`docs-internal/engine` has 42 entries: 37 files and 5 directories. The files are the design record for the control plane. The five directories are `design-choicse`, `GASOLINE`, `operate`, `sqlite`, and `universaldb`. The directory name `design-choicse` is misspelled in the source. This map keeps the spelling because a link must work.

| Note or directory | Subject |
|-------------------|---------|
| `ACTOR_ERRORS.md` | The error model for Actor calls. |
| `ACTOR_KEY_RESERVATION.md` | How the control plane reserves an Actor key. |
| `ACTOR_KV.md` | The Actor key-value store. |
| `ACTOR_LIFECYCLE.md` | The Actor state machine and its transitions. |
| `actor-rescheduling.md` | How an Actor moves to another runner. |
| `actor-task-dispatch.md` | How a task reaches an Actor. |
| `API.md` | The API crate split and the circular dependency that `rivet-api-types` breaks. |
| `bare-protocol-crates.md` | The rules for a new protocol crate. |
| `depot.md` | The Depot storage design. |
| `design-choicse/` | The embedded key-value design note. The directory name is misspelled in the source. |
| `DOCKER.md` | The Docker build. |
| `envoy-load-balancing.md` | How the newer envoy path balances load. |
| `ERRORS.md` | The structured error design. |
| `GASOLINE/` | The durable execution notes. |
| `GUARD.md` | The proxy design. |
| `HIBERNATING_WS.md` | The hibernating WebSocket design. |
| `inspector-protocol.md` | The inspector protocol. |
| `MEMORY_DEBUGGING.md` | The memory debugging method. |
| `napi-bridge.md` | The Node-API bridge. |
| `operate/` | Operations notes. |
| `oss-to-ee-upgrade.md` | The upgrade from the open-source to the enterprise build. |
| `PEGBOARD_TUNNEL_RETRIES.md` | Tunnel retry rules. |
| `RELEASING.md` | The release process. |
| `rivetkit-core-internals.md` | The Rust runtime internals. |
| `rivetkit-core-state-management.md` | Runtime state management. |
| `rivetkit-core-websocket.md` | The runtime WebSocket. |
| `rivetkit-rust-client.md` | The Rust client. |
| `rivetkit-telemetry.md` | Runtime telemetry. |
| `RUNNER_LIFECYCLE.md` | The runner lifecycle. |
| `RUNNER_SHUTDOWN.md` | The runner shutdown sequence. |
| `SDKS.md` | The SDK layout. |
| `SERVERLESS.md` | The serverless runner mode. |
| `sleep-sequence.md` | The Actor sleep sequence. |
| `sqlite/` | The SQLite notes. |
| `SQLITE_METRICS.md` | SQLite metrics. |
| `SQLITE_OPTIMIZATIONS.md` | SQLite optimizations. |
| `sqlite-vfs.md` | The SQLite virtual file system. |
| `TERMINOLOGY.md` | The project vocabulary. |
| `TEST_DEPENDENCIES.md` | The test dependency harness. |
| `TEST_SNAPSHOTS.md` | The snapshot tests. |
| `tls-trust-roots.md` | The TLS trust roots. |
| `universaldb/` | The UniversalDB notes. |

The other two internal trees are `docs-internal/rivetkit-typescript` and `docs-internal/integrations`. The first holds `ACTOR_KV_STRUCTURE.md`, `CONFIG_TYPES.md`, three dynamic Actor specifications, `EXAMPLE_PROJECT_STRUCTURE.md`, `HIBERNATABLE_CONNECTIONS.md`, `sqlite-ltx/`, `traces/`, and `workflow-engine/`. `sqlite-ltx/` holds the LTX page-format notes, including the version 3 plan; the source does not expand LTX (`docs-internal/rivetkit-typescript/sqlite-ltx/ltx-v3-plan.md:1-12`). `traces/` holds the design note for long-running spans in `@rivetkit/traces` (`docs-internal/rivetkit-typescript/traces/SCHEMA_FOR_LONG_RUNNING_SPANS.md:1-3`). `workflow-engine/` holds the note on workflow engine persistence and `flush()` (`docs-internal/rivetkit-typescript/workflow-engine/FLUSHING.md:1-3`). The three specifications are `DYNAMIC_ACTOR_FAILED_START_RELOAD_SPEC.md`, `DYNAMIC_ACTOR_TYPESCRIPT_SOURCE.md`, and `DYNAMIC_ACTORS_ARCHITECTURE.md`. The second tree holds `workflow-world-defaults-audit.md`.

The rule that a note can be stale applies to every note in this section. A note is a map, and a map can be stale. When a note and the code disagree, the code wins and the document says so. Section 3.3 gives one example: the `API.md` reason for the types crate matches the manifests. Section 7.3 gives a case where this map does not rely on a note.

### 7.3 Where an internal note is not used as evidence

`docs-internal/engine/TERMINOLOGY.md` is a vocabulary note. This map does not use it to define a type or a count. Every type name in this document comes from a source file. Most come from a `src/lib.rs` file, and some come from another source file such as `run_config.rs`. Every count comes from a command or a manifest. A terminology note can explain a word, but it cannot prove that a crate exists or that a constant has a value.

## 8. Examples, integrations, scripts, and the shared package

### 8.1 The 57 examples

`examples` has 57 project directories. Each project has its own manifest. Two Rust examples are Cargo workspace members: `chat-room-rust` and `hello-world-rust`. The other examples are TypeScript or framework projects.

The examples divide into seven groups. The grouping is editorial, and every directory appears once.

| Group | Count | Projects |
|-------|-------|----------|
| Hello world and framework baselines | 15 | `hello-world`, `hello-world-effect`, `hello-world-render`, `hello-world-cloudflare-workers`, `hello-world-cloudflare-workers-hono`, `hello-world-cloudflare-workers-raw`, `hello-world-supabase-functions`, `hello-world-rust`, `hono`, `hono-react`, `elysia`, `next-js`, `react`, `react-render`, `trpc` |
| State and actions | 5 | `actor-actions`, `state`, `state-render`, `cross-actor-actions`, `dynamic-actors` |
| Realtime and streams | 13 | `chat-room`, `chat-room-effect`, `chat-room-render`, `chat-room-rust`, `multiplayer-game`, `multiplayer-game-patterns`, `cursors`, `cursors-raw-websocket`, `native-websockets`, `raw-websocket-handler`, `raw-websocket-handler-proxy`, `stream`, `stream-render` |
| SQLite and databases | 5 | `sqlite-drizzle`, `sqlite-raw`, `turso`, `per-tenant-database`, `geo-distributed-database` |
| Workflows, queues, and schedules | 3 | `workflow-sdk`, `scheduling`, `queue-sandbox` |
| Agents and sandboxes | 7 | `ai-agent`, `ai-agent-effect`, `ai-generated-actor`, `ai-and-user-generated-actors-freestyle`, `sandbox`, `sandbox-coding-agent`, `experimental-durable-streams-ai-agent` |
| Platform and protocol details | 9 | `custom-serverless`, `raw-fetch-handler`, `jwt-counter`, `jwt-better-auth`, `collaborative-document`, `collaborative-document-ot`, `inspector-tabs`, `kitchen-sink`, `docs` |

The `docs` example is a project, not the documentation bundle. It is under `examples/docs` and has its own manifest. The name collision is real.

### 8.2 The two integrations

| Package | Directory | Purpose |
|---------|-----------|---------|
| `@rivet-dev/flue` | `integrations/flue-runtime` | The manifest description is "Run Flue agents and workflows as durable Rivet Actors". |
| `@rivet-dev/workflow-world` | `integrations/workflow-world` | The manifest description is "Vercel's Workflow SDK backed by Rivet Actors". |

The `pnpm-workspace.yaml:23-24` includes `integrations/*` and `integrations/*/examples/*`. An integration can ship its own example packages.

### 8.3 The 22 script directories

The Evidence column separates source-stated purposes from inferred ones. A stated purpose appears in a comment, README, or manifest. An inferred purpose is this map's judgment from file names and contents, because the source does not state the purpose.

| Script directory | What it does | Evidence |
|------------------|--------------|----------|
| `api` | Small TypeScript command-line scripts that call the HTTP API to create Actors and manage runner configurations. | Inferred from `scripts/api/actors/create.ts` and `scripts/api/runner-configs/upsert.ts`. |
| `cargo` | Shell wrappers around Cargo checks, including the `rivetkit-core` WebAssembly check. | Inferred from `scripts/cargo/check-rivetkit-core-wasm.sh:1`. |
| `ci` | Runs continuous integration checks, including the Supabase edge dependency-closure guard. | Stated at `scripts/ci/check-edge-native-closure.mjs:1-10`. |
| `debug` | Debugging helpers, including a Mermaid debugger and an Epoxy changelog decoder. | Inferred from `scripts/debug/debug_mermaid.ts` and `scripts/debug/decode_epoxy_changelog.js`. |
| `docker` | Builds and exports Docker images, including the air-gapped enterprise engine image. | Stated at `scripts/docker/build-export.sh:3-5`. |
| `docker-builder-base` | Builds and pushes the builder base images. | Stated at `scripts/docker-builder-base/build-push.sh:3-5`. |
| `docs` | Generates the HTTP API reference from the checked-in OpenAPI documents. | Stated at `scripts/docs/gen-api-reference.mjs:2-8`. |
| `fern` | Runs the Fern generator for the API clients. | Stated at `scripts/fern/gen_inner.sh:4-7`. |
| `format` | Runs the formatter over changed files. | Inferred from `scripts/format/agent-format.mjs:1-10`. |
| `frontend` | Promotes the `main` branch to production. | Stated at `scripts/frontend/promote-prod.sh:1-8`. |
| `openapi` | Generates the Rust OpenAPI client for a Fern group. | Stated at `scripts/openapi/gen_rust.ts:8-11`. |
| `pipboy` | Internal tooling built with Effect, an external TypeScript library (`scripts/pipboy/package.json:13`). The modules are `compare`, `footprint`, `github`, `hygiene`, `native`, `render`, `schema`, and `trace`. | Inferred from the module names. The source does not state a single purpose, so this map does not invent one. |
| `publish` | Release automation with a continuous integration path and a local release path. | Stated at `scripts/publish/package.json:7-9`. |
| `python` | A one-off scanner for the Epoxy changelog subspace. It stops at the first suspected version 3 entry. | Stated at `scripts/python/fdb-scan-epoxy-changelog.py:1-6`. |
| `ralph` | An internal automation tool. The directory holds `ralph.sh`, `prd.json`, `progress.txt`, `reviews/`, and `archive/`. | The source does not state the tool's purpose in the directory tree. |
| `rivetkit` | Small helper shell scripts for the SDK, including a cache reset and an SSE test. | Inferred from the file names. |
| `run` | Development environment variables and local run helpers. | Stated at `scripts/run/dev-env.sh:1-3`. |
| `setup` | Prepares the user-scoped agent working directory. | Stated at `scripts/setup/agents-dir.sh:1-3`. |
| `skill-evals` | Evaluates the agent skills. | Inferred from the directory name and the `evals` subdirectory. |
| `test-examples` | Starts each example's development server and verifies that its endpoints respond. | Stated at `scripts/test-examples/README.md:1-3`. |
| `tests` | Load tests and other test suites. | Stated at `scripts/tests/package.json:7-10`. |
| `vbare-gen-converters` | Generates Rust converter functions between two adjacent BARE schema versions. | Stated at `scripts/vbare-gen-converters/index.ts:1-17`. |

`scripts/tsconfig.json` is a file, not a directory. The count of 22 counts directories only. `scripts/pipboy` and `scripts/publish` and `scripts/tests` are pnpm workspace members, as `pnpm-workspace.yaml:27-29` shows.

The converter script runs when a protocol schema moves from one version to the next. A developer gives it the old schema, the new schema, and an output directory (`scripts/vbare-gen-converters/index.ts:9-10`). It writes two Rust files, one for each direction (`:12`). Each field that exists on only one side becomes `todo!()`, so the developer must decide how to migrate it (`:5-7`).

### 8.4 The shared virtual WebSocket package

`shared/typescript/virtual-websocket` is the package `@rivetkit/virtual-websocket`. The manifest description is "Virtual WebSocket implementation for creating linked WebSocket pairs". The package is a workspace member through `pnpm-workspace.yaml:31`. The engine runner depends on it. A linked pair gives a test a client and a server endpoint without a network socket.

## 9. Deployment assets

### 9.1 The self-host directory

`self-host` holds three kinds of deployment sample.

| Path | Contents |
|------|----------|
| `self-host/compose` | Six Compose projects: `dev`, `dev-host`, `dev-multidc`, `dev-multidc-multinode`, `dev-multinode`, and `template`. |
| `self-host/workers` | Eleven worker targets: `aws-ecs`, `aws-lambda`, `cloudflare`, `custom`, `docker-compose`, `gcp-cloud-run`, `kubernetes`, `railway`, `supabase`, `vercel`, and `vm`. |
| `self-host/control-plane` | Six control plane targets: `aws-ecs`, `custom`, `docker-compose`, `kubernetes`, `railway`, and `vm`. |

The `self-host/compose/template` directory is a pnpm workspace member (`pnpm-workspace.yaml:11`). The five development Compose projects have different host, datacenter, and node arrangements. In these names, a node is one engine container with its own host name and configuration. The project `dev-multinode` runs three engine containers in one datacenter (`self-host/compose/dev-multinode/README.md:35-36`; `self-host/compose/dev-multinode/docker-compose.yml:190,233,274`). The names state the difference: `dev`, `dev-host`, `dev-multidc`, `dev-multinode`, and `dev-multidc-multinode`. This map reads only the service definitions at the cited lines, not the whole Compose files. Document [15 Deployment and self-hosting](15-deployment-and-self-hosting.md) covers the deployment topology.

The workers group is larger than the control plane group. The source does not state the reason in the directory tree. This map does not supply one.

### 9.2 The Docker directory

| Path | Contents |
|------|----------|
| `docker/builder-base` | Seven Dockerfiles: `engine-builder`, `engine-runtime-full`, `engine-runtime-slim`, `linux-gnu`, `linux-musl`, `osxcross`, and `windows-mingw`. |
| `docker/build` | Seven cross-build Dockerfiles: `darwin-arm64`, `darwin-x64`, `linux-arm64-gnu`, `linux-arm64-musl`, `linux-x64-gnu`, `linux-x64-musl`, and `windows-x64`. |
| `docker/engine/Dockerfile` | The engine image. |
| `docker/compute-hello-world` | A small image for Rivet Compute with `Dockerfile`, `server.js`, and `index.html`. |

The builder base images separate the full runtime from the slim runtime. The cross-build images cover two architectures and three operating system families. The `docker/engine/Dockerfile` is the runtime image. Rivet Compute is the managed pool product (`.claude/reference/feature-flags.md:32`). A user deploys a container image to it. The compute hello world is a sample workload, not part of the control plane. Its server prints an informational message and serves one static page (`docker/compute-hello-world/server.js:8-18`). Document [15 Deployment and self-hosting](15-deployment-and-self-hosting.md) covers the images.

### 9.3 The frontend delivery files

`frontend` holds three Dockerfiles and three Railway templates. These files define three delivery targets, not three build flavors:

| File | Purpose | Delivery target |
|------|---------|-----------------|
| `frontend/cloud.Dockerfile` | Builds the cloud dashboard image. | `cloud` |
| `frontend/inspector.Dockerfile` | Builds the inspector image. | `inspector` |
| `frontend/ladle.Dockerfile` | Builds the Ladle component workbench image. | `ladle` |
| `frontend/railway.cloud.toml` | The Railway template for the cloud dashboard. | `cloud` |
| `frontend/railway.inspector.toml` | The Railway template for the inspector. | `inspector` |
| `frontend/railway.ladle.toml` | The Railway template for Ladle. | `ladle` |

The cloud image still serves the cloud, open-source, and enterprise build flavors at runtime, as section 4.3 states. The inspector and Ladle images are different applications, not additional flavors.

The `Caddyfile` and `Caddyfile.ladle` files serve the static builds. The `docker-entrypoint.sh` script starts the container. The `cloudflared` directory holds the tunnel configuration. These files are delivery plumbing, not application logic.

## 10. The snapshot: sibling repositories

The snapshot at `/Users/darkvoid/GitSources/src.Rivet/` holds the core repository and seven sibling repositories. Six are product repositories. The seventh is the website. This section gives one paragraph for each. Document [17 The product family](17-the-product-family.md) is the deep dive.

The sibling head commits below come from `git -C <repo> rev-parse HEAD` in the checked-out snapshot. They are independent of the core commit in the introduction. The language descriptions come from file counts and the top-level README. The file counts in the paragraphs below come from `find <repo> -name '*.<extension>' | wc -l` for each extension. This map did not read the application source of a sibling repository.

| Repository | Head commit | Dominant language | Relation to Rivet |
|------------|-------------|-------------------|-------------------|
| `agents` | `e963879702cf40ccf5be71e8be970626413dd28a` | TypeScript | Agent infrastructure that runs agents as Rivet Actors. |
| `agentos` | `c7462927ec452bbd5def5c0fc50a3fd74574d13a` | Rust and TypeScript | Sandbox runtime that can run as a durable instance on Rivet. |
| `workflows` | `2fab0efd5bf1a75035df5362ab57058680219264` | TypeScript | Durable multi-step operations for Rivet Actors. |
| `dynamic-apps` | `b5398eeb1fe3cf7d26065791a5f71a9b8b530f53` | TypeScript | User-generated request handlers with state in a Rivet Actor. |
| `open-orange` | `0f3c2eac56d7f67d16398d54c4560543621f5582` | TypeScript and JavaScript | Cloudflare Workers compatibility layer for Rivet. |
| `durable-object-migrator` | `816c28b376e47b511f7ea7a635ab19995d84169a` | TypeScript | Moves Cloudflare Durable Objects to a self-hosted Rivet deployment. |
| `website` | `10c8c186cc24d0aa54f15cedb9c2658fa4276483` | TypeScript and JavaScript | The documentation site that publishes `docs`. |

### 10.1 `agents`

`agents` is a TypeScript-dominant repository. A file count gives 476 `.ts` files, 60 `.tsx` files, 58 `.rs` files, and 7 `.py` files. The README describes it as "Agent infrastructure from Rivet" (`agents/README.md:3`).

Three names in this repository need a definition.

- **Pi** is an external coding-agent project at `https://github.com/earendil-works/pi`, not a Rivet package (`agents/README.md:9`).
- **`@rivet-dev/pi`** is the Rivet package that runs Pi as a durable Rivet Actor (`agents/packages/pi/package.json:2-4`).
- **`@rivet-dev/sandbox-adapter`** supplies sandbox providers for agent harnesses that run in Rivet Actors (`agents/packages/sandbox-adapter/package.json:2-4`). A sandbox is the isolated process that runs agent code.
- **`sandbox-agent`** is a separate product in the same repository. It runs coding agents in sandboxes and controls them over HTTP (`agents/README.md:10`; `agents/sandbox-agent/package.json:2`).

The relation is a consumer: the agents run on the Rivet Actor runtime.

### 10.2 `agentos`

`agentos` uses Rust and TypeScript. A file count gives 589 `.rs` files, 755 `.ts` files, and 139 `.js` files. The README describes small, fast, Linux-compatible WebAssembly and V8 sandboxes. The hosted mode runs a sandbox as a durable instance on Rivet Cloud or on a self-hosted Rivet deployment. The relation is a workload and a consumer: the sandbox is the process that an Actor can host.

### 10.3 `workflows`

`workflows` is a TypeScript repository. The README describes "Durable, replayable multi-step operations for Rivet Actors". The package is `@rivet-dev/workflows`. The relation is a layer above the Actor: a workflow step runs inside an Actor and replays from history. This repository is separate from the Gasoline engine in the core repository. Gasoline is the internal durable execution engine. This repository is the user-facing workflow package.

### 10.4 `dynamic-apps`

`dynamic-apps` is a TypeScript repository. The README describes user-generated request handlers that run in a sandboxed deployment virtual machine (`dynamic-apps/README.md:3-5`). The relation is a consumer that uses the Actor as the durable control point for a deployment. Three terms need a definition.

- **Release state.** A release is one immutable built artifact of a user application. `deployApp()` builds it, stores it through a per-app Rivet state Actor, and activates it (`dynamic-apps/packages/dynamic-apps/README.md:9-10,16-19`). The artifact is persisted in the Actor SQLite store (`dynamic-apps/specs/agentos-inline-runtime-and-logging.md:16-18`).
- **Cache hit.** The process already holds an agentOS virtual machine for that immutable release. agentOS is the sandbox runtime in the sibling `agentos` repository (section 10.2). The request goes to the cached instance, and the code makes no Actor or storage call (`dynamic-apps/packages/dynamic-apps/README.md:18-19`; `dynamic-apps/specs/agentos-inline-runtime-and-logging.md:37-41`).
- **Cache miss.** The process has no virtual machine for that release. The request resolves the release through the Actor, verifies and materializes the artifact once, creates the agentOS virtual machine, mounts the artifact at `/app`, and then executes the request (`dynamic-apps/specs/agentos-inline-runtime-and-logging.md:29-35`). The next request for the same release is a cache hit.

The source calls the stored form the "AOSP artifact" and does not expand the abbreviation (`dynamic-apps/packages/dynamic-apps/README.md:16-17`). AOSP is an opaque label in this source. This map keeps the label and describes the artifact by its role. The artifact is the built application directory that the runtime materialises and mounts at `/app` (`dynamic-apps/specs/agentos-inline-runtime-and-logging.md:16-18,29-35`).

### 10.5 `open-orange`

`open-orange` uses TypeScript and JavaScript. The package description is "Cloudflare Workers compatibility layer for Rivet". Cloudflare Workers is Cloudflare's serverless application runtime, and the source treats a Worker as the unit that it runs (`open-orange/README.md:10,25`). A Durable Object is Cloudflare's stateful keyed object. OpenOrange runs one Rivet Actor for each object, with its own SQLite database (`open-orange/README.md:154`). The relation is a compatibility adapter: it translates the Cloudflare programming model into the Rivet model.

### 10.6 `durable-object-migrator`

`durable-object-migrator` is a TypeScript repository. The README says "Move Cloudflare Durable Objects to your own server with no downtime and no lost writes." The relation is a migration tool. It moves data and traffic from Cloudflare Durable Objects to a self-hosted Rivet deployment.

### 10.7 The name `open-orange`

The directory on disk is `open-orange`. The product name in the package description is "OpenOrange". This map uses the directory name for the path and the product name for the product. The difference is a spelling convention, not two projects.

### 10.8 `website`

`website` is a TypeScript and JavaScript repository. It is the documentation site. It is not one of the six product repositories. The core repository has no `website` directory. The `docs` directory in the core repository is the content that this site publishes. Section 2.2 records the absence.

### 10.9 What this section does not do

This section does not trace a call path in a sibling repository. It does not compare the sibling source with the core source. It does not claim that a sibling package runs on a specific Rivet version. Those questions belong to document 17. The head commits are a snapshot fact. A later commit can change a file count.

## 11. The diagrams

This section gives four diagrams. The first shows the dependency direction between the major layers. The second shows the Actor scheduling and request path. The third shows the build-metadata flow that the workspace split creates. The fourth shows the repository tree.

### 11.1 The layer dependency diagram

The diagram reads from left to right. The left side is the caller, and the right side is durable storage. Each box names one real item and the crate that owns it. The unit of a solid arrow is a crate, not an item. A solid arrow means the source crate lists the target crate in its manifest. A dotted arrow is a runtime edge that no manifest states. A Cargo edge says which crate compiles against which. A runtime edge says which service calls which service.

```mermaid
flowchart LR
    SDK["rivetkit_client::Client"] -.->|HTTP or WebSocket| G["rivet_guard::start"]
    G --> AP["rivet_api_public::router"]
    AP --> AQ["rivet_api_peer::router"]
    AP --> PB["pegboard::registry"]
    AQ --> PB
    PB --> DEP["depot::registry"]
    PB --> GAS["gasoline::workflow::Workflow"]
    GAS --> UDB["universaldb::Database"]
    DEP --> UDB
    EP["epoxy::workflows::replica::Workflow"] --> UDB
    UDB -.->|Enterprise cold tier only| B[("object storage service")]
```

Five points need a note.

1. The SDK box is the only runtime caller in this diagram. `rivetkit_client::Client` calls Guard over HTTP or WebSocket. The SDK has no Cargo dependency on Guard. The client protocol crates define the bytes.
2. The API edges are Cargo edges. `rivet-api-public` lists `pegboard` at `engine/packages/api-public/Cargo.toml:19` and `rivet-api-peer` at `:22`. `rivet-api-peer` lists `pegboard` at `engine/packages/api-peer/Cargo.toml:20`. `rivet-guard` lists `rivet-api-public` at `engine/packages/guard/Cargo.toml:44`.
3. The source note `docs-internal/engine/API.md:7` describes a cycle: `rivet-api-peer` -> `pegboard` -> `rivet-api-peer`. The manifests show the first edge and the shared `rivet-api-types` crate, but no `pegboard -> rivet-api-peer` edge in this commit. The diagram omits that edge.
4. `epoxy` depends on `universaldb` (`engine/packages/epoxy/Cargo.toml:39`), not the reverse. The diagram draws Epoxy as a sibling of Depot.
5. The bucket arrow is a runtime edge. It is dotted because only the enterprise build writes the cold tier (`docs-internal/engine/oss-to-ee-upgrade.md:9-10`). The open-source build keeps its data in the configured UniversalDB driver.

The boxes name real items: `rivet_guard::start` (`engine/packages/guard/src/lib.rs:12`), `rivet_api_public::router` (`engine/packages/api-public/src/router.rs:53`), `rivet_api_peer::router` (`engine/packages/api-peer/src/router.rs:6`), `pegboard::registry` (`engine/packages/pegboard/src/lib.rs:14`), `depot::registry` (`engine/packages/depot/src/lib.rs:23`), `gasoline::workflow::Workflow` (`engine/packages/gasoline/src/workflow.rs:11`), `universaldb::Database` (`engine/packages/universaldb/src/database.rs:41`), and `epoxy::workflows::replica::Workflow` (`engine/packages/epoxy/src/lib.rs:22`).

### 11.2 The Actor scheduling and request path

The second diagram shows the path that the first diagram omits. The path starts at the same client and Guard boxes. It ends at the process that hosts the Actor. Dotted arrows are runtime calls or signals. No arrow in this diagram is a Cargo edge. A dotted arrow follows the direction of the request or signal. Guard does not dial out to the envoy or the runner. The envoy or runner opens the WebSocket to Guard, and Guard passes the connection to the named handler. The diagram draws that connection from the envoy or runner to Guard.

```mermaid
flowchart LR
    SDK["rivetkit_client::Client"] -.->|HTTP or WebSocket| G["rivet_guard::start"]
    G -.->|actor path| SEL["gateway3_selector::select_gateway"]
    SEL -.->|SampledIn| PG3["pegboard_gateway3::PegboardGateway3"]
    SEL -.->|WebSocket or not sampled| PG2["pegboard_gateway2::PegboardGateway2"]
    PG3 -.->|tunnel frames| PE["pegboard_envoy::PegboardEnvoyWs"]
    PG2 -.->|tunnel frames| PE
    PR["pegboard_runner::PegboardRunnerWsCustomServe"]
    PE -.->|"opens /envoys/connect to Guard"| G
    PR -.->|"opens /runners/connect to Guard"| G
    G -.->|wake signal| AW["pegboard::workflows::actor2::pegboard_actor2"]
    AW -.->|allocation| PE
    AW -.->|allocation| PR
    PE -.->|dispatches over tunnel| ACT["rivetkit::Actor"]
    PR -.->|tunnel| TR["@rivetkit/engine-runner::Runner"]
    TR -.->|hosts| RA["@rivetkit/engine-runner::RunnerActor"]
```

Three types in this diagram need a definition. `PegboardEnvoyWs` is the engine-side handler for the envoy WebSocket, and it implements `CustomServeTrait` (`engine/packages/pegboard-envoy/src/lib.rs:43-56`). `PegboardRunnerWsCustomServe` is the engine-side handler for the runner WebSocket, and it also implements `CustomServeTrait` (`engine/packages/pegboard-runner/src/lib.rs:31-44`). `RunnerActor` is the runner-side object that hosts one Actor (`rivetkit-typescript/packages/engine-runner/src/actor.ts:14`).

Four mechanisms give the diagram its shape.

1. Guard selects one gateway. `select_gateway` reads the feature configuration, the request, and the Actor protocol version (`engine/packages/guard/src/routing/pegboard_gateway/gateway3_selector.rs:122-164`). `use_gateway3()` is true only for `Decision::SampledIn` (`:72-74`; `engine/packages/guard/src/routing/pegboard_gateway/mod.rs:762`). A WebSocket request is not `SampledIn`, so it stays on `PegboardGateway2`.
2. The gateway carries the request to the envoy through the tunnel. The envoy process hosts the user Actor. The user's type implements the `rivetkit::Actor` trait (`rivetkit-rust/packages/rivetkit/src/actor.rs:35`). The dispatch reaches that implementation, not the trait itself.
3. A sleeping Actor is woken with a workflow signal. Guard sends `pegboard::workflows::actor2::Wake` to the Actor workflow (`engine/packages/guard/src/routing/pegboard_gateway/mod.rs:531-560`). If the Actor stops during the wait, Guard sends the signal again, up to eight times (`:626-648`).
4. The runner path is separate. Guard sends `/runners/connect` to `PegboardRunnerWsCustomServe` (`engine/packages/guard/src/routing/runner.rs:28-47,50-60`). The TypeScript package `@rivetkit/engine-runner` implements the runner side: `Runner` (`rivetkit-typescript/packages/engine-runner/src/mod.ts:203`), protocol 7 (`:22,752`), and `Tunnel` (`rivetkit-typescript/packages/engine-runner/src/tunnel.ts:41`). In this commit, only `@rivetkit/engine-test-runner` imports this package (`engine/sdks/typescript/test-runner/src/index.ts:2-3`). The runner path is therefore test-only in this commit. The TypeScript SDK's own Actor host is the `rivetkit` `Registry`, which starts an envoy in serverful mode (`rivetkit-typescript/packages/rivetkit/src/registry/index.ts:627-662,951-982`). Serverful mode means a long-running process, as section 4.1 defines it.

Allocation in this diagram means the choice of a host for the Actor. The Actor workflow chooses an envoy through `allocate_serverful` (`engine/packages/pegboard/src/workflows/actor2/runtime.rs:247`; `engine/packages/pegboard/src/workflows/actor2/alloc_serverful/mod.rs:13-21`). The `PR` arrow is the legacy runner path. The legacy `actor` workflow allocates to a runner (`engine/packages/pegboard/src/workflows/actor/mod.rs:296,445`).

### 11.3 The build-metadata sequence

The sequence shows why `rivet-build-meta` is a separate crate. The participants are real: the environment, Cargo, the `rivet-build-meta` build script, the `rivet-build-meta` source, the `rivet-engine` binary, `rivet_config::Config`, and the `rivet_api_public::metadata::get_inner` function that reads the values.

```mermaid
sequenceDiagram
    participant Env as git and override environment
    participant Cargo as Cargo
    participant Build as rivet-build-meta build.rs
    participant Src as rivet-build-meta/src/lib.rs
    participant Bin as rivet-engine
    participant Cfg as rivet_config::Config
    participant Api as rivet_api_public::metadata::get_inner
    Env->>Cargo: new commit or changed override variable
    Cargo->>Build: rerun because a declared input changed
    Build->>Cargo: cargo:rerun-if-env-changed=OVERRIDE_GIT_SHA, OVERRIDE_BUILD_TIMESTAMP
    Build->>Cargo: cargo:rustc-env=VERGEN_GIT_SHA, VERGEN_BUILD_TIMESTAMP
    Cargo->>Src: compile env! constants
    Bin->>Src: build_meta() and compiled_runtime_protocols()
    Bin->>Cfg: Config::load(paths, BuildMeta, RuntimeProtocols)
    Api->>Cfg: Config::build_meta()
```

The mechanism has three steps that a reader must not merge. First, the build script runs before the compile. It prints `cargo:rerun-if-env-changed` for the two override variables, `OVERRIDE_GIT_SHA` and `OVERRIDE_BUILD_TIMESTAMP` (`engine/packages/build-meta/build.rs:15-16`). It prints `cargo:rustc-env` for each override value that is set (`:18-24`). The `VERGEN_*` names hold the git SHA, the build timestamp, the Rust compiler version, the compiler host triple, the Cargo target triple, and the Cargo debug flag (`engine/packages/build-meta/src/lib.rs:14-30`). Second, `rerun-if-env-changed` registers an input for the next build. If the variable changes, Cargo runs the build script again on the following build. `rustc-env` passes a value to the current compilation. Third, Cargo compiles `src/lib.rs` with the injected variables. The crate reads them with the `env!` macro (`engine/packages/build-meta/src/lib.rs:12-30`).

The binary then calls `build_meta()` and `compiled_runtime_protocols()` (`engine/packages/build-meta/src/lib.rs:42-76`). It passes both values to `Config::load` (`engine/packages/engine/src/main.rs:37-42`). The `Config` struct stores both (`engine/packages/config/src/lib.rs:31-38`). The other crates call `Config::build_meta()` and `Config::protocols()` and do not compile the build script. This commit does not use `OUT_DIR` or `include!` for these values. The values are compile-time environment constants.

### 11.4 The repository tree as a flowchart

The fourth diagram shows the repository as families. Each box is a directory group. This diagram is a directory map, not a call graph. The box labels are directory paths, which are the correct unit for a repository tree. An arrow between groups means that at least one member of the source group relates to at least one member of the target group. A solid arrow is a Cargo dependency or a pnpm dependency. A dashed arrow is a build task, which includes a generator, or a descriptive relation, or a deployment input. Each arrow carries its exact relation as a label.

```mermaid
flowchart TB
    subgraph Rust["Rust control plane"]
        ENGINE["engine/packages: 66 crates"]
        RSDK["engine/sdks/rust: 10 crates"]
        SCHEMAS["engine/sdks/schemas"]
    end
    subgraph Clients["Client SDKs"]
        TS["rivetkit-typescript: 18 packages"]
        TSCODECS["engine/sdks/typescript/envoy-protocol"]
        RS["rivetkit-rust: 8 crates"]
        PY["rivetkit-python"]
        SWIFT["rivetkit-swift"]
        GO["engine/sdks/go/api-full"]
        RUSTAPI["engine/sdks/rust/api-full"]
        TSAPI["engine/sdks/typescript/api-full"]
    end
    subgraph Contracts["Public contracts"]
        OPENAPI["engine/artifacts/openapi.json"]
        RIVETKIT_OPENAPI["rivetkit-openapi/openapi.json"]
        ASYNCAPI["rivetkit-asyncapi"]
        JSONSCHEMA["rivetkit-json-schema"]
    end
    subgraph Surfaces["Product surfaces"]
        FRONTEND["frontend"]
        DOCS["docs"]
        INTERNAL["docs-internal"]
        EXAMPLES["examples: 57 projects"]
        INTEGRATIONS["integrations: 2 packages"]
    end
    subgraph Delivery["Build and deployment"]
        SCRIPTS["scripts: 22 directories"]
        SELFHOST["self-host"]
        DOCKER["docker"]
        SHARED["shared/typescript/virtual-websocket"]
    end
    SCHEMAS -->|vbare_compiler| RSDK
    SCHEMAS -->|"@bare-ts tools"| TSCODECS
    RSDK -->|rivet-envoy-client| RS
    ENGINE -.->|api-public-openapi-gen| OPENAPI
    OPENAPI -.->|Fern| TSAPI
    OPENAPI -.->|Fern| GO
    OPENAPI -.->|OpenAPI Generator| RUSTAPI
    ASYNCAPI -.->|describes WebSocket protocol| TS
    JSONSCHEMA -.->|describes setup input| TS
    FRONTEND -.->|embedded build| ENGINE
    SELFHOST -.->|deployment input| ENGINE
    DOCKER -.->|image build| ENGINE
    SHARED -->|pnpm dependency| TS
    TS -.->|used by| EXAMPLES
    RS -.->|used by| EXAMPLES
    TS -.->|used by| INTEGRATIONS
    SCRIPTS -.->|code generation and release| ENGINE
    SCRIPTS -.->|code generation and release| TS
```

The supported arrows have these sources. `SCHEMAS --> RSDK` and `SCHEMAS --> TSCODECS` come from the `vbare_compiler` call and the `@bare-ts/tools` call in `engine/sdks/rust/envoy-protocol/build.rs:19-20,60-88`. `RSDK --> RS` is a Cargo edge: `rivetkit-rust/packages/rivetkit/Cargo.toml:39` lists `rivet-envoy-client`. `ENGINE -.-> OPENAPI` is the build script at `engine/packages/api-public-openapi-gen/build.rs:5-25`. The three `OPENAPI` arrows come from Fern (`engine/sdks/api/fern/generators.yml:9-25`) and OpenAPI Generator (`scripts/openapi/gen_rust.ts:9-10,40-50`). `SHARED --> TS` is a pnpm dependency: `rivetkit-typescript/packages/rivetkit/package.json:222` lists `@rivetkit/virtual-websocket`.

Five arrows are descriptive, not generators. `ASYNCAPI` describes the WebSocket protocol that the TypeScript SDK speaks. `JSONSCHEMA` describes the input to `setup()`, as section 6.5 states. The `FRONTEND`, `SELFHOST`, and `DOCKER` arrows supply a build or deployment input to the engine. The diagram has no arrow for `rivetkit-python` or `rivetkit-swift`, because this commit does not show a generated contract for them. `RIVETKIT_OPENAPI` also has no arrow, because section 6.3 does not verify its consumer.

The crate and package counts in the boxes are exact for this commit. The diagram leaves out many build edges, so the set of arrows is not a complete dependency inventory. `engine/sdks/rust` holds protocol crates and generated API clients (`engine/sdks/rust/envoy-protocol`, `engine/sdks/rust/api-full`, and `engine/sdks/rust/data`; sections 3.3, 3.15, and 6.8). It does not hold the user-facing Rust SDK. Section 5 covers `rivetkit-rust`, which is the user-facing Rust SDK.

## 12. Where to find each concern

The table gives the primary crate for each concern and the crates that a reader should open next. The last column names the document that explains the mechanism.

| Concern | Primary crate or directory | Also open | Document |
|---------|---------------------------|-----------|----------|
| Scheduling | `pegboard` (`engine/packages/pegboard`) | `pegboard-runner`, `pegboard-envoy`, `rivet-runner-protocol` | [04 Pegboard](04-pegboard-scheduling.md) |
| Durable execution | `gasoline` (`engine/packages/gasoline`) | `gasoline-runtime`, `workflow-worker`, `@rivetkit/workflow-engine` | [05 Gasoline](05-gasoline-durable-execution.md) |
| SQLite storage | `depot` (`engine/packages/depot`) | `rivet-depot-client`, `rivet-depot-client-types`, `rivet-depot-protocol` | [06 Depot](06-actor-sqlite-and-depot.md) |
| Key-value storage | `universaldb` (`engine/packages/universaldb`) | `rivet-universaldb-commit`, `universalpubsub` | [07 UniversalDB](07-universaldb-and-buckets.md) |
| Replication | `epoxy` (`engine/packages/epoxy`) | `epoxy-protocol`, `universaldb` | [08 Epoxy](08-epoxy-multi-region.md) |
| Public routing | `rivet-guard` (`engine/packages/guard`) | `rivet-guard-core`, `pegboard-gateway3` | [09 Guard](09-guard-routing.md) |
| Realtime and hibernation | `pegboard-envoy` (`engine/packages/pegboard-envoy`) | `pegboard-envoy` (`hibernating_requests`), `pegboard-gateway2` (`hibernation_task`) | [10 Realtime](10-realtime-and-hibernation.md) |
| Authentication | `rivet-auth-jwt` (`engine/packages/auth-jwt`) | `rivet-auth`, `rivet-auth-policy` | [13 Authentication](13-auth-and-namespaces.md) |
| Telemetry and logs | `rivet-telemetry` (`engine/packages/telemetry`) | `rivet-metrics`, `rivet-metrics-server`, `rivet-runtime`, `rivet-logs` | [14 Observability](14-observability-and-operations.md) |
| Deployment | `self-host` and `docker` | `scripts/docker`, `frontend/*.Dockerfile` | [15 Deployment](15-deployment-and-self-hosting.md) |
| Tests | `rivet-test-deps` (`engine/packages/test-deps`) | `rivet-test-deps-docker`, `test-snapshot-gen`, `rivet-test-envoy` | [16 Tests](16-tests-benchmarks-and-limits.md) |
| Public API | `rivet-api-public` (`engine/packages/api-public`) | `rivet-api-builder`, `rivet-api-types`, `rivet-api-util` | [02 The control plane](02-control-plane-architecture.md) |
| Internal API | `rivet-api-peer` (`engine/packages/api-peer`) | `rivet-api-types`, `depot` | [02 The control plane](02-control-plane-architecture.md) |
| TypeScript SDK | `rivetkit-typescript/packages/rivetkit` | `@rivetkit/react`, `@rivetkit/next-js`, `@rivetkit/rivetkit-napi` | [11 RivetKit TypeScript](11-rivetkit-typescript.md) |
| Rust SDK | `rivetkit-rust/packages/rivetkit` | `rivetkit-core`, `rivetkit-client`, the protocol crates | [12 RivetKit Rust](12-rivetkit-rust-and-clients.md) |
| Product family | `/Users/darkvoid/GitSources/src.Rivet/*` | the six product repositories and the website | [17 Product family](17-the-product-family.md) |

The table has one entry per concern, so it cannot show every crate. Section 3 has every crate. A reader who knows the concern but not the crate can start here. A reader who knows the crate but not the concern can use the section 3 tables.

## 13. The shape of the workspace

**Aha:** The workspace has many small crates because a few boundaries prevent large rebuilds and break dependency cycles. The rest of the split is a pattern that the source does not state as a rule.

The source states four boundary decisions:

1. `rivet-build-meta` is separate because its build script changes on every commit. `engine/CLAUDE.md:72` and `engine/packages/build-meta/src/lib.rs:3-6` state this. The cost of a merge into `rivet-config` is a full rebuild on every commit.
2. Runtime protocol versions live in `rivet-build-meta`, not `rivet-config`, because a protocol bump would otherwise rebuild every crate that depends on the configuration crate. `engine/CLAUDE.md:77` states this.
3. `rivet-api-types` is separate because `rivet-api-peer` and `pegboard` would form a cycle without it. `docs-internal/engine/API.md` states this. The manifests match the premise of the note: both sides need the shared types. They do not complete a Cargo cycle, because `pegboard` does not list `rivet-api-peer` in this commit (section 3.3).
4. The `rivetkit` SDK stays a thin wrapper over `rivetkit-core`. `docs-internal/engine/bare-protocol-crates.md` states this.

The source does not state a general rule for the other crates. The rest of the split is visible in the dependency edges, but the reason is an inference. This map marks it as an inference.

What the edges show:

- `rivet-config`, `rivet-metrics`, `rivet-util`, and `rivet-error` have much more fan-in than fan-out. Their fan-in is 40, 25, 24, and 24 (section 3.20). Their dependency tables show the other side. `rivet-config` lists no workspace crate (`engine/packages/config/Cargo.toml:9-26`). `rivet-metrics` lists two external crates (`engine/packages/metrics/Cargo.toml:11-13`). `rivet-util` lists five workspace crates (`engine/packages/util/Cargo.toml:25-29`). `rivet-error` lists one workspace crate, `rivet-error-macros` (`engine/packages/error/Cargo.toml:11-16`). The comparison is an inference from these edges, not a source-stated ranking. A change to one of them therefore rebuilds much of the tree.
- `rivet-engine` and `rivet-guard` have high fan-out. They are composition roots. They wire the layers together and hold little reusable logic.
- Most protocol crates have almost no workspace dependencies. They depend on `vbare`, `serde`, and `serde_bare`. `serde_bare` is an external crate from `https://github.com/rivet-dev/serde_bare` that adds BARE serialization support to Serde (`Cargo.toml:413-414`). A codec change rebuilds the codec and its clients, not the engine.
- The three gateway crates show a different pattern. `pegboard-gateway`, `pegboard-gateway2`, and `pegboard-gateway3` are separate crates, and `rivet-guard` depends on all three. This split preserves three implementations at the same time. This map reads it as a compatibility boundary, not a build boundary.

What the boundaries buy:

| Benefit | Evidence |
|---------|----------|
| A protocol bump does not rebuild the configuration consumers. | `engine/CLAUDE.md:77`; the compiled versions are assembled in `rivet-build-meta`, not in `rivet-config`. |
| A commit does not rebuild the workspace. | `engine/CLAUDE.md:72`; only the binary and its exclusive dependents use `rivet-build-meta`. |
| A client can use the API types without the peer server crate. | `rivet-api-types` depends on `rivet-api-builder`, not on `rivet-api-peer`. |
| A user of the Rust client does not compile the `rivetkit-core` runtime. | `rivetkit-client` does not list `rivetkit-core` in its manifest. |
| A codec change is local to the codec and its consumers. | The generated protocol crates under `engine/sdks/rust` depend on `vbare` and on each other, not on the engine services. |
| The test harness has its own crates. | `rivet-test-deps` holds the harness and `rivet-test-deps-docker` holds the container variant. |

What the boundaries cost:

| Cost | Evidence |
|------|----------|
| 88 workspace members and 76 engine crates to name and track. | `Cargo.toml:3-92`. |
| Alias names such as `gas` and `util` hide the package name. | Section 3.20. |
| Constants must live in one shared crate. | `SHARD_SIZE`, `COMMIT_SEGMENT_MAX_SHARDS`, and `MAX_COMMIT_DIRTY_PAGES` in `rivet-depot-client-types` (section 3.7). |
| A shared helper crate becomes a large mixed crate. | `rivet-util` has 17 public modules. |
| A test helper is in the production dependency graph. | `gasoline` depends on `rivet-test-deps`. |
| Three gateway crates stay in the tree at once. | `rivet-guard` depends on all three. |
| A crate boundary does not stop a semantic cycle. | The `rivet-api-peer` and `pegboard` cycle needed a new crate, not a refactor. |

The obvious alternative is one large engine crate. That alternative removes the alias names, the 88-member list, and the shared constant crate. It also makes the build-metadata problem worse: a commit would rebuild the whole crate. It removes the compiler-enforced boundary between the public API and the peer API. That boundary is the separate compilation units and the one-way `rivet-api-public` -> `rivet-api-peer` edge (`engine/packages/api-public/Cargo.toml:22`). In one crate, public API code could reference peer-only code with no manifest edge to review. It makes a protocol bump rebuild the engine. The source does not state that the team considered this alternative. The map compares the costs from the observed edges, not from a design record.

A second alternative is a few medium crates: one for the API, one for scheduling, one for storage. That alternative keeps some build isolation. It still needs `rivet-build-meta` and the protocol crates as separate units, because the source states those reasons. It would merge the three gateways into one crate with a mode flag. The source does not state why the gateways stayed separate. This map does not invent a reason.

The shape is therefore a mix. A small number of boundaries have a stated reason. A larger number follow the same pattern: a crate per protocol, a crate per subsystem, and a small crate for a shared type. The pattern is consistent with the stated reasons. The source does not state that the pattern is a policy. A reader should treat the four source-stated decisions as facts and the rest as a reasonable reading of the manifests.

## What to read next

- [02 The control plane](02-control-plane-architecture.md) explains how the `rivet-engine` binary starts the services that section 3.8 lists.
- [04 Pegboard scheduling](04-pegboard-scheduling.md) explains the scheduling workflows that section 3.13 lists, including the legacy runner path and the newer envoy path.
- [17 The product family](17-the-product-family.md) is the deep dive on the six sibling repositories and the website.

### Glossary for this document

| Term | Meaning in this map |
|------|---------------------|
| Actor | A keyed durable process with SQLite, a queue, realtime connections, and a schedule. The name takes a capital A. |
| control plane | The engine binary and the services that schedule, route, and store Actor work. The name is lower case. |
| datacenter | A location that can host Actors and their durable store. The engine keeps one record per datacenter and refreshes it with a ping workflow (`engine/packages/datacenter/src/lib.rs:7-13`). |
| namespace | A named scope with its own identifier. Actors, runner configurations, and store keys carry a `namespace_id` (`engine/packages/types/src/namespaces.rs:5-10`). |
| bucket | The object storage service that holds durable data. The product description calls it the only external durable store. The open-source build uses RocksDB or PostgreSQL, and the cold tier to the bucket is Enterprise-only. |
| runner | The legacy host process that runs an Actor on a node. The current host process is the envoy. |
| envoy | The current host process. It connects to the engine through a gateway tunnel. |
| gateway | The engine-side WebSocket endpoint that terminates the tunnel from an envoy or runner. Three generations exist: `pegboard-gateway`, `pegboard-gateway2`, and `pegboard-gateway3` (`engine/packages/pegboard-gateway3/src/lib.rs:71`). Guard routes to it. |
| tunnel | The Universal PubSub channel that carries request and response frames between a gateway and an envoy or runner (`docs-internal/engine/PEGBOARD_TUNNEL_RETRIES.md:5`). |
| hibernation | A gateway2 behavior that keeps the client WebSocket open while the Actor is unloaded, and wakes the Actor on the next message (`engine/packages/pegboard-gateway2/src/hibernation_task.rs:22-33,105-118`). |
| Gasoline | The engine's internal Rust durable-execution engine for the control plane. It records each workflow step as a history event (`engine/packages/gasoline/src/history/event.rs:10-19`). |
| Epoxy | The geo-distributed strongly consistent key-value store. It runs single-decree Fast Paxos independently per key (`engine/packages/epoxy/README.md:3`). |
| UniversalDB | The transactional key-value layer over a configured driver, RocksDB or PostgreSQL. Depot and Epoxy write through it (section 3.15). |
| subspace | A key prefix that groups records in UniversalDB. `Subspace::new` builds the prefix from a tuple (`engine/packages/universaldb/src/utils/subspace.rs:11-23`). |
| protocol version | The integer that names the wire schema version that a process speaks. Most protocol crates export a `PROTOCOL_VERSION` constant and re-export their latest `vN` module (`engine/sdks/rust/envoy-protocol/src/lib.rs:5-13`). `rivetkit-actor-persist` names its constant `CURRENT_VERSION` instead (`rivetkit-rust/packages/actor-persist/src/lib.rs:7`). |
| cold tier | The long-term object-storage copy of aged data. The enterprise build writes it, and the open-source build leaves it empty (`docs-internal/engine/oss-to-ee-upgrade.md:9-19`). |
| depot | The storage subsystem that holds the SQLite pages for each Actor in a log-structured form. |
| workspace member | A directory listed in `Cargo.toml:3-92`. There are 88. |
| protocol crate | A crate that holds generated, versioned wire types. Most of the crates under `engine/sdks/rust` are protocol crates. The directory also holds a generated client, a test harness, and an envoy transport. |
| vbare | The schema tool that generates the versioned protocol types. |
| pnpm workspace | The JavaScript workspace manager that links the TypeScript packages. |
| composition root | A crate that wires other crates together and holds little reusable logic. `rivet-engine` and `rivet-guard` are examples. |
