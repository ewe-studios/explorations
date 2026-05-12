# copyparty Documentation

Comprehensive documentation for [copyparty](https://github.com/9001/copyparty), a portable HTTP file server.

## Document Index

### Foundation

| Document | Description |
|----------|-------------|
| [00-overview.md](00-overview.md) | Project overview, philosophy, architecture at a glance |
| [01-architecture.md](01-architecture.md) | Module dependency graph, layers, component relationships |

### Core Systems

| Document | Description |
|----------|-------------|
| [02-http-handlers.md](02-http-handlers.md) | HTTP request handling, routing, response generation |
| [03-authentication.md](03-authentication.md) | Authentication system, VFS permissions, access control |
| [04-file-operations.md](04-file-operations.md) | Uploads, downloads, directory operations |

### Features

| Document | Description |
|----------|-------------|
| [05-media-streaming.md](05-media-streaming.md) | Thumbnails, media transcoding, streaming |
| [06-web-ui.md](06-web-ui.md) | Frontend JavaScript, UI components |

### Infrastructure

| Document | Description |
|----------|-------------|
| [07-configuration.md](07-configuration.md) | Configuration system, CLI arguments |
| [08-plugins.md](08-plugins.md) | Plugin architecture, broker system |
| [09-data-flow.md](09-data-flow.md) | End-to-end request flows, sequence diagrams |

## Quick Navigation

- **Getting Started**: Start with [00-overview.md](00-overview.md)
- **Architecture**: See [01-architecture.md](01-architecture.md)
- **Understanding HTTP**: Read [02-http-handlers.md](02-http-handlers.md)
- **Security**: Check [03-authentication.md](03-authentication.md)
- **How Uploads Work**: See [04-file-operations.md](04-file-operations.md) and [09-data-flow.md](09-data-flow.md)

## Source Information

- **Source**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.copyparty/copyparty`
- **Repository**: https://github.com/9001/copyparty
- **Language**: Python 2.7+ / 3.3+
- **License**: MIT

## Documentation Standards

All documents follow the [markdown directive](/home/darkvoid/Boxxed/@dev/repo-expolorations/markdown_engineering/documentation_directive.md):

- Source-verified code snippets
- Mermaid diagrams for visualizing flows
- File path references to source code
- "Aha moments" highlighting design insights

---

*Generated for copyparty source code exploration*
