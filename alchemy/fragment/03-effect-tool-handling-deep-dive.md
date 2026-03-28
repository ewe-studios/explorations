# Effect-Based Tool Handling Deep Dive

## Overview

Fragment implements a sophisticated tool system built on the `effect` library that enables AI agents to interact with the external world through typed, composable functions. This document explores the complete architecture from tool definition through execution.

---

## 1. What Are Tools?

### Tool Definition and Purpose

Tools are **AI-accessible functions** with strictly typed inputs and outputs that extend an agent's capabilities beyond pure reasoning. They serve as the bridge between the AI's decision-making and real-world actions.

```typescript
export interface ITool<
  ID extends string,
  Input,
  Output,
  Err = never,
  Req = never,
  References extends any[] = any[],
> extends Fragment<"tool", ID, References> {
  readonly input: S.Schema<Input>;
  readonly output: S.Schema<Output>;
  readonly alias: ((model: string) => string | undefined) | undefined;
  readonly handler: (
    ...args: void extends Input ? [] : [Input]
  ) => Effect.Effect<Output, Err, Req>;
}
```

### Callable Functions with Typed I/O

Every tool is defined with:
- **Input Schema**: A `effect/Schema` that validates and types all input parameters
- **Output Schema**: A `effect/Schema` that defines the structure of returned data
- **Handler**: An Effect-gen function that performs the actual work

The typing ensures that:
```typescript
// Input is automatically inferred from referenced input() calls
type InputType = { command: string; timeout?: number }

// Output is automatically inferred from referenced output() calls
type OutputType = { exitCode: number; output: string }

// Handler return type is Effect<OutputType, ErrorType, RequirementType>
```

### Tool vs Regular Functions

| Aspect | Regular Function | Fragment Tool |
|--------|------------------|---------------|
| **Type Safety** | Runtime or compile-time only | Schema-validated at runtime |
| **AI Access** | Not directly accessible | Exposed to LLM with descriptions |
| **Description** | JSDoc comments only | Template string for AI understanding |
| **Composition** | Function calls | Effect.gen with yield* |
| **Error Handling** | throw/catch | Effect error channel |
| **Model Aliases** | N/A | Provider-specific naming (AnthropicBash) |

### AI-Accessible Tools

Tools are converted to a format that language models can understand:

```typescript
export const createEffectTool = <T extends Tool>(
  tool: T,
  model?: string,
  config?: RenderConfig,
): EffectTool.Any => {
  // Render description from template
  const description = renderTemplate(tool.template, tool.references, config);

  // Extract input schema fields
  const parameters = tool.input?.fields ?? {};

  // Get output schema
  const outputSchema = tool.output ?? S.Any;

  // Support model-specific aliases
  const toolName = model && tool.alias ? (tool.alias(model) ?? tool.id) : tool.id;

  return EffectTool.make(toolName, {
    description,
    parameters,
    success: outputSchema,
  });
};
```

---

## 2. Tool Inputs

### The input() Builder Function

Inputs are declared using the `input()` builder which creates a typed schema with optional description:

```typescript
export const input = <
  const ID extends string,
  Schema extends S.Struct.Field = typeof S.String,
>(
  id: ID,
  schema: Schema = S.String as any as Schema,
  options: { description?: string } = {},
): Input<ID, Schema, []> => {
  return {
    type: "input",
    id,
    schema,
    description: options?.description,
    template,
    references,
    render: {
      context: (input: Input<any, any, any[]>) => `\${${input.id}}`,
    },
  };
};
```

### String Inputs (Default)

The simplest input accepts any string:

```typescript
const command = input("command")`The command to execute`;
// Creates: { command: string }
```

### Typed Inputs with Schema

Use `effect/Schema` for type validation:

```typescript
import * as S from "effect/Schema";

const timeout = input(
  "timeout",
  S.optional(S.Number),
)`Optional timeout in milliseconds`;

const filePath = input(
  "filePath",
  S.String
)`The absolute path to the file to modify`;

const replaceAll = input(
  "replaceAll",
  S.Boolean,
)`Replace all occurrences (default false)`;
```

### Literal and Enum Inputs

For constrained values:

