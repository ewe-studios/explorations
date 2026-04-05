# Hooks System Integration

## Executive Summary

Claw-Code's hooks system provides a powerful mechanism for intercepting, auditing, modifying, and controlling tool execution. This document covers pre-tool and post-tool hooks, custom hook implementation, hook execution flow, security considerations, and integration patterns.

**Source Reference:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claw-code/`

---

## Table of Contents

1. [Overview](#overview)
2. [Hook Types and Events](#hook-types-and-events)
3. [Hook Configuration](#hook-configuration)
4. [Hook Execution Flow](#hook-execution-flow)
5. [Hook Command Implementation](#hook-command-implementation)
6. [Hook Run Result Handling](#hook-run-result-handling)
7. [Integration with Tool Pipeline](#integration-with-tool-pipeline)
8. [Security Considerations](#security-considerations)
9. [Use Cases](#use-cases)
10. [Testing Hooks](#testing-hooks)

---

## 1. Overview

### 1.1 What are Hooks?

Hooks are external commands that claw-code executes at specific points in the tool execution pipeline. They enable:
- **Auditing**: Log all tool uses for compliance
- **Validation**: Check tool inputs before execution
- **Modification**: Transform tool outputs
- **Access Control**: Deny specific tool uses based on custom logic
- **Notifications**: Alert on specific tool patterns

### 1.2 Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Tool Execution Request                       │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│                 PreToolUse Hook Phase                           │
│  - Execute all pre_tool_use commands                            │
│  - Each hook receives: tool_name, tool_input, event_type        │
│  - Hooks can: Allow, Warn, or Deny                              │
│  - If any hook denies: Tool execution SKIPPED                   │
└─────────────────────────────────────────────────────────────────┘
                                    │
                          ┌─────────┴─────────┐
                          │                   │
                       Allowed              Denied
                          │                   │
                          ▼                   ▼
              ┌───────────────────┐ ┌───────────────────┐
              │   Tool Execution  │ │  Return Denial    │
              │                   │ │  to Model         │
              └───────────────────┘ └───────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                PostToolUse Hook Phase                           │
│  - Execute all post_tool_use commands                           │
│  - Each hook receives: tool_name, input, output, is_error       │
│  - Hooks can: Allow, Warn, or mark as Error                     │
│  - Hook feedback appended to tool output                        │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Return to Model                              │
│  Tool result + any hook feedback                                │
└─────────────────────────────────────────────────────────────────┘
```

### 1.3 Key Characteristics

| Characteristic | Description |
|----------------|-------------|
| **External Commands** | Hooks are shell commands, not in-process callbacks |
| **JSON Payload** | Hook data passed via stdin as JSON |
| **Environment Variables** | Additional context via env vars |
| **Exit Code Based** | Hook decision communicated via exit code |
| **Stdout as Message** | Hook messages returned via stdout |

---

## 2. Hook Types and Events

### 2.1 HookEvent Enum

```rust
// runtime/src/hooks.rs:8-21
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookEvent {
    PreToolUse,
    PostToolUse,
}

impl HookEvent {
    fn as_str(self) -> &'static str {
        match self {
            Self::PreToolUse => "PreToolUse",
            Self::PostToolUse => "PostToolUse",
        }
    }
}
```

### 2.2 PreToolUse Hook

**When:** Before tool execution, after permission check

**Purpose:**
- Validate tool inputs
- Check against external policies
- Log tool use for audit
- Deny based on custom logic

**Available Data:**
- `tool_name`: Name of the tool
- `tool_input`: JSON input to the tool
- `hook_event_name`: "PreToolUse"

**Decisions:**
- Exit 0: Allow (stdout as message)
- Exit 2: Deny (stdout as denial reason)
- Exit other: Warn (stdout as warning message)

### 2.3 PostToolUse Hook

**When:** After tool execution completes

**Purpose:**
- Log tool results
- Validate outputs
- Redact sensitive data
- Trigger downstream actions

