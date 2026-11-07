---
title: Operations, testing, and conformance
---

# Operations, testing, and conformance

This document describes how the server starts, what it rebuilds from durable state, how it stops, which recovery paths exist, and how the test suite proves the behaviour.

## Terms and architecture

Three storage layers appear together. A reader must keep them apart.

- **PostgreSQL (Postgres)** holds the control state: the entity table, the permission tables, the wake registrations, the bridge rows, the scheduler rows, and the claim rows. The server reads and writes this state through Drizzle.
- **Durable Streams** holds the event log. Each entity owns a main stream at `/<entity path>/main`. A stream is append-only and has offsets. The stream server is a separate HTTP (Hypertext Transfer Protocol) service.
- **An Electric shape** is a live query result over one Postgres table, served by the Electric sync service at `/v1/shape`. A *shape stream* is an HTTP response that carries the rows and then the changes. The server uses shapes to learn about Postgres changes without polling.

`ElectricAgentsServer` also starts a *standalone runtime*. This is the object that wires the entity manager, the wake registry, the entity bridge manager, the Postgres sync manager, the tag outbox drainer, and the scheduler. The name means "one tenant, all parts in one process". It is not a separate deployment.

The *embedded streams server* is a `DurableStreamTestServer` from `@durable-streams/server`. It is an in-process stream server for development and tests. When `durableStreamsServer` is set and `durableStreamsUrl` is not, `start()` starts it and uses its origin. When an external Uniform Resource Locator (URL) is set, the server uses that URL and starts nothing.

An **entity** is one agent or agent-owned object, identified by a URL. A **principal** is an identity that acts on an entity. The principal kinds are `user`, `agent`, `service`, and `system`. A **tenant** is an isolation key. It selects Postgres rows and, through the routing adapter, may select a stream origin. See [08-tenancy-permissions-and-principals.md](08-tenancy-permissions-and-principals.md).

A **wake** is a notification that a stream source changed. A wake registration says who to notify, for which source, and under which conditions. See [05-wake-registry-and-scheduler.md](05-wake-registry-and-scheduler.md).

The word **claim** has five meanings in this package. The recovery table below uses all of them, so read this list first:

| Claim | Stored where | Who releases it |
|-------|--------------|-----------------|
| Runner claim | The stream server lease, with `claimToken` and `lease_ttl_ms` | The runner sends `done`, `stop`, or the lease expires |
| Scheduler task claim | `scheduled_tasks.claimed_by` and `claimed_at` | The scheduler deletes or reschedules the task; otherwise the claim expires after 30,000 ms |
| Tag outbox claim | `tag_stream_outbox.claimed_by` and `claimed_at` | The drainer deletes the row on success, or clears the claim on failure; a stale claim is older than 30,000 ms |
| Dispatch claim | `consumer_claims`, one row for a wake delivery | No code in this package marks a row `expired`; see the gap below |
| Claim token | The value in a claim response | The runner sends it with each append, and it ends at `done` or `stop` |

An **epoch** is a producer generation number. The Durable Streams protocol uses it to fence out an older writer. A producer triple is `(producerId, epoch, seq)`. A **manifest** is an event in an entity stream that lists the sources, schedules, and children that the entity owns. The *manifest key* is the key of that event. A **manifest source** is one entry with `kind: 'source'` and `sourceType: 'entities'`.

## Start-up

`ElectricAgentsServer.start()` runs a fixed sequence. A failure in any step calls `stop()` before it rethrows.

| Step | Action |
|------|--------|
| 1 | Start the embedded `DurableStreamTestServer` when no external streams URL is set |
| 2 | Create the HTTP (Hypertext Transfer Protocol) `undici` connection-pool agent with keep-alive |
| 3 | Run the Drizzle migrations. Drizzle is the TypeScript object-relational mapper that defines the schema |
| 4 | Create the PostgreSQL (Postgres) pool and the Drizzle database object |
| 5 | Start the standalone runtime |
| 6 | Ensure the principal entity type |
| 7 | Listen on the host and port |
| 8 | Register the mock agent, when configured |

