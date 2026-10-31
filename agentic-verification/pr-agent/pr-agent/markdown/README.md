---
title: PR-Agent Documentation
---

# PR-Agent Documentation

PR-Agent is an open-source, artificial intelligence (AI) powered code review agent. It reviews pull requests (PRs), writes descriptions, suggests code improvements, answers questions, and more, across ten git providers and nine webhook servers (plus a GitHub Action runner and a polling loop).

This documentation explains the project end to end, from the core agent to the packaging. Every chapter cites source files and line numbers. Every abbreviation is spelled out where it is used.

## Chapters

| Chapter | Topic |
|---------|-------|
| [00](00-overview.md) | Overview |
| [01](01-architecture.md) | Architecture |
| [02](02-the-core-agent.md) | The core agent |
| [03](03-ai-handlers-and-model-routing.md) | AI handlers and model routing |
| [04](04-token-handling-and-prompts.md) | Token handling and prompts |
| [05](05-pr-and-patch-processing.md) | Pull request and patch processing |
| [06](06-review-pipeline.md) | Review pipeline |
| [07](07-output-models-and-sinks.md) | Output models and sinks |
| [08](08-git-providers.md) | Git providers |
| [09](09-identity-and-secret-providers.md) | Identity and secret providers |
| [10](10-cli-and-config.md) | Command-line interface (CLI) and config |
| [11](11-servers-and-deployment.md) | Servers and deployment |
| [12](12-tools-review-and-describe.md) | Tools: review and describe |
| [13](13-tools-code-suggestions-and-questions.md) | Tools: suggestions and questions |
| [14](14-tools-docs-labels-and-help.md) | Tools: docs, labels, help |
| [15](15-mosaico-a2a.md) | Mosaico agent-to-agent (A2A) |
| [16](16-telemetry-and-observability.md) | Telemetry and observability |
| [17](17-packaging-ci-and-tests.md) | Packaging, continuous integration (CI), tests |
| [18](18-glossary.md) | Glossary |

## The one idea

A pull request is turned into a structured context blob — the diff, the file list, the language map, and the repository context — which is passed through a prompt template to a model handler, which returns output that is published back to the provider. Every step is a separate, configurable layer.

## Start here

Read [Overview](00-overview.md) first, then [Architecture](01-architecture.md).
