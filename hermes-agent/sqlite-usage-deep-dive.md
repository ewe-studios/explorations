---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.NousResearch/hermes-agent
repository: git@github.com:NousResearch/hermes-agent.git
explored_at: 2026-03-25
---

# SQLite Usage Deep Dive

Hermes Agent uses SQLite as its persistent state store with **FTS5 full-text search**, replacing the per-session JSONL file approach. This document explores the SessionDB implementation, FTS5 integration, schema migrations, and session management patterns.

## Architecture Overview

```
hermes_state.py (SessionDB class)
       |
       +-- WAL mode for concurrent reads + single writer
       +-- FTS5 virtual table for full-text search
       +-- Schema migrations (v1-v5)
       +-- Thread-safe with locking
       |
       +-- sessions table (metadata, tokens, costs)
       +-- messages table (full conversation history)
       +-- messages_fts (FTS5 virtual table with triggers)
```

### Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **WAL mode** | Enables concurrent readers (gateway platforms) + one writer |
| **FTS5** | Fast full-text search across all session messages |
| **parent_session_id** | Compression-triggered session chaining |
| **Source tagging** | Filter by 'cli', 'telegram', 'discord', etc. |
| **Separate from RL/batch** | Trajectories stored elsewhere |

## Database Initialization

### Connection Setup

```python
# hermes_state.py

DEFAULT_DB_PATH = Path(
    os.getenv("HERMES_HOME", Path.home() / ".hermes")
) / "state.db"

class SessionDB:
    def __init__(self, db_path: Path = None):
        self.db_path = db_path or DEFAULT_DB_PATH
        self.db_path.parent.mkdir(parents=True, exist_ok=True)

        self._lock = threading.Lock()
        self._conn = sqlite3.connect(
            str(self.db_path),
            check_same_thread=False,  # WAL handles concurrency
            timeout=10.0,
        )
        self._conn.row_factory = sqlite3.Row
        self._conn.execute("PRAGMA journal_mode=WAL")
        self._conn.execute("PRAGMA foreign_keys=ON")

        self._init_schema()
```

Using `check_same_thread=False` is safe with WAL mode because:
- Multiple readers can access simultaneously
- Single writer serialized via `self._lock`
- WAL file handles write-ahead buffering

### Schema Tables

```python
SCHEMA_SQL = """
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL,
    user_id TEXT,
    model TEXT,
    model_config TEXT,
    system_prompt TEXT,
    parent_session_id TEXT,
    started_at REAL NOT NULL,
    ended_at REAL,
    end_reason TEXT,
    message_count INTEGER DEFAULT 0,
    tool_call_count INTEGER DEFAULT 0,
    input_tokens INTEGER DEFAULT 0,
    output_tokens INTEGER DEFAULT 0,
    cache_read_tokens INTEGER DEFAULT 0,
    cache_write_tokens INTEGER DEFAULT 0,
    reasoning_tokens INTEGER DEFAULT 0,
    billing_provider TEXT,
    billing_base_url TEXT,
    billing_mode TEXT,
    estimated_cost_usd REAL,
    actual_cost_usd REAL,
    cost_status TEXT,
    cost_source TEXT,
    pricing_version TEXT,
    title TEXT,
    FOREIGN KEY (parent_session_id) REFERENCES sessions(id)
);

CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    role TEXT NOT NULL,
    content TEXT,
    tool_call_id TEXT,
    tool_calls TEXT,
    tool_name TEXT,
    timestamp REAL NOT NULL,
    token_count INTEGER,
    finish_reason TEXT
);

CREATE INDEX IF NOT EXISTS idx_sessions_source ON sessions(source);
CREATE INDEX IF NOT EXISTS idx_sessions_parent ON sessions(parent_session_id);
CREATE INDEX IF NOT EXISTS idx_sessions_started ON sessions(started_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, timestamp);
"""
```

### Index Strategy

| Index | Purpose |
|-------|---------|
| `idx_sessions_source` | Filter by platform (cli/telegram/discord) |
| `idx_sessions_parent` | Trace compression/delegation chains |
| `idx_sessions_started` | Reverse chronological listing |
| `idx_messages_session` | Fast message lookup per session |

