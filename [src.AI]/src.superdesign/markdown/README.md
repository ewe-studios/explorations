# src.superdesign Documentation

A VS Code extension providing an AI design agent that generates UI mockups, components, and wireframes from natural language.

| Attribute | Value |
|-----------|-------|
| Version | 0.0.11 |
| Type | VS Code Extension |
| Language | TypeScript |
| License | MIT |
| Source | `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.superdesign/superdesign/` |
| Repository | https://github.com/superdesigndev/superdesign |

## Features

| Feature | Description |
|---------|-------------|
| Product Mock | Generate full UI screens from a single prompt |
| UI Components | Create reusable components |
| Wireframes | Explore low-fidelity layouts |
| Fork & Iterate | Duplicate and evolve designs |
| Prompt-to-IDE | Copy prompts into Cursor, Windsurf, Claude Code |

## Foundation

- **[00-overview](00-overview.md)** -- What SuperDesign is, version, features, architecture at a glance
- **[01-architecture](01-architecture.md)** -- Full dependency graph, layer diagram, module map

## Module Deep Dives

- **[02-ai-providers](02-ai-providers.md)** -- Vercel AI SDK, 4 providers, model config, Helicone proxy
- **[03-tools](03-tools.md)** -- 9 coding tools (read, write, edit, bash, glob, grep, ls, theme, multiedit)
- **[04-webview-ui](04-webview-ui.md)** -- React app, components, hooks, message protocol
- **[05-superdesign-deep](05-superdesign-deep.md)** -- Source-level: every file, every function, every type

## Cross-Cutting

- **[06-design-workflow](06-design-workflow.md)** -- The 4-step design-to-code pipeline
- **[07-canvas-panel](07-canvas-panel.md)** -- Design file preview, file watching, CSS inlining
- **[08-commands-settings](08-commands-settings.md)** -- VS Code commands, settings, configuration flows
- **[09-chat-streaming](09-chat-streaming.md)** -- CoreMessage protocol, streaming, error handling
- **[10-type-system](10-type-system.md)** -- AgentService, ExecutionContext, WebviewContext, Canvas types

## Advanced Topics

- **[11-initialization-project](11-initialization-project.md)** -- Project initialization, directory structure, rule injection, Welcome flow
- **[12-design-prompt-engineering](12-design-prompt-engineering.md)** -- The 627-line system prompt, styling rules, workflow enforcement
- **[13-build-system-testing](13-build-system-testing.md)** -- Esbuild bundling, TypeScript config, packaging, test structure
