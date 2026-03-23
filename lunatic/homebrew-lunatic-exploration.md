---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.lunatic/homebrew-lunatic
repository: https://github.com/lunatic-solutions/homebrew-lunatic
explored_at: 2026-03-23T00:00:00Z
language: Ruby
---

# Project Exploration: homebrew-lunatic

## Overview

`homebrew-lunatic` is a Homebrew tap that provides a formula for installing the lunatic runtime on macOS via `brew install`. It is a minimal repository containing a single Ruby formula file.

## Repository

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.lunatic/homebrew-lunatic`
- **Remote:** `https://github.com/lunatic-solutions/homebrew-lunatic`
- **Primary Language:** Ruby
- **License:** MIT (assumed, from LICENSE file)

## Directory Structure

```
homebrew-lunatic/
  LICENSE
  README.md
  Formula/
    lunatic.rb              # Homebrew formula
```

## Formula Details

The formula (`lunatic.rb`) installs lunatic v0.13.2:

- **Class:** `Lunatic < Formula`
- **Description:** "A universal runtime for fast, robust and scalable server-side applications."
- **Homepage:** https://lunatic.solutions
- **Download URL:** `https://github.com/lunatic-solutions/lunatic/releases/download/v0.13.2/lunatic-macos-universal.tar.gz`
- **SHA256:** `b88299a9ba9044c461810d1f1ce3bf98a49b4a5a2f5261ab0b3857cf62a1d310`
- **License:** MIT or Apache-2.0
- **Install method:** Simply copies the `lunatic` binary to the bin directory

### Usage

```bash
brew tap lunatic-solutions/lunatic
brew install lunatic
```

## Ecosystem Role

This is pure distribution infrastructure. It provides macOS users with a one-command install path for the lunatic runtime binary. The formula downloads a pre-built universal (x86_64 + aarch64) macOS binary from GitHub Releases.

The formula targets the final release version (v0.13.2) of lunatic, which was the last significant release before the project was archived.
