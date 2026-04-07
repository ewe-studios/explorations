# cron/ Deep Dive Exploration

**Source Directory:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/hermes-agent/cron/`

**Status:** complete

---

## Module Overview

The `cron/` module provides a scheduled task execution system for Hermes Agent, enabling the agent to run automated tasks on schedules (cron expressions, intervals, one-shot reminders). This ~1,645 line module handles job storage, schedule parsing, execution, and output delivery.

Key features:
- **Cron expressions** - Standard 5-field cron syntax (minute hour day month weekday)
- **Interval schedules** - "every 30m", "every 2h" syntax
- **One-shot tasks** - "in 30 minutes", specific timestamps
- **Self-scheduling** - Agent can schedule its own reminders and follow-ups
- **Isolated sessions** - Jobs execute without prior conversation context
- **Auto-delivery** - Output routed to appropriate channels (Telegram, Discord, etc.)
- **File-based locking** - Prevents duplicate execution across processes

The gateway daemon calls `tick()` every 60 seconds to execute due jobs.

---

## Directory Structure

| File | Lines | Purpose |
|------|-------|---------|
| `__init__.py` | 42 | Package exports |
| `jobs.py` | 753 | Job storage and management |
| `scheduler.py` | 850 | Scheduler tick and execution |

**Total:** ~1,645 lines across 3 files

---

## Key Components

### 1. Job Storage (`jobs.py`)

Handles job CRUD operations and persistent storage.

**Storage Location:**
```python
HERMES_DIR = get_hermes_home()  # ~/.hermes/
CRON_DIR = HERMES_DIR / "cron"
JOBS_FILE = CRON_DIR / "jobs.json"
OUTPUT_DIR = CRON_DIR / "output"
```

**Job Schema:**
```python
{
    "id": "uuid4-string",
    "name": "Morning standup",
    "schedule": "0 9 * * *",  # or "every 30m", "2026-04-07T09:00"
    "prompt": "Run morning standup and summarize yesterday's progress",
    "skills": ["standup", "git-summary"],
    "model": "claude-sonnet-4-5",
    "deliver": "telegram",  # or "origin", "local", "discord:channel_id"
    "origin": {"platform": "telegram", "chat_id": "12345"},
    "created_at": "2026-04-01T10:00:00Z",
    "last_run": "2026-04-07T09:00:00Z",
    "next_run": "2026-04-08T09:00:00Z",
    "paused": False
}
```

**Key Functions:**
```python
def create_job(
    name: str,
    schedule: str,
    prompt: str,
    skills: Optional[List[str]] = None,
    model: Optional[str] = None,
    deliver: str = "local",
    origin: Optional[dict] = None,
) -> dict:
    """Create a new cron job."""

def get_job(job_id: str) -> Optional[dict]:
    """Retrieve a job by ID."""

def list_jobs() -> List[dict]:
    """List all jobs."""

def remove_job(job_id: str) -> bool:
    """Remove a job."""

def update_job(job_id: str, **updates) -> Optional[dict]:
    """Update job fields."""

def pause_job(job_id: str) -> Optional[dict]:
    """Pause a job (mark as inactive)."""

def resume_job(job_id: str) -> Optional[dict]:
    """Resume a paused job."""

def trigger_job(job_id: str) -> bool:
    """Manually trigger a job immediately."""

def get_due_jobs() -> List[dict]:
    """Get all jobs that are due for execution."""

def mark_job_run(job: dict, output: str, completed: bool) -> None:
    """Update job after execution, save output."""

def advance_next_run(job: dict) -> None:
    """Calculate and set next_run based on schedule."""
```

### 2. Schedule Parsing (`jobs.py`)

Parses various schedule formats into structured representations.

**Schedule Types:**
```python
def parse_schedule(schedule: str) -> Dict[str, Any]:
    """Parse schedule string into structured format.
    
    Returns dict with:
        - kind: "once" | "interval" | "cron"
        - For "once": "run_at" (ISO timestamp)
        - For "interval": "minutes" (int)
        - For "cron": "expr" (cron expression)
    
    Examples:
        "30m"              -> once in 30 minutes
        "2h"               -> once in 2 hours
        "every 30m"        -> recurring every 30 minutes
        "every 2h"         -> recurring every 2 hours
        "0 9 * * *"        -> cron expression (daily at 9am)
        "2026-04-07T14:00" -> once at timestamp
    """
