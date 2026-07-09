---
title: Herdr — Error Handling and Recovery
---

# Herdr — Error Handling and Recovery

**Herdr uses `io::Result` for system-level errors, `Option` for nullable values, and panic guards for terminal restoration.** The codebase handles errors at multiple levels: PTY I/O, terminal rendering, config validation, session restore, and server/client communication.

See [09-data-flow.md](09-data-flow.md) for data flows. This document covers error handling patterns throughout the system.

## Panic Hook and Terminal Restoration

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/main.rs:610-629`

The most critical error handling in Herdr is the panic hook, which ensures the terminal is restored even when a panic occurs:

```rust
// main.rs:610-629
let original_hook = std::panic::take_hook();
std::panic::set_hook(Box::new(move |info| {
    tracing::error!("PANIC: {info}");
    if panic_resets_modify_other_keys {
        let _ = std::io::Write::write_all(&mut io::stdout(), b"\x1b[>4;0m");
    }
    if crate::kitty_graphics::is_enabled() {
        let _ = crate::kitty_graphics::clear_all_host_graphics();
    }
    let _ = execute!(
        io::stdout(),
        PopKeyboardEnhancementFlags,
        DisableFocusChange,
        DisableBracketedPaste,
        DisableMouseCapture
    );
    ratatui::restore();
    original_hook(info);
}));
```

This hook:
1. Logs the panic via tracing
2. Resets modifyOtherKeys if enabled (prevents broken keyboard state)
3. Clears Kitty graphics placements if enabled
4. Disables all terminal enhancements (mouse, bracketed paste, focus change, keyboard enhancement)
5. Restores the raw terminal mode via `ratatui::restore()`
6. Calls the original panic hook (prints the panic message)

**Aha:** The panic hook captures the original hook with `std::panic::take_hook()` and calls it at the end. This means the standard panic message is still printed after Herdr cleans up the terminal. Without this, a panic would leave the terminal in raw mode — invisible input, no echo, broken display.

## PTY I/O Error Handling

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/pty/actor.rs`

The PTY actor handles several error conditions:

### HUP (Hang Up)

```rust
// actor.rs — read loop
fn read_once(&mut self) -> bool {
    let mut buf = [0u8; 8192];
    match libc::read(self.file.as_raw_fd(), buf.as_mut_ptr().cast(), 8192) {
        Ok(n) if n > 0 => {
            // Process bytes
        }
        Ok(0) => {
            // EOF — child exited, reader exit callback
            (self.on_reader_exit)();
            return false;
        }
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
            // Expected with non-blocking fd, continue
        }
        Err(e) if e.kind() == io::ErrorKind::Interrupted => {
            // EINTR, retry
        }
        Err(_) => {
            // Other error — likely HUP, exit
            (self.on_reader_exit)();
            return false;
        }
    }
    true
}
```

When the child process exits, the PTY master fd returns 0 (EOF) or an error. The actor calls `on_reader_exit()` which notifies the `TerminalRuntime` that the pane has died.

### Write Errors

```rust
// actor.rs:645-674
fn flush_pending_writes_once(&mut self) {
    while let Some(bytes) = self.pending_writes.front() {
        match libc::write(self.file.as_raw_fd(), bytes.as_ptr().cast(), bytes.len()) {
            Ok(n) => {
                if n < bytes.len() {
                    // Partial write — keep remainder
                    self.pending_writes[0] = bytes.split_off(n);
                    return;
                }
                self.pending_writes.pop_front();
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                return; // Will retry on next poll
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                continue; // EINTR, retry
            }
            Err(_) => {
                // Fatal error — clear pending writes
                self.pending_writes.clear();
                return;
            }
        }
    }
}
```

Partial writes are handled by keeping the unwritten portion in the queue. `WouldBlock` means the write buffer is full — the actor will retry on the next poll cycle.

## Pane Death Recovery

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/app/actions.rs:2324-2380`

When a pane's PTY process exits, the `PaneDied` event is processed:

```rust
// app/actions.rs:2324-2350
fn handle_pane_died(&mut self, pane_id: PaneId) {
    // 1. Find the workspace and tab containing the pane
    // 2. Remove the pane from the tab's layout
    // 3. If the tab has no more panes, close the tab
    // 4. If the workspace has no more tabs, close the workspace
    // 5. If the workspace is the last one, create a new default workspace
}
```

The cascade behavior ensures that:
- A single pane death doesn't crash the workspace
- The last pane in a tab closes the tab
- The last tab in a workspace closes the workspace
- The last workspace creates a new default workspace (so Herdr never shows an empty screen)

## Config Error Handling

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/config/io.rs:35-73`

Config loading errors are handled gracefully:

1. **File not found** — Use default config values (no error)
2. **TOML parse error** — Return diagnostics, use defaults for invalid sections
3. **Deserialization error** — Return diagnostics, use defaults for invalid fields
4. **Invalid keybinding** — Keep current keybinds, report diagnostic

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/app/mod.rs:1130-1147`

```rust
// app/mod.rs:1130-1147
if !invalid_section("keys") {
    match config.live_keybinds() {
        Ok(live) => {
            self.state.prefix_code = live.prefix.0;
            self.state.prefix_mods = live.prefix.1;
            self.state.keybinds = live.keybinds;
        }
        Err(keybind_diagnostics) => {
            diagnostics.extend(
                keybind_diagnostics
                    .into_iter()
                    .map(|diagnostic| format!("{diagnostic}; kept current keybinds")),
            );
        }
    }
}
```

If keybinds are invalid, the current keybinds are kept and a diagnostic message is added. The user sees the error but their current bindings continue to work.

## Session Restore Failure

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/app/mod.rs:274-352`

