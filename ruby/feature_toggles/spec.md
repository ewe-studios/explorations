# Feature Toggles -- Project Spec

## Source Codebase

**Location:** `/Users/darkvoid/GitSources/src.Ruby/feature_toggles/`
**Language:** Ruby (framework-agnostic gem with optional Rails integration)
**Version:** 1.2.1
**Part of:** A standalone gem by Michael Merkushin (bibendi), sponsored by Evil
Martians. Not part of Rails.

## What Feature Toggles Is

FeatureToggles manages pending features that take longer than a single release
cycle. You ship the code behind a toggle and switch the toggle per user or per
environment without a redeploy. A feature is a name plus a Ruby block (the
resolver). `enabled?` runs the block and coerces the answer to true or false.
The config is a Ruby Domain-Specific Language (DSL), not YAML — the source has
no YAML parser anywhere.

## Documentation Structure

```
ruby/feature_toggles/
├── spec.md                          ← THIS FILE
└── markdown/
    ├── README.md                    ← index
    ├── 00-overview.md
    ├── 01-config-dsl.md
    ├── 02-feature-object.md
    ├── 03-runtime-api.md
    └── 04-loading-and-rails.md
```

## Quality Requirements (Iron Rules)

1. At least two Mermaid diagrams, three code snippets, one `## The Aha Moment`
   (opening with a bolded `**Aha:**`), and two cross-references per document.
2. H1 first line `# NN -- Title: subtitle`, no YAML frontmatter, no leading
   blank line.
3. Every Mermaid diagram begins with an init directive on its own line; every
   node label is quoted.
4. Footer chain 00→01→02→03→04→README; no orphan pages.
5. ASD-STE100 Simplified Technical English; abbreviations spelled out at first
   use.
6. Real source code snippets with path + approximate line; no invented code.
7. HTML via `/Users/darkvoid/Dev/explorations/build.py ruby/feature_toggles`.

## Verification

Authored by an agent reading the full gem source (README, gemspec, every file
under `lib/`, and all `spec/` tests); verified by independent agents against
the gem source. When source and README disagree, the docs say so and prefer the
source.
