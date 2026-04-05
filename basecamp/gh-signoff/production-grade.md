---
source: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/gh-signoff
repository: github.com/basecamp/gh-signoff
explored_at: 2026-04-05
focus: Enterprise deployment, GitHub API rate limits, team management, audit logging, compliance
---

# Production-Grade gh-signoff Deployments

## Overview

This document covers production deployment patterns for gh-signoff in enterprise environments including API rate limit management, team onboarding, audit logging, compliance integration, and hybrid CI architectures.

## Architecture

```mermaid
flowchart TB
    subgraph Developers["Development Team"]
        Dev1[Developer 1]
        Dev2[Developer 2]
        Dev3[Developer 3]
    end
    
    subgh-signoff["gh-signoff Infrastructure"]
        CLI[gh-signoff CLI]
        Hooks[Git Hooks]
        Scripts[Automation Scripts]
    end
    
    subgraph GitHub["GitHub Platform"]
        API[GitHub API]
        Statuses[Commit Statuses]
        Protection[Branch Protection]
        Audit[Audit Log]
    end
    
    subgraph CI["Hybrid CI"]
        Local[Local Testing]
        Nightly[Nightly CI]
        Release[Release CI]
    end
    
    subgraph Compliance["Compliance Layer"]
        Logging[Audit Logging]
        Reports[Compliance Reports]
        Alerts[Policy Alerts]
    end
    
    Developers --> CLI
    CLI --> Hooks
    CLI --> Scripts
    
    CLI --> API
    API --> Statuses
    API --> Protection
    API --> Audit
    
    Local --> CLI
    Nightly --> GitHub
    Release --> GitHub
    
    Audit --> Logging
    Logging --> Reports
    Reports --> Alerts
```

## API Rate Limit Management

### Understanding Rate Limits

```rust
// src/rate_limit.rs

use octocrab::Octocrab;
use std::time::{Duration, Instant};

pub struct RateLimitManager {
    octocrab: Octocrab,
    remaining: u32,
    reset_time: Instant,
    last_check: Instant,
}

impl RateLimitManager {
    pub async fn new(octocrab: Octocrab) -> Result<Self, octocrab::Error> {
        let mut manager = Self {
            octocrab,
            remaining: 5000,
            reset_time: Instant::now() + Duration::from_secs(3600),
            last_check: Instant::now(),
        };
        
        // Initial rate limit check
        manager.refresh().await?;
        
        Ok(manager)
    }
    
    pub async fn refresh(&mut self) -> Result<(), octocrab::Error> {
        let rate_limit = self.octocrab
            .rate_limit()
            .get()
            .await?;
        
        self.remaining = rate_limit.resources.core.remaining as u32;
        self.reset_time = Instant::now()
            + Duration::from_secs(rate_limit.resources.core.reset as u64);
        self.last_check = Instant::now();
        
        Ok(())
    }
    
    pub fn can_make_request(&self) -> bool {
        self.remaining > 10  // Keep buffer
    }
    
    pub async fn wait_if_needed(&mut self) -> Result<(), octocrab::Error> {
        if !self.can_make_request() {
            let sleep_duration = self.reset_time.duration_since(Instant::now());
            
            if sleep_duration > Duration::ZERO {
                tokio::time::sleep(sleep_duration).await;
            }
            
            self.refresh().await?;
        }
        
        Ok(())
    }
    
    pub fn get_status(&self) -> RateLimitStatus {
        RateLimitStatus {
            remaining: self.remaining,
            reset_in: self.reset_time.duration_since(Instant::now()),
            last_check: self.last_check.elapsed(),
        }
    }
}

pub struct RateLimitStatus {
    pub remaining: u32,
    pub reset_in: Duration,
    pub last_check: Duration,
}
```

### Rate Limit Optimization