If session restore fails or produces no workspaces:

```rust
// app/mod.rs:285-352
let (workspaces, active, selected, ...) = if no_session {
    (Vec::new(), None, 0, ...)
} else if let Some(snap) = crate::persist::load() {
    let (ws, terminals, terminal_runtimes) = crate::persist::restore(...);
    if ws.is_empty() {
        // Restore produced no workspaces — start fresh
        logging::session_restored(0, "empty");
        (Vec::new(), None, 0, ...)
    } else {
        logging::session_restored(ws.len(), "ok");
        (ws, active, selected, ...)
    }
} else {
    // No snapshot file — start fresh
    (Vec::new(), None, 0, ...)
};
```

The fallback behavior ensures Herdr always starts with a usable state, even if the session file is corrupted or missing.

## Server Startup Errors

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/main.rs:594-602`

```rust
// main.rs:594-602
let _api_server = match api::start_server_with_capabilities(api_tx, event_hub.clone(), None) {
    Ok(server) => server,
    Err(err) if err.kind() == io::ErrorKind::AddrInUse => {
        eprintln!("error: herdr is already running");
        eprintln!("socket: {}", api::socket_path().display());
        std::process::exit(1);
    }
    Err(err) => return Err(err),
};
```

If the API socket is already in use, Herdr prints a helpful message and exits. This prevents multiple instances from conflicting.

## Handoff Error Recovery

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/server/handoff.rs` and `src/server/headless.rs:711-921`

The handoff system has multiple recovery points:

1. **Timeout during quiesce** — If `begin_handoff()` doesn't complete within the timeout, the input gate is reopened and the original server continues
2. **Version mismatch** — If the import process detects a version mismatch, it rejects the handoff and the source server continues
3. **FD transfer failure** — If `send_fds()` or `recv_fds()` fails, the handoff is aborted
4. **Restore failure** — If the import process fails to restore runtimes, it exits and the source server continues

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/server/headless.rs:711-921`

```rust
// server/headless.rs:711-921 — perform_live_handoff()
async fn perform_live_handoff(&mut self) -> io::Result<()> {
    // 1. Pause readers
    self.app.terminal_runtimes.set_handoff_readers_paused(true);

    // 2. Capture snapshot
    let snapshot = crate::persist::capture(&self.app.state, &self.app.terminal_runtimes);

    // 3. Spawn import process
    let mut import = spawn_handoff_import()?;

    // 4. Send manifest and FDs
    // If any step fails:
    self.app.terminal_runtimes.set_handoff_readers_paused(false);
    cleanup_failed_import_child(&mut import);
    return Err(/* error */);

    // 5. Wait for import to be ready
    // 6. Clean up and exit
}
```

If the handoff fails at any point, readers are resumed, the import child is cleaned up, and the original server continues serving clients.

## Nested Herdr Prevention

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/main.rs:303-330`

```rust
// main.rs:303-309
fn should_block_nested_for_env(config: &Config, herdr_env: Option<&str>) -> bool {
    !config.experimental.allow_nested && herdr_env == Some(HERDR_ENV_VALUE)
}
```

Herdr sets `HERDR_ENV=1` when spawning child processes. If a user tries to run `herdr` inside a Herdr-managed pane, the nested instance detects the environment variable and exits with an error message (unless `experimental.allow_nested = true`).

## Render Error Handling

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/app/mod.rs:817-866`

If rendering fails (e.g., terminal resize error), the error propagates up to `App::run()` which returns it to `main()`. The panic hook ensures the terminal is restored.

In headless server mode, render errors are logged but don't crash the server:

```rust
// server/headless.rs:2610-2887
fn render_and_stream(&mut self) {
    match render_virtual(&self.app.state, &self.app.terminal_runtimes, area) {
        Ok(frame) => { /* stream to clients */ }
        Err(e) => tracing::error!("render failed: {e}"),
    }
}
```

## Error Summary Table

| Error Type | Handling Strategy | Recovery |
|------------|------------------|----------|
| Panic | Panic hook restores terminal | Process exits, terminal restored |
| PTY EOF | Reader exits, PaneDied event | Pane removed, cascade close |
| PTY write error | Partial write queued, retry | Automatic retry on next poll |
| Config parse error | Section-level fallback | Invalid sections use defaults |
| Invalid keybinding | Keep current bindings | Diagnostic shown, bindings unchanged |
| Session restore failure | Start fresh with default workspace | Always starts usable |
| API socket in use | Print message, exit | User must stop existing instance |
| Handoff failure | Resume readers, cleanup | Source server continues |
| Nested Herdr | Print message, exit | User must enable allow_nested |
| Render error | Log error, continue | Server stays alive |

## Cross-References

- [01-architecture.md](01-architecture.md) — Overall system architecture
- [03-pty-management.md](03-pty-management.md) — PTY error handling
- [06-server-client.md](06-server-client.md) — Server error handling
- [09-data-flow.md](09-data-flow.md) — Error propagation in data flows

→ This is the last deep-dive document. See [README.md](README.md) for the full index.
