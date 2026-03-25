# supports-* Crates: Terminal Feature Detection in Rust

**Sources:**
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.zkat/supports-color/`
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.zkat/supports-hyperlinks/`
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.zkat/supports-unicode/`

---

## Table of Contents

1. [Overview](#overview)
2. [supports-color](#supports-color)
3. [supports-hyperlinks](#supports-hyperlinks)
4. [supports-unicode](#supports-unicode)
5. [Environment Variable Reference](#environment-variable-reference)
6. [CI/CD Detection](#cicd-detection)
7. [Integration with miette](#integration-with-miette)
8. [Code Examples](#code-examples)

---

## Overview

The `supports-*` family of crates provides reliable detection of terminal capabilities. These small, focused libraries help your application adapt its output based on what the terminal supports.

### The Problem

When writing CLI applications, you want to:
- Use colors and styling when supported
- Avoid ANSI codes when not supported
- Adapt output for CI environments
- Respect user preferences (NO_COLOR, etc.)

Manually detecting these capabilities is error-prone and platform-specific.

### The Solution

```
┌─────────────────────────────────────────────────────────────────┐
│                 Terminal Feature Detection                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────┐  ┌──────────────────┐  ┌───────────────┐ │
│  │ supports-color   │  │ supports-hyperlink│  │supports-unicode││
│  │                  │  │                  │  │               │ │
│  │ - Color levels   │  │ - OSC 8 links    │  │ - UTF-8       │ │
│  │ - 16/256/16M     │  │ - Terminal check │  │ - Emoji       │ │
│  │ - CI detection   │  │                  │  │               │ │
│  └──────────────────┘  └──────────────────┘  └───────────────┘ │
│         │                      │                    │           │
│         └──────────────────────┼────────────────────┘           │
│                                │                                 │
│                                ▼                                 │
│                    ┌─────────────────────┐                      │
│                    │   miette (errors)   │                      │
│                    │   orogene (output)  │                      │
│                    │   your CLI app      │                      │
│                    └─────────────────────┘                      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Crate Versions

| Crate | Version | MSRV |
|-------|---------|------|
| supports-color | 3.0.2 | 1.70.0 |
| supports-hyperlinks | 3.1.0 | 1.70.0 |
| supports-unicode | 3.0.0 | 1.70.0 |

---

## supports-color

**Version:** 3.0.0 | **License:** Apache-2.0

Detects terminal color support level and provides detailed information about capabilities.

### Color Levels

```rust
use supports_color::{ColorLevel, Stream};

/// Color level support details
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ColorLevel {
    level: usize,
    /// Basic ANSI colors (4-bit)
    pub has_basic: bool,
    /// 256-bit colors (8-bit)
    pub has_256: bool,
    /// 16 million colors - truecolor (24-bit RGB)
    pub has_16m: bool,
}
```

### Color Level Detection Flow

```
┌─────────────────────────────────────────────────────────────────┐
│              supports-color Detection Algorithm                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. Check FORCE_COLOR / CLICOLOR_FORCE                          │
│     └─▶ If set, return specified level (1-3)                    │
│                                                                  │
│  2. Check NO_COLOR                                              │
│     └─▶ If set (and not "0"), return None                       │
│                                                                  │
│  3. Check TERM = "dumb"                                         │
│     └─▶ If dumb terminal, return None                           │
│                                                                  │
│  4. Check if TTY (or IGNORE_IS_TERMINAL)                        │
│     └─▶ If not TTY, return None                                 │
│                                                                  │
│  5. Check for 16M color support                                 │
│     - COLORTERM=truecolor / 24bit                               │
│     - TERM=*-direct / *-truecolor                               │
│     - TERM_PROGRAM=iTerm.app                                    │
│     └─▶ Return Level 3 (16M)                                    │
│                                                                  │
│  6. Check for 256 color support                                 │
│     - TERM_PROGRAM=Apple_Terminal                               │
│     - TERM=*-256 / *-256color                                   │
│     └─▶ Return Level 2 (256)                                    │
│                                                                  │
│  7. Check for basic color support                               │
│     - COLORTERM set                                             │
│     - TERM supports ANSI (not dumb)                             │
│     - CLICOLOR set                                              │
│     - Running on CI                                             │
│     └─▶ Return Level 1 (Basic)                                  │
│                                                                  │
│  8. No color support detected                                   │
│     └─▶ Return None                                             │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Usage

```rust
use supports_color::{on, on_cached, Stream, ColorLevel};

// Check color support (fresh check each time)
if let Some(level) = on(Stream::Stdout) {
    if level.has_16m {
        println!("\x1b[38;2;255;100;0mTruecolor text!\x1b[0m");
    } else if level.has_256 {
        println!("\x1b[38;5;208m256-color text!\x1b[0m");
    } else if level.has_basic {
        println!("\x1b[33mBasic yellow!\x1b[0m");
    }
} else {
    println!("No color support, using plain text");
}

