---
title: 15 Deployment and self-hosting
---

# 15 Deployment and self-hosting

In the documented model, a Rivet cluster is a set of identical `rivet-engine`
processes, one persistent disk for each process, one object storage bucket for
the cluster, and the node processes that run Actors. The documented model has no
separate database and no message bus. The open-source build in this commit is
different: UniversalDB runs on RocksDB for one instance, or PostgreSQL plus NATS
for several instances. This document shows the documented model and the
open-source requirement as separate things. It covers the minimum production
topology, the node side, the three delivery shapes, the upgrade order, and the
recovery procedure for each part.

The source of truth is the core repository at commit
`4a58441c50d1ce20d8803cfb1ab7c3285a5141d6`, version 2.3.31. A citation has the
form `path:line` or `path:line-line`. All paths are relative to
`/Users/darkvoid/GitSources/src.Rivet/rivet`.

## 1. The deployment model

### 1.1 The four parts

The project README states the model in one paragraph. It names four parts.

```markdown
# README.md:218
**Every option runs the same binary. The only difference is who operates it.** Self-hosting means running four things: identical control plane instances (one to try it, three or more in production), a persistent disk per instance that holds recent writes and the instance's identity, one bucket per cluster on S3, Google Cloud Storage, Azure Blob Storage, or any S3-compatible store, and the nodes that run your Actors. There is no database to run and no message bus. Local dev matches production: the same binary, APIs, and control-plane behavior from laptop to production.
```

The four parts have different jobs.

| Part | Job | State it holds | Source |
|------|-----|----------------|--------|
| `rivet-engine` instances | Schedule Actors, route requests, run workflows | Recent writes and cluster metadata on the local disk | `README.md:218` |
| Persistent disk per instance | Keep recent writes and the version floor | UniversalDB rows in RocksDB (open-source build) | `engine/packages/config/src/config/db/mod.rs:23-36` |
| Object storage bucket | Hold the durable copy that survives a lost instance | Actor pages and cold data | `README.md:218`, `README.md:262` |
| Node processes | Run the Actor code inside your application process | Actor memory only | `docs/general/content/nodes.mdx:7` |

The word "database" has two senses in this document. In the README quote it means
a separate database server that the operator must run. Rivet does not need one in
the documented model. In the open-source build, the engine still writes its own
rows to a local database file, which is RocksDB by default
(`engine/packages/config/src/config/db/mod.rs:17-20`). That local file lives on
the instance disk. Section 12.4 treats the disk as a failure point for this
reason.

Three services appear in the diagram. `guard` is the public listener on port
6420. It accepts client requests and node connections. `api_peer` is the private
listener on port 6421. It serves the health route and the calls that one
datacenter makes to another. A node runs the SDK. On the envoy path the SDK
contains `envoy-client`, which opens the outbound WebSocket
(`engine/packages/config/src/defaults.rs:1-14`).

```mermaid
flowchart TB
    subgraph Z1["Zone A"]
        E1["rivet-engine<br/>guard :6420<br/>api_peer :6421"]
        N1["node process<br/>envoy-client"]
    end
    subgraph Z2["Zone B"]
        E2["rivet-engine<br/>guard :6420<br/>api_peer :6421"]
        N2["node process<br/>envoy-client"]
    end
    subgraph Z3["Zone C"]
        E3["rivet-engine<br/>guard :6420<br/>api_peer :6421"]
        N3["node process<br/>envoy-client"]
    end
    LB["load balancer<br/>HTTPS :443"]
    B[("bucket<br/>one per cluster")]
    LB -->|"client HTTP and WebSocket"| E1
    LB --> E2
    LB --> E3
    N1 -->|"outbound WebSocket /envoys/connect"| E1
    N2 -->|"outbound WebSocket /envoys/connect"| E2
    N3 -->|"outbound WebSocket /envoys/connect"| E3
    N2 -.->|"same endpoint, through the load balancer"| LB
    E1 -.->|"api_peer peer_url"| E2
    E2 -.->|"api_peer peer_url"| E3
    E1 --- B
    E2 --- B
    E3 --- B
```

A client can reach any instance through the load balancer. The Actor may be
connected to a different instance. For Actor traffic, Guard does not forward the
request to the instance that owns the node. It publishes the request to the
Universal Pub/Sub topic `pegboard.envoy.{namespace_id}.{envoy_key}`, and the
instance that holds the node WebSocket consumes it
(`engine/packages/pegboard/src/pubsub_subjects.rs:113-138`). The reply returns on
`pegboard.gateway.{gateway_id}` (`engine/packages/pegboard/src/pubsub_subjects.rs:86-111`).
This is why a multi-instance cluster needs a pub/sub service, and why the
open-source Kubernetes guide deploys NATS. The dashed `peer_url` edges carry
Epoxy and namespace calls between datacenters. They do not carry Actor traffic.

This is the minimum production topology in the project benchmark: three
instances, one per zone, and one bucket in the same region (`README.md:155`,
`docs/general/content/benchmarking.mdx:13-18`). The bucket is the shared store
in the documented model. In this open-source commit, the same three-instance
shape needs PostgreSQL and NATS instead (section 1.3).

Three words need exact definitions.

| Word | Meaning in this document | Source |
|------|--------------------------|--------|
| Instance | One `rivet-engine` process. It holds all services. | `engine/packages/engine/src/run_config.rs:4-70` |
| Zone | A cloud failure domain. Two zones do not fail together. | Not in the source; this is the cloud provider meaning |
| Datacenter | One entry in `topology.datacenters`, with a label and a peer address | `engine/packages/config/src/config/topology.rs:155-179` |

A production cluster places one instance in each zone. Each instance sets a
different `topology.datacenter_label`, and each label has one entry in
`topology.datacenters`. The Epoxy replica identifier is the datacenter label
itself: `epoxy_replica_id()` returns `dc_label()`
(`engine/packages/config/src/config/mod.rs:345-353`). Three labels therefore
mean three Epoxy replicas.

Two instances with the same label are not two replicas. They share one replica
identity. The Kubernetes sample does this: it runs two engine replicas, and both
use `"datacenter_label": 1`
(`self-host/control-plane/kubernetes/02-engine-configmap.yaml:15-27`). That shape
scales the request path but does not add an Epoxy voter. A Horizontal Pod
Autoscaler replica is an instance, not a datacenter.

The word “identical” is literal. Every instance runs the same binary and the
same service set. The services are defined once in `run_config.rs`.

```rust
// engine/packages/engine/src/run_config.rs:4-70
pub fn config(_rivet_config: rivet_config::Config) -> Result<RunConfigData> {
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
		// Core services
		Service::new(
			"dynamic_config",
			ServiceKind::Core,
			|config, pools| Box::pin(rivet_dynamic_config::start(config, pools)),
			false,
		),
		Service::new(
			"version_management",
			ServiceKind::Core,
			|config, pools| Box::pin(rivet_version_management::start(config, pools)),
			false,
		),
		Service::new(
			"epoxy_protocol_version",
			ServiceKind::Core,
			|config, pools| Box::pin(epoxy::protocol_version::start(config, pools)),
			false,
		),
		Service::new(
			"cache_purge",
			ServiceKind::Core,
			|config, pools| Box::pin(rivet_cache_purge::start(config, pools)),
			false,
		),
		Service::new(
			"profiling",
			ServiceKind::Core,
			|config, pools| Box::pin(rivet_profiling::start(config, pools)),
			false,
		),
	];

	Ok(RunConfigData { services })
}
```

The `requires_graceful_shutdown` flag is the last argument of `Service::new`.
`guard`, `workflow_worker`, and `pegboard_outbound` set it to `true`. The
service manager uses that flag during shutdown.

Every instance runs every service. Three of them need a note.

`bootstrap` is an `Oneshot` service, so it runs once per process at start and
again only if it fails (`engine/packages/service-manager/src/lib.rs:207-238`).
It is safe on all three instances. The setup steps create records only when they
are absent, and the coordinator step returns early when the current datacenter is
not the leader (`engine/packages/bootstrap/src/lib.rs:17-66`).

`workflow_worker` runs on every instance. Each worker pulls tasks from the shared
workflow store (`engine/packages/workflow-worker/src/lib.rs:4-13`).
`pegboard_outbound` runs on every instance and serves serverless pools that have
an external endpoint.

### 1.2 What replaces the database and the message bus

The documented model does not run PostgreSQL or NATS. The README says that
UniversalDB replaces the database, that the write path plus the bucket replaces
the message bus, and that instances find each other through the bucket
(`README.md:218`, `README.md:262`).

| Removed system | Replacement in the documented model | Code evidence |
|----------------|--------------------------------------|---------------|
| Relational database | UniversalDB over object storage | `README.md:262`; `docs-internal/engine/oss-to-ee-upgrade.md:9-19` |
| Message bus | UniversalDB transactions and a durable write log | `README.md:262` |
| Service discovery | The bucket: instances find each other through it | `README.md:262` |

The topology configuration is a separate thing. It does not discover instances.
It lists the datacenters, their labels, their public and peer addresses, and
which one is the leader (`engine/packages/config/src/config/topology.rs:155-179`).
An operator writes it by hand. Epoxy uses it to address replicas. It is not a
service registry.

The open-source tree also has two optional systems. PostgreSQL can replace the
local file system database. NATS can carry UniversalDB commit traffic between
instances. Both are selectable in the configuration.

```rust
// engine/packages/config/src/config/db/mod.rs:10-37
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Database {
	Postgres(Postgres),
	FileSystem(FileSystem),
}

impl Default for Database {
	fn default() -> Self {
		Self::FileSystem(FileSystem::default())
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FileSystem {
	pub path: PathBuf,
}

impl Default for FileSystem {
	fn default() -> Self {
		let default_path = dirs::data_local_dir()
			.map(|dir| dir.join("rivet-engine").join("db"))
			.unwrap_or_else(|| PathBuf::from("./data/db"));

		Self { path: default_path }
	}
}
```

The default is a local RocksDB database. It is not a bucket. This is the first
and most important difference between the documented model and the open-source
build.

The two systems are not equivalent, and NATS has two jobs.

| Question | Answer | Source |
|----------|--------|--------|
| Is NATS in the configuration schema? | Yes, as `PubSub::Nats` | `engine/packages/config/src/config/pubsub.rs:6-17` |
| Where does the pub/sub block go? | At the top level, as `"nats": {...}`, because the enum is flattened | `engine/packages/config/src/config/mod.rs:97-98` |
| Is it required for one instance? | No. The default pub/sub is `Memory` | `engine/packages/config/src/config/pubsub.rs:13-17` |
| Is it required for several instances? | Yes in practice. The request routing topics and the UniversalDB commit transport need a shared bus | `engine/packages/pegboard/src/pubsub_subjects.rs:113-138`; `engine/packages/config/src/config/db/postgres.rs:53-60` |
| Where does UniversalDB read the NATS block? | `database.postgres.nats`. If absent, the loader copies the top-level `nats` block into it | `engine/packages/config/src/config/db/postgres.rs:53-60`; `engine/packages/config/src/config/mod.rs:268-276` |
| What happens without it on PostgreSQL? | UniversalDB runs single-node, so two instances do not share commit transport | `engine/packages/config/src/config/db/postgres.rs:53-56` |

One `"nats"` block therefore drives both jobs. The Kubernetes ConfigMap uses that
shape: a top-level `"nats"` key and a `"postgres"` key
(`self-host/control-plane/kubernetes/02-engine-configmap.yaml:4-13`).

**Aha:** The documented model moves the durable database into the bucket, so
the bucket is not a backup. It is the database. The local disk is a cache and a
write buffer. An operator who treats the bucket as an optional archive will
build a cluster that loses data when an instance disk fails. The obvious
alternative is a conventional database cluster. That alternative costs at least
one more stateful system to operate, and the project rejects it for that reason.
The source does not state that reason in code. The README states only the result:
“There is no database to run and no message bus” (`README.md:218`).

### 1.3 The product claim and the open-source requirement

Two statements are true at the same time. Do not merge them.

| Class | Statement | Source | True in this open-source commit? |
|-------|-----------|--------|----------------------------------|
| Product claim | One bucket per cluster, no database, no message bus | `README.md:218`, `README.md:262` | No |
| Product claim | The bucket holds the cold tier | `README.md:262` | No, the cold tier is Enterprise-only |
| Open-source requirement | UniversalDB runs on RocksDB or PostgreSQL | `engine/packages/config/src/config/db/mod.rs:10-21`, `engine/packages/universaldb/src/driver/mod.rs:14-18` | Yes |
| Open-source requirement | Multiple instances share PostgreSQL and use NATS | `engine/packages/config/src/config/db/postgres.rs:53-60`, `engine/packages/pools/src/db/udb.rs:21-56` | Yes |
| Open-source requirement | One instance can use a local file system | `README.md:262`, `self-host/control-plane/vm/commands.sh:5-10` | Yes |
| Enterprise requirement | A bucket-only cluster needs the Enterprise build | `docs-internal/engine/oss-to-ee-upgrade.md:9-19` | Not in this repository |

The UniversalDB driver module names the two open-source drivers.

```rust
// engine/packages/universaldb/src/driver/mod.rs:14-18
pub mod postgres;
pub mod rocksdb;

pub use postgres::PostgresDatabaseDriver;
pub use rocksdb::RocksDbDatabaseDriver;
```

The self-host procedure in this repository follows the open-source requirement.
The Docker Compose guide sets `RIVET__FILE_SYSTEM__PATH` and runs one engine with
a local volume (`self-host/control-plane/docker-compose/docker-compose.yml:9-13`).
The Kubernetes guide deploys PostgreSQL and a two-node NATS cluster
(`self-host/control-plane/kubernetes/README.md:3-5`). Neither guide uses the
bucket-only shape.

For one open-source instance, `FileSystem` works. For more than one open-source
instance, the operator must use `Postgres` and `NATS`, because the instances must
share the same UniversalDB. For the bucket-only cluster, the operator needs the
Enterprise build, which is not in this repository.

The rest of this document states which path a claim belongs to: the legacy
runner path, the newer envoy path, the open-source storage path, or the
documented Enterprise path.

## 2. The bucket

### 2.1 Supported providers

The README names six provider families. The list is documentation-level. The
open-source engine has no bucket configuration for any of them. It has only
S3-named compaction constants.

