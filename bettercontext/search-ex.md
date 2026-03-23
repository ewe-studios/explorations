# Search-Ex - Deep Dive

## Overview

**search-ex** is a Cloudflare Worker that serves LLM-friendly, local mirrors of Hex docs (Elixir package documentation) with task maps, summaries, and extracted workflows for faster AI agent navigation.

---

## Purpose

The worker provides a proxy layer between AI agents and HexDocs.pm that:

1. Converts HTML documentation to clean Markdown
2. Adds deterministic navigation instructions
3. Provides machine-readable index files
4. Caches documentation for faster access

---

## Quick Start

### Base URL

- **Base**: `https://exag.dev`
- **Package index**: `https://exag.dev/{package}/index.json`
- **LLM index**: `https://exag.dev/{package}/llms.txt`
- **Module docs**: `https://exag.dev/{package}/{Module}.html` or `.md`
- **Guide docs**: `https://exag.dev/{package}/{guide}.html`

### Example Rewrite

Original: `https://hexdocs.pm/phoenix/overview.html`
Proxy: `https://exag.dev/phoenix/overview.html`

---

## Agent Instructions

```markdown
# Elixir Hex docs browsing
- Use https://exag.dev to browse docs for any package in your mix.exs deps.
- Example: if deps include {:phoenix, "~> 1.8"}, open https://exag.dev/phoenix/overview.html.
- Start at a package llms.txt or a module/guide page, then follow the Navigation block to `llms.txt` and `index.json`.
- Use `llms.txt` to pick a task map entrypoint before searching.
- Prefer `.md` pages for cleaner parsing; fall back to `.html` if needed.
```

---

## Implementation Plan

### Project Setup

```bash
# Cloudflare Workers + Bun setup
bun init worker
bun add wrangler
```

### Core Routing

```typescript
export default {
  async fetch(request: Request): Promise<Response> {
    const url = new URL(request.url);

    // Map to hexdocs.pm
    const hexdocsUrl = `https://hexdocs.pm${url.pathname}${url.search}`;

    // Handle llms.txt specially
    if (url.pathname.endsWith('/llms.txt')) {
      return await serveEnrichedLlmsTxt(hexdocsUrl);
    }

    // Fetch and convert HTML to Markdown
    const response = await fetch(hexdocsUrl);
    const html = await response.text();
    const markdown = await convertToMarkdown(html);

    return new Response(addInstructionHeader(markdown), {
      headers: { 'Content-Type': 'text/markdown' }
    });
  }
};
```

### Markdown Extraction

```typescript
async function convertToMarkdown(htmlUrl: string): Promise<string> {
  // For module pages, use .md endpoint directly
  const mdUrl = htmlUrl.replace('.html', '.md');
  const mdResponse = await fetch(mdUrl);

  if (mdResponse.ok) {
    return mdResponse.text();
  }

  // Fallback: fetch HTML and convert
  const htmlResponse = await fetch(htmlUrl);
  const html = await htmlResponse.text();

  // Find "Copy Markdown" link and follow it
  const markdownLink = extractMarkdownLink(html);
  if (markdownLink) {
    const mdResponse = await fetch(markdownLink);
    return mdResponse.text();
  }

  return htmlToMarkdown(html);
}
```

### Instruction Header Template

```typescript
function addInstructionHeader(markdown: string): string {
  return `# Navigation Instructions

## How to Navigate This Documentation

1. **Module Pages**: From \`llms.txt\`, find the module list. Each module has:
   - \`${'{Module}'}.md\` - Full module documentation
   - \`${'{Module}'}.html\` - HTML version
   - \`${'{Module}'}.html#summary\` - Function/type summary

2. **Function Lists**: Check the \`#summary\` anchor for quick reference

3. **Exceptions**: Listed in \`llms.txt\` under "Exceptions" section

4. **Full Markdown**: Use the "Copy Markdown" link from any HTML page

5. **Related Modules**: Follow links within Markdown content

---

${markdown}`;
}
```

### llms.txt Enrichment

```typescript
async function serveEnrichedLlmsTxt(hexdocsUrl: string): Promise<Response> {
  // Fetch upstream llms.txt
  const response = await fetch(hexdocsUrl);
  const content = await response.text();

  // Parse modules and exceptions
  const modules = parseModulesList(content);
  const exceptions = parseExceptionsList(content);

  // Build enriched Markdown
  let enriched = '# Enriched Documentation Index\n\n';

  enriched += '## Modules\n\n';
  for (const module of modules) {
    // Fetch module summary
    const moduleMd = await fetchModuleMarkdown(module.url);
    const functions = extractFunctionList(moduleMd);

    enriched += `### ${module.name}\n\n`;
    enriched += `${module.summary}\n\n`;
    enriched += `**Functions**: ${functions.join(', ')}\n\n`;
  }

  enriched += '## Exceptions\n\n';
  for (const exception of exceptions) {
    enriched += `- ${exception.name}: ${exception.summary}\n`;
  }

  return new Response(enriched, {
    headers: { 'Content-Type': 'text/markdown' }
  });
}
```

---

## Caching Strategy

```typescript
async function fetchWithCache(url: string): Promise<Response> {
  const cache = caches.default;
  let response = await cache.match(url);

  if (response) {
    return response;
  }

  response = await fetch(url);

  if (response.ok) {
    const responseToCache = response.clone();
    // Versioned docs are immutable
    if (isVersionedUrl(url)) {
      responseToCache.headers.set('Cache-Control', 'public, max-age=31536000');
    }
    await cache.put(url, responseToCache);
  }

  return response;
}
```

---

## Cloudflare Worker Configuration

### wrangler.toml

```toml
name = "search-ex"
main = "src/worker.ts"
compatibility_date = "2024-01-01"