**Available Data:**
- `tool_name`: Name of the tool
- `tool_input`: JSON input to the tool
- `tool_output`: Tool's output
- `tool_result_is_error`: Whether tool errored
- `hook_event_name`: "PostToolUse"

**Decisions:**
- Exit 0: Accept output (stdout appended)
- Exit 2: Mark as error (stdout as error message)
- Exit other: Warn (stdout as warning)

---

## 3. Hook Configuration

### 3.1 Configuration Structure

```rust
// runtime/src/config.rs:48-52
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimeHookConfig {
    pre_tool_use: Vec<String>,
    post_tool_use: Vec<String>,
}
```

### 3.2 JSON Configuration

```json
{
  "hooks": {
    "pre_tool_use": [
      "/usr/local/bin/audit-tool-use",
      "/usr/local/bin/check-policy"
    ],
    "post_tool_use": [
      "/usr/local/bin/log-tool-result",
      "/usr/local/bin/redact-secrets"
    ]
  }
}
```

### 3.3 Configuration Loading

```rust
// runtime/src/config.rs (partial)
impl RuntimeFeatureConfig {
    pub fn hooks(&self) -> &RuntimeHookConfig {
        &self.hooks
    }
}

impl RuntimeHookConfig {
    pub fn new(pre_tool_use: Vec<String>, post_tool_use: Vec<String>) -> Self {
        Self {
            pre_tool_use,
            post_tool_use,
        }
    }

    pub fn pre_tool_use(&self) -> &[String] {
        &self.pre_tool_use
    }

    pub fn post_tool_use(&self) -> &[String] {
        &self.post_tool_use
    }
}
```

### 3.4 RuntimeConfig Integration

```rust
// runtime/src/config.rs:38-46
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimeFeatureConfig {
    hooks: RuntimeHookConfig,
    mcp: McpConfigCollection,
    oauth: Option<OAuthConfig>,
    model: Option<String>,
    permission_mode: Option<ResolvedPermissionMode>,
    sandbox: SandboxConfig,
}
```

---

## 4. Hook Execution Flow

### 4.1 HookRunner Structure

```rust
// runtime/src/hooks.rs:49-58
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HookRunner {
    config: RuntimeHookConfig,
}

impl HookRunner {
    pub fn new(config: RuntimeHookConfig) -> Self {
        Self { config }
    }

    pub fn from_feature_config(feature_config: &RuntimeFeatureConfig) -> Self {
        Self::new(feature_config.hooks().clone())
    }
}
```

### 4.2 Pre-Tool Hook Execution

```rust
// runtime/src/hooks.rs:65-75
pub fn run_pre_tool_use(&self, tool_name: &str, tool_input: &str) -> HookRunResult {
    self.run_commands(
        HookEvent::PreToolUse,
        self.config.pre_tool_use(),
        tool_name,
        tool_input,
        None,  // No output yet
        false, // Not an error
    )
}
```

### 4.3 Post-Tool Hook Execution

```rust
// runtime/src/hooks.rs:77-93
pub fn run_post_tool_use(
    &self,
    tool_name: &str,
    tool_input: &str,
    tool_output: &str,
    is_error: bool,
) -> HookRunResult {
    self.run_commands(
        HookEvent::PostToolUse,
        self.config.post_tool_use(),
        tool_name,
        tool_input,
        Some(tool_output),
        is_error,
    )
}
```

### 4.4 Command Execution Loop

