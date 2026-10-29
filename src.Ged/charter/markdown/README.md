# charter -- Index

A technical exploration of charter — contract-first APIs on Cloudflare Workers,
in Go or TypeScript. One contract becomes the OpenAPI and AsyncAPI specs, Fern
SDKs, MCP tools, authentication, rate limits, and streaming. Source:
`/Users/darkvoid/GitSources/src.Ged/charter/`.

- [00-overview.md](./00-overview.md) — What charter is, the problem it solves, and the contract-first promise.
- [01-architecture.md](./01-architecture.md) — The packages, the layers, and the dependency graph.
- [02-humaworkers.md](./02-humaworkers.md) — Huma on Workers: the API, the router, and form bodies.
- [03-serve.md](./03-serve.md) — The standard server and the problem-details document.
- [04-auth.md](./04-auth.md) — The authentication middleware and the ways a caller proves itself.
- [05-jwt-discovery.md](./05-jwt-discovery.md) — JWT verification, key caching, and the discovery documents.
- [06-mcp.md](./06-mcp.md) — The Model Context Protocol endpoint and how operations become tools.
- [07-ratelimit-problem.md](./07-ratelimit-problem.md) — Rate limiting and RFC 9457 problem details.
- [08-streaming.md](./08-streaming.md) — The follow loop, the hub, SSE fragments, and AsyncAPI channels.
- [09-transport.md](./09-transport.md) — The platform transport and the WebSocket adapter.
- [10-d1-promise-web.md](./10-d1-promise-web.md) — D1, the promise bridge, web helpers, and bindings.
- [11-project-configuration.md](./11-project-configuration.md) — charter.toml and the project model.
- [12-cli.md](./12-cli.md) — The command dispatcher and the command registry.
- [13-build-deploy-local.md](./13-build-deploy-local.md) — Building the Worker, deploying it, and serving locally.
- [14-spec-sdks-fern.md](./14-spec-sdks-fern.md) — Spec generation, database code, and the Fern SDKs.
- [15-check-test-twins.md](./15-check-test-twins.md) — The standard test, checks, and the twin comparison.
- [16-ci-scaffold-repo.md](./16-ci-scaffold-repo.md) — CI, scaffolding, and repository management.
- [17-typescript-library-examples.md](./17-typescript-library-examples.md) — charter-lib and the example APIs.