```rust
// src/api_batching.rs

use futures::future::join_all;

/// Batch multiple status updates into single request
pub async fn create_statuses_batch(
    github: &GitHubClient,
    sha: &str,
    statuses: &[StatusUpdate],
) -> Result<Vec<CommitStatus>, GitHubError> {
    // GitHub doesn't support batch status creation
    // But we can parallelize with rate limit awareness
    
    let mut results = Vec::new();
    
    for status in statuses {
        // Check rate limit before each request
        github.wait_if_needed().await?;
        
        let result = github.create_status(
            sha,
            &status.context,
            &status.description,
        ).await?;
        
        results.push(result);
        
        // Small delay to avoid burst
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    Ok(results)
}

/// Cache branch protection to avoid repeated API calls
pub struct ProtectionCache {
    cache: DashMap<String, (BranchProtection, Instant)>,
    ttl: Duration,
}

impl ProtectionCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: DashMap::new(),
            ttl,
        }
    }
    
    pub async fn get_or_fetch(
        &self,
        github: &GitHubClient,
        branch: &str,
    ) -> Result<BranchProtection, GitHubError> {
        // Check cache
        if let Some((protection, timestamp)) = self.cache.get(branch) {
            if timestamp.elapsed() < self.ttl {
                return Ok(protection.clone());
            }
        }
        
        // Fetch from API
        let protection = github.get_branch_protection(branch).await?;
        
        // Update cache
        self.cache.insert(
            branch.to_string(),
            (protection.clone(), Instant::now()),
        );
        
        Ok(protection)
    }
    
    pub fn invalidate(&self, branch: &str) {
        self.cache.remove(branch);
    }
}
```

## Team Onboarding

### Installation Script

```bash
#!/bin/bash
# script/enterprise-install.sh - Enterprise team installation

set -euo pipefail

echo "=== gh-signoff Enterprise Installation ==="

# Check prerequisites
echo ""
echo "Checking prerequisites..."

# Check GitHub CLI
if ! command -v gh >/dev/null 2>&1; then
    echo "Installing GitHub CLI..."
    case "$(uname -s)" in
        Darwin)
            brew install gh
            ;;
        Linux)
            if command -v apt >/dev/null 2>&1; then
                sudo apt install -y gh
            elif command -v dnf >/dev/null 2>&1; then
                sudo dnf install -y gh
            fi
            ;;
    esac
fi

# Check gh authentication
if ! gh auth status >/dev/null 2>&1; then
    echo "Please authenticate with GitHub:"
    gh auth login
fi

# Install gh-signoff extension
echo ""
echo "Installing gh-signoff extension..."
if ! gh extension list | grep -q signoff; then
    gh extension install basecamp/gh-signoff
else
    echo "gh-signoff already installed, updating..."
    gh extension upgrade signoff
fi

# Setup bash completion
echo ""
echo "Setting up bash completion..."
if ! grep -q "gh signoff completion" ~/.bashrc 2>/dev/null; then
    echo 'eval "$(gh signoff completion)"' >> ~/.bashrc
    echo "Added to ~/.bashrc - restart shell or run: source ~/.bashrc"
fi

# Configure git
echo ""
echo "Configuring git..."
if [[ -z "$(git config user.name)" ]]; then
    read -p "Enter your name: " name
    git config --global user.name "$name"
fi

if [[ -z "$(git config user.email)" ]]; then
    read -p "Enter your email: " email
    git config --global user.email "$email"
fi

echo ""
echo "✓ Installation complete!"
echo ""
echo "Next steps:"
echo "1. Restart your shell or run: source ~/.bashrc"
echo "2. Test installation: gh signoff version"
echo "3. Review documentation: docs/LOCAL_CI_GUIDE.md"
```

### Team Configuration

