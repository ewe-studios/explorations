---
title: 02 Control Plane Architecture
---

# 02 Control Plane Architecture

The `rivet-engine` binary turns one configuration and one set of shared pools (section 7) into a running control plane. A control plane is the set of processes that schedule Actors and serve the public Application Programming Interface (API). An Actor is a keyed, stateful process. A namespace is a named container for Actors; the `Namespace` type stores `namespace_id` and `name` (`types/src/namespaces.rs:4-9`). An Actor name is the `name` string on the `Actor` record (`types/src/actors.rs:8-10`). An Actor key is the optional `key` string on the same record (`types/src/actors.rs:10`). The reservation in section 3.4 makes the tuple of namespace, Actor name, and Actor key unique. The binary parses the command line, builds a Tokio asynchronous runtime, loads and validates the configuration, initializes telemetry, creates the pool set, and starts a fixed list of services. Tokio is a Rust library that polls asynchronous tasks. This document follows that path from `main()` to a serving process. It also states which parts belong to one control plane instance and which parts belong to the whole cluster. Here, one control plane instance means one process of `rivet-engine`. Section 4 separates that process scope from the datacenter and cluster scopes. A path that starts with `engine/src/`, such as `engine/src/main.rs`, is shorthand for `engine/packages/engine/src/main.rs`. The source is commit `4a58441c50d1ce20d8803cfb1ab7c3285a5141d6`, version 2.3.31. All paths are relative to `/Users/darkvoid/GitSources/src.Rivet/rivet` unless the text names another root. A path that starts with a package name, such as `config/src/lib.rs`, is shorthand for `engine/packages/config/src/lib.rs`.

The central finding is that the binary has two layers. The first layer is process machinery: the runtime, the configuration, the pools, and the signal handler. The second layer is the service list. The service list is small and explicit. The database and the message bus are not separate daemons that the binary spawns. They are drivers that the pool layer selects from the configuration.

## 1. The entry point and the command line

The binary starts in `engine/packages/engine/src/main.rs`. The file is 60 lines. It defines a parser built with Clap, the Rust argument parser, a synchronous `main`, and an asynchronous `main_inner`. The split exists because Sentry, the error-reporting service, has a startup constraint. Sentry stores its client in a Hub. A Hub is a per-thread handle to the client and the current scope. The process Hub is the Hub of the thread that calls `sentry::init`. A new thread copies the client from the process Hub on its first use of the Sentry API (`sentry-core` 0.45.0 `src/hub_impl.rs:9-17`). The Sentry Rust documentation therefore requires `sentry::init` before an asynchronous runtime starts or a thread spawns. An asynchronous `main` starts the runtime first, so its worker threads can copy an empty Hub. Events from those threads then have no client to receive them. A comment in the telemetry crate states the rule (`engine/packages/telemetry/src/lib.rs:6`).

Rivet removes the attribute macro: `main` is synchronous (`main.rs:28-31`). The synchronous `main` alone does not satisfy the rule. `rivet_runtime::run` calls `build_tokio_runtime_builder()` and then `rt_builder.build()` (`runtime/src/lib.rs:27-28`). The `build()` call creates the worker threads. The runtime wrapper then spawns the metrics sampler at `runtime/src/lib.rs:39`, before it polls `main_inner`. The Sentry client binds later, inside `main_inner` after configuration load (`main.rs:46`; `telemetry/src/lib.rs:12-18`). A worker thread that uses the Sentry API before that bind keeps a client-less Hub for its lifetime. The split does not achieve the Sentry rule. The synchronous `main` controls when `main_inner` is created, but `sentry::init` still runs inside the runtime, after the worker threads exist. The telemetry comment gives the intent, not an achieved property. A worker thread that uses the Sentry Hub before the bind keeps a client-less Hub for its lifetime. Every Sentry event from that thread is dropped. Later Sentry statements in this document describe the installed path, not guaranteed delivery.

```rust
// engine/packages/engine/src/main.rs:10-24
#[derive(Parser)]
#[command(name = "Rivet", version, long_version = LONG_VERSION.as_str(), about)]
struct Cli {
	#[command(subcommand)]
	command: SubCommand,

	/// Path to the config file or directory of config files
	#[clap(long, global = true)]
	config: Vec<PathBuf>,
}

fn main() -> Result<()> {
	rivet_runtime::run(main_inner()).transpose()?;
	Ok(())
}
```

The `Cli` type has two fields. `command` is one `SubCommand` value. `config` is a vector of paths. The `--config` flag is global, so it is accepted before or after the subcommand. The flag can repeat. Each path can name a file or a directory.

The program name is `"Rivet"`, with a capital letter. The version comes from the package metadata. The long version comes from `rivet_build_meta::pretty_print()`. The binary stores the result in `LONG_VERSION`, a `Lazy<String>` (`engine/src/main.rs:8`). Clap evaluates that value on first use. The function formats `VERSION`, `GIT_SHA`, `BUILD_TIMESTAMP`, the compiler version, the host, the target, and the profile into one multi-line string (`build-meta/src/lib.rs:78-89`). The first log line reads `VERSION`, `GIT_SHA`, and `BUILD_TIMESTAMP` directly (`main.rs:29-34`; the call occupies those file lines). It therefore omits the compiler version, the compiler host, the Cargo target, and the Cargo profile that `pretty_print()` adds (`build-meta/src/lib.rs:78-88`). The first log line does not call `pretty_print()`. The omission means that a log line alone cannot identify the compiler, host, target, or profile of the build that produced it.

`main()` calls `rivet_runtime::run(main_inner())`. The call returns `Option<Result<()>>`. `transpose()` converts it to `Result<Option<()>>`, and `?` returns any error. The `None` case means the runtime was shut down manually. In that case the process exits with success. That path includes the third termination signal, which aborts the graceful drain and calls `rivet_runtime::shutdown()` (`service-manager/src/lib.rs:394-398`). The exit code is 0 even after this forced abort (`main.rs:30-33`). A supervisor therefore sees a successful exit.

`main_inner()` performs six steps in a fixed order.

```rust
// engine/packages/engine/src/main.rs:26-60
async fn main_inner() -> Result<()> {
	let cli = Cli::parse();

	tracing::info!(
		version=%rivet_build_meta::VERSION,
		git_sha=%rivet_build_meta::GIT_SHA,
		built_at=%rivet_build_meta::BUILD_TIMESTAMP,
		"starting rivet",
	);

	// Load config
	let config = rivet_config::Config::load(
		&cli.config,
		rivet_build_meta::build_meta(),
		rivet_build_meta::compiled_runtime_protocols(),
	)
	.await?;
	tracing::info!(config=?*config, "loaded config");

	// Initialize telemetry (does nothing if telemetry is disabled)
	let _guard = rivet_telemetry::init(&config);

	// Build run config
	let run_config = Arc::new(run_config::config(config.clone()).inspect_err(|err| {
		rivet_telemetry::capture_error(err);
	})?);

	// Execute command
	cli.command
		.execute(config, run_config)
		.await
		.inspect_err(|err| {
			rivet_telemetry::capture_error(err);
		})
}
```

The order matters for diagnosis. The first log line uses the tracing subscriber that `rivet_runtime::run` installed before it called `main_inner()`. The configuration load happens before Sentry initialization. A configuration error therefore returns from `main_inner()` without a Sentry capture. Errors from the run configuration and from the command are captured. `capture_error` still runs when telemetry is disabled, but no client receives the event (`telemetry/src/lib.rs:55-57`).

`rivet_config::Config::load` receives three arguments. The first is the list of paths. The second is the build metadata of this binary. The third is the set of protocol versions compiled into this binary. Section 3 explains each argument.

After the load, `rivet_telemetry::init(&config)` returns an optional Sentry guard. The guard is bound to `_guard`, so it lives until the end of `main_inner()`. If `telemetry.enabled` is false, the function returns `None`. The field defaults to enabled, so Sentry is on unless the operator disables it (`config/src/config/telemetry.rs:10-14`). Section 14 covers the details.

`run_config::config(config.clone())` builds the service list. The function ignores its configuration argument at this commit; the parameter is named `_rivet_config` (`engine/packages/engine/src/run_config.rs:4`). The result is wrapped in an `Arc`, because `SubCommand::execute` takes the alias `RunConfig`, which is `Arc<RunConfigData>` (`engine/packages/service-manager/src/lib.rs:111-115`).

The final call dispatches the subcommand. Each variant has an `execute` method. The methods take the configuration and, for `start`, the run configuration. Section 10 describes each subcommand.

The startup sequence is:

```mermaid
flowchart TD
    A["main()"] --> B["rivet_runtime::run(main_inner())"]
    B --> C["build_tokio_runtime_builder()"]
    C --> C2["rt_builder.build()"]
    C2 --> D["init_otel_providers()"]
    D --> E["traces::init_tracing_subscriber()"]
    E --> F["tokio::spawn(sample_runtime_metrics)"]
    F --> G["main_inner()"]
    G --> H["Cli::parse()"]
    H --> I["Config::load()"]
    I --> J["rivet_telemetry::init()"]
    J --> K["run_config::config()"]
    K --> L["SubCommand::execute()"]
    L --> M["start::Opts::execute()"]
    M --> N["Pools::new()"]
    N --> O["check_engine_admission()"]
    O --> P["rivet_service_manager::start()"]
```

Every box names a real function, method, or macro that the code calls. The diagram does not show the signal handler, because the handler starts lazily on the first call to `TermSignal::get()`.

## 2. The runtime wrapper

`engine/packages/runtime/src/lib.rs` owns the process runtime. It defines three public functions and one re-export. `run` starts the runtime and runs a future. `shutdown` asks the runtime wrapper to stop. `TermSignal` reports termination signals. The crate also re-exports `reload_log_filter` from the `traces` module.

### 2.1 Runtime construction

```rust
// engine/packages/runtime/src/lib.rs:22-51
static SHUTDOWN: OnceCell<Arc<Notify>> = OnceCell::const_new();

/// Returns `None` if the runtime was shut down manually.
pub fn run<F: Future>(f: F) -> Option<F::Output> {
	// Build runtime
	let mut rt_builder = build_tokio_runtime_builder();
	let rt = rt_builder.build().expect("failed to build tokio runtime");
	let output = rt.block_on(async move {
		let notify = SHUTDOWN
			.get_or_init(|| std::future::ready(Arc::new(Notify::new())))
			.await
			.clone();

		// Must be called from within a tokio context
		let providers = init_otel_providers();
		traces::init_tracing_subscriber(&providers);

		tokio::spawn(sample_runtime_metrics(tokio::runtime::Handle::current()));

		tokio::select! {
			_ = notify.notified() => {
				tracing::info!("shutting down runtime");
				None
			},
			res = f => Some(res),
		}
	});

	output
}
```

The function creates a multi-thread Tokio runtime with `Builder::new_multi_thread` and `enable_all()` (`engine/packages/runtime/src/lib.rs:62-64`). It builds the runtime before it enters the async block. It then runs three setup steps inside the runtime. The OpenTelemetry (OTel) provider guard is created first. The tracing subscriber is created second, because it needs the OTel tracer. The runtime metrics sampler is spawned third. Finally, a `tokio::select!` waits for either the shutdown notification or the command future.

The `SHUTDOWN` cell is a process-global `OnceCell<Arc<Notify>>`. It is initialized on the first call to `run`. `Notify` is a Tokio wake-up primitive. `OnceCell` is a standard-library type that holds one value and initializes it once. The cell makes `shutdown()` reachable from any task in the process. `notify_one()` stores one permit if no task waits. The `select!` therefore cannot miss a notification that arrives after `run` starts but before the `select!` polls its notification branch.

The runtime builder reads several environment variables. Each variable has a default.

| Environment variable | Effect | Default | Source |
|---|---|---|---|
| `TOKIO_RUNTIME_METRICS` | Enables the `TOKIO_TASK_POLL_DURATION` poll-time histogram and its `on_before_task_poll` hook when the variable is present | Disabled | `runtime/src/lib.rs:75-116` |
| `TOKIO_THREAD_STACK_SIZE` | Sets each worker stack size | 8 mebibytes (MiB), because `async-nats` needs a large stack | `runtime/src/lib.rs:118-123` |
| `TOKIO_WORKER_THREADS_SUBTRACT` | Sets worker count to available parallelism minus this value, minimum 1 | Tokio default | `runtime/src/lib.rs:125-133` |
| `TOKIO_MAX_BLOCKING_THREADS` | Sets the blocking pool size | Tokio default | `runtime/src/lib.rs:135-137` |
| `TOKIO_GLOBAL_QUEUE_INTERVAL` | Sets the global queue interval | Tokio default | `runtime/src/lib.rs:139-141` |
| `TOKIO_EVENT_INTERVAL` | Sets the event interval | Tokio default | `runtime/src/lib.rs:143-145` |
| `TOKIO_THREAD_KEEP_ALIVE` | Sets the keep-alive time in milliseconds | Tokio default | `runtime/src/lib.rs:147-149` |

The builder does not set a fixed worker count in the normal case. It uses the Tokio default, which is one worker for each available processor core. The comment on `TOKIO_WORKER_THREADS_SUBTRACT` says the variable subtracts from `available_parallelism`. The code clamps the result to at least 1.

The `TOKIO_RUNTIME_METRICS` check is a presence test, not a value test. The code calls `env::var("TOKIO_RUNTIME_METRICS").is_ok()` (`runtime/src/lib.rs:75`). Any value enables the histogram, including `0` and an empty string. To disable the histogram, unset the variable.

### 2.2 The shutdown notification and its callers

```rust
// engine/packages/runtime/src/lib.rs:53-60
/// Shuts down the entire rivet runtime, if one is running.
pub async fn shutdown() {
	if let Some(shutdown) = SHUTDOWN.get() {
		shutdown.notify_one();
	} else {
		tracing::error!("no runtime to shutdown");
	};
}
```

The function has exactly one caller in the engine packages at this commit: `rivet_runtime::shutdown().await` at `engine/packages/service-manager/src/lib.rs:396`. That line is reached only when the service manager receives the third termination signal. The first signal starts a graceful drain. The third signal aborts the drain.

The notification does not stop tasks by itself. It makes the `select!` in `run` return `None`. When `run` returns, the local `Runtime` value is dropped. Dropping a Tokio runtime shuts it down and drops the tasks that it owns. Dropping a task drops its future at the current await point. The future does not resume, so an asynchronous shutdown method that has not completed does not run its remaining steps. The process then returns from `main`. Section 11.4 applies this rule to the forced path. This is why the call is a last resort, not the normal path.

### 2.3 OpenTelemetry providers

`init_otel_providers()` is defined in `engine/packages/metrics-server/src/providers.rs`. It returns `Option<OtelProviderGuard>`. The guard holds the tracer provider, and its `Drop` implementation shuts that provider down (`metrics-server/src/providers.rs:126-137`). The function returns `None` unless `RIVET_OTEL_ENABLED=1` (`metrics-server/src/providers.rs:112-124`). When enabled, it reads `RIVET_OTEL_GRPC_ENDPOINT`, with the default `http://localhost:4317` (`metrics-server/src/providers.rs:81-84`). The exporter uses the OpenTelemetry Protocol (OTLP) over Google Remote Procedure Call (gRPC) with the tonic transport (`metrics-server/src/providers.rs:86-90`). tonic is the Rust gRPC library.

The sampler is dynamic. A sampler decides whether a trace is recorded and exported. `DynamicSampler` holds a ratio behind `RwLock`, and `set_sampler_ratio` changes it at runtime (`metrics-server/src/providers.rs:12-68`). The ratio controls a `TraceIdRatioBased` sampler (`providers.rs:49-51`). A ratio of 1.0 records every trace, and a ratio of 0.0 records none. A failed lock read falls back to 0.001 (`providers.rs:44`). The sampler is stored in the process-global `SAMPLER` static (`providers.rs:56`). The `dynamic_config` service calls `set_sampler_ratio` directly when it receives a tracing configuration message (`dynamic-config/src/tracing_config.rs:66-75`). The `tracing` subcommand only publishes that message. Section 10.6 covers that path.

### 2.4 The tracing subscriber

`engine/packages/runtime/src/traces.rs` builds the subscriber. It installs these layers in order:

1. A reloadable `EnvFilter` built from `RUST_LOG`.
2. An OpenTelemetry layer, if a provider guard exists, filtered by `RUST_TRACE`.
3. The Sentry tracing layer, constructed by `sentry::integrations::tracing::layer()` (`traces.rs:62`). The `sentry` crate provides this layer. It forwards tracing events and spans to Sentry.
4. A custom `SentryOtelLayer` that replaces the Sentry trace identifier with the OTel trace identifier. On each event, the layer reads the `OtelData` extension of the event's span (`traces.rs:112-131`). It copies that span's OTel trace identifier and span identifier into the Sentry scope `trace` context (`traces.rs:133-144`). If the event has no span, or the extension lacks either identifier, the layer does nothing (`traces.rs:120-133`). Without this layer, Sentry events would carry the Sentry trace identifier instead of the OTel identifier. An operator could then not join a Sentry error to the OTel trace for the same request.
5. The Tokio console layer, if `TOKIO_CONSOLE_ENABLE=1` (`traces.rs:66-85`). Tokio console is a task diagnostics tool. The layer starts an in-process server to which the `tokio-console` command-line tool connects. `console_subscriber::ConsoleLayer::builder().with_default_env().spawn()` builds it. The layer records task and span instrumentation, so it adds work while enabled.
6. A logfmt (key=value) layer, or a Stackdriver layer for Google Cloud Platform (GCP). Stackdriver is the former name of Google Cloud Logging. `tracing_stackdriver::layer()` emits structured JSON with severity and source fields for Cloud Logging (`traces.rs:103-108`). The logfmt layer emits flat key=value text (`traces.rs:86-100`).

The subscriber is a `Registry` that holds these layers (`runtime/src/traces.rs:44-109`). `EnvFilter` is a filter layer. It gates the whole subscriber, so a disabled event reaches no later layer (`runtime/src/traces.rs:59-63`). The OpenTelemetry layer also has its own `RUST_TRACE` filter (`runtime/src/traces.rs:47-55`). The remaining layers are sinks. Every enabled event reaches each sink, and each sink decides what to record (`runtime/src/traces.rs:59-108`).

The log format comes from `RUST_LOG_FORMAT`. The value `gcp` selects the GCP format. Every other value, including an unset variable, selects logfmt (`traces.rs:14-34`). The logfmt layer reads six more variables (`traces.rs:86-100`). `RUST_LOG_SPAN_NAME`, `RUST_LOG_SPAN_PATH`, `RUST_LOG_TARGET`, `RUST_LOG_LOCATION`, and `RUST_LOG_MODULE_PATH` add the matching field when set to `1`. `RUST_LOG_ANSI_COLOR` enables color when set to `1`.

Sentry has two stages. The tracing subscriber installs the Sentry tracing layer and `SentryOtelLayer` before `main_inner` runs (`traces.rs:59-63`). These layers route events to the current thread's Sentry Hub. The process Hub has no client until `rivet_telemetry::init` calls `sentry::init` (`telemetry/src/lib.rs:12-18`). That call happens after configuration load (`main.rs:46`). An event before the call therefore has no client to receive it. The call binds the client to the process Hub. A thread that first uses the Hub after the call copies the client. A worker thread that used the Hub before the call keeps its empty copy (section 1). The guard keeps the client alive until `main_inner` returns (`main.rs:46`). These statements describe the installed path, not guaranteed delivery. Before the bind, the layers run and the Hub has no client, so the event is dropped. After the bind, the same layers deliver to that client.

The subscriber is installed with `.init()` (`traces.rs:109`). A second installation would panic. This is acceptable because `main` calls `run` once per process, and no second call path exists.

`reload_log_filter` uses the stored `ReloadHandle` to replace the filter without a restart (`traces.rs:12`, and the function at the end of the file). The handle is stored in the `RELOAD_HANDLE` static, a `OnceLock<ReloadHandle>` (`traces.rs:12`, `38-42`). The `dynamic_config` service calls `rivet_runtime::reload_log_filter` when a tracing update arrives (`dynamic-config/src/tracing_config.rs:54-63`).

### 2.5 Runtime metrics

`sample_runtime_metrics` runs once every five seconds. The interval constant is `RUNTIME_METRICS_INTERVAL = Duration::from_secs(5)` (`runtime/src/lib.rs:157`). The function reads these values from the Tokio handle:

| Metric | Source call | Meaning |
|---|---|---|
| `TOKIO_GLOBAL_QUEUE_DEPTH` | `global_queue_depth()` | Tasks in the global queue |
| `TOKIO_BLOCKING_QUEUE_DEPTH` | `blocking_queue_depth()` | Jobs in the blocking queue |
| `TOKIO_ACTIVE_TASK_COUNT` | `num_alive_tasks()` | Live tasks |
| `TOKIO_BLOCKING_THREAD_COUNT` | `num_blocking_threads()` | Blocking threads |
| `TOKIO_IDLE_BLOCKING_THREAD_COUNT` | `num_idle_blocking_threads()` | Idle blocking threads |
| `TOKIO_WORKER_OVERFLOW_COUNT` | `worker_overflow_count(worker)` | Local queue overflows for one worker |
| `TOKIO_WORKER_LOCAL_QUEUE_DEPTH` | `worker_local_queue_depth(worker)` | Local queue depth for one worker |
| `TOKIO_WORKER_BUSY_DURATION_TOTAL` | `worker_total_busy_duration(worker)` | Busy time delta for one worker |

The sampler table does not include `TOKIO_TASK_POLL_DURATION`. That metric is a histogram of task poll durations in seconds (`runtime/src/metrics.rs:58-63`; `metrics/src/buckets.rs:17-30`). Only `TOKIO_RUNTIME_METRICS` enables it. When enabled, Tokio keeps a per-worker poll-time histogram with log buckets from 20 microseconds to 32 milliseconds (`runtime/src/lib.rs:105-115`). A log bucket has boundaries that grow logarithmically, so small durations get finer resolution than large ones. The builder sets `precision_exact(0)` and `max_buckets(20)`. The cap of 20 limits the memory and the per-poll work of the hook. Durations outside the range go to the underflow or overflow bucket. The `on_before_task_poll` hook reads the bucket counts, computes the delta for each bucket, and observes that delta into the histogram (`runtime/src/lib.rs:78-103`). The sampler does not read these values, so the hook is the only writer.

