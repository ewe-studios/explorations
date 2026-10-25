# Skooma -- Project Spec

## Source Codebase

**Location:** `/Users/darkvoid/GitSources/src.Ruby/skooma/`
**Language:** Ruby (gem)
**Version:** 0.4.0
**Part of:** A standalone gem by Svyatoslav Kryukov, built by Evil Martians. It
builds on the `json_skooma` library for JavaScript Object Notation (JSON)
Schema validation.

## What Skooma Is

Skooma validates an Application Programming Interface (API) implementation
against an OpenAPI 3.1.0 document. The document itself is a JSON Schema, so
Skooma compiles it into a tree of Ruby objects. A test helper captures a real
request or response and Skooma validates it against the schema for the matching
operation. The result is a list of violations.

## Documentation Structure

```
ruby/skooma/
├── spec.md                          ← THIS FILE
└── markdown/
    ├── README.md                    ← index
    ├── 00-overview.md
    ├── 01-loading-a-document.md
    ├── 02-matching-an-operation.md
    ├── 03-the-validation-call.md
    ├── 04-request-validation.md
    ├── 05-response-validation.md
    └── 06-test-helpers-and-coverage.md
```

## Quality Requirements (Iron Rules)

1. At least two Mermaid diagrams, three code snippets, one `## The Aha Moment`
   (opening with a bolded `**Aha:**`), and two cross-references per document.
2. H1 first line `# NN -- Title: subtitle`, no YAML frontmatter.
3. Footer chain 00→…→06→README; no orphan pages.
4. ASD-STE100 Simplified Technical English; abbreviations spelled out at first use.
5. Real source code snippets with path + approximate line.
6. HTML via `/Users/darkvoid/Dev/explorations/build.py ruby/skooma`.

## Verification

Authored by an agent reading the full gem source; verified by independent
agents (fresh-reader review + grandfather review against the gem source).
