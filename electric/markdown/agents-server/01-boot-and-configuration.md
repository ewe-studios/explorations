---
title: Boot and configuration
---

# Boot and configuration

This document traces the executable from process start to a listening HTTP (Hypertext Transfer Protocol) server. It also lists every environment variable that the package reads, with the default value for each one.

## The executable entry point

Two files make the command-line entry. `src/entrypoint.ts` is a thin gate. It runs `main()` only when Node.js started this file directly. An import from a test or another module does nothing.

```ts
// packages/agents-server/src/entrypoint.ts:6-11
if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  void main()
}
```

The package declares two binary names. Both point to the same built file:

```json
// packages/agents-server/package.json:6-9
"bin": {
  "electric-agents-server": "./dist/entrypoint.js",
  "durable-streams-electric-agents-server": "./dist/entrypoint.js"
}
```

`main()` in `src/entrypoint-lib.ts` does five things in order:

1. It calls `process.loadEnvFile()` inside a `try` block. A missing `.env` file is not an error. `process.loadEnvFile` exists from Node.js 20.12. The package declares `node >=18`, so the stated engine range and this call do not agree. The source has no guard or fallback.
2. It builds the server options from the environment.
3. It starts the server and prints the bound Uniform Resource Locator (URL).
4. It installs handlers for `SIGINT`, `SIGTERM`, `uncaughtException`, and `unhandledRejection`.
5. It exits with code 1 when start-up fails.

The stop function is idempotent. A second signal waits on the same promise instead of starting a second shutdown.

```ts
// packages/agents-server/src/entrypoint-lib.ts:241-251
const stop = async (exitCode: number): Promise<never> => {
  if (!stopping) {
    stopping =
      server?.stop().catch((error) => {
        console.error(`[agent-server] failed to stop cleanly`, error)
      }) ?? Promise.resolve()
  }

  await stopping
  process.exit(exitCode)
}
```

## Environment variables

The resolver is `resolveElectricAgentsEntrypointOptions` at `src/entrypoint-lib.ts:115`. Most settings accept two names. Some accept three or four. The `ELECTRIC_AGENTS_*` name wins over the short name.

| Setting | Variable names | Default | Notes |
|---------|----------------|---------|-------|
| Tenant or service identifier | `ELECTRIC_AGENTS_SERVICE`, `SERVICE`, then `ELECTRIC_AGENTS_TENANT_ID`, `TENANT_ID` | `default` | The service name wins over the tenant name. It is the Postgres tenant key and is passed to stream routing adapters |
| Public base URL (Uniform Resource Locator) | `ELECTRIC_AGENTS_BASE_URL`, `BASE_URL` | none | Must parse as a URL. It is used for callback and webhook URLs |
| Durable Streams URL | `ELECTRIC_AGENTS_DURABLE_STREAMS_URL`, `DURABLE_STREAMS_URL`, `STREAMS_URL` | none | This must be a backend prefix, not a bare origin. When absent, the process starts an embedded server |
| Durable Streams bearer token | `ELECTRIC_AGENTS_DURABLE_STREAMS_BEARER`, `DURABLE_STREAMS_BEARER` | none | The client adds the `Bearer ` prefix when it is absent |
| Postgres URL | `ELECTRIC_AGENTS_DATABASE_URL`, `DATABASE_URL` | **required** | Start-up fails when neither name is set |
| Electric URL | `ELECTRIC_AGENTS_ELECTRIC_URL`, `ELECTRIC_URL` | none | Needed for shape sync, projection, and bridges |
| Electric secret | `ELECTRIC_AGENTS_ELECTRIC_SECRET` | none | Passed to shape requests as `secret` |
| Webhook signing private key | `ELECTRIC_AGENTS_WEBHOOK_SIGNING_PRIVATE_KEY`, `WEBHOOK_SIGNING_PRIVATE_KEY` | generated at start | An Ed25519 key in PEM (Privacy-Enhanced Mail), JSON (JavaScript Object Notation) Web Key, buffer, or key-object form. An invalid or non-Ed25519 key throws at start |
| HTTP host | `ELECTRIC_AGENTS_HOST`, `HOST` | `0.0.0.0` | The entry point sets `0.0.0.0`. If the option is absent when the class is constructed directly, the class uses `127.0.0.1` |
| HTTP port | `ELECTRIC_AGENTS_PORT`, `PORT` | `4437` | Must be an integer from 1 to 65,535. A bad value throws at start |
| Working directory | `ELECTRIC_AGENTS_WORKING_DIRECTORY`, `WORKING_DIRECTORY` | `process.cwd()` | Base for the `.streams-data` directory |
| Dispatch recovery interval | `ELECTRIC_AGENTS_DISPATCH_RECOVERY_INTERVAL_MS` | none | Positive integer; read into the server options. No code reads the option in version 0.6.4. See [11](11-operations-testing-and-conformance.md) |
| Stale wake age | `ELECTRIC_AGENTS_STALE_OUTSTANDING_WAKE_AFTER_MS` | none | Positive integer; read into the server options. No code reads the option in version 0.6.4 |
| Embedded streams data directory | `ELECTRIC_AGENTS_STREAMS_DATA_DIR`, `STREAMS_DATA_DIR` | `<cwd>/.streams-data` | Used only when the embedded server starts |
| Embedded streams host | `ELECTRIC_AGENTS_STREAMS_HOST`, `STREAMS_HOST` | `127.0.0.1` | Used only when the embedded server starts |
| Embedded streams port | `ELECTRIC_AGENTS_STREAMS_PORT`, `STREAMS_PORT` | `0` | `0` means an ephemeral port |

