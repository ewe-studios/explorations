# AJ/DC -- Project Spec

## Source Codebase

**Location:** `/Users/darkvoid/GitSources/src.Ruby/ajdc`
**Language:** Ruby (Rails engine/gem)
**Version:** 0.1.0 (`lib/ajdc/version.rb`)
**Author:** Vladimir Dementyev (palkan)
**License:** MIT
**Repository:** https://github.com/palkan/ajdc
**Tagline:** Durable continuation on Active Job: the run and its steps are database rows, so progress survives a crash and a run can wait for a time or a signal without holding a worker.

**Requirements:** Ruby (MRI) >= 3.3, Rails >= 8.1 (`ActiveJob::Attributes` needs Rails 8.2), SQLite / PostgreSQL / MySQL.

**Scale:** One Ruby gem. Roughly 1,469 lines of library code in `lib/`
(`active_job/durable.rb` plus its `durable/` helpers, the `continuation/` and
`continuable/` refinements, and `ajdc/railtie`), a Rails generator
(`lib/generators/ajdc/install`), a database schema (`db/durable_schema.rb`),
and a shipped agent skill (`skills/ajdc/`: a `SKILL.md` decision procedure, five
references, six examples). Tests span 3,808 lines across `test/cases/`.

## What AJ/DC Is

AJ/DC (Active Job Durable Continuation) makes an `ActiveJob::Continuable` job
durable. `ActiveJob::Continuable` (Rails 8.1) already lets a job make progress
across graceful restarts by serializing its continuation into the job payload.
AJ/DC moves that state out of the payload and into the database: every run is a
row in `active_job_durable_runs`, every step attempt a row in
`active_job_durable_steps`. The payload carries only a `durable_run_id`.

That single change buys four capabilities a payload-serialized continuation
cannot offer:

1. **Crash durability.** A `SIGKILL` loses at most the work since the last
   checkpoint, because the cursor and the attribute values are committed to the
   run row at every checkpoint and step boundary, not only at a graceful stop.
2. **Waiting without a worker.** A step can park the run (`waiting` for a timer,
   `awaiting` for a signal) by writing the run row and dropping out of the
   queue; a single recurring `WakeJob` wakes due runs. A parked run costs one
   row, not a queue slot or a held worker.
3. **Identity and uniqueness.** Every run has a `key` (what it is about) and an
   `active_key` (what may run once at a time). `workflow_runs.for(...)` finds a
   run by its arguments; `unique_by` forbids a second live run for the same
   identity, with `on_conflict` deciding `:skip`, `:reject`, or `:replace`.
4. **Observability and control.** `MyJob.workflow_runs` is a plain Active Record
   relation; the nine statuses, `stuck_for`, `halted`, `awaiting` and friends
   answer "where is X?" and "what needs a person?" directly. A `halted` or
   `failed` run resumes in place with `resume!`; a `waiting`/`awaiting` run is
   woken with `wake_up`.

AJ/DC is therefore not a new job runner. It is `ActiveJob::Continuable` with the
continuation, the attribute values and the resumption count rebuilt from a run
row on every execution, plus timers, signals, halting, uniqueness and
housekeeping built on top of that row.

## Documentation Goal

A reader who finishes these documents can:

1. Explain what durable execution means on Active Job, and why moving the
   continuation into a database row beats the payload-serialized form.
2. Describe `ActiveJob::Continuable` and `ActiveJob::Continuation`: the `step`
   DSL, cursors (`set!`, `advance!`, `checkpoint!`), isolated steps, interrupts,
   resumptions, and the `continue`/`resume_job` machinery AJ/DC extends.
3. Trace the JSON-to-view equivalent for jobs: `perform_later` creates the run
   row, `perform` rebuilds the continuation from the row, and every step
   boundary and checkpoint writes the run and step rows.
4. Explain the data model: every column and index of the two tables, the
   `Run`/`Step` models, and the `StateType` JSON serialization.
5. Describe the nine run statuses and the guarded, last-write-wins transitions
   between them.
6. Explain identity (`key`) versus uniqueness (`active_key`): `identified_by`,
   `unique_by`, `on_conflict`, `ArgsMapper`, the SHA-256 digest fallback, and
   the unique-index race.
7. Explain timers (`wait`, `wait_until`) and signals (`await`, `wake_up`), and
   how `WakeJob` is the single clock that wakes every due run.
8. Explain halting, resuming, and cancelling: `halt_on`, `halt!`, `resume!`,
   `cancel!`, and the exceptions each guarded transition raises.
9. Describe the query surface (`workflow_runs` scopes, `for`, `at_step`,
   `stuck_for`, `newest_first`) and how runs are read and operated on.
10. Explain step callbacks (`before_step`, `after_step`, `around_step`) and
    `RescueHandlersFirst`, and how declared handlers win over continuation
    resume.
11. Describe housekeeping (`clean_up`, `keep_terminal_runs_for`) and the shipped
    agent skill (`skills/ajdc`) that encodes the decision procedure.
12. Explain the transaction story: why `perform_later` inside the caller's
    transaction is the outbox pattern, and what breaks with a separate database.

## Documentation Structure