```typescript
// Constrain to specific action types
const action = input("action", S.Literal("read", "write", "edit"));

// Using enums
const sort = input("sort", S.Enums(SortOrder));
```

### Input Description for AI

The template string following `input()` provides context for the AI:

```typescript
const pattern = input(
  "pattern",
)`The regex pattern to search for in file contents.
Supports full regex syntax (e.g., "log.*Error", "function\\s+\\w+", etc.)`;

const path = input(
  "path",
  S.optional(S.String),
)`The directory to search in. Defaults to ${cwd} if not specified.`;
```

These descriptions are interpolated into the tool definition visible to the AI.

---

## 3. Tool Outputs

### The output() Builder Function

Outputs define the structure of data returned to the AI:

```typescript
export const output = <
  ID extends string,
  Schema extends S.Schema<any> = S.Schema<string>,
>(
  id: ID,
  schema: Schema = S.String as any as Schema,
): Output<ID, Schema, []> => {
  return {
    type: "output",
    id,
    schema,
    template,
    references,
    render: {
      context: (output: Output<any, any, any[]>) => `^{${output.id}}`,
    },
  };
};
```

### Schema-Validated Outputs

Outputs use the same Schema system as inputs:

```typescript
const exitCode = output("exitCode", S.Number);
const out = output("output", S.String);
```

### Struct Outputs (S.Struct)

Complex outputs use Struct for multiple fields:

```typescript
const result = output(
  "result",
  S.Struct({
    success: S.Boolean,
    message: S.String,
    data: S.optional(S.Record(S.String, S.Any)),
  })
)`The result of the operation`;
```

### Output Description for AI

Like inputs, outputs have descriptions:

```typescript
const matches = output(
  "matches",
)`The search results showing file paths and matching lines, sorted by modification time.`;

const content = output(
  "content",
)`The file content, or an error message if the file cannot be read.`;

const files = output(
  "files",
)`The list of matching file paths, sorted by modification time (most recent first).`;
```

---

## 4. Tool Definition

### Tool(name) Template Syntax

Tools are defined using a tagged template literal pattern:

```typescript
export const bash = Tool("bash", {
  alias: (model) => (model?.includes("claude") ? "AnthropicBash" : undefined),
})`Executes a given bash ${command} in a persistent shell session with optional ${timeout}.
Returns the ${exitCode} and ${out} containing both stdout and stderr.

Before executing the command, please follow these steps:

1. Directory Verification:
   - If the command will create new directories or files, first use \`ls\` to verify
   ...
`(function* ({ command, timeout, workdir }) {
  // Handler implementation
});
```

### Input/Output Interpolation

The template string interpolates inputs and outputs using `${name}` and `^{name}`:

```typescript
export const grep = Tool(
  "grep",
)`Fast content search tool that works with any codebase size.
Returns ${matches} with file paths and line numbers.

Given a ${pattern} and optional ${path} and ${include}:
- Searches file contents using regular expressions
- Filter files by pattern with the include parameter
`(function* ({ pattern, path: searchDir, include }) {
  // Handler
});
```

### Effect.gen Handler Functions

Handlers are generator functions wrapped with `Effect.fn`:

```typescript
handler: Effect.fn(handler)

