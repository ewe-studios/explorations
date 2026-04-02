# free-code Usage Examples

Practical examples for using free-code in various scenarios.

---

## Table of Contents

1. [Basic Usage](#basic-usage)
2. [Model Provider Examples](#model-provider-examples)
3. [Common Tasks](#common-tasks)
4. [Advanced Features](#advanced-features)
5. [Team Collaboration](#team-collaboration)

---

## Basic Usage

### Starting a Session

```bash
# Start with default model (Sonnet)
free-code

# Specify a model
free-code --model claude-opus-4-6

# Simple mode (limited tools)
free-code --bare

# Plan mode (read-only with planning)
free-code --permission-mode read-only
```

### One-Shot Queries

```bash
# Quick question
free-code -p "What files are in this directory?"

# With specific model
free-code --model claude-opus-4-6 -p "Review this PR diff"

# Pipe input
git diff | free-code -p "Explain these changes"
```

### Interactive Commands

```
# In a session:

# Get help
/help

# Change model
/model claude-opus-4-6

# View session cost
/cost

# Compact context
/compact

# Enter plan mode
/plan

# View memory
/memory

# Exit
/exit
```

---

## Model Provider Examples

### Anthropic (Direct API)

```bash
# Set API key
export ANTHROPIC_API_KEY="sk-ant-..."

# Start session
free-code

# Use Opus for complex tasks
/model claude-opus-4-6

# Use Haiku for quick tasks
/model claude-haiku-4-5
```

### OpenAI Codex

```bash
# Enable Codex provider
export CLAUDE_CODE_USE_OPENAI=1

# Login with OAuth
free-code /login

# Use Codex models
/model gpt-5.3-codex
```

### AWS Bedrock

```bash
# Enable Bedrock
export CLAUDE_CODE_USE_BEDROCK=1
export AWS_REGION="us-east-1"

# Credentials via ~/.aws/credentials or IAM role
free-code

# Models are auto-mapped to Bedrock ARN format
/model claude-opus-4-6  # Uses us.anthropic.claude-opus-4-6-v1
```

### Google Vertex AI

```bash
# Enable Vertex
export CLAUDE_CODE_USE_VERTEX=1

# Authenticate
gcloud auth application-default login

# Start session
free-code
```

### Anthropic Foundry

```bash
# Enable Foundry
export CLAUDE_CODE_USE_FOUNDRY=1
export ANTHROPIC_FOUNDRY_API_KEY="..."

# Use deployment ID as model
free-code --model "my-deployment-id"
```

---

## Common Tasks

### Code Review

```
/review

# Or manually
Please review the changes in this PR:
@src/feature.ts
@src/feature.test.ts

Look for:
1. Potential bugs
2. Type safety issues
3. Missing error handling
4. Performance concerns
```

### Debugging

```
I'm getting this error:

[Error output here]

Here's the relevant code:
@src/failing-function.ts

Help me identify the root cause and suggest a fix.
```

### Writing New Code

```
I need to create a new API endpoint for user registration.

Requirements:
- POST /api/users/register
- Accept email, password, name
- Validate input
- Hash password with bcrypt
- Create user in database
- Send welcome email

Create the implementation using Express and TypeScript.
```

### Refactoring

```
Refactor @src/legacy-module.ts to:
1. Use modern TypeScript patterns
2. Add proper error handling
3. Split into smaller functions
4. Add type annotations
5. Write unit tests

Keep the same functionality but improve code quality.
```

### Documentation

```
Generate documentation for the API in @src/api/:

1. Create a README.md with:
   - Overview
   - Installation instructions
   - API endpoint reference
   - Example requests/responses

2. Add JSDoc comments to all functions
```

### Testing

```
Write comprehensive tests for @src/utils/:

1. Unit tests for each function
2. Edge cases
3. Error scenarios
4. Integration tests

Use Jest and follow our existing test patterns.
```

---

## Advanced Features

### UltraPlan Mode

```bash
# Enable with build:dev:full
bun run build:dev:full

# In session
/ultraplan

# Or use ultrathink for deeper reasoning
ultrathink: Let me analyze this complex architecture decision...
```

### Token Budget

```json
// In ~/.claude/config.json
{
  "tokenBudget": "100K",
  "tokenBudgetWarning": 0.8
}
```

```
# View token usage
/cost

# Check budget status
/usage
```

### Agent Spawning

```
@agent/senior-dev Please review this architecture:

@docs/architecture.md

Provide feedback on:
1. Scalability
2. Security
3. Maintainability
4. Cost efficiency
```

### Memory Management

```
# View memories
/memory

# Add memory (in conversation)
<user>Remember that we use PostgreSQL with connection pooling</user>

# Team memory
@.claude/TEAM.md What are our project conventions?
```

### MCP Integration

```bash
# Add MCP server to config
cat >> ~/.claude/config.json << EOF
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "~/projects"]
    },
    "postgres": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-postgres", "postgresql://localhost/mydb"]
    }
  }
}
EOF

# List MCP resources
/mcp list

# Use MCP tools
@filesystem Read the file at ~/projects/config.json
```

### Bridge Mode (Remote Control)

```bash
# Start bridge on local machine
free-code remote-control

# Connect from mobile/web
# Scan QR code shown in terminal
```

---

## Team Collaboration

### Team Memory Setup

Create `.claude/TEAM.md`:

```markdown
# Team Memory: Project X

## Tech Stack
- Frontend: React 18, TypeScript, Tailwind
- Backend: Node.js, Express, PostgreSQL
- Testing: Jest, Playwright

## Key Directories
- `src/api/` - REST API endpoints
- `src/components/` - React components
- `src/db/` - Database schemas and migrations
- `src/utils/` - Shared utilities

## Conventions
- ESLint + Prettier for formatting
- Conventional commits
- PR reviews required
- 80% test coverage minimum

## Local Setup
```bash
bun install
bun run dev
```

## Deployment
- Staging: `bun run deploy:staging`
- Production: `bun run deploy:prod` (requires approval)
```

### Shared Configuration

Team-wide config template:

```json
{
  "mcpServers": {
    "shared-db": {
      "command": "npx",
      "args": ["-y", "@company/mcp-db-server", "--host", "db.internal.company.com"]
    },
    "jira": {
      "command": "npx",
      "args": ["-y", "@company/mcp-jira"]
    }
  },
  "hooks": {
    "beforeToolUse": {
      "Bash": "/shared/hooks/security-check.sh"
    },
    "afterQuery": "/shared/hooks/log-query.sh"
  }
}
```

### Deploy to Team

Using Ansible:

```yaml
- name: Deploy free-code to developers
  hosts: developers
  tasks:
    - name: Install free-code
      shell: |
        curl -fsSL https://raw.githubusercontent.com/.../install.sh | bash

    - name: Deploy team config
      copy:
        src: team-config.json
        dest: ~/.claude/config.json
        mode: '0644'

    - name: Deploy team memory
      copy:
        src: TEAM.md
        dest: /projects/myproject/.claude/TEAM.md
```

---

## Scripting Examples

### Automated Code Review

```bash
#!/bin/bash
# review-pr.sh

PR_NUMBER=$1
PR_DIFF=$(gh pr view $PR_NUMBER --diff)

echo "$PR_DIFF" | free-code -p "
Review this PR for:
1. Bugs and logic errors
2. Security vulnerabilities
3. Performance issues
4. Code style violations

Be concise and actionable.
"
```

### Documentation Generator

```bash
#!/bin/bash
# generate-docs.sh

find src -name "*.ts" -type f | while read file; do
  echo "Processing $file..."
  free-code -p "
Generate JSDoc documentation for this file:

$(cat $file)

Output only the JSDoc comments, no explanation.
" >> "${file}.docs"
done
```

### Test Generator

```bash
#!/bin/bash
# generate-tests.sh

SOURCE_FILE=$1
TEST_FILE="${SOURCE_FILE%.ts}.test.ts"

free-code -p "
Write comprehensive Jest tests for this code:

$(cat $SOURCE_FILE)

Include:
- Happy path tests
- Edge cases
- Error scenarios
- Mock external dependencies

Output only the test code.
" > "$TEST_FILE"
```

---

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Ctrl+C` | Cancel current operation |
| `Ctrl+D` | Exit session |
| `Ctrl+L` | Clear screen |
| `Ctrl+R` | Search history |
| `Ctrl+K` | Quick command |
| `Esc` | Close dialog/focus |
| `:` | Enter command mode |
| `/` | Open slash command typeahead |
| `@` | File mention |
| `Tab` | Autocomplete |

---

## Environment Variable Reference

```bash
# Provider selection
export CLAUDE_CODE_USE_OPENAI=1
export CLAUDE_CODE_USE_BEDROCK=1
export CLAUDE_CODE_USE_VERTEX=1
export CLAUDE_CODE_USE_FOUNDRY=1

# API keys
export ANTHROPIC_API_KEY="sk-ant-..."
export AWS_ACCESS_KEY_ID="..."
export AWS_SECRET_ACCESS_KEY="..."
export GOOGLE_APPLICATION_CREDENTIALS="/path/to/creds.json"

# Model selection
export ANTHROPIC_MODEL="claude-opus-4-6"

# Runtime options
export CLAUDE_CODE_SIMPLE=1          # Simple mode
export CLAUDE_CODE_DISABLE_AUTO_COMPACT=1  # Disable auto-compact
```

---

## Troubleshooting Examples

### Check Model Connection

```bash
# List available models
free-code /model

# Test API connection
curl -H "X-API-Key: $ANTHROPIC_API_KEY" \
  https://api.anthropic.com/v1/models
```

### Clear Session State

```bash
# Clear all sessions
rm -rf ~/.claude/sessions/*

# Or specific session
rm -rf ~/.claude/sessions/<session-id>/
```

### Debug Mode

```bash
# Enable debug logging
export DEBUG=1
free-code

# Or use diagnostic command
free-code /doctor
```

---

## References

- [00-zero-to-free-code-engineer.md](./00-zero-to-free-code-engineer.md) — Getting started
- [production-grade.md](./production-grade.md) — Production deployment
- [../FEATURES.md](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code/FEATURES.md) — Feature flags
