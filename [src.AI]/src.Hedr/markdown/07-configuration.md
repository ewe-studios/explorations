---
title: Herdr — Configuration System
---

# Herdr — Configuration System

**Herdr uses a TOML configuration file at `~/.config/herdr/config.toml` with 12 sections, live reload, and per-section validation.** Config options control themes, keybindings, terminal behavior, UI layout, notifications, experiments, and more.

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/config/`, `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/config.rs`

See [01-architecture.md](01-architecture.md) for the overall architecture. This document covers the configuration system end-to-end.

## Config File Location

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/config/io.rs:87-92`

| Precedence | Source | Path |
|------------|--------|------|
| 1 (highest) | `HERDR_CONFIG_PATH` env var | Any path |
| 2 | XDG config dir | `~/.config/herdr/config.toml` |

The config file is a TOML document. Herdr prints the default config with `herdr --default-config`.

## Config Sections

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/config/model.rs:216-450`

```rust
// config/model.rs:216-230
pub struct Config {
    pub onboarding: Option<bool>,
    pub theme: ThemeConfig,
    pub terminal: TerminalConfig,
    pub update: UpdateConfig,
    pub keys: KeysConfig,
    pub ui: UiConfig,
    pub worktrees: WorktreesConfig,
    pub session: SessionConfig,
    pub remote: RemoteConfig,
    pub experimental: ExperimentalConfig,
    pub advanced: AdvancedConfig,
}
```

### [theme] Section

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/config/theme.rs`

```toml
[theme]
# Built-in themes: catppuccin, terminal, tokyo-night, dracula, nord,
#                  gruvbox, one-dark, solarized, kanagawa, rose-pine, vesper
name = "catppuccin"

# Override individual color tokens
[theme.custom]
# panel_bg = "reset"
# accent = "#f5c2e7"
```

**18 built-in themes:** catppuccin, catppuccin_latte, terminal, tokyo_night, tokyo_night_day, dracula, nord, gruvbox, gruvbox_light, one_dark, one_light, solarized, solarized_light, kanagawa, kanagawa_lotus, rose_pine, rose_pine_dawn, vesper.

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/app/state.rs:899`

Color parsing supports:
- Hex: `#rrggbb`
- Named: `red`, `green`, `cyan`, etc.
- RGB: `rgb(r,g,b)`
- Special: `reset` (use terminal default)

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/config/theme.rs:47-111`

### [terminal] Section

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/config/model.rs:160-169`

```toml
[terminal]
# Executable for new interactive panes (empty = $SHELL, then /bin/sh)
default_shell = ""

# Shell mode: "auto" (login on macOS), "login", or "non_login"
shell_mode = "auto"

# CWD policy: "follow" (inherit source), "home" ($HOME), "current" (Herdr's dir), or fixed path
new_cwd = "follow"
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `default_shell` | String | `""` (→ `$SHELL`) | Shell executable |
| `shell_mode` | `ShellModeConfig` | `auto` | Login vs non-login shell |
| `new_cwd` | `NewTerminalCwdConfig` | `follow` | CWD for new panes/tabs/workspaces |

### [update] Section

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/config/model.rs:28-40`

```toml
[update]
# Update channel: "stable" or "preview"
channel = "stable"
```

### [keys] Section

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/config/keybinds.rs`

```toml
[keys]
prefix = "ctrl+b"           # Prefix key for prefix mode
help = "prefix+?"           # Show keybind help
settings = "prefix+s"       # Open settings
detach = "prefix+q"         # Detach from server
new_tab = "prefix+c"        # New tab
split_vertical = "prefix+v"  # Vertical split
split_horizontal = "prefix+-" # Horizontal split
close_pane = "prefix+x"     # Close focused pane
zoom = "prefix+z"           # Toggle zoom/fullscreen
# ... many more

# Custom commands
[[keys.command]]
key = "prefix+alt+g"
type = "pane"
command = "lazygit"
```

**Aha:** The keybinding system supports both prefix-mode actions (require pressing the prefix key first) and direct keybindings (no prefix needed). Custom commands can be `shell` type (runs detached) or `pane` type (opens a temporary pane). The binding registry (`BindingRegistry`) detects conflicts at config load time and reports diagnostics.

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/config/keybinds.rs:318-367`

Key syntax:
- Plain keys: `a`, `1`, `?`
- Modifiers: `ctrl+`, `shift+`, `alt+`, `cmd+`/`super+`, `meta+`, `hyper+`
- Special keys: `enter`, `tab`, `esc`, `left`, `right`, `up`, `down`, `f1`..`f12`
- Indexed ranges: `prefix+1..9` (binds 1-9), `prefix+alt+1..9`

### [ui] Section

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/config/model.rs:364-426`

```toml
[ui]
sidebar_width = 26           # Default sidebar width
sidebar_min_width = 18       # Minimum sidebar width
sidebar_max_width = 36       # Maximum sidebar width
mobile_width_threshold = 64  # Terminal width for mobile layout
mouse_capture = true         # Capture mouse for Herdr UI
redraw_on_focus_gained = true # Full redraw when terminal regains focus
mouse_scroll_lines = 3       # Scroll lines per mouse wheel notch
confirm_close = true         # Confirm before closing workspace
prompt_new_tab_name = true   # Prompt for tab name on creation
show_agent_labels_on_pane_borders = false  # Show agent labels
agent_panel_scope = "all"    # "current" or "all"
accent = "cyan"              # Accent color for highlights
right_click_passthrough_modifier = ""  # Modifier for passthrough
```

### [ui.toast] Section

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/config/model.rs:44-50`

