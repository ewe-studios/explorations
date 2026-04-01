---
location: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/gh-signoff
repository: git@github.com:basecamp/gh-signoff.git
explored_at: 2026-03-29
language: Bash
category: Developer Tooling, CI/CD
---

# gh-signoff - Local CI Exploration

## Overview

`gh-signoff` is a **GitHub CLI extension** that enables "local CI" - running tests on your own machine and signing off on commits without traditional cloud CI infrastructure. It leverages GitHub's branch protection and commit statuses to create a lightweight approval workflow.

### Key Value Proposition

- **Fast Feedback**: Tests run locally on your fast laptop, not slow cloud CI
- **Cost-Effective**: No monthly CI bills - use existing hardware
- **Simple Workflow**: `rails test` → `gh signoff` → merge
- **GitHub Native**: Uses GitHub branch protection and commit statuses
- **Partial Signoffs**: Support for multiple CI steps (tests, lint, security)
- **Low Ceremony**: Perfect for small teams and simple apps

### Philosophy

> "Dev laptops are super fast these days. They're chronically underutilized. And you already own them. Cloud CI services are typically slow, expensive, and rented."

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Developer Workflow                            │
│                                                                 │
│  1. Make changes                                                │
│  2. Run tests locally: rails test                               │
│  3. Sign off: gh signoff                                        │
│  4. Push to GitHub                                              │
│  5. Create PR                                                   │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    GitHub API                              │   │
│  │                                                            │   │
│  │  POST /repos/:owner/:repo/statuses/:sha                   │   │
│  │  {                                                         │   │
│  │    "state": "success",                                     │   │
│  │    "context": "signoff",                                   │   │
│  │    "description": "Developer signed off"                   │   │
│  │  }                                                         │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                 Branch Protection Rules                          │
│                                                                 │
│  Required status checks:                                        │
│  - [x] signoff                                                  │
│  - [x] signoff/tests (optional partial)                         │
│  - [x] signoff/lint (optional partial)                          │
│                                                                 │
│  Merge button enabled when all checks pass ✓                    │
└─────────────────────────────────────────────────────────────────┘
```

## Installation

```bash
# Install the extension
gh extension install basecamp/gh-signoff

# Require signoff on default branch
gh signoff install

# Check signoff status
gh signoff status
```

## Usage

### Basic Signoff

```bash
# Run your tests
rails test

# Sign off on current commit
gh signoff

# Force signoff (ignore uncommitted/unpushed changes)
gh signoff -f
```

### Partial Signoffs

For projects with multiple CI steps:

```bash
# Sign off on individual checks
gh signoff tests      # Tests pass
gh signoff lint       # Linting pass
gh signoff security   # Security scan pass

# Or all at once
gh signoff tests lint security
```

### Installing Branch Protection

```bash
# Require signoff on default branch
gh signoff install

# Require on specific branch
gh signoff install --branch main

# Require multiple partial signoffs
gh signoff install tests lint security

# Require partial signoff on specific branch
gh signoff install --branch staging tests lint
```

### Checking Status

```bash
# Check if signoff is required
gh signoff check

# Check specific branch
gh signoff check --branch main

# Check partial signoffs
gh signoff check tests lint security

# View commit signoff status
gh signoff status
```

**Example output:**
```
✓ signoff
✓ tests
✗ lint
✗ security
```

## How It Works

### 1. Creating a Signoff

When you run `gh signoff`:

```bash
# Get current commit SHA
sha=$(git rev-parse HEAD)

# Get developer name
user=$(git config user.name)

# Create commit status via GitHub API
gh api \
  --method POST \
  "repos/:owner/:repo/statuses/${sha}" \
  -f state=success \
  -f context="signoff" \
  -f description="${user} signed off"
```

**Result in GitHub UI:**
```
All checks have passed
✓ signoff - developer signed off
```

### 2. Installing Branch Protection

When you run `gh signoff install`:

```bash
# Set branch protection via GitHub API
gh api \
  --method PUT \
  "repos/:owner/:repo/branches/${branch}/protection" \
  -H "Accept: application/vnd.github+json" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  -f "required_status_checks[strict]=false" \
  -f "required_status_checks[contexts][]="signoff"" \
  -f "enforce_admins=null" \
  -f "required_pull_request_reviews=null" \
  -f "restrictions=null"