```
ruby/ajdc/
├── spec.md                          ← THIS FILE (project tracker)
├── markdown/                        ← Source documentation
│   ├── README.md                    ← Index / table of contents
│   ├── 00-overview.md               ← durable continuation on Active Job, the four capabilities
│   ├── 01-architecture.md           ← one concern, one run record, one mediator
│   ├── 02-continuable-foundation.md ← ActiveJob::Continuable and Continuation (the dependency)
│   ├── 03-installation-and-setup.md ← two tables, one clock, one database decision
│   ├── 04-data-model.md             ← runs and step attempts: schema, models, StateType
│   ├── 05-steps-cursors-checkpoints.md ← step DSL, cursors, isolated, checkpoint
│   ├── 06-run-lifecycle.md          ← nine statuses and guarded transitions
│   ├── 07-serialization-and-restore.md ← the run is the source of truth
│   ├── 08-identity-and-uniqueness.md ← key vs active_key, unique_by, on_conflict
│   ├── 09-timers.md                 ← wait, wait_until, WakeJob, wake_up_due
│   ├── 10-signals.md                ← await and wake_up
│   ├── 11-halting-resuming-cancelling.md ← halt_on, halt!, resume!, cancel!
│   ├── 12-querying-and-observability.md ← scopes, for, at_step, stuck_for
│   ├── 13-callbacks-and-rescue.md   ← step callbacks and RescueHandlersFirst
│   └── 14-housekeeping-and-agent-skill.md ← cleanup, retention, the shipped skill
└── html/                            ← Rendered HTML (build.py output)
    ├── index.html
    ├── styles.css
    └── *.html
```

## Tasks

### Phase 1: Core Documentation (Markdown)

| # | Task | Status | File |
|---|------|--------|------|
| 1 | Overview: durable continuation, four capabilities | done | `00-overview.md` |
| 2 | Architecture: concern, run, mediator | done | `01-architecture.md` |
| 3 | Foundation: ActiveJob::Continuable and Continuation | done | `02-continuable-foundation.md` |
| 4 | Installation: tables, clock, database decision | done | `03-installation-and-setup.md` |
| 5 | Data model: schema, Run/Step, StateType | done | `04-data-model.md` |
| 6 | Steps, cursors, checkpoints | done | `05-steps-cursors-checkpoints.md` |
| 7 | Run lifecycle: nine statuses, transitions | done | `06-run-lifecycle.md` |
| 8 | Serialization and restore | done | `07-serialization-and-restore.md` |
| 9 | Identity and uniqueness | done | `08-identity-and-uniqueness.md` |
| 10 | Timers: wait, wait_until, WakeJob | done | `09-timers.md` |
| 11 | Signals: await and wake_up | done | `10-signals.md` |
| 12 | Halting, resuming, cancelling | done | `11-halting-resuming-cancelling.md` |
| 13 | Querying runs | done | `12-querying-and-observability.md` |
| 14 | Callbacks and rescue handlers | done | `13-callbacks-and-rescue.md` |
| 15 | Housekeeping and the agent skill | done | `14-housekeeping-and-agent-skill.md` |

### Phase 2: HTML Rendering

| # | Task | Status | File |
|---|------|--------|------|
| 16 | Generate all HTML pages | done | `html/*.html` |
| 17 | Index page with navigation | done | `html/index.html` |
| 18 | Shared CSS | done | `html/styles.css` |

### Phase 3: Grandfather Review (independent verification)

| # | Task | Status |
|---|------|--------|
| 19 | Names match source | done |
| 20 | Numbers match source | done |
| 21 | Flows match source | done |
| 22 | Nothing missing from the public API | done |

## Build System

**Script:** `/Users/darkvoid/Dev/explorations/build.py`
**Dependencies:** None (Python 3 stdlib only)

**Usage:**
```bash
cd /Users/darkvoid/Dev/explorations
python3 build.py ruby/ajdc
```

The `descriptions` dict in `build.py` has an `ajdc` entry (added).

## Quality Requirements (Iron Rules)

1. Detailed sections with real code snippets and source paths.
2. Teach the key idea in the first paragraph, then go deeper.
3. Short, clear sentences. Mermaid diagrams for every process.
4. At least two Mermaid diagrams per document.
5. At least three code snippets per document.
6. At least one Aha moment per document.
7. At least two cross-references per document.
8. Generated HTML via the shared build.py.
9. No orphan pages.
10. Navigation: index button, prev/next buttons, theme toggle.
11. All prose in ASD-STE100 Simplified Technical English: every abbreviation
    spelled out at first use, short sentences, one idea per sentence.

## Depth Requirement

AJ/DC is a small gem (about 1,469 lines of library code) but it stands on a
large dependency: `ActiveJob::Continuable` and `ActiveJob::Continuation` from
Rails 8.1. Both sides get source-level coverage. Every column, index, scope,
status transition, macro, exception, and the transaction story must be
documented so a reader never has to open the source to understand the behavior.
The installed dependency source lives at
`/Users/darkvoid/.gems/gems/activejob-8.1.4/lib/active_job/` (`continuable.rb`,
`continuation.rb`, `continuation/step.rb`, `continuation/validation.rb`,
`continuation/test_helper.rb`).

## Grandfather Review

The code is the grandfather. Review checks four axes:

1. Names match source: modules, classes, methods, columns, scopes, macros.
2. Numbers match source: status counts, column counts, defaults, intervals.
3. Flows match source: the enqueue-to-run pipeline, step lifecycle, wake path,
   uniqueness resolution, and the transaction story.
4. Nothing missing from the public API surface (macros, scopes, exceptions,
   job classes, configuration).

Every name, number, and flow must be checked against the source in
`/Users/darkvoid/GitSources/src.Ruby/ajdc/` and the installed activejob gem
before the task is marked DONE. Verification must be performed by independent
agents, not by the authors.

## Resume Point

To resume: finish any `pending` task above in document order, read the relevant
source files before writing, write the markdown, run
`python3 build.py ruby/ajdc`, and run the grandfather review against the source
before marking the task DONE.
