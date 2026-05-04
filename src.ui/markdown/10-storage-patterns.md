# OpenUI -- Storage Patterns

OpenUI and OpenClaw use three storage backends: JSON files for application data (apps, artifacts, notifications), SQLite for queryable data (agent tools, exec proxy), and localStorage for client settings. Each has specific limits, validation rules, and durability guarantees.

**Aha:** The notification store uses atomic writes via temp file + rename: it writes to a `.tmp` file first, then `fs.rename()` to the final path. On POSIX systems, `rename()` is atomic — either the old content or the new content is visible, never a partial write. This prevents corrupted notification files if the process crashes during write. The same pattern is used in production databases (Write-Ahead Logging) but here it's applied to a simple JSON file.

Source: `openclaw-ui/packages/claw-plugin/src/index.ts` — store implementations

## AppStore

Location: `{stateDir}/plugins/openclaw-ui/apps/{id}.json`

```json
{
  "id": "app-abc123",
  "title": "My App",
  "createdAt": "2026-05-04T10:00:00Z",
  "versions": [
    { "content": "...", "timestamp": "...", "source": "..." },
    { "content": "...", "timestamp": "...", "source": "..." }
  ]
}
```

Operations:
- **Create**: Generate UUID, write JSON file with initial version
- **Update**: Append to versions array (each entry has `{content, timestamp, source}`)
- **Restore**: Revert to an earlier version content
- **Max versions**: 25 — older versions are pruned on update

**Aha:** The version history is append-only. Restoring a previous version doesn't delete the newer versions — it just changes `activeVersion`. This means you can "undo the undo" by restoring to a newer version. The max 25 limit prevents unbounded growth.

## ArtifactStore

Location: `{stateDir}/plugins/openclaw-ui/artifacts/{id}.json`

```json
{
  "id": "artifact-xyz789",
  "kind": "markdown",
  "versions": [
    { "content": "# Title\n\nBody...", "timestamp": "...", "source": "..." }
  ]
}
```

Operations:
- **Create**: Generate UUID, write JSON file
- **Update**: Append version, keep max 25
- **List with kind filter**: Filter by `kind` field (currently only `markdown`)
- **Get**: Retrieve by ID

The kind field enables future expansion — when artifact types beyond markdown are added (images, spreadsheets), the list operation can filter by kind.

## UploadStore

Location: `{stateDir}/plugins/openclaw-ui/uploads/{id}.{ext}` + `index.json`

| Limit | Value | Rationale |
|-------|-------|-----------|
| Per file | 25 MB | Prevents single large uploads from filling disk |
| Per session | 200 MB | Prevents unbounded session growth |
| MIME validation | Yes | Prevents executable uploads |

Files are stored with their original extension. The `index.json` tracks metadata:

```json
{
  "entries": [
    { "id": "upload-abc", "name": "photo.jpg", "size": 1234567, "mimeType": "image/jpeg", "createdAt": "..." }
  ]
}
```

**Aha:** The comment in the code explicitly states "anything larger should go through a proper object store." The UploadStore is designed for small file attachments in chat sessions, not as a general-purpose file storage. The 25MB per-file limit is intentional — it catches the common case (photos, PDFs, documents) while preventing abuse.

## NotificationStore

Location: `{stateDir}/plugins/openclaw-ui/notifications/notifications.json`

| Property | Value |
|----------|-------|
| Format | Single JSON array |
| Max items | 400 |
| Write strategy | Atomic (temp file + rename) |
| Deduplication | Yes (dedupe keys) |

Atomic write process:
```typescript
// 1. Read existing notifications
const existing = JSON.parse(fs.readFileSync(path));

// 2. Add new notification (with dedupe key check)
if (!existing.some(n => n.dedupeKey === newNotification.dedupeKey)) {
  existing.push(newNotification);
}

// 3. Prune to max 400
while (existing.length > 400) existing.shift();

// 4. Atomic write: write to temp, then rename
const tmpPath = path + '.tmp';
fs.writeFileSync(tmpPath, JSON.stringify(existing));
fs.renameSync(tmpPath, path);  // Atomic on POSIX
```

**Aha:** The dedupe key prevents duplicate notifications when the same event is reported multiple times (e.g., a cron job that runs every minute and produces the same result). Without dedup, the notification store would fill with identical entries.

## Client Storage (localStorage)

Location: Browser `localStorage`, key `claw-settings-v1`

```json
{
  "gatewayUrl": "ws://localhost:3000",
  "token": "auth-token-...",
  "deviceToken": "device-uuid-..."
}
```

Client settings are never synced to the server. They survive browser restart but are cleared if the user clears browser data.

## SQLite Storage

Location: `{stateDir}/plugins/openclaw-ui/db/{namespace}.sqlite`

- **Per-namespace isolation**: Each namespace (session, user, team) has its own SQLite file
- **Read-only queries**: `db_query` tool uses read-only SQLite connection (no `INSERT`, `UPDATE`, `DELETE`)
- **Write operations**: `db_execute` tool allows writes (with validation)
- **No schema management**: Applications create tables as needed via `db_execute`

## Comparison

| Store | Format | Max Size | Atomic Writes | Deduplication |
|-------|--------|----------|--------------|---------------|
| AppStore | JSON files | Unlimited (25 versions per app) | No | No |
| ArtifactStore | JSON files | Unlimited (25 versions per artifact) | No | No |
| UploadStore | Binary + JSON index | 25MB per file, 200MB per session | No | No |
| NotificationStore | Single JSON array | 400 items | Yes (temp + rename) | Yes (dedupe key) |
| SQLite | SQLite database | Limited by disk | Yes (WAL mode) | No |
| localStorage | Browser key-value | ~5-10MB per origin | Yes (browser-managed) | No |

See [OpenClaw Plugin](08-openclaw-plugin.md) for how stores are used.
See [Gateway Socket](09-gateway-socket.md) for client settings storage.
See [Production Patterns](12-production-patterns.md) for broader storage considerations.
