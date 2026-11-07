---
title: Cloudflare Source Explorations
---

# Cloudflare source explorations

This page is the shared index for explorations of Cloudflare source repositories.

| Exploration | What it covers | Start here |
|-------------|----------------|------------|
| [Cloudflare MCP server](mcp/html/README.html) | The Model Context Protocol (MCP) server for the Cloudflare Application Programming Interface (API), with a deep study of code mode | [Overview](mcp/html/00-overview.html), then [the execute tool](mcp/html/04-execute-tool.html) |

## What the MCP exploration answers

The Cloudflare MCP server exposes the whole Cloudflare API through a small set of
tools. The main question is how a model can call thousands of API methods without
loading thousands of tool definitions into its context.

The answer is code mode. The model writes code. The code runs in a sandbox. The
sandbox sends API requests through a proxy that holds the user token. The model
never sees the token, and the context holds a few tool descriptions instead of the
full catalog.