// Cached check (faster for repeated calls)
let level = on_cached(Stream::Stdout);

// Check stderr separately
if let Some(level) = on(Stream::Stderr) {
    eprintln!("Stderr has color: {:?}", level);
}
```

### Terminal Detection

| Terminal | Color Level | Detection Method |
|----------|-------------|------------------|
| iTerm2 | 16M | `TERM_PROGRAM=iTerm.app` |
| GNOME Terminal | 256 | `VTE_VERSION` |
| Windows Terminal | 16M | `WT_SESSION` |
| VSCode Terminal | 16M | `TERM_PROGRAM=vscode` |
| xterm-256color | 256 | `TERM=xterm-256color` |
| CI (GitHub Actions) | Basic | `is_ci` crate |

---

## supports-hyperlinks

**Version:** 3.1.0 | **License:** Apache-2.0

Detects whether a terminal supports rendering OSC 8 hyperlinks (clickable links in terminal output).

### OSC 8 Hyperlinks

OSC 8 is an ANSI escape sequence that creates clickable links in supported terminals:

```
\x1b]8;;https://example.com\x1b\\Link text\x1b]8;;\x1b\\
```

Rendered as: [Link text](https://example.com) (clickable in terminal)

### Detection Algorithm

```rust
/// Returns true if the current terminal supports hyperlinks
pub fn supports_hyperlinks() -> bool {
    // 1. Force through environment variable
    if let Ok(arg) = std::env::var("FORCE_HYPERLINK") {
        return arg.trim() != "0";
    }

    // 2. DomTerm (web-based terminal)
    if std::env::var("DOMTERM").is_ok() {
        return true;
    }

    // 3. VTE-based terminals (v0.50+)
    if let Ok(version) = std::env::var("VTE_VERSION") {
        if version.parse().unwrap_or(0) >= 5000 {
            return true;
        }
    }

    // 4. Known terminal programs
    if let Ok(program) = std::env::var("TERM_PROGRAM") {
        if matches!(
            &program[..],
            "Hyper" | "iTerm.app" | "terminology" | "WezTerm" | "vscode" | "ghostty"
        ) {
            return true;
        }
    }

    // 5. TERM-based detection
    if let Ok(term) = std::env::var("TERM") {
        if matches!(&term[..], "xterm-kitty" | "alacritty" | "alacritty-direct") {
            return true;
        }
    }

    // 6. Special cases
    if let Ok(term) = std::env::var("COLORTERM") {
        if matches!(&term[..], "xfce4-terminal") {
            return true;
        }
    }

    // 7. Windows Terminal and Konsole
    std::env::var("WT_SESSION").is_ok() || std::env::var("KONSOLE_VERSION").is_ok()
}
```

### Usage

```rust
use supports_hyperlinks::{on, supports_hyperlinks, Stream};

// Check if hyperlinks are supported
if supports_hyperlinks() {
    println!("This terminal supports hyperlinks!");
}

// Check for specific stream (also checks if TTY)
if on(Stream::Stdout) {
    // Output OSC 8 hyperlink
    println!(
        "\x1b]8;;https://example.com\x1b\\Click me\x1b]8;;\x1b\\"
    );
} else {
    // Fallback to plain URL
    println!("Click me: https://example.com");
}
```

### Supported Terminals

| Terminal | Support | Notes |
|----------|---------|-------|
| iTerm2 | ✓ | Full support |
| WezTerm | ✓ | Full support |
| Kitty | ✓ | xterm-kitty TERM |
| Alacritty | ✓ | Recent versions |
| VSCode | ✓ | Integrated terminal |
| Windows Terminal | ✓ | Full support |
| GNOME Terminal | ✓ | VTE v0.50+ |
| Hyper | ✓ | Electron-based |
| Konsole | ✓ | KDE terminal |

---

## supports-unicode

**Version:** 3.0.0 | **License:** Apache-2.0

Detects whether a terminal supports Unicode rendering.

### Detection Algorithm

```rust
pub fn supports_unicode() -> bool {
    if std::env::consts::OS == "windows" {
        // Windows-specific checks
        std::env::var("CI").is_ok()  // CI environments
            || std::env::var("WT_SESSION").is_ok()  // Windows Terminal
            || std::env::var("ConEmuTask") == Ok("{cmd:Cmder}".into())  // Cmder
            || std::env::var("TERM_PROGRAM") == Ok("vscode".into())  // VSCode
            || std::env::var("TERM") == Ok("xterm-256color".into())
            || std::env::var("TERM") == Ok("alacritty".into())
    } else if std::env::var("TERM") == Ok("linux".into()) {
        // Linux kernel console
        false
    } else {
        // Unix: check locale
        let ctype = std::env::var("LC_ALL")
            .or_else(|_| std::env::var("LC_CTYPE"))
            .or_else(|_| std::env::var("LANG"))
            .unwrap_or_else(|_| "".into())
            .to_uppercase();
        ctype.ends_with("UTF8") || ctype.ends_with("UTF-8")
    }
}
```

### Usage

```rust
use supports_unicode::{on, supports_unicode, Stream};