| Provider | Named at | Evidence in code |
|----------|----------|------------------|
| Amazon Simple Storage Service (S3) | `README.md:218`, `README.md:262` | Only `CMP_S3_*` compaction constants (`engine/packages/depot/src/conveyer/constants.rs:190-200`) |
| Google Cloud Storage (GCS) | `README.md:218`, `README.md:262` | None in the open-source tree |
| Azure Blob Storage | `README.md:218`, `README.md:262` | None in the open-source tree |
| MinIO | `README.md:262` | None in the open-source tree |
| Cloudflare R2 | `README.md:262` | None in the open-source tree |
| Hetzner Object Storage | `README.md:262` | None in the open-source tree |

The benchmark documentation names Azure Blob Storage for the published numbers.
It says the bucket is in the same region as the control plane
(`docs/general/content/benchmarking.mdx:18`).

The only object-storage constants in the open-source tree are Depot compaction
limits. They describe an S3-compatible interface with object upload and delete.

```rust
// engine/packages/depot/src/conveyer/constants.rs:190-200
/// Workflow compaction uploads a complete shard set for one hot-fold boundary per cold pass. A
/// boundary can touch multiple shards, so this caps the object count high enough that
/// `CMP_S3_UPLOAD_LIMIT_BYTES` is the binding limit. `64 MiB / (SHARD_SIZE * PAGE_SIZE)` worth of
/// max-size shards fit under the byte budget, so 256 objects covers any boundary within it.
pub const CMP_S3_UPLOAD_MAX_OBJECTS: usize = 256;

/// Workflow compaction caps cold shard upload activity payloads.
pub const CMP_S3_UPLOAD_LIMIT_BYTES: usize = 64 * 1024 * 1024;

/// Workflow compaction caps S3 delete activity batches.
pub const CMP_S3_DELETE_MAX_OBJECTS: usize = 100;
```

These constants prove that the compaction code plans S3 uploads and deletes.
They do not prove that the open-source build enables the cold tier. The internal
upgrade document says it does not.

### 2.2 What the code can configure

The open-source configuration has no `bucket`, `s3`, `gcs`, or `azure` section.
The root struct lists every section. Unknown keys are rejected.

```rust
// engine/packages/config/src/config/mod.rs:70-123
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Root {
	#[serde(default)]
	pub auth: Option<Auth>,

	#[serde(default)]
	pub guard: Option<Guard>,

	#[serde(default)]
	pub api_peer: Option<ApiPeer>,

	#[serde(default)]
	pub pegboard: Option<Pegboard>,

	#[serde(default)]
	pub features: Option<Features>,

	#[serde(default)]
	pub logs: Option<Logs>,

	#[serde(default)]
	pub topology: Option<Topology>,

	#[serde(default, flatten)]
	pub database: Option<Database>,

	#[serde(default, flatten)]
	pub pubsub: Option<PubSub>,

	#[serde(default)]
	pub cache: Option<Cache>,

	#[serde(default)]
	pub clickhouse: Option<ClickHouse>,

	#[serde(default)]
	pub telemetry: Telemetry,

	#[serde(default)]
	pub runtime: Runtime,

	#[serde(default)]
	pub sqlite: Option<Sqlite>,

	#[serde(default)]
	pub metrics: Metrics,

	#[serde(default)]
	pub pyroscope: Option<Pyroscope>,

	#[serde(default)]
	pub outbound: Option<Outbound>,
}
```

An operator cannot point this build at a bucket by adding a key. The
`deny_unknown_fields` attribute rejects the key at load time. The internal
upgrade document says the same thing about Enterprise-only keys: “An EE config
does not load on OSS” (`docs-internal/engine/oss-to-ee-upgrade.md:60-62`).

The real open-source configuration for a single instance is one JSON block.

```jsonc
// self-host/control-plane/docker-compose/rivet-engine/config.jsonc:1-5
{
	"auth": {
		"admin_token": "dev"
	}
}
```

The Docker Compose file moves the local database onto a named volume. It does
this with an environment variable, not a bucket key.

```yaml
# self-host/control-plane/docker-compose/docker-compose.yml:1-25
services:
  rivet-engine:
    build:
      context: ../../..
      dockerfile: docker/engine/Dockerfile
      target: engine-full
    restart: unless-stopped
    command: /usr/bin/rivet-engine start
    environment:
      - RIVET__FILE_SYSTEM__PATH=/var/lib/rivet-engine
    volumes:
      - ./rivet-engine/config.jsonc:/etc/rivet/config.jsonc:ro
      - rivet-engine-data:/var/lib/rivet-engine
    ports:
      - '6420:6420'
    healthcheck:
      test:
        - CMD
        - curl
        - '-f'
        - http://127.0.0.1:6421/health
      interval: 2s
      timeout: 10s
      retries: 10
      start_period: 30s
```

### 2.3 Consistency and permissions

The source does not state a consistency requirement for the bucket. The source
does not state an Identity and Access Management (IAM) policy for the bucket.
The internal document states only that Enterprise reads and writes cold rows and
that open source keeps the schema (`docs-internal/engine/oss-to-ee-upgrade.md:9-19`).

The compaction constants show two bucket operations: upload an object and delete
an object. The grace constant also names an "unpublish" step, but the source does
not show that step as a bucket call. A bucket policy must permit the upload and
the delete. The repository does not give a policy document and does not state
whether a list permission is needed.

**Unverified:** the repository does not state the required bucket consistency
model. Do not assume read-after-write, list-after-write, or cross-region
replication from this source. The benchmark uses a bucket in the same region as
the control plane (`docs/general/content/benchmarking.mdx:18`). That is a
performance choice, not a consistency statement.

## 3. The instance disk

### 3.1 What is stored locally

The local database is a RocksDB store. The driver opens it at the configured
path. The default path is the user data directory plus `rivet-engine/db`, or
`./data/db` when the user data directory is unavailable
(`engine/packages/config/src/config/db/mod.rs:29-36`).

```rust
// engine/packages/pools/src/db/udb.rs:19-68
#[tracing::instrument(skip(config))]
pub async fn setup(config: &Config) -> Result<Option<UdbPool>> {
	let db_driver = match config.database() {
		config::Database::Postgres(pg) => {
			// A NATS config (set directly or inherited from the UPS config in
			// `validate_and_set_defaults`) selects UniversalDB multi-node mode.
			let nats = pg
				.nats
				.as_ref()
				.map(|nats| universaldb::driver::postgres::NatsConfig {
					addresses: nats.addresses.clone(),
					username: nats.username.clone(),
					password: nats.password.as_ref().map(|p| p.read().clone()),
					client_capacity: nats.client_capacity,
					subscription_capacity: nats.subscription_capacity,
				});

			let postgres_config = universaldb::driver::postgres::PostgresConfig {
				connection_string: pg.url.read().clone(),
				ssl_config: pg.ssl.as_ref().map(|ssl| {
					universaldb::driver::postgres::PostgresSslConfig {
						ssl_root_cert_path: ssl.root_cert_path.clone(),
						ssl_client_cert_path: ssl.client_cert_path.clone(),
						ssl_client_key_path: ssl.client_key_path.clone(),
					}
				}),
				nats,
				pool_max_size: None,
			};

			Arc::new(
				universaldb::driver::PostgresDatabaseDriver::new_with_config(
					config.clone(),
					postgres_config,
				)
				.await?,
			) as universaldb::DatabaseDriverHandle
		}
		config::Database::FileSystem(fs) => {
			Arc::new(universaldb::driver::RocksDbDatabaseDriver::new(fs.path.clone()).await?)
				as universaldb::DatabaseDriverHandle
		}
	};

	tracing::debug!("udb started");

	let db = universaldb::Database::new(db_driver).with_throttle(throttle_config(config));

	Ok(Some(UdbPool { db }))
}
```

UniversalDB stores every durable row that the control plane owns. This includes
Actor metadata, runner records, workflow state, and the version keys in
section 8. The user-facing Actor Structured Query Language (SQLite) pages go through Depot, which also uses
UniversalDB as its key space. Document
[07 UniversalDB and buckets](07-universaldb-and-buckets.md) covers that layout.

The SQLite page cache and the Depot rows live in the same UniversalDB store. On
the `FileSystem` backend, that store is the local disk. Depot reads and writes
pages through UniversalDB. The compaction code moves hot pages toward the cold
tier when that tier exists.

### 3.2 Why it must persist

A restart with the same disk continues the same cluster state. A restart with an
empty disk starts a new database. For the `FileSystem` backend, the disk is the
only copy of the cluster key space. Loss of that disk is loss of the cluster,
unless an earlier copy exists.

This is why the deployment guides mount a volume. The Virtual Machine guide uses
a named Docker volume and points the file system path at it.

```bash
# self-host/control-plane/vm/commands.sh:1-12
#!/usr/bin/env bash
# Commands from the guide, in order.
set -euo pipefail

docker run -d \
  --name rivet-engine \
  -p 6420:6420 \
  -v rivet-data:/data \
  -e RIVET__FILE_SYSTEM__PATH="/data" \
  rivetdev/engine:latest

curl http://localhost:6420/health
```

The configuration option is `file_system.path`. The environment form is
`RIVET__FILE_SYSTEM__PATH`, because the loader uses the prefix `RIVET`, the
separator `__`, and the list separator `,`
(`engine/packages/config/src/lib.rs:73-80`).

```rust
// engine/packages/config/src/lib.rs:55-96
		let mut settings = config_loader::Config::builder();

		// Start with default values
		settings = settings.add_source(config_loader::Config::try_from(&config::Root::default())?);

		if paths.is_empty() {
			let default_path = paths::system_config_dir();
			if default_path.exists() {
				// Add default config directory if it exists
				settings = add_source(settings, default_path)?;
			}
		} else {
			// Use provided paths
			for path in paths {
				settings = add_source(settings, path)?;
			}
		}

		// Add env source for overrides
		settings = settings.add_source(
			config_loader::Environment::with_prefix("RIVET")
				.try_parsing(true)
				.separator("__")
				.list_separator(",")
				.with_list_parse_key("foundationdb.addresses"),
		);

		// Read config
		let mut config_root = settings
			.build()
			.context("failed to build config")?
			.try_deserialize::<config::Root>()
			.context("failed to deserialize config")?;

		// Validate configuration at load time
		config_root.validate_and_set_defaults()?;

		Ok(Self::from_root_with_build_meta(
			config_root,
			build_meta,
			protocols,
		))
```

The default config directory is `/etc/rivet` on Linux,
`/Library/Application Support/rivet` on macOS, and `C:\ProgramData\rivet` on
Windows (`engine/packages/config/src/paths.rs:6-19`). The default directory is
read only when no `--config` path is given.

**Aha:** The Kubernetes manifest sets `RIVET_CONFIG_PATH`, but this commit does
not read that variable. A search of the whole repository finds only the
manifest. The manifest still works because it mounts the file at `/etc/rivet`,
which is the default config directory. An operator who copies the environment
variable pattern but mounts the file elsewhere will start the engine with
defaults and an empty authentication block. The engine then fails at startup,
because `auth.admin_token` has no default and must not be empty
(`engine/packages/config/src/config/mod.rs:260-266`,
`engine/packages/config/src/config/auth.rs:23-45`). The manifest avoids that
failure for a second reason as well: it passes `RIVET__AUTH__ADMIN_TOKEN` from a
secret (`self-host/control-plane/kubernetes/03-rivet-engine-deployment.yaml:32-36`).

### 3.3 The identity claim

The README says the persistent disk holds “the instance's identity”
(`README.md:218`). The open-source code does not write a per-instance identity
file. It creates a new random `NodeId` on every start
(`engine/packages/pools/src/pools.rs:24-45`).

```rust
// engine/packages/pools/src/pools.rs:22-53
impl Pools {
	#[tracing::instrument(skip(config))]
	pub async fn new(config: Config) -> Result<Pools> {
		// TODO: Choose client name for this service
		let client_name = "rivet";
		let token = CancellationToken::new();
		let node_id = NodeId::new();

		install_rustls_provider();

		let (ups, udb) = tokio::try_join!(
			crate::db::ups::setup(&config, client_name),
			crate::db::udb::setup(&config),
		)?;
		let clickhouse = crate::db::clickhouse::setup(&config)?;

		let pool = Pools(Arc::new(PoolsInner {
			config,
			node_id,
			_guard: token.clone().drop_guard(),
			ups: Some(ups),
			clickhouse,
			udb,
		}));

		// Initialize here to avoid cold starts elsewhere
		crate::reqwest::client().await?;
		crate::reqwest::guarded_client(pool.config()).await?;
		crate::reqwest::guarded_client_no_timeout(pool.config()).await?;

		Ok(pool)
	}
```

The datacenter label is not disk state. It is configuration: the operator sets
`topology.datacenter_label`, and `epoxy_replica_id()` returns that value
(`engine/packages/config/src/config/mod.rs:345-353`). A replacement instance must
receive the same label.

The cluster-wide version floor is different. It is `EngineVersionKey` in
UniversalDB (`engine/packages/version-management/src/keys.rs:4-33`). On the
`FileSystem` backend, the disk holds that row. If the disk is lost, the floor is
lost unless the disk is restored.

For an operator, the practical rule is the same for both values. Keep the disk,
and keep the configuration in source control. If the disk is lost, recover from
the shared database or a backup. If the label is lost, read it from the
configuration you saved.

## 4. The node and the runner

### 4.1 What a node is

The documentation defines a node as “a long-running process running your app
with the Rivet Software Development Kit (SDK)”, started with `registry.start()`
(`docs/general/content/nodes.mdx:7`). The node opens an outbound connection to
the control plane and hosts the Actors that the scheduler assigns to it.

A node needs no public Universal Resource Locator (URL) and no inbound port for Actor traffic
(`docs/general/content/nodes.mdx:42`). The control plane reaches the node over
the connection that the node opened. This is why a node can run behind Network
Address Translation (NAT) or in a private subnet.

A node joins a node pool. The pool must exist before the node connects. The
control plane refuses a node whose pool does not exist
(`docs/general/content/nodes.mdx:25`). The engine has no localhost exception: the
connection handler returns `WsError::NoRunnerConfig` when the pool lookup is
empty (`engine/packages/pegboard-envoy/src/conn.rs:110-122`). The exception is in
RivetKit, which creates the pool itself when the endpoint is on `localhost`
(`docs/general/content/nodes.mdx:25`).

Create the pool before the first production node. A pool name defaults to
`default` (`docs/general/content/environment-variables.mdx:24`).

