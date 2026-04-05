---
source: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/gh-signoff
repository: github.com/basecamp/gh-signoff
explored_at: 2026-04-05
focus: Rust implementation of gh-signoff patterns - GitHub API, commit statuses, branch protection, CLI design
---

# Rust Revision: gh-signoff in Rust

## Overview

This document translates gh-signoff's local CI patterns from Bash to Rust, covering GitHub API integration via octocrab, CLI design with clap, and production-grade error handling.

## Architecture Comparison

### Bash (Original gh-signoff)

```
gh-signoff (Bash)
    ├── gh CLI (GitHub API calls)
    ├── git commands
    ├── jq (JSON parsing)
    └── bash completion
```

### Rust (Revision)

```
gh-signoff-rs (Rust)
    ├── octocrab (GitHub API)
    ├── git2 (git operations)
    ├── clap (CLI framework)
    ├── serde_json (JSON parsing)
    ├── tokio (async runtime)
    └── shell completion via clap_complete
```

## Core Data Structures

```rust
// src/types.rs

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Signoff context (e.g., "signoff", "signoff/tests")
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SignoffContext(pub String);

impl SignoffContext {
    pub fn default() -> Self {
        Self("signoff".to_string())
    }
    
    pub fn partial(category: &str) -> Self {
        Self(format!("signoff/{}", category))
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// GitHub commit status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitStatus {
    pub url: String,
    pub id: u64,
    pub state: StatusState,
    pub context: String,
    pub description: Option<String>,
    pub target_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Status state enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StatusState {
    Success,
    Failure,
    Pending,
    Error,
}

impl std::fmt::Display for StatusState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StatusState::Success => write!(f, "✓"),
            StatusState::Failure => write!(f, "✗"),
            StatusState::Pending => write!(f, "⟳"),
            StatusState::Error => write!(f, "⚠"),
        }
    }
}

/// Combined status for a commit
#[derive(Debug, Clone)]
pub struct CombinedStatus {
    pub state: StatusState,
    pub sha: String,
    pub total_count: usize,
    pub statuses: Vec<CommitStatus>,
}

impl CombinedStatus {
    pub fn signoff_statuses(&self) -> Vec<&CommitStatus> {
        self.statuses
            .iter()
            .filter(|s| s.context.starts_with("signoff"))
            .collect()
    }
    
    pub fn is_complete(&self, required_contexts: &[SignoffContext]) -> bool {
        for required in required_contexts {
            let has_success = self.signoff_statuses()
                .iter()
                .any(|s| s.context == required.as_str() && s.state == StatusState::Success);
            
            if !has_success {
                return false;
            }
        }
        true
    }
}

/// Branch protection rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchProtection {
    pub url: String,
    pub required_status_checks: Option<RequiredStatusChecks>,
    pub enforce_admins: bool,
    pub required_pull_request_reviews: Option<PRReviewRequirements>,
    pub restrictions: Option<BranchRestrictions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredStatusChecks {
    pub strict: bool,
    pub contexts: Vec<String>,
    pub checks: Vec<StatusCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusCheck {
    pub context: String,
    pub app_id: Option<u64>,
}

/// Signoff configuration
#[derive(Debug, Clone)]
pub struct SignoffConfig {
    pub branch: String,
    pub required_contexts: Vec<SignoffContext>,
}
```

## GitHub API Client