// Basic check
if supports_unicode() {
    println!("✓ Unicode supported! Using symbols: ◆ ▶ ●");
} else {
    println!("ASCII only: Using symbols: * > o");
}

// Stream-specific check (also checks TTY)
if on(Stream::Stdout) {
    // Unicode output
    println!("Success ✓");
} else {
    // Piping to file - always safe to use Unicode
    println!("Success");
}
```

### Unicode Detection

| Platform | Check | Result |
|----------|-------|--------|
| Windows (WT) | `WT_SESSION` | ✓ |
| Windows (VSCode) | `TERM_PROGRAM=vscode` | ✓ |
| Windows (Cmder) | `ConEmuTask` | ✓ |
| Linux (console) | `TERM=linux` | ✗ |
| Unix (UTF-8) | `LANG=*.UTF-8` | ✓ |
| Unix (C locale) | `LC_ALL=C` | ✗ |

---

## Environment Variable Reference

### Force/Toggle Variables

| Variable | Values | Effect |
|----------|--------|--------|
| `FORCE_COLOR` | `true`, `1`, `2`, `3`, `false`, `0` | Force color level |
| `CLICOLOR_FORCE` | `1`, `0` | Force color (legacy) |
| `NO_COLOR` | Any (non-empty) | Disable color |
| `FORCE_HYPERLINK` | `1`, `0` | Force hyperlink support |
| `IGNORE_IS_TERMINAL` | `1`, `0` | Ignore TTY check |

### Detection Variables (Read-Only)

| Variable | Purpose |
|----------|---------|
| `TERM` | Terminal type (dumb, xterm-256color, etc.) |
| `COLORTERM` | Color terminal indicator (truecolor, 24bit) |
| `TERM_PROGRAM` | Terminal program (iTerm.app, vsode, etc.) |
| `VTE_VERSION` | VTE engine version |
| `WT_SESSION` | Windows Terminal session |
| `CI` | Continuous integration environment |
| `LC_ALL`, `LC_CTYPE`, `LANG` | Locale settings |

### Priority Order

```
1. FORCE_* variables (highest priority - user override)
2. NO_COLOR (user preference to disable)
3. TERM=dumb (terminal capability)
4. TTY check (is it a terminal?)
5. Feature detection (TERM_PROGRAM, COLORTERM, etc.)
6. CI detection (assume basic features)
7. No support (lowest)
```

---

## CI/CD Detection

The `supports-color` crate uses the `is_ci` crate to detect CI environments:

```rust
use is_ci::uncached;

fn supports_color(stream: Stream) -> usize {
    // ... other checks ...

    // CI environments typically support basic colors
    if is_ci::uncached() {
        return 1;  // Basic ANSI
    }

    0  // No support
}
```

### Detected CI Platforms

The `is_ci` crate detects:
- GitHub Actions (`GITHUB_ACTIONS`)
- GitLab CI (`GITLAB_CI`)
- Travis CI (`TRAVIS`)
- CircleCI (`CIRCLECI`)
- Jenkins (`BUILD_NUMBER`)
- Azure Pipelines (`TF_BUILD`)
- And many more...

---

## Integration with miette

miette uses all three supports-* crates for its fancy output:

```rust
// From miette's Cargo.toml
[dependencies]
supports-hyperlinks = { version = "3.0.0", optional = true }
supports-color = { version = "3.0.0", optional = true }
supports-unicode = { version = "3.0.0", optional = true }

[features]
fancy-no-backtrace = [
    "dep:supports-hyperlinks",
    "dep:supports-color",
    "dep:supports-unicode",
]
```

### How miette Uses Them

```rust
// Simplified from miette handler
use supports_color::on as supports_color;
use supports_unicode::on as supports_unicode;
use supports_hyperlinks::on as supports_hyperlinks;

