# Bash-Tool - Deep Dive Exploration

## Overview

**Bash-Tool** provides a sandboxed bash execution environment for AI agents, with tools for file operations and command execution. It supports both in-memory filesystems (just-bash) and full VM isolation (@vercel/sandbox).

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.VarcelLabs/bash-tool`

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     createBashTool()                         │
│                                                              │
│  ┌─────────────────┐    ┌──────────────────┐                │
│  │  Sandbox        │    │  File Loader     │                │
│  │  Abstraction    │ ←─→│  (streamFiles)   │                │
│  │                 │    │                  │                │
│  │  - just-bash    │    │  - Upload dir    │                │
│  │  - @vercel/     │    │  - Inline files  │                │
│  │    sandbox      │    │  - Glob patterns │                │
│  └────────┬────────┘    └──────────────────┘                │
│           │                                                  │
│           ↓                                                  │
│  ┌─────────────────────────────────────────┐                │
│  │           Tools                          │                │
│  │  - bash (execute commands)               │                │
│  │  - readFile (read files)                 │                │
│  │  - writeFile (write files)               │                │
│  └─────────────────────────────────────────┘                │
└─────────────────────────────────────────────────────────────┘
```

---

## Tech Stack

| Component | Technology |
|-----------|------------|
| Runtime | Node.js / TypeScript |
| Sandbox (default) | just-bash (in-memory filesystem) |
| Sandbox (full VM) | @vercel/sandbox |
| AI Integration | AI SDK tools |
| File Streaming | Overlay filesystem for large uploads |

---

## Key Implementation Details

### 1. Main Entry Point (`src/tool.ts`)

```typescript
export async function createBashTool(
  options: CreateBashToolOptions = {},
): Promise<BashToolkit> {
  // 1. Determine sandbox destination
  const defaultDestination =
    options.sandbox && isVercelSandbox(options.sandbox)
      ? "/vercel/sandbox/workspace"
      : "/workspace";
  const destination = options.destination ?? defaultDestination;

  // 2. Create or wrap sandbox
  let sandbox: Sandbox;
  let usingJustBash = false;

  if (options.sandbox) {
    // External sandbox provided
    if (isVercelSandbox(options.sandbox)) {
      sandbox = wrapVercelSandbox(options.sandbox);
    } else if (isJustBash(options.sandbox)) {
      sandbox = wrapJustBash(options.sandbox);
      usingJustBash = true;
    } else {
      sandbox = options.sandbox as Sandbox;
    }

    // Stream files and write in batches
    await writeFilesInBatches(sandbox, options.files, destination);
  } else {
    // Use just-bash (default)
    usingJustBash = true;
    sandbox = await createJustBashSandbox({
      files: loadFiles(options),
      cwd: destination,
    });
  }

  // 3. Generate tool prompt with file listing
  const toolPrompt = await createToolPrompt({
    sandbox,
    filenames: await getFilePaths(options),
    isJustBash: usingJustBash,
  });

  // 4. Create tools
  const tools = {
    bash: createBashExecuteTool({ sandbox, cwd: destination, files: fileList, ... }),
    readFile: createReadFileTool({ sandbox, cwd: destination }),
    writeFile: createWriteFileTool({ sandbox, cwd: destination }),
  };

  return { bash, tools, sandbox };
}
```

### 2. Sandbox Abstraction

```typescript
// src/types.ts
export interface Sandbox {
  executeCommand(command: string): Promise<{
    stdout: string;
    stderr: string;
    exitCode: number;
  }>;
  readFile(path: string): Promise<string>;
  writeFile(path: string, content: string): Promise<void>;
  writeFiles(batch: Array<{ path: string; content: Buffer }>): Promise<void>;
  stop(): Promise<void>;
}

// just-bash implementation
export async function createJustBashSandbox(options: {
  files?: Record<string, string>;
  cwd?: string;
}): Promise<Sandbox> {
  const bash = new Bash({
    cwd: options.cwd,
    env: { ...process.env },
  });

  // Write initial files to in-memory filesystem
  if (options.files) {
    for (const [path, content] of Object.entries(options.files)) {
      await bash.writeFile(path, content);
    }
  }

  return {
    async executeCommand(command: string) {
      const result = await bash.run(command);
      return {
        stdout: result.stdout,
        stderr: result.stderr,
        exitCode: result.exitCode,
      };
    },
    async readFile(path: string) {
      return await bash.readFile(path);
    },
    async writeFile(path: string, content: string) {
      await bash.writeFile(path, content);
    },
    async writeFiles(batch) {
      for (const file of batch) {
        await bash.writeFile(file.path, file.content.toString('utf-8'));
      }
    },
    async stop() {
      await bash.destroy();
    },
  };
}
```