## FTS5 Full-Text Search

### Virtual Table Setup

```python
FTS_SQL = """
CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    content,
    content=messages,
    content_rowid=id
);

CREATE TRIGGER IF NOT EXISTS messages_fts_insert AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts(rowid, content) VALUES (new.id, new.content);
END;

CREATE TRIGGER IF NOT EXISTS messages_fts_delete AFTER DELETE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content)
    VALUES('delete', old.id, old.content);
END;

CREATE TRIGGER IF NOT EXISTS messages_fts_update AFTER UPDATE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content)
    VALUES('delete', old.id, old.content);
    INSERT INTO messages_fts(rowid, content) VALUES (new.id, new.content);
END;
"""
```

**Key FTS5 features:**
- `content=messages` — Links to base table (storage efficiency)
- `content_rowid=id` — Uses messages.id for rowid
- Automatic trigger-based sync on INSERT/UPDATE/DELETE

### Query Sanitization

FTS5 has special query syntax that can cause errors with raw user input:

```python
@staticmethod
def _sanitize_fts5_query(query: str) -> str:
    """Sanitize user input for safe FTS5 MATCH queries.

    FTS5 special characters: " ( ) + * { } ^ AND OR NOT

    Strategy:
    1. Preserve balanced quoted phrases ("exact phrase")
    2. Strip unmatched special characters
    3. Wrap hyphenated terms in quotes (chat-send → "chat-send")
    4. Remove dangling boolean operators at start/end
    """
    # Step 1: Extract balanced double-quoted phrases
    _quoted_parts: list = []

    def _preserve_quoted(m: re.Match) -> str:
        _quoted_parts.append(m.group(0))
        return f"\x00Q{len(_quoted_parts) - 1}\x00"

    sanitized = re.sub(r'"[^"]*"', _preserve_quoted, query)

    # Step 2: Strip remaining FTS5-special characters
    sanitized = re.sub(r'[+{}()"^]', " ", sanitized)

    # Step 3: Collapse repeated * and remove leading *
    sanitized = re.sub(r"\*+", "*", sanitized)
    sanitized = re.sub(r"(^|\s)\*", r"\1", sanitized)

    # Step 4: Remove dangling boolean operators
    sanitized = re.sub(r"(?i)^(AND|OR|NOT)\b\s*", "", sanitized.strip())
    sanitized = re.sub(r"(?i)\s+(AND|OR|NOT)\s*$", "", sanitized.strip())

    # Step 5: Wrap unquoted hyphenated terms
    sanitized = re.sub(r"\b(\w+(?:-\w+)+)\b", r'"\1"', sanitized)

    # Step 6: Restore preserved quoted phrases
    for i, quoted in enumerate(_quoted_parts):
        sanitized = sanitized.replace(f"\x00Q{i}\x00", quoted)

    return sanitized.strip()
```

### Search Implementation

```python
def search_messages(
    self,
    query: str,
    source_filter: List[str] = None,
    role_filter: List[str] = None,
    limit: int = 20,
    offset: int = 0,
) -> List[Dict[str, Any]]:
    """Full-text search with FTS5.

    Supports:
      - Simple keywords: "docker deployment"
      - Phrases: '"exact phrase"'
      - Boolean: "docker OR kubernetes"
      - Prefix: "deploy*"

    Returns matching messages with snippets and surrounding context.
    """
    query = self._sanitize_fts5_query(query)

    # Build dynamic WHERE clause
    where_clauses = ["messages_fts MATCH ?"]
    params: list = [query]

    if source_filter:
        source_placeholders = ",".join("?" for _ in source_filter)
        where_clauses.append(f"s.source IN ({source_placeholders})")
        params.extend(source_filter)

    if role_filter:
        role_placeholders = ",".join("?" for _ in role_filter)
        where_clauses.append(f"m.role IN ({role_placeholders})")
        params.extend(role_filter)

    where_sql = " AND ".join(where_clauses)
    params.extend([limit, offset])

    sql = f"""
        SELECT
            m.id, m.session_id, m.role,
            snippet(messages_fts, 0, '>>>', '<<<', '...', 40) AS snippet,
            m.content, m.timestamp, m.tool_name,
            s.source, s.model, s.started_at AS session_started
        FROM messages_fts
        JOIN messages m ON m.id = messages_fts.rowid
        JOIN sessions s ON s.id = m.session_id
        WHERE {where_sql}
        ORDER BY rank
        LIMIT ? OFFSET ?
    """

    with self._lock:
        cursor = self._conn.execute(sql, params)
        matches = [dict(row) for row in cursor.fetchall()]

        # Add surrounding context (1 message before + after)
        for match in matches:
            ctx_cursor = self._conn.execute(
                """SELECT role, content FROM messages
                   WHERE session_id = ? AND id >= ? - 1 AND id <= ? + 1
                   ORDER BY id""",
                (match["session_id"], match["id"], match["id"]),
            )
            match["context"] = [
                {"role": r["role"], "content": (r["content"] or "")[:200]}
                for r in ctx_cursor.fetchall()
            ]

    # Remove full content (snippet is enough)
    for match in matches:
        match.pop("content", None)

    return matches
```

