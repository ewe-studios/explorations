# clawpatrol-plugin-aws Deep Exploration — Index

A technical exploration of the external clawpatrol plugin that gates AWS (Amazon Web Services) API calls across many accounts. The source is at `/Users/darkvoid/GitSources/src.s3/clawpatrol-plugin-aws`.

- [00-plugin-architecture.md](./00-plugin-architecture.md) — What the plugin registers (facet, credential, endpoint), the no-network rule, and the full request pipeline.
- [01-tls-termination-and-parsing.md](./01-tls-termination-and-parsing.md) — TLS termination, reading the request, and how each request becomes the `aws` facet.
- [02-account-selection-and-assume-role.md](./02-account-selection-and-assume-role.md) — Deriving the account id, choosing the base key, assuming the per-account role, and resolving account names.
- [03-sigv4-resigning.md](./03-sigv4-resigning.md) — Building a clean request and re-signing it with SigV4 (Signature Version 4).
- [04-s3-and-iam-mapping.md](./04-s3-and-iam-mapping.md) — Reconstructing S3 (Simple Storage Service) operations and mapping them to IAM (Identity and Access Management) actions.
- [05-chunked-and-body-handling.md](./05-chunked-and-body-handling.md) — Decoding aws-chunked bodies, size bounds, and the response body tap.