```

**Result:**
- Merge button disabled until signoff present
- Green checkmark required for merge

### 3. Partial Signoffs

Each partial signoff creates a separate status context:

```bash
# Context naming convention
gh signoff tests    → context: "signoff/tests"
gh signoff lint     → context: "signoff/lint"
gh signoff security → context: "signoff/security"
```

**GitHub status list:**
```
✓ signoff/tests - developer signed off
✓ signoff/lint - developer signed off
✗ signoff/security - not yet signed
```

## Implementation Details

### Clean Repository Check

Before allowing signoff, verify repository state:

```bash
is_clean() {
  # Check for uncommitted changes
  if [[ -n "$(git status --porcelain)" ]]; then
    return 1  # Dirty
  fi

  # Check branch has upstream
  if ! git rev-parse --abbrev-ref @{push} >/dev/null 2>&1; then
    return 1  # No tracking branch
  fi

  # Check for unpushed commits
  if [[ -n "$(git log @{push}..)" ]]; then
    return 1  # Unpushed changes
  fi

  return 0  # Clean
}
```

### Status Creation

```bash
cmd_create() {
  local force=false
  local contexts=()

  # Parse arguments
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -f) force=true; shift ;;
      *) contexts+=("$1"); shift ;;
    esac
  done

  # Verify clean repo (unless forced)
  if ! $force && ! is_clean; then
    fail "repository has uncommitted or unpushed changes"
  fi

  # Get commit info
  local user=$(git config user.name)
  local sha=$(git rev-parse HEAD)

  # Create status for each context
  for context in "${contexts[@]}"; do
    local context_name="signoff"
    [[ -n "$context" ]] && context_name="signoff/${context}"

    gh api \
      --method POST \
      "repos/:owner/:repo/statuses/${sha}" \
      -f state=success \
      -f context="${context_name}" \
      -f description="${user} signed off"
  done
}
```

### Branch Protection

```bash
cmd_install() {
  local branch=""
  local contexts=()

  # Parse arguments
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --branch) branch="$2"; shift 2 ;;
      *) contexts+=("$1"); shift ;;
    esac
  done

  # Default to default branch
  [[ -z "$branch" ]] && branch=$(gh api repos/:owner/:repo --jq .default_branch)

  # Build API fields
  local api_fields=()
  api_fields+=("--field" "required_status_checks[strict]=false")
  api_fields+=("--field" "enforce_admins=null")
  api_fields+=("--field" "required_pull_request_reviews=null")
  api_fields+=("--field" "restrictions=null")

  # Add contexts
  for context in "${contexts[@]}"; do
    local context_name="signoff"
    [[ -n "$context" ]] && context_name="signoff/${context}"
    api_fields+=("--field" "required_status_checks[contexts][]=${context_name}")
  done

  # Set protection
  gh api \
    --method PUT \
    "repos/:owner/:repo/branches/${branch}/protection" \
    "${api_fields[@]}"
}
```

### Status Display

```bash
cmd_status() {
  local sha=$(git rev-parse HEAD)

  # Get commit statuses
  local statuses=$(gh api "repos/:owner/:repo/commits/${sha}/status")

  # Get branch protection
  local protection=$(gh api "repos/:owner/:repo/branches/${branch}/protection")

  # Extract required contexts from protection rules
  local required=("signoff")
  while read -r ctx; do
    [[ -z "$ctx" || "$ctx" == "signoff" ]] && continue
    required+=("$ctx")
  done < <(echo "$protection" | jq -r '.required_status_checks?.contexts? | map(select(startswith("signoff"))) | .[]?')

  # Build context→status map
  local context_map=""
  while IFS=$'\t' read -r context state; do
    context_map="${context_map}${context}=${state};"
  done < <(echo "$statuses" | jq -r '.statuses[]? | select(.context? | startswith("signoff")) | [.context, .state] | @tsv')

  # Display status for each context
  for context in "${required[@]}"; do
    local display_name="$context"
    [[ "$context" == "signoff/"* ]] && display_name="${context#signoff/}"

    if [[ "$context_map" == *"${context}=success;"* ]]; then
      echo "✓ $display_name"
    else
      echo "✗ $display_name"
    fi
  done
}
```

## Workflow Comparison

### Traditional Cloud CI

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. Developer pushes commit                                      │
│ 2. GitHub triggers CI workflow                                  │
│ 3. CI provisions runner (30-60s cold start)                    │
│ 4. CI installs dependencies (60-120s)                          │
│ 5. CI runs tests (30-60s)                                       │
│ 6. CI reports status to GitHub                                  │
│                                                                 │
│ Total: 2-5 minutes per commit                                   │
│ Cost: $0.008/minute (GitHub Actions)                            │
└─────────────────────────────────────────────────────────────────┘
```

