---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.jj-vcs/jj-vcs.github.io
repository: https://github.com/jj-vcs/jj-vcs.github.io
explored_at: 2026-03-23T00:00:00Z
language: HTML
---

# Project Exploration: jj-vcs.github.io

## Overview

This is a minimal GitHub Pages repository that serves as a redirect from `jj-vcs.github.io` to the actual Jujutsu documentation hosted at `jj-vcs.github.io/jj/latest/`. The documentation itself is generated from the main `jj` repository using MkDocs and is deployed to a separate GitHub Pages path.

The repository contains only three files and serves a purely infrastructural purpose.

## Repository

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.jj-vcs/jj-vcs.github.io`
- **Remote:** `https://github.com/jj-vcs/jj-vcs.github.io` (assumed based on naming)
- **Primary Language:** HTML
- **License:** Present (LICENSE file)

## Directory Structure

```
jj-vcs.github.io/
  index.html    # HTTP meta-refresh redirect to jj docs
  README.md     # One-line description
  LICENSE       # License file
```

## Architecture

This is not a software project -- it is a static redirect page. The `index.html` contains a single `<meta http-equiv="refresh">` tag that redirects visitors to `https://jj-vcs.github.io/jj/latest/`.

The actual documentation is built from the main `jj` repository's `docs/` directory using MkDocs (configured by `jj/mkdocs.yml` and `jj/pyproject.toml`), and deployed to the `jj-vcs.github.io/jj/` path via GitHub Actions.

## Key Insights

- This is purely a redirect shim -- the real content lives in the `jj` repo's `docs/` directory.
- The documentation build system uses MkDocs with Python tooling (configured in `pyproject.toml` and `uv.lock` in the main jj repo).
- The redirect target (`jj/latest/`) suggests versioned documentation with `latest` as an alias for the current release.