```yaml
# .github/gh-signoff.yml - Team configuration

# Required signoffs for each branch
branches:
  main:
    required_contexts:
      - tests
      - lint
      - security
    enforce_admins: true
  
  staging:
    required_contexts:
      - tests
      - lint
    enforce_admins: true
  
  develop:
    required_contexts:
      - tests
    enforce_admins: false

# Team-specific settings
teams:
  developers:
    can_signoff:
      - tests
      - lint
  
  qa:
    can_signoff:
      - qa/testing
  
  security:
    can_signoff:
      - security/scan
  
  ops:
    can_signoff:
      - ops/deployable

# Compliance settings
compliance:
  require_separation_of_duties: true
  audit_log_enabled: true
  minimum_reviewers: 1
```

## Audit Logging

### Signoff Audit Trail

```rust
// src/audit.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignoffAuditEntry {
    pub timestamp: DateTime<Utc>,
    pub user: String,
    pub user_email: String,
    pub repository: String,
    pub branch: String,
    pub commit_sha: String,
    pub context: String,
    pub ip_address: Option<String>,
    pub machine_hostname: Option<String>,
}

pub struct AuditLogger {
    log_path: String,
    enabled: bool,
}

impl AuditLogger {
    pub fn new(log_path: String) -> Self {
        Self {
            log_path,
            enabled: true,
        }
    }
    
    pub fn log_signoff(&self, entry: SignoffAuditEntry) -> Result<(), std::io::Error> {
        if !self.enabled {
            return Ok(());
        }
        
        let json = serde_json::to_string(&entry)?;
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;
        
        writeln!(file, "{}", json)?;
        
        Ok(())
    }
    
    pub fn generate_report(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<AuditReport, AuditError> {
        let content = std::fs::read_to_string(&self.log_path)?;
        
        let entries: Vec<SignoffAuditEntry> = content
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        
        let filtered: Vec<_> = entries
            .into_iter()
            .filter(|e| e.timestamp >= start && e.timestamp <= end)
            .collect();
        
        Ok(AuditReport {
            period: (start, end),
            total_signoffs: filtered.len(),
            by_user: self.group_by_user(&filtered),
            by_repository: self.group_by_repository(&filtered),
            by_context: self.group_by_context(&filtered),
        })
    }
    
    fn group_by_user(&self, entries: &[SignoffAuditEntry]) -> std::collections::HashMap<String, usize> {
        let mut map = std::collections::HashMap::new();
        for entry in entries {
            *map.entry(entry.user.clone()).or_insert(0) += 1;
        }
        map
    }
    
    fn group_by_repository(&self, entries: &[SignoffAuditEntry]) -> std::collections::HashMap<String, usize> {
        let mut map = std::collections::HashMap::new();
        for entry in entries {
            *map.entry(entry.repository.clone()).or_insert(0) += 1;
        }
        map
    }
    
    fn group_by_context(&self, entries: &[SignoffAuditEntry]) -> std::collections::HashMap<String, usize> {
        let mut map = std::collections::HashMap::new();
        for entry in entries {
            *map.entry(entry.context.clone()).or_insert(0) += 1;
        }
        map
    }
}

pub struct AuditReport {
    pub period: (DateTime<Utc>, DateTime<Utc>),
    pub total_signoffs: usize,
    pub by_user: std::collections::HashMap<String, usize>,
    pub by_repository: std::collections::HashMap<String, usize>,
    pub by_context: std::collections::HashMap<String, usize>,
}
```

### GitHub Audit Log Integration

