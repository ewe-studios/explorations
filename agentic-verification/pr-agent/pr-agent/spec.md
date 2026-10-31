---
source_location: /Users/darkvoid/GitSources/src.pr-agent/pr-agent (commit b32a02128dbf629610e9f4009d13cc1e1bebef9b)
repository: git@github.com:The-PR-Agent/pr-agent.git
explored_at: 2026-10-31
documentation_goal: Explain PR-Agent end to end: the core agent and command dispatch, AI handlers and model routing, token and prompt handling, the pull-request processing pipeline, every git provider, the CLI and configuration system, every server, every tool, the Mosaico agent-to-agent layer, telemetry, and the packaging and test story.
---

# Spec: PR-Agent Documentation

## 1. Source

| Property | Value |
|----------|-------|
| Location | /Users/darkvoid/GitSources/src.pr-agent/pr-agent, commit `b32a021` |
| Language | Python (package `pr_agent`, requires Python 3.12 or newer) |
| Version | 0.49.0 |
| License | MIT |
| Author | QodoAI (community-maintained legacy project of Qodo) |
| Repository | git@github.com:The-PR-Agent/pr-agent.git |
| Main files studied | `pr_agent/agent/pr_agent.py`, `pr_agent/algo/*`, `pr_agent/git_providers/*`, `pr_agent/servers/*`, `pr_agent/tools/*`, `pr_agent/cli.py`, `pr_agent/config_loader.py`, `pr_agent/mosaico/*`, `pr_agent/telemetry/*`, `pr_agent/settings/*.toml` |

## 2. What PR-Agent Is

PR-Agent is an open-source, artificial-intelligence (AI) powered code review agent. It reviews pull requests, writes descriptions, suggests code improvements, answers questions, adds documentation, updates changelogs, generates labels, and checks compliance — across GitHub, GitLab, Bitbucket (cloud and server), Azure DevOps, Gitea, Gerrit, AWS CodeCommit, local checkouts, and plain diffs. It runs as a command-line tool and as webhook servers. The one idea: turn a pull request into a structured set of AI outputs through a shared pipeline of provider, prompt, token handling, and model routing.

## 3. Documentation Goal

1. Know the mental model: provider + tool + prompt + model handler, dispatched by `command2class`.
2. Know the core agent and the exact command-to-tool mapping.
3. Know the AI handler abstraction and model routing (LiteLLM, OpenAI, LangChain, cloud auth).
4. Know token handling, token budget, and the prompt fragment system.
5. Know how a pull request diff becomes context (pr_processing, git_patch_processing, file filter, language handler).
6. Know the review pipeline (merge, finding state, inline comment dedup, JSON output).
7. Know every git provider, identity provider, and secret provider.
8. Know the CLI, configuration loading, and configuration security.
9. Know every server entrypoint and its deployment target.
10. Know every tool and the prompt file it maps to.
11. Know the Mosaico agent-to-agent layer.
12. Know telemetry and observability.
13. Know packaging, CI, and tests.

## 4. Documentation Structure

```
agentic-verification/pr-agent/pr-agent/
├── exploration.md
├── spec.md
├── build.py
├── styles.css
├── markdown/   (README.md, 00 to 18)
└── html/
```

## 5. Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | markdown/README.md | TODO |
| 1 | 00-overview.md | TODO |
| 2 | 01-architecture.md | TODO |
| 3 | 02-the-core-agent.md | TODO |
| 4 | 03-ai-handlers-and-model-routing.md | TODO |
| 5 | 04-token-handling-and-prompts.md | TODO |
| 6 | 05-pr-and-patch-processing.md | TODO |
| 7 | 06-review-pipeline.md | TODO |
| 8 | 07-output-models-and-sinks.md | TODO |
| 9 | 08-git-providers.md | TODO |
| 10 | 09-identity-and-secret-providers.md | TODO |
| 11 | 10-cli-and-config.md | TODO |
| 12 | 11-servers-and-deployment.md | TODO |
| 13 | 12-tools-review-and-describe.md | TODO |
| 14 | 13-tools-code-suggestions-and-questions.md | TODO |
| 15 | 14-tools-docs-labels-and-help.md | TODO |
| 16 | 15-mosaico-a2a.md | TODO |
| 17 | 16-telemetry-and-observability.md | TODO |
| 18 | 17-packaging-ci-and-tests.md | TODO |
| 19 | 18-glossary.md | TODO |
| 20 | exploration.md | DONE |
| 21 | Grandfather review | TODO |
| 22 | Generate HyperText Markup Language (HTML) | TODO |

## 6. Build System

Run `python3 -I build.py .` inside this directory. It writes `html/`. Run `python3 -I build.py` in the parent `pr-agent` group to build the group index that links `pr-agent` and `skills`. No dependencies beyond the Python standard library.

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

A reader can explain PR-Agent's request flow, pick the right provider, tool, and configuration for a task, and trace a pull request from event to published output.

## 9. Resume Point

The exploration manifest is written and five deep-read agents are running. To continue: collect the agent reports, write markdown chapters 00 to 18, run the grandfather review, then rebuild the HyperText Markup Language (HTML).