A new server always runs the migrations before it serves traffic. `runMigrations` opens its own client with `max: 1`, applies the Drizzle journal, and calls `end()`. Only after that does `createDb` open the pool that the server keeps:

```ts
// packages/agents-server/src/db/index.ts:43-53
const migrationClient = postgres(postgresUrl, {
  max: 1,
  onnotice: () => {},
})
const db = drizzle(migrationClient)
await migrate(db, {
  migrationsFolder: resolveMigrationsFolder(),
})
await migrationClient.end()
```

The two-client design prevents a half-migrated pool. A failure in `migrate` rejects `start()` before any listener exists. `start()` tracks each started part in a field (`this.server`, `this.standaloneRuntime`, and so on). Its `catch` calls `stop()`, and `stop()` checks each field before it acts. A partial start therefore releases exactly the parts that started.

See [01-boot-and-configuration.md](01-boot-and-configuration.md) for the environment variables.

### What the runtime rebuilds

The runtime treats Postgres and the streams as the source of truth. It rebuilds three derived structures at start:

1. The wake registry. `rebuildWakeRegistry` starts one shape stream over `wake_registrations` when an Electric URL is present. `startSync` resolves a readiness promise only after the first `up-to-date`, and a second call awaits that same promise. Without an Electric URL, the method calls `loadRegistrations()` and reads the table once. That path has no live updates: a registration written by another process is invisible until a restart.
2. The cron streams. `rehydrateCronSchedules` finds wake registrations whose source starts with `/_cron/` and recreates each cron stream.
3. The manifest indexes. The same method clears `entity_manifest_sources` and replays each entity's main stream.

The wake shape uses the Electric client's `ShapeStream` with `table: wake_registrations` and a `where` clause on the tenant. The client sends `up-to-date` and `must-refetch` as in-band control messages on that same response. The registry caches rows in memory and never queries Postgres on the delivery path.

The order matters. The bridge manager starts after the cron and manifest rebuild, because the bridges read the same tables.

```ts
// packages/agents-server/src/standalone-runtime.ts:151-175
if (startWakeRegistry) {
  serverLog.info(`[agent-server] rebuilding wake registry...`)
  await runtime.manager.rebuildWakeRegistry(
    options.electricUrl,
    options.electricSecret
  )
}
if (rehydrateOnStart) {
  serverLog.info(`[agent-server] rehydrating cron schedules...`)
  await runtime.rehydrateCronSchedules()
}
if (startEntityBridgeManager) {
  serverLog.info(`[agent-server] starting entity bridge manager...`)
  await entityBridgeManager.start()
  entityBridgeManagerStarted = true
}
if (startPgSyncBridgeManager) {
  serverLog.info(`[agent-server] starting pg-sync bridge manager...`)
  await runtime.pgSyncBridgeManager.start?.()
}
if (startTagStreamOutboxDrainer) {
  serverLog.info(`[agent-server] starting tag stream outbox drainer...`)
  tagStreamOutboxDrainer.start()
  tagStreamOutboxDrainerStarted = true
}
```

The scheduler starts last. Its cron tick can deliver a wake only after the wake registry and the entity bridges are ready. `Scheduler.start()` issues `LISTEN scheduled_tasks_wake` on the Postgres client. `LISTEN`/`NOTIFY` is the Postgres publish-and-subscribe mechanism: another process that inserts a task calls `NOTIFY`, and every listener wakes. The scheduler also polls every 10,000 ms, because a `NOTIFY` can be lost if no listener is connected.

```mermaid
sequenceDiagram
    participant C as Caller
    participant S as ElectricAgentsServer
    participant P as Postgres
    participant D as Durable Streams
    participant R as Standalone runtime

    C->>S: start()
    S->>D: start embedded server, when configured
    S->>P: runMigrations
    S->>P: createDb
    S->>R: startStandaloneAgentsRuntime
    R->>P: read wake_registrations
    R->>D: subscribe shape and rebuild wake registry
    R->>P: read cron wake registrations
    R->>D: recreate cron streams
    R->>P: clear entity_manifest_sources
    R->>D: read each entity main stream
    R->>P: restore manifest indexes
    R->>D: start entity bridges
    R->>D: start pg-sync bridges
    R->>P: start tag outbox drainer
    R->>P: start scheduler with LISTEN
    S-->>C: base URL
```

