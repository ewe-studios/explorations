# mirage Documentation

Unified Virtual File System for AI Agents.

## Document Index

| # | Document | Description |
|---|----------|-------------|
| 00 | [Overview](00-overview.html) | Philosophy, quick start |
| 01 | [Architecture](01-architecture.html) | VFS, dispatcher, resources |
| 02 | [Python SDK](02-python-sdk.html) | Python implementation |
| 03 | [TypeScript SDK](03-typescript-sdk.html) | TypeScript implementation |
| 04 | [Resources](04-resources.html) | Built-in resource types |
| 05 | [Commands](05-commands.html) | Command dispatch |
| 06 | [FUSE & Server](06-fuse-server.html) | FUSE and server modes |
| 07 | [Frameworks](07-frameworks.html) | Framework integrations |
| 08 | [Extending](08-extending.html) | Adding new resources |

## Quick Links

- **Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.strukto-ai/mirage/`
- **Repository:** https://github.com/strukto-ai/mirage.git
- **PyPI:** https://pypi.org/project/mirage-ai/
- **NPM:** https://www.npmjs.com/package/@struktoai/mirage-node

## What is mirage?

Mirage mounts multiple backends (S3, Google Drive, Slack, Gmail, etc.) as a single virtual filesystem:

```python
from mirage import Workspace, RAMResource, S3Resource

ws = Workspace({
    '/data':   RAMResource(),
    '/s3':     S3Resource(bucket='logs'),
})

# Use familiar bash commands
await ws.execute('ls /s3')
await ws.execute('cat /s3/config.json')
await ws.execute('cp /s3/report.csv /data/local.csv')
```

## Resources

| Resource | Protocol | Description |
|----------|----------|-------------|
| RAM | ram:// | In-memory storage |
| Disk | file:// | Local filesystem |
| S3 | s3:// | AWS S3 buckets |
| GCS | gcs:// | Google Cloud Storage |
| Redis | redis:// | Redis database |
| MongoDB | mongodb:// | MongoDB collections |
| Postgres | postgres:// | PostgreSQL |
| GitHub | github:// | GitHub repos |
| Slack | slack:// | Slack channels |
| Gmail | gmail:// | Gmail messages |
| SSH | ssh:// | Remote SSH hosts |