## Schema Migrations

### Migration History

```python
def _init_schema(self):
    cursor = self._conn.cursor()
    cursor.executescript(SCHEMA_SQL)

    # Check version and run migrations
    cursor.execute("SELECT version FROM schema_version LIMIT 1")
    row = cursor.fetchone()

    if row is None:
        cursor.execute("INSERT INTO schema_version (version) VALUES (?)",
                       (SCHEMA_VERSION,))
    else:
        current_version = row["version"]

        if current_version < 2:
            # v2: add finish_reason to messages
            try:
                cursor.execute("ALTER TABLE messages ADD COLUMN finish_reason TEXT")
            except sqlite3.OperationalError:
                pass  # Column already exists
            cursor.execute("UPDATE schema_version SET version = 2")

        if current_version < 3:
            # v3: add title to sessions
            try:
                cursor.execute("ALTER TABLE sessions ADD COLUMN title TEXT")
            except sqlite3.OperationalError:
                pass
            cursor.execute("UPDATE schema_version SET version = 3")

        if current_version < 4:
            # v4: unique index on title (NULL-safe)
            try:
                cursor.execute(
                    "CREATE UNIQUE INDEX IF NOT EXISTS idx_sessions_title_unique "
                    "ON sessions(title) WHERE title IS NOT NULL"
                )
            except sqlite3.OperationalError:
                pass
            cursor.execute("UPDATE schema_version SET version = 4")

        if current_version < 5:
            # v5: token and billing expansion
            new_columns = [
                ("cache_read_tokens", "INTEGER DEFAULT 0"),
                ("cache_write_tokens", "INTEGER DEFAULT 0"),
                ("reasoning_tokens", "INTEGER DEFAULT 0"),
                ("billing_provider", "TEXT"),
                ("billing_base_url", "TEXT"),
                ("billing_mode", "TEXT"),
                ("estimated_cost_usd", "REAL"),
                ("actual_cost_usd", "REAL"),
                ("cost_status", "TEXT"),
                ("cost_source", "TEXT"),
                ("pricing_version", "TEXT"),
            ]
            for name, column_type in new_columns:
                try:
                    safe_name = name.replace('"', '""')
                    cursor.execute(f'ALTER TABLE sessions ADD COLUMN "{safe_name}" {column_type}')
                except sqlite3.OperationalError:
                    pass
            cursor.execute("UPDATE schema_version SET version = 5")

    self._conn.commit()
```

### Migration Safety Pattern

Key safety features:
- **Try/except on ALTER TABLE** — Handles idempotent re-runs
- **Version-gated** — Only runs if version < target
- **Defensive escaping** — `name.replace('"', '""')` for SQL injection safety
- **WHERE title IS NOT NULL** — Allows multiple NULL titles (unique constraint quirk)

## Session CRUD Operations

### Create Session