// Where handler is:
function* (input: Input.Of<References>) {
  // Access services
  const fs = yield* FileSystem.FileSystem;
  const config = yield* Effect.serviceOption(FragmentConfig);

  // Execute commands
  const { stdout, stderr, exitCode } = yield* Command.make("rg", ...args).pipe(
    Command.stdout("pipe"),
    Command.stderr("pipe"),
    exec,
  );

  // Return structured output
  return { matches: output };
}
```

### yield* for Effect Composition

Effects are composed using `yield*` within generators:

```typescript
export const read = Tool("read")`Reads a file from the local filesystem.`(function* ({
  filePath,
  offset,
  limit,
}) {
  // Get services
  const config = yield* Effect.serviceOption(FragmentConfig).pipe(
    Effect.map(Option.getOrElse(() => ({ cwd: process.cwd() }))),
  );
  const fs = yield* FileSystem.FileSystem;
  const path = yield* Path.Path;

  // Check for security restrictions
  if (filePath.includes(".env")) {
    return { content: "Environment files (.env) are not readable" };
  }

  // Resolve path
  const resolvedPath = path.isAbsolute(filePath)
    ? filePath
    : path.join(config.cwd, filePath);

  // Check existence
  const exists = yield* fs
    .exists(resolvedPath)
    .pipe(Effect.catchAll(() => Effect.succeed(false)));

  if (!exists) {
    return { content: `File not found: ${filePath}` };
  }

  // Read content
  const fileContent = yield* fs
    .readFileString(resolvedPath)
    .pipe(Effect.catchAll((e) => Effect.succeed(`Failed: ${e}`)));

  return { content: fileContent.split("\n").slice(offset, offset + limit).join("\n") };
});
```

### Return Type Inference

The tool's return type is automatically inferred:

```typescript
// From the Tool() call signature:
<Eff extends YieldWrap<Effect.Effect<any, any, any>>>(
  handler: (input: Input.Of<References>) => Generator<
    Eff,
    NoInfer<Output.Of<References>>,
    never
  >
) => Tool<
  ID,
  Input.Of<References>,
  Output.Of<References>,
  [Eff] extends [YieldWrap<Effect.Effect<infer _A, infer E, infer _R>>] ? E : never,
  [Eff] extends [YieldWrap<Effect.Effect<infer _A, infer _E, infer R>>] ? R : never,
  References
>
```

---

## 5. Toolkit Composition

### Toolkit Builder

Toolkits group related tools together:

```typescript
const ToolkitBuilder = defineFragment("toolkit")<{}>({
  render: {
    context: (toolkit: Toolkit) => `🧰${toolkit.id}`,
  },
  get tools(): Tool[] {
    return collectFlat(
      (this as unknown as Fragment<"toolkit", string, any[]>).references,
      isTool,
    );
  },
});

export const Toolkit = <ID extends string>(id: ID) =>
  ToolkitBuilder(id) as unknown as <const References extends any[]>(
    template: TemplateStringsArray,
    ...references: References
  ) => Toolkit<ID, ExtractTools<References>, References>;
```

### Bundling Tools Together

```typescript
import { read } from "./read.ts";
import { write } from "./write.ts";
import { edit } from "./edit.ts";
import { glob } from "./glob.ts";
import { grep } from "./grep.ts";

export const CodingToolkit = Toolkit("coding")`
  File operations toolkit containing:
  ${read} - Read file contents
  ${write} - Write file contents
  ${edit} - Edit file with string replacement
  ${glob} - Find files by pattern
  ${grep} - Search file contents
` {}
```

### ${tool} Interpolation in Toolkits

Tool references in templates render with emoji and name:

```typescript
render: {
  context: (tool: Tool) => `🛠️${tool.id}`,
}

// In toolkit context:
// 🛠️read, 🛠️write, 🛠️edit, 🛠️glob, 🛠️grep
```

### Agent Inheritance of Toolkit Tools

Agents inherit tools from referenced toolkits:

```typescript
// Agent references toolkit
const agent = Agent("coder")`
  You are a coding assistant.
  You have access to 🧰coding toolkit.
`([CodingToolkit]);

// Tools are collected transitively
export const collectToolkits = (agent: Agent): Toolkit[] =>
  collectReferences(agent.references ?? [], {
    matches: isToolkit,
    shouldRecurse: (v) => isToolkit(v) || isFile(v) || isTool(v) || isRole(v),
  });
```

---

## 6. Tool Registration

### Tool Name to Handler Mapping

Tools are registered with their handlers via `createHandlerLayer`:

```typescript
export const createHandlerLayer = (
  toolkits: Toolkit[],
  model?: string,
  config?: RenderConfig,
): Layer.Layer<EffectTool.Handler<string>, never, never> => {
  const allTools = toolkits.flatMap((tk) => tk.tools);

  const handlers: Record<string, (params: any) => any> = {};
  for (const tool of allTools) {
    const handler = (params: any) => (tool.handler as any)(params);

    const toolName = model && tool.alias
      ? (tool.alias(model) ?? tool.id)
      : tool.id;

    handlers[toolName] = handler;
  }

  const effectToolkit = EffectToolkit.merge(
    ...toolkits.map((tk) => createEffectToolkit(tk, model, config)),
  );

  return effectToolkit.toLayer(handlers);
};
```

### Model-Specific Aliases

Tools can have provider-specific names:

```typescript
export const bash = Tool("bash", {
  alias: (model) => (model?.includes("claude") ? "AnthropicBash" : undefined),
})`...`(handler);

