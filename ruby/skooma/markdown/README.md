# Skooma Deep Exploration — Index

A technical exploration of the Ruby gem `skooma-0.4.0` (source at `/Users/darkvoid/GitSources/src.Ruby/skooma`), a library for validating Application Programming Interface (API) implementations against OpenAPI 3.1.x documents.

- [00-overview.md](./00-overview.md) — What Skooma is, the problem it solves, and the map of the whole library.
- [01-loading-a-document.md](./01-loading-a-document.md) — How an OpenAPI document is loaded, resolved, and compiled into a JSON Schema tree.
- [02-matching-an-operation.md](./02-matching-an-operation.md) — How a request path and Hypertext Transfer Protocol (HTTP) method route to one operation.
- [03-the-validation-call.md](./03-the-validation-call.md) — The call that validates an environment and returns the list of violations.
- [04-request-validation.md](./04-request-validation.md) — How parameters and request bodies are parsed, coerced, and validated.
- [05-response-validation.md](./05-response-validation.md) — How response status, content, media type, and headers are validated.
- [06-test-helpers-and-coverage.md](./06-test-helpers-and-coverage.md) — The RSpec and Minitest helpers, coverage reporting, and external references.
