# AJ/DC -- Documentation Index

AJ/DC (Active Job Durable Continuation) is a Rails gem by Vladimir Dementyev
that makes `ActiveJob::Continuable` jobs durable. The run and its steps are
database rows, so progress survives a crash, a run can wait for a time or a
signal without holding a worker, and every run is identified, queryable, and
resumable.

These documents explain the whole gem, including its dependency
`ActiveJob::Continuable` from Rails 8.1. Read them in order; each one builds on
the last.

1. [00-overview.md](./00-overview.md) -- durable continuation on Active Job: the four capabilities a database-backed run buys over a payload-serialized continuation.
2. [01-architecture.md](./01-architecture.md) -- one concern, one run record, one mediator: how `ActiveJob::Durable`, the `Run`/`Step` models, and the `Execution` mediator fit together.
3. [02-continuable-foundation.md](./02-continuable-foundation.md) -- the dependency: `ActiveJob::Continuable`, the `Continuation` class, cursors, interrupts, and resumptions.
4. [03-installation-and-setup.md](./03-installation-and-setup.md) -- two tables, one clock, one database decision: the generator, `connects_to`, and the recurring jobs.
5. [04-data-model.md](./04-data-model.md) -- the schema, `Run`/`Step` models, and the `StateType` JSON serialization, column by column.
6. [05-steps-cursors-checkpoints.md](./05-steps-cursors-checkpoints.md) -- the `step` DSL, cursors, isolated steps, and how checkpoints commit progress.
7. [06-run-lifecycle.md](./06-run-lifecycle.md) -- the nine statuses and the guarded, last-write-wins transitions between them.
8. [07-serialization-and-restore.md](./07-serialization-and-restore.md) -- how the payload carries only `durable_run_id` and the run row is rebuilt into a continuation on every execution.
9. [08-identity-and-uniqueness.md](./08-identity-and-uniqueness.md) -- `key` vs `active_key`, `identified_by`, `unique_by`, and `on_conflict`.
10. [09-timers.md](./09-timers.md) -- `wait`, `wait_until`, and the single `WakeJob` that wakes every due run.
11. [10-signals.md](./10-signals.md) -- `await` and `wake_up`: parking a run until an external signal arrives.
12. [11-halting-resuming-cancelling.md](./11-halting-resuming-cancelling.md) -- `halt_on`, `halt!`, `resume!`, and `cancel!`: the states that need a person.
13. [12-querying-and-observability.md](./12-querying-and-observability.md) -- `workflow_runs`, the scopes, `for`, `at_step`, and `stuck_for`.
14. [13-callbacks-and-rescue.md](./13-callbacks-and-rescue.md) -- step callbacks and `RescueHandlersFirst`: why declared handlers win over continuation resume.
15. [14-housekeeping-and-agent-skill.md](./14-housekeeping-and-agent-skill.md) -- `clean_up`, retention, and the shipped agent skill that encodes the decision procedure.

The source of truth is the gem at `/Users/darkvoid/GitSources/src.Ruby/ajdc`
and, for the dependency, the installed
`/Users/darkvoid/.gems/gems/activejob-8.1.4/lib/active_job/` files.