Other variables are read outside the entrypoint resolver:

| Variable | Default | Effect |
|----------|---------|--------|
| `ELECTRIC_AGENTS_PG_POOL_MAX` | `100` | Maximum Postgres connections in the shared pool |
| `ELECTRIC_AGENTS_SPAWN_CONCURRENCY` | `16` | Parallelism of the spawn persistence queue |
| `ELECTRIC_AGENTS_MAX_ATTACHMENT_BYTES` | `26,214,400` (25 mebibytes) | Largest attachment the server accepts |
| `ELECTRIC_AGENTS_REWRITE_LOOPBACK_WEBHOOKS_TO` | none | Rewrites `localhost`, `127.0.0.1`, and `::1` webhook hosts. Docker Compose sets this to `host.docker.internal` |
| `ELECTRIC_AGENTS_LOG_LEVEL` | `info` | Log level for the Pino logger |
| `ELECTRIC_AGENTS_LOG_FILE` | enabled | The file is disabled only when the value is exactly `false`; any other value keeps it on |
| `ELECTRIC_AGENTS_LOG_DIR` | `<cwd>/logs` | Directory for the JSON Lines log file |
| `ELECTRIC_INSECURE` | none | `true` enables the development principal fallback |
| `NODE_ENV` | none | A value other than `production` also enables the development principal fallback |

The development principal fallback means the server accepts a request with no principal and uses the built-in `system:dev-local` principal. The fallback is on when `NODE_ENV` is not `production`, and an unset `NODE_ENV` is not `production`. It is also on when the process owns the embedded streams server, even with `NODE_ENV=production`, because that deployment has no separate authentication boundary. A production deployment must set `NODE_ENV=production`, leave `ELECTRIC_INSECURE` unset or `false`, and use an external streams server to turn the fallback off. See [08-tenancy-permissions-and-principals.md](08-tenancy-permissions-and-principals.md) for the three-valued rule.

```ts
// packages/agents-server/src/entrypoint-lib.ts:194-210
const dataDir =
  readEnv(env, [`ELECTRIC_AGENTS_STREAMS_DATA_DIR`, `STREAMS_DATA_DIR`]) ??
  `${cwd}/.streams-data`

return new DurableStreamTestServer({
  host:
    readEnv(env, [`ELECTRIC_AGENTS_STREAMS_HOST`, `STREAMS_HOST`]) ??
    DEFAULT_STREAMS_HOST,
  port:
    readOptionalPort(
      env,
      [`ELECTRIC_AGENTS_STREAMS_PORT`, `STREAMS_PORT`],
      `embedded streams port`
    ) ?? 0,
  dataDir,
  webhooks: true,
})
```

## The embedded Durable Streams server

The package depends on `@durable-streams/server`. That package exports `DurableStreamTestServer`. Despite the name, the entrypoint uses it as a real local server when no external URL is set.

