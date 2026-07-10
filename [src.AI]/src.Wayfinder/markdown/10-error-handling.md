---
title: Error Handling — Exit Codes, Error Types, and Recovery
---

# Error Handling — Exit Codes, Error Types, and Recovery

RAC has a consistent, three-valued exit code contract across all 32 subcommands. Understanding this contract is essential for scripting, CI integration, and debugging.

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Wayfinder/rac-core/` — `rac/cli.py`, `rac/errors.py`, `rac/services/*.py`

**Aha: Exit 0 includes "valid failure" outcomes.** An empty corpus exits 0 (not 1), a search with no matches exits 0, the explorer quitting by the user exits 0, and `--fail-on none` always exits 0. This is because these are valid operational states, not errors. Exit 1 is reserved for actual findings (validation errors, broken references, gate failures). This distinction matters in CI: `rac validate empty-dir/` should not fail the build.

**Aha: The CLI never exits 2 for a finding — only for usage/IO errors.** Exit 2 is exclusively for argument errors (missing required flags), file-not-found, not-a-directory, malformed config, and similar operational problems. A validation failure (exit 1) means the corpus has issues; a usage error (exit 2) means the command was invoked incorrectly. This lets scripts distinguish "the corpus is broken" from "I ran the command wrong."

## Exit Code Contract

Source: `rac-core/src/rac/cli.py:42-79` — docstring of `cli.py`

| Exit Code | Meaning | Examples |
|-----------|---------|----------|
| 0 | Success | validate passes, inspect reports Unknown, relationships found or not, find with no matches, explorer quit by user, export produced (even empty corpus), watchkeeper with `--fail-on none`, telemetry status shown, skill installed |
| 1 | Finding made | validate errors found, stats has no valid artifacts, relationships broken/ambiguous/self-referencing, review priority 1-2 issues, doctor validation/relationship error, eval `--check` gate fired, watchkeeper review recommended or warning found, rename refused (unknown/ambiguous/colliding) |
| 2 | Usage/IO error | file not found, not a directory, unsupported type, refuse-to-overwrite, missing output directory, repository not initialized, invalid repository key, explorer extra not installed, MCP `--root` not a directory, unknown skill name |

## Error Types by Command

### validate

- **Exit 0:** All artifacts valid (or corpus empty).
- **Exit 1:** Structural errors found (missing required sections, malformed requirements) or, with `--corpus`, broken corpus references (retired/missing decisions).
- **Exit 2:** File not found, `--corpus` on directory target, `--sarif` on single file.

Source: `rac-core/src/rac/cli.py:208-262` — `cmd_validate()`

```python
def cmd_validate(args: argparse.Namespace) -> int:
    if args.file != "-" and Path(args.file).is_dir():
        result = validate_directory(args.file, recursive=not args.top_level)
        # Output rendering...
        return EXIT_OK if result.ok else EXIT_VALIDATION_FAILED
    # Single file or stdin...
    if corpus is not None:
        corpus_result = validate_stdin_against_corpus(product, corpus, source_path=source_path)
        return EXIT_OK if corpus_result.ok else EXIT_VALIDATION_FAILED
    issues = validate_product(product, start)
    return EXIT_VALIDATION_FAILED if has_errors(issues) else EXIT_OK
```

### relationships --validate

- **Exit 0:** All references resolve.
- **Exit 1:** Broken, ambiguous, self-referencing, or duplicate references found.
- **Exit 2:** Path not found, `--sarif` without `--validate`.

### doctor

- **Exit 0:** No validation or relationship-integrity errors (orphan, hub, and injection findings are warnings only).
- **Exit 1:** Validation or relationship-integrity error found.
- **Exit 2:** Directory not found.

**Aha: Doctor's findings are severity-graded.** Validation and relationship errors are errors (exit 1), but orphan artifacts, high-fan-out hubs, and injection-style content heuristics are warnings (exit 0). This means `rac doctor` is a diagnostic tool, not a gate — it reports everything but only fails on actual integrity issues.

Source: `rac-core/src/rac/cli.py:543-562` — `cmd_doctor()`

### gate

- **Exit 0:** No blocking findings under the corpus enforcement policy.
- **Exit 1:** Blocking findings found (advisory findings annotate but never fail).
- **Exit 2:** Directory not found, malformed `.rac/config.yaml`.

### watchkeeper

- **Exit 0:** No changes requiring attention under `--fail-on` policy, or `--fail-on none`.
- **Exit 1:** Review recommended (`--fail-on error`, default) or warning found (`--fail-on warning`).
- **Exit 2:** Revision unknown, directory not in git repository.

### eval --check

- **Exit 0:** Gate PASS — all metrics above floors and within tolerance of baseline.
- **Exit 1:** Gate FAILED — hard-negative violation, metric below floor, or metric below baseline minus tolerance.
- **Exit 2:** Baseline/config/query file missing or malformed.

## Exception Hierarchy

Source: `rac-core/src/rac/errors.py` — error types

RAC defines typed exception classes for different failure domains:

```python
class RacError(Exception):
    """Base class for RAC errors."""

class RacNotFoundError(RacError):
    """Artifact or resource not found."""

class RacExecError(RacError):
    """rac binary not found (SDK-specific)."""

class RacOutputError(RacError):
    """Unexpected output from rac binary (SDK-specific)."""
```

Service-level exceptions are raised for operational errors:

- **`TemplateNotFound`**: Requested artifact type has no canonical template.
- **`OutputPathExists`**: Target path already exists (never overwritten).
- **`OutputDirectoryMissing`**: Parent directory of output path does not exist.
- **`MissingRepositoryConfig`**: `.rac/config.yaml` not found.
- **`MalformedRepositoryConfig`**: `.rac/config.yaml` is unreadable or invalid.
- **`IdGenerationExhausted`**: Entropy source failed during ID generation.
- **`ConversionError`**: Document conversion failed (recognized format, but conversion error).
- **`UnsupportedDocument`**: Document format not supported (needs extra dependency).
- **`RepositoryKeyConflict`**: Established key differs from requested.
- **`InvalidRepositoryKey`**: Key syntax invalid.
- **`NotAGitRepository`**: Directory is not a git repository.
- **`RevisionNotFound`**: Git revision does not exist.
- **`SkillNotFound`**: Requested skill name not bundled.
- **`SkillFileExists`**: Target skill file already exists (never overwritten).
- **`HookNotFound`**: Requested hook style not bundled.
- **`HookFileExists`**: Target hook file already exists (never overwritten).
- **`NotAGitWorkTree`**: Directory is not a git work tree.
- **`CorpusNotEmpty`**: Corpus already has artifacts (quickstart refused).
- **`ExplorerUnavailable`**: `textual` extra not installed.
- **`PortalShellMissing`**: Vendored Portal shell not found.
- **`PortalSeamMissing`**: Vendored Portal seam not found.

## BrokenPipeError Handling

Source: `rac-core/src/rac/cli.py:2204-2212`

```python
except BrokenPipeError:
    # The downstream consumer closed the pipe (e.g. `rac export … | head`):
    # die quietly instead of dumping a traceback. Pointing stdout's fd at
    # devnull absorbs the interpreter's exit-time flush of the dead pipe,
    # which would otherwise print "Exception ignored" noise to stderr.
    outcome = usage.OUTCOME_ERROR
    devnull = os.open(os.devnull, os.O_WRONLY)
    os.dup2(devnull, sys.stdout.fileno())
    return 1
```

This handler prevents noisy tracebacks when the downstream consumer of a pipe closes early (e.g., `rac export | head -5`). The stdout file descriptor is redirected to `/dev/null` so the Python interpreter's exit-time flush doesn't print "Exception ignored in sys.stdout.flush" to stderr.

## Refusal vs. Error Pattern

RAC distinguishes between **refusals** (valid inputs that produce a negative result) and **errors** (invalid inputs or operational failures):

| Scenario | Exit Code | Category |
|----------|-----------|----------|
| `rac validate` finds errors | 1 | Finding |
| `rac rename` OLD not found | 1 | Refusal |
| `rac rename` NEW collides | 1 | Refusal |
| `rac resolve` ID not found | 1 | Refusal |
| `rac skill install` file exists | 1 | Refusal |
| `rac hook install` file exists | 1 | Refusal |
| `rac quickstart` corpus not empty | 1 | Refusal |
| `rac eval --check` below baseline | 1 | Gate failure |
| `rac validate` file not found | 2 | Usage error |
| `rac rename` not a directory | 2 | Usage error |
| `rac export --out` without --html/--okf | 2 | Usage error |

**Aha: Refusals exit 1, not 2.** A refused rename (unknown OLD, colliding NEW) exits 1 because the command was used correctly — it's the corpus that rejected the operation. This is distinct from a usage error (exit 2) where the command arguments themselves are wrong. The distinction matters for scripting: exit 1 means "try again with different inputs," exit 2 means "fix your command."

Source: `rac-core/src/rac/cli.py:477-519` — `cmd_rename()`

```python
if not plan.ok:
    # Every refusal (unknown/ambiguous OLD, invalid/colliding NEW,
    # filename-only alias) leaves the corpus untouched and exits 1 — the
    # rename was rejected. EXIT_USAGE (2) is reserved for argument/IO errors
    # like "not a directory" above, so a refused rename stays distinguishable
    # from a misused command.
    return EXIT_VALIDATION_FAILED
```

## MCP Error Handling

Source: `rac-core/src/rac/mcp/errors.py` — error serialization

The MCP server never raises protocol exceptions for failed lookups. Instead, it returns structured error data in the JSON response body:

```python
def from_resolution(result: ResolutionResult) -> dict:
    if result.outcome == OUTCOME_NOT_FOUND:
        return {"error": "not_found", "id": result.query}
    if result.outcome == OUTCOME_DUPLICATE:
        return {"error": "duplicate", "id": result.query, "paths": result.duplicate_paths}
    return {}
```

This means an AI agent receiving an error response can recover programmatically — parsing the JSON body is simpler than catching and handling exceptions. The error types are: `not_found`, `duplicate`, and `unreadable`.

## Telemetry Failure Handling

Source: `rac-core/src/rac/usage.py` — `record_command()`

Telemetry recording is silent-fail: if the log file cannot be written, the command still succeeds. This ensures telemetry never alters output or exit codes (ADR-032):

```python
def record_command(command: str, outcome: str, duration_ms: int) -> None:
    if not consent_record.load_consent().share_usage:
        return
    try:
        _append_log_entry(command, outcome, duration_ms)
    except OSError:
        pass  # silent fail — telemetry must never break the CLI
```

## Cross-References

- [RAC Architecture](01-architecture.md) — exit code constants, CLI dispatch
- [Data Flow End-to-End](09-data-flow.md) — how errors flow through each layer
- [RAC SDK](06-rac-sdk.md) — SDK error types and exit code handling
- [Proofkeeper Autonomous QA](03-proofkeeper.md) — proofkeeper exit codes (0/1/2/3)
- [Wayfinder Router](04-wayfinder-router.md) — gateway error handling (upstream failures, circuit breakers)

## Next Document

All documentation for Wayfinder/RAC is now complete. Review the [Overview](00-overview.md) for the full index, or explore any deep-dive document linked above.