```toml
[ui.toast]
# off = disable pop-up notifications
# herdr = show top-right in-app toasts
# terminal = ask the outer terminal to show a desktop notification
# system = ask the OS notification service directly
delivery = "off"
```

### [ui.sound] Section

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/config/sound.rs`

```toml
[ui.sound]
enabled = true
# path = "sounds/notification.mp3"
# done_path = "sounds/done.mp3"
# request_path = "sounds/request.mp3"

# Per-agent overrides: default | on | off
[ui.sound.agents]
droid = "off"
```

Sound paths are resolved relative to the config file's directory.

### [session] Section

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/config/model.rs:171-185`

```toml
[session]
# Resume AI agent panes into native conversation sessions on restore
resume_agents_on_restore = true
```

### [remote] Section

```toml
[remote]
# Whether herdr manages the SSH config for herdr --remote bridge
manage_ssh_config = true
```

### [experimental] Section

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/config/model.rs:450-600`

```toml
[experimental]
allow_nested = false                    # Allow herdr inside herdr
kitty_graphics = false                  # Kitty graphics protocol
pane_history = false                    # Save pane screen history
switch_ascii_input_source_in_prefix = false  # macOS IME workaround
reveal_hidden_cursor_for_cjk_ime = false     # CJK IME cursor visibility
cjk_ime_agents = []                     # Agent allowlist for cursor reveal
cjk_ime_cursor_shape = "steady_block"   # Cursor shape when revealed
```

### [advanced] Section

```toml
[advanced]
# Maximum scrollback buffer size per pane (bytes)
scrollback_limit_bytes = 10000000
```

### [worktrees] Section

```toml
[worktrees]
directory = "~/.herdr/worktrees"
```

## Config Loading

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/config/io.rs:35-73`

```rust
// config/io.rs:35-73
impl Config {
    pub fn load() -> LoadedConfig {
        // 1. Read config file from path
        // 2. Parse TOML
        // 3. Deserialize into Config struct
        // 4. Collect diagnostics for invalid sections
        // 5. Return LoadedConfig { config, diagnostics }
    }
}
```

## Live Config Reload

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/app/mod.rs:1084-1286`

Config can be reloaded at runtime via `prefix+shift+r` or `herdr server reload-config`:

```rust
// app/mod.rs:1094-1120
fn apply_config_from_disk(&mut self, notify_success: bool) -> ConfigReloadReport {
    let report = match crate::config::load_live_config() {
        Ok(loaded) => self.apply_live_config(
            &loaded.config, &loaded.diagnostics, &loaded.invalid_sections, notify_success,
        ),
        Err(diagnostics) => { /* failed */ },
    };
    report
}
```

**Key insight:** Sections are validated independently. If `[keys]` has an invalid binding, only that section is rejected — `[ui]`, `[theme]`, etc. still apply. The `invalid_sections` list tracks which sections failed validation.

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/app/mod.rs:1130-1131`

```rust
// app/mod.rs:1130-1131
let invalid_section = |section: &str| invalid_sections.iter().any(|invalid| invalid == section);
if !invalid_section("keys") {
    // Apply keybinds
}
if !invalid_section("ui") {
    // Apply UI settings
}
```

The reload report has three statuses:
- `Applied` — All sections applied successfully
- `Partial` — Some sections applied, others rejected
- `Failed` — Config file could not be loaded

## Sidebar Bounds Validation

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/config/model.rs:206-212`

The sidebar bounds are validated before being applied:

```rust
// config/model.rs:206-212
pub fn validated_sidebar_bounds(min: u16, max: u16) -> Option<(u16, u16)> {
    if min > max {
        None // Invalid — caller should fall back to defaults
    } else {
        Some((min, max))
    }
}
```

If `sidebar_min_width > sidebar_max_width`, the entire `[ui]` section is treated as invalid and the previous settings are kept. This prevents crashes from `u16::clamp(min, max)` panicking when `min > max`.

## Default Config

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/main.rs:65-301`

The default config template is embedded in `main.rs` as a constant string. Running `herdr --default-config` prints this template with all options documented in comments.

## Key Constants

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/config.rs:31-34`

| Constant | Value | Description |
|----------|-------|-------------|
| `CONFIG_PATH_ENV_VAR` | `"HERDR_CONFIG_PATH"` | Environment variable for config override |
| `DEFAULT_SCROLLBACK_LIMIT_BYTES` | `10,000,000` | Default max scrollback per pane (10 MB) |
| `DEFAULT_MOUSE_SCROLL_LINES` | `3` | Default mouse wheel scroll lines |
| `DEFAULT_MOBILE_WIDTH_THRESHOLD` | `64` | Default mobile layout threshold (columns) |

## Cross-References

- [01-architecture.md](01-architecture.md) — Overall system architecture
- [08-cli-commands.md](08-cli-commands.md) — CLI commands for config management
- [05-tui-rendering.md](05-tui-rendering.md) — Theme and color system
- [10-error-handling.md](10-error-handling.md) — Config error handling

→ Next: [08-cli-commands.md](08-cli-commands.md) — CLI command reference
