# clawpatrol

An agent-security firewall (Go) that sits between agents and production,
parses their traffic at the wire, and gates each action against rules written
in HCL (HashiCorp Configuration Language). This exploration is read from
`/Users/darkvoid/GitSources/src.s3/clawpatrol/`.

- [00-overview.md](./00-overview.md) — what clawpatrol is, its deployment shapes, and the shape of a rule
- [01-gateway-architecture.md](./01-gateway-architecture.md) — the gateway binary, WireGuard forwarder, and DNS virtual IPs
- [02-wire-traffic-parsing.md](./02-wire-traffic-parsing.md) — SNI peek, TLS termination, SQL AST parsing, and Kubernetes URL decomposition
- [03-hcl-rule-language-facets.md](./03-hcl-rule-language-facets.md) — the HCL config pipeline, the rule block, facets, and CEL matching
- [04-action-gating-approval.md](./04-action-gating-approval.md) — rule dispatch, fail-closed matching, and human approval
- [05-plugin-sdk.md](./05-plugin-sdk.md) — the built-in plugin registry and the author-facing plugin SDK
- [06-external-plugins.md](./06-external-plugins.md) — how sandboxed out-of-process plugins run and get permissions
- [07-dashboard.md](./07-dashboard.md) — the embedded TypeScript dashboard and its HTTP server
- [08-aws-credential.md](./08-aws-credential.md) — how the AWS credential and Kubernetes endpoint plug in