The busy duration needs special treatment. Tokio reports a cumulative duration for each worker. A Prometheus (metrics system) counter only advances by a delta. The function keeps `last_busy_nanos` for each worker and adds the difference (`runtime/src/lib.rs:170-204`).

The source comments explain the design. An older version read the queue depths, worker counts, and busy durations from `on_before_task_poll`. That hook runs on the hot path for every poll. The values are only needed at scrape time. The interval sampler costs the same per scrape and nothing per poll. The `TOKIO_TASK_POLL_DURATION` histogram still uses the hook, because it must see every poll (`runtime/src/lib.rs:159-165`).

### 2.6 The termination signal

```rust
// engine/packages/runtime/src/term_signal.rs:12-12
const FORCE_CLOSE_THRESHOLD: usize = 3;
```

`TermSignalHandler` waits for the terminate signal (SIGTERM) or the interrupt signal (SIGINT) on Unix, and for Ctrl+C on Windows (`term_signal.rs:16-28`). On each signal it increments `count`. It sends `count >= FORCE_CLOSE_THRESHOLD` through a `watch` channel (`term_signal.rs:64-74`). The initial value is `false` (`term_signal.rs:38`). The threshold is three.

The first signal sends `false`. The second signal sends `false`. The third signal sends `true`. A `watch` send notifies every receiver even when the new value equals the old value. Each send therefore wakes the manager.

`TermSignal::recv` waits for a version change and returns the current value (`term_signal.rs:113-116`). The value `false` means the graceful drain may continue. The value `true` means abort the graceful attempt and stop immediately (`term_signal.rs:112`).

`TermSignal::get` initializes the handler once. The handler runs as a Tokio task. When `RIVET_TEST_RUNTIME=1`, it runs on a separate current-thread runtime instead (`term_signal.rs:89-107`). This avoids a conflict with the test harness runtime.

`TermSignal::stop` aborts the handler task (`term_signal.rs:118-125`). The service manager calls it after the pools shut down.

## 3. Configuration

`engine/packages/config` is a library, not a service. It builds a typed `Config` value from four inputs: compiled defaults, the default configuration directory, explicit paths, and environment variables. The third input replaces the second. The loader reads the default directory only when no explicit path is given (`config/src/lib.rs:56-64`). It also validates the result before any service starts.

### 3.1 The load algorithm

```rust
// engine/packages/config/src/lib.rs:50-97
pub async fn load<P: AsRef<Path>>(
	paths: &[P],
	build_meta: BuildMeta,
	protocols: RuntimeProtocols,
) -> Result<Self> {
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
}
```

The layering order is important. Earlier sources supply base values. Later sources override them. Items 2 and 3 are alternatives; the loader never reads both.

1. `Root::default()` supplies compiled defaults.
2. If `--config` is absent, the system directory supplies values. If the directory does not exist, the loader skips it silently.
3. If `--config` is present, each path supplies values in order. The system directory is not read in this case.
4. The environment supplies the final overrides.

The environment prefix is `RIVET`. The separator is a double underscore. For example, `RIVET__AUTH__ADMIN_TOKEN` sets `auth.admin_token`. The call does not set a separate prefix separator (`config/src/lib.rs:75-79`). The `config` crate uses the configured separator for that position when `prefix_separator` is absent (`config` 0.14.1 `src/env.rs:239-245`). The crate joins the prefix and the key with the separator when `prefix_separator` is absent. `RIVET` plus `__` therefore gives the lowercase pattern `rivet__` (`src/env.rs:239-250`). The crate lowercases each variable name, skips names that do not start with that pattern, strips the prefix, and replaces every double underscore with a dot (`src/env.rs:258-278`). `RIVET_AUTH__ADMIN_TOKEN` does not match the pattern and is skipped. Single underscores inside a field name remain, so `ADMIN_TOKEN` becomes `admin_token`. The environment source also sets `.list_separator(",")` and `.try_parsing(true)` (`config/src/lib.rs:75-79`). `try_parsing` makes the `config` crate convert a value that looks like a boolean or a number into that type. A `bool` or `u16` field therefore receives a parsed value, not a string. This checkout does not vendor the `config` crate, so the exact coercion for a string field is not visible. The comma list separator applies to an environment value that populates a list. A scalar value that contains a comma is at risk of being split, so a value that must stay literal should come from a file source. The repository comment states the limit: the `config` crate can parse only `Vec<String>` from the environment (`config/src/config/mod.rs:42-47`). The special list key for FoundationDB addresses is registered explicitly. That list key is stale at this commit. The `Root` schema has no `foundationdb` field, and the `Database` enum has no FoundationDB variant (`config/src/config/db/mod.rs:10-13`). The environment loader still registers the key. A process that sets the variable adds a key that `Root` does not declare, and `deny_unknown_fields` rejects the configuration (`config/src/config/mod.rs:70-71`). The registration therefore has no useful effect.

`add_source` accepts a file or a directory (`config/src/lib.rs:358-399`). For a directory, it reads every regular file with one of these extensions: `json`, `json5`, `jsonc`, `yaml`, `yml`. The `yaml` and `yml` entries are two spellings of the same YAML Ain't Markup Language (YAML) format. It ignores other entries. It does not recurse into subdirectories, because it tests `path.is_file()` (`config/src/lib.rs:376-386`). It does not sort the entries, so their order is the order that `std::fs::read_dir` returns. The `config` crate applies the sources in that order. The layer order between inputs is deterministic: compiled defaults, then the directory, then explicit paths, then environment variables. The file order inside one directory is not deterministic. A duplicate scalar key in two files is resolved by that file order, so the winner is not stable across runs (`config/src/lib.rs:376-386`). For a file, it selects the parser from the extension. JavaScript Object Notation (JSON), JavaScript Object Notation version 5 (JSON5), JavaScript Object Notation with comments (JSONC), and YAML are the supported formats. JSON5 and JSONC are parsed with the `json5` crate and converted to JSON before the `config` crate sees them (`config/src/lib.rs:401-428`). A directory scan ignores a file whose extension is not in that list. An explicitly named file with an unsupported extension is an error, because the loader calls `add_file_source` for it directly. A path that exists but is neither a file nor a directory is also an error.

A missing explicit path is not an error. The function logs at debug level and returns the builder unchanged (`config/src/lib.rs:366-371`). This behavior is visible in the code, but it can surprise an operator who mistypes a path.

### 3.2 The default configuration directory

`engine/packages/config/src/paths.rs` gives the directory for each supported operating system.

| Operating system | Directory | Source |
|---|---|---|
| Linux | `/etc/rivet` | `config/src/paths.rs:6-9` |
| macOS | `/Library/Application Support/rivet` | `config/src/paths.rs:11-14` |
| Windows | `C:\ProgramData\rivet` | `config/src/paths.rs:16-19` |

An unsupported target fails at compile time (`config/src/paths.rs:21-25`).

### 3.3 The schema root

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

Every section that carries an enum variant is optional. Three sections are required values with defaults: `telemetry`, `runtime`, and `metrics` (`config/src/config/mod.rs:104-115`). The optional fields are not an accident. A long comment above the struct explains the reason (`config/src/config/mod.rs:42-68`). The third-party `config` crate merges maps by key. It does not replace a struct when the user provides a different variant. The comment uses this example: a default `{ "memory": {} }` and a user value `{ "file_system": {} }` merge into one map (`config/src/config/mod.rs:59-65`). The merged map then holds two variant keys. The loader cannot tell which variant the user intended, so the user value cannot replace the default. The `memory` key in this example is historical; the `Database` enum has no `Memory` variant at this commit. A current example would use the default `{ "postgres": {} }` and the user value `{ "file_system": {} }`.

Serde gives an enum an externally tagged representation by default. The value is a one-entry map. The key is the variant name, and the value is the variant payload. `Database::Postgres` therefore becomes `{"postgres": {...}}` (`config/src/config/db/mod.rs:10-13`). The `#[serde(flatten)]` attribute on `Root.database` removes that one-entry map layer and merges the entry into the root map (`config/src/config/mod.rs:94-95`). This is why the configuration file writes `"postgres": {...}` at the top level. The same rule applies to `Root.pubsub` (`config/src/config/mod.rs:97-98`). The serde documentation says that `deny_unknown_fields` is not supported with `flatten`. The serde derive 1.0.228 still enforces it for this struct. It buffers unknown keys and rejects any entry that no flattened field consumes (`serde_derive` 1.0.228 `src/de/struct_.rs:276-290`, `347-360`). The buffer lives in the root struct's generated visitor. The visitor holds each map key that no direct field consumed. It then replays those keys through every flattened field after the map ends. An unknown key inside a flattened payload fails in that payload's own deserializer (`serde_derive` 1.0.228 `src/de/struct_.rs:276-290`, `347-360`). The two flattened enums each consume their own one-entry map, so one `Database` variant and one `PubSub` variant can appear together. An example is `postgres` from `Database` and `nats` from `PubSub`. At this commit the `Database` enum has `Postgres` and `FileSystem` variants (`config/src/config/db/mod.rs:10-13`).

The `Option` wrapper avoids that merge. `Config::load` starts with `Root::default()` (`config/src/lib.rs:58`). Every optional section in that value is `None` (`config/src/config/mod.rs:70-123`). There is therefore no default variant map for the user map to merge with. The accessor methods apply the defaults after deserialization. They use `LazyLock` for the repeated reads (`config/src/config/mod.rs:67-68`, `156-159`). `LazyLock` is a standard-library type that initializes a value on first access.

The same comment gives a second rule. The rule applies to environment overrides. Only a `Vec<String>` field can receive a comma-separated list from the environment. Any other collection field must be a map type, such as `HashMap`. Most collection fields in `Root` are therefore maps. The reason is that the environment source produces string values. The `config` crate has a list parser only for a list of strings, and it has no list parser for a list of structs or numbers. A map avoids the problem because each entry is set by its own environment key. This checkout does not contain the `config` crate source, so the parser internals are not visible here. The repository comment is the authority for this rule.

`deny_unknown_fields` applies to the root and to most nested structs. An unknown key is an error. This catches spelling errors. It also means that an older binary rejects a configuration written for a newer binary.

Accessors hide the optional fields. `Root::guard()` returns the stored value or a static default (`config/src/config/mod.rs:156-159`). `Root::api_peer()`, `topology()`, `database()`, `pubsub()`, `sqlite()`, `cache()`, `pegboard()`, `features()`, `logs()`, and `outbound()` follow the same pattern. The list is not exhaustive. `insecure_allow_unauthenticated()`, `udb_throttle_bytes_per_second()`, `clickhouse()`, and the datacenter helpers also exist (`config/src/config/mod.rs:150-260`, `346-371`). `clickhouse()` returns an `Option` with no default. `telemetry`, `runtime`, and `metrics` are not optional, so they are read directly.

### 3.4 Validation

`validate_and_set_defaults` runs after deserialization (`config/src/config/mod.rs:260-343`). It enforces these rules:

- `auth` must exist. The error text says to configure `auth.admin_token`, and it names `auth.insecure_allow_unauthenticated` for the legacy behavior. That flag disables authentication.
- The JSON Web Token (JWT) issuer, the `iss` claim that tokens carry, must be derivable and valid. `derive_issuer` uses the leader datacenter's `public_url` and normalizes it (`config/src/config/auth.rs:249-256`). The leader supplies the value because the issuer must be one value for the whole cluster, and the leader defines the canonical public URL. `leader_dc()` returns `None` when the topology has no leader. `derive_issuer` then returns the error "cannot derive JWT issuer without a topology leader" (`auth.rs:249-256`), and configuration load fails. A normalized issuer must be an absolute `http` or `https` Uniform Resource Locator (URL) with a host, and it must have no credentials, query, or fragment (`config/src/config/auth.rs:216-246`). Normalization also removes trailing slashes from the path (`auth.rs:236-246`). `Auth::validate` requires a non-empty `admin_token` and applies the JWT rules (`config/src/config/auth.rs:38-44`, `141-211`).
- If `pubsub` is `Nats` and `database` is `Postgres` without its own `nats`, the Postgres section inherits the NATS section. NATS is the message bus.
- Five sections run their own `validate` method. The five are Guard, Pegboard, Features, Runtime, and Sqlite. `Auth::validate` is a sixth call, invoked directly before those five (`config/src/config/mod.rs:266-282`), so the earlier Auth rule is not part of the five. Guard is the external Hypertext Transfer Protocol (HTTP) and WebSocket listener, and Pegboard is the Actor scheduler. `Guard::validate` rejects a zero rate limit request count, a zero rate limit period, a zero `max_in_flight`, and zero values in its timeout and cache settings (`config/src/config/guard.rs:112-171`). The rate limit period is in milliseconds. The fields with defaults are `proxy_retry_max_attempts` (7 attempts), `proxy_retry_initial_interval_ms` (150 milliseconds), `upstream_request_timeout_ms` (30,000 milliseconds), `websocket_setup_timeout_ms` (30,000 milliseconds), `websocket_connect_attempt_timeout_ms` (5,000 milliseconds), `websocket_send_timeout_ms` (5,000 milliseconds), `websocket_flush_timeout_ms` (2,000 milliseconds), `http_client_pool_idle_timeout_ms` (30,000 milliseconds), `admission_client_state_cache_capacity` (10,000 entries), and `admission_client_state_cache_idle_timeout_ms` (3,600,000 milliseconds) (`guard.rs:9-18`, `128-168`). `Pegboard::validate` bounds the hash load-balancer settings (`config/src/config/pegboard.rs:200-231`). For `EnvoyLoadBalancer::Hash`, `samples` must be 1 through 8, `virtual_nodes` must be 1 through 64, `max_scan` must be 1 through 256, and `slot_jitter` must be 0 through 64. It also rejects a zero `envoy_expire_scheduler_max_concurrent_expires`. `Features::validate` limits `guard_gateway_v3.percentage` to 0 through 100 (`config/src/config/features.rs:18-24`, `37-45`). The feature has three modes: `off`, `opportunistic`, and `on`. The percentage samples requests in `on` mode (`features.rs:47-56`). `Runtime::validate` rejects zero worker poll, wake, dedup, and pull limits (`config/src/config/runtime.rs:97-121`). `Sqlite::validate` rejects zero intervals, storage, and compaction budgets, and it bounds the compaction percentages (`config/src/config/sqlite.rs:293-369`). The fields are `manager_idle_poll_interval_ms`, `manager_reclaim_interval_ms`, `max_storage_bytes`, `compaction_write_bytes_per_second`, `compaction_read_bytes_per_second`, and the stage, install, and reclaim throttle settings (`sqlite.rs:299-350`). It also requires `compaction_max_hot_drain_span_txids` to be at least 128 (`sqlite.rs:318-326`).
- A UniversalDB (UDB) throttle budget of zero is an error. The field is `runtime.udb_throttle_bytes_per_second`, a map from a throttle name to a `UdbThrottle` (`config/src/config/runtime.rs:65-77`). Each `UdbThrottle` has optional `read` and `write` budgets in bytes per second (`runtime.rs:71-94`). The throttle names are operator-chosen; the code has no fixed set. A lookup supplies a name and an axis, `read` or `write` (`runtime.rs:224-231`). A zero budget would stall that axis, so validation rejects it (`config/src/config/mod.rs:284-290`). An absent axis is unthrottled (`runtime.rs:65-67`, `224-231`). This rule differs from the zero `gasoline_prune_eligibility_duration`, which means "never prune" (`runtime.rs:58-60`).
- With more than one datacenter, every datacenter must set `valid_hosts`.
- A map-form datacenter is a JSON object keyed by datacenter name. It must not set `name`, because the key supplies it; validation fails if `name` is present. A list-form datacenter is a JSON array of objects. It must set `name`; validation fails if the name is empty (`config/src/config/mod.rs:305-327`). Section 4 shows the enum and the fields.
- `force_shutdown_duration` must be at least the larger of `worker_shutdown_duration` and `guard_shutdown_duration`.

The last rule links three timers. A force shutdown that fires before a graceful drain would defeat the drain.

`Root::dc_label()` returns `topology().datacenter_label` as a `u16` (`config/src/config/mod.rs:350-353`). `Root::epoxy_replica_id()` casts the same value to a `u64` (`config/src/config/mod.rs:345-348`). The cast produces the numeric replica identifier that Epoxy expects. The code does not derive replica identity in any other way. This alias is the definition of the replica identifier, so the two values cannot differ. The rule is therefore one datacenter, one Epoxy replica. Epoxy is the replicated key-value protocol that document [08 Epoxy](08-epoxy-multi-region.md) explains. In the control plane it replicates three kinds of state. The first is Actor key reservations. A reservation maps a namespace, an Actor name, and an Actor key to one identifier. That identifier's label names the datacenter that owns the key (section 1 defines the three terms; `pegboard/src/workflows/actor2/keys.rs:174-197`, `118-141`). The identifier is an 18-byte value: 16 random bytes followed by the datacenter label as an unsigned 16-bit integer (`util-id/src/lib.rs:41-45`, `60-68`).

The read path is `LookupKeyOptimistic`. It reads `ReservationByKeyKey`, the tuple of namespace, name, and key, through Epoxy (`pegboard/src/workflows/actor2/keys.rs:159-199`). The read uses `get_optimistic` with `CachingBehavior::Optimistic`, so it returns without full consensus (`keys.rs:176-199`; `epoxy/src/ops/kv/get_optimistic.rs:1-30`). The activity returns two shapes. `Found(id)` carries a stored identifier. `NotFound(id, replicas)` carries a new candidate identifier and the runner-config replica identifiers. The code generates the candidate locally with `Id::new_v1(ctx.config().dc_label())` (`keys.rs:193`). The candidate is not written to Epoxy. It is only the proposed value in the check-and-set. A process that loses the race discards it.

The `reserve_key` helper then chooses a branch. This table gives the state and the branch.

| State from `LookupKeyOptimistic` | Local replica in `target_replicas` | Action | Output |
|---|---|---|---|
| `Found(id)` and `id.label()` equals the local datacenter label | not used | `ReserveActorKey` scans `ActorByKeyKey` for a live Actor and writes one if none exists | `Success`, or `KeyExists` |
| `Found(id)` and `id.label()` differs from the local datacenter label | not used | no local write | `ForwardToDatacenter(id.label())` |
| `NotFound` | yes | `Propose` sends a check-and-set that expects no value | `Committed` then `Success` or `KeyExists`; a mismatch handles the stored identifier |
| `NotFound` | no | no local write | `ForwardToDatacenter(first replica label)` |

The sources for the table are `keys.rs:44-58`, `78-110`, `114-142`, `174-197`, and `211-247`.

The `ReserveActorKey` activity is the only branch that touches the local UniversalDB (UDB). It scans `ActorByKeyKey` for an entry whose `is_destroyed` flag is false. Such an entry means another Actor holds the key. The activity then returns `ExistingActor { existing_actor_id }`, which becomes `KeyExists` (`keys.rs:278-316`, `114-127`). With no live entry, the activity writes `ActorByKeyKey` with the current workflow and returns `Success`.

A `Propose` that commits leads to the same activity. A `Propose` that returns `ExpectedValueDoesNotMatch` carries the winning value. The helper deserializes that value and handles the existing reservation (`keys.rs:78-110`).

The forwarding branch does not forward inside the workflow. The workflow sends `Failed` with `KeyReservedInDifferentDatacenter { datacenter_label }` on the `("actor_id", ...)` topic. It then destroys the workflow (`pegboard/src/workflows/actor2/mod.rs:194-208`). The `create` operation subscribed to that topic before dispatch (`pegboard/src/ops/actor/create.rs:84-104`). When `forward_request` is true and `datacenter_name` is absent, the operation calls `forward_to_datacenter` (`create.rs:143-157`). That function resolves the namespace name. It then sends an HTTP `POST /actors` request with a `namespace` query to the target datacenter through `request_remote_datacenter::<CreateResponse>` (`create.rs:318-367`). The original caller receives the response from the target datacenter, so the create succeeds as one request. The api-peer handlers set `forward_request: true` (`api-peer/src/actors/create.rs:48`; `api-peer/src/actors/get_or_create.rs:65`). When `forward_request` is false, or `datacenter_name` is set, the caller receives `KeyReservedInDifferentDatacenter` as an error (`create.rs:143-161`). The second is runner configurations (`pegboard/src/ops/runner_config/upsert.rs:162-181`). The third is the JSON Web Token (JWT) signing key ring (`auth-jwt/src/ops/key_ring/compare_and_set.rs:58-81`). Both use the same pattern as reservations: the writer proposes a value with an expected prior value, and a reader fetches the value through Epoxy. A datacenter's own UniversalDB is not shared with other datacenters (section 4). These three values must therefore cross datacenters through Epoxy.

### 3.5 Runtime defaults and shutdown timers

`engine/packages/config/src/config/runtime.rs` holds the timers that control shutdown and the workflow worker.

| Property | Default | Accessor | Source |
|---|---|---|---|
| `worker_shutdown_duration` | 30 seconds | `worker_shutdown_duration()` | `config/src/config/runtime.rs:29-31`, `132-134` |
| `guard_shutdown_duration` | 10 minutes | `guard_shutdown_duration()` | `config/src/config/runtime.rs:48-50`, `192-194` |
| `force_shutdown_duration` | 10 minutes | `force_shutdown_duration()` | `config/src/config/runtime.rs:51-54`, `196-199` |
| `worker_poll_interval_ms` | 16000 milliseconds | `worker_poll_interval()` | `config/src/config/runtime.rs:32-34`, `136-138` |
| `gasoline_prune_eligibility_duration` | 7 days | `gasoline_prune_eligibility_duration()` | `config/src/config/runtime.rs:58-60`, `205-215` |
| `gasoline_prune_interval_duration` | 12 hours | `gasoline_prune_interval_duration()` | `config/src/config/runtime.rs:61-62`, `217-222` |
| `worker_load_shedding_beta` | 0.95 | `worker_load_shedding_beta()` | `config/src/config/runtime.rs:27-28`, `128-130` |
| `worker_max_workflows_per_pull` | 1000 | `worker_max_workflows_per_pull()` | `config/src/config/runtime.rs:43-44`, `162-164` |
| `worker_max_deduped_workflows_per_pull` | 10000 | `worker_max_deduped_workflows_per_pull()` | `config/src/config/runtime.rs:39-40`, `158-160` |

A zero `gasoline_prune_eligibility_duration` means "never prune". By contrast, validation rejects a zero UniversalDB (UDB) throttle budget (`config/src/config/mod.rs:286-290`).

