# Mirage — Unified Virtual File System for AI Agents

Mounts 50+ services (S3, Slack, Gmail, GitHub, databases...) as a single filesystem. AI agents use standard Unix commands — `cat`, `grep`, `ls`, `find` — across all backends.

## Documents

| # | Document | What You'll Learn |
|---|----------|-------------------|
| 00 | [Overview](00-overview.md) | VFS architecture, 4-layer design, sandboxed in-process shell, 50+ backends |
| 11 | [Snapshot & Replay](11-snapshot-replay.md) | Workspace serialization, drift detection, revision pins, copy() |
| 12 | [FUSE & Native](12-fuse-native.md) | FUSE integration, native_exec, virtual vs native execution |
