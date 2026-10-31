---
source_location: /Users/darkvoid/GitSources/src.Ged/mise-cloudflare (commit 24301bbdb4f30de375c4fe5b1fe9831231efd365)
repository: git@github.com:joeblew999/mise-cloudflare.git
explored_at: 2026-10-31
documentation_goal: Explain everything mise-cloudflare does, why it does it, and how it makes Cloudflare Workers, GitHub Actions, workflows, deployments and releases easy — plus what it does wrong, how it can be better, and what is worth learning.
---

# Spec: mise-cloudflare Documentation

## 1. Source

| Property | Value |
|----------|-------|
| Location | /Users/darkvoid/GitSources/src.Ged/mise-cloudflare, commit `24301bb` |
| Language | Go (module `github.com/joeblew999/mise-cloudflare`, `go 1.26.0`) |
| Version | version is injected by goreleaser at build time (`-X main.version={{ .Version }}`) |
| License | MIT (Copyright 2026 joeblew999) |
| Size | 79 tracked files, 6,600 lines of Go |
| Main files studied | `cmd/mise-cloudflare/*.go`, `internal/{wconfig,cf,cfapi,rules,wrangler}`, `templates/`, `mise.toml`, `.github/workflows/*.yml`, `.goreleaser.yaml`, the end-to-end tests |

## 2. What mise-cloudflare Is

mise-cloudflare is a backend plugin for [mise](https://mise.jdx.dev), the polyglot version and task manager. It turns one Cloudflare Worker repository into thirteen numbered mise tasks (03 through 15) that run, deploy, connect and undo the Worker. The single idea that makes it different: there is no API token anywhere. Instead of tokens, the repository has a **repo's own login** — a wrangler auth profile bound to the repository's folder — and Cloudflare's Workers Builds deploys every push with Cloudflare's own build token, which never leaves Cloudflare. A Go command-line interface (CLI) reads nothing about the wrangler config itself; it asks the repository's own wrangler, via a small embedded JavaScript bridge, and it decides every outcome on the HTTP status Cloudflare returns, never on the wording.

## 3. Documentation Goal

1. Know the mental model: the thirteen numbered tasks (03 through 15), the "repo's own login" instead of tokens, and the rule that every step has an undo.
2. Know every command of the CLI (init, check, show, serve, set-secrets, deploy, connect-github, set-preview-secrets, preview, login, delete, logout, clean, verify, update, template, run-wrangler, show-version, show-help).
3. Know the bridge that reads a wrangler config without parsing it (`internal/wconfig`), and why it was built that way.
4. Know the login model: wrangler auth profiles, the device-authorization flow, scope checking, and the refusal of stray `CLOUDFLARE_API_TOKEN`.
5. Know the check rules and every lockfile, secret file and pin they enforce.
6. Know the two-half embedded template, init, template, and how the pin keeps the repo on one release.
7. Know the status report ("what's done, what to run next") and its priority-ordered `next` field.
8. Know Workers Builds: production, previews, staging, the previews' own resources, and how `connect-github` drives the cf CLI.
9. Know secrets, the previews' own KV/D1/R2/Queue/Flagship resources, delete, clean, logout and update.
10. Know the test strategy (the walk, the corpus, the bidirectional mise↔Go checker) and the release pipeline (goreleaser, GitHub Actions, the module proxy).
11. Know the strengths, weaknesses and risks of the approach, ranked, and what is worth learning.

## 4. Documentation Structure

```
src.Ged/mise-cloudflare/
├── exploration.md
├── spec.md
├── build.py
├── styles.css
├── markdown/   (README.md, 00 to 14)
└── html/
```

## 5. Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | markdown/README.md | DONE |
| 1 | 00-overview.md | DONE |
| 2 | 01-architecture.md | DONE |
| 3 | 02-the-wconfig-bridge.md | DONE |
| 4 | 03-the-login-model.md | DONE |
| 5 | 04-cloudflare-api-and-cf-cli.md | DONE |
| 6 | 05-the-check-rules.md | DONE |
| 7 | 06-templates-and-init.md | DONE |
| 8 | 07-the-status-report.md | DONE |
| 9 | 08-serve-and-deploy.md | DONE |
| 10 | 09-connect-github.md | DONE |
| 11 | 10-secrets-and-resources.md | DONE |
| 12 | 11-delete-clean-logout-update.md | DONE |
| 13 | 12-testing-and-release.md | DONE |
| 14 | 13-critical-assessment.md | DONE |
| 15 | 14-glossary.md | DONE |
| 16 | exploration.md | DONE |
| 17 | Grandfather review | DONE |
| 18 | Generate HyperText Markup Language (HTML) | DONE |

## 6. Build System

Run `python3 -I build.py .` inside this directory. It writes `html/`. No dependencies beyond the Python standard library. This copy of `build.py` escapes raw angle brackets, processes inline markup inside table cells, and carries a `mise-cloudflare` entry in its `descriptions` dictionary.

## 7. Quality Requirements (Iron Rules)

1. Detailed sections with code snippets.
2. Teach key facts, principles and ideas quickly.
3. Clear articulation.
4. Mermaid diagrams (minimum 2 per document).
5. Good visual assets.
6. Generated HyperText Markup Language (HTML).
7. Cross-references.
8. Source path references.
9. Aha moments.
10. Navigation: index plus prev/next buttons.

Also: ASD-STE100 Simplified Technical English. Abbreviations are spelled out where used. The code wins over the docs.

## 8. Expected Outcome

A reader can run the thirteen numbered tasks (03 through 15), explain why there is no API token, decide what to copy into their own Worker-repository workflow, and find the code for any claim.

## 9. Resume Point

Scaffolding (spec, exploration, build, styles) is written. To continue: write the markdown documents in `markdown/`, run `python3 -I build.py .`, then run the grandfather review against the source at commit `24301bb` and fix every discrepancy.