The remaining entries have these meanings. `gasoline_prune_eligibility_duration` is the age at which a completed workflow becomes eligible for history pruning. `gasoline_prune_interval_duration` is how often the pruner looks for eligible workflows. `worker_load_shedding_beta` is the beta term of the exponential moving average that estimates worker load; it is not a beta feature (`config/src/config/runtime.rs:27-28`; `gasoline/src/db/kv/mod.rs:1195-1198`). The sample is the CPU usage of the current container or process, measured over a 150-millisecond interval (`gasoline/src/db/kv/system.rs:8`, `57-128`). The update is `smoothed = beta * smoothed + (1 - beta) * sample` (`system.rs:121`). The default 0.95 gives most weight to the previous value, so the estimate reacts slowly to a sudden load change.

### 3.6 Ports and listeners

A listener exists when its service runs, and an omitted host or port falls back to a compiled default. Guard, the external listener, serves the Hypertext Transfer Protocol (HTTP) and, when configured, the Hypertext Transfer Protocol Secure (HTTPS). The peer listener serves the internal Application Programming Interface (API). Guard and API peer are in the default service list (`engine/src/run_config.rs:6-21`). A service selection can remove them (section 5.4). Metrics is always appended inside the manager start (`service-manager/src/lib.rs:147-152`). The `guard` and `api_peer` configuration sections are optional. `Root::guard()` and `Root::api_peer()` return a compiled default when a section is absent (`config/src/config/mod.rs:156-164`), so the listener still starts. The only optional listener is HTTPS, which exists only when `guard.https` is present. The compiled defaults are:

```rust
// engine/packages/config/src/defaults.rs:9-14
pub mod ports {
	pub const GUARD: u16 = 6420;
	pub const API_PEER: u16 = 6421;

	pub const METRICS: u16 = 6430;
}
```

| Listener | Enabled when | Default host | Default port | Configuration | Source |
|---|---|---|---|---|---|
| Guard HTTP | the `guard` service runs; it is in the default service list | `[::]` | 6420 | `guard.host`, `guard.port` | `config/src/defaults.rs:4-6`, `config/src/config/guard.rs:173-179` |
| Guard HTTPS | `guard.https` is present | `0.0.0.0` | No default; `guard.https.port` is a required `u16` when the table exists | `guard.https.port` | `config/src/config/guard.rs:356-359`, `guard-core/src/server.rs:43-52` |
| API peer | the `api_peer` service runs; it is in the default service list | `[::]` | 6421 | `api_peer.host`, `api_peer.port` | `config/src/defaults.rs:5`, `11`, `config/src/config/api_peer.rs:13-20` |
| Metrics | always; the manager appends the service | `[::]` | 6430 | `metrics.host`, `metrics.port` | `config/src/defaults.rs:6`, `13`, `config/src/config/metrics.rs:14-21` |

The HTTPS listener binds `0.0.0.0`, not the configured Guard host. It uses only `https.port` (`guard-core/src/server.rs:43-52`). The certificate and key paths are required fields of `Tls` (`config/src/config/guard.rs:361-369`). HTTPS is enabled only when `guard.https` is present. A certificate resolver is the object that loads the certificate chain and private key and supplies them to the TLS listener. Guard logs `No TLS configuration found, HTTPS will not be enabled` when no Transport Layer Security (TLS) certificate resolver exists (`guard/src/lib.rs:33-37`). It logs `TLS certificate resolver configured` otherwise.

Two more defaults are useful for operators. The NATS address default is `127.0.0.1:4222` (`config/src/config/pubsub.rs:47-64`). The OTLP endpoint default is `http://localhost:4317` (`metrics-server/src/providers.rs:81-84`).

The PostgreSQL default URL is `postgresql://postgres:postgres@127.0.0.1:5432/postgres` (`config/src/config/db/postgres.rs:63-70`). The default database variant is `FileSystem`, and its default path is the platform data directory plus `rivet-engine/db`, or `./data/db` when no data directory exists. The platform data directory comes from `dirs::data_local_dir()` (`config/src/config/db/mod.rs:32-36`).

### 3.7 A real configuration file

```jsonc
// self-host/compose/dev/rivet-engine/config.jsonc:1-36
{
	"auth": {
		"admin_token": "dev"
	},
	"api_peer": {
		"host": "0.0.0.0"
	},
	"topology": {
		"datacenter_label": 1,
		"datacenters": {
			"default": {
				"datacenter_label": 1,
				"is_leader": true,
				"peer_url": "http://rivet-engine:6421",
				"public_url": "http://rivet-engine:6420",
				"valid_hosts": [
					"rivet-engine",
					"127.0.0.1",
					"localhost"
				]
			}
		}
	},
	"outbound": {
		"allow_private_networks": true
	},
	"postgres": {
		"url": "postgresql://postgres:postgres@postgres:5432/rivet_engine"
	},
	"clickhouse": {
		"http_url": "http://clickhouse:9300",
		"native_url": "http://clickhouse:9301",
		"username": "system",
		"password": "default"
	}
}
```

This file exercises several layers. `auth.admin_token` is set, so validation passes. `api_peer.host` is `0.0.0.0`; the port is absent, so the default 6421 applies. The topology has one datacenter named `default`. Its label is 1 and it is the leader. The public URL uses Guard port 6420. The peer URL uses API peer port 6421. `valid_hosts` lists three names, which the regional host check uses. That check compares the request hostname with `valid_hosts` during routing and returns the `MustUseRegionalHost` error on a mismatch (`guard/src/routing/mod.rs:372-402`; `guard/src/errors.rs:70-84`). The `database` field uses `#[serde(flatten)]`, so the `Database` enum contributes its one-entry table to the root map (`config/src/config/mod.rs:94-95`). The `postgres` key selects the `Postgres` variant of the externally tagged enum (`config/src/config/db/mod.rs:10-15`). The `clickhouse` table is a normal optional section. ClickHouse is the analytics database.

The file does not set `pubsub`. The default is `Memory`. The `postgres` section does not set `nats`, so UniversalDB runs in single-node mode. In single-node mode the driver keeps an in-process commit transport and holds a leader lease on the PostgreSQL database (`universaldb/src/driver/postgres/database.rs:165-188`). The lease is a row with an expiry that only one process may hold; section 12 explains it. The process does not use the multi-node NATS transport. A second process cannot also hold that lease, so it cannot share the same database concurrently in this mode (section 12).

### 3.8 The protocol version map

```rust
// engine/packages/config/src/runtime_protocols.rs:3-9
#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, strum::FromRepr)]
pub enum RuntimeProtocolKind {
	Envoy = 0,
	Ups = 1,
	UniversaldbCommit = 2,
	Epoxy = 3,
}
```

`RuntimeProtocols` holds one `RuntimeProtocol` for each kind: `envoy`, `ups`, `universaldb_commit`, and `epoxy` (`runtime_protocols.rs:56-62`). Each `RuntimeProtocol` has a compiled version and an optional override (`runtime_protocols.rs:22-54`). `version()` returns the override when present, and the compiled version otherwise.

The `envoy` kind is the wire protocol for the Actor-to-engine tunnel. An Actor runtime connects to Guard at `/envoys/connect`, and Guard serves the connection with `PegboardEnvoyWs` (`guard/src/routing/envoy.rs:28-61`). The tunnel parses its path with `protocols().envoy.version()` (`pegboard-envoy/src/lib.rs:86-89`). The `ups` kind versions UniversalPubSub messages. The publisher reads `config.protocols().ups.version()` once for a message and encodes every chunk at that version (`universalpubsub/src/pubsub.rs:193-201`; `universalpubsub/src/chunking.rs:309-345`). The subscriber decodes with `UpsMessage::deserialize_with_embedded_version` (`chunking.rs:59`). The `universaldb_commit` kind versions the commit messages that a follower sends to a leader. The follower reads `shared.commit_protocol_version()` and encodes the request with it (`universaldb/src/driver/postgres/commit.rs:112-119`; `universaldb/src/driver/postgres/codec.rs:36-53`). The leader reads the embedded version with `decode_commit_request` (`codec.rs:57-58`). A commit request that exceeds the NATS payload limit also requires `CHUNKED_COMMIT_PROTOCOL_VERSION` (`commit.rs:136-141`). The `epoxy` kind versions cross-datacenter Epoxy requests (section 3.4). Section 9.3 defines the routing targets.

The tunnel path is a query string. It must carry five parameters (`pegboard-envoy/src/utils.rs:23-78`).

- `protocol_version` is the envoy wire protocol version chosen by the Actor runtime. The engine accepts it only when it is at least 2 and at most the fleet version.
- `namespace` is the namespace name. The engine resolves it to a namespace identifier through `resolve_for_name_global` (`pegboard-envoy/src/lib.rs:92-100`).
- `pool_name` is the runner configuration name for the Actor. The engine records it on the connection and uses it for runner lookup (`lib.rs:103-110`). It is not the `Pools` object of section 7. It is limited to 128 characters (`utils.rs:49-51`).
- `envoy_key` is the connection key. It is 3 to 128 identifier characters (`utils.rs:57-70`). It names the receiver and eviction subjects (`lib.rs:113-122`).
- `version` is the envoy's own `u32` counter. It is not a protocol version. The engine stores it in `VersionKey` and logs a warning when a reconnect changes it (`pegboard-envoy/src/conn.rs:380-399`).

The upper bound is `ctx.config().protocols().envoy.version()` (`pegboard-envoy/src/lib.rs:89`). That value is the override when present, and the compiled version otherwise. The `version_management` refresh sets the override to the lowest live version (section 3.8). A value outside the range returns `WsError::InvalidRequest` with the message "unsupported `protocol_version` query parameter" (`utils.rs:31-42`).

The `Default` implementation sets every compiled version to 0 (`runtime_protocols.rs:64-78`). The comment states that 0 is not a real schema version. A process that reaches the wire with 0 fails instead of negotiating the wrong format. For the `envoy` kind, the engine rejects a value below 2 at the handshake (`pegboard-envoy/src/utils.rs:11`, `31-42`). For the `ups` and `universaldb_commit` kinds, the receiver decodes the embedded version, and version 0 has no schema (`universalpubsub/src/chunking.rs:59`; `universaldb/src/driver/postgres/codec.rs:57-58`). The failure is therefore on the process that sends the placeholder. Only the binary that depends on the protocol crates can supply real versions. The version constants come from those crates' build metadata, and `rivet_build_meta::compiled_runtime_protocols()` merges them. `main.rs` supplies them to `Config::load` (`engine/src/main.rs:37-41`).

The versions are not static for the life of the process. `Config::set_protocols` replaces the map (`config/src/lib.rs:139-141`). `version_management` refreshes it every 15 seconds (`version-management/src/lib.rs:10-11`, `14-51`).

Each live process writes one entry for each protocol. `ProtocolVersionKey::pack` builds a tuple with five elements (`version-management/src/keys.rs:83-98`). The elements are `RIVET`, `VERSION`, `PROTOCOL`, the protocol kind as a `usize`, and the compiled version as a `u16`. The subspace key keeps the first four elements (`version-management/src/keys.rs:131-140`). The full key adds the `u16` version as a fifth element (`keys.rs:83-98`). Tuple packing orders each element numerically. The version is therefore the last sort element inside the subspace, so the key sort order is the version order. Two processes with the same compiled version write the same key. The value is the last update timestamp in milliseconds, stored as a little-endian `i64` (`version-management/src/keys.rs:70-80`).

The refresh reads one protocol subspace with a range scan in increasing key order (`version-management/src/lib.rs:113-122`). The lowest version therefore comes first. The scanning transaction deletes each entry whose timestamp is more than 30 seconds old and then continues (`version-management/src/lib.rs:10`, `127-139`). A dead process stops refreshing its entry, so the next scan deletes it. The first live entry is the lowest live version. The refresh sets that version as the override and stops reading (`version-management/src/lib.rs:158-159`). A higher version can be live, but the lowest live version is the one that every process can decode.

If the lowest live version is lower than this process's compiled version, the override is lower. `version()` then returns the lower value, so this binary encodes at the older version. The binary keeps its newer schema unused until every older process stops. A newer format therefore cannot reach the wire while an older peer is live.

If the lowest live version is higher than this process's compiled version, the refresh returns a rollback error (`version-management/src/lib.rs:141-156`). A rollback error means this binary is older than a live peer. It cannot decode the peer's format. The function returns before it writes an entry for that protocol, and before it processes the remaining protocols. One call processes all four protocols in a fixed order, and the first rollback stops that tick (`version-management/src/lib.rs:106-170`). In `rivet_version_management::start`, the `?` operator ends the service with that error (`version-management/src/lib.rs:23-28`). The service is `Core`, so the manager logs the crash and restarts it after one second; the error repeats until the operator runs a newer binary.

On the normal path, the refresh writes the process's own entry after the scan. The key uses the compiled version, and the value is the current timestamp. `MutationType::Max` is an atomic UniversalDB mutation. `apply_max` converts both values to a little-endian `i64` and keeps the numerically larger one (`universaldb/src/atomic.rs:75-87`; `universaldb/src/options.rs:54-57`). The timestamp therefore never moves backward, even if the clock does (`version-management/src/lib.rs:162-167`).

The comment says the write must follow the read (`version-management/src/lib.rs:162`). A UniversalDB transaction can read its own writes. If the write ran first, the scan would meet this process's own entry. That matters when this process's compiled version is lower than a live peer's version. The own entry would then be the first live entry, so the rollback check would compare the compiled version with itself. The process would not detect the newer live peer. When the compiled version is higher than every live peer, the own entry sorts last and the scan still finds a peer first.

The scan, the deletes, the rollback check, and the write run in one transaction (`version-management/src/lib.rs:23-28`, `65-99`). A rollback error aborts the transaction, so the expired-entry deletes from that tick do not commit either. A concurrent process sees the old set or the new set, not a partial set. Concurrent writes to one key use `MutationType::Max`, so they do not conflict (`universaldb/src/atomic.rs:75-87`).

The Epoxy protocol is different. An Epoxy peer lives in another datacenter. It writes to its own database, so the local protocol-version subspace from this section does not contain the peer's version (`epoxy/src/protocol_version.rs:1-10`). The `epoxy_protocol_version` service probes each replica in the cluster topology over HTTP every 60 seconds with a 5 second timeout (`epoxy/src/protocol_version.rs:21-28`, `62-86`). It stores each answer in the process-global `PEER_VERSIONS` map, keyed by replica URL (`epoxy/src/protocol_version.rs:35-40`). A cross-datacenter request calls `negotiate` before it encodes the request (`epoxy/src/http_client.rs:169`). `negotiate` reads the local Epoxy version from `config.protocols().epoxy.version()` (`epoxy/src/protocol_version.rs:50-51`). That value is the override when present and the compiled version otherwise (section 3.8). It then returns the lower of the local value and the stored peer value (`epoxy/src/protocol_version.rs:53-59`). A peer with no stored answer is treated as the local version. That rule covers startup. The probe interval fires its first tick immediately, but the service skips a pass when the cluster configuration is not yet readable (`epoxy/src/protocol_version.rs:72-96`). A request that arrives before a peer's first answer uses the local version (`epoxy/src/protocol_version.rs:49-61`). If that version is higher than the peer's compiled version, the peer cannot decode the request and returns an error. A skipped pass can extend the window to the next 60-second tick.

## 4. One control plane instance and the whole cluster

The source uses three scopes that are easy to mix.

| Scope | Definition in the source | Identity | Shared state |
|---|---|---|---|
| One process | One run of the `rivet-engine` binary | A random `NodeId`, a fresh Universally Unique Identifier (UUID) version 4 | Its own runtime and its own in-process caches |
| One datacenter | All processes with the same `topology.datacenter_label` | The label, which is also the Epoxy replica identifier | The configured UDB and UniversalPubSub (UPS) endpoints for that datacenter. UDB is shared when `database` is `Postgres` and `postgres.nats` is set. UPS is shared when `pubsub` is `Nats`. The two choices are independent |
| The cluster | All datacenters in `topology.datacenters` | One leader datacenter plus zero or more follower datacenters | Cross-datacenter HTTP through `peer_url` and the Epoxy protocol |

Every service in section 6 runs in every process, including a follower datacenter. The service list is unconditional (`engine/src/run_config.rs:5-67`). A follower therefore runs `dynamic_config`, `version_management`, and `epoxy_protocol_version` as well. The process scope covers the runtime, the caches, and the `NodeId`. The datacenter scope covers the shared UniversalDB and UniversalPubSub endpoints. The cluster scope covers Epoxy replication and cross-datacenter `peer_url` calls. The leader check appears only inside specific workflows and bootstrap tasks, not in the service list. For example, the Epoxy coordinator and the default namespace run only on the leader (`bootstrap/src/lib.rs:53-58`, `90-94`).

The topology type makes the relation explicit.

```rust
// engine/packages/config/src/config/topology.rs:7-14
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Topology {
	/// Must be included in `datacenters`
	pub datacenter_label: u16,
	/// Map of all datacenters, including this datacenter.
	pub datacenters: DatacentersRepr,
}
```

`Topology::current_dc()` finds this process's datacenter by label. `Topology::leader_dc()` finds the datacenter with `is_leader` set. `Topology::is_leader()` returns true only when the current datacenter is the leader (`config/src/config/topology.rs:27-44`). The doc comment says the label must appear in `datacenters`, but no validation checks it. `current_dc()` returns the error "topology must have a own datacenter" when the label matches nothing (`topology.rs:34-37`). `is_leader()` treats that error as false (`topology.rs:39-44`).

`DatacentersRepr` is an untagged enum, so the input shape selects the variant (`config/src/config/topology.rs:78-84`). The map form uses the key as the datacenter name. The list form is deprecated and must set `name` on each entry.

The `Datacenter` type holds the addresses.

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

`public_url` is the external origin. `proxy_url` is the private origin for other datacenters; when absent, `proxy_url()` returns `public_url` (`topology.rs:192-194`). `peer_url` is the API peer address. Cross-datacenter server code joins `peer_url` with an endpoint (`api-util/src/lib.rs:74-84`).

The topology is static configuration. No service in `run_config.rs` performs membership or directory registration. Every instance must receive the same topology. The source gives no other discovery mechanism in these packages.

In multi-node mode, how does a second instance find the first? It does not find the process. It opens the same durable store. Then it uses conditional workflow dispatch to avoid doing first-run work twice. The bootstrap service shows this pattern in Gasoline, the durable workflow engine. `.unique()` is a dispatch flag: the dispatcher looks for an existing workflow with the same name and tags, and returns that workflow instead of creating a second one. Five bootstrap functions in the concurrent group call `.unique()` (`bootstrap/src/lib.rs:60-65`, `79-84`, `126-130`, `138-142`, `150-154`). The `try_join!` has six branches: replica then coordinator, default namespace, backfills, metrics aggregator, gas pruner, and datacenter ping (`bootstrap/src/lib.rs:17-28`). Two of these five add a tag that names the replica: `setup_epoxy_coordinator` and `setup_epoxy_replica`. The other three (metrics aggregator, gas pruner, and datacenter ping) add no tag. A sixth function, `setup_auth_jwt_key_rotation`, calls `.unique()` after the join with the tag `("auth", "jwt")` (`bootstrap/src/lib.rs:43-48`). The backfill module adds eight more `.unique()` dispatches (`bootstrap/src/backfill.rs:13`, `26`, `35`, `48`, `61`, `74`, `85`, `98`). The Epoxy backfill is the only tagged one, with `("replica", replica_id)` (`bootstrap/src/backfill.rs:34-36`). The other seven are untagged. The default-namespace function adds the tag `("namespace_id", namespace_id)` but does not call `.unique()` (`bootstrap/src/lib.rs:107-115`).

A tag is one key and value pair. The builder stores tags in a `serde_json::Map` (`gasoline/src/builder/common/workflow.rs:22`, `79-92`). That map is a sorted tree at this commit, so its entries iterate in sorted key order. `serde_json` is built without the `preserve_order` feature, so `Map` is a BTreeMap. Gasoline adds no ordering guarantee of its own. `.unique()` sets a flag on the builder (`gasoline/src/builder/common/workflow.rs:96-104`). At dispatch, the code reads the first tag entry from the sorted map (`gasoline/src/db/kv/mod.rs:407-413`). It uses that tag to select a range. For each tag, the code also writes an index entry under `ByNameAndTagKey`, the workflow-tag secondary index in the `WORKFLOW` keyspace. That entry stores the set of the remaining tags (`gasoline/src/db/kv/mod.rs:318-359`). The key tuple is `(WORKFLOW, BY_NAME_AND_TAG, workflow_name, k, v, workflow_id)` (`gasoline/src/db/kv/keys/workflow.rs:950-966`). The stored value is the JSON list of the remaining tags. A remaining tag is every tag except the range-selecting `k` (`gasoline/src/db/kv/mod.rs:343-358`; `gasoline/src/db/kv/keys/workflow.rs:937-947`). The lookup scans the range and returns the first entry whose stored set contains every requested rest tag (`gasoline/src/db/kv/mod.rs:432-463`). Extra stored tags are allowed, so the test is a subset test, not an equality test. A tagged call with one tag has an empty remaining set, so every stored entry in that `k` and `v` range matches. The subset rule has one consequence for identity. A `.unique()` call with one tag can match a workflow that was dispatched with more tags. The match is by name and the range tag, not by the full tag set.

Every workflow also writes a null-tag index entry. Its key tuple is `(WORKFLOW, BY_NAME_AND_TAG, workflow_name, "", "", workflow_id)` (`gasoline/src/db/kv/keys/workflow.rs:923-930`, `950-966`). The two empty strings place it before every real tag entry in sort order. Its value is the full tag list, not the remaining tags (`gasoline/src/db/kv/mod.rs:361-365`; `keys/workflow.rs:937-947`). An untagged `.unique()` has an empty rest-tag set. It scans the null-tag subspace (`gasoline/src/db/kv/mod.rs:423-430`). The empty set is a subset of every entry, so the first entry with the requested name matches (`gasoline/src/db/kv/mod.rs:454-462`). An untagged `.unique()` is therefore idempotent by name alone. The builder comment says the flag has no effect without tags (`gasoline/src/builder/common/workflow.rs:94-95`). The dispatch code performs the null-subspace lookup, so the code is the stronger statement.