```rust
// src/github.rs

use octocrab::{Octocrab, models, APIError};
use std::sync::Arc;

pub struct GitHubClient {
    octocrab: Arc<Octocrab>,
    owner: String,
    repo: String,
}

impl GitHubClient {
    pub fn new() -> Result<Self, octocrab::Error> {
        let octocrab = Octocrab::builder()
            .personal_token(std::env::var("GITHUB_TOKEN")?)
            .build()?;
        
        // Get owner/repo from current directory
        let (owner, repo) = Self::parse_repo()?;
        
        Ok(Self {
            octocrab: Arc::new(octocrab),
            owner,
            repo,
        })
    }
    
    fn parse_repo() -> Result<(String, String), GitError> {
        let repo = git2::Repository::open_from_env()?;
        let remote = repo.find_remote("origin")?;
        let url = remote.url().ok_or("No remote URL")?;
        
        // Parse owner/repo from URL
        // https://github.com/owner/repo.git → (owner, repo)
        // git@github.com:owner/repo.git → (owner, repo)
        let parts: Vec<&str> = url
            .trim_end_matches(".git")
            .split('/')
            .collect();
        
        if parts.len() < 2 {
            return Err("Invalid repo URL".into());
        }
        
        let owner = parts[parts.len() - 2]
            .trim_start_matches("git@github.com:")
            .to_string();
        let repo = parts[parts.len() - 1].to_string();
        
        Ok((owner, repo))
    }
    
    /// Create a commit status
    pub async fn create_status(
        &self,
        sha: &str,
        context: &SignoffContext,
        description: &str,
    ) -> Result<CommitStatus, GitHubError> {
        let status = self.octocrab
            .repos(&self.owner, &self.repo)
            .create_status(sha, models::StatusState::Success)
            .context(context.as_str())
            .description(description)
            .send()
            .await?;
        
        Ok(CommitStatus {
            url: status.url,
            id: status.id.0,
            state: StatusState::Success,
            context: status.context,
            description: status.description,
            target_url: status.target_url,
            created_at: status.created_at,
            updated_at: status.updated_at,
        })
    }
    
    /// Get combined status for a commit
    pub async fn get_combined_status(
        &self,
        sha: &str,
    ) -> Result<CombinedStatus, GitHubError> {
        let status = self.octocrab
            .repos(&self.owner, &self.repo)
            .combined_status_for_ref(&format!("refs/heads/{}", sha))
            .send()
            .await?;
        
        Ok(CombinedStatus {
            state: status.state.into(),
            sha: status.sha,
            total_count: status.total_count,
            statuses: status.statuses.into_iter().map(|s| CommitStatus {
                url: s.url,
                id: s.id.0,
                state: s.state.into(),
                context: s.context,
                description: s.description,
                target_url: s.target_url,
                created_at: s.created_at,
                updated_at: s.updated_at,
            }).collect(),
        })
    }
    
    /// Get branch protection rules
    pub async fn get_branch_protection(
        &self,
        branch: &str,
    ) -> Result<BranchProtection, GitHubError> {
        let route = format!(
            "/repos/{}/{}/branches/{}/protection",
            self.owner, self.repo, branch
        );
        
        let response: BranchProtection = self.octocrab
            .get(route, None::<&()>)
            .await?;
        
        Ok(response)
    }
    
    /// Update branch protection
    pub async fn update_branch_protection(
        &self,
        branch: &str,
        contexts: &[SignoffContext],
    ) -> Result<BranchProtection, GitHubError> {
        let route = format!(
            "/repos/{}/{}/branches/{}/protection",
            self.owner, self.repo, branch
        );
        
        let body = serde_json::json!({
            "required_status_checks": {
                "strict": false,
                "contexts": contexts.iter().map(|c| c.as_str()).collect::<Vec<_>>(),
            },
            "enforce_admins": null,
            "required_pull_request_reviews": null,
            "restrictions": null,
        });
        
        let response: BranchProtection = self.octocrab
            .put(route, Some(&body))
            .await?;
        
        Ok(response)
    }
    
    /// Get default branch
    pub async fn get_default_branch(&self) -> Result<String, GitHubError> {
        let repo = self.octocrab
            .repos(&self.owner, &self.repo)
            .get()
            .await?;
        
        Ok(repo.default_branch)
    }
}
```

## Repository State Checking