```rust
// runtime/src/hooks.rs:95-150
fn run_commands(
    &self,
    event: HookEvent,
    commands: &[String],
    tool_name: &str,
    tool_input: &str,
    tool_output: Option<&str>,
    is_error: bool,
) -> HookRunResult {
    // Early exit if no hooks configured
    if commands.is_empty() {
        return HookRunResult::allow(Vec::new());
    }

    // Build JSON payload
    let payload = json!({
        "hook_event_name": event.as_str(),
        "tool_name": tool_name,
        "tool_input": parse_tool_input(tool_input),
        "tool_input_json": tool_input,
        "tool_output": tool_output,
        "tool_result_is_error": is_error,
    }).to_string();

    let mut messages = Vec::new();

    // Execute each hook command sequentially
    for command in commands {
        match self.run_command(command, event, tool_name, tool_input, tool_output, is_error, &payload) {
            HookCommandOutcome::Allow { message } => {
                if let Some(message) = message {
                    messages.push(message);
                }
            }
            HookCommandOutcome::Deny { message } => {
                let message = message.unwrap_or_else(|| {
                    format!("{} hook denied tool `{tool_name}`", event.as_str())
                });
                messages.push(message);
                // Early return on denial
                return HookRunResult {
                    denied: true,
                    messages,
                };
            }
            HookCommandOutcome::Warn { message } => {
                messages.push(message);
                // Continue to next hook
            }
        }
    }

    HookRunResult::allow(messages)
}
```

---

## 5. Hook Command Implementation

### 5.1 Command Execution

```rust
// runtime/src/hooks.rs:152-205
fn run_command(
    &self,
    command: &str,
    event: HookEvent,
    tool_name: &str,
    tool_input: &str,
    tool_output: Option<&str>,
    is_error: bool,
    payload: &str,
) -> HookCommandOutcome {
    // Build shell command
    let mut child = shell_command(command);
    child.stdin(std::process::Stdio::piped());
    child.stdout(std::process::Stdio::piped());
    child.stderr(std::process::Stdio::piped());

    // Set environment variables
    child.env("HOOK_EVENT", event.as_str());
    child.env("HOOK_TOOL_NAME", tool_name);
    child.env("HOOK_TOOL_INPUT", tool_input);
    child.env("HOOK_TOOL_IS_ERROR", if is_error { "1" } else { "0" });
    if let Some(tool_output) = tool_output {
        child.env("HOOK_TOOL_OUTPUT", tool_output);
    }

    // Execute with JSON payload on stdin
    match child.output_with_stdin(payload.as_bytes()) {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let message = (!stdout.is_empty()).then_some(stdout);

            // Interpret exit code
            match output.status.code() {
                Some(0) => HookCommandOutcome::Allow { message },
                Some(2) => HookCommandOutcome::Deny { message },
                Some(code) => HookCommandOutcome::Warn {
                    message: format_hook_warning(command, code, message.as_deref(), stderr.as_str()),
                },
                None => HookCommandOutcome::Warn {
                    message: format!(
                        "{} hook `{command}` terminated by signal while handling `{tool_name}`",
                        event.as_str()
                    ),
                },
            }
        }
        Err(error) => HookCommandOutcome::Warn {
            message: format!(
                "{} hook `{command}` failed to start for `{tool_name}`: {error}",
                event.as_str()
            ),
        },
    }
}
```

### 5.2 Environment Variables

| Variable | Description |
|----------|-------------|
| `HOOK_EVENT` | "PreToolUse" or "PostToolUse" |
| `HOOK_TOOL_NAME` | Name of the tool (e.g., "bash", "read_file") |
| `HOOK_TOOL_INPUT` | Raw JSON input string |
| `HOOK_TOOL_IS_ERROR` | "1" if tool errored, "0" otherwise |
| `HOOK_TOOL_OUTPUT` | Tool output (PostToolUse only) |

### 5.3 JSON Payload Structure

```json
{
  "hook_event_name": "PreToolUse",
  "tool_name": "bash",
  "tool_input": {
    "command": "ls -la"
  },
  "tool_input_json": "{\"command\":\"ls -la\"}",
  "tool_output": null,
  "tool_result_is_error": false
}
```

### 5.4 Exit Code Semantics

| Exit Code | Meaning | Effect |
|-----------|---------|--------|
| 0 | Allow | Tool execution continues |
| 2 | Deny | Tool execution blocked (PreToolUse) or marked as error (PostToolUse) |
| Other | Warn | Warning logged, tool execution continues |
| Signal | Terminated | Warning logged, tool execution continues |