The default namespace does not use `.unique()` because its tag value is a fresh random identifier on each run (`bootstrap/src/lib.rs:106`). A lookup on that tag would never match an earlier run. The namespace workflow enforces uniqueness by name instead. It reads the `ByNameKey` index inside a serializable transaction. The isolation argument is explicit at that read; Gasoline activities do not default to `Serializable` (`namespace/src/workflows/namespace.rs:159-163`). If the name exists, it returns `NameNotUnique` (`namespace/src/workflows/namespace.rs:145-170`).

The whole dispatch runs one UDB transaction (`gasoline/src/db/kv/mod.rs:931-962`). The lookup uses `Serializable` isolation (`gasoline/src/db/kv/mod.rs:400-438`). `Serializable` is the stronger of the two UniversalDB isolation levels; the other is `Snapshot` (`universaldb/src/utils/mod.rs:17-21`). A serializable transaction behaves as if it ran alone in some order with the other committed transactions. UniversalDB aborts the losing transaction when another transaction changes a range that it read before commit. The closure then runs again on a fresh transaction, after a backoff, up to the configured retry limit (`universaldb/src/driver/postgres/database.rs:336-396`). A match returns the existing workflow identifier (`gasoline/src/db/kv/mod.rs:294-303`). A new workflow writes a new entry into that same range (`gasoline/src/db/kv/mod.rs:336-358`). The retry of the losing transaction then finds the winner and returns its identifier. The guarantee covers processes that share one UDB database, and it does not cross datacenters. Document [05 Gasoline](05-gasoline-durable-execution.md) covers the engine.

There is a strict condition for two processes to share a datacenter. The default database is `FileSystem`, which is a local RocksDB (embedded key-value store) directory. RocksDB holds an exclusive lock on that directory, and a second open fails with "failed to open rocksdb" (`universaldb/src/driver/rocksdb/database.rs:51`). Two processes therefore cannot share it. The `Database` enum has only two variants.

```rust
// engine/packages/config/src/config/db/mod.rs:10-21
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
```

The UniversalDB pool turns those variants into drivers. A `Postgres` database uses `PostgresDatabaseDriver`; a `FileSystem` database uses `RocksDbDatabaseDriver` (`pools/src/db/udb.rs:19-61`). The driver directory at this commit holds `postgres` and `rocksdb` (`engine/packages/universaldb/src/driver`). No bucket driver exists there.

A shared datacenter also needs NATS. The Postgres configuration comment states the rule. When a NATS section is set, UniversalDB runs in multi-node mode and uses NATS for follower-to-leader commit transport. When it is absent, UniversalDB runs single-node (`config/src/config/db/postgres.rs:53-60`). The pool setup repeats this rule and builds the NATS configuration only when the option is present (`pools/src/db/udb.rs:21-34`). The validation step copies the UniversalPubSub NATS section into the Postgres section when the Postgres section omits it (`config/src/config/mod.rs:268-276`). A single `pubsub: nats` table can therefore select both shared pubsub and multi-node UniversalDB. The validation step clones the whole `Nats` value into `Postgres.nats` only when three conditions hold: `pubsub` is `Nats`, `database` is `Postgres`, and `pg.nats` is `None` (`config/src/config/mod.rs:268-276`). The clone is a full value copy of the parsed section. It copies no other field, and it does not run for `FileSystem`. The flattened enums do not change this rule, because validation runs after deserialization.

**Aha:** The exploration README says that the only external durable store is an object storage bucket, and that control plane instances find each other through that bucket. The configuration type and the driver directory in this commit do not support that claim for the control plane. The two database variants are PostgreSQL and a local file system, and the two pubsub variants are NATS and process memory. The `PubSub` enum has `Nats(Nats)` and `Memory(Memory)` variants (`config/src/config/pubsub.rs:8-15`). Object storage is outside the control plane. Depot is the per-database SQLite-on-UniversalDB storage engine (`depot/README.md:3-5`). Hot SQLite pages are UniversalDB values, not separate files on disk. Cold pages are shard objects in object storage; compaction uploads them with `CMP_S3_UPLOAD_MAX_OBJECTS` and `CMP_S3_UPLOAD_LIMIT_BYTES` limits (`depot/src/conveyer/constants.rs:190-200`). Section 6 describes that cold path. Gasoline is imported as `gas`, so `from_gas_id` means "from a Gasoline identifier". It maps each Actor to one database identifier and each namespace to one bucket identifier (`pegboard/src/actor_sqlite.rs:317-319`). That bucket identifier is a Depot-local namespace key, built by `BucketId::from_gas_id` from the namespace identifier. It is not an object-storage bucket. Documents [06 Depot](06-actor-sqlite-and-depot.md) and [07 UniversalDB](07-universaldb-and-buckets.md) describe it. This document follows the code. The likely cost of the code design is that self-hosting without PostgreSQL and NATS gives a single-process deployment, not a cluster. The alternative, a bucket-backed membership and storage layer, would remove those two services but would put object storage on the coordination path.

## 5. The service manager

`engine/packages/service-manager/src/lib.rs` turns a list of service descriptions into running Tokio tasks. It also owns restart policy, the implicit metrics service, the cron scheduler, and the first stage of shutdown.

### 5.1 The service type

```rust
// engine/packages/service-manager/src/lib.rs:15-48
#[derive(Clone)]
pub struct Service {
	pub name: &'static str,
	pub kind: ServiceKind,
	pub run: Arc<
		dyn Fn(
				rivet_config::Config,
				rivet_pools::Pools,
			) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>
			+ Send
			+ Sync,
	>,
	pub requires_graceful_shutdown: bool,
}

impl Service {
	pub fn new<F, Fut>(
		name: &'static str,
		kind: ServiceKind,
		run: F,
		requires_graceful_shutdown: bool,
	) -> Self
	where
		F: Fn(rivet_config::Config, rivet_pools::Pools) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = Result<()>> + Send + 'static,
	{
		Self {
			name,
			kind,
			run: Arc::new(move |config, pools| Box::pin(run(config, pools))),
			requires_graceful_shutdown,
		}
	}
}
```

The fields have these purposes:

| Field | Type | Purpose |
|---|---|---|
| `name` | `&'static str` | The selection name for `--services` and `--except-services`, and the task name |
| `kind` | `ServiceKind` | Selects the lifecycle behavior and documents the role |
| `run` | `Arc<dyn Fn(Config, Pools) -> Future>` | The service entry point; every service has the same signature |
| `requires_graceful_shutdown` | `bool` | Decides whether the task is aborted or retained on SIGTERM |

`Service::new` boxes the future and stores the closure in an `Arc`. The clone is cheap. The `start` command clones selected services into the manager.

### 5.2 Kinds and behaviors

```rust
// engine/packages/service-manager/src/lib.rs:50-109
/// Defines the type of the service. Used for filtering service types to run.
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceKind {
	ApiPublic,
	ApiPeer,
	Standalone,
	Singleton,
	Oneshot,
	Cron(CronConfig),
	/// Run no matter what.
	Core,
}

impl ServiceKind {
	fn behavior(&self) -> ServiceBehavior {
		use ServiceKind::*;

		match self {
			ApiPublic | ApiPeer | Standalone | Singleton | Core => ServiceBehavior::Service,
			Oneshot => ServiceBehavior::Oneshot,
			Cron(config) => ServiceBehavior::Cron(config.clone()),
		}
	}

	pub fn eq(&self, other: &Self) -> bool {
		use ServiceKind::*;

		match (self, other) {
			(ApiPublic, ApiPublic)
			| (ApiPeer, ApiPeer)
			| (Standalone, Standalone)
			| (Singleton, Singleton)
			| (Oneshot, Oneshot)
			| (Core, Core) => true,
			(Cron(_), Cron(_)) => true,
			_ => false,
		}
	}
}

/// Defines how a service should be ran.
#[derive(Debug, Clone, PartialEq)]
enum ServiceBehavior {
	/// Spawns a service that will run indefinitely.
	///
	/// If crashes or exits, will be restarted.
	Service,
	/// Runs a task that will exit upon completion.
	///
	/// If crashes, it will be retried indefinitely.
	Oneshot,
	/// Runs a task on a schedule.
	Cron(CronConfig),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CronConfig {
	pub run_immediately: bool,
	pub schedule: String,
}
```

Only two kinds change behavior:

- `Oneshot` maps to `ServiceBehavior::Oneshot`.
- `Cron` maps to `ServiceBehavior::Cron`.

The other five kinds map to `ServiceBehavior::Service`. This includes `Core` and `Singleton`. `Singleton` is declared but no service in `run_config.rs` uses it at this commit. The `Core` doc comment says "Run no matter what", but `behavior()` does not enforce that. The doc comment is stale at this commit. No code path filters by kind. `start.rs` filters by name (`start.rs:33-69`). The manager appends the metrics service after the filter (`service-manager/src/lib.rs:147-152`). Only the implicit metrics service bypasses the name filter.

`ServiceKind::eq` compares variants and ignores the `CronConfig` contents. The manager does not use this method in the start path. It is a public helper.

`ServiceBehavior` is private. This is intentional. A caller describes a service with a public kind. The lifecycle mapping is an internal detail.

### 5.3 Restart policy

The manager spawns one task for each service with `ServiceBehavior::Service`.

```rust
// engine/packages/service-manager/src/lib.rs:170-199
					let join_handle = tokio::task::Builder::new()
						.name(&task_name)
						.spawn(async move {
							tracing::debug!(service=%service.name, "starting service");

							loop {
								match (service.run)(config.clone(), pools.clone()).await {
									Result::Ok(_) => {
										if shutting_down.load(Ordering::SeqCst) {
											tracing::info!(service=%service.name, "service exited");
											break;
										} else {
											tracing::error!(service=%service.name, "service exited unexpectedly");
										}
									}
									Err(err) => {
										tracing::error!(service=%service.name, ?err, "service crashed");

										if shutting_down.load(Ordering::SeqCst) {
											break;
										}
									}
								}

								tokio::time::sleep(Duration::from_secs(1)).await;

								tracing::info!(service=%service.name, "restarting service");
							}
						})
						.context("failed to spawn service")?;
```

A service that returns `Ok(())` before shutdown is logged as "service exited unexpectedly". It is then restarted. A service that returns an error is logged as "service crashed" and restarted. The delay is one second. The loop never gives up. This policy is the same for `Core`, `Standalone`, `Singleton`, `ApiPublic`, and `ApiPeer`. A panic is not an `Err`; it unwinds the task and ends this loop (section 13).

The `shutting_down` flag is an `AtomicBool` shared by all tasks and the manager. During shutdown, an exit breaks the loop instead of restarting. The flag is set on the first termination signal (`service-manager/src/lib.rs:381`).

`Oneshot` behaves differently (`service-manager/src/lib.rs:207-245`). A successful return breaks the loop. An error is logged. During shutdown, an error also breaks. Otherwise, the manager sleeps one second and retries. The retry is indefinite, as the comment says.

`Cron` has three parts (`service-manager/src/lib.rs:246-346`). If `run_immediately` is true, the manager spawns a task that runs now. That task retries up to eight times with one second between attempts. It does not retry after the eighth failure; it logs "cron failed all restart attempts". The manager then registers the schedule with `tokio_cron_scheduler::Job::new_async_tz` in the Coordinated Universal Time (UTC) time zone. It also spawns a pending dummy task. The comment says the dummy task prevents the start command from stopping when a cron is the only service (`service-manager/src/lib.rs:336-340`). The dummy task never finishes, so the join set stays non-empty between cron firings. The scheduled closure uses the same eight-attempt loop for each firing (`service-manager/src/lib.rs:298-334`). The immediate task and the dummy task are separate tasks.

No service in `run_config.rs` uses `Cron` at this commit. The type is available to other binaries that use the manager, such as test harnesses or future services.

### 5.4 The metrics service is implicit

```rust
// engine/packages/service-manager/src/lib.rs:142-160
pub async fn start(
	config: rivet_config::Config,
	pools: rivet_pools::Pools,
	mut services: Vec<Service>,
) -> Result<()> {
	services.push(Service::new(
		"metrics",
		ServiceKind::Core,
		|config, _pools| rivet_metrics_server::run_standalone(config),
		false,
	));

	// Spawn services
	tracing::info!(services=?services.len(), "starting services");
	let mut running_services = Vec::new();
	let cron_schedule = tokio_cron_scheduler::JobScheduler::new().await?;

	let mut term_signal = rivet_runtime::TermSignal::get();
	let shutting_down = Arc::new(AtomicBool::new(false));
```

The declared list in `run_config.rs` has ten entries, so a default `start` spawns eleven service tasks (`engine/src/run_config.rs:5-67`; `service-manager/src/lib.rs:147-152`). The `starting services` log reports that count. The `Oneshot` retry runs inside the spawned task, so the manager does not observe it. A metrics bind failure exits the process instead of returning an error to the manager (`metrics-server/src/server.rs:17-24`).

The manager appends the metrics service after it receives the list. The `start` command has already applied `--services` or `--except-services`. The metrics service is therefore not in the list that the operator can filter. It always runs. Its shutdown flag is `false`, so it is aborted on the first signal.

**Aha:** The metrics service is appended inside `start`, not declared in `run_config.rs`. The author chose this so that no service selection can remove observability. The obvious alternative is to declare `metrics` in the service list. That alternative would make `rivet-engine start --services api_peer` run without a metrics endpoint, because `start.rs` would select only the named service. The cost of the chosen design is that the metrics listener cannot be disabled. The `metrics` configuration section has only `host` and `port` (`config/src/config/metrics.rs:6-21`). The manager always appends the service (`service-manager/src/lib.rs:147-152`), and `run_standalone` always binds (`metrics-server/src/server.rs:12-25`).

The metrics server binds `metrics.host:metrics.port`, with defaults `[::]:6430` (`metrics-server/src/server.rs:12-15`, `metrics-server/src/lib.rs:4`). A bind failure calls `std::process::exit(1)` (`metrics-server/src/server.rs:17-24`). The comment says a server that cannot bind is critical. This exit happens inside a service task and bypasses the service manager.

### 5.5 The manager loop

After spawning, the manager waits. At the top of each loop iteration it builds a `join_fut` from the handles that are not finished (`service-manager/src/lib.rs:353-361`). It selects between that future and `term_signal.recv()` (`service-manager/src/lib.rs:363-401`).

The `join_fut` completes only when every handle in that set finishes. Long-running services do not finish. The `Oneshot` bootstrap task finishes, but the manager still waits for the rest. The `Cron` dummy task never finishes. A single finished handle does not complete the future or cause a rebuild. The set is rebuilt only after the future completes or after a signal wakes the select. A rebuild builds a new `FuturesUnordered` from the handles that are not finished (`service-manager/src/lib.rs:355-361`). A handle that finished before the rebuild is filtered out, so the manager does not rejoin it. The loop is therefore effectively a wait for a termination signal in normal operation.

On a signal, the manager does four things:

1. If this is the first signal, it spawns a force-shutdown timer. The timer sleeps `force_shutdown_duration` and then calls `std::process::exit(1)` with a warning that the timeout indicates a bug (`service-manager/src/lib.rs:369-379`).
2. It sets `shutting_down` to true.
3. It aborts every task whose `requires_graceful_shutdown` is false. It keeps the others.
4. If `recv()` returned true, it waits 50 milliseconds, calls `rivet_runtime::shutdown()`, and breaks the loop (`service-manager/src/lib.rs:393-399`).

If `recv()` returned false, the loop runs again. The manager now waits only for the graceful tasks, because the non-graceful tasks are no longer in `running_services`. A second signal repeats the abort step and returns false again. A third signal returns true and ends the process. The 50 millisecond sleep gives the aborted tasks a short time to observe cancellation before the runtime is torn down.

After the loop, the manager performs two final steps.

```rust
// engine/packages/service-manager/src/lib.rs:404-411
	// Shut down udb
	pools.udb()?.shutdown().await;

	// Stops term signal handler bg task
	rivet_runtime::TermSignal::stop();

	Ok(())
}
```

`pools.udb()` returns a clone of the UDB pool. The pool holds a `universaldb::Database` value, and that value is a cloneable handle over a shared driver (`pools/src/db/udb.rs:6-17`; `universaldb/src/database.rs:42-47`). `shutdown()` therefore acts on the shared driver, not on this handle alone. It flushes throttle charges and calls the driver's `shutdown` (`universaldb/src/database.rs:209-219`). The PostgreSQL driver aborts its resolver, garbage-collection, and pool-metrics tasks and hands off leadership (`universaldb/src/driver/postgres/database.rs:405-418`). Another holder of the pool observes the stopped tasks. It does not get a fresh driver. If the pool is missing, `?` returns an error from `start`. `Pools::new` always creates a UDB pool, so this error is not expected in the normal binary. The test constructor `Pools::test` also creates one (`pools/src/pools.rs:57-80`). `TermSignal::stop` aborts the signal handler task so it does not keep the runtime alive.

## 6. The service list

`engine/packages/engine/src/run_config.rs` is the declared list of services that the binary can start. The manager adds the metrics service after selection (`service-manager/src/lib.rs:147-152`).

```rust
// engine/packages/engine/src/run_config.rs:5-67
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
```

`Pegboard` is the Actor scheduler that document [04 Pegboard](04-pegboard-scheduling.md) explains. The table adds the implicit metrics service and the purpose of each entry. The Graceful column is the `requires_graceful_shutdown` field. The `guard` row carries `ApiPublic` because Guard hosts the `api-public` router; section 9 explains that arrangement.

| Service | Kind | Entry point | Graceful | Purpose |
|---|---|---|---|---|
| `api_peer` | `ApiPeer` | `rivet_api_peer::start` | No | Binds the API peer listener and serves local datacenter operations |
| `guard` | `ApiPublic` | `rivet_guard::start` | Yes | Binds the external listener, routes requests, and runs the `api-public` router in process |
| `workflow_worker` | `Standalone` | `rivet_workflow_worker::start` | Yes | Runs the Gasoline workflow worker and its merged workflow registry |
| `pegboard_outbound` | `Standalone` | `pegboard_outbound::start` | Yes | Holds outbound streams to serverless runners and drains them on shutdown |
| `bootstrap` | `Oneshot` | `rivet_bootstrap::start` | No | Creates first-run records and workflows |
| `dynamic_config` | `Core` | `rivet_dynamic_config::start` | No | Applies runtime configuration and tracing broadcasts |
| `version_management` | `Core` | `rivet_version_management::start` | No | Refreshes protocol versions from UniversalDB every 15 seconds |
| `epoxy_protocol_version` | `Core` | `epoxy::protocol_version::start` | No | Probes peer replicas for their Epoxy protocol version |
| `cache_purge` | `Core` | `rivet_cache_purge::start` | No | Subscribes to cache purge messages and purges the local cache |
| `profiling` | `Core` | `rivet_profiling::start` | No | Owns the optional Pyroscope continuous profiler for Central Processing Unit (CPU) samples |
| `metrics` | `Core` | `rivet_metrics_server::run_standalone` | No | Serves the Prometheus registry on port 6430 |

The API peer entry is small. The function reads the configured host and port, builds the router, binds a `TcpListener`, and calls `axum::serve` from axum, the web framework (`api-peer/src/lib.rs:16-30`). There is no explicit graceful shutdown in this function.

```rust
// engine/packages/api-peer/src/lib.rs:16-30
#[tracing::instrument(skip_all)]
pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> Result<()> {
	let host = config.api_peer().host();
	let port = config.api_peer().port();
	let addr = SocketAddr::from((host, port));

	let router = router::router(config, pools).await?;

	let listener = tokio::net::TcpListener::bind(addr).await?;
	tracing::info!(?host, ?port, "api-peer server listening");

	axum::serve(listener, router).await?;

	Ok(())
}
```

The `guard` entry does more. `rivet_guard::start` creates a `StandaloneCtx`, which is a Gasoline workflow context (`guard/src/lib.rs:12-22`). It builds a `SharedState` from the context and the pubsub pool (`guard/src/lib.rs:25`). `SharedState` holds three Pegboard gateway shared states and an optional JWT key ring cache (`guard/src/shared_state.rs:63-68`). Each gateway shared state holds the pubsub client, a gateway identifier, a receiver subject built from that identifier, and a map of in-flight requests. It also holds the hibernation timeout and the garbage-collection, tunnel-ping, message-acknowledgement, and pending-size settings (`pegboard-gateway/src/shared_state.rs:126-138`; `pegboard-gateway2/src/shared_state.rs:181-192`; `pegboard-gateway3/src/shared_state.rs:185-199`). The defaults are 90,000 milliseconds for the hibernation timeout, 15,000 milliseconds for the garbage-collection interval, 30,000 milliseconds for the tunnel-ping timeout, and 30,000 milliseconds for the message-acknowledgement timeout. The maximum pending size is 128 mebibytes (MiB). The source for these defaults is `config/src/config/pegboard.rs:271-355`. Gateway 3 also holds three buffered-response limits: 20 MiB for a buffered response, 384 queued messages, and 20 MiB for the streaming queue (`pegboard-gateway3/src/shared_state.rs:196-198`, `208-235`; `config/src/config/pegboard.rs:395-398`, `340-352`). The three fields are `pegboard_gateway`, `pegboard_gateway2`, and `pegboard_gateway3`. They back the three Actor gateway implementations `PegboardGateway`, `PegboardGateway2`, and `PegboardGateway3` (`guard/src/routing/pegboard_gateway/mod.rs:762-793`, `970`). The three implementations serve different routes. `PegboardGateway` serves path-based Actor requests through `handle_actor_v1` (`guard/src/routing/pegboard_gateway/mod.rs:809-978`). `PegboardGateway2` serves header-based Actor requests through `handle_actor_v2`, and it is the default (`guard/src/routing/pegboard_gateway/mod.rs:518-807`). Section 9.3 gives the full selection rule. In one line, gateway 3 receives only a candidate request kind, a supported envoy protocol version of at least 7, and a matching sampling decision. Every other request stays on gateway 2. `PegboardGateway3` is the streaming implementation. `select_gateway` chooses it from `features.guard_gateway_v3` (`guard/src/routing/pegboard_gateway/gateway3_selector.rs:122-172`). A WebSocket request, an unknown envoy protocol, an incompatible protocol, or the `off` mode stays on gateway 2 (`gateway3_selector.rs:146-164`). The three modes are `Off`, `Opportunistic`, and `On` (`gateway3_selector.rs:140-164`). In `opportunistic` mode, only a candidate request kind can move to gateway 3. The candidate kinds are an HTTP Server-Sent Events (SSE) request and an HTTP request whose body length is not known (`gateway3_selector.rs:16-36`, `183-199`). A WebSocket request and any other HTTP request are not candidates (`gateway3_selector.rs:33-35`). A missing protocol version, or one outside the supported range, also keeps the request on gateway 2 (`gateway3_selector.rs:147-155`). In `on` mode, the configured percentage samples requests (`gateway3_selector.rs:156-164`). The decision is a stable hash, not a random draw. The function hashes the namespace identifier and the Actor identifier with `xxh3_128` and reduces the result modulo 10,000 (`gateway3_selector.rs:229-246`). Each Actor therefore lands on the same side of the decision after a retry or a reconnect. The percentage compares the bucket with `percentage * 100`, and the values 0 and 100 short-circuit to false and true (`gateway3_selector.rs:229-236`). `SharedState::start` starts the three gateway states and the key ring cache (`shared_state.rs:41-52`). The start function then builds the routing function and the cache key function (`guard/src/lib.rs:29-30`). The cache key function produces the `u64` key for Guard's route cache (`guard-core/src/route.rs:43-48`).