fn render_diagnostic(&self, diag: &dyn Diagnostic) -> String {
    let color = supports_color(Stream::Stderr);
    let unicode = supports_unicode(Stream::Stderr);
    let hyperlinks = supports_hyperlinks(Stream::Stderr);

    // Choose characters based on support
    let chars = if unicode {
        FancyCharacters::unicode()
    } else {
        FancyCharacters::ascii()
    };

    // Build output with or without colors
    if color.is_some() {
        self.render_with_colors(diag, &chars, hyperlinks)
    } else {
        self.render_narratable(diag, &chars)
    }
}
```

---

## Code Examples

### Basic Feature Detection

```rust
use supports_color::{on, Stream};
use supports_unicode::on as supports_unicode;
use supports_hyperlinks::on as supports_hyperlinks;

fn print_status(message: &str, success: bool) {
    let unicode = supports_unicode(Stream::Stdout);
    let color = on(Stream::Stdout);

    let symbol = if unicode {
        if success { "✓" } else { "✗" }
    } else {
        if success { "[OK]" } else { "[FAIL]" }
    };

    if let Some(level) = color {
        let color_code = if success { 32 } else { 31 };
        if level.has_256 {
            println!("\x1b[38;5;{}m{} {}\x1b[0m", color_code, symbol, message);
        } else {
            println!("\x1b[{}m{} {}\x1b[0m", color_code, symbol, message);
        }
    } else {
        println!("{} {}", symbol, message);
    }
}
```

### Hyperlink Output

```rust
use supports_hyperlinks::{on, Stream};

fn print_link(text: &str, url: &str) {
    if on(Stream::Stdout) {
        // OSC 8 hyperlink
        println!("\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", url, text);
    } else {
        // Fallback
        println!("{}: {}", text, url);
    }
}

// Usage
print_link("Documentation", "https://docs.example.com");
```

### Cached Checks

```rust
use supports_color::{on_cached, Stream, ColorLevel};

struct OutputFormatter {
    color: Option<ColorLevel>,
    unicode: bool,
    hyperlinks: bool,
}

impl OutputFormatter {
    fn new() -> Self {
        Self {
            color: on_cached(Stream::Stdout),
            unicode: supports_unicode::on(Stream::Stdout),
            hyperlinks: supports_hyperlinks::on(Stream::Stdout),
        }
    }

    fn format(&self, message: &str) -> String {
        if let Some(level) = self.color {
            if level.has_16m {
                format!("\x1b[38;2;100;200;255m{}\x1b[0m", message)
            } else {
                format!("\x1b[36m{}\x1b[0m", message)
            }
        } else {
            message.to_string()
        }
    }
}
```

### Building a Compatible CLI

```rust
use supports_color::{on, Stream};
use supports_unicode::on as supports_unicode;

struct Cli {
    color: Option<ColorLevel>,
    unicode: bool,
}

impl Cli {
    fn new() -> Self {
        Self {
            color: on(Stream::Stdout),
            unicode: supports_unicode(Stream::Stdout),
        }
    }

    fn progress(&self, current: usize, total: usize) {
        if self.unicode {
            // Pretty progress bar
            let percent = (current * 100) / total;
            let filled = (percent / 5) as usize;
            let empty = 20 - filled;
            println!("[{}{}] {}%", "█".repeat(filled), "░".repeat(empty), percent);
        } else {
            // ASCII progress bar
            let percent = (current * 100) / total;
            let filled = (percent / 5) as usize;
            let empty = 20 - filled;
            println!("[{}{}] {}%", "#".repeat(filled), "-".repeat(empty), percent);
        }
    }

    fn error(&self, message: &str) {
        if self.color.is_some() {
            eprintln!("\x1b[31mError:\x1b[0m {}", message);
        } else {
            eprintln!("Error: {}", message);
        }
    }
}
```

### Environment Variable Override

```rust
use supports_color::{on, Stream};

fn main() {
    // User can override with:
    // FORCE_COLOR=1 myapp      - Force basic color
    // FORCE_COLOR=3 myapp      - Force 16M color
    // NO_COLOR=1 myapp         - Disable color
    // TERM=dumb myapp          - Force dumb terminal

    match on(Stream::Stdout) {
        Some(level) if level.has_16m => {
            println!("Using truecolor output");
        }
        Some(level) if level.has_256 => {
            println!("Using 256-color output");
        }
        Some(_) => {
            println!("Using basic ANSI color output");
        }
        None => {
            println!("Using plain text output");
        }
    }
}
```

---

## Summary

The supports-* family provides:

1. **Reliable Detection:** Multiple heuristics for accurate results
2. **Environment Respect:** Honors NO_COLOR, FORCE_COLOR, etc.
3. **CI Awareness:** Detects and adapts to CI environments
4. **Cross-Platform:** Works on Windows, macOS, Linux
5. **Easy Integration:** Simple APIs, minimal overhead
6. **Caching Support:** Efficient repeated checks
7. **TTY Detection:** Distinguishes terminals from pipes

These crates are essential building blocks for modern, user-friendly CLI applications that adapt gracefully to any environment.