```python
def create_session(
    self,
    session_id: str,
    source: str,
    model: str = None,
    model_config: Dict[str, Any] = None,
    system_prompt: str = None,
    user_id: str = None,
    parent_session_id: str = None,
) -> str:
    """Create a new session. Returns session_id."""
    with self._lock:
        self._conn.execute(
            """INSERT INTO sessions (id, source, user_id, model, model_config,
               system_prompt, parent_session_id, started_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)""",
            (
                session_id,
                source,
                user_id,
                model,
                json.dumps(model_config) if model_config else None,
                system_prompt,
                parent_session_id,
                time.time(),
            ),
        )
        self._conn.commit()
    return session_id
```

### End Session

```python
def end_session(self, session_id: str, end_reason: str) -> None:
    """Mark session as ended."""
    with self._lock:
        self._conn.execute(
            "UPDATE sessions SET ended_at = ?, end_reason = ? WHERE id = ?",
            (time.time(), end_reason, session_id),
        )
        self._conn.commit()
```

### Update Token Counts

```python
def update_token_counts(
    self,
    session_id: str,
    input_tokens: int = 0,
    output_tokens: int = 0,
    model: str = None,
    cache_read_tokens: int = 0,
    cache_write_tokens: int = 0,
    reasoning_tokens: int = 0,
    estimated_cost_usd: Optional[float] = None,
    actual_cost_usd: Optional[float] = None,
    cost_status: Optional[str] = None,
    cost_source: Optional[str] = None,
    pricing_version: Optional[str] = None,
    billing_provider: Optional[str] = None,
    billing_base_url: Optional[str] = None,
    billing_mode: Optional[str] = None,
) -> None:
    """Increment token counters, backfill model if not set."""
    with self._lock:
        self._conn.execute(
            """UPDATE sessions SET
               input_tokens = input_tokens + ?,
               output_tokens = output_tokens + ?,
               cache_read_tokens = cache_read_tokens + ?,
               cache_write_tokens = cache_write_tokens + ?,
               reasoning_tokens = reasoning_tokens + ?,
               estimated_cost_usd = COALESCE(estimated_cost_usd, 0) + COALESCE(?, 0),
               actual_cost_usd = CASE
                   WHEN ? IS NULL THEN actual_cost_usd
                   ELSE COALESCE(actual_cost_usd, 0) + ?
               END,
               cost_status = COALESCE(?, cost_status),
               cost_source = COALESCE(?, cost_source),
               pricing_version = COALESCE(?, pricing_version),
               billing_provider = COALESCE(billing_provider, ?),
               billing_base_url = COALESCE(billing_base_url, ?),
               billing_mode = COALESCE(billing_mode, ?),
               model = COALESCE(model, ?)
               WHERE id = ?""",
            (
                input_tokens, output_tokens, cache_read_tokens,
                cache_write_tokens, reasoning_tokens,
                estimated_cost_usd, actual_cost_usd, actual_cost_usd,
                cost_status, cost_source, pricing_version,
                billing_provider, billing_base_url, billing_mode,
                model, session_id,
            ),
        )
        self._conn.commit()
```

**Key patterns:**
- `COALESCE(a, 0) + COALESCE(?, 0)` — Incremental addition with NULL safety
- `COALESCE(?, column)` — Set once, never overwrite
- `model = COALESCE(model, ?)` — Backfill if not already set

## Message Storage

### Append Message

```python
def append_message(
    self,
    session_id: str,
    role: str,
    content: str = None,
    tool_name: str = None,
    tool_calls: Any = None,
    tool_call_id: str = None,
    token_count: int = None,
    finish_reason: str = None,
) -> int:
    """Append message to session. Returns message row ID."""
    with self._lock:
        cursor = self._conn.execute(
            """INSERT INTO messages (session_id, role, content, tool_call_id,
               tool_calls, tool_name, timestamp, token_count, finish_reason)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)""",
            (
                session_id, role, content, tool_call_id,
                json.dumps(tool_calls) if tool_calls else None,
                tool_name, time.time(), token_count, finish_reason,
            ),
        )
        msg_id = cursor.lastrowid

        # Update counters
        num_tool_calls = len(tool_calls) if isinstance(tool_calls, list) else 1 if tool_calls else 0
        if num_tool_calls > 0:
            self._conn.execute(
                """UPDATE sessions SET message_count = message_count + 1,
                   tool_call_count = tool_call_count + ? WHERE id = ?""",
                (num_tool_calls, session_id),
            )
        else:
            self._conn.execute(
                "UPDATE sessions SET message_count = message_count + 1 WHERE id = ?",
                (session_id,),
            )

        self._conn.commit()
    return msg_id
```