// For Claude models: tool registers as "AnthropicBash"
// For other models: tool registers as "bash"
```

### Handler Layer Construction

The handler layer connects Effect tools to @effect/ai:

```typescript
// Convert Toolkit to Effect Toolkit
export const createEffectToolkit = <T extends Toolkit>(
  toolkit: T,
  model?: string,
  config?: RenderConfig,
): EffectToolkit.Toolkit<EffectToolkit.ToolsByName<EffectTool.Any[]>> => {
  const effectTools = toolkit.tools.map((tool) =>
    createEffectTool(tool, model, config),
  );
  return EffectToolkit.make(...effectTools);
};

// Create handler layer that executes tool calls
return {
  messages,
  toolkit: effectToolkit,
  toolkitHandlers: createHandlerLayer(allToolkits, model),
};
```

### Effect Provisioning

Effects require their dependencies to be provided via Layers:

```typescript
const context = createContext(agent, options).pipe(
  Effect.provide(FileSystem.layer),
  Effect.runPromise,
);
```

---

## 7. Tool Execution Flow

### Execution Sequence

```
┌─────────────────────────────────────────────────────────────────┐
│                    AI Decision to Call Tool                      │
│            (Analyzes context, determines needed action)          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Tool Call Parsing                             │
│         (Extract tool name and parameters from LLM output)       │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Input Validation                              │
│           (Schema validation of parameters via effect/Schema)    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Handler Execution                             │
│         (Effect.gen runs with yielded effects composed)          │
│    - Service acquisition (FileSystem, Command, Path, etc.)      │
│    - Effect composition (pipe, flatMap, catchAll)               │
│    - External interaction (file ops, process execution)         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Result Formatting                             │
│          (Output schema validation and structure creation)       │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Response to AI                                │
│         (Tool result message added to conversation)              │
└─────────────────────────────────────────────────────────────────┘
```

### AI Decides to Call Tool

The AI receives tool definitions in its system prompt:

```
You can (and should) use the following tools to accomplish your tasks.

## Toolkits

### coding

🛠️read - Reads a file from the local filesystem
🛠️write - Writes a file to the local filesystem
🛠️edit - Performs exact string replacements in files
🛠️glob - Fast file pattern matching tool
🛠️grep - Fast content search tool
```

### Tool Call Parsing

Tool calls are parsed from the model response:

```typescript
// Tool call format from LLM
{
  type: "tool-call",
  id: "call_123",
  name: "read",
  params: { filePath: "src/index.ts" },
  providerExecuted: false,
}
```

### Input Validation

Schemas validate inputs automatically:

```typescript
// Schema validation happens when handler is called
const handler = (params: any) => (tool.handler as any)(params);