### 3. Bash Tool Implementation (`src/tools/bash.ts`)

```typescript
export function createBashExecuteTool(options: {
  sandbox: Sandbox;
  cwd: string;
  files: string[];
  toolPrompt: string;
  extraInstructions?: string;
  onBeforeBashCall?: (params: { command: string }) => void | { command: string };
  onAfterBashCall?: (params: { command: string; result: any }) => void | { result: any };
  maxOutputLength?: number;
}) {
  return tool({
    description: `Execute bash commands in the sandbox.

${options.toolPrompt}

Available files:
${options.files.slice(0, 50).join('\n')}
${options.files.length > 50 ? `... and ${options.files.length - 50} more files` : ''}

Working directory: ${options.cwd}

${options.extraInstructions || ''}

Tip: Use 'ls -la' to explore, 'cat' to read files, 'grep -r' to search.`,

    inputSchema: z.object({
      command: z.string().describe('The bash command to execute'),
    }),

    execute: async ({ command }) => {
      // Before hook
      const beforeResult = options.onBeforeBashCall?.({ command });
      if (beforeResult?.command) {
        command = beforeResult.command;
      }

      // Execute command
      const result = await options.sandbox.executeCommand(command);

      // Truncate output if needed
      const maxOutputLength = options.maxOutputLength ?? 10000;
      let stdout = result.stdout;
      let stderr = result.stderr;

      if (stdout.length > maxOutputLength) {
        stdout = stdout.slice(0, maxOutputLength) + '\n... (output truncated)';
      }
      if (stderr.length > maxOutputLength) {
        stderr = stderr.slice(0, maxOutputLength) + '\n... (output truncated)';
      }

      const truncatedResult = { ...result, stdout, stderr };

      // After hook
      const afterResult = options.onAfterBashCall?.({ command, result: truncatedResult });
      return afterResult?.result ?? truncatedResult;
    },
  });
}
```

### 4. Tool Prompt Generation (`src/tools-prompt.ts`)

```typescript
export async function createToolPrompt(options: {
  sandbox: Sandbox;
  filenames: string[];
  isJustBash: boolean;
  toolPrompt?: string;
}): Promise<string> {
  const { sandbox, filenames, isJustBash } = options;

  // Get filesystem structure
  const fileTree = buildFileTree(filenames);

  // Build prompt
  return `You have access to a bash environment.

Working directory: /workspace

File structure:
${formatFileTree(fileTree)}

${filenames.length} files available.

Common commands:
- ls -la : List files
- cat <file> : View file contents
- grep -r "pattern" : Search files
- find . -name "*.ts" : Find files by extension
- head/tail <file> : View beginning/end of file
- wc -l <file> : Count lines

${isJustBash ? 'This is an in-memory filesystem. Changes persist only for this session.' : 'This is a full VM sandbox.'}

${options.toolPrompt || ''}`;
}
```

### 5. File Streaming (`src/files/loader.ts`)

```typescript
export async function* streamFiles(options: {
  files?: Record<string, string>;
  uploadDirectory?: { source: string; include?: string };
}): AsyncGenerator<{ path: string; content: Buffer }> {
  // Stream inline files
  if (options.files) {
    for (const [path, content] of Object.entries(options.files)) {
      yield { path, content: Buffer.from(content, 'utf-8') };
    }
  }

  // Stream from directory on disk
  if (options.uploadDirectory) {
    const { source, include } = options.uploadDirectory;
    const files = await glob(include ?? '**/*', { cwd: source });

    for (const file of files) {
      const content = await fs.readFile(path.join(source, file));
      yield { path: file, content };
    }
  }
}
```

### 6. Batch File Writing

```typescript
const WRITE_BATCH_SIZE = 20;

async function writeFilesInBatches(
  sandbox: Sandbox,
  files: Record<string, string>,
  destination: string
) {
  let batch: Array<{ path: string; content: Buffer }> = [];

  for await (const file of streamFiles({ files })) {
    batch.push({
      path: path.posix.join(destination, file.path),
      content: file.content,
    });

    if (batch.length >= WRITE_BATCH_SIZE) {
      await sandbox.writeFiles(batch);
      batch = [];
    }
  }

  // Write remaining files
  if (batch.length > 0) {
    await sandbox.writeFiles(batch);
  }
}
```