### Get Messages

```python
def get_messages(self, session_id: str) -> List[Dict[str, Any]]:
    """Load all messages ordered by timestamp."""
    with self._lock:
        cursor = self._conn.execute(
            "SELECT * FROM messages WHERE session_id = ? ORDER BY timestamp, id",
            (session_id,),
        )
        rows = cursor.fetchall()

    result = []
    for row in rows:
        msg = dict(row)
        if msg.get("tool_calls"):
            try:
                msg["tool_calls"] = json.loads(msg["tool_calls"])
            except (json.JSONDecodeError, TypeError):
                pass
        result.append(msg)
    return result
```

### Get as OpenAI Format

```python
def get_messages_as_conversation(self, session_id: str) -> List[Dict[str, Any]]:
    """Load messages in OpenAI conversation format."""
    with self._lock:
        cursor = self._conn.execute(
            "SELECT role, content, tool_call_id, tool_calls, tool_name "
            "FROM messages WHERE session_id = ? ORDER BY timestamp, id",
            (session_id,),
        )
        rows = cursor.fetchall()

    messages = []
    for row in rows:
        msg = {"role": row["role"], "content": row["content"]}
        if row["tool_call_id"]:
            msg["tool_call_id"] = row["tool_call_id"]
        if row["tool_name"]:
            msg["tool_name"] = row["tool_name"]
        if row["tool_calls"]:
            try:
                msg["tool_calls"] = json.loads(row["tool_calls"])
            except (json.JSONDecodeError, TypeError):
                pass
        messages.append(msg)
    return messages
```

## Session Resolution (ID and Title)

### Resolve Session ID from Prefix

```python
def resolve_session_id(self, session_id_or_prefix: str) -> Optional[str]:
    """Resolve exact or prefixed session ID.

    Returns exact ID if exists, else tries prefix match.
    Returns None for ambiguous prefixes.
    """
    # First try exact match
    exact = self.get_session(session_id_or_prefix)
    if exact:
        return exact["id"]

    # Escape LIKE wildcards
    escaped = (
        session_id_or_prefix
        .replace("\\", "\\\\")
        .replace("%", "\\%")
        .replace("_", "\\_")
    )

    with self._lock:
        cursor = self._conn.execute(
            "SELECT id FROM sessions WHERE id LIKE ? ESCAPE '\\\\' "
            "ORDER BY started_at DESC LIMIT 2",
            (f"{escaped}%",),
        )
        matches = [row["id"] for row in cursor.fetchall()]

    if len(matches) == 1:
        return matches[0]
    return None  # Ambiguous (0 or 2+ matches)
```

### Title Management

```python
MAX_TITLE_LENGTH = 100

@staticmethod
def sanitize_title(title: Optional[str]) -> Optional[str]:
    """Validate and sanitize session title.

    - Strips leading/trailing whitespace
    - Removes ASCII control chars (0x00-0x1F, 0x7F)
    - Removes Unicode control chars (zero-width, directional overrides)
    - Collapses internal whitespace to single spaces
    - Enforces MAX_TITLE_LENGTH
    """
    if not title:
        return None

    # Remove ASCII control characters (keep whitespace for normalization)
    cleaned = re.sub(r'[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]', '', title)

    # Remove Unicode control characters
    cleaned = re.sub(
        r'[\u200b-\u200f\u2028-\u202e\u2060-\u2069\ufeff\ufffc\ufff9-\ufffb]',
        '', cleaned,
    )

    # Collapse internal whitespace
    cleaned = re.sub(r'\s+', ' ', cleaned).strip()

    if not cleaned:
        return None

    if len(cleaned) > SessionDB.MAX_TITLE_LENGTH:
        raise ValueError(
            f"Title too long ({len(cleaned)} chars, max {SessionDB.MAX_TITLE_LENGTH})"
        )

    return cleaned
```

### Resolve Title to Session ID