```

**Duration Parsing:**
```python
def parse_duration(s: str) -> int:
    """Parse duration string into minutes.
    
    Examples:
        "30m" -> 30
        "2h" -> 120
        "1d" -> 1440
    """
```

### 3. Scheduler (`scheduler.py`)

Executes due jobs and handles output delivery.

**Key Function:**
```python
def tick() -> None:
    """Check for due jobs and execute them.
    
    Called every 60 seconds by the gateway daemon.
    Uses file-based lock to prevent concurrent execution.
    """
```

**Lock File:**
```python
_LOCK_DIR = _hermes_home / "cron"
_LOCK_FILE = _LOCK_DIR / ".tick.lock"
# fcntl (Unix) or msvcrt (Windows) for file locking
```

**Job Execution Flow:**
1. Acquire lock file
2. Query due jobs from `jobs.json`
3. For each due job:
   - Spawn subprocess: `hermes chat --cron-job <job_id>`
   - Capture stdout (markdown output)
   - Save output to `~/.hermes/cron/output/<job_id>/<timestamp>.md`
   - Route delivery based on `deliver` field
   - Update `last_run` and `next_run`
4. Release lock

**Delivery Resolution:**
```python
def _resolve_delivery_target(job: dict) -> Optional[dict]:
    """Resolve the concrete auto-delivery target for a cron job."""
    
    # "local" -> no delivery (output saved locally only)
    # "origin" -> back to original platform/chat where job was created
    # "telegram" -> Telegram home channel
    # "discord:channel_id" -> specific Discord channel
    # "Alice (dm)" -> resolved via channel_directory
```

**Silent Marker:**
```python
SILENT_MARKER = "[SILENT]"
# When a cron agent has nothing new to report, it can start its
# response with this marker to suppress delivery. Output is still
# saved locally for audit.
```

### 4. Delivery Target Resolution (`scheduler.py`)

Handles routing cron output to appropriate platforms.

**Delivery Formats:**
| Format | Behavior |
|--------|----------|
| `local` | No delivery, local audit only |
| `origin` | Back to creating platform/chat |
| `telegram` | Telegram home channel |
| `discord:channel_id` | Specific Discord channel |
| `slack:Alice (dm)` | Resolved via channel directory |

**Channel Resolution:**
```python
from gateway.channel_directory import resolve_channel_name
resolved = resolve_channel_name(platform_key, chat_id)
```

---

## Cron Expression Format

Standard 5-field cron syntax:

```
* * * * *
| | | | |
| | | | +---- Day of week (0-6, Sun=0)
| | | +------ Month (1-12)
| | +-------- Day of month (1-31)
| +---------- Hour (0-23)
+------------ Minute (0-59)
```

**Special Characters:**
- `*` - Any value
- `,` - List specifier (e.g., `1,3,5`)
- `-` - Range (e.g., `1-5`)
- `/` - Step values (e.g., `*/15` = every 15)

**Examples:**
| Expression | Meaning |
|------------|---------|
| `0 9 * * *` | Daily at 9:00 AM |
| `*/30 * * * *` | Every 30 minutes |
| `0 9 * * 1-5` | Weekdays at 9:00 AM |
| `0 0 1 * *` | First of every month |
| `0 */2 * * *` | Every 2 hours |

---

## Integration Points

### With Gateway (`gateway/`)
- Gateway daemon calls `tick()` every 60 seconds
- Delivery routing uses gateway platform connections
- Session isolation managed by gateway session system

### With CLI (`hermes_cli/`)
- `hermes cron` subcommand for job management
- `hermes gateway install` sets up systemd service
- Jobs executed via `hermes chat --cron-job`

### With Agent (`agent/`)
- Jobs execute in isolated AIAgent sessions
- No prior conversation context
- Skills specified in job are activated

### With Session Database
- Job outputs saved to `~/.hermes/cron/output/`
- Timestamped for audit trail
- Accessible via session search

---

## Related Files

**Individual File Explorations:**
- [scheduler.md](./cron/scheduler.md)
- [jobs.md](./cron/jobs.md)

**Related Modules:**
- [gateway/run.md](../gateway/run.md) - Gateway daemon that ticks scheduler
- [hermes_cli/cron.md](../hermes_cli/cron.md) - CLI cron commands
- [tools/cronjob_tools.md](../tools/cronjob_tools.md) - Self-scheduling tools for agents

---

*Deep dive created: 2026-04-07*
