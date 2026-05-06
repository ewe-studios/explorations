# Aipack -- Database Schema

Aipack uses SQLite via `rusqlite` with a custom ORM-like layer built on `modql`. The database tracks runs, tasks, logs, errors, and installation work items, all within a single file.

Source: `aipack/src/model/db/db_impl.rs` — SQLite connection wrapper
Source: `aipack/src/model/db/rt_db_setup.rs` — database initialization
Source: `aipack/src/model/model_manager.rs` — ModelManager singleton
Source: `aipack/src/model/base/crud_fns.rs` — generic CRUD operations
Source: `aipack/src/model/base/db_bmc.rs` — BMC (Business Model Controller) trait
Source: `aipack/src/model/entities/run.rs` — Run entity
Source: `aipack/src/model/entities/task.rs` — Task entity
Source: `aipack/src/model/entities/log.rs` — Log entity
Source: `aipack/src/model/entities/err.rs` — Error entity

## ModelManager

```rust
// model_manager.rs
pub struct ModelManager {
    db: Arc<Db>,  // rusqlite connection wrapped in Arc
}

// Lazy singleton for the main application
pub struct OnceModelManager {
    // tokio::sync::OnceCell<ModelManager>
}

impl OnceModelManager {
    pub fn get(&self) -> Result<&ModelManager> {
        // Initializes on first access
    }
}
```

The `OnceModelManager` provides lazy singleton initialization via `tokio::sync::OnceCell`. The SQLite connection is created once at first access and shared across all tasks via `Arc`.

## Database Setup

```rust
// rt_db_setup.rs
fn setup_db(db: &rusqlite::Connection) -> Result<()> {
    // Enable WAL mode for better concurrent read performance
    db.execute("PRAGMA journal_mode=WAL", [])?;

    // Create tables
    db.execute_batch(r#"
        CREATE TABLE IF NOT EXISTS run (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uid TEXT NOT NULL,
            label TEXT,
            parent_id INTEGER,
            ctime INTEGER NOT NULL,
            mtime INTEGER NOT NULL,
            has_task_stages INTEGER,
            has_prompt_parts INTEGER,
            start INTEGER,
            end INTEGER,
            ba_start INTEGER,
            ba_end INTEGER,
            tasks_start INTEGER,
            tasks_end INTEGER,
            aa_start INTEGER,
            aa_end INTEGER,
            end_state TEXT,
            end_err_id INTEGER,
            end_skip_reason TEXT,
            agent_name TEXT,
            agent_path TEXT,
            model TEXT,
            concurrency INTEGER,
            total_cost REAL,
            total_task_ms INTEGER,
            flow_redo_count INTEGER
        );

        CREATE TABLE IF NOT EXISTS task (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uid TEXT NOT NULL,
            run_id INTEGER NOT NULL,
            parent_id INTEGER,
            num INTEGER,
            label TEXT,
            ctime INTEGER,
            mtime INTEGER,
            has_stages INTEGER,
            input TEXT,
            input_summary TEXT,
            data_start INTEGER,
            data_end INTEGER,
            ai_start INTEGER,
            ai_end INTEGER,
            ai_gen_start INTEGER,
            ai_gen_end INTEGER,
            out_start INTEGER,
            out_end INTEGER,
            end_state TEXT,
            end_err_id INTEGER,
            end_skip_reason TEXT,
            prompt_tokens INTEGER,
            completion_tokens INTEGER,
            cache_read_tokens INTEGER,
            cache_creation_tokens INTEGER,
            reasoning_tokens INTEGER,
            total_cost REAL,
            ai_price_name TEXT
        );

        CREATE TABLE IF NOT EXISTS log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            run_id INTEGER,
            task_id INTEGER,
            ctime INTEGER,
            typ TEXT,
            content TEXT
        );

        CREATE TABLE IF NOT EXISTS err (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            run_id INTEGER,
            task_id INTEGER,
            stage TEXT,
            ctime INTEGER,
            typ TEXT,
            content TEXT
        );

        CREATE TABLE IF NOT EXISTS ucontent (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            run_id INTEGER,
            task_id INTEGER,
            typ TEXT,
            subtyp TEXT,
            data TEXT,
            ctime INTEGER,
            mtime INTEGER
        );

        CREATE TABLE IF NOT EXISTS pin (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            typ TEXT,
            subtyp TEXT,
            label TEXT,
            ctime INTEGER,
            mtime INTEGER
        );

        CREATE TABLE IF NOT EXISTS work (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            typ TEXT,
            subtyp TEXT,
            state TEXT,
            data TEXT,
            ctime INTEGER,
            mtime INTEGER
        );
    "#)?;

    Ok(())
}
```

## Entity Types

