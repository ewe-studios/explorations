---
title: Cloudflare Model Context Protocol Server — Source Exploration
---

# Cloudflare Model Context Protocol Server — Source Exploration

This set studies the official Cloudflare Model Context Protocol (MCP) server
for the Cloudflare Application Programming Interface (API) at
`/Users/darkvoid/GitSources/src.Cloudflare/mcp`, from
`git@github.com:cloudflare/mcp.git`. This document set lives under
`/Users/darkvoid/Dev/explorations/src.Cloudflare/mcp/markdown/`. The source path
is a read-only checkout. This study writes only under the study path. The main
question is how Code Mode works: how a model calls a large API by writing code
against a catalog that stays on the server.
The study is source-grounded: each factual claim cites a file and line in the
repository, written as `path:line` or `path:line-line`.

## Scope and sampling rule

The `src/generated` directory measures 53,784 KiB by the disk-usage command
`du -sk` in this checkout.
Here KiB means kibibyte (1,024 bytes), MiB means mebibyte (1,048,576 bytes),
and MB means megabyte (1,000,000 bytes). That value is 52.5 MiB, or about
55.1 MB. The generated tools account for 27,692 KiB (27.0 MiB). The spec
directory accounts for 26,092 KiB (25.5 MiB). The two values sum to the total.
The single `spec.json.txt` file inside the spec directory is 26,325,766 bytes
(25.1 MiB, or 26.3 MB). The directory is slightly larger than the file because
it also holds the other two artifacts and because of block allocation.

This study samples generated code, and does not enumerate the catalog. The
sample is one product group (`d1`), one shared schema file (`d1/outputs.ts`),
the forge runtime (`_/tool.ts`), the index, and one spec artifact
(`spec.json.txt`). Counts come from directory measurement and one parse of that
spec artifact. The generation pipeline is described from its script. A product
not sampled may differ in size, but the generated shape comes from one
generator. See
[13-limitations-and-open-questions.md](13-limitations-and-open-questions.md).

The study is read-only. It makes no network call, installs no package, and
changes no source file.

## Terms used across the set

| Term | Meaning |
| --- | --- |
| Code Mode | A tool design where the model writes code against a server-side catalog. The server does not give the model one tool for each endpoint. |
| Model Context Protocol (MCP) | The protocol that this server answers. |
| Worker | A deployed Cloudflare program that runs in an isolate. |
| isolate | A small JavaScript runtime with its own memory. |
| Worker Loader | The Cloudflare binding that builds dynamic Workers from a module map. |
| dynamic Worker | A Worker that the server builds at run time from a module map. |
| OpenAPI document | A machine-readable description of an HTTP API, including paths, methods, parameters, and schemas. |
| sandbox | The dynamic Worker that runs model-written code. |
| Forge | Cloudflare's code generator, installed as `@cloudflare/forge`. It turns an OpenAPI document into tool packages. |
| spec | The processed OpenAPI document that `search` reads. |
| tool | One callable function that an MCP client offers to a model. |
| Remote Procedure Call (RPC) | A call from one Worker to a method on another Worker. |
| `GlobalOutbound` | The Worker entrypoint that receives every outbound `fetch` from the execute sandbox and adds the API token. |

## Documents

| Document | One-line description |
| --- | --- |
| [00-overview.md](00-overview.md) | The problem, the two-mode surface, the token claim, and the core design idea. |
| [01-repository-map.md](01-repository-map.md) | The two Workers, the source tree, the generated trees, and the measured sizes. |
| [02-the-code-mode-surface.md](02-the-code-mode-surface.md) | The four Code Mode tools, the mode switch, and what each tool exposes. |
| [03-search-tool.md](03-search-tool.md) | How `search` runs untrusted code against the spec with no network access. |
| [04-execute-tool.md](04-execute-tool.md) | How `execute` builds a dynamic Worker, proxies API calls, and reports failures. |
| [05-the-isolate-boundary.md](05-the-isolate-boundary.md) | The token boundary, outbound restriction, result encoding, and resource limits. |
| [06-generated-code-sample.md](06-generated-code-sample.md) | A measured sample of the generated tools and the spec artifact. |
| [07-generation-pipeline.md](07-generation-pipeline.md) | Forge, `generate-tools.ts`, committed output, and the six-hourly refresh. |
| [08-request-path.md](08-request-path.md) | The route from `index.ts` through Open Authorization (OAuth), the MCP handler, and server creation. |
| [09-authentication.md](09-authentication.md) | OAuth grants, direct API tokens, identity probes, and refresh admission. |
| [10-permissions-and-refusals.md](10-permissions-and-refusals.md) | Permission lookup, refusal explanations, empty lists, and scope step-up. |
| [11-results-and-validation.md](11-results-and-validation.md) | Result truncation, direct-tool result shaping, and response schema checks. |
| [12-operations.md](12-operations.md) | Bindings, deployment, the cron, skills, events, metrics, tests, and the bench. |
| [13-limitations-and-open-questions.md](13-limitations-and-open-questions.md) | Contradictions, unverifiable claims, and gaps in the repository evidence. |

## Reading order

Read `00` and `01` first. Then read `02`, `03`, `04`, and `05` as one block:
they describe the Code Mode mechanism from surface to sandbox. Documents `06`
and `07` cover generation. Documents `08` through `12` cover serving,
authentication, permissions, results, and operations. Document `13` lists what
the repository cannot prove.

## Central finding

The model never sees the OpenAPI document. It sees a small tool surface, and it
writes code. The code runs in a fresh dynamic Worker. The credential stays in
the parent Worker and enters the sandbox only as an outbound proxy property.
The proxy is `GlobalOutbound`, a Worker entrypoint that receives the sandbox's
`fetch` calls. That property belongs to the entrypoint's props. It never
becomes a variable in the sandbox module. In Code Mode, the API response
returns to the model as text, capped at about 6,000 tokens unless the `search`
or `execute` call sets `truncate: false`. See
[04-execute-tool.md](04-execute-tool.md) and
[05-the-isolate-boundary.md](05-the-isolate-boundary.md).

## Repository and study conventions

- Source paths are relative to `/Users/darkvoid/GitSources/src.Cloudflare/mcp`.
- A citation such as `README.md:3` names the source repository's file, not this
  document.
- Numbers are measured from the checkout, not copied from prose, unless the
  text says otherwise.
- Prose claims are checked against code. A disagreement is reported, not
  silently resolved. See [13-limitations-and-open-questions.md](13-limitations-and-open-questions.md).
- English follows ASD-STE100 Simplified Technical English. Each document
  spells out an abbreviation at its first use in that document.