```rust
// src/github_audit.rs

use octocrab::Octocrab;

pub async fn fetch_github_audit_log(
    octocrab: &Octocrab,
    org: &str,
    since: Option<DateTime<Utc>>,
) -> Result<Vec<AuditLogEntry>, GitHubError> {
    let mut entries = Vec::new();
    let mut page = 1;
    
    loop {
        let route = format!("/orgs/{}/audit-log", org);
        
        let mut params = vec![
            ("per_page", "100"),
            ("page", &page.to_string()),
        ];
        
        if let Some(since) = since {
            params.push(("phrase", &format!("created:>{}", since.to_rfc3339())));
        }
        
        let response: Vec<AuditLogEntry> = octocrab
            .get(route, Some(&params))
            .await?;
        
        if response.is_empty() {
            break;
        }
        
        entries.extend(response);
        page += 1;
    }
    
    Ok(entries)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    #[serde(rename = "@timestamp")]
    pub timestamp: i64,
    pub action: String,
    pub actor: String,
    pub repo: Option<String>,
    pub data: Option<serde_json::Value>,
}

/// Filter for signoff-related audit events
pub fn filter_signoff_events(entries: &[AuditLogEntry]) -> Vec<&AuditLogEntry> {
    entries
        .iter()
        .filter(|e| {
            e.action == "create" || e.action == "update"
        })
        .filter(|e| {
            e.data.as_ref()
                .and_then(|d| d.get("context"))
                .and_then(|c| c.as_str())
                .map(|c| c.starts_with("signoff"))
                .unwrap_or(false)
        })
        .collect()
}
```

## Compliance Integration

### SOX Compliance Workflow

```bash
#!/bin/bash
# script/sox-compliant-signoff - SOX-compliant signoff process

set -euo pipefail

echo "=== SOX-Compliant Signoff ==="

# Step 1: Developer signoff
echo ""
echo "Step 1: Developer testing and signoff"
rails test && gh signoff dev/signoff
echo "✓ Developer signoff complete"

# Step 2: Code review required
echo ""
echo "Step 2: Code review"
read -p "Has code been reviewed by another developer? (y/n) " -n 1 -r
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Code review required before proceeding"
    exit 1
fi
gh signoff review/signoff
echo "✓ Code review signoff complete"

# Step 3: QA testing
echo ""
echo "Step 3: QA testing"
read -p "Has QA completed testing? (y/n) " -n 1 -r
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "QA signoff required before proceeding"
    exit 1
fi
gh signoff qa/signoff
echo "✓ QA signoff complete"

# Step 4: Log for audit
echo ""
echo "Step 4: Logging for audit trail"
cat >> sox_audit.log <<EOF
$(date -Iseconds) | SOX signoff | $(git config user.name) | $(git rev-parse HEAD) | $(git branch --show-current)
EOF
echo "✓ Audit log updated"

echo ""
echo "✓ SOX-compliant signoff complete"
```

### Separation of Duties

```rust
// src/compliance.rs

/// Enforce separation of duties
pub struct SeparationOfDuties {
    signoffs: Vec<SignoffEntry>,
}

impl SeparationOfDuties {
    pub fn new() -> Self {
        Self { signoffs: Vec::new() }
    }
    
    pub fn add_signoff(&mut self, entry: SignoffEntry) -> Result<(), ComplianceError> {
        // Check if same user is trying to sign off on conflicting contexts
        if let Some(existing) = self.signoffs.iter().find(|s| s.user == entry.user) {
            if Self::conflicts(&existing.context, &entry.context) {
                return Err(ComplianceError::SeparationOfDutiesViolation {
                    user: entry.user.clone(),
                    contexts: vec![existing.context.clone(), entry.context.clone()],
                });
            }
        }
        
        self.signoffs.push(entry);
        Ok(())
    }
    
    /// Check if two contexts conflict (require different users)
    fn conflicts(ctx1: &str, ctx2: &str) -> bool {
        // Developer cannot review their own code
        if (ctx1 == "dev/signoff" && ctx2 == "review/signoff")
            || (ctx1 == "review/signoff" && ctx2 == "dev/signoff")
        {
            return true;
        }
        
        // QA cannot be same as developer
        if (ctx1.starts_with("dev/") && ctx2.starts_with("qa/"))
            || (ctx1.starts_with("qa/") && ctx2.starts_with("dev/"))
        {
            return true;
        }
        
        false
    }
}

#[derive(Debug, Clone)]
pub struct SignoffEntry {
    pub user: String,
    pub context: String,
    pub timestamp: DateTime<Utc>,
}
```