The function tries three forms in order. First it tries an Actor path key (`guard/src/cache/mod.rs:20-45`). Then it tries an `actor` target key. That key uses the `x-rivet-target` header, or the `rivet_target.` prefix in the `sec-websocket-protocol` header, to find the target (`cache/mod.rs:47-75`). The function `build_cache_key_target_based` returns an error for every target except `actor` (`cache/pegboard_gateway.rs:40`). The caller treats that error as a miss and falls through to the next form, so the request is not rejected (`cache/mod.rs:71-78`). The function then reads the Actor identifier from `x-rivet-actor` or the `rivet_actor.` subprotocol (`cache/pegboard_gateway.rs:44-80`). Finally the function hashes the host, the path, and the method (`cache/mod.rs:77-88`). The three forms match the three routing inputs: an Actor path, an Actor target, and a request with neither. `x-rivet-target` is an HTTP request header. `rivet_target.` is a prefix for one entry in the WebSocket subprotocol list (`guard/src/routing/mod.rs:20-29`).

`ProxyService` is Guard's request proxy. `ProxyService::new` builds a `RouteCache` with a capacity of 10,000 entries. The time to live comes from `guard.route_cache_ttl_ms`, which defaults to 600 seconds (`guard-core/src/proxy_service.rs:117`, `137`; `guard-core/src/route.rs:46-55`; `config/src/config/guard.rs:33`, `189-191`). The insertion call is commented out at this commit, so the cache stays empty (`proxy_service.rs:190-196`). `handle_request` therefore computes the cache key, finds no entry, and calls the routing function on every request (`proxy_service.rs:748`, `145-180`). The capacity and time to live have no effect while insertion stays disabled. They take effect again only when the insertion call is restored. Routing correctness does not depend on the cache, because the miss path recomputes the route on every request. The routing function returns a `RoutingOutput`. `RoutingOutput::Route` carries a `RouteConfig`, which holds a vector of `RouteTarget` values (`guard-core/src/route.rs:11-21`). Each `RouteTarget` holds a host, a port, and a path. `RoutingOutput::CustomServe` carries a `CustomServeTrait` object (`route.rs:23-29`). The `api-public` route uses `CustomServe` because Guard runs the router in its own process. The start function then builds the optional TLS certificate resolver and calls `rivet_guard_core::run_server` (`guard/src/lib.rs:31-41`). The `api-public` router is created inside the routing function, not at startup. Section 9 follows that call.

The `workflow_worker` entry builds a merged registry from the workflow definitions that seven crates export. The crates are `pegboard`, `namespace`, `epoxy`, `gasoline_runtime`, `rivet_auth_jwt`, `datacenter`, and `depot` (`workflow-worker/src/lib.rs:14-23`). "Merged" means the worker registers every definition in one dispatch table, so a workflow from any crate can pull on the same worker. It starts a Gasoline `Worker` with no external shutdown receiver (`workflow-worker/src/lib.rs:5-12`). The worker listens for termination signals itself. `Worker::shutdown` takes `TermSignal::get()`, which is the same process-global watch that the manager uses (`gasoline/src/worker.rs:394-399`). The manager therefore does not send a message; the worker subscribes to the same signal. Section 11.3 covers its drain.

The `pegboard_outbound` entry creates a context, opens outbound connections, and on shutdown waits for those connections to close (`pegboard-outbound/src/lib.rs:27-53`). It uses `guard_shutdown_duration` for its drain budget. A serverless runner is a process outside this control plane that runs Actors on demand. The engine opens the connection to it, so the connection is outbound.

The `dynamic_config` entry joins two listeners: one for structured configuration and one for tracing configuration (`dynamic-config/src/lib.rs:21-29`). The first listens on `DynamicConfigSubject`, whose subject string is `rivet.config.dynamic`. The second listens on `TracingConfigSubject`, whose subject string is `rivet.debug.tracing.config` (`dynamic-config/src/pubsub_subjects.rs:5-6`). The first calls `Config::apply_dynamic` (`config/src/lib.rs:165-345`). The function starts from the current dynamic snapshot. It merges each present field of the incoming `DynamicConfigUpdate` over that snapshot. A field with a value overrides the snapshot. A field with an explicit null reverts to the value loaded at startup (`config/src/lib.rs:159-200`). An absent field leaves the snapshot unchanged (`config/src/dynamic.rs:12-14`). The merged result then passes `validate_and_set_defaults` (`config/src/lib.rs:341-348`). The function publishes it to readers only after validation succeeds. The update can change five groups. The first group is `sqlite.max_storage_bytes`, which caps the UniversalDB bytes that one SQLite database may occupy. A commit above the cap is refused with `depot.quota_exceeded` (`config/src/config/sqlite.rs:21-30`). Compaction is background depot work that folds recent deltas, offloads older pages to object storage, and reclaims freed space (`config/src/config/sqlite.rs:87-101`, `146-157`). The three job families are hot, cold, and reclaim. A hot job folds recent deltas on the primary store. A cold job moves older pages to object storage. A reclaim job deletes data that is no longer reachable. A branch is one independent SQLite database. A lane is one admitted path in the compaction pipeline, such as hot staging, hot install, cold staging, cold publish, or reclaim. Each lane scales the shared byte budget with its own multiplier (`config/src/config/sqlite.rs:103-157`). A utilization mark is the fraction of a lane's budget below which every charge is admitted. Above the mark, the admit probability ramps linearly to zero (`config/src/config/sqlite.rs:108-124`, `136-157`). The second group is `sqlite.compaction_admission_percent`. A stable hash of the branch identifier selects the admitted percentage of database branches, and only those branches start compaction jobs (`config/src/config/sqlite.rs:87-95`). The third group is four cluster-wide byte budgets: `compaction_write_bytes_per_second`, `compaction_read_bytes_per_second`, `actor_write_bytes_per_second`, and `actor_read_bytes_per_second` (`config/src/config/sqlite.rs:42-67`). They cap compaction writes, compaction reads, Actor page writes, and Actor page reads in bytes per second. A budget is cluster-wide because each process charges its own usage into windowed counters in UniversalDB. Every process reads those counters back to estimate the shared rate (`universaldb/src/throttle.rs:1-21`). The update value itself reaches each process over the `rivet.config.dynamic` broadcast. UniversalDB does not store the configured value. The counters, not the configuration, are the shared state. The fourth group is the compaction tuning fields: `compaction_hot_fold_direct_to_shard`, `compaction_max_hot_drain_span_txids`, and the three lane throttle settings. These settings scale the compaction byte budget per lane and set the utilization mark where admission ramps down (`config/src/dynamic.rs:59-107`; `config/src/config/sqlite.rs:96-157`, `180-194`). The fifth group is the workflow worker poll, pull, wake, and concurrency limits (`config/src/dynamic.rs:113-141`). The first four groups live at `config/src/dynamic.rs:17-107`. Only the fields in `DynamicConfigUpdate` can change. The second listener calls the tracing reload and OpenTelemetry sampler functions.

The `cache_purge` entry subscribes to `CachePurgeSubject`. The subject string is `rivet.cache.purge` (`cache/src/purge.rs:8-11`). On each message it calls `purge_local`, so a purge does not loop back through the message bus (`cache-purge/src/lib.rs:7-43`). `purge_local` removes the named keys from this process's cache and does not publish another message (`cache-purge/src/lib.rs:23-36`). The publisher is the cache request path. It publishes a `CachePurgeMessage` with `PublishOpts::broadcast()` (`cache/src/req_config.rs:395-417`). The api-peer route `POST /cache/purge` and the Epoxy propose path both reach that publisher (`api-peer/src/internal.rs:28`; `epoxy/src/ops/propose.rs:1120`).

On Linux and macOS with an `x86_64` or `aarch64` target, the `profiling` entry is a no-op when `config.pyroscope` is absent. It parks forever instead of returning, because a return would make the manager restart it in a loop (`profiling/src/pyroscope.rs:12-25`). When configured, it subscribes to `ProfileConfigSubject` and starts or stops a Pyroscope agent (`profiling/src/pyroscope.rs:35-79`). On any other target, such as Windows or a 32-bit architecture, the function logs a warning when `pyroscope` is configured. It logs a debug message otherwise, then awaits `std::future::pending::<()>()` forever (`profiling/src/lib.rs:1-31`).

### 6.1 Why `Core` and `Standalone` both exist

The source does not state a reason for the split. The two kinds have no operational difference at this commit:

- `ServiceKind::behavior()` maps both to `ServiceBehavior::Service`.
- Both are restarted after a crash or an unexpected exit.
- Both are aborted on the first termination signal if `requires_graceful_shutdown` is false.
- `start.rs` selects by name, not by kind.
- `Core` does not bypass the name filter. Only the metrics service bypasses it, because it is appended after the filter.

The likely reading is documentation. The comment says "Core" means "Run no matter what", and the grouping comment in `run_config.rs` separates Core services from the others. A reader can see which services are considered essential. The code does not enforce the label. This document does not invent a stronger reason, because the source gives none.

The `ApiPublic` and `ApiPeer` kinds have the same problem. They describe a role, not a behavior. The behavioral difference between the two API surfaces comes from their own code, not from the kind. Guard is graceful because its boolean is true. API peer is not graceful because its boolean is false.

## 7. The shared pools

`engine/packages/pools` is a library that every service receives. The `Pools` value holds the handles that are expensive to create and must be shared. These are the configuration, a process identifier, a drop guard, the pubsub client, the database, and the optional analytics client. The crate also provides three HTTP clients, the outbound policy, and the metrics registry as process-wide statics. The cache is not a pool. Each service builds a `CacheInner` from the configuration and the pools (`cache/src/inner.rs:16-29`).

The UniversalDB pool also installs the throttle. `Database::with_throttle` starts a background flusher, and the budget closure reads the dynamic configuration on every check (`pools/src/db/udb.rs:65-80`; `universaldb/src/database.rs:57-90`). A transaction opts in by calling `charge_throttle` with a throttle name and an axis, `read` or `write` (`universaldb/src/throttle.rs:1-5`). After that call, every byte the transaction reads or writes is charged automatically, except that a range clear must add its removed volume by hand (`throttle.rs:25-30`). `check_throttle` answers from process-local state without a database read (`throttle.rs:4-6`). Charges stay in process memory until the flusher folds them into windowed counters stored in UniversalDB (`universaldb/src/throttle.rs:3-8`). A window is a fixed time bucket, and the default width is 10,000 milliseconds (`throttle.rs:133-141`). The configured byte rate applies per window, so the budget for one window is the byte rate multiplied by the window width in seconds (`throttle.rs:133-135`). Reads charge as each operation returns, against a counter the transaction resolved when it opted in (`universaldb/src/throttle.rs:30-39`). A failed attempt still charges its reads, because those bytes left storage. Writes charge only after the transaction commits (`universaldb/src/throttle.rs:41-49`). Every process that opens the same database reads the same counters, so one budget covers every process that shares the database.

### 7.1 The pool structure

```rust
// engine/packages/pools/src/pools.rs:9-53
// TODO: Automatically shutdown all pools on drop
pub(crate) struct PoolsInner {
	pub(crate) config: Config,
	pub(crate) node_id: NodeId,
	pub(crate) _guard: DropGuard,
	pub(crate) ups: Option<UpsPool>,
	pub(crate) clickhouse: Option<clickhouse::Client>,
	pub(crate) udb: Option<UdbPool>,
}

#[derive(Clone)]
pub struct Pools(Arc<PoolsInner>);

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

`Pools` is a newtype over `Arc<PoolsInner>`. Cloning it is cheap. Every service receives its own clone, and all clones share the same inner value.

`Pools::new` starts the pubsub and database setups at the same time with `tokio::try_join!`. This is a small but real parallelism win. ClickHouse setup is synchronous and runs after the join. The function then creates the three HTTP clients with `reqwest`. `reqwest` is the Rust HTTP client library. The comment says this avoids a cold start in the first request.

The cancellation token is stored as a drop guard. `CancellationToken` is a tokio-util value that signals cancellation to its clones. `DropGuard` is the tokio-util handle that cancels the token when the guard drops. When the last `Pools` clone drops, the guard cancels the token. The TODO above the struct says that automatic shutdown of all pools on drop is not implemented. The token has no visible subscriber in this file. A search of the engine packages finds no subscriber at this commit. The guard therefore cancels a token that nothing observes; it is not a working drain path. This document therefore does not claim that it drains the pools.

The Rustls crypto provider is installed once. Rustls is a Rust implementation of TLS. `install_rustls_provider` tolerates a second installation and logs at debug level (`pools/src/pools.rs:113-118`). This matters because tests and embedded binaries can share the process.

### 7.2 Every pool and shared handle

| Handle | Type | Optional | Purpose | Source |
|---|---|---|---|---|
| `config` | `rivet_config::Config` | No | The validated configuration, with dynamic updates and protocol versions | `pools/src/pools.rs:11` |
| `node_id` | `NodeId` | No | A random UUID version 4 for this process | `pools/src/node_id.rs:6-18` |
| `_guard` | `tokio_util::sync::DropGuard` | No | Ties a cancellation token to the pool lifetime; full drain is a TODO | `pools/src/pools.rs:13`, `41` |
| `ups` | `UpsPool`, alias of `universalpubsub::PubSub` | Stored as `Option`, set by `new` | Publish and subscribe bus; NATS or in-process memory | `pools/src/pools.rs:14`, `pools/src/db/ups.rs:13` |
| `udb` | `UdbPool`, wrapping `universaldb::Database` | Stored as `Option`, set by `new` | Durable key space and transactions; PostgreSQL or RocksDB | `pools/src/pools.rs:16`, `pools/src/db/udb.rs:6-17` |
| `clickhouse` | `clickhouse::Client` | Yes | Optional analytics database client | `pools/src/pools.rs:15`, `pools/src/db/clickhouse.rs:6-9` |
| HTTP client | `reqwest::Client` via `OnceCell` | No | Requests to trusted engine endpoints; 30 second timeout | `pools/src/reqwest.rs:8-29` |
| Guarded HTTP client | `reqwest::Client` via `OnceCell` | No | Requests to user-supplied URLs; outbound policy at Domain Name System (DNS) and redirect time; 30 second timeout | `pools/src/reqwest.rs:31-42` |
| Guarded HTTP client without timeout | `reqwest::Client` via `OnceCell` | No | Long-lived streams such as serverless Server-Sent Events | `pools/src/reqwest.rs:44-51` |
| Outbound policy | `Arc<rivet_outbound_guard::Policy>` via `OnceCell` | No | The allow and deny policy for user-supplied URLs | `pools/src/reqwest.rs:71-80` |
| Metrics registry | Process-global `rivet_metrics::REGISTRY` | No | Prometheus collectors and the metrics server | `pools/src/metrics.rs:1-36` |

The getters make the optionality explicit. `ups()` returns `Result<UpsPool>` and errors with `Error::MissingUpsPool` when absent (`pools/src/pools.rs:96-98`). `udb()` does the same with `Error::MissingUdbPool` (`pools/src/pools.rs:108-110`). `clickhouse()` returns an error with a plain context message, and `clickhouse_option()` returns the option (`pools/src/pools.rs:100-106`). In the normal `new` path, UPS and UDB are always present. The options exist for tests and for embedded use.

`db::udb::setup` returns `Result<Option<UdbPool>>`, but both driver arms build a database and the function ends with `Ok(Some(...))` (`pools/src/db/udb.rs:19-61`). `Pools::new` also wraps the UPS pool in `Some` (`pools/src/pools.rs:42`). A `None` therefore cannot reach a getter in the normal binary. The `Option` fields are for the test constructor and for embedded callers.

`Pools::test` omits ClickHouse and skips the HTTP client warmup (`pools/src/pools.rs:55-80`). This is how tests run without an analytics service.

### 7.3 What is shared and what is not

The pools answer the question "what is shared" only partly.

- The configuration is shared by reference. Dynamic updates are shared through the `Config` value, not through the pools.
- The UDB driver is shared by every service. A transaction from the guard and a transaction from the bootstrap service go through the same driver.
- The pubsub client is shared. A publish from `api_peer` can reach a subscriber in `cache_purge` in the same process, and through NATS in another process.
- The HTTP clients are process-wide statics, shared even by code that does not hold a `Pools` value.
- The cache is not shared as a pool. `CacheInner::from_env(&config, pools.clone())` constructs a cache handle per service (`guard/src/lib.rs:13`, `bootstrap/src/lib.rs:6`, `pegboard-outbound/src/lib.rs:29`). The cache uses UPS to broadcast purge messages, so the entries are coordinated but not stored in one place.
- The `NodeId` is per process, not per service.
- The metrics registry is per process, not per service.

The service graph shows the flow from services to pools to drivers.

```mermaid
flowchart LR
    subgraph PROC["one rivet-engine process"]
        subgraph SVC["services"]
            AP["api_peer"]
            GD["guard (api-public router)"]
            WW["workflow_worker"]
            PO["pegboard_outbound"]
            BS["bootstrap"]
            DC["dynamic_config"]
            VM["version_management"]
            EP["epoxy_protocol_version"]
            CP["cache_purge"]
            PR["profiling"]
            MS["metrics"]
        end
        P["Pools (newtype over Arc of PoolsInner)"]
        UDB["udb field (UdbPool wrapping universaldb::Database)"]
        UPS["ups field (UpsPool alias of universalpubsub::PubSub)"]
        CH["clickhouse field (clickhouse::Client)"]
        RQ["three reqwest::Client statics (not Pools fields)"]
    end

    PG["PostgreSQL server"]
    NATS["NATS server"]
    CHS["ClickHouse server"]
    HTTP["external HTTP endpoints"]

    AP --> P
    GD --> P
    WW --> P
    PO --> P
    BS --> P
    DC --> P
    VM --> P
    EP --> P
    CP --> P
    PR --> P
    P --> UDB
    P --> UPS
    P --> CH
    P -. "warms once in Pools::new" .-> RQ
    UDB -. "Postgres driver only" .-> PG
    UPS -. "NATS driver only" .-> NATS
    CH -. "when configured" .-> CHS
    RQ --> HTTP
