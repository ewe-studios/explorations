# clawpatrol-plugin-aws -- Project Spec

## Source Codebase

**Location:** `/Users/darkvoid/GitSources/src.s3/clawpatrol-plugin-aws/`
**Language:** Go
**Version:** Go 1.26.3 (from go.mod), clawpatrol SDK v0.5.0, plugin Version 0.3.10 (from main.go)
**Part of:** An external plugin for clawpatrol (`github.com/denoland/clawpatrol-plugin-aws`)

## What clawpatrol-plugin-aws Is

clawpatrol-plugin-aws is an external clawpatrol plugin that gates calls to the
AWS (Amazon Web Services) APIs across many accounts. It terminates the agent's
TLS (Transport Layer Security), parses each request into an `aws` facet so
rules can match AWS operations by name, selects the target account from the
request, assumes a per-account role with a single base key, re-signs the
request with SigV4 (Signature Version 4), and forwards to AWS through the
gateway's brokered dial. It runs with no network of its own.

## Documentation Structure

```
src3/clawpatrol-plugin-aws/
├── spec.md                          ← THIS FILE
└── markdown/
    ├── README.md                    ← index / terminal page
    ├── 00-plugin-architecture.md
    ├── 01-tls-termination-and-parsing.md
    ├── 02-account-selection-and-assume-role.md
    ├── 03-sigv4-resigning.md
    ├── 04-s3-and-iam-mapping.md
    └── 05-chunked-and-body-handling.md
```

## Quality Requirements (Iron Rules)

1. At least two Mermaid diagrams, three code snippets, one `## The Aha Moment`
   (opening with a bolded `**Aha:**`), and two cross-references per document.
2. H1 first line `# NN -- Title: subtitle`, no YAML frontmatter.
3. Every Mermaid fence opens with a `%%{init: ...}%%` directive; with
   `htmlLabels: false`, quote every node label and never put an unquoted
   `(` or `)` inside a label.
4. Footer chain 00→01→02→03→04→05→README; no orphan pages.
5. ASD-STE100 Simplified Technical English; abbreviations spelled out at first use.
6. Real source code snippets with path + line range, verbatim.
7. HTML via `python3 build.py src3/clawpatrol-plugin-aws`.

## Verification

Authored by an agent that read README.md, go.mod, every `.go` file (main.go,
parse.go, s3op.go, iamaction.go, assumerole.go, credselect.go, orgresolver.go,
chunked.go), and every `*_test.go` file in full. Every factual claim is checked
against the source; where the README and the code disagree, the code wins.