### Manifest replay

The runtime does not trust the index tables. It rebuilds them from the streams:

```ts
// packages/agents-server/src/runtime.ts:148-186
const { entities } = await this.manager.registry.listEntities({
  limit: 10_000,
})
await this.manager.registry.clearEntityManifestSources()

for (const entity of entities) {
  try {
    const events = await this.streamClient.readJson<
      Record<string, unknown>
    >(entity.streams.main)
    const manifestEvents = new Map<string, Record<string, unknown>>()

    for (const event of events) {
      if (event.type !== `manifest` || typeof event.key !== `string`) {
        continue
      }
      manifestEvents.set(event.key, event)
    }

    for (const [manifestKey, event] of manifestEvents) {
      const headers = event.headers as Record<string, unknown> | undefined
      const operation = headers?.operation as string | undefined
      const value = event.value as Record<string, unknown> | undefined
      await this.applyManifestEntitySource(
        entity.url,
        manifestKey,
        operation,
        value
      )
```

The replay keeps only the last `manifest` event per `key`. That is the same last-writer rule that the live path uses. A stream with ten manifest versions produces at most one index row. The `key` is the manifest key, which is the stable name of one entry inside the entity's manifest, for example one source or one schedule. When the last event has operation `delete`, `applyManifestEntitySource` receives `undefined` and removes the row.

**Key insight: the rebuild limit is 10,000 entities.** The call has a fixed `limit: 10_000` and no paging loop. A deployment with more entities rebuilds only the first page. The entities beyond that page keep no manifest index row after a restart. If a live manifest event later arrives for such an entity, the live path writes the row. Until then, its bridges are unprotected from the idle sweep and its schedules are not restored. The source does not document this as a limit. Treat it as an observed bound.

## Shutdown

`stop()` sets a flag first, then tears down in this order:

1. Stop the HTTP listener and close idle and active connections.
2. Abort the mock agent wakes and drain them, with a 5,000 ms cap.
3. Stop the standalone runtime.
4. Stop an embedded streams server, when this process owns it.
5. End the Postgres pool.
6. Close the HTTP agent.

The standalone runtime stops its own parts in this order, which is the reverse of its start order: scheduler, tag outbox drainer, entity bridge manager, then the runtime. The runtime stops the entity manager and the pg-sync bridge manager in parallel. The entity manager is the object that owns the wake registry and evaluates wakes. Its shutdown first stops the wake registry sync, then closes the registry. The registry is the Postgres data-access layer for entities, permissions, bridges, and the outbox.

```mermaid
flowchart TB
    A["stop() called"] --> B["Set shuttingDown flag"]
    B --> C["Close HTTP listener"]
    C --> D["Abort and drain mock wakes, 5 s cap"]
    D --> E["Stop standalone runtime"]
    E --> F["Stop scheduler"]
    F --> G["Stop tag outbox drainer and release claims"]
    G --> H["Stop entity bridge manager"]
    H --> I["Stop entity manager and pg-sync manager in parallel;<br/>entity manager stops the wake registry sync"]
    I --> J["Stop embedded streams server, when owned"]
    J --> L["End Postgres pool"]
    L --> M["Close HTTP agent"]
```

On a graceful stop, the tag outbox drainer releases its claimed rows. The guarantee covers `stop()` only. A killed process leaves the rows claimed until the 30,000 ms stale window passes. A stopped process cannot strand rows in a claimed state:

```ts
// packages/agents-server/src/tag-stream-outbox-drainer.ts:55-66
async stop(): Promise<void> {
  this.stopping = true
  if (this.timer) {
    clearInterval(this.timer)
    this.timer = null
  }
  try {
    await this.activeDrain
  } finally {
    await this.registry.releaseTagOutboxClaims(this.workerId, this.tenantId)
  }
}
```

`scheduler.stop()` sets `running` to false, wakes the loop, waits for the loop promise, and unlistens the Postgres `LISTEN`. The wake registry `stopSync()` aborts the shape request and clears the ready promise. It does not delete the subscriptions; they live in the streams server.

