# ActiveJob Deep Exploration — Index

A technical exploration of the Ruby on Rails job framework, `activejob-8.1.4` (installed at `/Users/darkvoid/.gems/gems/activejob-8.1.4`).

- [00-overview.md](./00-overview.md) — What ActiveJob is: one job interface over many queue backends, and the map of the whole framework.
- [01-the-job.md](./01-the-job.md) — The job object, its attributes, the class-level DSL, and its serialized hash form.
- [02-enqueue-lifecycle.md](./02-enqueue-lifecycle.md) — The call chain from `perform_later` through `enqueue` to the adapter, including options and transaction deferral.
- [03-execution.md](./03-execution.md) — `perform_now`, `execute`, argument decoding, and the nested run that calls your `perform`.
- [04-arguments-and-serializers.md](./04-arguments-and-serializers.md) — `ActiveJob::Arguments`, the serializer registry, and how models become Global IDs and reload from the database.
- [05-queue-adapters.md](./05-queue-adapters.md) — The `enqueue` / `enqueue_at` adapter contract, the async, inline, and test adapters, and how real backends plug in.
- [06-callbacks.md](./06-callbacks.md) — The six before / after / around hooks, ordering, and halting.
- [07-exceptions-retry-discard.md](./07-exceptions-retry-discard.md) — `retry_on`, `discard_on`, `rescue_from`, backoff and jitter, and how a failed job re-enqueues.
- [08-continuable.md](./08-continuable.md) — Steps, cursors, checkpoints, interruption, and resume; the foundation the ajdc exploration builds on.
- [09-instrumentation-and-timezone.md](./09-instrumentation-and-timezone.md) — Notifications, logging, structured events, and locale and timezone restoration.
- [10-rails-integration-and-testing.md](./10-rails-integration-and-testing.md) — The Railtie, the job generator, the test adapter, and the test helpers.