```python
def resolve_session_by_title(self, title: str) -> Optional[str]:
    """Resolve title to session ID, preferring latest in lineage.

    If exact title exists, returns that ID.
    If not, searches for "title #N" variants.
    If both exist, returns latest numbered variant.
    """
    # Try exact match
    exact = self.get_session_by_title(title)

    # Search for numbered variants: "title #2", "title #3"
    escaped = title.replace("\\", "\\\\").replace("%", "\\%").replace("_", "\\_")
    with self._lock:
        cursor = self._conn.execute(
            "SELECT id, title, started_at FROM sessions "
            "WHERE title LIKE ? ESCAPE '\\\\' ORDER BY started_at DESC",
            (f"{escaped} #%",),
        )
        numbered = cursor.fetchall()

    if numbered:
        return numbered[0]["id"]  # Most recent
    elif exact:
        return exact["id"]
    return None
```

### Generate Next Title in Lineage

```python
def get_next_title_in_lineage(self, base_title: str) -> str:
    """Generate next title (e.g., "my session" → "my session #2").

    Strips existing #N suffix to find base, increments highest number.
    """
    # Strip existing #N suffix
    match = re.match(r'^(.*?) #(\d+)$', base_title)
    base = match.group(1) if match else base_title

    # Find all numbered variants
    escaped = base.replace("\\", "\\\\").replace("%", "\\%").replace("_", "\\_")
    with self._lock:
        cursor = self._conn.execute(
            "SELECT title FROM sessions WHERE title = ? OR title LIKE ? ESCAPE '\\\\'",
            (base, f"{escaped} #%"),
        )
        existing = [row["title"] for row in cursor.fetchall()]

    if not existing:
        return base  # No conflict

    # Find highest number (unnumbered = #1)
    max_num = 1
    for t in existing:
        m = re.match(r'^.* #(\d+)$', t)
        if m:
            max_num = max(max_num, int(m.group(1)))

    return f"{base} #{max_num + 1}"
```

## Session Listing and Export

### List Sessions with Preview

```python
def list_sessions_rich(
    self,
    source: str = None,
    limit: int = 20,
    offset: int = 0,
) -> List[Dict[str, Any]]:
    """List sessions with preview and last_active timestamp.

    Uses correlated subqueries instead of N+2 queries.
    Returns: id, source, model, title, started_at, ended_at,
             message_count, preview, last_active
    """
    source_clause = "WHERE s.source = ?" if source else ""
    query = f"""
        SELECT s.*,
            COALESCE(
                (SELECT SUBSTR(REPLACE(REPLACE(m.content, X'0A', ' '), X'0D', ' '), 1, 63)
                 FROM messages m
                 WHERE m.session_id = s.id AND m.role = 'user' AND m.content IS NOT NULL
                 ORDER BY m.timestamp, m.id LIMIT 1),
                ''
            ) AS _preview_raw,
            COALESCE(
                (SELECT MAX(m2.timestamp) FROM messages m2 WHERE m2.session_id = s.id),
                s.started_at
            ) AS last_active
        FROM sessions s
        {source_clause}
        ORDER BY s.started_at DESC
        LIMIT ? OFFSET ?
    """
    params = (source, limit, offset) if source else (limit, offset)

    with self._lock:
        cursor = self._conn.execute(query, params)
        rows = cursor.fetchall()

    sessions = []
    for row in rows:
        s = dict(row)
        raw = s.pop("_preview_raw", "").strip()
        s["preview"] = (raw[:60] + "..." if len(raw) > 60 else "") if raw else ""
        sessions.append(s)

    return sessions
```

### Export All Sessions

```python
def export_all(self, source: str = None) -> List[Dict[str, Any]]:
    """Export all sessions with messages as list of dicts.

    Suitable for JSONL backup/analysis.
    """
    sessions = self.search_sessions(source=source, limit=100000)
    results = []
    for session in sessions:
        messages = self.get_messages(session["id"])
        results.append({**session, "messages": messages})
    return results
```

## Session Compression and Parent Chains

### Parent Session Chain

The `parent_session_id` foreign key enables:

```
Session A (original) → compressed → Session B (parent=A)
Session B → compressed → Session C (parent=B)
Session C → delegated → Session D (parent=C)
```