### 5.5 Shell Command Wrapper

```rust
// runtime/src/hooks.rs:231-247
fn shell_command(command: &str) -> CommandWithStdin {
    #[cfg(windows)]
    let command_builder = {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C").arg(command);
        CommandWithStdin::new(cmd)
    };

    #[cfg(not(windows))]
    let command_builder = {
        let mut cmd = Command::new("sh");
        cmd.arg("-lc").arg(command);
        CommandWithStdin::new(cmd)
    };

    command_builder
}
```

---

## 6. Hook Run Result Handling

### 6.1 HookRunResult Structure

```rust
// runtime/src/hooks.rs:23-47
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookRunResult {
    denied: bool,
    messages: Vec<String>,
}

impl HookRunResult {
    pub fn allow(messages: Vec<String>) -> Self {
        Self {
            denied: false,
            messages,
        }
    }

    pub fn is_denied(&self) -> bool {
        self.denied
    }

    pub fn messages(&self) -> &[String] {
        &self.messages
    }
}
```

### 6.2 HookCommandOutcome Enum

```rust
// runtime/src/hooks.rs:208-212
enum HookCommandOutcome {
    Allow { message: Option<String> },
    Deny { message: Option<String> },
    Warn { message: String },
}
```

### 6.3 Result Integration in Tool Pipeline

```rust
// runtime/src/conversation.rs:226-268
let result_message = match permission_outcome {
    PermissionOutcome::Allow => {
        // Run pre-tool hooks
        let pre_hook_result = self.hook_runner.run_pre_tool_use(&tool_name, &input);

        if pre_hook_result.is_denied() {
            // Hook denied - return denial message
            let deny_message = format!("PreToolUse hook denied tool `{tool_name}`");
            ConversationMessage::tool_result(
                tool_use_id,
                tool_name,
                format_hook_message(&pre_hook_result, &deny_message),
                true,  // Mark as error
            )
        } else {
            // Execute tool
            let (mut output, mut is_error) = match self.tool_executor.execute(&tool_name, &input) {
                Ok(output) => (output, false),
                Err(error) => (error.to_string(), true),
            };

            // Merge pre-hook feedback
            output = merge_hook_feedback(pre_hook_result.messages(), output, false);

            // Run post-tool hooks
            let post_hook_result = self.hook_runner.run_post_tool_use(
                &tool_name, &input, &output, is_error
            );

            if post_hook_result.is_denied() {
                is_error = true;
            }

            // Merge post-hook feedback
            output = merge_hook_feedback(
                post_hook_result.messages(),
                output,
                post_hook_result.is_denied(),
            );

            ConversationMessage::tool_result(tool_use_id, tool_name, output, is_error)
        }
    }
    PermissionOutcome::Deny { reason } => {
        ConversationMessage::tool_result(tool_use_id, tool_name, reason, true)
    }
};
```

### 6.4 Feedback Merging

```rust
// runtime/src/conversation.rs:408-424
fn merge_hook_feedback(messages: &[String], output: String, denied: bool) -> String {
    if messages.is_empty() {
        return output;
    }

    let mut sections = Vec::new();

    // Include original output if not empty
    if !output.trim().is_empty() {
        sections.push(output);
    }

    // Append hook feedback
    let label = if denied {
        "Hook feedback (denied)"
    } else {
        "Hook feedback"
    };
    sections.push(format!("{label}:\n{}", messages.join("\n")));

    sections.join("\n\n")
}
```

---

## 7. Integration with Tool Pipeline

