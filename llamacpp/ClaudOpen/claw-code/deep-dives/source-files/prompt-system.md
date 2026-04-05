# System Prompt Construction Deep-Dive

A comprehensive analysis of how Claw Code builds system prompts, discovers project context, and integrates git information.

## Table of Contents

1. [Overview](#overview)
2. [SystemPromptBuilder](#systempromptbuilder)
3. [Project Context Discovery](#project-context-discovery)
4. [Instruction Files](#instruction-files)
5. [Git Integration](#git-integration)
6. [Prompt Assembly](#prompt-assembly)
7. [Dynamic Boundary](#dynamic-boundary)
8. [Testing](#testing)

---

## Overview

The system prompt is the foundation of Claw Code's behavior. It instructs the model about:

- Available tools and their schemas
- Project-specific context and conventions
- Git status and recent changes
- User preferences and constraints
- Permission boundaries

**Location**: `rust/crates/runtime/src/prompt.rs`

**Key Components**:
- `SystemPromptBuilder` - Builder pattern for prompt construction
- `ProjectContext` - Discovered project metadata
- `InstructionFile` - Parsed CLAUDE.md and similar files

---

## SystemPromptBuilder

### Structure

```rust
#[derive(Debug, Clone)]
pub struct SystemPromptBuilder {
    working_dir: PathBuf,
    permission_mode: PermissionMode,
    model: String,
    instructions: Vec<String>,
    project_context: Option<ProjectContext>,
    git_info: Option<GitInfo>,
    custom_tools: Vec<ToolDefinition>,
    mcp_tools: Vec<String>,
}

impl SystemPromptBuilder {
    pub fn new(working_dir: PathBuf) -> Self {
        Self {
            working_dir,
            permission_mode: PermissionMode::WorkspaceWrite,
            model: String::from("claude-opus-4-6"),
            instructions: Vec::new(),
            project_context: None,
            git_info: None,
            custom_tools: Vec::new(),
            mcp_tools: Vec::new(),
        }
    }
}
```

### Builder Methods

```rust
impl SystemPromptBuilder {
    /// Set permission mode
    pub fn permission_mode(mut self, mode: PermissionMode) -> Self {
        self.permission_mode = mode;
        self
    }

    /// Set model name
    pub fn model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    /// Add instruction content
    pub fn add_instruction(mut self, instruction: String) -> Self {
        self.instructions.push(instruction);
        self
    }

    /// Set discovered project context
    pub fn project_context(mut self, context: ProjectContext) -> Self {
        self.project_context = Some(context);
        self
    }

    /// Set git information
    pub fn git_info(mut self, info: GitInfo) -> Self {
        self.git_info = Some(info);
        self
    }

    /// Add custom tool definitions
    pub fn add_tool(mut self, tool: ToolDefinition) -> Self {
        self.custom_tools.push(tool);
        self
    }

    /// Add MCP tool names
    pub fn add_mcp_tool(mut self, tool_name: String) -> Self {
        self.mcp_tools.push(tool_name);
        self
    }

    /// Build the final system prompt
    pub fn build(self) -> String {
        let mut prompt = String::new();

        // 1. Core system instructions
        prompt.push_str(&self.build_core_instructions());

        // 2. Project context
        if let Some(context) = &self.project_context {
            prompt.push_str("\n\n");
            prompt.push_str(&self.build_project_context(context));
        }

        // 3. Instruction files
        if !self.instructions.is_empty() {
            prompt.push_str("\n\n");
            prompt.push_str(&self.build_instructions_section());
        }

        // 4. Git information
        if let Some(info) = &self.git_info {
            prompt.push_str("\n\n");
            prompt.push_str(&self.build_git_section(info));
        }

        // 5. Tool definitions
        prompt.push_str("\n\n");
        prompt.push_str(&self.build_tools_section());

        // 6. Permission boundaries
        prompt.push_str("\n\n");
        prompt.push_str(&self.build_permission_section());

        prompt
    }
}
```

---

## Project Context Discovery

### ProjectContext Structure

```rust
#[derive(Debug, Clone)]
pub struct ProjectContext {
    /// Path to project root (where .git/ or config found)
    pub root: PathBuf,

    /// Project name from package.json, Cargo.toml, etc.
    pub name: Option<String>,

    /// Project type detected
    pub project_type: Option<ProjectType>,

    /// Discovered instruction files
    pub instruction_files: Vec<InstructionFile>,

    /// Ancestor chain from working directory to root
    pub ancestor_chain: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectType {
    Rust,      // Cargo.toml
    Node,      // package.json
    Python,    // setup.py or pyproject.toml
    Go,        // go.mod
    TypeScript,// tsconfig.json
    Mixed,     // Multiple
}
```

### Discovery Algorithm

```rust
impl ProjectContext {
    pub fn discover(working_dir: &Path) -> io::Result<Option<Self>> {
        let mut ancestor_chain = Vec::new();
        let mut current = working_dir.to_path_buf();
        let mut instruction_files = Vec::new();
        let mut project_type: Option<ProjectType> = None;
        let mut project_name: Option<String> = None;
        let mut root: Option<PathBuf> = None;

        // Walk up directory tree
        loop {
            ancestor_chain.push(current.clone());

            // Check for .git directory (project root)
            if current.join(".git").exists() {
                root = Some(current.clone());
            }

            // Check for project markers
            if current.join("Cargo.toml").exists() {
                project_type = Some(ProjectType::Rust);
                project_name = Self::read_cargo_name(&current.join("Cargo.toml"))?;
                if root.is_none() {
                    root = Some(current.clone());
                }
            }

            if current.join("package.json").exists() {
                project_type = match project_type {
                    None => Some(ProjectType::Node),
                    Some(_) => Some(ProjectType::Mixed),
                };
                project_name = Self::read_package_name(&current.join("package.json"))?;
                if root.is_none() {
                    root = Some(current.clone());
                }
            }

            if current.join("pyproject.toml").exists() || current.join("setup.py").exists() {
                project_type = match project_type {
                    None => Some(ProjectType::Python),
                    Some(_) => Some(ProjectType::Mixed),
                };
                if root.is_none() {
                    root = Some(current.clone());
                }
            }

            // Check for CLAUDE.md instruction file
            if current.join("CLAUDE.md").exists() {
                let content = fs::read_to_string(current.join("CLAUDE.md"))?;
                instruction_files.push(InstructionFile {
                    path: current.join("CLAUDE.md"),
                    content,
                    depth: ancestor_chain.len() - 1,
                });
            }

            // Check for .claude/commands/ directory
            let claude_commands = current.join(".claude").join("commands");
            if claude_commands.exists() {
                instruction_files.extend(Self::read_claude_commands(&claude_commands)?);
            }

            // Stop at filesystem root
            let parent = match current.parent() {
                Some(p) => p.to_path_buf(),
                None => break,
            };

            if parent == current {
                break;
            }

            current = parent;
        }

        if root.is_none() {
            return Ok(None);  // No project root found
        }

        Ok(Some(Self {
            root: root.unwrap(),
            name: project_name,
            project_type,
            instruction_files,
            ancestor_chain,
        }))
    }

    fn read_cargo_name(path: &Path) -> io::Result<Option<String>> {
        let content = fs::read_to_string(path)?;
        // Simple TOML parsing for name = "..."
        for line in content.lines() {
            if let Some(name) = line.strip_prefix("name = \"") {
                if let Some(name) = name.strip_suffix('"') {
                    return Ok(Some(name.to_string()));
                }
            }
        }
        Ok(None)
    }

    fn read_package_name(path: &Path) -> io::Result<Option<String>> {
        let content = fs::read_to_string(path)?;
        let json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(json["name"].as_str().map(String::from))
    }

    fn read_claude_commands(dir: &Path) -> io::Result<Vec<InstructionFile>> {
        let mut files = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("md") {
                let content = fs::read_to_string(&path)?;
                files.push(InstructionFile {
                    path,
                    content,
                    depth: 0,
                });
            }
        }
        Ok(files)
    }
}
```

### InstructionFile Structure

```rust
#[derive(Debug, Clone)]
pub struct InstructionFile {
    pub path: PathBuf,
    pub content: String,
    /// Depth in ancestor chain (0 = working directory)
    pub depth: usize,
}
```

---

## Instruction Files

### Content Limits

```rust
/// Maximum bytes per instruction file
const MAX_INSTRUCTION_FILE_BYTES: usize = 4000;

/// Maximum total bytes for all instruction files
const MAX_TOTAL_INSTRUCTION_BYTES: usize = 12000;

/// Truncate instruction content if needed
fn truncate_instruction(content: &str, max_bytes: usize) -> String {
    if content.len() <= max_bytes {
        return content.to_string();
    }

    // Truncate at character boundary
    let mut truncated = content;
    while truncated.len() > max_bytes {
        truncated = &truncated[..truncated.len() - 1];
    }

    // Ensure we don't end in middle of character
    while !truncated.is_char_boundary(truncated.len()) {
        truncated = &truncated[..truncated.len() - 1];
    }

    format!("{}...\n\n[truncated]", truncated)
}
```

### Building Instructions Section

```rust
impl SystemPromptBuilder {
    fn build_instructions_section(&self) -> String {
        let mut section = String::from("<instructions>\n");

        let mut total_bytes = 0;
        let mut file_count = 0;

        // Sort by depth (closest first)
        let mut sorted_files: Vec<_> = self.project_context
            .as_ref()
            .map(|ctx| ctx.instruction_files.clone())
            .unwrap_or_default();
        sorted_files.sort_by_key(|f| f.depth);

        for file in sorted_files {
            let remaining_bytes = MAX_TOTAL_INSTRUCTION_BYTES.saturating_sub(total_bytes);
            if remaining_bytes == 0 {
                section.push_str("<!-- Additional instruction files were truncated due to token limits -->\n");
                break;
            }

            let file_bytes = std::cmp::min(file.content.len(), remaining_bytes)
                .min(MAX_INSTRUCTION_FILE_BYTES);
            let content = truncate_instruction(&file.content, file_bytes);

            section.push_str(&format!(
                "<!-- From: {} -->\n\n{}\n\n",
                file.path.display(),
                content
            ));

            total_bytes += content.len();
            file_count += 1;
        }

        section.push_str("</instructions>");
        section
    }
}
```

---

## Git Integration

### GitInfo Structure

```rust
#[derive(Debug, Clone)]
pub struct GitInfo {
    /// Current branch name
    pub branch: String,

    /// Git status output (porcelain format)
    pub status: String,

    /// Recent commits (hash + subject)
    pub recent_commits: Vec<GitCommit>,

    /// Git diff of working directory
    pub diff: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GitCommit {
    pub hash: String,
    pub subject: String,
    pub author: String,
    pub date: String,
}
```

### Git Information Collection

```rust
impl GitInfo {
    pub fn collect(working_dir: &Path) -> Option<Self> {
        // Check if we're in a git repository
        let git_dir = working_dir.join(".git");
        if !git_dir.exists() && !working_dir.ancestors().any(|p| p.join(".git").exists()) {
            return None;
        }

        Some(Self {
            branch: Self::get_branch(working_dir)?,
            status: Self::get_status(working_dir)?,
            recent_commits: Self::get_recent_commits(working_dir)?,
            diff: Self::get_diff(working_dir),
        })
    }

    fn get_branch(working_dir: &Path) -> Option<String> {
        let output = std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(working_dir)
            .output()
            .ok()?;

        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            None
        }
    }

    fn get_status(working_dir: &Path) -> Option<String> {
        let output = std::process::Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(working_dir)
            .output()
            .ok()?;

        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            None
        }
    }

    fn get_recent_commits(working_dir: &Path) -> Option<Vec<GitCommit>> {
        let output = std::process::Command::new("git")
            .args([
                "log",
                "--format=%H|%s|%an|%ad",
                "--date=short",
                "-n",
                "10",
            ])
            .current_dir(working_dir)
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let commits = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                let parts: Vec<_> = line.split('|').collect();
                if parts.len() == 4 {
                    Some(GitCommit {
                        hash: parts[0].to_string(),
                        subject: parts[1].to_string(),
                        author: parts[2].to_string(),
                        date: parts[3].to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Some(commits)
    }

    fn get_diff(working_dir: &Path) -> Option<String> {
        let output = std::process::Command::new("git")
            .args(["diff", "HEAD"])
            .current_dir(working_dir)
            .output()
            .ok()?;

        if output.status.success() {
            let diff = String::from_utf8_lossy(&output.stdout).to_string();
            // Limit diff size
            if diff.len() > 10000 {
                Some(diff[..10000].to_string() + "\n\n[diff truncated]")
            } else {
                Some(diff)
            }
        } else {
            None
        }
    }
}
```

### Building Git Section

```rust
impl SystemPromptBuilder {
    fn build_git_section(&self, info: &GitInfo) -> String {
        let mut section = String::from("<git_info>\n");

        // Branch
        section.push_str(&format!("Branch: {}\n\n", info.branch));

        // Status
        if !info.status.is_empty() {
            section.push_str("Status:\n```\n");
            section.push_str(&info.status);
            section.push_str("```\n\n");
        }

        // Recent commits
        if !info.recent_commits.is_empty() {
            section.push_str("Recent commits:\n");
            for commit in &info.recent_commits {
                section.push_str(&format!(
                    "- {} {} ({}, {})\n",
                    &commit.hash[..7],
                    commit.subject,
                    commit.author,
                    commit.date
                ));
            }
            section.push('\n');
        }

        // Diff (if available and not too large)
        if let Some(diff) = &info.diff {
            if diff.len() < 5000 {
                section.push_str("Current diff:\n```diff\n");
                section.push_str(diff);
                section.push_str("```\n");
            }
        }

        section.push_str("</git_info>");
        section
    }
}
```

---

## Prompt Assembly

### Core Instructions

```rust
impl SystemPromptBuilder {
    fn build_core_instructions(&self) -> String {
        format!(
            include_str!("system_prompt_template.txt"),
            model = self.model,
            permission_mode = format!("{:?}", self.permission_mode),
        )
    }
}
```

### System Prompt Template

```
You are CLAW (Command Line AI Worker), an AI assistant running in a terminal environment.

## Your Capabilities

You have access to the following tools:
- Bash command execution
- File reading and writing
- Grep search
- Glob pattern matching
- Web requests (curl)
- And more...

## Response Format

When you need to use a tool, respond with a tool_use block:

<tool_use>
<name>tool_name</name>
<input>{"key": "value"}</input>
</tool_use>

## Current Configuration

Model: {model}
Permission Mode: {permission_mode}

{dynamic_boundary}
```

### Building Tools Section

```rust
impl SystemPromptBuilder {
    fn build_tools_section(&self) -> String {
        let mut section = String::from("<tools>\n");

        // Built-in tools
        for tool in &self.custom_tools {
            section.push_str(&format_tool_definition(tool));
        }

        // MCP tools
        for tool_name in &self.mcp_tools {
            section.push_str(&format!("- MCP Tool: {}\n", tool_name));
        }

        section.push_str("</tools>");
        section
    }
}

fn format_tool_definition(tool: &ToolDefinition) -> String {
    format!(
        "<tool>\n<name>{}</name>\n<description>{}</description>\n<input_schema>{}</input_schema>\n</tool>\n\n",
        tool.name,
        tool.description,
        serde_json::to_string_pretty(&tool.input_schema).unwrap_or_default()
    )
}
```

### Permission Section

```rust
impl SystemPromptBuilder {
    fn build_permission_section(&self) -> String {
        let permissions = match self.permission_mode {
            PermissionMode::ReadOnly => {
                "You are in READ_ONLY mode. You can only use tools that read data:\n\
                 - read_file, glob, grep, bash (read-only commands)\n\
                 You CANNOT write files, modify code, or execute destructive commands."
            }
            PermissionMode::WorkspaceWrite => {
                "You are in WORKSPACE_WRITE mode. You can:\n\
                 - Read and write files within the project directory\n\
                 - Execute bash commands\n\
                 - Use all development tools\n\
                 You CANNOT access files outside the workspace or run dangerous system commands."
            }
            PermissionMode::DangerFullAccess => {
                "You are in DANGER_FULL_ACCESS mode. You have full system access.\n\
                 Exercise caution with destructive operations."
            }
        };

        format!("<permissions>\n{}\n</permissions>", permissions)
    }
}
```

---

## Dynamic Boundary

### SYSTEM_PROMPT_DYNAMIC_BOUNDARY

```rust
/// Marker for where dynamic content begins in the system prompt
pub const SYSTEM_PROMPT_DYNAMIC_BOUNDARY: &str = "<!-- DYNAMIC_CONTENT -->";

/// Calculate estimated token count for system prompt
pub fn estimate_tokens(prompt: &str) -> usize {
    // Rough estimate: 4 characters per token
    prompt.len() / 4
}

/// Check if system prompt needs compaction
pub fn needs_compaction(prompt: &str, max_tokens: usize) -> bool {
    estimate_tokens(prompt) > max_tokens
}
```

### Truncation Strategy

```rust
impl SystemPromptBuilder {
    pub fn build_with_token_limit(self, max_tokens: usize) -> String {
        let mut prompt = self.clone().build();

        while estimate_tokens(&prompt) > max_tokens {
            // Try to reduce git info first
            if let Some(info) = &self.git_info {
                if info.diff.is_some() {
                    // Remove diff (largest component)
                    let mut reduced = self.clone();
                    reduced.git_info.as_mut().unwrap().diff = None;
                    prompt = reduced.build();
                    continue;
                }

                if !info.recent_commits.is_empty() {
                    // Reduce commit count
                    let mut reduced = self.clone();
                    reduced.git_info.as_mut().unwrap().recent_commits.truncate(5);
                    prompt = reduced.build();
                    continue;
                }
            }

            // Truncate instruction files
            if let Some(ctx) = &self.project_context {
                if !ctx.instruction_files.is_empty() {
                    // Remove furthest instruction file
                    // ... truncation logic
                }
            }

            // If we can't reduce further, return what we have
            break;
        }

        prompt
    }
}
```

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_basic_system_prompt() {
        let builder = SystemPromptBuilder::new(PathBuf::from("/test"));
        let prompt = builder.build();

        assert!(prompt.contains("<tools>"));
        assert!(prompt.contains("<permissions>"));
        assert!(prompt.contains("READ_ONLY") || prompt.contains("WORKSPACE_WRITE"));
    }

    #[test]
    fn includes_project_context() {
        let context = ProjectContext {
            root: PathBuf::from("/test"),
            name: Some("test-project".to_string()),
            project_type: Some(ProjectType::Rust),
            instruction_files: vec![],
            ancestor_chain: vec![PathBuf::from("/test")],
        };

        let builder = SystemPromptBuilder::new(PathBuf::from("/test"))
            .project_context(context);
        let prompt = builder.build();

        assert!(prompt.contains("test-project"));
    }

    #[test]
    fn includes_git_info() {
        let info = GitInfo {
            branch: String::from("main"),
            status: String::from(" M src/main.rs\n"),
            recent_commits: vec![],
            diff: None,
        };

        let builder = SystemPromptBuilder::new(PathBuf::from("/test"))
            .git_info(info);
        let prompt = builder.build();

        assert!(prompt.contains("main"));
        assert!(prompt.contains("src/main.rs"));
    }

    #[test]
    fn respects_token_limits() {
        // Create builder with lots of content
        let mut instructions = Vec::new();
        for i in 0..100 {
            instructions.push(format!("Instruction {}", "x".repeat(1000), i));
        }

        let builder = SystemPromptBuilder::new(PathBuf::from("/test"));
        let prompt = builder.build_with_token_limit(8000);

        assert!(estimate_tokens(&prompt) <= 8000);
    }
}
```

---

## Related Files

| File | Purpose |
|------|---------|
| `rust/crates/runtime/src/prompt.rs` | SystemPromptBuilder implementation |
| `rust/crates/runtime/src/config.rs` | Configuration for prompt builder |
| `rust/crates/runtime/src/conversation.rs` | Uses prompt builder in runtime |
| `rust/crates/rusty-claude-cli/src/main.rs` | System prompt dump command |

---

## Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `CLAWD_NO_GIT` | Disable git integration | false |
| `CLAWD_MAX_INSTRUCTION_BYTES` | Max instruction file bytes | 12000 |
| `CLAWD_SYSTEM_PROMPT_MAX_TOKENS` | Max system prompt tokens | 16000 |