---

## Usage Examples

### Basic Usage

```typescript
import { createBashTool } from "bash-tool";
import { Agent, stepCountIs } from "ai";

const { tools, sandbox } = await createBashTool({
  files: {
    "src/index.ts": "export const hello = 'world';",
    "package.json": '{"name": "my-project"}',
  },
});

const agent = new Agent({
  model: model,
  tools,
  stopWhen: stepCountIs(20),
});

const result = await agent.generate({
  prompt: "Analyze the project and create a summary report",
});

await sandbox.stop();
```

### Upload Directory

```typescript
const { tools } = await createBashTool({
  uploadDirectory: {
    source: "./my-project",
    include: "**/*.{ts,json}",  // Glob filter
  },
});
```

### Using @vercel/sandbox (Full VM)

```typescript
import { Sandbox } from "@vercel/sandbox";

const sandbox = await Sandbox.create();
const { tools } = await createBashTool({
  sandbox,
  files: { "index.ts": "console.log('hello');" },
});
```

### Persistent Sandbox

```typescript
import { Sandbox } from "@vercel/sandbox";

// First invocation
const newSandbox = await Sandbox.create();
const sandboxId = newSandbox.sandboxId;
// Store sandboxId in database/session

// Subsequent invocations - reconnect
const existingSandbox = await Sandbox.get({ sandboxId });
const { tools } = await createBashTool({ sandbox: existingSandbox });
// All previous files and state preserved
```

### Command Interception

```typescript
const { tools } = await createBashTool({
  onBeforeBashCall: ({ command }) => {
    console.log("Running:", command);
    // Block dangerous commands
    if (command.includes("rm -rf")) {
      return { command: "echo 'Blocked dangerous command'" };
    }
  },
  onAfterBashCall: ({ command, result }) => {
    console.log(`Exit code: ${result.exitCode}`);
    return { result: { ...result, stdout: result.stdout.trim() } };
  },
});
```

---

## Tool Definitions

### bash

```typescript
{
  name: "bash",
  description: "Execute bash commands in the sandbox",
  inputSchema: z.object({
    command: z.string().describe("The bash command to execute"),
  }),
  returns: z.object({
    stdout: z.string(),
    stderr: z.string(),
    exitCode: z.number(),
  }),
}
```

### readFile

```typescript
{
  name: "readFile",
  description: "Read the contents of a file",
  inputSchema: z.object({
    path: z.string().describe("Path to the file"),
  }),
  returns: z.object({
    content: z.string(),
  }),
}
```

### writeFile

```typescript
{
  name: "writeFile",
  description: "Write content to a file",
  inputSchema: z.object({
    path: z.string().describe("Path where to write"),
    content: z.string().describe("Content to write"),
  }),
  returns: z.object({
    success: z.boolean(),
  }),
}
```

---

## File Structure

```
bash-tool/
├── src/
│   ├── files/
│   │   └── loader.ts           # File streaming utilities
│   ├── sandbox/
│   │   ├── just-bash.ts        # just-bash implementation
│   │   └── vercel.ts           # @vercel/sandbox wrapper
│   ├── tools/
│   │   ├── bash.ts             # Bash tool
│   │   ├── read-file.ts        # Read file tool
│   │   └── write-file.ts       # Write file tool
│   ├── index.ts                # Main entry
│   ├── tool.ts                 # createBashTool function
│   ├── tools-prompt.ts         # Tool prompt generation
│   └── types.ts                # TypeScript types
├── package.json
└── README.md
```

---

## Rust Implementation Considerations

### 1. Sandbox Trait

```rust
#[async_trait]
pub trait Sandbox: Send + Sync {
    async fn execute_command(&self, command: &str) -> Result<CommandOutput>;
    async fn read_file(&self, path: &str) -> Result<String>;
    async fn write_file(&self, path: &str, content: &str) -> Result<()>;
    async fn stop(&self) -> Result<()>;
}

pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}
```

### 2. In-Memory Filesystem (memfs)

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct InMemoryFs {
    files: Arc<RwLock<HashMap<String, String>>>,
}