### 7.1 Complete Pipeline with Hooks

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. Model generates ToolUse                                      │
│    { "name": "bash", "input": {"command": "ls"} }               │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│ 2. Permission Check                                             │
│    PermissionPolicy.authorize("bash", input, prompter)          │
│    → PermissionOutcome::Allow or Deny                           │
└─────────────────────────────────────────────────────────────────┘
                                    │
                          ┌─────────┴─────────┐
                          │                   │
                       Allow               Deny
                          │                   │
                          ▼                   ▼
              ┌───────────────────┐ ┌───────────────────┐
              │ 3. PreToolUse     │ │ Return denial     │
              │    Hooks          │ │ to model          │
              │                   │ │                   │
              │ for hook in hooks:│ │                   │
              │   result = run    │ │                   │
              │   if denied:      │ │                   │
              │     return denial │ │                   │
              └───────────────────┘ └───────────────────┘
                          │
                          ▼
              ┌───────────────────┐
              │ 4. Tool Execution │
              │    execute_tool() │
              │    → output       │
              └───────────────────┘
                          │
                          ▼
              ┌───────────────────┐
              │ 5. PostToolUse    │
              │    Hooks          │
              │                   │
              │ for hook in hooks:│
              │   result = run    │
              │   append feedback │
              └───────────────────┘
                          │
                          ▼
              ┌───────────────────┐
              │ 6. Return Result  │
              │    to Model       │
              └───────────────────┘
```

### 7.2 HookRunner in ConversationRuntime

```rust
// runtime/src/conversation.rs:100-110
pub struct ConversationRuntime<C, T> {
    session: Session,
    api_client: C,
    tool_executor: T,
    permission_policy: PermissionPolicy,
    system_prompt: Vec<String>,
    max_iterations: usize,
    usage_tracker: UsageTracker,
    hook_runner: HookRunner,  // Hooks integration
    auto_compaction_input_tokens_threshold: u32,
}
```

### 7.3 HookRunner Initialization

```rust
// runtime/src/conversation.rs:136-156
pub fn new_with_features(
    session: Session,
    api_client: C,
    tool_executor: T,
    permission_policy: PermissionPolicy,
    system_prompt: Vec<String>,
    feature_config: RuntimeFeatureConfig,
) -> Self {
    let usage_tracker = UsageTracker::from_session(&session);
    Self {
        session,
        api_client,
        tool_executor,
        permission_policy,
        system_prompt,
        max_iterations: usize::MAX,
        usage_tracker,
        hook_runner: HookRunner::from_feature_config(&feature_config),
        auto_compaction_input_tokens_threshold: auto_compaction_threshold_from_env(),
    }
}
```

---

## 8. Security Considerations

### 8.1 Hook Security Model

| Concern | Mitigation |
|---------|------------|
| Hook command injection | Commands configured in trusted config file |
| Hook privilege escalation | Hooks run with same privileges as claw-code |
| Information disclosure | Hooks receive full tool input/output |
| DoS via slow hooks | No timeout on hook execution |
| Infinite loops | No recursion protection |

### 8.2 Trust Boundaries

```
┌─────────────────────────────────────────────────────────────────┐
│                    TRUSTED CONFIG                               │
│  settings.json with hook commands                               │
│  → Only administrators should modify                            │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│                    HOOK EXECUTION                               │
│  Hooks run with same UID as claw-code                           │
│  → Can access everything claw-code can access                   │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│                    DATA EXPOSURE                                │
│  Hooks receive: tool_name, input, output                        │
│  → May include sensitive data (paths, commands, results)        │
└─────────────────────────────────────────────────────────────────┘
```

### 8.3 Security Best Practices

1. **Restrict Config Access**: Only trusted admins should modify hook configuration
2. **Audit Hook Scripts**: Review hook scripts for security issues
3. **Limit Data Exposure**: Hooks should only log what's necessary
4. **Set Timeouts**: Add timeout wrappers around slow hooks
5. **Validate Outputs**: Don't blindly trust hook stdout

### 8.4 Recommended Hook Security

```bash
#!/bin/bash
# Example: Secure audit hook with timeout

# Set timeout
TIMEOUT=5

# Read payload
read -r PAYLOAD

# Log with timestamp (not sensitive data)
echo "$(date -Iseconds) - Tool: $(echo $PAYLOAD | jq -r '.tool_name')" >> /var/log/clawd-audit.log