[vars]
HEXDOCS_BASE = "https://hexdocs.pm"

[[routes]]
pattern = "exag.dev/*"
zone_name = "exag.dev"
```

### Package.json

```json
{
  "name": "search-ex",
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "wrangler dev",
    "deploy": "wrangler deploy",
    "check": "tsc --noEmit",
    "format": "prettier --write ."
  },
  "devDependencies": {
    "@cloudflare/workers-types": "^4.0.0",
    "wrangler": "^3.0.0",
    "typescript": "^5.0.0"
  }
}
```

---

## Testing

```typescript
// vitest.config.ts
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
    environment: 'miniflare'
  }
});

// src/worker.test.ts
import { describe, it, expect } from 'vitest';

describe('HexDocs Proxy', () => {
  it('converts readme.html to Markdown', async () => {
    const response = await worker.fetch('https://exag.dev/phoenix/readme.html');
    const text = await response.text();

    expect(text).toContain('# Navigation Instructions');
    expect(text).toContain('# Phoenix');
    expect(response.headers.get('Content-Type')).toBe('text/markdown');
  });

  it('enriches llms.txt', async () => {
    const response = await worker.fetch('https://exag.dev/phoenix/llms.txt');
    const text = await response.text();

    expect(text).toContain('## Modules');
    expect(text).toContain('## Exceptions');
  });
});
```

---

## Open Questions (from PLAN.md)

1. **llms.txt content**: Include all module Markdown or just summaries + function lists?
2. **JSON variant**: Should we also emit JSON for tooling?
3. **Caching aggression**: How aggressive for "latest" (non-versioned) docs?

---

## Production Rust Implementation

For a Rust-based equivalent using Cloudflare Workers (via `worker-build`):

### Architecture

```
search-ex-rs/
├── src/
│   ├── lib.rs        # Worker entry
│   ├── converter.rs  # HTML to Markdown
│   ├── parser.rs     # llms.txt parsing
│   └── cache.rs      # Caching logic
├── Cargo.toml
└── wrangler.toml
```

### Key Crates

- `worker` - Cloudflare Workers Rust bindings
- `scraper` - HTML parsing
- `pulldown-cmark` - Markdown processing
- `serde_json` - JSON handling

### Core Worker

```rust
use worker::*;

#[event(fetch)]
async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
    let url = req.url()?;

    // Handle llms.txt
    if url.path().ends_with("/llms.txt") {
        return serve_enriched_llms_txt(&url, &env).await;
    }

    // Fetch and convert
    let hexdocs_url = format!("https://hexdocs.pm{}", url.path());
    let response = Fetch::Url(hexdocs_url.parse()?).send().await?;

    if response.headers().get("Content-Type")?.starts_with("text/html") {
        let html = response.text().await?;
        let markdown = html_to_markdown(&html);
        let with_header = add_instruction_header(&markdown);

        return Response::ok(with_header)?;
    }

    Ok(response)
}
```
