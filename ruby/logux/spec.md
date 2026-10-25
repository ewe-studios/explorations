# Logux -- Project Spec

## Source Codebase

**Location:**
- `/Users/darkvoid/GitSources/src.Ruby/logux-core/`
- `/Users/darkvoid/GitSources/src.Ruby/logux-server/`
- `/Users/darkvoid/GitSources/src.Ruby/logux-actions/`

**Language:** JavaScript/TypeScript, Node.js (ESM-only, `"type": "module"`)

**Version:**
- `@logux/core` 0.12.1
- `@logux/server` 0.17.1
- `@logux/actions` 0.8.1

**Part of:** the Logux framework. Logux is a JavaScript/TypeScript
client-server synchronization framework. Three pnpm monorepos ship it:
`@logux/core` (the action log, node primitives, and the Logux wire protocol),
`@logux/server` (the WebSocket server that syncs actions and tracks
subscriptions), and `@logux/actions` (type definitions and action creators for
the built-in actions).

## What Logux Is

Every change is a plain action object appended to a persistent log on both
client and server. The log replicates in both directions over WebSocket. An
action has a `type` string and data fields. Its metadata carries an identifier
(`meta.id`), a creation time (`meta.time`), a sequence number (`meta.added`),
and `reasons` that decide how long the action stays in the log. Both sides keep
the same log order, so conflicts resolve deterministically by order. A
subscription is itself a log action, so shared state (subscriptions, sync maps,
processing receipts) is durable like everything else.

## Documentation Structure

```
ruby/logux/
├── spec.md                          ← THIS FILE
└── markdown/
    ├── README.md                    ← index (last in the footer chain)
    ├── 00-what-is-logux.md
    ├── 01-action-and-meta.md
    ├── 02-log-and-node.md
    ├── 03-wire-protocol.md
    ├── 04-server-and-subscriptions.md
    ├── 05-offline-connect-reconnect.md
    ├── 06-sync-map.md
    ├── 07-processing-lifecycle.md
    ├── 08-builtin-actions.md
    └── 09-composition.md
```

## Quality Requirements (Iron Rules)

1. First line of every numbered doc is the H1 `# NN -- Title: subtitle`.
   No YAML frontmatter, no leading blank line.
2. At least two Mermaid diagrams per doc. Each begins with its init directive
   on its own line: `%%{init: {"flowchart": {"htmlLabels": false}}}%%` for a
   flowchart, or `%%{init: {"sequence": {"htmlLabels": false}}}%%` for a
   sequence diagram. Every node label is quoted. No unquoted parentheses
   inside labels.
3. At least three real code snippets per doc. Each is fenced and attributed
   with a source path plus a line range inside the fence as a `//` comment.
   Source is copied verbatim; nothing is invented. When the exact line is
   uncertain, the comment says `around lines N-M`.
4. Exactly one `## The Aha Moment` per doc. Its first line starts with a bolded
   `**Aha:**`.
5. At least two cross-references per doc, in a `## Cross-References` section
   with relative `./NN-slug.md` links.
6. Last line of every numbered doc is the footer
   `Continue to [NN-slug.md](NN-slug.md).` The chain runs
   00 → 01 → 02 → 03 → 04 → 05 → 06 → 07 → 08 → 09 → README. No orphans, no
   broken links.
7. ASD-STE100 Simplified Technical English: short sentences, one idea per
   sentence, active voice, no idiom. Spell out every abbreviation at first use.
8. Every factual claim is verified against the source. When a README and the
   code disagree, the exploration says so and prefers the code.

## Verification

Authored by an agent reading the full source of the three monorepos. It will be
verified by independent agents that re-read the same source. See the
`activejob` and `ajdc` specs for the same process applied to sibling
explorations.