# Allow
exit 0
```

---

## 9. Use Cases

### 9.1 Audit Logging

**Hook Script:**
```bash
#!/bin/bash
# /usr/local/bin/audit-tool-use

read -r PAYLOAD

TOOL_NAME=$(echo "$PAYLOAD" | jq -r '.tool_name')
EVENT=$(echo "$PAYLOAD" | jq -r '.hook_event_name')
TIMESTAMP=$(date -Iseconds)

echo "$TIMESTAMP - $EVENT - $TOOL_NAME" >> /var/log/clawd-audit.log

exit 0
```

**Configuration:**
```json
{
  "hooks": {
    "pre_tool_use": ["/usr/local/bin/audit-tool-use"],
    "post_tool_use": ["/usr/local/bin/audit-tool-use"]
  }
}
```

### 9.2 Policy Enforcement

**Hook Script:**
```bash
#!/bin/bash
# /usr/local/bin/check-destructive-commands

read -r PAYLOAD

TOOL_NAME=$(echo "$PAYLOAD" | jq -r '.tool_name')

if [ "$TOOL_NAME" = "bash" ]; then
    COMMAND=$(echo "$PAYLOAD" | jq -r '.tool_input.command')

    # Block destructive commands
    if echo "$COMMAND" | grep -qE "rm -rf /|dd if=.*of=/dev|mkfs"; then
        echo "Destructive command blocked: $COMMAND"
        exit 2
    fi
fi

exit 0
```

### 9.3 Secret Redaction

**Hook Script:**
```bash
#!/bin/bash
# /usr/local/bin/redact-secrets

read -r PAYLOAD

OUTPUT=$(echo "$PAYLOAD" | jq -r '.tool_output // empty')

if [ -n "$OUTPUT" ]; then
    # Redact common secret patterns
    REDACTED=$(echo "$OUTPUT" | sed -E 's/(AKIA|sk-|ghp_)[A-Za-z0-9]+/\1***REDACTED***/g')

    if [ "$OUTPUT" != "$REDACTED" ]; then
        echo "Warning: Secrets redacted from output"
        echo "$REDACTED"
    fi
fi

exit 0
```

### 9.4 Slack Notifications

**Hook Script:**
```bash
#!/bin/bash
# /usr/local/bin/notify-dangerous-tools

read -r PAYLOAD

TOOL_NAME=$(echo "$PAYLOAD" | jq -r '.tool_name')
EVENT=$(echo "$PAYLOAD" | jq -r '.hook_event_name')

# Only notify on dangerous tool use
if [ "$TOOL_NAME" = "bash" ] && [ "$EVENT" = "PreToolUse" ]; then
    COMMAND=$(echo "$PAYLOAD" | jq -r '.tool_input.command')

    curl -X POST "$SLACK_WEBHOOK_URL" \
        -H 'Content-Type: application/json' \
        -d "{
            \"text\": \"Dangerous tool used: $TOOL_NAME\",
            \"blocks\": [
                {
                    \"type\": \"section\",
                    \"text\": {
                        \"type\": \"mrkdwn\",
                        \"text\": \"*Tool:* $TOOL_NAME\\n*Command:* \\\`$COMMAND\\\`\"
                    }
                }
            ]
        }"
fi

exit 0
```

### 9.5 Compliance Checking

**Hook Script:**
```bash
#!/bin/bash
# /usr/local/bin/compliance-check

read -r PAYLOAD

TOOL_NAME=$(echo "$PAYLOAD" | jq -r '.tool_name')
INPUT=$(echo "$PAYLOAD" | jq -r '.tool_input_json')

# Log to compliance system
curl -X POST "$COMPLIANCE_API/log" \
    -H 'Content-Type: application/json' \
    -H "Authorization: Bearer $COMPLIANCE_TOKEN" \
    -d "{
        \"tool\": \"$TOOL_NAME\",
        \"input\": $INPUT,
        \"timestamp\": \"$(date -Iseconds)\"
    }"