The embedded server is enabled by *absence*. `createEmbeddedStreamsServer` returns `undefined` when `ELECTRIC_AGENTS_DURABLE_STREAMS_URL`, `DURABLE_STREAMS_URL`, or `STREAMS_URL` has a value.

The embedded server returns an HTTP origin. Its stream backend is mounted under `/v1/stream`. The server class converts the origin to a backend prefix:

```mermaid
flowchart TB
    A["Read ELECTRIC_AGENTS_DURABLE_STREAMS_URL,<br/>DURABLE_STREAMS_URL, STREAMS_URL"] --> B{"Any value set?"}
    B -->|yes| C["Use the value unchanged<br/>as the backend prefix"]
    B -->|no| D["Start DurableStreamTestServer"]
    D --> E["Origin: http://host:port"]
    E --> F["Append /v1/stream"]
    F --> G["Backend prefix"]
    C --> H["StreamClient"]
    G --> H
```

```ts
// packages/agents-server/src/server.ts:141-149
function durableStreamTestServerBackendUrl(origin: string): string {
  // DurableStreamTestServer.start() returns the HTTP origin, while the
  // reference server's stream backend is mounted under /v1/stream.
  // User-provided durableStreamsUrl values are already backend prefixes and
  // are passed through unchanged.
  const url = new URL(origin)
  url.pathname = `${url.pathname.replace(/\/+$/, ``)}/v1/stream`
  return url.toString().replace(/\/+$/, ``)
}
```

**Aha: the two URL forms are not interchangeable.** An embedded server returns `http://127.0.0.1:41234`. The client must call `http://127.0.0.1:41234/v1/stream/...`. A user-supplied `durableStreamsUrl` must already include the backend prefix. The server does not add it twice. The resolver checks only that the value parses as a URL; it does not check for `/v1/stream`. A bare origin passes start-up and fails at the first stream call.

## The start sequence

```mermaid
sequenceDiagram
    participant E as entrypoint
    participant S as ElectricAgentsServer
    participant DS as DurableStreamTestServer
    participant PG as Postgres
    participant RT as standalone runtime
    participant H as HTTP listener

    E->>E: loadEnvFile and read variables
    E->>S: new ElectricAgentsServer(options)
    E->>S: start()
    S->>DS: start() when embedded
    DS-->>S: origin URL
    S->>S: rewrite origin to /v1/stream
    S->>S: create HTTP client connection pool (undici Agent)
    S->>PG: runMigrations()
    S->>PG: createDb() with pool size
    S->>RT: startStandaloneAgentsRuntime()
    RT-->>S: manager and bridge manager
    S->>PG: ensurePrincipalEntityType()
    S->>H: createServer and listen
    H-->>E: bound URL
```

The order is deliberate:

- Migrations run before any registry query.
- The runtime and its background loops start before the HTTP listener accepts work.
- The principal entity type exists before the first request can create an entity.

The class stores the start options and mutates `durableStreamsUrl` after the embedded server starts. This mutation is safe because the stream client is constructed immediately after it.

The host default depends on the caller. The executable always sets a host, so the option is never absent and the default is `0.0.0.0`. A program that constructs `ElectricAgentsServer` directly and omits the host gets `127.0.0.1`, because `server.ts` supplies that value at listen time. A deployment that needs to accept connections from another machine must set `ELECTRIC_AGENTS_HOST` or pass the option.

The HTTP connection pool uses these values: keep-alive timeout 60,000 milliseconds (ms), maximum keep-alive 600,000 ms, 256 connections, pipelining 1, no body timeout, and no headers timeout. The constants are in `packages/agents-server/src/server.ts:224-231`. The keep-alive timeout is how long an idle connection waits for the next request. The maximum keep-alive caps that timeout, so a connection does not stay in the pool without bound. The long timeouts are necessary because Durable Streams reads can stay open for a long time.

## The stop sequence

`stop()` sets `shuttingDown` first. New subscription webhooks then receive `503 SERVER_STOPPING`. A subscription webhook is an inbound delivery that the Durable Streams server sends to this server. The remaining steps are:

1. Close the HTTP server and its idle and active connections.
2. Abort the mock agent wakes and wait at most 5,000 ms for them to drain. The mock agent is the built-in test handler that serves the conformance suite. A wake is one handler run that a stream event started.
3. Stop the standalone runtime. It stops the scheduler, then the tag outbox drainer, then the entity bridge manager. The runtime then stops the entity manager and the pg-sync bridge manager in parallel. The entity manager stops the wake registry sync and closes the registry.
4. Stop the embedded streams server, if the process owns it.
5. End the Postgres client.
6. Close the undici HTTP client `Agent`.

```ts
// packages/agents-server/src/server.ts:295-313
if (this.mockAgentBootstrap) {
  this.mockAgentBootstrap.runtime.abortWakes()
  await Promise.race([
    this.mockAgentBootstrap.runtime.drainWakes().catch(() => {}),
    new Promise<void>((resolve) => setTimeout(resolve, 5_000)),
  ])
  this.mockAgentBootstrap = undefined
}

if (this.standaloneRuntime) {
  await this.standaloneRuntime.stop()
  this.standaloneRuntime = undefined
  this.entityBridgeManager = undefined
  this.electricAgentsManager = undefined
}
```

The 5,000 ms cap prevents a stuck handler from blocking shutdown forever.

## Docker Compose

The package ships two Compose files.

### `docker-compose.dev.yml`

This file starts the dependencies only:

| Service | Image | Purpose | Default host port |
|---------|-------|---------|-------------------|
| `postgres` | `postgres:18` | Registry database | `5432` (`PG_HOST_PORT`) |
| `electric` | Electric SQL | Shape source | `3060` (`ELECTRIC_HOST_PORT`) |
| `jaeger` | Jaeger all-in-one | Trace user interface (UI) and OpenTelemetry collector | `16686`, `4318`, `4317` |

The Postgres service is the same one the `start` script expects. The script default is:

```json
// packages/agents-server/package.json:40
    "start": "cross-env-shell DATABASE_URL=${DATABASE_URL:-postgresql://electric_agents:electric_agents@localhost:5432/electric_agents} ELECTRIC_URL=${ELECTRIC_URL:-http://localhost:3060} \"tsx --watch src/entrypoint.ts\"",
```

### `docker-compose.full.yml`

This file runs the complete stack:

- `postgres:18-alpine` with `wal_level=logical`, `max_connections=300`, and `shared_buffers=256MB`.
- `electric` with `ELECTRIC_INSECURE='true'`.
- `electric-agents` from `electricax/agents-server:${ELECTRIC_AGENTS_SERVER_IMAGE_TAG:-latest}`.

The `electric-agents` service sets `ELECTRIC_AGENTS_HOST=0.0.0.0`, `ELECTRIC_AGENTS_PORT=4437`, `ELECTRIC_AGENTS_STREAMS_DATA_DIR=/var/lib/electric-agents/streams`, `ELECTRIC_AGENTS_STREAMS_HOST` (default `127.0.0.1`), `ELECTRIC_AGENTS_STREAMS_PORT` (default `0`), and `ELECTRIC_AGENTS_ELECTRIC_URL` (default `http://electric:3000`). It maps `${ELECTRIC_AGENTS_PORT:-4437}:4437`. It mounts the `electric-agents-postgres-data` and `electric-agents-streams-data` volumes.

The container must bind `0.0.0.0` to be reachable from outside. The *embedded streams server* stays on `127.0.0.1`, because only the server process calls it.

### The image

The `Dockerfile` uses a multi-stage build on `node:22-bookworm-slim`. It builds workspace dependencies in dependency order: `typescript-client`, `agents-mcp`, `agents-runtime`, `agents-server-ui`, then `agents-server`. It runs `pnpm deploy --legacy --prod`, copies the `drizzle` directory, exposes port 4437, and sets `ENTRYPOINT ["node", "dist/entrypoint.js"]`.

## Runtime requirements

| Requirement | Value |
|-------------|-------|
| Node.js | 18 or later |
| Module format | ECMAScript module (ESM) |
| Build tool | `tsdown` |
| Test runner | `vitest` |
| Development runner | `tsx` |

## Related documents

- [04-stream-layout-and-stream-client.md](04-stream-layout-and-stream-client.md) explains what the stream client does with the URL.
- [11-operations-testing-and-conformance.md](11-operations-testing-and-conformance.md) explains the test backend that uses the same environment names.
- [06-http-routing.md](06-http-routing.md) explains what happens after the listener binds.