### Refusal during shutdown

Only one route family refuses requests during shutdown. `rejectIfShuttingDown` returns `503 SERVER_STOPPING` (Service Unavailable) for `/_electric/subscription-webhooks/` paths. Other routes keep serving until the listener closes.

The rationale is delivery semantics. A webhook sender treats `503` as a retryable failure and sends again. If the process accepted the delivery and then stopped before it forwarded the event, the wake would be lost. The refusal moves the event back to the sender. See [06-http-routing.md](06-http-routing.md).

### Signals and exit codes

The entry point handles four process events:

```ts
// packages/agents-server/src/entrypoint-lib.ts:267-283
process.on(`SIGINT`, () => {
  void stop(0)
})

process.on(`SIGTERM`, () => {
  void stop(0)
})

process.on(`uncaughtException`, (error) => {
  console.error(error)
  void stop(1)
})

process.on(`unhandledRejection`, (error) => {
  console.error(error)
  void stop(1)
})
```

`SIGINT` (signal interrupt) and `SIGTERM` (signal terminate) exit with code 0. An uncaught exception and an unhandled rejection exit with code 1. The `stop` function is idempotent: concurrent signals share one promise.

A clean stop gives no guarantee that a runner has finished. The runner's claim is a lease on the work stream, and the upstream stream server expires it. The `consumer_claims` row that records the dispatch can remain `active` in Postgres, because nothing in this package writes `expired`. A later claim uses a new epoch, so it inserts a new row instead of reusing the stale one.

## Recovery paths that exist

The system has no single recovery worker. It has several small, local recovery rules:

| Failure | Recovery |
|---------|----------|
| Wake shape stream error | `recoverSync` stops the shape, reloads the registrations, and restarts it; it has no backoff |
| Entity bridge shape reset | Bootstrap from `-1`, rebuild the member set |
| Pg-sync shape reset | Restart at `now`; do not replay the table |
| Pg-sync subscription error | Retry with a 1,000 ms base that doubles to a 30,000 ms cap, plus up to 20 percent jitter |
| Scheduler process death | Another process reclaims the task after the 30,000 ms claim expiry |
| Runner loss | The claim lease expires upstream; the health route reports liveness `expired` after the heartbeat lease passes |
| Tag outbox failure | Retry up to 10 times, then dead-letter |
| Duplicate spawn | Postgres error 23505 maps to `EntityAlreadyExistsError` |
| Server restart | Rebuild the wake registry, cron streams, and manifest indexes |

Each rule is in the component that owns the data. The server does not scan the whole database.

Some terms in the table need a definition:

- An offset of `-1` means "from the start of the shape log". An offset of `now` means "from the current tail". Both are sentinel values in the Electric shape protocol.
- The jitter is `base + floor(base * 0.2 * random())`. At the 30,000 ms cap, the delay is between 30,000 and 36,000 ms.
- "Wake shape stream error" recovers with no backoff. `recoverSync` stops the shape, reloads every registration, and starts the shape again. A source that keeps failing can therefore produce a tight loop of shape requests. This is an observed risk, not a designed limit.
- The scheduler claim (`scheduled_tasks.claimed_by`) is not the runner claim and not the dispatch claim. The 30,000 ms expiry in this row is the `claimExpiryMs` option, which defaults to 30,000. The scheduler does not renew an in-flight task, so a task that runs longer than 30,000 ms becomes reclaimable.
- "Liveness" in the runner row is the heartbeat timestamp compared with the runner's liveness lease, which defaults to 30,000 ms. It is not the upstream stream lease.
- A dead-letter row keeps its data and stops retrying. The drainer never scans it again.

## The dispatch-recovery gap

A *dispatch claim* is a row in `consumer_claims` that says one consumer is delivering one wake. Its key is `(tenant_id, consumer_id, epoch)`. An *outstanding wake* is a `wake_notifications` row that has a wake but no confirmed completion. The package has no loop that sweeps either table.

The server options declare two recovery settings:

