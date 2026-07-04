# bolt.diy: Overview

AI Agent web app built with Remix + Vite. Supports multiple AI providers (OpenAI, Anthropic, Google). Deployable on Cloudflare Pages with wrangler.

## Architecture

```
app/
├── entry.client.tsx    # Client entry
├── entry.server.tsx    # Server entry
├── root.tsx            # Root layout
├── lib/
│   ├── crypto.ts       # Crypto utilities
│   ├── fetch.ts        # Fetch utilities
│   └── security.ts     # Security utilities
└── routes/
    ├── _index.tsx      # Home page
    ├── api.chat.ts     # Chat API
    ├── api.enhancer.ts # Prompt enhancer
    ├── api.github-*.ts # GitHub integration
    ├── api.bug-report.ts # Bug reports
    └── api.configured-providers.ts # Provider config
```

## API Routes

| Route | Purpose |
|-------|---------|
| `/api/chat` | Chat with AI |
| `/api/enhancer` | Prompt enhancement |
| `/api/github-*` | GitHub integration (branches, stats, templates) |
| `/api/bug-report` | Bug reporting |
| `/api/check-env-key` | Environment key validation |
| `/api/export-api-keys` | API key export |
| `/api/git-proxy` | Git proxy |
| `/api/git-info` | Git information |
| `/api/configured-providers` | Configured AI providers |

## Deployment

Deployed via `wrangler pages deploy` on Cloudflare Pages. Supports Docker builds.