impl InMemoryFs {
    pub fn new() -> Self {
        Self {
            files: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn write(&self, path: &str, content: &str) {
        self.files.write().await.insert(path.to_string(), content.to_string());
    }

    async fn read(&self, path: &str) -> Option<String> {
        self.files.read().await.get(path).cloned()
    }
}
```

### 3. Command Execution with memfs

```rust
pub struct MemFsSandbox {
    fs: InMemoryFs,
    cwd: String,
}

#[async_trait]
impl Sandbox for MemFsSandbox {
    async fn execute_command(&self, command: &str) -> Result<CommandOutput> {
        // Parse command and handle special cases
        if command.starts_with("cat ") {
            let path = command.strip_prefix("cat ").unwrap().trim();
            let full_path = format!("{}/{}", self.cwd, path);
            match self.fs.read(&full_path).await {
                Some(content) => Ok(CommandOutput {
                    stdout: content,
                    stderr: String::new(),
                    exit_code: 0,
                }),
                None => Ok(CommandOutput {
                    stdout: String::new(),
                    stderr: format!("cat: {}: No such file", path),
                    exit_code: 1,
                }),
            }
        } else {
            // For other commands, use actual shell
            let output = Command::new("sh")
                .arg("-c")
                .arg(command)
                .current_dir(&self.cwd)
                .output()
                .await?;

            Ok(CommandOutput {
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                exit_code: output.status.code().unwrap_or(1),
            })
        }
    }

    async fn read_file(&self, path: &str) -> Result<String> {
        let full_path = format!("{}/{}", self.cwd, path);
        self.fs.read(&full_path).await
            .ok_or_else(|| anyhow!("File not found: {}", path))
    }

    async fn write_file(&self, path: &str, content: &str) -> Result<()> {
        let full_path = format!("{}/{}", self.cwd, path);
        self.fs.write(&full_path, content).await;
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())  // Cleanup if needed
    }
}
```

### 4. Container-Based Sandbox

```rust
use bollard::{Docker, container::*};

pub struct ContainerSandbox {
    docker: Docker,
    container_id: String,
}

impl ContainerSandbox {
    pub async fn create(image: &str) -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;

        let config = Config {
            image: Some(image.to_string()),
            attach_stdin: Some(true),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            open_stdin: Some(true),
            tty: Some(true),
            ..Default::default()
        };

        let container = docker.create_container::<&str, &str>(None, config).await?;

        docker.start_container(&container.id).await?;

        Ok(Self {
            docker,
            container_id: container.id,
        })
    }
}

#[async_trait]
impl Sandbox for ContainerSandbox {
    async fn execute_command(&self, command: &str) -> Result<CommandOutput> {
        let config = ExecCreateContainerOptions {
            cmd: Some(vec!["sh", "-c", command]),
            attach_stdout: true,
            attach_stderr: true,
            ..Default::default()
        };

        let exec = self.docker.create_exec(&self.container_id, config).await?;

        // Start exec and capture output
        // ... (stream output handling)

        Ok(output)
    }

    async fn read_file(&self, path: &str) -> Result<String> {
        // Use docker cp equivalent
        // ...
    }

    async fn write_file(&self, path: &str, content: &str) -> Result<()> {
        // Create archive and use docker cp equivalent
        // ...
    }

    async fn stop(&self) -> Result<()> {
        self.docker.stop_container(&self.container_id, None).await?;
        self.docker.remove_container(&self.container_id, None).await?;
        Ok(())
    }
}
```

### 5. Tool Definition with AI SDK

```rust
use ai_sdk::{tool, ToolDefinition};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct BashInput {
    command: String,
}

#[derive(Serialize, Deserialize)]
struct BashOutput {
    stdout: String,
    stderr: String,
    exit_code: i32,
}

fn create_bash_tool(sandbox: Arc<dyn Sandbox>) -> ToolDefinition<BashInput, BashOutput> {
    tool(
        "bash",
        "Execute bash commands in the sandbox environment",
        |input: BashInput| async move {
            let result = sandbox.execute_command(&input.command).await?;
            Ok(BashOutput {
                stdout: result.stdout,
                stderr: result.stderr,
                exit_code: result.exit_code,
            })
        },
    )
}
```

---

## Key Takeaways

1. **Sandbox Abstraction** - Unified interface for different sandbox implementations
2. **File Streaming** - Batch writing for memory efficiency with large uploads
3. **Tool Prompt Generation** - Auto-generate filesystem context for agents
4. **Hook Support** - onBeforeBashCall/onAfterBashCall for command interception
5. **Multiple Modes** - In-memory (fast) or full VM (isolated)

---

## See Also

- [Main Vercel Labs Exploration](./exploration.md)
- [@vercel/sandbox](https://vercel.com/docs/vercel-sandbox)
- [just-bash npm](https://npmjs.com/package/just-bash)
