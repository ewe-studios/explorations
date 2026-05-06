# Aipack -- Rust Equivalents

This document describes how to replicate aipack's core patterns in other languages and frameworks. Each section identifies the key design pattern and shows equivalent approaches.

## Agent Definition as Markdown

**Aipack approach:** `.aip` files combine TOML options, Lua scripts, and text prompts in a single markdown file parsed with a state machine.

**Equivalent in other languages:**

- **Python:** Use `markdown-it-py` or `mistune` to parse structured markdown. Store configuration in YAML frontmatter (Jekyll-style).
- **TypeScript:** Use `remark` with custom plugins to extract fenced code blocks by language label.
- **Go:** Use `goldmark` with AST traversal to find heading + code block pairs.

```python
# Python equivalent
import mistune
from mistune.renderers.mdx import MDXRenderer

class AgentParser:
    def parse(self, content: str) -> Agent:
        # Custom renderer that captures sections
        sections = {}
        current_section = None
        for line in content.split('\n'):
            if line.startswith('# '):
                current_section = line[2:].strip().lower()
            elif current_section and line.startswith('```'):
                # Toggle code block capture
                ...
        return Agent(sections)
```

## Lua Embedding for Scripting

**Aipack approach:** Embed Lua 5.4 via `mlua` with 30+ custom modules exposed as `aip.*` globals.

**Equivalent in other languages:**

- **Python:** Embed via `pyo3` (Python in Rust) or use `rustpython`. More commonly, use `rhai` — a Rust-native scripting language.
- **TypeScript:** Use `deno_core` (V8 embedded) or `boa` (Rust JS engine).
- **Go:** No good embedded scripting option except `go-lua` or `otto` (JavaScript).

```rust
// Rhai equivalent (Rust-native, no C dependency)
use rhai::{Engine, EvalAltResult};

let mut engine = Engine::new();
engine.register_fn("read_file", |path: &str| -> String {
    std::fs::read_to_string(path).unwrap_or_default()
});
engine.register_fn("write_file", |path: &str, content: &str| {
    std::fs::write(path, content).unwrap()
});

let result: String = engine.eval(r#"
    let content = read_file("config.json");
    write_file("output.txt", "processed: " + content);
"#)?;
```

Rhai is a compelling alternative to Lua for Rust projects — it's pure Rust, has no FFI overhead, and integrates seamlessly with Rust types.

## flume Channel-Based Event Dispatch

**Aipack approach:** A flume unbounded channel dispatches `ExecActionEvent` to the executor, which spawns each action as a tokio task.

**Equivalent in other languages:**

- **Python:** Use `asyncio.Queue` with `asyncio.create_task` for each action.
- **TypeScript:** Use `EventEmitter` or Node.js `Readable` streams with async handlers.
- **Go:** Native goroutines + channels.

```python
# Python asyncio equivalent
import asyncio

class Executor:
    def __init__(self):
        self.action_queue = asyncio.Queue()
        self.active_actions = 0

    async def start(self):
        while True:
            action = await self.action_queue.get()
            self.active_actions += 1
            asyncio.create_task(self._perform_action(action))

    async def _perform_action(self, action):
        try:
            await self._handle(action)
        finally:
            self.active_actions -= 1
            if self.active_actions == 0:
                await self._notify_done()
```

## Generation Counter Cancellation

**Aipack approach:** Custom `CancelTx`/`CancelRx` with atomic generation counter instead of `tokio::CancellationToken`.

**Equivalent in other languages:**

- **Python:** Use `asyncio.Event` with a generation counter:

```python
import asyncio
import threading

class CancellationToken:
    def __init__(self):
        self._generation = 0
        self._lock = threading.Lock()
        self._events: list[asyncio.Event] = []

    def cancel(self):
        with self._lock:
            self._generation += 1
            for event in self._events:
                event.set()

    async def wait(self, last_seen: int):
        while True:
            with self._lock:
                gen = self._generation
            if gen > last_seen:
                return gen
            event = asyncio.Event()
            self._events.append(event)
            await event.wait()
```

- **Go:** Use `context.Context` with value-based generation tracking.

## SQLite with modql ORM

**Aipack approach:** Custom BMC pattern on top of `rusqlite` with `modql` derive macros for field extraction and row construction.

**Equivalent in other languages:**

- **Python:** `sqlite3` + `dataclasses` + `attrs` for struct-like entities.
- **TypeScript:** `better-sqlite3` with TypeScript interfaces and `sql-template-tag`.
- **Go:** `database/sql` with `sqlx` for struct scanning.

```python
# Python dataclass equivalent
from dataclasses import dataclass
from typing import Optional
import sqlite3
import time

@dataclass
class Run:
    id: Optional[int] = None
    uid: str = ""
    label: Optional[str] = None
    ctime: int = 0
    start: Optional[int] = None
    end: Optional[int] = None
    end_state: Optional[str] = None

    @classmethod
    def create(cls, db: sqlite3.Connection, run: "Run") -> int:
        cursor = db.execute(
            "INSERT INTO run (uid, label, ctime) VALUES (?, ?, ?)",
            (run.uid, run.label, int(time.time() * 1_000_000))
        )
        return cursor.lastrowid

    @classmethod
    def get(cls, db: sqlite3.Connection, id: int) -> "Run":
        row = db.execute("SELECT * FROM run WHERE id = ?", (id,)).fetchone()
        return cls(**dict(row)) if row else None
```

## Pricing Calculator with Longest-Prefix Match

**Aipack approach:** Static pricing data with longest-prefix matching for model names.

**Equivalent:**

```python
def find_model_pricing(provider_data: list, model_name: str) -> Optional[ModelPricing]:
    best_match = None
    for model in provider_data:
        if model_name.startswith(model.name):
            if best_match is None or len(model.name) > len(best_match.name):
                best_match = model
    return best_match
```

This pattern is language-agnostic and appears in any system that needs flexible model name matching (e.g., LiteLLM, OpenRouter).

## Ratatui TUI Architecture

**Aipack approach:** State machine with `AppStage`, event-driven rendering, ping timer for periodic refresh.

**Equivalent in other languages:**

- **Python:** `textual` or `rich` + `prompt_toolkit` for panel-based TUIs.
- **TypeScript:** `ink` (React for terminal) or `blessed`.
- **Go:** `bubbletea` (Elm architecture for terminal).

```go
// bubbletea equivalent (Go)
type model struct {
    runs []Run
    selected int
    stage appStage
}

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg := msg.(type) {
    case tea.KeyMsg:
        switch msg.String() {
        case "down":
            m.selected++
        case "enter":
            m.stage = StageRunDetail
        case "esc":
            m.stage = StageRunList
        }
    case tickMsg:
        m.runs = refreshFromDB()
    }
    return m, nil
}
```

See [CLI Structure](01-cli-structure.md) for dispatch patterns.
See [TUI Architecture](10-tui-architecture.md) for view rendering patterns.