```rust
// src/git_utils.rs

use git2::{Repository, StatusOptions, StatusShow};

pub struct GitUtils {
    repo: Repository,
}

impl GitUtils {
    pub fn open() -> Result<Self, GitError> {
        let repo = Repository::open_from_env()?;
        Ok(Self { repo })
    }
    
    /// Check if repository is clean (no uncommitted changes)
    pub fn is_clean(&self) -> Result<bool, GitError> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true);
        opts.show(StatusShow::IndexAndWorkdir);
        
        let statuses = self.repo.statuses(Some(&mut opts))?;
        
        // If any status entries, repo is dirty
        if statuses.len() > 0 {
            return Ok(false);
        }
        
        Ok(true)
    }
    
    /// Check if current branch has upstream tracking
    pub fn has_upstream(&self) -> Result<bool, GitError> {
        let head = self.repo.head()?;
        let branch = head.shorthand().ok_or("No current branch")?;
        
        let branch = self.repo.find_branch(branch, git2::BranchType::Local)?;
        let upstream = branch.upstream();
        
        Ok(upstream.is_ok())
    }
    
    /// Check if there are unpushed commits
    pub fn has_unpushed(&self) -> Result<bool, GitError> {
        let repo = &self.repo;
        
        let head = repo.head()?;
        let head_commit = repo.find_commit(head.target().ok_or("No HEAD")?)?;
        
        let branch = head.shorthand().ok_or("No current branch")?;
        let branch = repo.find_branch(branch, git2::BranchType::Local)?;
        let upstream = branch.upstream()?;
        
        let upstream_commit = repo.find_commit(
            upstream.get().target().ok_or("No upstream target")?
        )?;
        
        // Check if HEAD is ancestor of upstream
        let oid = head_commit.id();
        let upstream_oid = upstream_commit.id();
        
        if repo.graph_descendant_of(upstream_oid, &oid)? {
            // HEAD is ancestor of upstream - no unpushed commits
            Ok(false)
        } else {
            // HEAD has commits not in upstream
            Ok(true)
        }
    }
    
    /// Get current commit SHA
    pub fn head_sha(&self) -> Result<String, GitError> {
        let head = self.repo.head()?;
        let sha = head.target().ok_or("No HEAD target")?;
        Ok(sha.to_string())
    }
    
    /// Get current user name
    pub fn user_name(&self) -> Result<String, GitError> {
        let config = self.repo.config()?;
        let name = config.get_string("user.name")?;
        Ok(name)
    }
}

/// Full clean check (combines all checks)
pub fn verify_clean_repo() -> Result<CleanStatus, GitError> {
    let utils = GitUtils::open()?;
    
    if !utils.is_clean()? {
        return Ok(CleanStatus::Dirty);
    }
    
    if !utils.has_upstream()? {
        return Ok(CleanStatus::NoUpstream);
    }
    
    if utils.has_unpushed()? {
        return Ok(CleanStatus::Unpushed);
    }
    
    Ok(CleanStatus::Clean)
}

pub enum CleanStatus {
    Clean,
    Dirty,
    NoUpstream,
    Unpushed,
}
```

## CLI Implementation