```ts
// packages/agents-server/src/server.ts:77-86
/**
 * Disabled by default. When set to a positive interval, periodically
 * recovers expired dispatch claims and stale outstanding wakes.
 */
dispatchRecoveryIntervalMs?: number
/**
 * Age threshold for outstanding wakes recovered by the periodic loop.
 * Defaults to dispatchRecoveryIntervalMs when periodic recovery is enabled.
 */
staleOutstandingWakeAfterMs?: number
```

The entry point reads `ELECTRIC_AGENTS_DISPATCH_RECOVERY_INTERVAL_MS` and `ELECTRIC_AGENTS_STALE_OUTSTANDING_WAKE_AFTER_MS` into these fields. But no code in this package reads the fields after that. `startStandaloneAgentsRuntime` does not receive them, and a repository search finds no consumer. Two related facts support this:

- `consumer_claims` allows the status `expired`, but no statement in `src/` writes it.
- `wake_notifications` allows the claim status `expired`, but no statement in `src/` writes it.

So the periodic recovery loop described by the comment is not present in version 0.6.4. The safest statement is this. **Expired claims are recovered by the stream server lease and by a new claim, not by a Postgres sweep.** A claim row that is never released stays `active` in Postgres. A later claim with a new epoch inserts a new row, because the primary key is `(tenant_id, consumer_id, epoch)`; it does not overwrite the stale row. Treat the two environment variables as unused. This is an open question for the reader to confirm against a later version.

## Logging and tracing

Logging uses `pino`, a structured logger for Node.js. The logger writes two streams when both are enabled:

- A file at `<ELECTRIC_AGENTS_LOG_DIR or ./logs>/agent-server-<timestamp>.jsonl`. The suffix `.jsonl` means JavaScript Object Notation (JSON) Lines: one JSON record on each line.
- Pretty output on the console, unless the variable `VITEST` is set or Node.js reports an Electron version.

The `VITEST` variable is set by the test runner, so a test run does not colour its output. The Electron check is `process.versions.electron`, and the source names it `IS_ELECTRON_MAIN`. `ELECTRIC_AGENTS_LOG_FILE` disables the file only when its value is exactly the string `false`; any other value, including an empty string, keeps the file on. `ELECTRIC_AGENTS_LOG_LEVEL` sets the level and defaults to `info`. The level `silent` suppresses every record, but the file is still created and stays empty.

Tracing uses the OpenTelemetry application programming interface (API). The tracer is named `agent-server`. Spans cover:

- `stream.create`, `stream.fork`, `stream.append`, and the other stream client methods
- `electric_agents.spawn`, `electric_agents.forkSubtree`, `electric_agents.evaluateWakes`, and `electric_agents.deliverWake`
- `db.createEntity`

The package depends on the OpenTelemetry API only. It has no software development kit and no exporter. A host process must install and start an OpenTelemetry software development kit for spans to leave the process. `injectTraceHeaders` calls the World Wide Web Consortium (W3C) `propagation.inject`, which writes the `traceparent` header, and outbound stream calls carry that header. An exporter in the host process can then connect the stream server span to the caller span.

## Testing

The tests use real components:

- A Postgres database in Docker
- An Electric shape endpoint in Docker
- A `DurableStreamTestServer` on an ephemeral port
- An `ElectricAgentsServer` on an ephemeral port

No database or stream is mocked in the route tests and the lifecycle tests. This is why the suite finds ordering bugs between the stream and the control tables. A test that asserted the Postgres row alone would miss a missing stream append.

Run the suite with `pnpm test` from the package directory. Run one file with `pnpm exec vitest run test/runners-router.test.ts`. Docker must be running. The first run pulls three images and takes several minutes.

### Serial execution

The suite runs one test file at a time:

```ts
// packages/agents-server/vitest.config.ts:4-6
test: {
  fileParallelism: false,
  maxWorkers: 1,
```

Every file shares one Postgres schema and one stream server port range. A parallel run would reset the schema under another test. The cost is a long run. The benefit is a deterministic shared state.

### The test backend

`test/test-backend.ts` owns the Docker Compose stack. The file is `docker-compose.dev.yml`, and it starts three services:

| Service | Image | Purpose |
|---------|-------|---------|
| `postgres` | `postgres:18` | Control tables, with `wal_level=logical` |
| `electric` | `electricsql/electric:latest` | Shape endpoint |
| `jaeger` | `jaegertracing/all-in-one:latest` | Trace collection, for a host that sends spans |

This package depends only on the OpenTelemetry API. It ships no software development kit and no exporter, so the test suite does not send spans to Jaeger. The service is present so a host process that installs an exporter can point it at the stack.

`ensureElectricAgentsTestBackend` tries `select 1` on `TEST_POSTGRES_URL`. When the connection fails and no explicit backend is configured, it runs `docker compose up -d --wait`. The compose file path is `../docker-compose.dev.yml` relative to `test/`.

The connection targets come from these variables:

| Variable | Default |
|----------|---------|
| `DATABASE_URL` | `postgres://electric_agents:electric_agents@localhost:5432/electric_agents` |
| `ELECTRIC_URL` | `http://localhost:3060` |
| `CLICKHOUSE_URL` | unset |

An explicit backend means `ELECTRIC_AGENTS_TEST_BACKEND_MANAGED` is not the string `1` and any of those three variables is set. In that mode the suite uses the given services and never runs Compose.

The compose project name includes the worktree hash, so two checkouts do not collide. Host ports derive from the same hash:

| Service | Base port | Span | Override |
|---------|-----------|------|----------|
| Electric Agents | 4400 | 600 | `ELECTRIC_AGENTS_PORT` |
| Postgres | 55000 | 700 | `PG_HOST_PORT` |
| Electric | 56400 | 700 | `ELECTRIC_HOST_PORT` |
| Jaeger user interface | 57100 | 700 | `JAEGER_UI_PORT` |
| Jaeger OpenTelemetry Protocol HTTP | 57800 | 700 | `JAEGER_OTLP_HTTP_PORT` |
| Jaeger OpenTelemetry Protocol gRPC | 58500 | 700 | `JAEGER_OTLP_GRPC_PORT` |

The Jaeger ports accept 0, which asks the operating system to choose a free port. The others reject 0.

A reset drops the `public` and `drizzle` schemas, recreates `public`, and reruns the migrations. Dropping `drizzle` also removes the migration journal, so `runMigrations` applies every migration from the first one:

```ts
// packages/agents-server/test/test-backend.ts:228-232
await pg.unsafe(`
  DROP SCHEMA IF EXISTS drizzle CASCADE;
  DROP SCHEMA IF EXISTS public CASCADE;
  CREATE SCHEMA public AUTHORIZATION CURRENT_USER;
`)
```

A lock directory serializes resets across processes. Its path is `os.tmpdir()/<compose project>-agent-server-test-backend.lock`. A `mkdir` that fails with code `EEXIST` means another process holds the lock, so the caller waits 100 ms and tries again. The wait has no retry limit and no timeout, so a lock that a dead process never releases blocks the run. Delete that directory to clear a stale lock. Do not delete it while another test run is active.

### Schema freshness

The backend must match the current migration set. `ensureExpectedSchema` checks ten markers:

- `entities.tags`
- `entities.tags_index`
- `tag_stream_outbox.dead_lettered_at`
- `entity_manifest_sources`
- `entities.created_by`
- `entity_effective_permissions`
- `shared_state_links`
- `entity_bridges.principal_url`
- the absence of the legacy `entities.metadata`
- `entity_types.externally_writable_collections`

When the markers do not match and `ELECTRIC_AGENTS_TEST_BACKEND_MANAGED` is exactly the string `1`, the code stops the stack, removes volumes, restarts, and runs the migrations again. When the variable has any other value, including `true` or `0`, the code does not reset and throws at once. A second mismatch after the reset also throws, with the exact reset command. The failure is loud by design. A stale schema would make a test pass for the wrong reason.

### Global teardown

`test/global-setup.ts` returns a teardown function. It runs `docker compose down -v` after the last test file. `ELECTRIC_AGENTS_KEEP_BACKEND=1` skips the teardown, so a developer can iterate without a cold start.