### Resolution to Parent

```python
def _resolve_to_parent(session_id: str) -> str:
    """Walk delegation chain to root parent."""
    visited = set()
    sid = session_id

    while sid and sid not in visited:
        visited.add(sid)
        session = db.get_session(sid)
        if not session:
            break
        parent = session.get("parent_session_id")
        if parent:
            sid = parent  # Walk up
        else:
            break

    return sid
```

This is used in `session_search_tool.py` to:
- Exclude current session lineage from search results
- Group messages by root conversation

## Session Cleanup (Pruning)

```python
def prune_sessions(self, older_than_days: int = 90, source: str = None) -> int:
    """Delete sessions older than N days.

    Only prunes ended sessions (not active ones).
    Returns count of deleted sessions.
    """
    cutoff = time.time() - (older_than_days * 86400)

    with self._lock:
        if source:
            cursor = self._conn.execute(
                """SELECT id FROM sessions
                   WHERE started_at < ? AND ended_at IS NOT NULL AND source = ?""",
                (cutoff, source),
            )
        else:
            cursor = self._conn.execute(
                "SELECT id FROM sessions WHERE started_at < ? AND ended_at IS NOT NULL",
                (cutoff,),
            )
        session_ids = [row["id"] for row in cursor.fetchall()]

        for sid in session_ids:
            self._conn.execute("DELETE FROM messages WHERE session_id = ?", (sid,))
            self._conn.execute("DELETE FROM sessions WHERE id = ?", (sid,))

        self._conn.commit()

    return len(session_ids)
```

## Thread Safety and Concurrency

```python
class SessionDB:
    def __init__(self, ...):
        self._lock = threading.Lock()
        self._conn = sqlite3.connect(
            str(self.db_path),
            check_same_thread=False,  # WAL handles this
            timeout=10.0,
        )
        self._conn.execute("PRAGMA journal_mode=WAL")
```

### Why This Works

| Pattern | Benefit |
|---------|---------|
| **WAL mode** | Multiple readers, one writer without blocking |
| **Per-method locking** | Serializes writes, prevents race conditions |
| **check_same_thread=False** | Safe with WAL — each method opens fresh cursor |
| **Row factory** | `sqlite3.Row` for dict-like access |

## Session Search Tool Integration

The `session_search` tool uses FTS5 + LLM summarization:

```python
# session_search_tool.py

def session_search(
    query: str,
    role_filter: str = None,
    limit: int = 3,
    db=None,
    current_session_id: str = None,
) -> str:
    """Search past sessions and return focused summaries."""

    # 1. FTS5 search
    raw_results = db.search_messages(
        query=query,
        role_filter=role_filter,
        limit=50,
    )

    # 2. Resolve to parent sessions (handle compression/delegation)
    for result in raw_results:
        resolved_sid = _resolve_to_parent(result["session_id"])
        # Skip current session lineage
        if current_lineage_root and resolved_sid == current_lineage_root:
            continue

    # 3. Parallel LLM summarization
    summaries = await asyncio.gather(*[
        _summarize_session(text, query, meta)
        for session_id, match_info, text, meta in tasks
    ])

    return json.dumps({
        "success": True,
        "query": query,
        "results": summaries,
        "count": len(summaries),
    })
```

## Summary

Hermes Agent's SQLite usage provides:

1. **FTS5 Full-Text Search** — Fast keyword search across all sessions with query sanitization
2. **Schema Migrations** — Version-gated, idempotent ALTER TABLE with defensive escaping
3. **WAL Mode Concurrency** — Thread-safe multi-reader/single-writer pattern
4. **Parent Session Chains** — Compression and delegation tracking via `parent_session_id`
5. **Token/Cost Tracking** — Incremental counters with COALESCE patterns
6. **Title Resolution** — Prefix matching, numbered lineages, uniqueness constraints
7. **Rich Listing** — Correlated subqueries for preview/last_active without N+1
8. **Export/Prune** — JSONL backup and age-based cleanup

The FTS5 integration with automatic triggers and the parent_session_id chain pattern enable powerful search and compression features while maintaining data integrity through WAL mode.