// Invalid input would fail at schema parse time
```

### Handler Execution

The handler runs as an Effect:

```typescript
// Handler execution context
function* ({ command, workdir }) {
  const config = yield* Effect.serviceOption(FragmentConfig);
  const validator = yield* Effect.serviceOption(CommandValidator);

  // Security validation
  if (validator) {
    const validationError = yield* validator.validate(command);
    if (validationError) {
      return `Security violation: ${validationError}`;
    }
  }

  // Command execution
  const cmd = Command.make(command).pipe(
    Command.runInShell(true),
    Command.workingDirectory(workdir ?? config.cwd),
  );

  const [exitCode, output] = yield* pipe(
    Command.start(cmd),
    Effect.flatMap((process) =>
      Effect.all(
        [
          process.exitCode,
          Stream.merge(process.stdout, process.stderr)
            .pipe(Stream.decodeText())
            .pipe(Stream.mkString),
        ],
        { concurrency: 3 },
      ),
    ),
  );

  return { exitCode, output };
}
```

### Result Formatting

Results are formatted according to output schema:

```typescript
// Tool result format
{
  type: "tool-result",
  id: "call_123",
  name: "read",
  isFailure: false,
  result: { content: "file contents here..." },
  providerExecuted: false,
}
```

---

## 8. Built-in Tools

### bash: Shell Command Execution

```typescript
export const bash = Tool("bash", {
  alias: (model) => (model?.includes("claude") ? "AnthropicBash" : undefined),
})`Executes a given bash ${command} in a persistent shell session.
Returns the ${exitCode} and ${out} containing both stdout and stderr.`(function* ({ command, workdir }) {
  const config = yield* Effect.serviceOption(FragmentConfig);
  const validator = yield* Effect.serviceOption(CommandValidator);

  // Security validation
  if (validator) {
    const validationError = yield* validator.validate(command).pipe(
      Effect.map(() => null),
      Effect.catchAll((e) => Effect.succeed(`${e}`)),
    );
    if (validationError) {
      return `Security violation: ${validationError}`;
    }
  }

  const cmd = Command.make(command).pipe(
    Command.runInShell(true),
    Command.workingDirectory(workdir ?? config.cwd),
  );

  const [exitCode, output] = yield* pipe(
    Command.start(cmd),
    Effect.flatMap((process) =>
      Effect.all(
        [
          process.exitCode,
          Stream.merge(process.stdout, process.stderr)
            .pipe(Stream.decodeText())
            .pipe(Stream.mkString),
        ],
        { concurrency: 3 },
      ),
    ),
  );

  return { exitCode, output };
});
```

**Key Features:**
- Security validation via CommandValidator service
- Working directory support via `workdir` parameter
- Concurrent stdout/stderr collection
- Timeout support (default 120s, max 600s)
- Model-specific aliasing (AnthropicBash for Claude)

### read: File Reading

```typescript
export const read = Tool("read")`Reads a file from the local filesystem.`(function* ({
  filePath,
  offset = 0,
  limit = 2000,
}) {
  const config = yield* Effect.serviceOption(FragmentConfig);
  const fs = yield* FileSystem.FileSystem;
  const path = yield* Path.Path;

  // Security: block .env files
  if (filePath.includes(".env")) {
    return { content: "Environment files (.env) are not readable" };
  }

  const resolvedPath = path.isAbsolute(filePath)
    ? filePath
    : path.join(config.cwd, filePath);

  const exists = yield* fs.exists(resolvedPath)
    .pipe(Effect.catchAll(() => Effect.succeed(false)));

  if (!exists) {
    // Provide suggestions
    const dir = path.dirname(resolvedPath);
    const base = path.basename(resolvedPath);
    const files = yield* fs.readDirectory(dir)
      .pipe(Effect.catchAll(() => Effect.succeed([])));

    const suggestions = files
      .filter((e) => e.toLowerCase().includes(base.toLowerCase()))
      .slice(0, 3);

    if (suggestions.length > 0) {
      return { content: `File not found. Did you mean?\n${suggestions.join("\n")}` };
    }
    return { content: `File not found: ${filePath}` };
  }

  const content = yield* fs.readFileString(resolvedPath)
    .pipe(Effect.catchAll((e) => Effect.succeed(`Failed: ${e}`)));

  return { content: content.split("\n").slice(offset, offset + limit).join("\n") };
});
```

**Key Features:**
- .env file security blocking
- Path suggestion on not-found
- Offset/limit for large files
- Directory detection

### write: File Writing

```typescript
export const write = Tool("write")`Writes a file to the local filesystem.`(function* ({
  filePath,
  content,
}) {
  const config = yield* Effect.serviceOption(FragmentConfig);
  const path = yield* Path.Path;
  const fs = yield* FileSystem.FileSystem;

  const resolvedPath = path.isAbsolute(filePath)
    ? filePath
    : path.join(config.cwd, filePath);

  // Auto-create parent directories
  const dir = path.dirname(resolvedPath);
  yield* fs.makeDirectory(dir, { recursive: true })
    .pipe(Effect.catchAll(() => Effect.void));

  const writeResult = yield* fs.writeFileString(resolvedPath, content)
    .pipe(Effect.catchAll((e) => Effect.succeed(`Failed: ${e}`)));

  // Get LSP diagnostics
  const diagnostics = yield* getDiagnosticsIfAvailable(resolvedPath, content);
  const formatted = formatDiagnostics(diagnostics);

  return {
    result: formatted
      ? `Wrote file: ${filePath}\n\n${formatted}`
      : `Wrote file: ${filePath}`,
  };
});
```

**Key Features:**
- Auto-creates parent directories
- LSP diagnostic feedback
- Relative path handling

### edit: Code Editing

```typescript
export const edit = Tool("edit")`Performs exact string replacements.`(function* ({
  filePath,
  oldString,
  newString,
  replaceAll = false,
}) {
  const config = yield* Effect.serviceOption(FragmentConfig);
  const path = yield* Path.Path;
  const fs = yield* FileSystem.FileSystem;

  const resolvedPath = path.isAbsolute(filePath)
    ? filePath
    : path.join(config.cwd, filePath);

  // Handle file creation (empty oldString)
  if (oldString === "") {
    newContent = newString;
  } else {
    // Validate file exists
    const stat = yield* fs.stat(resolvedPath)
      .pipe(Effect.catchAll(() => Effect.succeed(null)));

    if (!stat) {
      return { result: `File not found: ${filePath}` };
    }

    // Read existing content
    const oldContent = yield* fs.readFileString(resolvedPath);

    // Perform replacement with error handling
    const replaceResult = yield* replace(oldContent, oldString, newString, replaceAll)
      .pipe(
        Effect.catchTag("ReplaceSameStringError", () =>
          Effect.succeed("oldString and newString must be different"),
        ),
        Effect.catchTag("ReplaceNotFoundError", (e) =>
          Effect.succeed(`Could not find "${e.oldString.slice(0, 100)}..." in ${filePath}`),
        ),
        Effect.catchTag("ReplaceMultipleMatchesError", (e) =>
          Effect.succeed(`Found multiple matches. Use replaceAll=true or provide more context.`),
        ),
      );

    if (replaceResult.startsWith("Could not find") ||
        replaceResult.startsWith("Found multiple")) {
      return { result: replaceResult };
    }
    newContent = replaceResult;
  }

  // Write changes
  yield* fs.writeFileString(resolvedPath, newContent);

  // Get LSP diagnostics
  const diagnostics = yield* getDiagnosticsIfAvailable(resolvedPath, newContent);

  return {
    result: diagnostics
      ? `${oldString === "" ? "Created" : "Edited"} file: ${filePath}\n\n${formatDiagnostics(diagnostics)}`
      : `${oldString === "" ? "Created" : "Edited"} file: ${filePath}`,
  };
});
```

**Key Features:**
- First-occurrence replacement (default)
- replaceAll option for multiple matches
- File creation with empty oldString
- Detailed error messages
- LSP diagnostics

### glob: File Pattern Matching

```typescript
export const glob = Tool("glob")`Fast file pattern matching tool.`(function* ({
  pattern,
  path: searchDir,
}) {
  const config = yield* Effect.serviceOption(FragmentConfig);
  const path = yield* Path.Path;
  const fs = yield* FileSystem.FileSystem;

  let searchPath = searchDir || config.cwd;
  searchPath = path.isAbsolute(searchPath)
    ? searchPath
    : path.resolve(config.cwd, searchPath);

  const fileList: { path: string; mtime: number }[] = [];
  const limit = 100;
  let truncated = false;

  const foundFiles = yield* Ripgrep.findFiles({
    cwd: searchPath,
    glob: [pattern],
  }).pipe(Effect.catchAll(() => Effect.succeed([])));

  for (const filePath of foundFiles) {
    if (fileList.length >= limit) {
      truncated = true;
      break;
    }
    const stats = yield* fs.stat(filePath)
      .pipe(Effect.catchAll(() => Effect.succeed(null)));
    if (!stats) continue;
    fileList.push({
      path: filePath,
      mtime: stats.mtime.pipe(Option.getOrUndefined)?.getTime() || 0,
    });
  }

  fileList.sort((a, b) => b.mtime - a.mtime);

  if (fileList.length === 0) {
    return { files: `No files found matching "${pattern}" in ${searchPath}` };
  }

  return {
    files: fileList.map((f) => f.path).join("\n") +
      (truncated ? `\n\n(Results truncated, consider more specific pattern)` : ""),
  };
});
```

**Key Features:**
- ripgrep-based file finding
- Modification time sorting
- Result limit (100 files)
- Truncation warning

### grep: Content Search

```typescript
export const grep = Tool("grep")`Fast content search tool.`(function* ({
  pattern,
  path: searchDir,
  include,
}) {
  const config = yield* Effect.serviceOption(FragmentConfig);
  const fs = yield* FileSystem.FileSystem;

  const searchPath = searchDir || config.cwd;

  const rgArgs = ["-nH", "--field-match-separator=|", "--regexp", pattern];
  if (include) {
    rgArgs.push("--glob", include);
  }
  rgArgs.push(searchPath);

  const { stdout, stderr, exitCode } = yield* Command.make("rg", ...rgArgs).pipe(
    Command.stdout("pipe"),
    Command.stderr("pipe"),
    exec,
    Effect.catchAll(() => Effect.succeed({ stdout: "", stderr: "", exitCode: 1 })),
  );

  if (exitCode === 1) {
    return { matches: `No matches found for "${pattern}" in ${searchPath}` };
  }

  const lines = stdout.split(/\r?\n/);
  const matchList: { path: string; modTime: number; lineNum: number; lineText: string }[] = [];

  for (const line of lines) {
    if (!line) continue;
    const [filePath, lineNumStr, ...lineTextParts] = line.split("|");
    if (!filePath || !lineNumStr) continue;

    const stats = yield* fs.stat(filePath)
      .pipe(Effect.catchAll(() => Effect.succeed(null)));
    if (!stats) continue;

    const modTime = stats.mtime.pipe(Option.getOrUndefined);
    if (!modTime) continue;

    matchList.push({
      path: filePath,
      modTime: modTime.getTime(),
      lineNum: parseInt(lineNumStr, 10),
      lineText: lineTextParts.join("|"),
    });
  }

  matchList.sort((a, b) => b.modTime - a.modTime);

  const limit = 100;
  const truncated = matchList.length > limit;
  const finalMatches = truncated ? matchList.slice(0, limit) : matchList;

  // Format output
  const outputLines = [`Found ${finalMatches.length} matches`];
  let currentFile = "";
  for (const match of finalMatches) {
    if (currentFile !== match.path) {
      currentFile = match.path;
      outputLines.push(`\n${match.path}:`);
    }
    const truncatedText = match.lineText.length > 2000
      ? match.lineText.substring(0, 2000) + "..."
      : match.lineText;
    outputLines.push(`  Line ${match.lineNum}: ${truncatedText}`);
  }

  return { matches: outputLines.join("\n") };
});
```

**Key Features:**
- ripgrep for fast searching
- File glob filtering
- Modification time sorting
- Line number output
- Result truncation

---

## 9. Error Handling

### Tool Execution Errors

Errors flow through the Effect error channel:

```typescript
// Error types inferred from handler
type ToolError = "ReplaceSameStringError" | "ReplaceNotFoundError" | "ReplaceMultipleMatchesError"