```bash
# docs/general/content/nodes.mdx:19-22
curl -X PUT "$RIVET_API/pools/default?namespace=$RIVET_NAMESPACE" \
  -H "Authorization: Bearer $RIVET_TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{}'
```

### 4.2 Two node protocols

This commit has two node protocols. Both are live. A claim about one does not
automatically apply to the other.

| Property | Legacy runner path | Envoy path |
|----------|--------------------|------------|
| Client | `rivetkit-typescript/packages/engine-runner` | `engine/sdks/rust/envoy-client` |
| Connection path | `/runners/connect` | `/envoys/connect` |
| Protocol constant | `PROTOCOL_MK1_VERSION = 3`, `PROTOCOL_MK2_VERSION = 7` | `envoy_protocol::PROTOCOL_VERSION` from the highest schema, version 8 |
| Router | `guard::routing::runner` | `guard::routing::envoy` |
| Server entry | `pegboard_runner::PegboardRunnerWsCustomServe` | `pegboard_envoy::PegboardEnvoyWs` |
| Init message | `ToServerInit` | `ToEnvoyInit` |
| Still served by this engine | Yes | Yes |
| Default for RivetKit | No | Yes. `RIVETKIT_RUNTIME_MODE` falls back to `envoy` |

The default matters for an operator. `getRivetkitRuntimeMode` returns `envoy`
when `RIVETKIT_RUNTIME_MODE` is unset
(`rivetkit-typescript/packages/rivetkit/src/utils/env-vars.ts:59-66`). The legacy
path remains served, and the `@rivetkit/engine-runner` package still speaks it
(`rivetkit-typescript/packages/engine-runner/src/mod.ts:21-22`). A new deployment
should therefore expect `/envoys/connect` traffic by default, and must keep
`/runners/connect` open only while an older client is in use.

The runner protocol constants are explicit.

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

The TypeScript engine-runner hard-codes protocol 7.

```typescript
// rivetkit-typescript/packages/engine-runner/src/mod.ts:21-22
const KV_EXPIRE: number = 30_000;
const PROTOCOL_VERSION: number = 7;
```

The envoy schema directory has files `v1.bare` through `v8.bare`. The build
script sets `PROTOCOL_VERSION` to the highest file version
(`engine/sdks/rust/envoy-protocol/build.rs:22-30`). Therefore the compiled envoy
ceiling in this commit is 8.

### 4.3 Registration, step by step

The envoy path is the newer path. Its registration sequence is the clearest
example of how a node joins a cluster.

The sequence uses names that need a definition. `envoy_key` is a random identifier that the
client generates for one registration. The client keeps it for the life of the
connection and replaces it on rotation
(`engine/sdks/rust/envoy-client/src/context.rs:58-69`). `runner_key` is the same
idea on the legacy path: the TypeScript runner makes a random Universally Unique
Identifier (UUID) at construction
(`rivetkit-typescript/packages/engine-runner/src/mod.ts:205`). `RoutingOutput` is
the Guard return type. `RoutingOutput::CustomServe(PegboardEnvoyWs)` means "this
request belongs to the envoy service, hand the socket to it"
(`engine/packages/guard/src/routing/envoy.rs:56-62`). `init_conn` is the server
function that validates the URL, resolves the namespace, loads the pool, and
claims the registration (`engine/packages/pegboard-envoy/src/conn.rs:90-339`).
`Conn` is the per-connection struct that the server creates after the claim
(`engine/packages/pegboard-envoy/src/conn.rs:31`).

```mermaid
sequenceDiagram
    participant N as "envoy-client"
    participant G as "Guard /envoys/connect"
    participant P as "PegboardEnvoyWs"
    participant C as "conn::init_conn"
    participant U as "UniversalDB"
    participant W as "pegboard-envoy Conn"
    N->>G: "WebSocket /envoys/connect?protocol_version=8&namespace=...&envoy_key=...&version=...&pool_name=..."
    G->>G: "check_connection_auth(ctx, shared_state, req_ctx, envoy)"
    G->>P: "RoutingOutput::CustomServe(PegboardEnvoyWs)"
    P->>C: "init_conn(ctx, ws_handle, UrlData)"
    C->>U: "namespace::ops::resolve_for_name_global"
    C->>U: "pegboard::ops::runner_config::get"
    C-->>N: "ToEnvoyInit(envoy_lost_threshold, actor_stop_threshold, max_response_payload_size)"
    C->>U: "claim_registration txn"
    U-->>C: "EnvoyConnIdKey, PoolNameKey, VersionKey, SlotsKey, ProtocolVersionKey, ActiveEnvoyKey"
    C->>W: "spawn connection tasks"
```

The client side builds the URL. The client also sends its token in the WebSocket
subprotocol list, not in the URL.

```rust
// engine/sdks/rust/envoy-client/src/connection/mod.rs:352-369
fn ws_url(shared: &SharedContext) -> String {
	let ws_endpoint = shared
		.config
		.endpoint
		.replace("http://", "ws://")
		.replace("https://", "wss://");
	let base_url = ws_endpoint.trim_end_matches('/');

	format!(
		"{}/envoys/connect?protocol_version={}&namespace={}&envoy_key={}&version={}&pool_name={}",
		base_url,
		protocol::PROTOCOL_VERSION,
		urlencoding::encode(&shared.config.namespace),
		urlencoding::encode(&shared.envoy_key()),
		urlencoding::encode(&shared.config.version.to_string()),
		urlencoding::encode(&shared.config.pool_name),
	)
}
```

The client configuration carries the values that the operator must supply.

```rust
// engine/sdks/rust/envoy-client/src/config.rs:14-30
pub struct EnvoyConfig {
	pub version: u32,
	pub endpoint: String,
	pub token: Option<String>,
	pub namespace: String,
	pub pool_name: String,
	pub prepopulate_actor_names: HashMap<String, ActorName>,
	pub metadata: Option<serde_json::Value>,
	/// When `start_envoy` is called, create a new envoy every time instead of using a single global envoy
	/// instance for the entire runtime.
	pub not_global: bool,

	/// Debug option to inject artificial latency (in ms) into WebSocket communication.
	pub debug_latency_ms: Option<u64>,

	pub callbacks: Arc<dyn EnvoyCallbacks>,
}
```

The Guard router checks the token, then hands the socket to the envoy service.

```rust
// engine/packages/guard/src/routing/envoy.rs:28-62
/// Route requests to the envoy service using path-based routing
/// Matches path: /envoys/connect
#[tracing::instrument(skip_all)]
pub async fn route_request_path_based(
	ctx: &StandaloneCtx,
	shared_state: &SharedState,
	req_ctx: &RequestContext,
) -> Result<Option<RoutingOutput>> {
	// Check if path matches /envoys/connect
	let path_without_query = req_ctx.path().split('?').next().unwrap_or(req_ctx.path());
	if path_without_query != "/envoys/connect" && path_without_query != "/envoys/connect/" {
		return Ok(None);
	}

	tracing::debug!(hostname=%req_ctx.hostname(), path=%req_ctx.path_for_logs(), "routing to envoy via path");

	route_envoy_internal(ctx, shared_state, req_ctx)
		.await
		.map(Some)
}

/// Internal envoy routing logic shared by both header-based and path-based routing
#[tracing::instrument(skip_all)]
async fn route_envoy_internal(
	ctx: &StandaloneCtx,
	shared_state: &SharedState,
	req_ctx: &RequestContext,
) -> Result<RoutingOutput> {
	validate_regional_host(ctx, req_ctx)?;

	check_connection_auth(ctx, shared_state, req_ctx, "envoy").await?;

	let tunnel = pegboard_envoy::PegboardEnvoyWs::new(&ctx);
	Ok(RoutingOutput::CustomServe(Arc::new(tunnel)))
}
```

The server sends its thresholds first. The node learns how long the control
plane will wait before it declares the node lost.

```rust
// engine/packages/pegboard-envoy/src/conn.rs:146-166
	let (_, runner_config_protocol_changed, missed_commands) = tokio::try_join!(
		// Send init packet as soon as possible
		async {
			let pb = ctx.config().pegboard();

			// Send init packet
			let init_msg = versioned::ToEnvoy::wrap_latest(protocol::ToEnvoy::ToEnvoyInit(
				protocol::ToEnvoyInit {
					metadata: protocol::ProtocolMetadata {
						envoy_lost_threshold: pb.envoy_lost_threshold(),
						actor_stop_threshold: pb.actor_stop_threshold(),
						max_response_payload_size: pb.envoy_max_response_payload_size() as u64,
					},
				},
			));
			let init_msg_serialized = init_msg.serialize(protocol_version)?;
			ws_handle
				.send(Message::Binary(init_msg_serialized.into()))
				.await
		},
		udb.txn("envoy_conn_prepare", |tx| {
```

The `claim_registration` transaction is the moment the node becomes visible to
the scheduler. It writes the owner index, the pool index, the version, the
protocol version, and the active-envoy index.

```rust
// engine/packages/pegboard-envoy/src/conn.rs:374-435
	udb.txn("envoy_conn_register", |tx| async move {
		let tx = tx.with_subspace(pegboard::keys::subspace());
		let create_ts_key =
			pegboard::keys::envoy::CreateTsKey::new(namespace_id, envoy_key.to_string());
		let last_ping_ts_key =
			pegboard::keys::envoy::LastPingTsKey::new(namespace_id, envoy_key.to_string());
		let version_key =
			pegboard::keys::envoy::VersionKey::new(namespace_id, envoy_key.to_string());

		let (create_ts_entry, old_last_ping_ts_entry, version_entry) = tokio::try_join!(
			tx.read_opt(&create_ts_key, Serializable),
			tx.read_opt(&last_ping_ts_key, Serializable),
			tx.read_opt(&version_key, Serializable),
		)?;

		if let Some(old_version) = version_entry {
			if old_version != version {
				tracing::warn!(
					?namespace_id,
					%envoy_key,
					old_version,
					new_version = version,
					"envoy_key reconnected with changed version; operationally prohibited - investigate"
				);
			}
		}

		tx.write(
			&pegboard::keys::envoy::EnvoyConnIdKey::new(namespace_id, envoy_key.to_string()),
			envoy_conn_id,
		)?;
		tx.write(
			&pegboard::keys::envoy::PoolNameKey::new(namespace_id, envoy_key.to_string()),
			pool_name.to_string(),
		)?;
		tx.write(&version_key, version)?;
		tx.atomic_op(
			&pegboard::keys::envoy::SlotsKey::new(namespace_id, envoy_key.to_string()),
			&0i64.to_le_bytes(),
			MutationType::Add,
		);

		let create_ts = if let Some(create_ts) = create_ts_entry {
			create_ts
		} else {
			let create_ts = util::timestamp::now();
			tx.write(&create_ts_key, create_ts)?;
			create_ts
		};
		let last_ping_ts = util::timestamp::now();
		tx.write(&last_ping_ts_key, last_ping_ts)?;
		tx.write(
			&pegboard::keys::envoy::ProtocolVersionKey::new(namespace_id, envoy_key.to_string()),
			protocol_version,
		)?;

		tx.write(
			&pegboard::keys::ns::ActiveEnvoyKey::new(
				namespace_id,
				create_ts,
				envoy_key.to_string(),
			),
```

The pool name must already exist. `init_conn` reads the runner configuration and
returns `WsError::NoRunnerConfig` when it does not.

```rust
// engine/packages/pegboard-envoy/src/conn.rs:110-122
	let pool_res = ctx
		.op(pegboard::ops::runner_config::get::Input {
			runners: vec![(namespace.namespace_id, pool_name.clone())],
			bypass_cache: false,
		})
		.await?;

	let Some(pool) = pool_res.into_iter().next() else {
		return Err(errors::WsError::NoRunnerConfig {
			pool_name: pool_name.clone(),
		}
		.build());
	};
```

The legacy runner path has the same shape. The TypeScript client builds a URL
with `/runners/connect` and protocol 7. It sends `ToServerInit` after the socket
opens.