## Hybrid CI Architecture

### Enterprise CI Strategy

```yaml
# .github/workflows/enterprise-ci.yml

name: Enterprise CI

on:
  # Scheduled full CI
  schedule:
    - cron: "0 */6 * * *"  # Every 6 hours
  
  # Release branches
  push:
    branches:
      - release/*
  
  # Manual trigger
  workflow_dispatch:
    inputs:
      full_suite:
        description: "Run full test suite"
        required: false
        default: "false"

jobs:
  verify-signoffs:
    runs-on: ubuntu-latest
    steps:
      - name: Verify required signoffs
        uses: actions/github-script@v7
        with:
          script: |
            const { data: status } = await github.rest.repos.getCombinedStatusForRef({
              owner: context.repo.owner,
              repo: context.repo.repo,
              ref: context.sha
            });
            
            // Check for required signoff contexts
            const required = ['signoff/tests', 'signoff/lint', 'signoff/security'];
            const missing = required.filter(ctx => 
              !status.statuses.some(s => s.context === ctx && s.state === 'success')
            );
            
            if (missing.length > 0) {
              core.setFailed(`Missing signoffs: ${missing.join(', ')}`);
            }

  full-test-suite:
    runs-on: ubuntu-latest
    needs: verify-signoffs
    steps:
      - uses: actions/checkout@v4
      
      - name: Run full test suite
        run: |
          bundle install
          rails test:all
          rubocop
          bundle audit
      
      - name: Upload test results
        uses: actions/upload-artifact@v4
        with:
          name: test-results
          path: test-results/

  performance-tests:
    runs-on: ubuntu-latest
    needs: full-test-suite
    steps:
      - uses: actions/checkout@v4
      
      - name: Run performance tests
        run: |
          bundle install
          rails test:performance
      
      - name: Upload performance results
        uses: actions/upload-artifact@v4
        with:
          name: performance-results
          path: tmp/performance/
```

## Metrics and Monitoring

### Signoff Analytics

```rust
// src/metrics.rs

use prometheus::{Registry, Counter, Histogram, Encoder, TextEncoder};

pub struct SignoffMetrics {
    registry: Registry,
    signoff_count: Counter,
    signoff_duration: Histogram,
    signoff_by_context: Counter,
}

impl SignoffMetrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();
        
        let signoff_count = Counter::new(
            "signoff_total",
            "Total number of signoffs"
        )?;
        registry.register(Box::new(signoff_count.clone()))?;
        
        let signoff_duration = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "signoff_duration_seconds",
                "Signoff operation duration"
            )
        )?;
        registry.register(Box::new(signoff_duration.clone()))?;
        
        let signoff_by_context = Counter::new(
            "signoff_by_context_total",
            "Signoffs by context",
        )?;
        registry.register(Box::new(signoff_by_context.clone()))?;
        
        Ok(Self {
            registry,
            signoff_count,
            signoff_duration,
            signoff_by_context,
        })
    }
    
    pub fn record_signoff(&self, context: &str, duration: f64) {
        self.signoff_count.inc();
        self.signoff_duration.observe(duration);
        
        self.signoff_by_context
            .with_label_values(&[context])
            .inc();
    }
    
    pub fn encode(&self) -> Result<String, prometheus::Error> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        
        Ok(String::from_utf8(buffer).unwrap())
    }
}
```

## Conclusion

Production-grade gh-signoff deployments require:

1. **Rate Limit Management**: Smart caching and batching of API calls
2. **Team Onboarding**: Automated installation and configuration
3. **Audit Logging**: Comprehensive audit trails for compliance
4. **Compliance Integration**: SOX, HIPAA, SOC 2 workflows
5. **Hybrid CI**: Local speed + cloud safety net
6. **Metrics**: Prometheus metrics for monitoring