```rust
// src/cli.rs

use clap::{Parser, Subcommand};
use crate::github::GitHubClient;
use crate::git_utils::{verify_clean_repo, CleanStatus};

#[derive(Parser)]
#[command(name = "gh signoff")]
#[command(about = "Sign off on commits without CI infrastructure")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// Force signoff (ignore uncommitted/unpushed changes)
    #[arg(short = 'f', long)]
    force: bool,
    
    /// Branch to operate on
    #[arg(long)]
    branch: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Sign off on the current commit
    Create {
        /// Contexts for partial signoff
        contexts: Vec<String>,
    },
    
    /// Install signoff requirement
    Install {
        /// Contexts for partial signoff
        contexts: Vec<String>,
    },
    
    /// Uninstall signoff requirement
    Uninstall,
    
    /// Check if signoff is required
    Check {
        /// Contexts to check
        contexts: Vec<String>,
    },
    
    /// Show signoff status
    Status,
    
    /// Show version
    Version,
    
    /// Generate shell completion
    Completion {
        /// Shell type
        shell: clap_complete::Shell,
    },
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let github = GitHubClient::new()?;
    
    match cli.command {
        Some(Commands::Create { contexts }) => {
            cmd_create(&github, &contexts, cli.force).await?;
        }
        Some(Commands::Install { contexts }) => {
            cmd_install(&github, &contexts, cli.branch.as_deref()).await?;
        }
        Some(Commands::Uninstall) => {
            cmd_uninstall(&github, cli.branch.as_deref()).await?;
        }
        Some(Commands::Check { contexts }) => {
            cmd_check(&github, &contexts, cli.branch.as_deref()).await?;
        }
        Some(Commands::Status) => {
            cmd_status(&github, cli.branch.as_deref()).await?;
        }
        Some(Commands::Version) => {
            println!("gh-signoff {}", env!("CARGO_PKG_VERSION"));
        }
        Some(Commands::Completion { shell }) => {
            generate_completion(shell);
        }
        None => {
            // Default to create
            cmd_create(&github, &[], cli.force).await?;
        }
    }
    
    Ok(())
}

async fn cmd_create(
    github: &GitHubClient,
    contexts: &[String],
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Verify clean repo unless forced
    if !force {
        match verify_clean_repo()? {
            CleanStatus::Clean => {}
            CleanStatus::Dirty => {
                return Err("Repository has uncommitted changes".into());
            }
            CleanStatus::NoUpstream => {
                return Err("Current branch has no upstream".into());
            }
            CleanStatus::Unpushed => {
                return Err("Repository has unpushed changes".into());
            }
        }
    }
    
    // Get user name and SHA
    let utils = GitUtils::open()?;
    let user = utils.user_name()?;
    let sha = utils.head_sha()?;
    
    // Determine contexts
    let contexts: Vec<SignoffContext> = if contexts.is_empty() {
        vec![SignoffContext::default()]
    } else {
        contexts.iter()
            .map(|c| SignoffContext::partial(c))
            .collect()
    };
    
    // Create statuses
    for context in &contexts {
        github.create_status(
            &sha,
            context,
            &format!("{} signed off", user),
        ).await?;
        
        println!("✓ Signed off on {} for {}", &sha[..8], context.as_str());
    }
    
    Ok(())
}

async fn cmd_install(
    github: &GitHubClient,
    contexts: &[String],
    branch: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Determine branch
    let branch = branch.unwrap_or(&github.get_default_branch().await?);
    
    // Determine contexts
    let contexts: Vec<SignoffContext> = if contexts.is_empty() {
        vec![SignoffContext::default()]
    } else {
        contexts.iter()
            .map(|c| SignoffContext::partial(c))
            .collect()
    };
    
    // Update branch protection
    github.update_branch_protection(branch, &contexts).await?;
    
    println!("✓ GitHub {} branch now requires signoff", branch);
    
    Ok(())
}

async fn cmd_check(
    github: &GitHubClient,
    contexts: &[String],
    branch: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let branch = branch.unwrap_or(&github.get_default_branch().await?);
    
    let protection = github.get_branch_protection(branch).await?;
    
    let required = protection
        .required_status_checks
        .map(|r| r.contexts)
        .unwrap_or_default();
    
    let contexts: Vec<SignoffContext> = if contexts.is_empty() {
        vec![SignoffContext::default()]
    } else {
        contexts.iter()
            .map(|c| SignoffContext::partial(c))
            .collect()
    };
    
    for context in &contexts {
        if required.contains(&context.as_str().to_string()) {
            println!("✓ GitHub {} branch requires signoff on {}", branch, context.as_str());
        } else {
            println!("✗ GitHub {} branch does not require signoff on {}", branch, context.as_str());
        }
    }
    
    Ok(())
}

async fn cmd_status(
    github: &GitHubClient,
    branch: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let utils = GitUtils::open()?;
    let sha = utils.head_sha()?;
    
    let branch = branch.unwrap_or(&github.get_default_branch().await?);
    
    let status = github.get_combined_status(&sha).await?;
    let protection = github.get_branch_protection(branch).await?;
    
    // Get required contexts
    let required: Vec<SignoffContext> = protection
        .required_status_checks
        .map(|r| r.contexts)
        .unwrap_or_default()
        .iter()
        .filter(|c| c.starts_with("signoff"))
        .map(|c| SignoffContext(c.clone()))
        .collect();
    
    // Check each required context
    for context in &required {
        let display = context.as_str()
            .trim_start_matches("signoff/");
        
        let has_success = status.statuses
            .iter()
            .any(|s| s.context == context.as_str() && s.state == StatusState::Success);
        
        if has_success {
            println!("✓ {}", display);
        } else {
            println!("✗ {}", display);
        }
    }
    
    Ok(())
}

fn generate_completion(shell: clap_complete::Shell) {
    use clap::CommandFactory;
    
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    
    clap_complete::generate(
        shell,
        &mut cmd,
        name,
        &mut std::io::stdout(),
    );
}
```

## Conclusion

The Rust implementation of gh-signoff provides:

1. **Type Safety**: Compile-time checking of contexts and states
2. **Async API**: Non-blocking GitHub API calls via octocrab
3. **Better Error Handling**: Explicit error types with context
4. **Git Integration**: Full git2 integration for state checking
5. **Shell Completion**: Automatic completion generation via clap_complete
6. **Single Binary**: Easy distribution like the Bash version