| Entity | Purpose | Key Fields |
|--------|---------|------------|
| `run` | Agent execution runs | uid, label, start/end, ba_start/end (before_all), tasks_start/end, aa_start/end (after_all), end_state, total_cost, flow_redo_count |
| `task` | Individual tasks within a run | uid, run_id, num, input, data/ai/out timing, token counts, total_cost |
| `log` | Text logs and print output | run_id, task_id, typ, content |
| `err` | Error records | run_id, task_id, stage, content |
| `ucontent` | User content (inputs/outputs) | run_id, task_id, typ, subtyp, data |
| `pin` | Pinned items/bookmarks | typ, subtyp, label |
| `work` | Installation work items | typ, subtyp, state, data |

## BMC Pattern (Business Model Controller)

```rust
// base/db_bmc.rs
pub trait DbBmc {
    const TABLE: &'static str;
    const ENTITY_TYPE: EntityType;
}

// base/crud_fns.rs
pub fn create<B: DbBmc>(mm: &ModelManager, fields: Vec<(&str, Value)>) -> Result<Id> {
    let db = mm.db();
    let field_names = fields.iter().map(|(n, _)| n).collect::<Vec<_>>();
    let sql = format!("INSERT INTO {} ({}) VALUES ({})", B::TABLE, field_names.join(", "), placeholders(len));
    db.execute(&sql, params_from_values(&fields))?;
    Ok(Id::new(db.last_insert_rowid()))
}

pub fn get<B: DbBmc, T: SqliteFromRow>(mm: &ModelManager, id: Id) -> Result<T> {
    let sql = format!("SELECT * FROM {} WHERE id = ?", B::TABLE);
    let row = db.query_one(&sql, [id.as_i64()])?;
    T::from_row(&row)
}

pub fn list<B: DbBmc, T: SqliteFromRow>(mm: &ModelManager, options: Option<ListOptions>, filter: Option<Filter>) -> Result<Vec<T>> {
    let sql = build_select_sql::<B>(options, filter);
    let rows = db.query_all(&sql, [])?;
    rows.into_iter().map(T::from_row).collect()
}
```

Each entity implements the `DbBmc` trait to associate it with a table:

```rust
// entities/run.rs
impl DbBmc for RunBmc {
    const TABLE: &'static str = "run";
    const ENTITY_TYPE: EntityType = EntityType::Run;
}

impl RunBmc {
    pub fn create(mm: &ModelManager, run_c: RunForCreate) -> Result<Id> {
        let fields = run_c.sqlite_not_none_fields();
        base::create::<Self>(mm, fields)
    }

    pub fn list_for_display(mm: &ModelManager, limit: Option<i64>) -> Result<Vec<Run>> {
        let mut options = ListOptions::from_order_bys("!id");  // descending by id
        if let Some(limit) = limit { options.limit = Some(limit); }
        Self::list(mm, Some(options))
    }

    pub fn set_end_error(mm: &ModelManager, run_id: Id, stage: Option<Stage>, error: &Error) -> Result<()> {
        // 1. Create err record
        let err_c = ErrForCreate { stage, run_id: Some(run_id), ... };
        let err_id = ErrBmc::create(mm, err_c)?;

        // 2. Update run with error reference
        let run_u = RunForUpdate { end_state: Some(EndState::Err), end_err_id: Some(err_id), .. };
        Self::update(mm, run_id, run_u)?;
    }
}
```

## Field Extraction via Macros

```rust
// entities/run.rs
#[derive(Debug, Clone, Fields, SqliteFromRow)]
pub struct Run {
    pub id: Id,
    pub uid: Uuid,
    pub label: Option<String>,
    pub ctime: EpochUs,
    pub mtime: EpochUs,
    // ...
}

// The #[derive(Fields)] macro generates sqlite_not_none_fields():
// Returns Vec<(&str, Value)> with only the non-None fields for INSERT/UPDATE
```

The `modql` derive macros generate:
- `sqlite_not_none_fields()` — extracts non-None fields for SQL operations
- `SqliteFromRow` — constructs the struct from a `rusqlite::Row`

## Entity Relationships

```
run (1) ──────┬────── (N) task
              │
              ├── (N) log
              │
              ├── (N) err
              │
              └── (N) ucontent

work (standalone — installation tracking)
pin (standalone — bookmarks)
```

## RunningState and EndState

```rust
// Derived from Run entity
impl From<&Run> for RunningState {
    fn from(value: &Run) -> Self {
        if value.end.is_some() {
            RunningState::Ended(value.end_state)  // Ok, Err, Skip
        } else if value.start.is_some() {
            RunningState::Running
        } else {
            RunningState::Waiting
        }
    }
}

enum EndState {
    Ok,
    Err,
    Skip,
}
```

The `RunningState` is derived from `Run` timestamps, not stored separately. This avoids consistency issues between the entity and its computed state.

See [Runtime System](11-runtime-system.md) for how RtModel uses these entities.
See [Run System](04-run-system.md) for how run/task records are created during execution.
