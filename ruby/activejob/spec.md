# ActiveJob -- Project Spec

## Source Codebase

**Location:** `/Users/darkvoid/.gems/gems/activejob-8.1.4/`
**Language:** Ruby (Rails framework gem)
**Version:** 8.1.4
**Part of:** Ruby on Rails. The job framework: one interface (`ActiveJob::Base` /
`ApplicationJob`) over many queue backends (adapters).

## What ActiveJob Is

A job is a Ruby class with one `perform` method. `perform_later` serializes the
job (its class, queue, options, and its arguments) and hands it to a queue
adapter; later the adapter runs it, which deserializes the arguments, rebuilds
the job, and calls `perform`. Plain Ruby objects — including Active Record
models, which become GlobalIDs — are the arguments. Callbacks, `retry_on` /
`discard_on`, and concerns make it feel like the rest of Rails.

## Documentation Structure

```
ruby/activejob/
├── spec.md                          ← THIS FILE
└── markdown/
    ├── README.md                    ← index
    ├── 00-overview.md
    ├── 01-the-job.md
    ├── 02-enqueue-lifecycle.md
    ├── 03-execution.md
    ├── 04-arguments-and-serializers.md
    ├── 05-queue-adapters.md
    ├── 06-callbacks.md
    ├── 07-exceptions-retry-discard.md
    ├── 08-continuable.md
    ├── 09-instrumentation-and-timezone.md
    └── 10-rails-integration-and-testing.md
```

## Quality Requirements (Iron Rules)

1. At least two Mermaid diagrams, three code snippets, one `## The Aha Moment`
   (opening with a bolded `**Aha:**`), and two cross-references per document.
2. H1 first line `# NN -- Title: subtitle`, no YAML frontmatter.
3. Footer chain 00→…→10→README; no orphan pages.
4. ASD-STE100 Simplified Technical English; abbreviations spelled out at first use.
5. Real source code snippets with path + approximate line.
6. HTML via `/Users/darkvoid/Dev/explorations/build.py activejob`.

## Verification

Authored by an agent reading the full gem source; verified by independent
agents (fresh-reader review + grandfather review against the gem source). See
the `ajdc` spec for the same process applied to the sibling exploration.