// Caught and converted to user messages
const replaceResult = yield* replace(oldContent, oldString, newString, replaceAll)
  .pipe(
    Effect.catchTag("ReplaceSameStringError", () =>
      Effect.succeed("oldString and newString must be different"),
    ),
    Effect.catchTag("ReplaceNotFoundError", (e) =>
      Effect.succeed(`Could not find "${e.oldString.slice(0, 100)}..." in ${filePath}`),
    ),
    Effect.catchTag("ReplaceMultipleMatchesError", (e) =>
      Effect.succeed(`Found multiple matches. Use replaceAll=true.`),
    ),
  );
```

### Validation Failures

Input validation failures are caught:

```typescript
// File not found
if (!exists) {
  return { content: `File not found: ${filePath}` };
}

// Directory instead of file
if (stat?.type === "Directory") {
  return {
    content: `Cannot read directory as a file: ${filePath}\nContents:\n${entries.join("\n")}`,
  };
}

// Security violation
if (filePath.includes(".env")) {
  return { content: "Environment files (.env) are not readable for security reasons" };
}
```

### Error Messages to AI

Errors are formatted as tool results:

```typescript
// Read error
{ result: "File not found: /path/to/file.ts" }

// Edit error
{ result: `Could not find oldString in file. The text "import { Effect }..." was not found in src/index.ts.` }