```typescript
// rivetkit-typescript/packages/engine-runner/src/mod.ts:743-759
	get pegboardUrl() {
		const wsEndpoint = this.pegboardEndpoint
			.replace("http://", "ws://")
			.replace("https://", "wss://");

		// Ensure the endpoint ends with /runners/connect
		const baseUrl = wsEndpoint.endsWith("/")
			? wsEndpoint.slice(0, -1)
			: wsEndpoint;
		return `${baseUrl}/runners/connect?protocol_version=${PROTOCOL_VERSION}&namespace=${encodeURIComponent(this.#config.namespace)}&runner_key=${encodeURIComponent(this.#runnerKey)}`;
	}

	// MARK: Runner protocol
	async #openPegboardWebSocket() {
		const protocols = ["rivet"];
		if (this.config.token)
			protocols.push(`rivet_token.${this.config.token}`);
```

```typescript
// rivetkit-typescript/packages/engine-runner/src/mod.ts:817-839
			// Send init message
			const init: protocol.ToServerInit = {
				name: this.#config.runnerName,
				version: this.#config.version,
				totalSlots: this.#config.totalSlots,
				prepopulateActorNames: new Map(
					Object.entries(this.#config.prepopulateActorNames).map(
						([name, data]) => [
							name,
							{ metadata: JSON.stringify(data.metadata) },
						],
					),
				),
				metadata: JSON.stringify(this.#config.metadata),
			};

			this.__sendToServer({
				tag: "ToServerInit",
				val: init,
			});

			// Start command acknowledgment interval (5 minutes)
			const ackInterval = 5 * 60 * 1000; // 5 minutes in milliseconds
```

On the server, the runner path looks up an existing runner by key before it
creates a new one. A live runner keeps its identity across a reconnect. An
expired runner gets a new identifier.

```rust
// engine/packages/pegboard-runner/src/conn.rs:94-141
	// Look up existing runner by key
	let existing_runner = ctx
		.op(pegboard::ops::runner::get_by_key::Input {
			namespace_id: namespace.namespace_id,
			name: init.name().to_string(),
			key: runner_key.clone(),
		})
		.await
		.with_context(|| {
			format!(
				"failed to get existing runner by key: {}:{}",
				init.name(),
				runner_key
			)
		})?;

	let runner_id = if let Some(runner) = existing_runner.runner {
		// IMPORTANT: Before we spawn/get the workflow, we try to update the runner's last ping ts.
		// This ensures if the workflow is currently checking for expiry that it will not expire
		// (because we are about to send signals to it) and if it is already expired (but not
		// completed) we can choose a new runner id.
		let update_ping_res = ctx
			.op(pegboard::ops::runner::update_alloc_idx::Input {
				runners: vec![pegboard::ops::runner::update_alloc_idx::Runner {
					runner_id: runner.runner_id,
					action: Action::UpdatePing { rtt: 0 },
				}],
			})
			.await
			.with_context(|| format!("failed to update ping for runner: {}", runner.runner_id))?;

		if update_ping_res
			.notifications
			.into_iter()
			.next()
			.map(|notif| matches!(notif.eligibility, RunnerEligibility::Expired))
			.unwrap_or_default()
		{
			// Runner expired, create a new one
			Id::new_v1(ctx.config().dc_label())
		} else {
			// Use existing runner
			runner.runner_id
		}
	} else {
		// No existing runner for this key, create a new one
		Id::new_v1(ctx.config().dc_label())
	};
```

### 4.4 What the operator must provide

| Requirement | Detail | Source |
|-------------|--------|--------|
| A Node.js or Bun process | The TypeScript SDK runs on native Node.js and Bun | `README.md:224`, `docs/general/content/nodes.mdx:7` |
| A Rust process for the envoy client | The Rust SDK starts `start_envoy(EnvoyConfig)` | `rivetkit-rust/packages/rivetkit-core/src/registry/mod.rs:653-665` |
| Outbound WebSocket egress | The node connects to `/runners/connect` or `/envoys/connect` | `engine/packages/guard/src/routing/runner.rs:36-46`, `engine/packages/guard/src/routing/envoy.rs:36-46` |
| A bearer token | `RIVET_TOKEN`, sent as `rivet_token.<token>` in the WebSocket subprotocol list | `docs/general/content/environment-variables.mdx:16-17`; `engine/sdks/rust/envoy-client/src/connection/native.rs:100-106` |
| A namespace | `RIVET_NAMESPACE`, default `default` | `docs/general/content/environment-variables.mdx:18` |
| An existing pool | `RIVET_POOL`, default `default`; the pool must exist | `docs/general/content/nodes.mdx:25`, `docs/general/content/environment-variables.mdx:24` |
| A version number | `RIVET_ENVOY_VERSION`; increase on every deploy | `docs/general/content/environment-variables.mdx:25` |
| No inbound port | Actor traffic uses the outbound socket | `docs/general/content/nodes.mdx:42` |

The node environment variables are read by the TypeScript SDK.

```typescript
// rivetkit-typescript/packages/rivetkit/src/utils/env-vars.ts:8-46
// Rivet configuration
export const getRivetEngine = (): string | undefined =>
	getEnvUniversal("RIVET_ENGINE");
export const getRivetEndpoint = (): string | undefined =>
	getEnvUniversal("RIVET_ENDPOINT");
export const getRivetToken = (): string | undefined =>
	getEnvUniversal("RIVET_TOKEN");
export const getRivetNamespace = (): string | undefined =>
	getEnvUniversal("RIVET_NAMESPACE");
export const getRivetPool = (): string | undefined =>
	getEnvUniversal("RIVET_POOL");
export const getRivetTotalSlots = (): number | undefined => {
	const value = getEnvUniversal("RIVET_TOTAL_SLOTS");
	return value !== undefined ? parseInt(value, 10) : undefined;
};
export const getRivetRunEngine = (): boolean =>
	getEnvUniversal("RIVET_RUN_ENGINE") === "1";
export const getRivetRunEngineHost = (): string | undefined =>
	getEnvUniversal("RIVET_RUN_ENGINE_HOST");
export const getRivetRunEnginePort = (): number | undefined => {
	const value = getEnvUniversal("RIVET_RUN_ENGINE_PORT");
	return value !== undefined ? parseInt(value, 10) : undefined;
};
export const getRivetRunEngineVersion = (): string | undefined =>
	getEnvUniversal("RIVET_RUN_ENGINE_VERSION");
export const getRivetRunServices = (): boolean | undefined => {
	const value = getEnvUniversal("RIVET_RUN_SERVICES");
	return value === undefined ? undefined : value === "1";
};
export const getRivetEnvoyKind = (): string | undefined =>
	getEnvUniversal("RIVET_ENVOY_KIND");
export const getRivetEnvoyVersion = (): number | undefined => {
	const value = getEnvUniversal("RIVET_ENVOY_VERSION");
	return value !== undefined ? parseInt(value, 10) : undefined;
};
export const getRivetPublicEndpoint = (): string | undefined =>
	getEnvUniversal("RIVET_PUBLIC_ENDPOINT");
export const getRivetPublicToken = (): string | undefined =>
	getEnvUniversal("RIVET_PUBLIC_TOKEN");
```

The executor can be in the same process as the application. A node is not a
separate container. It is the application process, with the SDK started.

## 5. The three delivery shapes

The README names three shapes and says that every shape runs the same binary.
The difference is who operates it (`README.md:218`). Clients use Hypertext
Transfer Protocol Secure (HTTPS). Internal instances use plain Hypertext
Transfer Protocol (HTTP) or HTTPS.

```mermaid
flowchart TB
    subgraph SH["Self-host: your account"]
        SC["rivet-engine<br/>you operate"]
        SD[("local disk or Postgres")]
        SB[("bucket<br/>Enterprise model")]
        SN["nodes<br/>you operate"]
        SC --- SD
        SC -.-> SB
        SN -->|"outbound /envoys/connect"| SC
    end
    subgraph CL["Rivet Cloud"]
        CC["rivet-engine<br/>Rivet operates"]
        CB[("storage<br/>Rivet account")]
        CN["nodes<br/>Rivet Cloud or your compute"]
        CC --- CB
        CN -->|"outbound"| CC
    end
    subgraph BY["Bring Your Own Cloud (BYOC)"]
        BC["rivet-engine and operator<br/>your cloud account, Rivet operates"]
        BB[("bucket<br/>your account")]
        BN["nodes<br/>your compute"]
        BC --- BB
        BN -->|"outbound"| BC
        BC -->|"outbound to Rivet Cloud"| CC
    end
    U["user"] -->|"HTTPS"| SC
    U -->|"HTTPS"| CC
    U -->|"HTTPS"| BC
```

### 5.1 Self-host

In the documented self-host model, the operator runs all four parts. The
operator provides the instances, the disk, the bucket, and the nodes. The
operator also provides the public endpoint and the Transport Layer Security
(TLS) certificate.

The open-source self-host procedure in this repository is different. One
instance uses a local RocksDB path. More than one instance uses a shared
PostgreSQL database and NATS (section 1.3). No guide in this repository uses a
bucket as the only durable store.

The README says one instance is enough to try Rivet, and three or more are
needed in production (`README.md:218`). The benchmark uses exactly three
instances, one per availability zone, and acknowledges each write at two of the
three (`README.md:155`, `docs/general/content/benchmarking.mdx:15`).

### 5.2 Rivet Cloud

In Rivet Cloud, Rivet operates the control plane. The README says there is
“nothing to operate” for the application, or the operator can bring their own
nodes and let Rivet Cloud provide the control plane and routing layer
(`README.md:201`).

The README does not state where Rivet Cloud stores Actor data. The repository
does not contain the cloud deployment configuration. Do not infer a bucket
location from this source.

### 5.3 Bring Your Own Cloud

Bring Your Own Cloud (BYOC) runs the whole stack inside the customer Virtual
Private Cloud (VPC). Rivet operates it over an outbound connection. The README
states that every Actor's data stays in the customer account and the customer
bucket (`README.md:210`).

The public website source in this repository adds operational detail. These
claims come from the website, not from the engine code. Treat them as product
documentation.

```typescript
// frontend/src/app/byoc/agent-instructions.ts:26-56
		"## 1. Read the setup guide",
		BYOC_QUICKSTART_DOCS_URL,
		"",
		"## 2. Ask me how I want to configure the deployment",
		"Before provisioning, ask for the choices below together. Reuse answers I have already given, suggest the documented defaults, and wait for my confirmation rather than guessing.",
		"- Cloud provider: AWS or Google Cloud, and the AWS account ID or Google Cloud project ID.",
		"- Deployment name (name): used for cloud resource names and internal DNS; suggest rivet-prod.",
		"- Regions (regions): which cloud regions to use; for multiple regions, also ask which should be the leader.",
		"- Endpoint access (rivet_endpoint_access): explicitly choose private HTTP, or public HTTPS plus private HTTP. Do not assume a choice.",
		"- Public hostname (rivet_hostname): ask only for public access; recommend a hostname such as rivet.mycompany.com.",
		"- Kubernetes namespaces (kubernetes_namespace, kubernetes_operator_namespace): suggest rivet and rivet-operator; they must differ.",
		"Also confirm whether to create new infrastructure or reference compatible existing resources. Do not adopt or replace existing infrastructure implicitly.",
		"",
		"## 3. Prepare and install",
		"Check the guide's prerequisites and cloud authentication. Ask me to complete any missing authentication.",
		"Download and extract this setup kit, then use terraform/byoc-aws/ or terraform/byoc-gcp/ for the selected provider:",
		BYOC_SETUP_KIT_URL,
		"Configure terraform.tfvars.json with my choices. The cluster identity and token are already included here; do not ask me to download config or copy a token from the dashboard.",
		`Write the following non-secret Terraform inputs to ${CLUSTER_CONFIG_FILENAME} in the same directory. Preserve the Cloud API endpoint when provided:`,
		"```json",
		config.trimEnd(),
		"```",
		"For the first apply, pass this operator token as RIVET_BYOC_OPERATOR_TOKEN in the Terraform process environment (decode the JSON string below):",
		JSON.stringify(operatorToken),
		"Do not write the token to Terraform inputs or a credentials file, put it in command arguments, or expose it in logs or source control. Do not repeat it in your response. Later applies use the token stored in cloud Secret Manager.",
		"Show me the Terraform plan and get approval before applying. Reuse the same extracted kit, Terraform directory, and state for retries; do not download a newer kit or recreate infrastructure to resolve a failure.",
		"Follow the guide's DNS timing. If you lack DNS access, show me the exact records to create and keep Terraform running when it is waiting for certificate validation.",
		"",
		"## 4. Verify and hand off",
		"Verify Terraform completed and the operator is healthy and connected to Rivet Cloud. If any check cannot be completed, report it as unverified rather than claiming success.",
		"Provisioning the operator does not deploy the Engine. Summarize the installation and explain the guide's next step for requesting the Engine deployment from Rivet.",
```

The cluster identity file is `rivet.auto.tfvars.json`. It holds a non-secret
cluster identifier and an optional cloud Application Programming Interface
(API) URL.

```typescript
// frontend/src/app/byoc/cluster-config.ts:30-41
	return (
		JSON.stringify(
			{
				byoc_cluster_id: clusterId,
				...(origin.origin === "https://cloud-api.rivet.dev"
					? {}
					: { cloud_api_url: origin.origin }),
			},
			null,
			2,
		) + "\n"
	);
```

The website plan copy lists these claims: the stack runs in the customer Virtual
Private Cloud (VPC), Rivet operates it, data residency, Customer-Managed
Encryption Key (CMEK) support, and enterprise support
(`frontend/src/content/byoc.ts:13-24`). The claim belongs to the BYOC product.
The open-source configuration described in this document has no CMEK setting.

| BYOC detail | Value | Source type |
|-------------|-------|-------------|
| Cloud providers | Amazon Web Services (AWS) or Google Cloud | Website source |
| Provisioning tool | Terraform directories `byoc-aws/` or `byoc-gcp/` | Website source |
| Kubernetes namespaces | `rivet` and `rivet-operator`, must differ | Website source |
| Endpoint access | Private HTTP, or public HTTPS plus private HTTP | Website source |
| Operator token | `RIVET_BYOC_OPERATOR_TOKEN` | Website source |
| Operator connection | Outbound connection to Rivet Cloud | Website source |
| Engine deployment | The operator install does not deploy the Engine | Website source |
| Trial | 14 days | Website source |

**Unverified:** the repository does not contain the BYOC Terraform modules. The
setup kit is a downloaded archive from `releases.rivet.dev`, and it is not in
this snapshot. The steps above are the instructions in the website source, not
a trace of the Terraform code.

## 6. Deployment guides in the repository

The repository holds deployment artifacts under `self-host/`. Each directory has
a `README.md` that points to the public guide. The guides are embedded from
these files with a documentation component, so the files and the guide should
match (`self-host/control-plane/docker-compose/README.md:5-6`).

### 6.1 Docker Compose

The Docker Compose guide runs two services: `rivet-engine` and `runner`. The
engine uses the default file system database on a named volume. The runner is a
separate process that connects to the engine with `RIVET_ENDPOINT` and
`RIVET_TOKEN`.

```yaml
# self-host/control-plane/docker-compose/docker-compose.yml:26-42
  runner:
    build:
      context: ../..
      dockerfile: docker/runner/Dockerfile
    platform: linux/amd64
    restart: unless-stopped
    environment:
      - RIVET_ENDPOINT=http://rivet-engine:6420
      - RIVET_TOKEN=dev
    stop_grace_period: 4s
    ports:
      - '5050:5050'
    depends_on:
      rivet-engine:
        condition: service_healthy
volumes:
  rivet-engine-data:
```

The engine health check uses `/health` on port 6421. The readiness and liveness
probes in the Kubernetes manifest use the same route.

### 6.2 Kubernetes

The Kubernetes directory holds a full manifest set. The set is not the
bucket-only model. It deploys PostgreSQL and NATS, and it runs two engine
replicas with a Horizontal Pod Autoscaler from 2 to 10
(`self-host/control-plane/kubernetes/README.md:3-5`,
`self-host/control-plane/kubernetes/03-rivet-engine-deployment.yaml:11`,
`self-host/control-plane/kubernetes/05-rivet-engine-hpa.yaml`).

The engine deployment sets a 25-second termination grace period, a 15-second
`preStop` sleep, and three probes on port 6421.

```yaml
# self-host/control-plane/kubernetes/03-rivet-engine-deployment.yaml:21-84
    spec:
      serviceAccountName: rivet-engine
      terminationGracePeriodSeconds: 25
      containers:
      - name: rivet-engine
        image: rivetdev/engine:latest
        args:
        - start
        env:
        - name: RIVET_CONFIG_PATH
          value: /etc/rivet/config.jsonc
        - name: RIVET__AUTH__ADMIN_TOKEN
          valueFrom:
            secretKeyRef:
              name: rivet-secrets
              key: admin-token
        ports:
        - containerPort: 6420
          name: guard
          protocol: TCP
        - containerPort: 6421
          name: api-peer
          protocol: TCP
        resources:
          requests:
            cpu: 2000m
            memory: 2Gi
          limits:
            cpu: 3000m
            memory: 4Gi
        startupProbe:
          httpGet:
            path: /health
            port: 6421
          initialDelaySeconds: 30
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 30
        readinessProbe:
          httpGet:
            path: /health
            port: 6421
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 2
        livenessProbe:
          httpGet:
            path: /health
            port: 6421
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 3
        lifecycle:
          preStop:
            exec:
              command: ["/bin/sh", "-c", "sleep 15"]
        volumeMounts:
        - mountPath: /etc/rivet
          name: config
          readOnly: true
      volumes:
      - name: config
        configMap:
          name: engine-config
```

The ConfigMap selects PostgreSQL and NATS. This is the open-source multi-instance
path. It does not use a bucket.

```yaml
# self-host/control-plane/kubernetes/02-engine-configmap.yaml:1-29
apiVersion: v1
data:
  config.jsonc: |
    {
      "postgres": {
        "url": "postgresql://postgres:postgres@postgres:5432/rivet"
      },

      "nats": {
        "addresses": [
          "nats.rivet-engine.svc.cluster.local:4222"
        ]
      },

      "topology": {
        "datacenter_label": 1,
        "datacenters": [
          {
            "name": "default",
            "datacenter_label": 1,
            "is_leader": true,
            "public_url": "http://localhost:6420",
            "peer_url": "http://rivet-engine.rivet-engine.svc.cluster.local:6421",
            "proxy_url": "http://rivet-engine.rivet-engine.svc.cluster.local:6420"
          }
        ]
      }
    }
kind: ConfigMap
```

The Service is a `NodePort` service. It exposes Guard on 30420 and the private
API on 30421.
```yaml
# self-host/control-plane/kubernetes/04-rivet-engine-service.yaml:10-24
spec:
  type: NodePort
  ports:
  - name: guard
    port: 6420
    targetPort: 6420
    nodePort: 30420
    protocol: TCP
  - name: api-peer
    port: 6421
    targetPort: 6421
    nodePort: 30421
    protocol: TCP
  selector:
    app: rivet-engine
```

The sample `public_url` is a placeholder. It is `http://localhost:6420`
(`self-host/control-plane/kubernetes/02-engine-configmap.yaml:22`). The public
URL is the origin that clients use (`engine/packages/config/src/config/topology.rs:164-165`).
Replace it with the address that clients really use, such as
`https://rivet.example.com`. A localhost value works only for a client on the
same host.

The `NodePort` type exposes the api peer on 30421 as well as Guard on 30420. The
api peer is the peer interface, and its address goes in `peer_url`
(`engine/packages/config/src/config/topology.rs:166-167`). It is not the listener
for client traffic; Guard is. A `NodePort` opens it on every node. Restrict port
30421 with a network policy, or publish only Guard, before production.

The manifest README says the bundled PostgreSQL is a single-replica StatefulSet
and recommends a managed service for production
(`self-host/control-plane/kubernetes/README.md:31`).

The 25-second pod grace period is shorter than the Guard shutdown budget. The
Guard budget defaults to 10 minutes
(`engine/packages/config/src/config/runtime.rs:192-194`). The 15-second `preStop`
sleep also comes from the same 25 seconds. Kubernetes sends `SIGKILL` with about
10 seconds left for Guard. Raise `terminationGracePeriodSeconds` above the
Guard budget, or lower the budget, before production use.

The 10-minute force timer and the 25-second grace period do not conflict in the
way they appear to. The force timer is an upper bound inside the process. It
fires only if a graceful service does not stop
(`engine/packages/service-manager/src/lib.rs:369-405`). Kubernetes stops the
process first, so the force timer never fires under this manifest. The protocol
hold-back does not depend on a graceful stop either. If `SIGKILL` removes the
heartbeat, the entry expires after 30 seconds (section 8.3). The grace period
therefore changes how long requests have to drain. It does not change the
version floor.

### 6.3 Virtual machine and bare metal

The Virtual Machine (VM) guide runs one container with a named volume and a
published port. The commands are in `self-host/control-plane/vm/commands.sh`.
The README lists two files, `commands.sh` and `commands-2.sh`
(`self-host/control-plane/vm/README.md:10-13`). Only `commands.sh` exists in
this snapshot.

The order is: run the container, then request `/health`.

### 6.4 Custom build

The custom guide has two options. The first runs the published image. The second
builds the binary from source.

```bash
# self-host/control-plane/custom/commands.sh:1-10
#!/usr/bin/env bash
# Commands from the guide, in order.
set -euo pipefail

docker run -p 6420:6420 rivetdev/engine

git clone https://github.com/rivet-dev/rivet.git
cd rivet
cargo build --release -p rivet-engine
./target/release/rivet-engine
```

These commands are a trial, not a production recipe. They set no admin token and
no data volume. The engine refuses to start without `auth.admin_token`
(`engine/packages/config/src/config/mod.rs:260-266`). Add
`-e RIVET__AUTH__ADMIN_TOKEN=...` and a volume with
`RIVET__FILE_SYSTEM__PATH` before you use this path.

The binary has subcommands for `start`, `database`, `workflow`, `config`,
`auth`, `tracing`, `profile`, `epoxy`, `depot`, and `udb`
(`engine/packages/engine/src/lib.rs:10-58`). The `depot` subcommands export one
Actor database. Section 10 explains why they are not a cluster backup.

### 6.5 Railway, Render, Hetzner, and Amazon Elastic Container Service (Amazon ECS)

The repository has different levels of support for these platforms.

| Platform | Repository artifact | State |
|----------|---------------------|-------|
| Railway | `self-host/control-plane/railway/commands.sh` | Only `railway init` |
| Render | None | README link only (`README.md:226`) |
| Hetzner | None | README link only (`README.md:226`) |
| Amazon Elastic Container Service (ECS) | `self-host/control-plane/aws-ecs/README.md` | Explicit TODO, no artifacts |

The Railway file has two commands. It does not deploy a service.

```bash
# self-host/control-plane/railway/commands.sh:1-5
# Using Railway CLI
railway init

# Or create via dashboard
# https://railway.app/new
```

The ECS directory says the platform had no existing guide to port and asks for
artifacts (`self-host/control-plane/aws-ecs/README.md:1-13`). Do not invent an
ECS task definition. The repository does not contain one.

### 6.6 Worker and node guides

The `self-host/workers/` tree holds node deployment artifacts. These run the
application process, not the control plane.

| Platform | Files | State |
|----------|-------|-------|
| Docker Compose | `Dockerfile`, `compose.yaml` | Present |
| Kubernetes | `Dockerfile`, `rivetkit-app.yaml`, `rivetkit-secrets.yaml`, `commands.sh` | Present |
| VM | `commands.sh` | Present |
| Google Cloud Run | `Dockerfile`, `commands.sh` | Present |
| Cloudflare Workers | `wrangler.toml`, `commands.sh` | Present |
| Supabase | `commands.sh` | Present |
| Vercel | `config.yaml`, `route.ts` | Present |
| AWS ECS | `README.md` | TODO |
| AWS Lambda | `README.md` | TODO |
| Custom | `README.md` | TODO |
| Railway | `README.md` | TODO |

The Kubernetes node manifest sets a long termination grace period. The comment
ties the value to the Actor stop budget.

```yaml
# self-host/workers/kubernetes/rivetkit-app.yaml:14-25
    spec:
      # Allow enough time for actors to gracefully stop on SIGTERM.
      # The runner waits up to 30m for actors to finish.
      # Add buffer for runner shutdown overhead after actors stop.
      # See: /docs/actors/versions#graceful-shutdown-sigterm
      terminationGracePeriodSeconds: 2100
      containers:
        - name: rivetkit-app
          image: registry.example.com/your-team/rivetkit-app:latest
          envFrom:
            - secretRef:
                name: rivetkit-secrets
```

The VM worker commands copy the build, enable a systemd unit, restart it, and
follow the journal.

```bash
# self-host/workers/vm/commands.sh:1-12
#!/usr/bin/env bash
# Commands from the guide, in order.
set -euo pipefail

scp -r ./dist user@server:/opt/rivetkit-app

sudo systemctl daemon-reload
sudo systemctl enable --now rivetkit-app.service

sudo systemctl restart rivetkit-app.service

sudo journalctl -u rivetkit-app.service -f
```

## 7. Scaling

### 7.1 Add an instance

An instance is identical to the others. Add it with the same configuration, the
same topology, and the same shared database. Then route traffic to it through
the load balancer.

The safe maximum depends on the storage path.

| Storage path | Can add an instance? | Reason |
|--------------|----------------------|--------|
| `FileSystem` | No, not as a cluster | Each process has its own RocksDB path. The instances do not share rows. |
| `Postgres` + `NATS` | Yes | The driver and the message bus connect to shared services (`engine/packages/pools/src/db/udb.rs:21-56`). |
| Documented bucket model | Yes | The bucket is the shared store (`README.md:218`). |

The project benchmark uses three instances and acknowledges each write at two of
three (`README.md:155`). The source does not state a maximum instance count.
Epoxy uses the datacenter label as the replica identifier
(`engine/packages/config/src/config/mod.rs:345-353`). The topology validation
requires `valid_hosts` for every datacenter when the topology has more than one
(`engine/packages/config/src/config/mod.rs:292-300`).

Adding an instance has two different meanings. An instance in an existing
datacenter adds request capacity and shares that datacenter's replica identity.
An instance in a new datacenter adds an Epoxy voter. The second case needs a new
`datacenters` entry, a new label, and the same updated topology on every
instance.

### 7.2 Add a node

Run more copies of the application with the same namespace and pool. The
documentation recommends using the platform autoscaler and keeping spare
capacity (`docs/general/content/nodes.mdx:95-99`).

A new node starts with an empty Actor set. The scheduler fills it with Actors as
requests arrive. The cost of a cold start does not depend on the number of
instances. It depends on the node that receives the Actor and the state that the
Actor must load.

The control plane allocates Actors to envoys with a hash-ring default. The
default is `EnvoyLoadBalancer::Hash` with 8 virtual nodes, 2 samples, a maximum
scan of 16 stale entries, and a slot jitter of 4
(`engine/packages/config/src/config/pegboard.rs:511-522`).

### 7.3 Add a datacenter

A datacenter is an entry in `topology.datacenters`. One entry must be the
leader. The local instance selects its own entry by `datacenter_label`.

```rust
// engine/packages/config/src/config/topology.rs:155-179
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Datacenter {
	/// When configuring `datacenters` via a hashmap this is automatically derived from the key. Required
	/// when configuring via list (which is deprecated)
	#[serde(default = "String::new")]
	pub name: String,
	pub datacenter_label: u16,
	pub is_leader: bool,
	/// Public origin that can be used to connect to this region.
	pub public_url: Url,
	/// URL of the api-peer service
	pub peer_url: Url,
	/// URL of the guard service that other datacenters can access privately. Goes to the same place as
	// public_url.
	pub proxy_url: Option<Url>,
	/// List of hosts that are valid to connect to this region with. This is used in regional
	/// endpoints to validate that incoming requests to this datacenter are going to a
	/// region-specific domain.
	///
	/// IMPORTANT: Do not use a global origin that routes to multiple different regions. This will
	/// cause unpredictable behavior when requests are expected to go to a specific region.
	#[serde(default)]
	pub valid_hosts: Option<Vec<String>>,
}
```

The multi-datacenter development configuration shows the shape. One datacenter
is the leader. Each datacenter has a distinct label and a distinct public URL.

```jsonc
// self-host/compose/dev-multidc/datacenters/dc-a/rivet-engine/config.jsonc:8-45
	"topology": {
		"datacenter_label": 1,
		"datacenters": {
			"dc-a": {
				"datacenter_label": 1,
				"is_leader": true,
				"peer_url": "http://rivet-engine-dc-a:6421",
				"public_url": "http://rivet-engine-dc-a:6420",
				"valid_hosts": [
					"rivet-engine-dc-a",
					"127.0.0.1",
					"localhost"
				]
			},
			"dc-b": {
				"datacenter_label": 2,
				"is_leader": false,
				"peer_url": "http://rivet-engine-dc-b:6421",
				"public_url": "http://rivet-engine-dc-b:6420",
				"valid_hosts": [
					"rivet-engine-dc-b",
					"127.0.0.1",
					"localhost"
				]
			},
			"dc-c": {
				"datacenter_label": 3,
				"is_leader": false,
				"peer_url": "http://rivet-engine-dc-c:6421",
				"public_url": "http://rivet-engine-dc-c:6420",
				"valid_hosts": [
					"rivet-engine-dc-c",
					"127.0.0.1",
					"localhost"
				]
			}
		}
	},
```

Document [08 Epoxy](08-epoxy-multi-region.md) explains the commit protocol
between replicas. The deployment rule from this source is narrow. Do not put a
global hostname in `valid_hosts`, because a global name can route to the wrong
region (`engine/packages/config/src/config/topology.rs:171-178`).

### 7.4 Limits from code

| Limit | Value | Source |
|-------|-------|--------|
| HTTP request body | 20 MiB | `engine/packages/config/src/config/guard.rs:323-325` |
| WebSocket message | 32 MiB | `engine/packages/config/src/config/guard.rs:6-7` |
| WebSocket frame | 32 MiB | `engine/packages/config/src/config/guard.rs:6-7` |
| Storage for one Actor SQLite database | 10 GiB | `engine/packages/config/src/config/sqlite.rs:207-208` |
| Actor response payload through an envoy | 20 MiB | `engine/packages/config/src/config/pegboard.rs:395-398` |
| Preload total | 1 MiB | `engine/packages/config/src/config/pegboard.rs:449-451` |
| Pool metadata | 16 KiB | `docs/general/content/nodes.mdx:46` |
| Proxy retries | 7 attempts | `engine/packages/config/src/config/guard.rs:9` |
| Outbound redirects | 4 | `engine/packages/config/src/config/outbound.rs:68-70` |
| Envoy hash-ring virtual nodes | 8 | `engine/packages/config/src/config/pegboard.rs:516-522` |

## 8. Upgrade

### 8.1 Two kinds of version

A cluster has two unrelated version concepts.

| Version | Meaning | Applies to | Source |
|---------|---------|------------|--------|
| Node version | The application build version. A higher number wins allocation. | Node processes | `docs/general/content/versions.mdx:9-14` |
| Runtime protocol version | The wire and storage schema version that the fleet has agreed to speak. | `rivet-engine` processes | `engine/CLAUDE.md:75-80` |

The node version is an integer. The operator sets it with
`RIVET_ENVOY_VERSION`. It must come from the build, not from the clock at
runtime (`docs/general/content/versions.mdx:104`). It must increase on every
deploy. A rollback to a lower node version is unsafe, because allocation only
moves toward higher numbers (`docs/general/content/versions.mdx:14`).

The runtime protocol version has four kinds: `envoy`, `ups`,
`universaldb_commit`, and `epoxy`
(`engine/packages/config/src/runtime_protocols.rs:3-9`).

### 8.2 Admission

A new engine process checks admission before it starts any service. The check
runs in one serializable transaction.

```rust
// engine/packages/version-management/src/lib.rs:56-104
pub async fn check_engine_admission(
	config: &rivet_config::Config,
	pools: &rivet_pools::Pools,
) -> Result<()> {
	let current_version = semver::Version::parse(env!("CARGO_PKG_VERSION"))
		.context("failed to parse cargo pkg version as semver")?;

	tracing::debug!(%current_version, "checking engine admission");

	let protocols = pools
		.udb()?
		.txn("engine_check_admission", |tx|  {
			let config = config.clone();
			let current_version = current_version.clone();
			async move {
				if let Some(existing_version) = tx.read_opt(&keys::EngineVersionKey::new(), Serializable).await? {
					if !config.runtime.allow_version_rollback() {
						if current_version < existing_version {
							bail!(
								"{}",
								formatdoc!(
									"
									Rivet Engine has been rolled back to a previous version:
									- Last Used Version: {existing_version}
									- Current Version:   {current_version}
									Cannot proceed without potential data corruption.
									
									(If you know what you're doing, this error can be disabled in the Rivet config via `allow_version_rollback: true`)
									"
								)
							);
						}
					}
				}

				// NOTE: We serializably read this key and write it in the same txn making all engine
				// connections serial. This is by design
				tx.write(&keys::EngineVersionKey::new(), current_version)?;

				// NOTE: These writes rely on the conflict guarantees of the above `EngineVersionKey`
				update_protocol_versions(&tx).await
			}
		})
		.await?;

	config.set_protocols(protocols);

	Ok(())
}
```

The transaction is serial on purpose. Every engine process reads and writes the
same `EngineVersionKey` in one transaction. The comment states the reason: the
protocol-version writes rely on the conflict guarantee of that key
(`engine/packages/version-management/src/lib.rs:91-96`).

`allow_version_rollback` defaults to false
(`engine/packages/config/src/config/runtime.rs:201-203`). A lower engine version
therefore fails admission unless the operator sets it to true.

The check has an effect before any traffic moves. The transaction writes
`EngineVersionKey` with the running binary's version on every start
(`engine/packages/version-management/src/lib.rs:91-93`). If you start a newer
binary and then abandon the upgrade, the old binary is now blocked. The floor
rose even though the new version served no request. The only way back is
`allow_version_rollback: true`, and the source warns that this risks data
corruption (`engine/packages/version-management/src/lib.rs:78-84`). Treat the
first start of a new binary as a one-way step.

### 8.3 The hold-back loop

The fleet never jumps to a new protocol version while an older process still
heartbeats. A new process pins each protocol to the oldest live version.

```rust
// engine/packages/version-management/src/lib.rs:106-171
async fn update_protocol_versions(
	tx: &universaldb::Transaction,
) -> Result<rivet_config::RuntimeProtocols> {
	let now = util::timestamp::now();
	let mut protocols = rivet_build_meta::compiled_runtime_protocols();

	for protocol in &mut protocols {
		let protocol_version_subspace = universaldb::utils::Subspace::all()
			.subspace(&keys::ProtocolVersionKey::subspace(protocol.kind));
		// Read versions in increasing order to get the most out of date version first
		let mut stream = tx.get_ranges_keyvalues(
			universaldb::RangeOption {
				mode: StreamingMode::Small,
				..(&protocol_version_subspace).into()
			},
			Serializable,
		);

		while let Some(entry) = stream.try_next().await? {
			let (existing_key, update_ts) = tx.read_entry::<keys::ProtocolVersionKey>(&entry)?;

			// Check expired and delete
			if update_ts < now - PROTOCOL_VERSION_ENTRY_EXPIRED_THRESHOLD_MS {
				tracing::debug!(
					kind = %protocol.kind,
					version = existing_key.version,
					last_update_ts = update_ts,
					age_ms = now - update_ts,
					"deleting expired protocol version entry"
				);

				tx.delete(&existing_key);
				continue;
			}

			if protocol.compiled_version() < existing_key.version {
				bail!(
					"{}",
					formatdoc!(
						"
						Rivet Engine has been rolled back to a previous version:
						- Last Used `{0}` Protocol Version: {1}
						- Current `{0}` Protocol Version:   {2}
						Cannot proceed without potential data corruption.
						",
						protocol.kind,
						existing_key.version,
						protocol.compiled_version(),
					)
				);
			}

			protocol.set_override_version(existing_key.version);
			break;
		}

		// NOTE: Must write after reading to prevent RYOW
		tx.atomic_op(
			&keys::ProtocolVersionKey::new(protocol.kind, protocol.compiled_version()),
			&now.to_le_bytes(),
			MutationType::Max,
		);
	}

	Ok(protocols)
}
```

Three constants control the behaviour.

| Constant | Value | Effect |
|----------|-------|--------|
| `PROTOCOL_VERSION_ENTRY_EXPIRED_THRESHOLD_MS` | 30,000 | An entry older than 30 seconds is deleted |
| `PROTOCOL_VERSION_CHECK_INTERVAL` | 15 seconds | The background service rechecks every 15 seconds |
| Heartbeat write | `MutationType::Max` | Keeps the newest timestamp for one version |

The background service runs every 15 seconds and updates the process config when
the agreed set changes (`engine/packages/version-management/src/lib.rs:10-51`).
A call site that can speak more than one version must read the agreed version
from `config.protocols()`, not from the compiled constant
(`engine/CLAUDE.md:78-80`).

### 8.4 Safe order

The order below follows the code and the project documents. Do not shorten it.

```mermaid
sequenceDiagram
    participant O as "operator"
    participant E1 as "old rivet-engine"
    participant U as "UniversalDB"
    participant E2 as "new rivet-engine"
    O->>E2: "start new instance"
    E2->>U: "check_engine_admission txn"
    U-->>E2: "EngineVersionKey and oldest ProtocolVersionKey"
    E2->>E2: "config.set_protocols(oldest live version)"
    O->>E1: "SIGTERM"
    E1->>U: "stop protocol heartbeat"
    Note over U: "entry expired after 30000 ms"
    E2->>U: "update_protocol_versions every 15000 ms"
    U-->>E2: "newest live entry is now the compiled version"
    E2->>E2: "config.set_protocols(compiled version)"
    O->>O: "repeat for the next instance"
```

1. Read the release notes and confirm that the new binary is not older than the
   running binary. A downgrade fails admission by default. Remember that the
   first start raises the version floor (section 8.2).
2. Deploy one instance. The new instance starts at the oldest live protocol
   version, so it speaks the version that the fleet already speaks.
3. Stop the old instance with `SIGTERM`. The service manager gives graceful
   services time to stop, then stops UniversalDB
   (`engine/packages/service-manager/src/lib.rs:369-405`).
4. Wait at least 30 seconds for the old heartbeat to expire. The hold-back loop
   checks every 15 seconds.
5. Repeat for each instance. The last instance moves the fleet to the compiled
   version.
6. For nodes, give the new node a higher version. To move old Actors to the new
   version, set `drain_on_version_upgrade` on the pool with a `PUT` request to
   `/pools/{pool}?namespace={namespace}` (`docs/general/content/versions.mdx:229-235`).
   The default is `false`, and the two versions then run side by side
   (`docs/general/content/versions.mdx:238-241`).
7. Give each node at least 60 seconds between `SIGTERM` and `SIGKILL`
   (`docs/general/content/versions.mdx:291`). The Kubernetes worker manifest
   uses 2100 seconds (`self-host/workers/kubernetes/rivetkit-app.yaml:15-19`).

The service manager also handles a forced shutdown. It starts a timer when the
termination signal arrives. If a graceful service does not stop within the
force duration, the process exits. The force duration defaults to 10 minutes and
must be at least as large as the worker and Guard durations
(`engine/packages/config/src/config/runtime.rs:192-199`,
`engine/packages/config/src/config/mod.rs:330-340`).

### 8.5 Cross-datacenter upgrade

Epoxy cannot use the local heartbeat subspace. Its peers are replicas in other
datacenters, and each peer writes to its own database. Each replica publishes
the version that its own datacenter agreed on over HTTP. A request uses the
lower of the local version and the peer version.

```rust
// engine/packages/epoxy/src/protocol_version.rs:42-60
/// The version to encode a request to `replica_url` at.
///
/// Returns the lower of our own negotiated version and the peer's most recently discovered version.
/// This performs no I/O.
///
/// A replica the service has not reached yet falls back to our own negotiated version, which
/// reproduces the behavior from before peer discovery existed rather than downgrading whenever a
/// peer is briefly unreachable.
pub async fn negotiate(config: &rivet_config::Config, replica_url: &str) -> u16 {
	let local_version = config.protocols().epoxy.version();

	match PEER_VERSIONS
		.read_async(replica_url, |_, version| *version)
		.await
	{
		Some(peer_version) => local_version.min(peer_version),
		None => local_version,
	}
}
```

The probe interval is 60 seconds and the probe timeout is 5 seconds
(`engine/packages/epoxy/src/protocol_version.rs:21-28`). A multi-datacenter
upgrade therefore moves in steps. Upgrade one datacenter, wait for the peers to
discover it, then upgrade the next. The source does not give a maximum wait
beyond the 60-second refresh interval.

The fallback has a failure direction. When the peer has not been reached,
`negotiate` returns the local version
(`engine/packages/epoxy/src/protocol_version.rs:56-59`). If the local version is
newer than the peer's real version, the local replica encodes the request at the
new version. The old peer cannot decode that request. The 5-second probe and the
60-second refresh make this window real during an upgrade. Upgrade a datacenter
only after its peers report the new version, or accept that requests can fail
while peer discovery is incomplete. The source does not state how a decode
failure is retried.

### 8.6 VBARE compatibility

Persisted and wire data uses versioned Binary Application Record Encoding (BARE)
codecs through the `vbare` crate.
The repository rule says to use `vbare::OwnedVersionedData` for any persisted or
wire encoding (`CLAUDE.md:25`). The runner protocol is a worked example.

```rust
// engine/packages/runner-protocol/src/versioned.rs:8-59
pub enum ToClientMk2 {
	V4(v4::ToClient),
	V5(v5::ToClient),
	V7(v7::ToClient),
}

impl OwnedVersionedData for ToClientMk2 {
	type Latest = v7::ToClient;

	fn wrap_latest(latest: v7::ToClient) -> Self {
		ToClientMk2::V7(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		if let ToClientMk2::V7(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			4 => Ok(ToClientMk2::V4(rivet_util::serde::bare_from_slice!(
				payload
			)?)),
			5 => Ok(ToClientMk2::V5(rivet_util::serde::bare_from_slice!(
				payload
			)?)),
			6 | 7 => Ok(ToClientMk2::V7(rivet_util::serde::bare_from_slice!(
				payload
			)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToClientMk2::V4(data) => rivet_util::serde::bare_to_vec!(&data).map_err(Into::into),
			ToClientMk2::V5(data) => rivet_util::serde::bare_to_vec!(&data).map_err(Into::into),
			ToClientMk2::V7(data) => rivet_util::serde::bare_to_vec!(&data).map_err(Into::into),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok, Ok, Ok, Self::v4_to_v5, Self::v5_to_v7, Ok]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok, Self::v7_to_v5, Self::v5_to_v4, Ok, Ok, Ok]
	}
}
```

The compatibility rule is: a new version adds a converter. It does not change an
old converter retroactively. The internal guide states that migration code must
never be changed after it is written (`.claude/reference/vbare-migrations.md:95`).
For an operator, the consequence is simple. An upgrade that changes a protocol
needs every peer to be on a build that can decode the old version. The hold-back
loop enforces that for the fleet. The envoy client also retries a failed decode
as the previous version for serverless start payloads
(`engine/sdks/rust/envoy-client/src/handle.rs:785-796`).

### 8.7 Open source to Enterprise

The internal upgrade document is explicit. Stored rows are compatible in both
directions. Compaction workflows are not. Enterprise adds durable steps to the
compaction workflows. A workflow that is mid-flight during the binary swap will
diverge and raise `WorkflowError::HistoryDiverged`
(`docs-internal/engine/oss-to-ee-upgrade.md:22-36`).

The required procedure is:

1. Stop Actor traffic, or at least stop write traffic heavy enough to trigger
   compaction.
2. Let compaction drain, or force the running compaction workflows to
   completion.
3. Confirm that no compaction workflows are running.
4. Deploy the Enterprise build.
5. Configure `sqlite.workflow_cold_storage` if cold storage is wanted.

Source: `docs-internal/engine/oss-to-ee-upgrade.md:38-46`.

A downgrade from Enterprise to open source is not supported. The internal
document says to treat it as a restore from backup. Enterprise-only keys such as
`kafka`, `compute_gateway`, `metadata`, and `sqlite.workflow_cold_storage` are
rejected by the open-source `deny_unknown_fields`
(`docs-internal/engine/oss-to-ee-upgrade.md:51-62`).

## 9. Configuration

### 9.1 Top-level sections

Configuration comes from defaults, the system config directory or the
`--config` paths, then environment variables
(`engine/packages/config/src/lib.rs:55-90`). The sections are validated at load
time (`engine/packages/config/src/config/mod.rs:260-343`).

| Section | Type | Default | Effect |
|---------|------|---------|--------|
| `auth` | `Auth` | None, required | Admin token and JSON Web Token (JWT) settings |
| `guard` | `Guard` | `Guard::default()` | Public listener, TLS, timeouts, body limits |
| `api_peer` | `ApiPeer` | `::`:6421 | Private API listener |
| `pegboard` | `Pegboard` | `Pegboard::default()` | Scheduler thresholds and load balancer |
| `features` | `Features` | `GuardGatewayV3::Off` | Guard gateway rollout switch |
| `logs` | `Logs` | None | Redirect log directory |
| `topology` | `Topology` | One leader at 127.0.0.1:6420 | Datacenters and regional hosts |
| `database` | `Database` | `FileSystem` | RocksDB path or PostgreSQL URL |
| `pubsub` | `PubSub` | `Memory` | In-process or NATS message bus |
| `cache` | `Cache` | Enabled, in-memory | Cache layer |
| `clickhouse` | `ClickHouse` | None | Analytics database |
| `telemetry` | `Telemetry` | Enabled | Opt-out telemetry |
| `runtime` | `Runtime` | Defaults | Worker, shutdown, and rollback policy |
| `sqlite` | `Sqlite` | Defaults | Per-database quota and compaction |
| `metrics` | `Metrics` | `::`:6430 | Prometheus metrics listener |
| `pyroscope` | `Pyroscope` | None | Profiling server |
| `outbound` | `Outbound` | Deny private networks | Outbound request policy |

Two rows in this table are not extra systems to deploy.

`clickhouse` is optional. The pool setup returns `None` when the section is
absent (`engine/packages/pools/src/db/clickhouse.rs:9-16`). It is for analytics,
not for Actor state, and it is not one of the four parts.

`sqlite` configures the per-Actor SQLite databases that Depot stores. It is not a
separate database server. The section sets the quota for one Actor database and
the byte-rate throttles for Depot work
(`engine/packages/config/src/config/sqlite.rs:18-50`). The 10 GiB value in
section 7.4 is the cap for one Actor database, not for the cluster.

The authentication block is mandatory. The loader fails when `auth` is absent,
and validation fails when `admin_token` is empty.

```rust
// engine/packages/config/src/config/auth.rs:23-45
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Auth {
	pub admin_token: Secret<String>,

	/// Restores legacy unauthenticated access behavior for actor and routing endpoints.
	#[serde(default)]
	pub insecure_allow_unauthenticated: bool,

	/// Short-lived, namespace-scoped JWT authentication. Enabled by default.
	#[serde(default)]
	pub jwt: Jwt,
}

impl Auth {
	pub fn validate(&self, desired_issuer: &str) -> Result<()> {
		if self.admin_token.read().is_empty() {
			anyhow::bail!("auth.admin_token cannot be empty");
		}

		self.jwt.validate(desired_issuer)
	}
}
```

A JWT is enabled by default. The default lifetime is one hour,
the maximum is 24 hours, key rotation is seven days, the publish lead is ten
minutes, the maximum signing lifetime is 14 days, the verifier cache is 60
seconds, and stale verification is allowed for five minutes
(`engine/packages/config/src/config/auth.rs:10-21`).

### 9.2 Environment variables

The engine prefix is `RIVET`, the separator is `__`, and the list separator is
`,` (`engine/packages/config/src/lib.rs:73-80`). A double underscore maps to a
nested key. A single underscore stays inside a key name.

| Variable | Type | Default | Effect |
|----------|------|---------|--------|
| `RIVET__AUTH__ADMIN_TOKEN` | string | none | Required admin token |
| `RIVET__FILE_SYSTEM__PATH` | path | user data dir or `./data/db` | RocksDB directory |
| `RIVET__POSTGRES__URL` | string | `postgresql://postgres:postgres@127.0.0.1:5432/postgres` | Shared database URL |
| `RIVET__GUARD__HOST` | IP address | `::` | Public listener address |
| `RIVET__GUARD__PORT` | integer | 6420 | Public listener port |
| `RIVET__API_PEER__PORT` | integer | 6421 | Private API port |
| `RIVET__METRICS__PORT` | integer | 6430 | Metrics port |
| `RIVET__TELEMETRY__ENABLED` | boolean | true | Telemetry switch |
| `RIVET__TOPOLOGY__DATACENTER_LABEL` | integer | 1 | Local datacenter label |
| `RIVET__OUTBOUND__ALLOW_PRIVATE_NETWORKS` | boolean | false | Allow private outbound targets |
| `RIVET__PEGBOARD__ENVOY_LOST_THRESHOLD` | integer | 15000 | Node loss threshold in milliseconds |
| `RIVET__PEGBOARD__ACTOR_STOP_THRESHOLD` | integer | 1800000 | Actor stop budget in milliseconds |
| `RIVET__RUNTIME__ALLOW_VERSION_ROLLBACK` | boolean | false | Permit a lower engine version |

The node variables are separate from the engine prefix.

| Variable | Type | Default | Effect |
|----------|------|---------|--------|
| `RIVET_ENDPOINT` | URL | none | Address of the control plane |
| `RIVET_TOKEN` | string | none | Node token |
| `RIVET_NAMESPACE` | string | `default` | Namespace slug |
| `RIVET_POOL` | string | `default` | Pool name |
| `RIVET_ENVOY_VERSION` | integer | none | Node build version |
| `RIVET_TOTAL_SLOTS` | integer | none | Actor slots on the node |
| `RIVETKIT_RUNTIME_MODE` | `envoy` or `serverless` | `envoy` | Runtime mode |

### 9.3 Listeners and TLS

Three listeners have defaults. All three bind to the unspecified IPv6 address.

```rust
// engine/packages/config/src/defaults.rs:1-14
pub mod hosts {
	use std::net::{IpAddr, Ipv6Addr};

	pub const GUARD: IpAddr = IpAddr::V6(Ipv6Addr::UNSPECIFIED);
	pub const API_PEER: IpAddr = IpAddr::V6(Ipv6Addr::UNSPECIFIED);
	pub const METRICS: IpAddr = IpAddr::V6(Ipv6Addr::UNSPECIFIED);
}

pub mod ports {
	pub const GUARD: u16 = 6420;
	pub const API_PEER: u16 = 6421;

	pub const METRICS: u16 = 6430;
}
```

The configuration type for TLS exists. It holds four certificate and key paths:
one pair for Actor traffic and one pair for the private API. The serving code in
this commit does not read them. The certificate resolver returns `Ok(None)`
before any path is used. Setting the four paths changes nothing in this commit.
Do not treat them as a TLS enable switch.

```rust
// engine/packages/config/src/config/guard.rs:353-369
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
#[derive(Default)]
pub struct Https {
	pub port: u16, // Port for HTTPS traffic
	pub tls: Tls,  // TLS configuration
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
#[derive(Default)]
pub struct Tls {
	pub actor_cert_path: PathBuf,
	pub actor_key_path: PathBuf,
	pub api_cert_path: PathBuf,
	pub api_key_path: PathBuf,
}
```

```rust
// engine/packages/guard/src/tls.rs:83-87
#[tracing::instrument(level = "debug", skip_all)]
pub async fn create_cert_resolver(
	_ctx: &gas::prelude::StandaloneCtx,
) -> Result<Option<CertResolverFn>> {
	return Ok(None);
```

The Guard server checks for that resolver. With no resolver, it logs “No TLS
certificate resolver provided, HTTPS will not work properly”
(`engine/packages/guard-core/src/server.rs:55-63`). Terminate TLS at a load
balancer or a proxy, and keep HTTP inside the private network. The BYOC
instructions offer that shape as “private HTTP, or public HTTPS plus private
HTTP” (`frontend/src/app/byoc/agent-instructions.ts:34`).

### 9.4 Limits and outbound policy

The outbound policy is a security control. Its defaults deny every
non-globally-routable destination except loopback. A self-hosted deployment that
points a serverless runner at a Docker Compose service name must enable private
networks.

```rust
// engine/packages/config/src/config/outbound.rs:10-41
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Outbound {
	/// Allow destinations that resolve to loopback addresses (127.0.0.0/8, ::1).
	///
	/// Enabled by default so local development against `http://localhost:...` works without
	/// configuration.
	pub allow_loopback: Option<bool>,
	/// Allow destinations that resolve to private, link-local, shared (CGNAT), or otherwise
	/// non-globally-routable addresses.
	///
	/// Self-hosted deployments that point runners at addresses on their own network, such as a
	/// Docker Compose service name, need this enabled.
	pub allow_private_networks: Option<bool>,
	/// Allow plaintext `http://` destinations. When disabled only `https://` is permitted.
	pub allow_insecure_scheme: Option<bool>,
	/// Additional CIDRs that are always permitted, evaluated after `deny_cidrs`.
	///
	/// Use this to reach a specific internal service without opening up the whole private range.
	pub allow_cidrs: Option<Vec<String>>,
	/// Additional CIDRs that are always denied. Takes precedence over every allow rule.
	pub deny_cidrs: Option<Vec<String>>,
	/// Hostnames that are always permitted, whatever they resolve to.
	///
	/// Matched exactly and case-insensitively, with no wildcards. Use this for a first-party
	/// service whose address is not known ahead of time, such as a Kubernetes service name.
	/// Anything served at a listed host becomes reachable by every runner config, so list
	/// individual hosts rather than a shared entry point.
	pub allow_hosts: Option<Vec<String>>,
	/// Maximum number of redirects to follow. Every hop is re-checked against this policy.
	pub max_redirects: Option<usize>,
}
```

The development configuration enables private networks for exactly this reason
(`self-host/compose/dev/rivet-engine/config.jsonc:24-26`).

The limits in section 7.4 are the main hard bounds. They apply at the Guard
edge, at the Actor database, and at the node connection. A deployment that needs
a larger value must change the named key. It must not assume that the value is
unlimited.

## 10. Backup and restore

### 10.1 What must be backed up

The backup set follows the storage path.

| Item | Why | Reproducible? |
|------|-----|---------------|
| UniversalDB rows | Namespaces, Actors, runners, workflows, versions, and the Depot page and key-value rows | No |
| Bucket objects | The documented durable copy | No, when the bucket is the primary store |
| Instance configuration | Topology, auth, storage, limits | Yes, from source control |
| TLS certificates and keys | Public endpoint identity | Yes, from the certificate authority |
| `auth.admin_token` | Access control | Yes, from a secret store |

UniversalDB rows and Actor Depot data are one item, not two. Depot keeps its
pages and key-value rows in UniversalDB (section 3.1). On `FileSystem` they all
live in one RocksDB directory. A backup that copies only the "metadata" and not
the pages is incomplete.

For `FileSystem`, the UniversalDB data is the RocksDB directory. Back it up with
a file system snapshot, or use the per-database export in section 10.2.

For `Postgres`, use the normal PostgreSQL backup tooling. The repository does
not include a PostgreSQL backup command.

For the documented bucket model, the bucket is the durable store. The repository
does not contain a bucket backup or restore tool. The Enterprise build is not in
this snapshot. **Unverified:** the repository does not state a bucket lifecycle,
versioning, or replication policy. Use the provider's own object versioning and
replication features, and confirm them against the Enterprise documentation.

### 10.2 Per-Actor export and import

The `rivet-engine depot` command has an export and an import. The export reads
one Depot database and writes a directory with a manifest, a key-value file, and
a cold directory. The import writes that directory back into a local RocksDB
database.

```rust
// engine/packages/engine/src/commands/depot.rs:17-33
#[derive(Parser)]
pub enum SubCommand {
	/// Diagnose Depot-backed SQLite storage for one database
	Doctor(DoctorOpts),
	/// Execute SQL against one Depot-backed SQLite database
	Execute(ExecuteOpts),
	/// Export one Depot database without mutating its storage
	Export(ExportOpts),
	/// Import a Depot export into an empty local RocksDB database
	Import(ImportOpts),
	/// Resolve actor IDs to their owning namespace IDs
	LookupActorNamespaces(LookupActorNamespacesOpts),
	/// Scan current Depot databases for a specific corruption pattern using snapshot reads only
	ScanCorruption(ScanCorruptionOpts),
	/// Repair one Depot database after writing and verifying an exact backup
	Repair(RepairOpts),
}
```

The export selector is either an Actor identifier or a bucket and database pair.
The command requires exactly one selector.

```rust
// engine/packages/engine/src/commands/depot.rs:133-180
#[derive(Parser)]
pub struct ExportOpts {
	#[arg(long)]
	bucket_id: Option<Uuid>,
	#[arg(long)]
	database_id: Option<String>,
	#[arg(long)]
	actor_id: Option<Id>,
	#[arg(long)]
	output: PathBuf,
}

impl ExportOpts {
	pub async fn execute(self, config: rivet_config::Config) -> Result<()> {
		let pools = rivet_pools::Pools::new(config.clone()).await?;
		let udb = pools.udb()?;
		let bucket_database = self.bucket_id.is_some() || self.database_id.is_some();
		let actor = self.actor_id.is_some();
		ensure!(
			usize::from(bucket_database) + usize::from(actor) == 1,
			"provide exactly one selector: --bucket-id/--database-id or --actor-id"
		);
		let target = if let Some(actor_id) = self.actor_id {
			let namespace_id = lookup_actor_namespace_id(&udb, actor_id).await?;
			depot_transfer::ExportTarget {
				bucket_id: namespace_id,
				database_id: actor_id.to_string(),
			}
		} else {
			depot_transfer::ExportTarget {
				bucket_id: Id::v1(
					self.bucket_id
						.context("--bucket-id is required with --database-id")?,
					0,
				),
				database_id: self
					.database_id
					.context("--database-id is required with --bucket-id")?,
			}
		};
		let summary = depot_transfer::export_database(&config, &udb, target, &self.output)
			.await
			.context("export Depot database")?;

		println!("{}", serde_json::to_string_pretty(&summary)?);
		Ok(())
	}
}
```

The `--bucket-id` flag has a second meaning. In this command it is the namespace
identifier, not an object-storage bucket. The actor selector maps it directly:
`bucket_id: namespace_id` (`engine/packages/engine/src/commands/depot.rs:155-160`).
Everywhere else in this document, "bucket" means object storage. The flag keeps
an older name.

An import is restricted to the local file system backend. It cannot import into
PostgreSQL.

```rust
// engine/packages/engine/src/commands/depot_transfer.rs:195-201
pub fn verify_local_import_config(config: &rivet_config::Config) -> Result<()> {
	ensure!(
		matches!(config.database(), DatabaseConfig::FileSystem(_)),
		"Depot import is restricted to the local file_system/RocksDB database backend"
	);
	Ok(())
}
```

The export is per database. It is not a cluster backup. The manifest records the
bucket identifier, the database identifier, the branch identifiers, and a
checksum for each file (`engine/packages/engine/src/commands/depot_transfer.rs:58-98`).
Use it to move or rescue one Actor database, not to restore a cluster.

The practical commands are:

```bash
# Per-Actor export. The output directory must not exist.
rivet-engine depot export --actor-id <ACTOR_ID> --output /backup/actor-<ACTOR_ID>

# Import into an empty local RocksDB database.
rivet-engine depot import --input /backup/actor-<ACTOR_ID>
```

The command names and flags come from
`engine/packages/engine/src/commands/depot.rs:17-33` and
`engine/packages/engine/src/commands/depot.rs:133-199`. The source does not give
an example command line, so the two lines above are a construction from the
flags, not a quoted guide.

### 10.3 How a lost instance rejoins

A lost instance is an instance that stopped and lost its local disk. The
recovery depends on the storage path.

| Storage path | Recovery |
|--------------|----------|
| `FileSystem` | Restore the disk snapshot. Without it, the cluster state is gone. |
| `Postgres` + `NATS` | Start a new instance with the same PostgreSQL URL, NATS addresses, and topology. It reads the shared state. |
| Documented bucket model | Start a new identical instance with the same bucket and topology. It reads the bucket. |

The `rivet-engine start` path always runs admission first
(`engine/packages/engine/src/commands/start.rs:71-76`). A new instance with an
older binary fails admission. A new instance with the same or a newer binary
joins and writes its version.

The source does not contain a command that registers a replacement instance by
identifier. The instance is not a named member. It is an identical process with
the same configuration.

### 10.4 What has no tool

The repository has no cluster-level backup command. It has no bucket restore
command. It has no command that rebuilds an entire UniversalDB from a bucket.
The reason follows from section 10.3. In the documented model, the bucket is the
primary store, so a new instance reads it at start and needs no rebuild step. In
this open-source build, the bucket is not a store, so no rebuild command exists
to write. A cluster that runs the open-source build and needs disaster recovery
must protect the database itself, not a bucket.

The `depot repair vacuum` command repairs one database after writing a verified
pre-repair export, but it is a repair tool, not a restore tool
(`engine/packages/engine/src/commands/depot.rs:444-464`).

The `database nuke` command deletes the entire local RocksDB directory after a
`--yes` flag (`engine/packages/engine/src/commands/db/mod.rs:51-66`). It is a
destructive reset. It is not a cleanup command for old rows.

## 11. Security posture

### 11.1 The trust boundary

The cluster has three zones.

| Zone | Who can reach it | What it trusts |
|------|------------------|----------------|
| Public edge | Clients, node processes | A bearer token or a JWT |
| Private instance network | Other instances | The shared database and the topology configuration |
| Node connection | Outbound only, from the node to the control plane | The token presented at connect time |

A node opens the connection. The control plane does not open a connection to the
node. This is the reason a node needs no inbound port
(`docs/general/content/nodes.mdx:42`).

An instance trusts the shared database. An operator who can write to the
database can change the cluster. Protect the database with the same care as the
admin token.

### 11.2 Tokens

The engine requires an admin token. `Auth::validate` rejects an empty value
(`engine/packages/config/src/config/auth.rs:37-44`). The token is a `Secret`,
so the debug output does not print it.

A node presents its token in the WebSocket subprotocol list, not in a header
alone. The TypeScript runner builds the list as `rivet` and
`rivet_token.<token>` (`rivetkit-typescript/packages/engine-runner/src/mod.ts:756-759`).
The envoy client does the same
(`engine/sdks/rust/envoy-client/src/connection/native.rs:100-106`). A proxy must
forward the `Sec-WebSocket-Protocol` header unchanged.

The Guard router calls `check_connection_auth` before it hands the socket to the
runner or envoy service (`engine/packages/guard/src/routing/runner.rs:56-61`,
`engine/packages/guard/src/routing/envoy.rs:56-61`).

A JWT is namespace-scoped and short-lived. The defaults are in section 9.1. The
`insecure_allow_unauthenticated` flag restores legacy unauthenticated access. Do
not enable it in production (`engine/packages/config/src/config/auth.rs:28-30`).

### 11.3 TLS

TLS is not active on Guard in this commit. The configuration has four
certificate paths for the two traffic classes
(`engine/packages/config/src/config/guard.rs:361-369`). The resolver returns
`Ok(None)` before it reads a path (`engine/packages/guard/src/tls.rs:84-87`).
Terminate TLS at a load balancer or a proxy. The source does not state a minimum
TLS version. The source does not state a cipher policy. Set both at the load
balancer.

The public URL in each datacenter is the origin that clients use
(`engine/packages/config/src/config/topology.rs:164-165`). The `valid_hosts` list
restricts regional endpoints to known names
(`engine/packages/config/src/config/topology.rs:171-178`). Set it when the
topology has more than one datacenter, because validation requires it
(`engine/packages/config/src/config/mod.rs:292-300`).

### 11.4 Outbound policy

The outbound policy protects the internal network from a caller who can
configure a serverless runner URL. The default denies private, link-local,
shared, and otherwise non-globally-routable addresses
(`engine/packages/config/src/config/outbound.rs:4-9`). The default allows
loopback and plain HTTP.

For production, set `allow_loopback` to false, set `allow_insecure_scheme` to
false, and leave `allow_private_networks` false unless a runner must reach a
private address. When a private address is required, prefer `allow_hosts` or
`allow_cidrs` over the whole private range
(`engine/packages/config/src/config/outbound.rs:26-38`).

### 11.5 Bucket permissions

The repository does not contain a bucket policy. The compaction constants show
object upload and object delete calls
(`engine/packages/depot/src/conveyer/constants.rs:190-203`). Grant those two
operations on the cluster bucket, and no others that the source does not show. Do
not grant access to other buckets. **Unverified:** the exact action names, the
required resource scope, and whether a list permission is needed are not in this
source.

## 12. Failure drills

### 12.1 Lost instance

| Item | Detail |
|------|--------|
| Symptom | One public endpoint stops answering. The load balancer removes it. |
| Detection | The instance misses its health probe. Kubernetes uses `/health` on port 6421 (`self-host/control-plane/kubernetes/03-rivet-engine-deployment.yaml:59-72`). |
| Recovery | Start a new identical instance. With `Postgres`, it reads the shared state. With `FileSystem`, restore the disk first. |
| Mechanism | Admission runs at start (`engine/packages/engine/src/commands/start.rs:71-76`). The local database is reopened from the configured path (`engine/packages/pools/src/db/udb.rs:57-59`). |
| Source limit | The source does not state an instance-loss timeout. The quorum claim is “2 of 3” (`README.md:155`). |

The source does not state how the load balancer detects a lost instance. That is
outside the engine. The health route exists on the private API port.

### 12.2 Lost bucket access

| Item | Detail |
|------|--------|
| Symptom | Cold reads fail, or compaction cannot publish or retire objects. |
| Detection | The `SQLITE_S3_REQUEST_FAILURES_TOTAL` counter is registered (`engine/packages/depot/src/metrics.rs:173-178`), but this tree contains no code that increments it. Watch compaction job failures and logs instead. |
| Recovery | Restore the credential or the network path. The source does not give a bucket-specific recovery procedure. |
| Mechanism | Compaction uploads objects and deletes them after the grace period (`engine/packages/depot/src/conveyer/constants.rs:190-203`). |
| Source limit | The open-source build does not write cold rows, so this drill applies to the Enterprise build. The registered counter stays at zero in the open-source build. |

**Unverified:** the source does not state whether a cold-read failure is
retried, served from the hot tier, or returned as an error. Do not claim a
specific user-visible result.

### 12.3 Lost node

| Item | Detail |
|------|--------|
| Symptom | Actors on the node stop answering. Requests to those Actors stall, then retry. |
| Detection | The control plane marks the runner or envoy lost after the threshold. |
| Recovery | The Actors reschedule to another node in the pool. |
| Mechanism | `runner_lost_threshold` defaults to 15,000 ms (`engine/packages/config/src/config/pegboard.rs:267-269`). `envoy_lost_threshold` also defaults to 15,000 ms (`engine/packages/config/src/config/pegboard.rs:378-380`). |
| Source conflict | `docs/general/content/versions.mdx:298` says 40 seconds for `envoy_lost_threshold`. The code default is 15 seconds. The code wins. |

The server sends both thresholds to the node in the init message. The legacy
path sends `runner_lost_threshold` and `actor_stop_threshold`
(`engine/packages/pegboard-runner/src/conn.rs:391-397`). The envoy path sends
`envoy_lost_threshold`, `actor_stop_threshold`, and
`max_response_payload_size` (`engine/packages/pegboard-envoy/src/conn.rs:152-159`).

A graceful node stop is different. The node stops its Actors first. Give the
node at least 60 seconds between `SIGTERM` and `SIGKILL`
(`docs/general/content/versions.mdx:283-291`). The worker manifest uses 2100
seconds (`self-host/workers/kubernetes/rivetkit-app.yaml:15-19`).

### 12.4 Full disk

| Item | Detail |
|------|--------|
| Symptom | UniversalDB writes fail. RocksDB reports an input/output error. |
| Detection | Engine logs show write failures. This document found no dedicated free-space metric in the engine packages. |
| Recovery | Free space, or move the file system path to a larger disk and restore the data. |
| Mechanism | The local path is `file_system.path` (`engine/packages/config/src/config/db/mod.rs:23-36`). |
| Source limit | The source does not define a reserve or a read-only mode. |

A full disk also stops compaction, because compaction writes to the same
database. The throttle code can slow compaction, but it cannot make room on the
disk. The source does not state a minimum free-space rule.

### 12.5 Clock skew

| Item | Detail |
|------|--------|
| Symptom | Protocol version entries expire too early or too late. The version floor can stay low after an upgrade. |
| Detection | Compare host time with a trusted time source. |
| Recovery | Correct the clock with Network Time Protocol (NTP). |
| Mechanism | A version entry stores `now.to_le_bytes()` and the write uses `MutationType::Max` (`engine/packages/version-management/src/lib.rs:162-167`). The expiry check compares the reader's `now` with that stored value (`engine/packages/version-management/src/lib.rs:109-139`). |
| Source limit | The source does not state a permitted skew. |

The comparison uses two clocks. The writer stores its clock value. Each reader
compares that value with its own clock. Four cases follow.

| Clock fault | Effect on the version floor |
|-------------|------------------------------|
| Writer clock ahead | The stored timestamp is in the future. The entry survives past its real heartbeat and pins the fleet to the old version. |
| Writer clock behind | `Max` keeps the largest value, so one behind writer is ignored while a healthy writer is present. If all writers are behind, entries expire early. |
| Reader clock ahead | A live entry looks expired, so that instance deletes it and the fleet can advance early. |
| Reader clock behind | The entry looks live longer, so the floor stays low longer. |

The writer-ahead case is the dangerous one. The entry does not expire when the
old instance stops, because the timestamp it left is in the future. The fleet
stays on the older protocol until the clock catches up. The source does not give
a tolerance. Run a time service on every instance.

### 12.6 Drill summary

| Drill | First signal | First action | Do not do |
|-------|--------------|--------------|-----------|
| Lost instance | Health probe fails | Start an identical instance | Start an instance with an older binary |
| Lost bucket access | Compaction job fails (Enterprise). The S3 counter does not rise in open source | Restore credentials and network | Assume the hot tier has every page |
| Lost node | Actor requests stall | Wait for the loss threshold, then confirm rescheduling | Lower the threshold to force a fast move without testing |
| Full disk | Write errors in the log | Free space or move the disk | Delete the RocksDB directory |
| Clock skew | Version or node loss flaps | Correct time with NTP | Set `allow_version_rollback` to hide the symptom |

## 13. A minimum production checklist

This checklist follows only the source in this document. It does not add steps
from the public guides.

1. Choose the storage path. Use `Postgres` and `NATS` for more than one
   open-source instance. Use one `FileSystem` instance only on a single host.
2. Set `auth.admin_token` from a secret. Do not leave the development token.
3. Set `topology.datacenter_label` and every datacenter entry. Give each entry a
   public URL, a peer URL, and `valid_hosts`.
4. Set the Guard listener. Terminate TLS at the load balancer, because the
   Guard certificate resolver returns `Ok(None)` in this commit.
5. Set the outbound policy. Keep private networks denied unless a runner needs
   them.
6. Run three instances in three zones, one per zone (`README.md:155`). In this
   build, give them PostgreSQL and NATS.
7. Create the node pool before the first node connects
   (`docs/general/content/nodes.mdx:25`).
8. Give every node a token, a namespace, a pool, and a build version.
9. Set `RIVET_ENVOY_VERSION` from the build. Increase it on every deploy.
10. Keep the instance disk on a volume. In the Enterprise bucket model, keep
    the bucket in the same region.
11. Back up the shared database, or the RocksDB directory, and the
    configuration.
12. Monitor `/health` on port 6421 and metrics on port 6430. The
    `SQLITE_S3_REQUEST_FAILURES_TOTAL` counter exists, but nothing in this
    open-source tree increments it, so do not depend on it here.
13. Upgrade one instance at a time. Wait for the 30-second protocol entry
    expiry. Keep the node version monotonic.

## What to read next

- [04 Pegboard](04-pegboard-scheduling.md) explains the scheduler, runner pools,
  and the runner protocol in depth.
- [07 UniversalDB and buckets](07-universaldb-and-buckets.md) explains the
  key space, the drivers, and the bucket storage model.
- [14 Observability and operations](14-observability-and-operations.md) explains
  telemetry, metrics, logs, and the command line interface.

## Glossary

> **Actor**: A keyed, durable process that Rivet schedules on a node.
>
> **bucket**: An object storage container. It is the only external durable store
> in the documented model.
>
> **control plane**: The set of `rivet-engine` processes that schedule Actors and
> route requests.
>
> **Envoy**: The newer node connection path. The client is in
> `engine/sdks/rust/envoy-client`, and the server path is `/envoys/connect`.
>
> **Node**: An application process that runs the Rivet SDK and hosts Actors.
>
> **Runner**: The older node connection path. The client is the TypeScript
> `engine-runner`, and the server path is `/runners/connect`.
>
> **UniversalDB**: The key-value database under every durable record. Open source
> uses RocksDB or PostgreSQL. The documented Enterprise model uses a bucket.
>
> **BYOC**: Bring Your Own Cloud. The stack runs in the customer VPC and Rivet
> operates it over an outbound connection.
