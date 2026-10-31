---
title: mise-cloudflare Documentation Index
---

# mise-cloudflare Documentation

> **A mise plugin (Go) that turns a Cloudflare Worker repository into thirteen numbered tasks (03 through 15), with no API token anywhere.** Run it locally, deploy it, connect GitHub for previews, staging and production, and undo every step.

This set explains everything mise-cloudflare does, why it does it, and how it makes Cloudflare Workers, GitHub Actions, workflows, deployments and releases easy. Every claim traces to a file in the repository `joeblew999/mise-cloudflare` (commit `24301bb`).

## Foundation

- [Overview](00-overview.md): what mise-cloudflare is, the problem it solves, the thirteen numbered tasks, the design rules
- [Architecture](01-architecture.md): the module map, the dependency graph, and the command dispatch

## Core Deep Dives

- [The wconfig bridge](02-the-wconfig-bridge.md): how the tool reads a wrangler config without parsing it
- [The login model](03-the-login-model.md): the "repo's own login", auth profiles, the device flow
- [The Cloudflare API and the cf CLI](04-cloudflare-api-and-cf-cli.md): the REST wrapper and Cloudflare's own CLI
- [The check rules](05-the-check-rules.md): tokens, secret files, lockfiles, the wrangler pin
- [Templates and init](06-templates-and-init.md): the two-half embedded template and the pin
- [The status report](07-the-status-report.md): "what's done, what to run next"
- [Serve and deploy](08-serve-and-deploy.md): local dev, deploy, preview, verify
- [Connect GitHub](09-connect-github.md): Workers Builds, previews, staging and production
- [Secrets and resources](10-secrets-and-resources.md): secrets, the previews' own KV/D1/R2/Queue/Flagship
- [Delete, clean, logout and update](11-delete-clean-logout-update.md): every undo and the update flow

## Cross-Cutting

- [Testing and release](12-testing-and-release.md): the walk, the corpus, the bidirectional checker, goreleaser
- [Critical assessment](13-critical-assessment.md): what it does wrong, how it can be better, where it really gets it

## Reference

- [Glossary](14-glossary.md): every term and abbreviation in one place

---

Generated from source code. Every claim traces back to implementation.
