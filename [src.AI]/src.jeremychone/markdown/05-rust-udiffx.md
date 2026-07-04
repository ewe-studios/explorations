# rust-udiffx: Unified Diff Patcher

Unified diff patcher with security policy, hunk matching, incremental apply, and file trash support.

Source: `rust-udiffx/src/` — 12+ modules.

## Module Structure

```
rust-udiffx/src/
├── lib.rs            # SecurityPolicy, apply_file_changes, exports
├── applier.rs        # apply_file_changes, ApplyPatchIncrementalData
├── apply_changes_status.rs  # Apply results
├── error.rs          # Error types
├── extract.rs        # Extract diffs
├── file_changes.rs   # FileChanges, FileDirective
├── file_directives.rs # New, Patch, Delete directives
├── files_context.rs  # load_files_context
├── fs_guard.rs       # Filesystem guards
├── patch_completer/  # Hunk matching, completion
├── prompt/           # Prompt generation (feature: prompt)
└── security_policy.rs # SecurityPolicy
```

## apply_file_changes

Source: `applier.rs`

```rust
pub fn apply_file_changes(
    base_dir: impl Into<SPath>,
    file_changes: FileChanges,
    security_policy: impl Into<SecurityPolicy>,
) -> Result<ApplyChangesStatus>
```

Applies file changes relative to `base_dir` with security policy enforcement.

## SecurityPolicy

Default policy (strict):
- Writes confined to `base_dir`
- Reads confined to `base_dir`
- No path-check bypasses

Custom policy can relax restrictions.

## FileDirective

```rust
enum FileDirective {
    New { file_path, content },
    Patch { file_path, hunks },
    Delete { file_path },
    // ...
}
```

## Cross-References

- [Overview](00-overview.md) — All 21 projects

[Next → rust-agentic](06-rust-agentic.md)
