---
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/basecamp/gh-signoff
explored_at: 2026-03-29
prerequisites: Git basics, GitHub CLI installed, Bash shell
---

# Zero to Local CI Engineer - Complete Fundamentals

## Table of Contents

1. [What is Local CI?](#what-is-local-ci)
2. [Why Local CI?](#why-local-ci)
3. [Installation](#installation)
4. [Your First Signoff](#your-first-signoff)
5. [Branch Protection](#branch-protection)
6. [Partial Signoffs](#partial-signoffs)
7. [Team Workflows](#team-workflows)
8. [Advanced Usage](#advanced-usage)
9. [Troubleshooting](#troubleshooting)

## What is Local CI?

**Local CI** is a workflow where you run tests on your own machine instead of cloud CI services. When tests pass, you "sign off" on the commit using `gh signoff`, which creates a GitHub commit status that satisfies branch protection rules.

### Traditional CI vs Local CI

**Traditional Cloud CI:**
```
Developer → Push → GitHub → Triggers CI → Provisions Runner →
Installs Dependencies → Runs Tests → Reports Status → 2-5 minutes
```

**Local CI:**
```
Developer → Run Tests Locally → gh signoff → Push →
GitHub (status already green) → 5-10 seconds
```

## Why Local CI?

### Speed

| Step | Cloud CI | Local CI |
|------|----------|----------|
| Push to trigger | 1s | - |
| Provision runner | 30-60s | - |
| Install dependencies | 60-120s | Already installed |
| Run tests | 30-60s | 30-60s |
| Report status | 5s | 1s (instant) |
| **Total** | **2-5 minutes** | **30-60 seconds** |

### Cost

| Scenario | Cloud CI | Local CI |
|----------|----------|----------|
| 100 commits/month | ~$50-100 | $0 |
| 1000 commits/month | ~$500-1000 | $0 |
| Unlimited | Expensive | Free |

### When Local CI Makes Sense

**Good fit:**
- Small teams (1-10 developers)
- Fast test suites (< 2 minutes)
- Trusted team members
- Simple applications (Rails, basic web apps)
- Cost-conscious projects

**Not recommended:**
- Large teams requiring strict enforcement
- Multi-platform testing (Windows/macOS/Linux)
- Hardware-specific testing
- Regulatory/compliance requirements

## Installation

### Prerequisites

1. **GitHub CLI (gh)** installed:
```bash
# macOS
brew install gh

# Ubuntu/Debian
sudo apt install gh

# Verify
gh --version
```

2. **GitHub Authentication:**
```bash
gh auth login
# Follow prompts to authenticate
```

3. **Git configured:**
```bash
git config user.name "Your Name"
git config user.email "your@email.com"
```

### Install gh-signoff

```bash
# Install the extension
gh extension install basecamp/gh-signoff

# Verify installation
gh signoff version
# Output: gh-signoff 0.2.1

# Setup bash completion (optional)
echo 'eval "$(gh signoff completion)"' >> ~/.bashrc
source ~/.bashrc
```

### Configure Branch Protection

```bash
# Require signoff on default branch (main/master)
gh signoff install

# Verify it's configured
gh signoff check
# Output: ✓ GitHub main branch requires signoff
```

## Your First Signoff

### Workflow

```bash
# 1. Make your changes
git checkout -b feature/my-feature
# ... edit files ...
git add .
git commit -m "Add new feature"

# 2. Run your test suite
rails test
# Output: 50 runs, 0 failures, 0 errors

# 3. Sign off
gh signoff
# Output: ✓ Signed off on abc1234

# 4. Push to GitHub
git push origin feature/my-feature

# 5. Create pull request
gh pr create --title "Add new feature" --body "Description..."
```

### What Happens

When you run `gh signoff`:

```bash
# 1. Get current commit SHA
sha=$(git rev-parse HEAD)
# abc1234def5678...

# 2. Get your name
user=$(git config user.name)
# "Your Name"

# 3. Create GitHub status via API
gh api \
  --method POST \
  "repos/:owner/:repo/statuses/${sha}" \
  -f state=success \
  -f context="signoff" \
  -f description="Your Name signed off"
```

### GitHub Status Display

After signoff, your PR shows:
```
✓ All checks have passed
  ✓ signoff - Your Name signed off
```

## Branch Protection

### Installing Protection

```bash
# Install on default branch
gh signoff install

# Install on specific branch
gh signoff install --branch staging

# Install on multiple branches (run separately)
gh signoff install --branch main
gh signoff install --branch staging
```

### What Branch Protection Does

When you run `gh signoff install`, it configures:

```json
{
  "required_status_checks": {
    "strict": false,
    "contexts": ["signoff"]
  },
  "enforce_admins": null,
  "required_pull_request_reviews": null,
  "restrictions": null
}
```

This means:
- Merges blocked until "signoff" status is success
- Applies to everyone (including admins)
- No PR review requirements changed
- No branch restriction changes

### Verifying Protection

```bash
# Check if signoff is required
gh signoff check

# Check specific branch
gh signoff check --branch main

# Output examples:
# ✓ GitHub main branch requires signoff
# ✗ GitHub staging branch does not require signoff
```

### Removing Protection

```bash
# Remove from default branch
gh signoff uninstall

# Remove from specific branch
gh signoff uninstall --branch staging

# Remove all protection (including non-signoff rules)
# Warning: This removes ALL branch protection!
gh api \
  --method DELETE \
  "repos/:owner/:repo/branches/main/protection"
```

## Partial Signoffs

For projects with multiple CI steps (tests, lint, security):

### Setting Up Partial Signoffs

```bash
# Install with multiple contexts
gh signoff install tests lint security

# This creates three required statuses:
# - signoff/tests
# - signoff/lint
# - signoff/security
```

### Signing Off

```bash
# Run tests, sign off
rails test && gh signoff tests

# Run linter, sign off
rubocop && gh signoff lint

# Run security scan, sign off
bundle audit && gh signoff security

# Or all at once
gh signoff tests lint security
```

### Status Display

```bash
gh signoff status

# Output:
# ✓ signoff
# ✓ tests
# ✓ lint
# ✗ security
```

### Checking Partial Signoffs

```bash
# Check if specific contexts are required
gh signoff check tests
gh signoff check lint security

# Check all
gh signoff check tests lint security
```

## Team Workflows

### Recommended Workflow

```bash
# 1. Create feature branch
git checkout -b feature/my-feature

# 2. Make changes, run tests frequently
rails test:all  # Run full test suite

# 3. Before pushing, final signoff
gh signoff

# 4. Push and create PR
git push origin feature/my-feature
gh pr create

# 5. After code review, merge
gh pr merge --merge --delete-branch
```

### Multi-Developer Workflow

Each developer signs off on their own commits:

```
Commit 1 (alice):
  ✓ signoff - Alice signed off

Commit 2 (bob):
  ✓ signoff - Bob signed off
```

### Force Signoff

When you need to bypass clean-repo checks:

```bash
# Force signoff (ignores uncommitted/unpushed changes)
gh signoff -f

# Warning: Only use when you understand the implications!
```

### Scripting Signoffs

Add to your test script:

```bash
#!/bin/bash
# script/ci

# Run tests
rails test

# Auto-signoff if tests pass
if [ $? -eq 0 ]; then
  gh signoff
else
  echo "Tests failed, not signing off"
  exit 1
fi
```

## Advanced Usage

### Pre-commit Hook

Automatically check tests before allowing commit:

```bash
# .git/hooks/pre-commit
#!/bin/bash

echo "Running tests..."
rails test

if [ $? -ne 0 ]; then
  echo "Tests failed, commit blocked"
  exit 1
fi

echo "Tests passed"
exit 0
```

### Post-commit Signoff

Automatically sign off after successful commit:

```bash
# .git/hooks/post-commit
#!/bin/bash

# Run tests
rails test --quiet

if [ $? -eq 0 ]; then
  gh signoff 2>/dev/null || true  # Silent fail if gh not available
fi
```

### CI Hybrid Approach

Use local CI for PRs, cloud CI for releases:

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main]  # Only run on pushes to main
  schedule:
    - cron: "0 0 * * *"  # Nightly full CI

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: bundle install
      - run: rails test
```

### Multiple Repositories

Manage signoffs across multiple repos:

```bash
# Repo 1
cd ~/projects/repo1
gh signoff install

# Repo 2
cd ~/projects/repo2
gh signoff install

# Each repo maintains separate signoff status
```

## Troubleshooting

### Common Issues

**"gh command not found":**
```bash
# Install GitHub CLI
brew install gh  # macOS
sudo apt install gh  # Ubuntu

# Verify
which gh
gh --version
```

**"Repository has uncommitted changes":**
```bash
# Check what's dirty
git status

# Option 1: Commit or stash changes
git add .
git commit -m "WIP"

# Option 2: Force signoff (use carefully!)
gh signoff -f
```

**"Repository has unpushed changes":**
```bash
# Check what's unpushed
git log @{push}..

# Push first
git push

# Then signoff
gh signoff
```

**"Branch does not require signoff":**
```bash
# Install branch protection
gh signoff install

# Or check if on wrong branch
git branch
gh signoff check --branch $(git branch --show-current)
```

**"Failed to create status":**
```bash
# Check gh authentication
gh auth status

# Re-authenticate if needed
gh auth login

# Check repository permissions
# You need write access to create statuses
```

### Debug Mode

```bash
# Enable debug output
export SIGNOFF_DEBUG=1
gh signoff status

# Shows detailed API calls and responses
```

### Checking Status Programmatically

```bash
# In scripts
if gh signoff check >/dev/null 2>&1; then
  echo "Signoff required"
else
  echo "Signoff not required"
fi

# Get signoff contexts
contexts=$(gh signoff completion --contexts)
echo "Required contexts: $contexts"
```

### Migration from Cloud CI

```bash
# Week 1: Parallel run
# - Keep cloud CI
# - Install gh-signoff
# - Encourage team to use locally

# Week 2: Require signoff
gh signoff install
# - Cloud CI still runs
# - Signoff now required for merge

# Week 3: Reduce cloud CI frequency
# - Change cloud CI to nightly only
# - Daily full CI for safety net

# Week 4: Evaluate
# - If team disciplined, keep local-only
# - If issues, adjust process
```

## Best Practices

### Do

- Run full test suite before signing off
- Keep test suite fast (< 2 minutes)
- Use partial signoffs for different check types
- Maintain cloud CI as safety net (nightly)
- Document signoff process for team

### Don't

- Sign off without running tests
- Use `-f` flag to bypass checks routinely
- Rely solely on local CI for compliance requirements
- Forget to update team documentation

---

**Next Steps:**
- [01-gh-signoff-exploration.md](./01-gh-signoff-exploration.md) - Full architecture
- [rust-revision.md](./rust-revision.md) - Rust implementation considerations
- [production-grade.md](./production-grade.md) - Production deployment guide