### Local CI with gh-signoff

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. Developer runs tests locally (30-60s)                       │
│ 2. Developer runs: gh signoff (1s)                             │
│ 3. Developer pushes commit                                      │
│ 4. GitHub shows green checkmark immediately                     │
│                                                                 │
│ Total: 30-60 seconds per commit                                 │
│ Cost: $0 (uses existing hardware)                               │
└─────────────────────────────────────────────────────────────────┘
```

## When to Use gh-signoff

### Good Fit

- Small teams (1-10 developers)
- Simple applications (Rails, basic web apps)
- Fast local test suites (< 2 minutes)
- Trusted team members
- Cost-conscious projects

### Not Recommended

- Large teams requiring strict enforcement
- Complex multi-platform testing (Windows, macOS, Linux)
- Hardware-specific testing
- Regulatory/compliance requirements
- High-turnover teams

## Advanced Features

### Bash Completion

```bash
# Add to ~/.bashrc
eval "$(gh signoff completion)"

# Tab completion now works:
gh signoff <TAB>
# Shows: create install uninstall check status version tests lint security
```

### Scripting

```bash
# Check if signoff is required
if gh signoff check >/dev/null 2>&1; then
  echo "Signoff required"
fi

# Get signoff contexts
contexts=$(gh signoff completion --contexts)

# Force signoff in CI
gh signoff create -f
```

### Multiple Developers

Each developer's signoff creates a separate status:

```
✓ signoff - alice signed off
✓ signoff - bob signed off
```

## Security Considerations

### Trust Model

gh-signoff assumes:
1. Developers run actual tests before signing off
2. Developers don't sign off on untested code
3. Team has discipline and accountability

### Enforcement

- Branch protection prevents merges without signoff
- Status contexts are immutable (only GitHub can create)
- Audit trail in GitHub status history

### Bypassing

- `-f` flag bypasses clean-repo check
- Developers could sign off without running tests
- Solution: Team culture, code review, occasional audits

## Limitations

1. **No Parallelization**: Tests run on single machine
2. **Platform-Specific**: Can't test Windows/macOS from Linux laptop
3. **No Hardware Diversity**: Can't test across different hardware
4. **Trust-Dependent**: Relies on developer discipline
5. **No Build Artifacts**: Doesn't produce deployable artifacts

## Production Considerations

### Hybrid Approach

Use gh-signoff for quick feedback + cloud CI for releases:

```yaml
# .github/workflows/ci.yml
name: CI
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
  schedule:
    - cron: "0 0 * * *"  # Nightly full CI

jobs:
  test:
    runs-on: ubuntu-latest
    # Skip if gh-signoff present (for PR workflow)
    if: github.event_name == 'schedule' || github.event_name == 'push'
    steps:
      - uses: actions/checkout@v4
      - run: bundle install
      - run: rails test
```

### Migration Strategy

```bash
# Week 1: Install gh-signoff, encourage voluntary use
gh signoff install

# Week 2: Require signoff, keep cloud CI as backup
# Update branch protection to require both

# Week 3: Evaluate - if team disciplined, reduce cloud CI frequency
# Change scheduled CI from hourly to daily

# Week 4: Optional - remove cloud CI for PRs entirely
```

## Related Tools

- **GitHub Actions**: Cloud CI for complex workflows
- **CircleCI**: Alternative cloud CI
- **Buildkite**: Self-hosted CI
- **Tugboat**: Preview environments

---

## Related Deep Dives

- [00-zero-to-local-ci-engineer.md](./00-zero-to-local-ci-engineer.md) - Fundamentals
- [rust-revision.md](./rust-revision.md) - Rust implementation considerations
- [production-grade.md](./production-grade.md) - Production deployment guide
