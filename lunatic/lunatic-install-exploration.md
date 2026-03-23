---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.lunatic/lunatic_install
repository: https://github.com/lunatic-solutions/lunatic_install
explored_at: 2026-03-23T00:00:00Z
language: Shell, PowerShell
---

# Project Exploration: lunatic_install

## Overview

`lunatic_install` provides one-line installation scripts for the lunatic runtime on macOS, Linux, and Windows. These are convenience scripts that download pre-built binaries from GitHub Releases and install them to `~/.lunatic/bin/`.

## Repository

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.lunatic/lunatic_install`
- **Remote:** `https://github.com/lunatic-solutions/lunatic_install`
- **Primary Language:** Shell, PowerShell
- **License:** Not specified

## Directory Structure

```
lunatic_install/
  README.md
  install.sh                # Unix installer (macOS + Linux)
  install.ps1               # Windows installer (PowerShell)
  install_test.sh           # Unix installer tests
  install_test.ps1          # Windows installer tests
```

## Shell Installer (install.sh)

### Platform Detection

The script detects the platform and selects the appropriate binary:
- `Darwin x86_64` -> `macos-x86_64`
- `Darwin aarch64` -> `macos-aarch64`
- `Linux *` -> `linux-amd64`
- `Windows_NT` -> `windows-amd64`
- Linux aarch64 -> Error (not supported)

### Installation Process

1. Checks for `tar` dependency
2. Determines download URL (latest release or specific version via `$1`)
3. Creates `$HOME/.lunatic/bin/` directory
4. Downloads tarball via `curl`
5. Extracts with `tar`
6. Sets executable permissions
7. Prints PATH configuration instructions if `lunatic` is not already in PATH

### Usage

```bash
# Latest version
curl -fsSL https://lunatic.solutions/install.sh | sh

# Specific version
curl -fsSL https://lunatic.solutions/install.sh | sh -s v0.13.2
```

### Environment

- `LUNATIC_INSTALL` - Override install directory (default: `$HOME/.lunatic`)

## PowerShell Installer (install.ps1)

Same logic adapted for Windows:
1. Downloads a `.zip` file (always `windows-amd64`)
2. Extracts to `$HOME\.lunatic\bin\`
3. Adds the bin directory to the user's PATH environment variable
4. Supports version selection via `-v` flag or positional argument

### Usage

```powershell
irm https://lunatic.solutions/install.ps1 | iex
```

## Ecosystem Role

This is the primary distribution mechanism for lunatic, following the same pattern established by Deno, Bun, and Wasmer. The install scripts are hosted at `lunatic.solutions/install.sh` and linked from the main lunatic README. They provide the simplest possible onboarding path: a single curl command.

The scripts work alongside `homebrew-lunatic` (for macOS Homebrew users) and `cargo install lunatic-runtime` (for Rust developers) as alternative installation methods.