```

`Pools` is one in-process value. Its fields are inside the process boundary: `udb`, `ups`, `clickhouse`, `config`, `node_id`, and `_guard`. The three `reqwest::Client` values are also in-process, but they are process-wide statics, not `Pools` fields. `Pools::new` initializes them once (`pools/src/pools.rs:48-50`).

The server boxes are outside the process. The `udb` field reaches a PostgreSQL server only when the driver is PostgreSQL. With `FileSystem`, the driver is RocksDB and uses a local directory (`pools/src/db/udb.rs:19-61`). The `ups` field reaches a NATS server only when the driver is NATS. With `Memory`, the driver stays in the process (`universalpubsub/src/driver/memory/mod.rs:25-31`). The `clickhouse` field is optional. The metrics service receives the pools but does not use them; its closure parameter is `_pools` (`service-manager/src/lib.rs:150`). The diagram therefore shows no arrow from `metrics` to `Pools`.

## 8. Bootstrap of a fresh datacenter

`engine/packages/bootstrap` runs once at startup as an `Oneshot` service. It creates the records and workflows that a new datacenter needs. The service is mostly idempotent. Fourteen dispatch sites use `.unique()`: five in the concurrent group, the JSON Web Token (JWT) key rotation after the join, and eight in the backfill module. The default namespace does not use `.unique()`; it uses a name check inside the namespace workflow.

```rust
// engine/packages/bootstrap/src/lib.rs:5-34
pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> Result<()> {
	let cache = rivet_cache::CacheInner::from_env(&config, pools.clone())?;
	let ctx = StandaloneCtx::new(
		db::DatabaseKv::new(config.clone(), pools.clone()).await?,
		config.clone(),
		pools,
		cache,
		"bootstrap",
		Id::new_v1(config.dc_label()),
		Id::new_v1(config.dc_label()),
	)?;

	tokio::try_join!(
		async {
			// Replicas must exist before coordinator
			setup_epoxy_replica(&ctx).await?;
			setup_epoxy_coordinator(&ctx).await
		},
		create_default_namespace(&ctx),
		backfill::run(&ctx),
		setup_pegboard_metrics_aggregator(&ctx),
		setup_gas_pruner(&ctx),
		setup_datacenter_ping(&ctx),
	)?;

	// Must happen after coordinator is reconfigured
	setup_auth_jwt_key_rotation(&ctx).await?;

	Ok(())
}
```

The function creates a `StandaloneCtx` with the label `"bootstrap"`. The constructor takes `db`, `config`, `pools`, `cache`, `name`, `ray_id`, and `req_id` (`gasoline/src/ctx/standalone.rs:38-46`). The two `Id` values fill `ray_id` and `req_id`. A `ray_id` identifies one request tree. A `req_id` identifies one request in that tree. `MessageCtx` is the context for publishing and subscribing messages. It carries the `ray_id` so that work started by a message stays in the same trace (`gasoline/src/ctx/message.rs:20-29`). `Id::new_v1(label)` builds an identifier from 16 random UUID version 4 bytes plus a big-endian `u16` label (`util-id/src/lib.rs:41-45`). The context passes `ray_id` to its `MessageCtx` (`standalone.rs:53`). It records both identifiers in its tracing span (`standalone.rs:49-51`). Bootstrap has no incoming request, so it passes two fresh identifiers that carry the datacenter label. The 16 UUID bytes are random on each call, so two bootstraps get different `ray_id` values (`util-id/src/lib.rs:41-45`). Then it runs six tasks concurrently.

The first task is a sequence. `setup_epoxy_replica` must finish before `setup_epoxy_coordinator`. The comment states the reason: the replicas must exist before the coordinator. The replica task dispatches the replica workflow with the tag `("replica", ctx.config().epoxy_replica_id())` (`bootstrap/src/lib.rs:80-84`). The tag value is a `u64`. The tag map stores a non-string value as its compact JSON text, so the identifier becomes a decimal string (`gasoline/src/db/kv/mod.rs:3873-3879`). The coordinator task returns early when this datacenter is not the leader (`bootstrap/src/lib.rs:53-58`). On the leader, it dispatches the coordinator workflow with the tag `("replica", ctx.config().epoxy_replica_id())` (`bootstrap/src/lib.rs:60-65`). It then sends a `Reconfigure` signal to that workflow (`bootstrap/src/lib.rs:68-72`).

The dependency is a handshake. An Epoxy replica is the workflow for one datacenter. It holds that datacenter's copy of the replicated state. A coordinator is the leader datacenter's workflow. It owns cluster membership and the replication protocol. `BeginLearning` is a workflow signal. A signal is a durable message that UniversalDB stores in a workflow mailbox. The Gasoline signal builder writes it with `ctx.signal(...).to_workflow::<...>().tag(...).send()` or `ctx.signal(...).to_workflow_id(...).send()` (`bootstrap/src/lib.rs:68-72`; `epoxy/src/replica/message_request.rs:112-125`). `BeginLearning` carries the `ClusterConfig` (`epoxy/src/workflows/replica/mod.rs:34-37`). `ClusterConfig` holds the coordinator replica identifier, a configuration epoch, and one `ReplicaConfig` per replica. Each `ReplicaConfig` holds the replica identifier, its status, its API peer address, and its Guard address (`epoxy/src/types.rs:8-23`). The status is `Joining`, `Learning`, or `Active` (`types.rs:31-36`).

The replica workflow enters `setup_replica` and blocks on `ctx.listen::<BeginLearning>()` (`epoxy/src/workflows/replica/setup.rs:15-19`). The coordinator workflow first runs an activity that sends `BeginLearningRequest` to every replica (`epoxy/src/workflows/coordinator/mod.rs:31-35`). The replica HTTP handler receives that request and signals the replica workflow by its `replica` tag (`epoxy/src/replica/message_request.rs:112-125`). The route is `POST /v{version}/epoxy/message` (`epoxy/src/http_routes.rs:18`, `41-49`). A coordinator that starts before the replica workflow exists would send the request with no workflow to receive the signal, and the replica would wait forever. The dispatch order prevents that.

The later `Reconfigure` signal is separate. `reconfigure()` compares the cluster configuration with the topology (`epoxy/src/workflows/coordinator/reconfigure.rs:12-17`). If a new replica exists, it health-checks the replica, marks it as joining, broadcasts the joining configuration, and sends `BeginLearning` to the new replicas (`reconfigure.rs:18-48`). The health check waits for the replica to come online, or stops the pass when the configuration changes. The broadcast reaches every replica and pushes the new configuration. `BeginLearning` then gives each new replica the configuration it must catch up to (`reconfigure.rs:18-48`). If no new replica exists, it pushes the current configuration to every replica (`reconfigure.rs:50-53`). Bootstrap sends `Reconfigure` after both workflows exist (`bootstrap/src/lib.rs:68-72`).

The second task creates the default namespace. It returns early on a non-leader (`bootstrap/src/lib.rs:90-94`). On the leader, it resolves the name `"default"` in the local namespace table. If no namespace exists, it dispatches a namespace workflow. The workflow receives `Id::new_v1(dc_label)`, the name `"default"`, and the display name `"Default"` (`bootstrap/src/lib.rs:96-119`). This is the only namespace that a fresh datacenter gets for free. A follower does not run this branch. In multi-node mode it reads the same namespace table from the shared database. A follower with its own file-system database has no default namespace.

The dispatch is not unique (`bootstrap/src/lib.rs:107-115`). Every process in a leader datacenter runs this branch, so two processes can pass the name lookup at the same time. The namespace workflow's `insert_db` activity prevents duplicate records. `insert_db` is the activity that writes a namespace record and its name index in one transaction. `ByNameKey` is the UniversalDB key for that name index (`namespace/src/workflows/namespace.rs:145-178`). `insert_db` reads `ByNameKey` with `Serializable` isolation (`namespace/src/workflows/namespace.rs:159-163`). It returns `NameNotUnique` when the name exists. Otherwise it writes the name index in the same transaction (`namespace.rs:165-170`). UDB rejects the losing transaction, and the retry finds the winner. The workflow catches `NameNotUnique`, sends a `Failed` message, and returns `Ok(())` (`namespace.rs:43-51`). The message target is the topic `("namespace_id", namespace_id)` (`namespace.rs:44-46`, `75-78`). The public namespace-create endpoint creates the identifier first with `Id::new_v1`. It then subscribes to `CreateComplete` and to `Failed` on the topic `("namespace_id", namespace_id)` before it dispatches (`api-peer/src/namespaces.rs:97-116`). The subscription subject comes from that topic (`gasoline/src/ctx/message.rs:195-202`). Correlation is by the identifier: the endpoint generated it, so only its workflow publishes on that topic. A `tokio::select!` waits for either message. The select has no timeout branch, so a request waits until the workflow publishes one (`namespaces.rs:118-125`). The `Failed` branch returns the carried error to the caller (`namespaces.rs:118-124`). `CreateComplete` is an empty message that the workflow sends after it creates the namespace successfully (`namespace/src/workflows/namespace.rs:53`, `73`). The default-namespace branch in bootstrap neither subscribes to that topic nor waits for the workflow to run. `dispatch()` commits the workflow record and its tag indexes in one UniversalDB transaction (`gasoline/src/db/kv/mod.rs:931-962`). It does not run the workflow body, so the `insert_db` activity and the namespace record are not committed yet (`gasoline/src/builder/common/workflow.rs:128-150`). The branch therefore logs a created workflow even when the workflow later reports a conflict. Concurrent bootstraps can therefore create one namespace record and one extra workflow that reports the conflict. They cannot create two records with the name `"default"`.

The third task runs the backfill module. The fourth creates the Pegboard metrics aggregator workflow. The fifth creates the Gasoline pruner workflow. The sixth creates the datacenter ping workflow. The last three use `.unique()` with no tags (`bootstrap/src/lib.rs:124-157`). An untagged `.unique()` is idempotent by workflow name, as section 4 explains. Concurrent instances therefore create one workflow each, not one per instance.

The backfill module is in `bootstrap/src/backfill.rs`. It runs schema and data backfills for older datacenters. It dispatches eight workflows, and each dispatch uses `.unique()` (`bootstrap/src/backfill.rs:6-104`). Seven are untagged, so they are idempotent by name. The Epoxy backfill carries the tag `("replica", replica_id)` (`bootstrap/src/backfill.rs:32-38`). A fresh datacenter has nothing to backfill, so the tasks complete quickly. The document does not describe each backfill; document [07 UniversalDB](07-universaldb-and-buckets.md) covers the storage migrations.

Each backfill is guarded twice. `is_complete` reads a `backfill::CompleteKey` with `Snapshot` isolation (`bootstrap/src/backfill.rs:106-127`). Every backfill workflow calls a `MarkComplete` activity when it finishes. Four workflows define that activity: the Actor runner name selector, Epoxy, Depot compaction, and dead-workflow backfills (`pegboard/src/workflows/actor_runner_name_selector_backfill.rs:86-101`; `epoxy/src/workflows/backfill.rs:191-211`; `depot/src/workflows/compaction_backfill.rs:271-291`; `gasoline-runtime/src/workflows/dead_wf_backfill.rs:89-109`). The serverless, runner pool, runner pool 2, and Actor migration fix workflows import `MarkCompleteInput` from the Actor runner module. They call the `MarkComplete` activity with `MarkCompleteInput { name: BACKFILL_NAME.to_string() }` (`pegboard/src/workflows/serverless/backfill.rs:36-39`; `pegboard/src/workflows/runner_pool_backfill.rs:42-45`; `pegboard/src/workflows/runner_pool2_backfill.rs:71-74`; `pegboard/src/workflows/actor_migration_fix_backfill.rs:46-49`). The input type and the activity are defined together in the Actor runner module (`pegboard/src/workflows/actor_runner_name_selector_backfill.rs:81-104`). The activity writes a `backfill::CompleteKey` (`actor_runner_name_selector_backfill.rs:86-101`). `Snapshot` is safe because `CompleteKey` is a monotonic marker. A stale false result can cause a redundant dispatch, and the `.unique()` guard suppresses it while the first workflow still exists. A later bootstrap skips a completed backfill. The `.unique()` dispatch is the second guard. Two processes can pass `is_complete` at the same time, but only one dispatch creates the workflow.

The final step runs after the join. `setup_auth_jwt_key_rotation` reads the authentication configuration. It returns early when JWT authentication is disabled or when this datacenter is not the leader (`bootstrap/src/lib.rs:36-41`). On the leader, it dispatches the key-rotation workflow with the tag `("auth", "jwt")` (`bootstrap/src/lib.rs:43-49`). The comment at line 30 says this must happen after the coordinator is reconfigured.

The failure path is a consequence of the service kind. If any task in the `try_join!` returns an error, `start` returns that error. `tokio::try_join!` returns early and drops the other five branch futures (`bootstrap/src/lib.rs:17-28`). Work that those branches already committed stays. Work inside a dropped future stops. The `Oneshot` behavior logs the error and retries after one second, forever (`service-manager/src/lib.rs:218-235`). A retry runs all six branches again.

The retry is safe because every creation has an identity. Two tagged `.unique()` creations are idempotent by name and tag: the replica and the coordinator (`bootstrap/src/lib.rs:60-65`, `79-84`). Three untagged `.unique()` creations are idempotent by name: the metrics aggregator, the gas pruner, and the datacenter ping (`bootstrap/src/lib.rs:126-130`, `138-142`, `150-154`). The eight backfill dispatches are idempotent by name or by name and tag (`bootstrap/src/backfill.rs:6-104`). The JWT key-rotation workflow is also tagged `.unique()`, and it runs after the join (`bootstrap/src/lib.rs:43-48`). The default-namespace branch is not unique. Its `insert_db` activity enforces the name check. A retry therefore either finds the existing record or creates one new record (`bootstrap/src/lib.rs:96-119`; `namespace/src/workflows/namespace.rs:159-170`). The service never blocks the other services, because the manager spawns it in parallel with them. A broken bootstrap can therefore repeat in the log while the API listeners are already serving.

## 9. The two API surfaces

The repository has two API crates: `api-public` and `api-peer`. They serve different callers. They are not two network services in the `run_config.rs` list.

`api-public` has no listener of its own in `run_config.rs`. The `guard` service hosts it. Guard's routing function receives a request, selects a target, and calls `api_public::route_request` (`guard/src/routing/api_public.rs:53-85`). That function builds the axum router and wraps it in `ApiPublicService`. `ApiPublicService` implements `CustomServeTrait`, and the function returns it as `RoutingOutput::CustomServe(Arc<dyn CustomServeTrait>)` (`guard/src/routing/api_public.rs:16-49`, `82-84`; `guard-core/src/route.rs:23-29`). The trait has no blanket implementation for `axum::Router`; the wrapper supplies `handle_request`. Guard needs the trait because `RoutingOutput` is either a proxy route or a local handler (`guard-core/src/route.rs:23-29`). `ApiPublicService` holds only the router and clones it per request, so no state survives between requests (`guard/src/routing/api_public.rs:16-49`). That method clones the router, calls its `tower::Service::call`, collects the response body, and converts it to `ResponseBody` (`api_public.rs:22-48`). `ProxyService::handle_request` receives the `CustomServe` variant, collects the request body once, and calls `handler.handle_request` for each retry attempt (`guard-core/src/proxy_service.rs:1005-1054`). A local call is retried because the handler can report a transient guard error while an Actor starts or a tunnel opens. The retry predicate accepts only guard-group codes such as `service_unavailable`, `actor_ready_timeout`, or `tunnel_message_timeout`. It also accepts a 503 or 504 response with a matching `x-rivet-error` header (`guard-core/src/utils.rs:465-502`). The full guard set is `service_unavailable`, `actor_ready_timeout`, `actor_wake_retries_exceeded`, `actor_stopped_while_waiting`, `tunnel_request_aborted`, `tunnel_message_timeout`, `tunnel_response_closed`, and `gateway_response_start_timeout` (`utils.rs:475-490`). Any other error or status returns to the caller without another attempt. The body is cloned from the collected copy for each attempt. A retry can therefore repeat a non-idempotent handler call, but only after one of those transient results. The router therefore runs inside the Guard process.

`api-peer` is a network service. It binds `api_peer.host:api_peer.port` and calls `axum::serve` (`api-peer/src/lib.rs:16-30`). Other datacenters reach it through `peer_url`.

### 9.1 The api-peer router

```rust
// engine/packages/api-peer/src/router.rs:10-42
	create_router("api-peer", config, pools, |mut router| {
		router = epoxy::http_routes::mount_routes(router);
		router
			// MARK: Namespaces
			.route("/namespaces", get(namespaces::list))
			.route("/namespaces", post(namespaces::create))
			// MARK: Runner configs
			.route("/runner-configs", get(runner_configs::list))
			.route("/runner-configs/{runner_name}", put(runner_configs::upsert))
			.route(
				"/runner-configs/{runner_name}",
				delete(runner_configs::delete),
			)
			// MARK: Actors
			.route("/actors", get(actors::list::list))
			.route("/actors", post(actors::create::create))
			.route("/actors", put(actors::get_or_create::get_or_create))
			.route("/actors/{actor_id}", delete(actors::delete::delete))
			.route("/actors/names", get(actors::list_names::list_names))
			.route(
				"/actors/{actor_id}/kv/keys/{key}",
				get(actors::kv_get::kv_get),
			)
			.route("/actors/{actor_id}/sleep", post(actors::sleep::sleep))
			.route(
				"/actors/{actor_id}/reschedule",
				post(actors::reschedule::reschedule),
			)
			// MARK: Runners
			.route("/runners", get(runners::list))
			.route("/runners/names", get(runners::list_names))
			// MARK: Envoys
			.route("/envoys", get(envoys::list))
```

The router also mounts depot inspection routes under `/depot/inspect/...`. It mounts internal routes under `/cache/purge`, `/epoxy/...`, and `/debug/profile/config` (`api-peer/src/router.rs:43-105`). These routes are for operators and for other engine components. They are not part of the public API.

The api-peer handlers are local datacenter operations. The rule is written in the engine instructions:

```markdown
<!-- engine/CLAUDE.md:123-125 -->
## API routing

- `api-public` owns cross-datacenter forwarding for external requests; `api-peer` handlers should be local datacenter operations and must not add forwarding requirements.
```

The rule is an instruction to contributors, not a runtime check. The code follows it in the handlers that this document opened.

### 9.2 The api-public router

```rust
// engine/packages/api-public/src/router.rs:64-101
	create_router("api-public", config, pools, move |router| {
		router
			// Root redirect
			.route(
				"/",
				axum::routing::get(|| async { Redirect::permanent("/ui/") }),
			)
			// MARK: Metadata
			.route("/metadata", axum::routing::get(metadata::get))
			// MARK: Namespaces
			.route("/namespaces", axum::routing::get(namespaces::list))
			.route("/namespaces", axum::routing::post(namespaces::create))
			.route(
				"/auth/tokens",
				axum::routing::post(auth::tokens::create)
					.layer(axum::extract::DefaultBodyLimit::max(32 * 1024)),
			)
			.route(
				"/auth/tokens/inspect",
				axum::routing::get(auth::tokens::inspect),
			)
			.route("/runner-configs", axum::routing::get(runner_configs::list))
			.route(
				"/runner-configs/serverless-health-check",
				axum::routing::post(runner_configs::serverless_health_check),
			)
			.route(
				"/runner-configs/{runner_name}",
				axum::routing::put(runner_configs::upsert),
			)
			.route(
				"/runner-configs/{runner_name}",
				axum::routing::delete(runner_configs::delete),
			)
			.route(
				"/runner-configs/{runner_name}/refresh-metadata",
				axum::routing::post(runner_configs::refresh_metadata),
			)
```

The router continues with Actor routes, runner routes, envoy routes, datacenter routes, the health fanout, and the user interface routes (`api-public/src/router.rs:102-141`). It then adds a Cross-Origin Resource Sharing (CORS) layer that mirrors the request origin and an authentication middleware (`api-public/src/router.rs:142-155`).

`create_router` also installs a permissive CORS layer (`api-builder/src/router.rs:67-72`). The order in that function matters. It installs the 404 fallback first (`api-builder/src/router.rs:67-68`). It then calls `.layer(CorsLayer::permissive())` before it adds `/health` and before it merges the caller's router (`api-builder/src/router.rs:69-72`). In axum 0.6, `Router::layer` wraps only the routes that exist at that call (`axum` 0.6.20 `src/routing/mod.rs:233-248`). The permissive layer therefore wraps the fallback route only. It does not wrap `/health` or the `api-public` routes. Those routes arrive later through the merge. The mirror layer inside the `api-public` closure is the CORS layer for them (`api-public/src/router.rs:142-150`). The tower-http CORS layer overwrites a response header with the same name when it extends the response (`tower-http` 0.6.11 `src/cors/mod.rs:764`). The two layers therefore cannot combine two `Access-Control-Allow-Origin` values. The effective policy is permissive CORS on the fallback, no CORS on `/health`, and mirror CORS on the `api-public` routes.

The cross-datacenter forwarding is visible in the namespace list handler.

```rust
// engine/packages/api-public/src/namespaces.rs:45-58
	if ctx.config().is_leader() {
		rivet_api_peer::namespaces::list(ctx.into(), (), query).await
	} else {
		let leader_dc = ctx.config().leader_dc()?;
		request_remote_datacenter::<ListResponse>(
			ctx.config(),
			leader_dc.datacenter_label,
			"/namespaces",
			axum::http::Method::GET,
			Some(&query),
			Option::<&()>::None,
		)
		.await
	}
```

On the leader, `api-public` calls the `api-peer` handler directly in the same process. On a follower, it sends an HTTP request to the leader's `peer_url`. The helper is `request_remote_datacenter`. It joins the endpoint to `peer_url` and uses the trusted HTTP client (`api-util/src/lib.rs:63-97`). This is the exact meaning of "api-public owns cross-datacenter forwarding".

Another helper, `fanout_to_datacenters`, sends a request to every datacenter and runs the local handler for the current datacenter without an HTTP hop (`api-util/src/lib.rs:99-156`). The callers are six list handlers. Four are the envoy list, the runner-config list, the runner list, and the runner name list. Two are the Actor name list and the Actor list (`api-public/src/envoys.rs:46`; `api-public/src/runner_configs/list.rs:72`; `api-public/src/runners.rs:47`, `111`; `api-public/src/actors/list_names.rs:63`; `api-public/src/actors/list.rs:196`). The health route does not call this helper. It implements its own local-versus-remote split: it collects the local report directly and calls `send_health_checks` for the remote datacenters (`api-public/src/health.rs:87-105`).

### 9.3 There is no path prefix between external and peer routes

The two routers mount most routes at the same paths. For example, both have `/namespaces` and `/actors`. The separation is by listener and by target, not by a path prefix.

- External traffic arrives at Guard on port 6420, or at the HTTPS port.
- Peer traffic arrives at API peer on port 6421.
- Inside Guard, routing first tries path-based Actor, runner, and envoy routes (`guard/src/routing/mod.rs:212-254`).
- A matching path route returns immediately. A conflicting `x-rivet-target` header is ignored (`guard/src/routing/mod.rs:215-254`).
- Guard reads the `x-rivet-target` header, or the `rivet_target.` WebSocket subprotocol, only after every path route misses (`guard/src/routing/mod.rs:256-279`).
- If no target is present, Guard tries `api-public` as the default (`guard/src/routing/mod.rs:340-356`). If a target is present but no header route matches, Guard returns `NoRoute` (`guard/src/routing/mod.rs:358-365`).
- `api-public` is reached only when the target is `"api-public"` or when the default branch runs (`guard/src/routing/api_public.rs:58-61`).

The first stage matches three concrete path shapes. The Actor shape is `/gateway/{actor_id}` or `/gateway/{actor_id}/{tail}`, with an optional `@token` on the identifier (`guard/src/routing/actor_path.rs:158-193`). The runner shape is `/runners/connect` or `/runners/connect/` (`guard/src/routing/runner.rs:36-40`). The envoy shape is `/envoys/connect` or `/envoys/connect/` (`guard/src/routing/envoy.rs:36-40`). A path such as `/namespaces` matches none of them, so it reaches the default `api-public` branch. The target list has no `peer` value. Peer handlers run only in the `api_peer` process on port 6421; Guard never serves that router (`api-peer/src/lib.rs:16-30`). An external client therefore cannot select a peer handler with the target header.

The accepted target strings are `actor`, `runner`, `envoy`, and `api-public` (`guard/src/routing/pegboard_gateway/mod.rs:205`, `runner.rs:17`, `envoy.rs:17`, `api_public.rs:59`). `actor` routes to a Pegboard Actor through the gateway. `runner` routes to the runner tunnel, not to a service in `run_config.rs`. That route accepts `/runners/connect` or a header-targeted request and serves `PegboardRunnerWsCustomServe` (`guard/src/routing/runner.rs:11-61`). `PegboardRunnerWsCustomServe` is a Guard-local WebSocket handler, not a forward to a service (`pegboard-runner/src/lib.rs:31-69`). It holds a `StandaloneCtx`, parses the tunnel URL, and initializes the runner connection. It then subscribes to three subjects before it accepts the client WebSocket: `pegboard.runner.{runner_id}`, `pegboard.runner.eviction-by-id.{runner_id}`, and `pegboard.runner.eviction-by-name.{namespace_id}.{runner_name}.{runner_key}` (`pegboard-runner/src/lib.rs:88-114`; `pegboard/src/pubsub_subjects.rs:20`, `43`, `74`). It subscribes first so the proxy can retry a failed connection. The `envoy` tunnel returns `PegboardEnvoyWs` instead (`guard/src/routing/envoy.rs:60-61`). Neither tunnel is `pegboard_outbound`; that service manages engine-initiated serverless connections (`pegboard-outbound/src/lib.rs:28-53`). `envoy` opens the Actor-to-engine tunnel that section 3.8 defines. `api-public` routes to the external API router. A header-targeted `actor` request calls `select_gateway` after the Actor is ready. The selector returns gateway 3 or gateway 2 (`guard/src/routing/pegboard_gateway/mod.rs:748-805`). The path-based `/gateway/...` route uses `PegboardGateway` and `handle_actor_v1` (`guard/src/routing/pegboard_gateway/mod.rs:809-978`). A header route matches the target string, not the path. Each target module checks the target before it handles the request (`guard/src/routing/envoy.rs:17-19`; `api_public.rs:58-61`).

Both routers get a `/health` route from `create_router`. The helper adds `/health`, a 404 fallback, permissive CORS, a trace layer, logging middleware, and the `ApiCtx` middleware, which supplies the per-request API context (`api-builder/src/router.rs:53-90`). The fixed order is 404 fallback, permissive CORS, trace layer, `/health`, merge of the caller router, logging middleware, then `ApiCtx` middleware (`api-builder/src/router.rs:67-87`). `ApiCtx` holds a ray identifier, a request identifier, and a `StandaloneCtx` (`api-builder/src/context.rs:11-16`). It dereferences to `StandaloneCtx`, so a handler reaches the database, configuration, pools, and cache through it. The `api-public` router wraps this value in its own `ApiCtx`. That type adds the token, the key-ring cache, the authentication state, and a flag that records whether authentication ran (`api-public/src/ctx.rs:11-20`). The health response contains `status`, `runtime`, and the package version (`api-builder/src/router.rs:92-99`). The `runtime` value is the literal string `"engine"` (`api-builder/src/router.rs:96`).

This design has a cost. A client cannot tell the two surfaces apart from the path alone. An operator must know which port or which header routes the request. The source does not state a reason for this choice. It is a consequence of reusing the same `create_router` helper and the same route paths for both surfaces.

## 10. The subcommands

`engine/packages/engine/src/lib.rs` defines ten subcommands. Two have aliases: `Database` also accepts `db`, and `Workflow` also accepts `wf`. Clap derives the other names from the variant names in kebab case. Every derived name here is one word. `Config`, `Auth`, `Tracing`, `Profile`, `Epoxy`, `Depot`, and `Udb` therefore appear as `config`, `auth`, `tracing`, `profile`, `epoxy`, `depot`, and `udb`.

```rust
// engine/packages/engine/src/lib.rs:10-58
#[derive(Parser)]
pub enum SubCommand {
	/// Starts the Rivet server
	Start(start::Opts),
	/// Manages databases
	#[clap(alias = "db")]
	Database {
		#[clap(subcommand)]
		command: db::SubCommand,
	},
	/// Manages workflows
	#[clap(alias = "wf")]
	Workflow {
		#[clap(subcommand)]
		command: wf::SubCommand,
	},
	/// Manage the Rivet config
	Config {
		#[clap(subcommand)]
		command: config::SubCommand,
	},
	/// Manage authentication state
	Auth {
		#[clap(subcommand)]
		command: auth::SubCommand,
	},
	/// Manage tracing configuration
	Tracing {
		#[clap(subcommand)]
		command: tracing::SubCommand,
	},
	/// Manage CPU profiling
	Profile {
		#[clap(subcommand)]
		command: profile::SubCommand,
	},
	/// Epoxy debug commands
	Epoxy {
		#[clap(subcommand)]
		command: epoxy::SubCommand,
	},
	/// Depot SQLite diagnostic commands
	Depot {
		#[clap(subcommand)]
		command: depot::SubCommand,
	},
	/// Allows inspection of UDB data
	Udb(udb::Opts),
}
```

Every variant except `Start` receives only the configuration. `Start` also receives the run configuration. The dispatch is a direct match with no shared behavior. The `Database` variant keeps the name `Database`; `db` is an alias. The `Workflow` variant keeps the name `Workflow`; `wf` is an alias. Clap accepts either spelling.

| Subcommand | Main implementing file | Reads the store | Side effects |
|---|---|---|---|
| `start` | `engine/src/commands/start.rs` | Yes, through admission | Durable write of `EngineVersionKey` and protocol versions; starts services |
| `db` | `engine/src/commands/db/mod.rs` | Depends on the subcommand | `db nuke` deletes the local database directory; `db shell` queries ClickHouse |
| `wf` | `engine/src/commands/wf/mod.rs` | Yes | Durable workflow history and state writes |
| `config` | `engine/src/commands/config.rs` | No direct store use | Dynamic-config broadcast; local validation and apply |
| `auth` | `engine/src/commands/auth.rs` | Yes | Durable JWT key-ring writes |
| `tracing` | `engine/src/commands/tracing.rs` | No | Tracing-config broadcast |
| `profile` | `engine/src/commands/profile.rs` | No | HTTP request to the profiling endpoint |
| `epoxy` | `engine/src/commands/epoxy.rs` | Through API peer | Coordinator state changes over HTTP for some commands |
| `depot` | `engine/src/commands/depot.rs` | Yes | Durable repair and import writes; export and scan read only |
| `udb` | `engine/src/commands/udb/mod.rs` | Yes | Direct UniversalDB writes |

### 10.1 `start`

`start` is the only subcommand that runs the service manager. It accepts `--services` and `--except-services`; Clap marks them as conflicting (`engine/src/commands/start.rs:10-18`). If neither is present, it runs the full list. If `--except-services` is present, it removes each named service and fails when a name is unknown. If `--services` is present, it selects each named service in the given order and fails when a name is unknown (`start.rs:33-69`). The manager then spawns the service tasks in vector order, and the tasks run concurrently after they start (`service-manager/src/lib.rs:163-204`). The order therefore controls task creation, not request ordering.

Before starting, it redirects logs when `logs.redirect_logs_dir` is set. The `rivet_logs` crate creates that directory, writes one file per UTC day named `log-MM-DD-YY`, and points standard output and standard error at the file (`logs/src/unix.rs:32-90`). The retention is seven days (`start.rs:7-8`, `26-31`). It then creates the pools, calls `rivet_version_management::check_engine_admission`, and calls `rivet_service_manager::start` (`start.rs:71-76`).

The admission check is a serial transaction. `EngineVersionKey` is a single UniversalDB key whose value is the engine version last admitted for this database (`version-management/src/keys.rs:5-23`). The check reads `EngineVersionKey` and rejects a version rollback unless `runtime.allow_version_rollback` is true. It writes the current version and updates the protocol versions in the same transaction (`version-management/src/lib.rs:56-104`). The current version goes to `EngineVersionKey`, whose tuple is `(RIVET, VERSION, ENGINE)` (`version-management/src/keys.rs:25-33`). The transaction then calls `update_protocol_versions` (`version-management/src/lib.rs:93-96`). That function writes one `ProtocolVersionKey` per protocol with the same `MutationType::Max` rule that the 15-second refresh uses (section 3.8). The two writers are the admission check and the 15-second refresh. They use the same keys and the same values. The comment says: "We serializably read this key and write it in the same txn making all engine connections serial. This is by design" (`version-management/src/lib.rs:91-92`). The scope is every call to `check_engine_admission`. Each starting engine process calls it once, so startup admission is serial across the datacenter. The comment does not make every later UDB transaction serial. Those transactions run with the normal isolation rules (`universaldb/src/database.rs:110-154`). This admission call is the first durable action of a starting process. The write stays if a later step fails. No shutdown path deletes `EngineVersionKey`, and no service removes it. A process that admits, then fails to bind a port, leaves the version recorded (`version-management/src/lib.rs:65-99`). A protocol entry is removed only when a later scan finds it more than 30 seconds old (`version-management/src/lib.rs:127-139`). If the failed process was the only writer, that entry remains until the next process starts.

### 10.2 `db`

The `db` subcommand has two operations. `db shell` opens a `clickhouse-client` shell against the ClickHouse database named by the `service` argument, with an optional query (`engine/src/util/db.rs:6-9`, `52-56`). The database type enum currently has only `Clickhouse`, with the alias `ch` (`engine/src/commands/db/mod.rs:4-25`, `27-50`). `db nuke` deletes the local file-system database directory. It requires `--yes`. The PostgreSQL branch returns "nuke postgres not implemented" (`db/mod.rs:51-67`).

### 10.3 `wf`

The `wf` subcommand inspects and repairs Gasoline workflows. Its variants are `get`, `list`, `silence`, `wake`, `revive`, `dead`, `prune-history`, `history`, `repair`, `signal`, and `registry` (`engine/src/commands/wf/mod.rs:22-121`). `get` prints the named workflows. `list` filters by tags, name, and state. `silence` hides a workflow so it does not appear as dead or running again. `wake` sets the immediate-wake property. `revive` wakes dead workflows that match a name and an error substring. `dead` lists dead workflows and the repairs that apply. A dead workflow is one whose last attempt failed and which can no longer make progress on its own. Gasoline records it until an operator repairs, revives, or silences it (`gasoline/src/db/debug.rs:402-408`). `prune-history` deletes history for completed workflows before a timestamp. `history` prints the event graph with optional JSON, forgotten events, locations, and timestamps. `repair` validates the raw history, prints each check, applies a repair, wakes the workflow, and verifies that it replays (`wf/mod.rs:92-110`). `signal` reads and edits workflow signals (`engine/src/commands/wf/signal.rs:8-40`). `registry` prints the current workflow registry (`wf/mod.rs:119-120`).

This subcommand is an operator tool for the durable execution engine. Document [05 Gasoline](05-gasoline-durable-execution.md) explains the history and replay model.

### 10.4 `config`

`config show` prints the current configuration in the process that runs the command. `config update` changes a supported field at runtime. It applies the change to its own process first (`engine/src/commands/config.rs:333-337`). The comment gives the reason. The operator sees a rejected value as an error in the command, instead of finding it in every process log after the broadcast (`config.rs:333-335`). It then publishes a `SetDynamicConfigMessage` to `DynamicConfigSubject` with `PublishOpts::broadcast()` (`config.rs:339-345`). The receiving `dynamic_config` service applies the change through `Config::apply_dynamic`. The broadcast reaches other processes only when the pubsub driver is NATS. The default `Memory` driver keeps subscribers in the publishing process (`universalpubsub/src/driver/memory/mod.rs:25-31`).

The command is a separate process from the running engine, and it exits after the publish. With `Memory`, the publish reaches only subscribers in the command's own process. That process runs no `dynamic_config` service, so no subscriber applies the change. The running engine is a different process and keeps its old value. An operator who uses `config update` with `Memory` therefore sees a success message and no effect on the engine.

Only the fields declared in `DynamicConfigUpdate` can change. At this commit these are five groups. The first group is `sqlite.max_storage_bytes`, the UniversalDB byte cap on one SQLite database (`config/src/config/sqlite.rs:21-30`). The second group is `sqlite.compaction_admission_percent`, the percentage of database branches admitted to run compaction (`config/src/config/sqlite.rs:87-95`). The third group is the four compaction and actor byte-rate budgets (`config/src/config/sqlite.rs:42-67`). The fourth group is the compaction throttle settings, the hot-fold mode switch, and the hot-drain span cap (`config/src/dynamic.rs:59-107`). The fifth group is the workflow worker poll, pull, wake, and concurrency limits (`config/src/dynamic.rs:113-141`). The first four groups live at `config/src/dynamic.rs:17-107`. An omitted value reverts the field to the value loaded from the file. `Config::apply_dynamic` runs the same validation as a config loaded from disk, and it publishes nothing when validation fails (`config/src/lib.rs:159-165`; `config/src/config/mod.rs:260`). A zero or invalid value therefore fails validation, and the update is not published.

### 10.5 `auth`

The `auth` subcommand manages JWT signing keys. `auth jwt keys` prints the authoritative public key ring and the signing deadlines (`engine/src/commands/auth.rs:19-20`). The authoritative ring is the latest key-ring snapshot in UniversalDB, and every process reads it (`auth-jwt/src/ops/rotation/status.rs:46-50`). The signing deadlines are the active rotation due time, the active hard signing deadline, the pending-key deadline, and the retiring-key deadlines (`status.rs:33-42`). `auth jwt rotate` stages a generation-fenced normal rotation (`auth.rs:21-26`). Staged means the rotation becomes pending until the running key-rotation workflow commits it. The command does not write the new key itself. `auth jwt revoke-key` is the emergency path. It replaces the active signer at once and revokes the named key identifiers (`auth.rs:27-36`). The command calls `rivet_auth_jwt::ops::rotation::request_emergency::Input { request_id, expected_generation, revoke_kids }` (`auth.rs:78-93`). The operation validates the request against the current key ring, finds the key-rotation workflow by its tags, and sends it an `EmergencyRotate` signal (`auth-jwt/src/ops/rotation/request_emergency.rs:19-50`). The command then polls the key-ring status until `last_emergency_receipt` carries its own generated request identifier, with a 120-second deadline (`auth.rs:94-102`, `138-145`). The command does not write the key ring in UniversalDB itself. The key-rotation workflow commits the emergency rotation. Both changes require a matching `--expected-generation` and `--yes` (`auth.rs:11-43`, `46-80`). A generation is a counter stored on the key-ring snapshot (`auth-jwt/src/ops/rotation/status.rs:21`, `59-60`). Each committed rotation adds one to that counter (`auth-jwt/src/planner.rs:138-149`, `482-486`). The command reads the current generation and requires equality with `--expected-generation` (`engine/src/commands/auth.rs:128-136`). The rotation operation repeats the check before it sends the signal. It finds the key-rotation workflow by its tags and sends `StartRotation` to that workflow (`auth-jwt/src/ops/rotation/request_normal.rs:26-38`). This fence prevents a command from acting on a key ring that another operator has already changed.

The commands use a `StandaloneCtx` and JWT ops. They do not call the HTTP API. This makes them usable during an incident when the API is degraded. Document [13 Auth](13-auth-and-namespaces.md) covers the key ring and the token format.

### 10.6 `tracing`

`tracing config` publishes a `SetTracingConfigMessage` to `TracingConfigSubject` with a broadcast publish (`engine/src/commands/tracing.rs:6-40`). The same NATS condition applies. The default `Memory` driver delivers the message to subscribers in this process only (`universalpubsub/src/driver/memory/mod.rs:25-31`). The command process runs no `dynamic_config` service and exits after the publish. With `Memory`, the message therefore reaches no subscriber, and the running engine keeps its old tracing settings. The message carries an optional log filter and an optional sampler ratio. The log filter flag is `--filter`, short `-f`. The sampler ratio flag is `--sampler-ratio`, short `-s` (`tracing.rs:6-20`). An empty filter resets the filter to the default. A flag with no value resets the sampler ratio to the default. The `dynamic_config` service reloads the filter and calls `set_sampler_ratio`.

The command prints the new values after a successful publish. It does not confirm that another process applied them. This is a fire-and-forget broadcast.

### 10.7 `profile`

`profile enable` and `profile disable` send an HTTP `PUT` to `/debug/profile/config` on an API peer endpoint. The endpoint defaults to `http://localhost:6421` (`engine/src/commands/profile.rs:6-23`). The request body has `enabled` and an optional `sample_rate` (`profile.rs:25-30`). The api-peer router mounts the route at `/debug/profile/config` (`api-peer/src/router.rs:105`). The handler publishes a `SetProfileConfigMessage` to `ProfileConfigSubject` with a broadcast publish (`api-peer/src/internal.rs:56-73`). The `profiling` service receives that message and starts or stops the agent.

The command does not use the `--config` value; its `execute` parameter is `_config` (`profile.rs:33`). The endpoint comes from the flag or the default.

### 10.8 `epoxy`

The `epoxy` subcommand is a set of debug operations against the local replica and the coordinator. It can print replica debug state and get or set the coordinator cluster state. It can also trigger a coordinator reconfiguration. It can get a key locally or with an optimistic read. It can set a key or fan a key lookup out to all replicas (`engine/src/commands/epoxy.rs:9-104`). Each command takes an optional `--endpoint`, which is the API peer base URL. When it is absent, the command uses the topology `peer_url` and removes a trailing slash (`epoxy.rs:299-311`). The key argument uses tuple format, the same syntax as the UniversalDB command-line interface. The `--base64` flag reads the key as base64 instead (`epoxy.rs:33-42`; `engine/src/commands/udb/key_parser.rs:1-25`). An optimistic read calls the `/epoxy/replica/kv/{key}/optimistic` route. The server asks the local replica and its cache first, then fans out to the other replicas (`epoxy.rs:74-95`).

The routes it calls are mounted by `epoxy::http_routes::mount_routes` in the api-peer router (`api-peer/src/router.rs:11`). The internal handlers are under `/epoxy/coordinator/...` and `/epoxy/replica/...` (`api-peer/src/router.rs:79-104`). Document [08 Epoxy](08-epoxy-multi-region.md) explains the protocol that these commands inspect.

### 10.9 `depot`

The `depot` subcommand diagnoses and repairs Depot-backed SQLite storage. Its variants are `doctor`, `execute`, `export`, `import`, `lookup-actor-namespaces`, `scan-corruption`, and `repair` (`engine/src/commands/depot.rs:17-33`). `doctor` runs storage diagnostics. `execute` runs Structured Query Language (SQL) against one database. `export` copies a database without changing it. `import` loads an export into an empty local UniversalDB database. `import` requires the `FileSystem` backend, which is RocksDB (`depot_transfer.rs:195-200`). The check is `verify_local_import_config`, which asserts that `config.database()` matches `DatabaseConfig::FileSystem` before the transfer starts (`depot_transfer.rs:195-200`, called at `454`). The source gives the restriction in the assertion message but does not record a reason for excluding PostgreSQL. `lookup-actor-namespaces` maps Actor identifiers to namespace identifiers. `scan-corruption` uses snapshot reads only. `repair` writes and verifies an exact backup before it changes the database (`depot.rs:19-33`).

Document [06 Depot](06-actor-sqlite-and-depot.md) covers the storage format and the repair model.

### 10.10 `udb`

The `udb` subcommand opens an interactive UniversalDB shell, or runs one query with `-q`. The shell has history at `/tmp/rivet-udb-cli-history` and keeps a current tuple path in the prompt (`engine/src/commands/udb/mod.rs:17-71`). The path is fixed, so every `udb` shell on the host shares one history file. The file is not keyed by database or cluster. It survives across shells because the shell writes it on normal exit (`udb/mod.rs:40-58`). A shell that is killed before exit loses that session's entries. It creates the pools and uses `pools.udb()` directly. This is the lowest-level inspector for the durable key space. Document [07 UniversalDB](07-universaldb-and-buckets.md) explains the key encoding.

## 11. Shutdown

Shutdown has three levels: the signal handler, the service manager, and each service. The levels run in that order. The third signal ends the sequence without waiting for the service drains.

### 11.1 The signal sequence

The signal handler counts SIGTERM and SIGINT on Unix, or Ctrl+C on Windows (`runtime/src/term_signal.rs:49-75`). The count starts at zero. Each signal increments it. The value sent to subscribers is `count >= 3`.

| Signal | `recv()` result | Manager action |
|---|---|---|
| First | `false` | Start force timer, set `shutting_down`, abort non-graceful tasks, keep waiting |
| Second | `false` | Run the abort step again; only graceful tasks remain, so it aborts none; keep waiting |
| Third | `true` | Sleep 50 milliseconds, call `rivet_runtime::shutdown()`, break |

Each signal sends a new value on the `watch` channel. The channel notifies the manager even when the value is still `false`, so the second signal wakes the loop.

The force timer is spawned only once, on the first signal (`service-manager/src/lib.rs:369-379`). Its duration is `force_shutdown_duration`, which defaults to 10 minutes. When it expires it logs a warning and calls `std::process::exit(1)`. The warning text says the timeout indicates a bug.

A service that does not respect termination is not killed by the manager on the first signal. The manager keeps its handle and waits. The force timer is the backstop. A service is killed earlier only when its `requires_graceful_shutdown` flag is false, because the manager calls `abort()` on its handle.

### 11.2 The service manager drain

On the first signal, the manager iterates `running_services` and aborts every task with `requires_graceful_shutdown == false`. It keeps the rest (`service-manager/src/lib.rs:383-391`). The retained set for the default service list is:

| Retained task | Signal source | Behavior on drain |
|---|---|---|
| `guard` | `TermSignal::get()` at `guard-core/src/server.rs:77`; `recv()` at `295-310` | Hyper (the HTTP library) graceful shutdown, then wait for connection tasks up to `guard_shutdown_duration` |
| `workflow_worker` | `TermSignal::get()` at `gasoline/src/worker.rs:97`; `recv()` at `163` and `443-448` | Stop leasing, stop running workflows, wait up to `worker_shutdown_duration` |
| `pegboard_outbound` | `TermSignal::get()` at `pegboard-outbound/src/lib.rs:51`; `recv()` at `76-81` | Wait for outbound connections to close up to `guard_shutdown_duration` |

The manager does not send a shutdown message to these services. Each service reads the process-global `TermSignal` directly (`runtime/src/term_signal.rs:112-116`). The manager keeps their task handles and waits.

Everything else is aborted at once. This includes `api_peer`, `bootstrap` if it is still running, `dynamic_config`, `version_management`, `epoxy_protocol_version`, `cache_purge`, `profiling`, and `metrics`.

The flag comes from `run_config.rs`. Only `guard`, `workflow_worker`, and `pegboard_outbound` set it to true. The other seven entries and the appended `metrics` service set it to false (`engine/src/run_config.rs:5-67`; `service-manager/src/lib.rs:147-152`). An aborted `version_management` task stops its 15-second refresh. Its protocol entry then stops updating, and another process's scan deletes it after 30 seconds (`version-management/src/lib.rs:10`, `127-139`).

The consequence is important. An in-flight HTTP request to the API peer port is not drained. `api-peer` calls `axum::serve` with no graceful shutdown (`api-peer/src/lib.rs:27`). When the manager aborts the task, the future is dropped, the listener closes, and in-flight peer requests fail. The caller sees a reset connection or an incomplete response, not a structured error. This includes cross-datacenter requests that another datacenter forwarded to this one.

An in-flight request to Guard is different. Guard owns the public listener and implements a drain. The connection task remains active until it completes or the drain budget expires.

### 11.3 Guard and the workflow worker

Guard's server loop runs until a shutdown signal. It then computes the remaining connection tasks and starts a graceful shutdown with Hyper, the HTTP library (`guard-core/src/server.rs:267-293`). The graceful shutdown stops accepting new connections and waits for active requests. After hyper finishes, Guard waits for its own connection tasks with `wait_idle()` on the HTTP and HTTPS factories. The loop selects between completion, an abort signal, a progress tick every 7 seconds, and a timeout after `guard_shutdown_duration` (`guard-core/src/server.rs:295-323`). The shutdown signal and the abort signal both come from `TermSignal::get()`, the process-global watch that section 5.3 defines (`guard-core/src/server.rs:77`, `257`, `305`). The manager does not send Guard a private message. The default budget is 10 minutes. When the budget expires, Guard logs a warning and returns. It does not force-kill the connection tasks; returning ends the service future, and the manager then proceeds.

The workflow worker's drain starts in `Worker::shutdown` (`gasoline/src/worker.rs:395-473`). It reads `worker_shutdown_duration`, with a default of 30 seconds. It spawns a shutdown ping task. That task writes `LastPingTsKey` and `VersionKey` but not `LastActivePingTsKey` or `ActiveWorkerIdxKey` (`gasoline/src/db/kv/mod.rs:846-898`, called with `false` at `worker.rs:385`). The active worker index is the keyed list that a workflow pull uses to divide workflow hashes among live workers. Each entry is an `ActiveWorkerIdxKey` that holds the last active ping timestamp, the worker version, and the worker identifier (`gasoline/src/db/kv/keys/worker.rs:141-163`). A pull scans entries whose timestamp is at most two ping intervals old, keeps the highest version, and sorts by worker identifier (`gasoline/src/db/kv/mod.rs:1191-1192`, `1288-1322`). The scan boundary is `now - 2 * PING_INTERVAL` (`gasoline/src/db/kv/mod.rs:1192`). The ping interval is 10 seconds (`gasoline/src/worker.rs:24`), so an entry becomes invisible to the pull 20 seconds after its last active ping. The pull does not delete that entry; the key simply falls below the scan start. The worker then computes its index in that sorted list. It pulls only workflows whose hash modulo the worker count equals its own index or the next index (`gasoline/src/db/kv/mod.rs:1441-1449`). The next index is deliberate redundancy. It keeps a workflow reachable when a worker leaves the active set and the sorted list shrinks. The source comment states the cost: more transaction conflicts (`gasoline/src/db/kv/mod.rs:1442-1445`).

A workflow lease has no independent expiry timestamp. `LeaseKey` stores the workflow name and the worker identifier (`gasoline/src/db/kv/keys/workflow.rs:10-35`). The GC task calls `clear_expired_leases` every 10 seconds (`gasoline/src/worker.rs:26`, `330-347`). That function reads every lease and compares the worker's `LastPingTsKey` with `now - WORKER_LOST_THRESHOLD_MS`, which is 30 seconds (`gasoline/src/db/kv/mod.rs:571-599`; `gasoline/src/worker.rs:33`). When the last ping is older than that threshold, GC clears the lease, clears `WorkerIdKey`, and writes an immediate wake condition so another worker can pull the workflow (`gasoline/src/db/kv/mod.rs:600-650`). A lost worker's lease therefore survives until its last ping is more than 30 seconds old and a GC pass observes it. The shutdown ping therefore stops advertising the worker in the active-worker index while its heartbeat timestamp continues to advance. The next step calls `mark_worker_inactive`, which deletes the entry (`gasoline/src/db/kv/mod.rs:901-918`). The worker then sends a stop signal to every running workflow. Then it waits for the workflow task handles. The loop selects between completion, a progress tick every 7 seconds, an abort signal, and the timeout. If the timeout expires, it logs "worker shutdown timed out" and continues. It then aborts the ping task and logs how many workflows did not stop.

The worker's stop signal is cooperative. A stop point is a call to `check_stop` before an activity. If the stop flag is set, the call returns `WorkflowError::WorkflowEvicted` and the workflow yields (`gasoline/src/ctx/workflow.rs:533-539`). A workflow that does not reach a stop point within 30 seconds remains in the `running_workflows` map. The worker still returns. A lease is a UniversalDB record that grants one worker the right to run a workflow. Another worker may take the workflow only after that lease expires, or after the owner misses the 30-second worker-lost threshold (`gasoline/src/worker.rs:32-33`, `292-300`). Document [05 Gasoline](05-gasoline-durable-execution.md) explains why a replay is safe.

`pegboard_outbound` uses the same shape. It waits for outbound connection tasks, logs progress every 7 seconds, and uses `guard_shutdown_duration` as the budget (`pegboard-outbound/src/lib.rs:51-70`). A serverless stream that does not close within the budget is abandoned. Abandoned means the service stops waiting for its task and returns. The task then ends when the runtime shuts down, so the runner can see the connection close without a completion message (`pegboard-outbound/src/lib.rs:70-80`).

### 11.4 The final steps

After the retained services finish, or after the third signal, the manager runs the code in section 5.5. It calls `pools.udb()?.shutdown().await` (`service-manager/src/lib.rs:404-405`). This is the only pool with an explicit shutdown call in the manager. That call first flushes throttle charges and then shuts down the driver (`universaldb/src/database.rs:209-218`). A throttle charge is the byte count that a transaction accumulates against a budget shared by every process that uses the database. A background flusher normally folds these counts into shared windowed counters every interval (`universaldb/src/throttle.rs:1-8`). The final flush folds the remaining charges into the shared counters before the driver stops (`universaldb/src/throttle.rs:539-545`). The Postgres driver aborts three tasks (`universaldb/src/driver/postgres/database.rs:407-410`). The resolver task finds the leader and routes commits. The garbage-collection task removes expired deduplication rows. The pool-metrics task samples the leader and follower pool occupancy into gauges (`database.rs:266-315`; `resolver/mod.rs:106-116`). It then calls `resolver::handoff`, which releases the leader lease and, in multi-node mode, publishes an election wake on the NATS messaging system (`universaldb/src/driver/postgres/resolver/mod.rs:224-241`). An election wake is an empty NATS message on the election subject. It tells standby replicas to try the lease now instead of waiting for the next poll. The FileSystem driver keeps the default no-op shutdown, so it has no lease to release (`universaldb/src/driver/mod.rs:38-43`). The manager then calls `TermSignal::stop()` and returns.

On the forced path, this call races the runtime shutdown. The manager calls `rivet_runtime::shutdown()`, which sends the notification (`runtime/src/lib.rs:53-60`). The `select!` in `run` then returns `None` (`runtime/src/lib.rs:41-47`). The `select!` drops the command future when the notification wins. The manager may enter `pools.udb()?.shutdown().await`, but the drop can end that future before the call finishes. The UDB driver shutdown and the lease handoff are therefore not guaranteed on the forced path.

If the services finish normally, `run` receives `Some(Ok(()))` and returns it to `main`.

The runtime is dropped at the end of `run`. Tokio shuts down its worker threads and drops any remaining tasks. The process exits with the result of `main`.

The full shutdown has this sequence:

```mermaid
sequenceDiagram
    participant OP as Operator
    participant TS as TermSignal
    participant SM as service_manager::start
    participant SH as rivet_runtime::shutdown
    participant RT as rivet_runtime::run
    participant GD as guard
    participant WW as workflow_worker
    participant PO as pegboard_outbound
    participant UDB as UdbPool

    OP->>TS: SIGTERM (count 1)
    TS-->>SM: recv() = false
    SM->>SM: spawn force-shutdown timer
    SM->>SM: abort tasks with requires_graceful_shutdown = false (api_peer, metrics, and others)
    SM->>GD: keep task
    SM->>WW: keep task
    SM->>PO: keep task
    Note over GD,WW: each retained service calls TermSignal::recv() itself. The manager sends no shutdown message
    Note over PO: PO also calls TermSignal::recv() itself

    alt Services finish first
        par Guard drain
            GD->>GD: graceful.shutdown(), wait_idle()
        and Worker drain
            WW->>WW: mark inactive, stop workflows, wait
        and Outbound drain
            PO->>PO: wait for connections to close
        end
        GD-->>SM: Ok(())
        WW-->>SM: Ok(())
        PO-->>SM: Ok(())
        SM->>UDB: shutdown()
        SM->>TS: TermSignal::stop()
        SM-->>RT: Some(Ok(()))
        RT-->>OP: main returns, process exits
    else Third signal during drain
        OP->>TS: SIGTERM (count 2)
        TS-->>SM: recv() = false
        Note over SM: the abort step runs again. No non-graceful task remains
        OP->>TS: SIGTERM (count 3)
        TS-->>SM: recv() = true
        Note over SM: the drains above are still running
        SM->>SM: sleep 50 milliseconds
        SM->>SH: rivet_runtime::shutdown()
        SH->>SH: notify_one() on the SHUTDOWN Notify
        Note over RT: the select! branch wakes and returns None. The manager receives no reply
        par UDB shutdown races the drop
            SM->>UDB: shutdown() (may finish or be dropped)
        and Runtime drops the command future
            RT->>RT: drop the command future (the manager)
        end
        RT->>RT: drop the runtime and its remaining tasks
        RT-->>OP: main returns, process exits
    end
```

The forced path is an alternative to the normal drain, not a step after it. It does not wait for Guard, the worker, or the outbound connections. The second signal repeats the abort step. The third signal sends the runtime notification. `run` then returns `None` to `main`, and the runtime drops the command future.

## 12. A worked example

This section walks through the example configuration in section 3.7. The operator starts the container with this file and no extra flags.

1. `Cli::parse()` reads the subcommand `start` and the global `--config` path. If the path is `/etc/rivet/config.jsonc`, the default directory is not read.
2. `Config::load` adds `Root::default()` as the first source. That value has no optional sections, so the accessors supply their defaults after deserialization (section 3.3). The default database is `FileSystem`, and the default pubsub is `Memory`. The default topology has one datacenter named `default`. Its label is 1 and it is the leader. Its `public_url` is `http://127.0.0.1:6420` and its `peer_url` is `http://127.0.0.1:6421` (`config/src/config/topology.rs:47-76`).
3. The file overrides `auth`, `api_peer`, `topology`, `outbound`, `postgres`, and `clickhouse`. The flattened `postgres` table sets the `Database` enum to `Postgres`. If the table were absent, the `database()` accessor would return the default `FileSystem`. The `clickhouse` table sets the optional analytics client.
4. The file does not set `pubsub`, so the `pubsub()` accessor returns the default `Memory`. The file does not set `postgres.nats`, so UniversalDB runs in single-node mode.
5. `validate_and_set_defaults` passes, because `auth.admin_token` is set. The single datacenter does not need `valid_hosts`; this file sets it anyway. `force_shutdown_duration` is absent, so it defaults to 10 minutes. `worker_shutdown_duration` defaults to 30 seconds. `guard_shutdown_duration` defaults to 10 minutes. The force timer is not shorter than either drain.
6. `rivet_telemetry::init` reads `telemetry.enabled`. The file does not set it, and the default is true, so Sentry is initialized unless the file or the environment disables it. The process also logs through the tracing subscriber.
7. `run_config::config` returns the ten declared services in section 6.
8. `start` selects all services, because there are no include or exclude flags.
9. `Pools::new` builds the pubsub client from `Memory`, the UDB driver from `Postgres`, the ClickHouse client from the `clickhouse` table, and the three HTTP clients.
10. `check_engine_admission` opens the Postgres-backed UDB, reads `EngineVersionKey`, writes the current version, and updates the protocol versions in the same transaction.
11. The service manager appends `metrics` and spawns all eleven tasks. The metrics server binds `[::]:6430`. The API peer binds `0.0.0.0:6421`, because the file overrides the host. Guard binds `[::]:6420`, because the file does not set a host or port. HTTPS is disabled, because `guard.https` is absent.
12. `bootstrap` creates the Epoxy replica workflow. The datacenter is the leader. It therefore also creates the coordinator, the default namespace, the metrics aggregator, the pruner, and the ping workflow. It dispatches the eight backfill workflows. It creates the JWT rotation workflow when JWT is enabled.
13. `api_peer` serves local calls. Guard serves external calls and builds the `api-public` router on demand.

The result is one control plane instance. It is also the whole cluster, because the topology has one datacenter and there is no shared pubsub or multi-node database.

A second process with the same file names the same PostgreSQL database. If the first process still renews its lease, the second process does not start. Single-node mode acquires a leader lease on that database (`universaldb/src/driver/postgres/database.rs:165-188`). The gate is `acquire_single_node_gate` (`universaldb/src/driver/postgres/resolver/mod.rs:60`). `GATE_TOTAL` is 25 seconds, because it is `LEASE_TTL_SECS * 2 + 5` and `LEASE_TTL_SECS` is 10 (`resolver/mod.rs:33-36`; `resolver/lease.rs:8`). The gate tries once per second (`resolver/mod.rs:38`, `60-100`). The deadline check runs after each attempt, so startup ends on the first attempt at or after 25 seconds. A failure tells the operator to configure NATS for a multi-node setup (`resolver/mod.rs:91-97`). The source comment gives the requirement: the gate must exceed the lease time-to-live so a crashed predecessor's lease can expire (`resolver/mod.rs:33-35`). The source does not explain the factor of two or the extra five seconds. A live first process renews every 3 seconds (`resolver/mod.rs:28-29`), so its lease does not expire and the second process cannot acquire it. The lease row is not deleted. A graceful release sets its expiry to the current time, so a standby can take over at once (`resolver/lease.rs:70-83`). A crashed leader stops renewing, and the row expires after 10 seconds (`resolver/lease.rs:7-8`, `46-61`). The second process can acquire the lease after that expiry, or at once after a graceful release. The `Memory` pubsub would also be private to each process.

A two-process datacenter therefore needs `pubsub: nats` and `postgres.nats`, or a single `pubsub: nats` section that the Postgres section inherits (`config/src/config/mod.rs:268-276`). In that mode the two processes share one database and one pubsub bus. The `.unique()` workflows then de-duplicate first-run work across the processes (section 8).

## 13. Failure paths

The startup path has several failure points. Each one has a different consequence.

| Failure | Consequence | Source |
|---|---|---|
| The configuration file is unreadable | `Config::load` returns an error; `main_inner` returns before telemetry; the process exits | `config/src/lib.rs:401-428` |
| A configuration path does not exist | The path is ignored with a debug log; startup continues with the remaining sources | `config/src/lib.rs:358-370` |
| A configuration file has an unsupported extension | `add_file_source` returns an error; the process exits | `config/src/lib.rs:410-425` |
| An unknown configuration key is present | Deserialization fails because of `deny_unknown_fields`; the process exits | `config/src/config/mod.rs:70-71` |
| Authentication is absent | Validation fails with an actionable message; the process exits | `config/src/config/mod.rs:260-266` |
| The database connection fails | `Pools::new` returns an error; the start command exits | `pools/src/pools.rs:32-36` |
| The engine version is older than the stored version | Admission fails unless `allow_version_rollback` is true; the process exits | `version-management/src/lib.rs:71-89` |
| The metrics port is already bound | `run_standalone` calls `std::process::exit(1)` | `metrics-server/src/server.rs:17-24` |
| The API peer port is already bound | `api_peer` returns an error; the manager logs "service crashed" and retries after one second | `api-peer/src/lib.rs:24-27`, `service-manager/src/lib.rs:185-197` |
| Guard fails to bind | The service returns an error; the manager logs the crash and retries | `guard-core/src/server.rs:34-40` |
| A service panics | The task ends; the restart loop was inside the task, so it cannot restart; the manager keeps waiting for the other handles | `service-manager/src/lib.rs:170-198` |
| Bootstrap fails | The `Oneshot` task logs the error and retries after one second, forever | `service-manager/src/lib.rs:218-235` |
| A graceful drain exceeds its budget | Guard or the worker logs a warning and returns; the manager continues | `guard-core/src/server.rs:318-321`, `gasoline/src/worker.rs:449-452` |
| The force timer expires | `std::process::exit(1)` | `service-manager/src/lib.rs:373-378` |

The panic case deserves attention. The restart loop is inside the spawned task. A panic unwinds that task. The manager only sees that the join handle finished. The current `join_fut` ignores finished handles and waits for the others (`service-manager/src/lib.rs:355-362`). The manager therefore keeps waiting and does not restart the panicked service. The next loop iteration can occur only after every remaining handle finishes or a signal arrives (`service-manager/src/lib.rs:364-401`). A service that catches its own errors and returns `Err` is restarted. A service that panics is not. The source does not install a panic hook that restarts tasks.

The metrics bind failure is the other exceptional case. It bypasses the manager and exits the process immediately. The comment says a bind failure is critical. This is consistent with the implicit-service design: the process should not run without its metrics endpoint.

## 14. Observability at startup

Observability starts before the configuration is loaded. The runtime installs the tracing subscriber and the OTel provider guard. The first log line is therefore visible even when the configuration is invalid. This is a deliberate ordering choice. The alternative, initialize logging after configuration, would hide configuration errors from the log.

The telemetry crate adds Sentry on top of tracing. The Sentry tracing layers are installed before `main_inner` runs (`runtime/src/traces.rs:59-63`). The client is installed later, after configuration load (`main.rs:46`). An event before that call has no client to receive it. A worker thread that used the Hub before this call keeps an empty Hub (section 1). A configuration error returns before the call, so Sentry does not capture it.

`rivet_telemetry::init` returns `None` when `telemetry.enabled` is false (`telemetry/src/lib.rs:7-10`). The Sentry scope receives three allowlisted tags: the database kind, the pubsub kind, and the cache driver (`telemetry/src/lib.rs:25-52`). The cache tag is `"in_memory"` when the cache is enabled and the driver is `CacheDriver::InMemory`. It is `"disabled"` when the cache is disabled (`telemetry/src/lib.rs:42-52`; `config/src/config/cache.rs:21-31`). The comment explains why the tags are allowlisted. A serialized configuration can contain secrets, and even a server URL can contain credentials. The function therefore does not attach the configuration.

The Sentry destination is not configurable. `SENTRY_URL` is a hard-coded data source name (DSN) string constant (`telemetry/src/lib.rs:4`). No configuration field changes it. `rivet_telemetry::init` passes that constant to `sentry::init` only when `config.telemetry.enabled` is true (`telemetry/src/lib.rs:9-19`). The source has no empty-value check, so this document makes no claim about an empty DSN. The flag gates only that call. The tracing layers were installed earlier by the runtime and stay active when the flag is false (`runtime/src/traces.rs:59-63`; `telemetry/src/lib.rs:7-10`). With no client, those layers route events to an empty Hub, and the events are dropped.

These signal paths are independent:

| Signal | Mechanism | Endpoint or destination | Source |
|---|---|---|---|
| Logs | `tracing-subscriber` with logfmt or GCP format | Standard output, or a file when `logs.redirect_logs_dir` is set (seven-day retention) | `runtime/src/traces.rs:36-110`, `engine/src/commands/start.rs:7-8`, `26-31` |
| Traces | OTel OTLP over gRPC, enabled only when `RIVET_OTEL_ENABLED=1` | `RIVET_OTEL_GRPC_ENDPOINT`, default `http://localhost:4317` | `metrics-server/src/providers.rs:81-90`, `112-124` |
| Metrics | Prometheus text exposition | Metrics listener, default port 6430 | `metrics-server/src/server.rs:11-54` |
| Errors | Sentry | Hard-coded `SENTRY_URL` constant, not configurable; enabled by default, opt-out through `telemetry.enabled` | `telemetry/src/lib.rs:4-23` |
| CPU profiles | Pyroscope agent, optional | Configured `pyroscope.server_url` | `profiling/src/pyroscope.rs:12-79` |

Document [14 Observability](14-observability-and-operations.md) covers the metric names, the log fields, and the operational commands.

## 15. What to read next

- [04 Pegboard](04-pegboard-scheduling.md) explains the scheduler that the `workflow_worker` and `pegboard_outbound` services feed.
- [05 Gasoline](05-gasoline-durable-execution.md) explains the workflow engine behind `bootstrap`, the repair commands, and the worker drain.
- [09 Guard](09-guard-routing.md) explains how the external listener routes a request to an Actor and how the `api-public` router fits into that path.

## 16. Glossary for this document

| Term | Meaning in this document |
|---|---|
| control plane | The set of processes that schedule Actors and serve the public interface |
| Engine instance | One process of the `rivet-engine` binary, identified by a random `NodeId` |
| Datacenter | One entry in `topology.datacenters`, identified by a `datacenter_label`; also one Epoxy replica |
| Cluster | All datacenters in the topology, including the leader and the followers |
| Service | One `Service` value in the manager, with a name, a kind, a run closure, and a graceful flag |
| Service kind | The public label that selects a lifecycle behavior; only `Oneshot` and `Cron` change behavior |
| Pool | A shared handle in `Pools`, such as the UDB driver or the pubsub client |
| UniversalDB (UDB) | The durable key space under every durable record; PostgreSQL or RocksDB at this commit |
| UniversalPubSub (UPS) | The publish and subscribe layer; NATS or process memory at this commit |
| `api-public` | The external API router, hosted inside Guard; it owns cross-datacenter forwarding |
| `api-peer` | The network service on the API peer port; it serves local datacenter operations |
| Graceful shutdown | A drain that waits for in-flight work up to a configured budget |
| Bootstrap | The `Oneshot` service that creates first-run records and workflows |
| Guard | The external HTTP and WebSocket listener; it hosts the `api-public` router |
| Envoy | The Actor-to-engine tunnel; an Actor runtime connects to Guard at `/envoys/connect`, and `PegboardEnvoyWs` serves the tunnel |
| `x-rivet-target` | The HTTP request header that names a routing target; the WebSocket form is one `rivet_target.` prefix in `sec-websocket-protocol` |
| Actor | A keyed, stateful process that the control plane schedules |
| Namespace | A named container for Actors; the `Namespace` type stores `namespace_id` and `name` |
| Actor name | The `name` string on an `Actor` record |
| Actor key | The optional `key` string on an `Actor` record; together with the namespace and Actor name it identifies one Actor |
| Pegboard | The Actor scheduler; `pegboard_outbound` holds its serverless runner streams |
| Gasoline | The workflow engine; it dispatches, replays, and prunes workflows |
| Epoxy | The replicated key-value protocol across datacenters; one datacenter is one replica |