exit 0
```

---

## 10. Testing Hooks

### 10.1 Unit Tests

```rust
// runtime/src/hooks.rs:292-349
#[cfg(test)]
mod tests {
    use super::{HookRunResult, HookRunner};
    use crate::config::{RuntimeFeatureConfig, RuntimeHookConfig};

    #[test]
    fn allows_exit_code_zero_and_captures_stdout() {
        let runner = HookRunner::new(RuntimeHookConfig::new(
            vec![shell_snippet("printf 'pre ok'")],
            Vec::new(),
        ));

        let result = runner.run_pre_tool_use("Read", r#"{"path":"README.md"}"#);

        assert_eq!(result, HookRunResult::allow(vec!["pre ok".to_string()]));
    }

    #[test]
    fn denies_exit_code_two() {
        let runner = HookRunner::new(RuntimeHookConfig::new(
            vec![shell_snippet("printf 'blocked by hook'; exit 2")],
            Vec::new(),
        ));

        let result = runner.run_pre_tool_use("Bash", r#"{"command":"pwd"}"#);

        assert!(result.is_denied());
        assert_eq!(result.messages(), &["blocked by hook".to_string()]);
    }

    #[test]
    fn warns_for_other_non_zero_statuses() {
        let runner = HookRunner::from_feature_config(&RuntimeFeatureConfig::default().with_hooks(
            RuntimeHookConfig::new(
                vec![shell_snippet("printf 'warning hook'; exit 1")],
                Vec::new(),
            ),
        ));

        let result = runner.run_pre_tool_use("Edit", r#"{"file":"src/lib.rs"}"#);

        assert!(!result.is_denied());
        assert!(result
            .messages()
            .iter()
            .any(|message| message.contains("allowing tool execution to continue")));
    }

    #[cfg(windows)]
    fn shell_snippet(script: &str) -> String {
        script.replace('\'', "\"")
    }

    #[cfg(not(windows))]
    fn shell_snippet(script: &str) -> String {
        script.to_string()
    }
}
```

### 10.2 Integration Testing

```bash
#!/bin/bash
# Test hook script
# test-hook.sh

cat << 'EOF' | /path/to/hook-command
{
  "hook_event_name": "PreToolUse",
  "tool_name": "bash",
  "tool_input": {"command": "echo hello"},
  "tool_input_json": "{\"command\":\"echo hello\"}",
  "tool_output": null,
  "tool_result_is_error": false
}
EOF

echo "Exit code: $?"
```

### 10.3 Hook Development Tips

1. **Test with jq**: Validate JSON parsing
   ```bash
   echo '{"tool_name": "test"}' | jq -r '.tool_name'
   ```

2. **Handle missing fields**: Not all fields present in all events
   ```bash
   OUTPUT=$(echo "$PAYLOAD" | jq -r '.tool_output // empty')
   ```

3. **Exit codes matter**: 0=allow, 2=deny, other=warn

4. **Stdout is message**: Only stdout is captured as feedback

---

## 11. Summary

### 11.1 Hook System Capabilities

| Capability | Description |
|------------|-------------|
| Pre-execution control | Block tool uses before execution |
| Post-execution processing | Modify outputs after execution |
| Audit logging | Log all tool uses |
| Policy enforcement | Custom access control rules |
| Output transformation | Redact, filter, or enhance outputs |
| Notifications | Alert on specific tool patterns |

### 11.2 Key Files

| File | Purpose |
|------|---------|
| `runtime/src/hooks.rs` | Hook execution logic |
| `runtime/src/config.rs` | Hook configuration |
| `runtime/src/conversation.rs` | Hook integration in tool pipeline |

### 11.3 Best Practices

1. **Keep hooks fast**: Slow hooks delay tool execution
2. **Handle errors gracefully**: Exit 0 on errors to avoid blocking
3. **Log appropriately**: Don't log sensitive data unnecessarily
4. **Test thoroughly**: Test all exit code paths
5. **Document hooks**: Maintain documentation of hook behavior

---

*Document generated from source analysis of claw-code repository.*
*Source: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claw-code/*