The coverage script uses that flag around the main run, then runs the conformance file in a second process. The first command excludes `conformance.test.ts`; the second runs only that file. Two processes give the conformance suite a fresh module state, and `KEEP_BACKEND=1` keeps the Docker services alive between them:

```json
// packages/agents-server/package.json:42
    "coverage": "ELECTRIC_AGENTS_KEEP_BACKEND=1 pnpm exec vitest run --coverage $(find test -name '*.test.ts' ! -name 'conformance.test.ts' | sort) && pnpm exec vitest run test/conformance.test.ts",
```

### Conformance

`test/conformance.test.ts` runs four suites from the shared `agents-server-conformance-tests` package. Each suite starts its own `DurableStreamTestServer` and `ElectricAgentsServer`, and resets the backend first.

| Suite | Function | What it proves |
|-------|----------|----------------|
| Electric Agents | `runElectricAgentsConformanceTests` | The entity protocol |
| Command line interface | `runCliConformanceTests` | The command line client |
| Mock agent | `runMockAgentTests` | The agent handler and wake pipeline |
| Command line interface with mock agent | `runMockAgentCliTests` | Both layers together |

The suite package has 28 `describe` groups and one further `describe.skip` group. Across all groups are 111 `test` declarations: 97 ordinary declarations and 14 `test.skip` declarations. Three of the ordinary declarations are inside the skipped group, so at most 94 cases can run. The active groups cover spawn, send, list, kill, entity type registration, typed spawn, schema validation gates, and schema evolution. They also cover the serve endpoint, random action sequences, tools, state protocol formats, error paths, type revisions, concurrency, and the auth boundary. The coverage also includes eight command line groups and the mock agent end to end.

The conformance suite is a specification, not a smoke test. The same package can run against any server that claims the Electric Agents protocol. That is why it lives in its own workspace package.

The command line suite is conditional. The test computes the path to `electric-agents-cli/src/index.ts` and uses `describe.skip` when the file is absent. A checkout without the command line package still passes.

```mermaid
flowchart LR
    A["vitest run"] --> B["global setup"]
    B --> C["ensure test backend"]
    C --> D{"Postgres reachable?"}
    D -->|no| E["docker compose up -d --wait"]
    D -->|yes| F["use it"]
    E --> F
    F --> G["reset backend, serialized by lock"]
    G --> H["run one test file"]
    H --> I["reset backend for the next file"]
    I --> J["global teardown: compose down -v"]
```

**Aha: the test backend is a shared, mutable system.** Correctness therefore depends on serialization. The reset lock, `maxWorkers: 1`, and the schema check all defend the same property: one schema, one stream, one test at a time. Without them, a parallel suite would create false failures and false passes.

## Behavioural tests

Some routes are proven by direct tests against the server:

- `runners-router.test.ts` covers owner checks, claims, heartbeats, and health.
- `wake-registry.test.ts` is 1,602 lines and covers registration, conditions, and delivery.
- `electric-agents-routes.test.ts` is 1,620 lines, the largest test file, and covers the entity routes.
- `pg-sync-bridge-manager.test.ts` covers the probe, the retry, and `must-refetch`. The probe is one shape page fetched at registration to prove the source is a shape endpoint. `must-refetch` is the control message that tells a shape client its cursor is invalid.
- `entity-bridge-manager.test.ts` covers the bootstrap and the membership events.
- `server-claim-write-token.test.ts` covers the claim token lifecycle: the runner claims, receives a `claimToken`, then receives a write token on the wake callback, and loses both at `done` or `stop`.
- `scheduler-integration.test.ts` starts a real server and a real scheduler.

A test usually starts a server, calls the route, then reads the stream to confirm the side effect. That pattern matches the product rule: the stream is the record.

## Related documents

- [01-boot-and-configuration.md](01-boot-and-configuration.md) lists every environment variable and the start and stop methods.
- [05-wake-registry-and-scheduler.md](05-wake-registry-and-scheduler.md) explains the recovery of the wake sync and the scheduler claim expiry.
- [06-http-routing.md](06-http-routing.md) lists the health route and the shutdown guard.
- [10-pg-sync-and-bridges.md](10-pg-sync-and-bridges.md) explains the bridge bootstrap and retry rules.
