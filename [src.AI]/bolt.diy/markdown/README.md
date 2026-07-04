# bolt.diy — AI-Powered Full-Stack Web Development

bolt.diy is an open-source AI-powered full-stack web development platform that runs in the browser. Generate complete web applications from natural language prompts, with 19+ LLM providers, in-browser execution via WebContainers, and one-click deployment.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/bolt.diy/` — 396 TypeScript files, MIT license

## Documents

### Foundation
| # | Document | What You'll Learn |
|---|----------|-------------------|
| 00 | [Overview](00-overview.md) | Platform capabilities, 19+ LLMs, WebContainers, deployment |
| 01 | [Architecture](01-architecture.md) | Remix framework, component tree, data flow, project structure |

### Deep Dives
| # | Document | What You'll Learn |
|---|----------|-------------------|
| 02 | [LLM Providers](02-llm-providers.md) | Provider abstraction, streaming, 19+ integrations |
| 03 | [Code Generation](03-code-generation.md) | AI prompt engineering, file generation, diff application |
| 04 | [WebContainer Runtime](04-webcontainer-runtime.md) | In-browser Node.js, npm in browser, preview system |
| 05 | [Deployment](05-deployment.md) | Netlify, Cloudflare, Git integration, Electron |

## Build HTML

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations/[src.AI]/bolt.diy
python3 build.py .
```