// Edit error - multiple matches
{ result: `Found multiple matches for oldString "const x = 1". Provide more surrounding context to identify the correct match, or use replaceAll=true.` }

// Bash security error
{ output: "Security violation: command 'rm -rf /' is not allowed" }
```

### Retry Strategies

The AI handles retries based on error messages:

```
Error Type              →  AI Retry Strategy
─────────────────────────────────────────────────
File not found          →  Use glob to find correct path
Multiple matches        →  Add more context or use replaceAll=true
Text not found          →  Re-read file to get current content
Security violation      →  Use alternative approach
Timeout                 →  Increase timeout or optimize command
```

Example retry flow:

```
AI: ${edit} with oldString="const x"
Result: Found multiple matches for "const x"

AI: ${edit} with oldString="const x = 1;" newString="const x = 2;" replaceAll=true
Result: Edited file: src/index.ts
```

---

## Appendix: Tool Comparison Table

| Tool | Primary Use | Input Types | Output Structure | Special Features |
|------|-------------|-------------|------------------|------------------|
| bash | Shell commands | command: string, timeout?: number, workdir?: string | { exitCode: number, output: string } | Security validation, model alias |
| read | File reading | filePath: string, offset?: number, limit?: number | { content: string } | .env blocking, suggestions |
| write | File writing | filePath: string, content: string | { result: string } | Auto mkdir, LSP diagnostics |
| edit | String replacement | filePath, oldString, newString, replaceAll?: boolean | { result: string } | Error recovery, LSP diagnostics |
| glob | File finding | pattern: string, path?: string | { files: string } | mtime sorting, truncation |
| grep | Content search | pattern: string, path?: string, include?: string | { matches: string } | ripgrep, line numbers |

---

## Appendix: Execution Flow Summary

```
┌─────────────────────────────────────────────────────────────────────┐
│                         TOOL EXECUTION FLOW                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  1. AI Context Preparation                                          │
│     └─> createContext(agent) builds system prompt with tools        │
│     └─> createEffectToolkit() converts tools to @effect/ai format   │
│     └─> createHandlerLayer() builds handler implementations         │
│                                                                      │
│  2. Tool Call Reception                                             │
│     └─> LLM outputs tool call with name and parameters              │
│     └─> Tool call parsed into { name, params, id }                  │
│                                                                      │
│  3. Input Validation                                                │
│     └─> Schema validation against tool.input                        │
│     └─> Automatic type coercion and validation                      │
│                                                                      │
│  4. Handler Execution                                               │
│     └─> Effect.fn(handler) runs as generator                        │
│     └─> Services acquired via yield* Effect.service(Foo)            │
│     └─> Effects composed via yield* effect.pipe(...)                │
│     └─> External actions performed (fs, commands, etc.)             │
│                                                                      │
│  5. Error Handling                                                  │
│     └─> Errors caught via Effect.catchTag/Effect.catchAll           │
│     └─> Errors converted to user-friendly messages                  │
│     └─> Messages returned as tool results                           │
│                                                                      │
│  6. Output Formatting                                               │
│     └─> Return value validated against tool.output                  │
│     └─> Result structured as { result: ... } or { matches: ... }    │
│                                                                      │
│  7. Response to AI                                                  │
│     └─> Tool result message added to conversation                   │
│     └─> AI continues with next action                               │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Key Takeaways

1. **Schema-Driven**: All I/O is validated through `effect/Schema`, ensuring type safety at runtime.

2. **Effect-Based**: Handlers are Effect generators that compose async operations cleanly.

3. **Template Literals**: Tool definitions use tagged templates for natural language descriptions.

4. **Model Aliases**: Tools can have provider-specific names (AnthropicBash for Claude).

5. **Toolkit Composition**: Tools group into toolkits, which agents inherit via references.

6. **Layer Architecture**: Handlers are provided via Effect Layer for dependency injection.

7. **Error Recovery**: Detailed error messages enable AI to self-correct and retry.

8. **Built-in Tools**: bash, read, write, edit, glob, grep provide comprehensive file/shell access.
