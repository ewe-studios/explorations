# 09 -- Tools, Registry, Settings, and Environment

The harness is not only about models. It runs tools, loads extensions, resolves settings, and builds a per-conversation environment. This document is a deep dive into all four, starting with how a tool call runs from admission to settlement.

## The Tool Task

A tool is a typed function the model or another tool can call. `ToolTask` is the built-in task of kind `pi.tool` that runs one call. Its checkpoint has two phases.

```ts
// packages/durable/src/harness/tool.ts -- the checkpoint (verbatim)
export type ToolTaskCheckpoint =
  | { phase: "call" }
  | { phase: "execute"; arguments: JsonObject; replay: "safe" | "unsafe" };
```

The Aha is the split. `call` does everything: resolve the tool, validate the arguments, record intent, and run it. `execute` is reached only by recovery, after a crash between recording intent and settling. It applies the replay rule.

```ts
// packages/durable/src/harness/tool.ts -- the two inputs (verbatim)
export type ToolTaskInput =
  | { kind: "model"; assistant: EntryId; callId: string }
  | { kind: "nested"; parent: TaskId; parentCallId: string; key: string; call: ToolCall; progress?: false };
```

A model-issued call reads its arguments from the assistant entry. A nested call, one a running tool made through `executeTool()`, carries them directly, because no entry holds them.

## The Call Phase

The `call` phase resolves, validates, records intent, then runs, all in one handler.

```ts
// packages/durable/src/harness/tool.ts -- the call phase (abridged)
call: async (task, runtime, context) => {
  const call = await readCall(runtime, task.input, context);
  const tool = await resolveTool(runtime, task.input, call.name, context);
  if (tool === undefined) return settle(..., harnessError("tool_unavailable", `Tool ${call.name} is not available`), ...);

  const prepared = prepare(tool, call.arguments as JsonObject);
  const checked = "error" in prepared ? prepared : validate(tool, call, prepared.args);
  if ("error" in checked) return settle(..., invalid(checked.error), ...);

  let args = checked.args;
  let block: string | undefined;
  await runtime.hooks.each("beforeTool", async (hook) => {
    if (block !== undefined) return;
    const decision = await hook({ ...call, arguments: args }, runtime, context);
    if (decision?.block !== undefined) block = decision.block;
    else if (decision?.arguments !== undefined) args = decision.arguments;
  });
  if (block !== undefined) return settle(..., harnessError("blocked", `Tool call blocked: ${block}`), ...);

  const validated = validate(tool, call, args);   // re-validate after hooks may have changed args
  if ("error" in validated) return settle(..., invalid(validated.error), ...);

  const final = copyJson(validated.args, { omitUndefinedProperties: true }) as JsonObject;
  await runtime.commit(async (tx) => {
    // mark the slot running, record durable intent
    const intent = { phase: "execute", arguments: final, replay: tool.replay ?? "unsafe" } as const;
    return { status: "running", checkpoint: intent };
  }, context);
  await run(runtime, task.input, call, tool, final, context);
}
```

The pipeline is six steps, and every failure settles instead of throwing.

1. **Resolve** the tool among the caller's tools.
2. **Prepare** the arguments with `prepareArguments`, which repairs common model mistakes.
3. **Validate** against the tool's `parameters`.
4. **Hook** `beforeTool`: the first `block` wins, otherwise `arguments` replace the call's arguments.
5. **Re-validate** after hooks may have changed the arguments.
6. **Commit intent**, then run.

The Aha is the commit of intent. Before execution, the task writes `{ phase: "execute", arguments, replay }` and marks itself `running`. That single commit is the recovery point. A crash before it leaves the task in `call`, which re-resolves cleanly. A crash after it leaves the task in `execute`.

### The Replay Rule

`execute` is reached only by recovery. It reruns only when both the stored and the current policy say `safe`.

```ts
// packages/durable/src/harness/tool.ts -- the execute phase (abridged)
execute: async (task, runtime, context) => {
  const { arguments: args, replay } = task.state.checkpoint;
  const call = await readCall(runtime, task.input, context);
  const tool = await resolveTool(runtime, task.input, call.name, context);
  if (replay === "safe" && tool?.replay === "safe") {
    await runtime.commit(async (tx) => { /* clear progress */ }, context);
    return run(runtime, task.input, call, tool, args, context);
  }
  const message = `Tool ${call.name} was interrupted and may have partially run`;
  const ending = { status: "failed", message } as const;
  await settle(runtime, task.input, call, ending, (slot) => fromSlot(slot, "interrupted", message), context);
}
```

The Aha is the double check. The stored `replay` is what the call recorded at intent time. The current `tool.replay` is what the tool is now. Both must be `safe`. If the tool was downgraded to `unsafe` since, the interrupted call is not rerun. An `unsafe` call ends `failed` with the message that it may have partially run.

The `replay` flag is per tool. A tool that sends an email is `unsafe`. A tool that reads a file is `safe`. After a crash, the scheduler re-runs only safe tools automatically.

## Running and Progress

`run` executes the tool and throttles progress commits. Progress is committed into the call's slot in `pi.live`, not into the tool result.

```ts
// packages/durable/src/harness/tool.ts -- run (abridged)
async function run(runtime, input, call, tool, args, context) {
  const limits = {
    maxBytes: tool.outputLimits?.maxBytes ?? DEFAULT_MAX_BYTES,
    maxLines: tool.outputLimits?.maxLines ?? DEFAULT_MAX_LINES,
    retain: tool.outputLimits?.retain ?? "head",
  };
  const reported = { output: new OutputBuffer(limits), limits, diagnostics: [], details: undefined };
  const streams = input.kind === "model" || input.progress !== false;
  const progress = publishProgress(runtime, reported, streams, context);
  // ...
  env = await runtime.env(context);
  try {
    result = await tool.execute(args, api, context);
  } catch (error) {
    // aborted -> rethrow; otherwise -> failed result with the error
  }
  // settle the result
}
```

The output is bounded. The default limits are 2000 lines or 50 KiB, whichever is hit first.

```ts
// packages/durable/src/truncate.ts -- the defaults (verbatim)
export const DEFAULT_MAX_LINES = 2000;
export const DEFAULT_MAX_BYTES = 50 * 1024; // 50KB
```

Progress is throttled by the progress policy.

```ts
// packages/durable/src/harness/tool.ts -- publishProgress (abridged)
function publishProgress(runtime, reported, enabled, context): Progress {
  let written = { text: "", details: undefined, diagnostics: 0 };
  return new Progress(
    async () => {
      if (!enabled) return 0;
      const snapshot = reported.output.snapshot();
      // capture everything synchronously: the tool keeps reporting while the commit is in flight
      // write only what changed since the last commit
      await runtime.commit(async (tx) => {
        const slot = toolSlot(await tx.doc(LiveDoc, runtime.conversationId), runtime.taskId);
        if (slot === undefined) return undefined;
        if ((slot.output ?? "") !== snapshot.text) slot.output = snapshot.text;
        // ...
      }, context);
      written = current;
      return bytes;
    },
    (error) => { if (!runtime.signal.aborted) runtime.report(error); },
    runtime.settings.progress.outputIntervalMs,
  );
}
```

The Aha is the string field. `slot.output` is assigned as one string field. chord then diffs it into an append, or a trim plus an append for a sliding tail. Replacing the slot object would record the whole window on every commit. One string field is what makes streaming output cheap to store.

### The Tool API

A tool executes against a `ToolExecutionApi`. It is a plain object of operations, every one of which rejects after the invocation ends.

```ts
// packages/durable/src/harness/types.ts -- the api (abridged)
export interface ToolExecutionApi extends DocumentObserver, DocumentReader {
  readonly taskId: TaskId;
  readonly conversationId: ConversationId;
  readonly callId: string;
  readonly registry: RegistrySnapshot;
  agent(context): Promise<Agent>;
  readonly models: Models;
  readonly env: ExecutionEnv | undefined;
  output(chunk: string | Uint8Array, skipped?): void;
  readonly outputWindow: ShellOutputWindow | undefined;
  retainedOutput(): { text: string; truncated: boolean };
  diagnostic(diagnostic: ToolDiagnostic): void;
  details(value: TDetails, context): Promise<void>;
  commit<T>(change: (tx: Tx) => T | Promise<T>, context): Promise<T>;
  memo<T>(name, context): Promise<T | undefined>;
  createTask(task, input, options, context): Promise<TaskId>;
  getTask(id, context): Promise<TaskRecord | undefined>;
  waitForTask(id, context): Promise<SettledTask>;
  conversation(id, context): Promise<ConversationHandle | undefined>;
  executeTool(name, args, context, options?): Promise<NestedToolExecutionResult>;
}
```

A tool writes output with `output()`, records diagnostics with `diagnostic()`, replaces live details with `details()`, and makes its own committed changes with `commit()`. It can spawn child tasks with `createTask()` and call other tools with `executeTool()`.

The Aha is that the API is the entire world a tool sees. A tool never touches storage, the Session line, or the registry directly. It talks to the harness through this one object, which checks the invocation on every call.

## Nested Calls

A tool can call another tool. `executeTool` runs it as a nested call: its own `pi.tool` task, owned by the calling task.

```ts
// packages/durable/src/harness/tool.ts -- executeTool (abridged)
executeTool: async (name, toolArgs, callContext, options = {}) => {
  const key = options.key ?? String(++sequence);
  if (options.key !== undefined) checkKey(options.key);
  madeNestedMemo ??= runtime.memo(MADE_NESTED_CALLS, true, callContext).catch((error) => {
    madeNestedMemo = undefined;
    throw error;
  });
  const admission = madeNestedMemo.then(() =>
    startNestedCall(runtime, call.id, name, toolArgs, { key, progress: options.progress, abandonOnRestart: !resumes }, callContext),
  );
  admissions.push(admission);
  const id = await admission;
  const settled = await runtime.waitForTask(id, callContext);
  const stored = (await runtime.snapshot(NestedCallDoc, runtime.taskId, key, callContext))?.result;
  return stored === undefined
    ? fallbackResult(id, name, settled.state.outcome)
    : copyJson(stored);
}
```

The nested call's result is stored in a `NestedCallDoc`, one document per call.

```ts
// packages/durable/src/harness/tool.ts -- the nested-call document (abridged)
export const NestedCallDoc = defineDocFamily<NestedCallState, null>({
  kind: "pi.tool.nested-call",
  version: 1,
  scope: "task",
  family: true,
  initial: () => ({}),
});
```

The Aha is the key. A nested call is named by a key within its parent call. By default the key is the call's position (`1`, `2`, ...). A rerun of a replay-safe tool that makes a call with the same key, in the same order, gets the same nested call back instead of starting another. The key makes reattachment possible.

A memo `MADE_NESTED_CALLS` is committed before the first nested call of an attempt.

```ts
// packages/durable/src/harness/tool.ts -- the memo (verbatim)
const MADE_NESTED_CALLS = "pi.tool.madeNestedCalls";
```

The Aha is why the memo is committed first. Only after it is durable can `pi.live` hold slots below the call, which settling removes. A crash between the memo and the call only costs a scan. Without the memo, settling could not know which slots to remove.

The nested call key is guarded.

```ts
// packages/durable/src/harness/tool.ts -- key rules (abridged)
function checkKey(key: string): void {
  if (key === "" || key.includes("/") || key === "__proto__" || /^[1-9][0-9]*$/.test(key)) {
    throw new Error(`Nested call key ${JSON.stringify(key)} must be non-empty, without "/", not "__proto__", and not a positive integer`);
  }
}
```

The key becomes a path segment of the call ID `<callId>/<key>`, so it cannot contain `/`. It cannot be `__proto__` or a positive integer, which default keys use.

## The Three Result Channels

A tool returns on three channels.

```ts
// packages/durable/src/harness/types.ts -- the result channels (verbatim)
export type ToolExecutionResult<TDetails extends JsonValue = JsonValue> = {
  readonly output?: ToolResultMessage["content"];   // for the model
  readonly structuredOutput?: JsonValue;            // for programs
  readonly isError?: boolean;
  readonly details?: TDetails;                      // for UIs
  readonly diagnostics?: readonly ToolDiagnostic[];
  readonly usage?: Usage;
  readonly control?: ToolControl;
};
```

The Aha is the three audiences. The model reads `output`. A program reads `structuredOutput`. A UI reads `details`. One execution serves all three without leaking one audience's format to another.

`structuredOutput` is validated against the tool's `structuredOutputSchema`.

```ts
// packages/durable/src/harness/tool.ts -- validate structured output (abridged)
function structuredOutputError(tool, result): string | undefined {
  const schema = tool.structuredOutputSchema;
  const value = result.structuredOutput;
  if (schema === undefined) {
    return value === undefined ? undefined : `Tool ${tool.name} returned structuredOutput but declares no structuredOutputSchema`;
  }
  if (value === undefined) return result.isError === true ? undefined : `Tool ${tool.name} returned no structuredOutput`;
  let validator = structuredValidators.get(schema);
  if (validator === undefined) { validator = Compile(schema); structuredValidators.set(schema, validator); }
  if (validator.Check(value)) return undefined;
  const [first] = validator.Errors(value);
  return `Tool ${tool.name} returned structuredOutput that does not match its schema: ${...}`;
}
```

The Aha is the recovery from a broken contract. A nested call whose `structuredOutput` breaks the contract gets an error result without it. A model-issued call only loses it, and the break is reported to the host. The model never sees structured output, so its call stands.

### Tool Control

A tool result can steer the run.

```ts
// packages/durable/src/harness/types.ts -- post-tool control (verbatim)
export type ToolControl = {
  readonly addTools?: readonly string[];
  readonly terminate?: boolean;
  readonly handoff?: string;
};
```

`addTools` installs tools mid-run. `terminate` ends the run without another model request when every result of the round sets it to `true`. `handoff` names a tool to take over.

## Settling the Result

`settle` commits the tool's terminal state. A model-issued call appends a `pi.tool-result` entry. A nested call writes its result to the caller's `NestedCallDoc`.

```ts
// packages/durable/src/harness/tool.ts -- settle (abridged)
async function settle(runtime, input, call, ending, build, context, durationMs?) {
  const nestedIds = (await runtime.ownedTasks(context))
    .filter((record) => record.kind === ToolTask.definition.name && record.input.kind === "nested")
    .map((record) => record.id);
  const madeNested = (await runtime.memo(MADE_NESTED_CALLS, context)) === true;
  await Promise.all(nestedIds.map((id) => runtime.abortOwned(id, context)));   // abort first, all at once
  await runtime.commit(async (tx) => {
    const live = await tx.doc(LiveDoc, conversationId);
    const slot = toolSlot(live, runtime.taskId);
    const result = build(slot);
    if (input.kind === "nested") {
      // write result to NestedCallDoc, record usage, finish the slot
      settled = { kind: "nested" };
    } else {
      // append a pi.tool-result entry, finish the slot
      settled = { kind: "model", entryId: entry.id, ...control };
    }
    if (madeNested) removeNestedSlots(live, runtime.taskId);
    if (ending.status === "aborted") return { status: "terminal", outcome: { status: "aborted", result: settled } };
    if (ending.status === "failed") return { status: "terminal", outcome: { status: "failed", error, result: settled } };
    return { status: "terminal", outcome: { status: "completed", result: settled } };
  }, context);
}
```

The Aha is the abort-first rule. A call that settles aborts every nested call it left running, and marks them all before waiting for any. One may run until a sibling is cancelled, so the marks all land first. This is how a parent's settlement pulls its whole subtree down cleanly.

`appendToolResult` writes the result entry and renders diagnostics into the model-visible content.

```ts
// packages/durable/src/harness/tool.ts -- append the result entry (abridged)
export async function appendToolResult(tx, conversationId, call, result, meta) {
  const diagnostics = [...(result.diagnostics ?? [])];
  const content: Content = [...(result.output ?? [])];
  if (diagnostics.length > 0) content.push({ type: "text", text: renderDiagnostics(diagnostics) });
  const message = { role: "toolResult", toolCallId: call.id, toolName: call.name, content, ... };
  if (result.usage !== undefined) await recordUsage(tx, conversationId, "tools", call.name, result.usage);
  return tx.appendEntry(ToolResultEntry, conversationId, { model: [message], data: { diagnostics } });
}
```

The Aha is the two stores. The model sees the rendered diagnostics in `content`. Programs see the structured list in `data.diagnostics`. The stored message is exactly what the model sees.

## The Registry

Tools come from extensions. An extension bundles tools, prompt sections, hooks, wraps, and task definitions. The registry holds installed extensions and publishes an immutable snapshot on each change.

```ts
// packages/durable/src/harness/registry.ts -- install (abridged)
install(extension: Extension<Tool>): void {
  validateExtension(extension);
  const current = this.#current.installed();
  const index = current.findIndex((installed) => installed.name === extension.name);
  const next = index < 0 ? [...current, extension] : current.map((installed, at) => (at === index ? extension : installed));
  this.#publish(next);
}
```

`#publish` builds and validates the next state, then swaps it in synchronously.

```ts
// packages/durable/src/harness/registry.ts -- publish (abridged)
#publish(extensions): void {
  this.#current = new RegistryState(extensions);   // throws on a task name collision
  for (const listener of [...this.#listeners]) listener();
}
```

`validateExtension` rejects two tools with the same name in one extension, and two sections with the same key.

```ts
// packages/durable/src/harness/registry.ts -- validate (abridged)
function validateExtension(extension: Extension): void {
  const tools = new Set<string>();
  for (const tool of extension.tools ?? []) {
    if (tools.has(tool.name)) throw new Error(`Extension ${extension.name} has two tools named ${tool.name}`);
    tools.add(tool.name);
  }
  const sections = new Set<string>();
  for (const { key } of extension.sections ?? []) {
    if (!SECTION_KEY.test(key)) throw new TypeError(`Section key ${JSON.stringify(key)} must match ${SECTION_KEY}`);
    if (key === INSTRUCTIONS_KEY) throw new Error(`Section key ${key} is reserved for the agent's instructions`);
    if (sections.has(key)) throw new Error(`Extension ${extension.name} has two sections with key ${key}`);
    sections.add(key);
  }
}
```

The section key must match `^[a-z][a-z0-9_-]*$`. The `instructions` key is reserved for the agent's instructions. This is a whitelist that keeps prompt section keys safe and collision-free.

The Aha is that a scheduler holds a snapshot, so a mid-run registry change does not corrupt a running task. A running generation uses the tools that existed when it started. Installing a tool does not retroactively change an in-flight turn. The snapshot is a point-in-time view.

## Settings

Settings are layered and resolved to a single value. The full resolution is shown in [06-durable-harness.md](./06-durable-harness.md). The two fields that matter most here are `toolExecution` and `progress`.

`toolExecution` is `"parallel"` or `"sequential"`. It controls whether the tools of one round run at once or one after another in call order. `progress` throttles how often running output is committed.

The Aha is that `toolExecution` is a per-tool override too. One sequential tool makes its whole round sequential, because a round is a single model request.

## Environment

The harness builds an environment for each conversation. `env` is a function that takes a target and returns an `ExecutionEnv`.

```ts
// packages/durable/src/harness/types.ts -- the env target (verbatim)
export type EnvTarget = {
  readonly conversationId: ConversationId;
  readonly cwd?: string;
  readonly read: DocumentReader;
};
```

```ts
// packages/durable/src/harness/harness.ts -- build the environment (abridged)
async buildEnv(id, context): Promise<ExecutionEnv | undefined> {
  const build = this.#options.env;
  if (build === undefined) return undefined;
  const cwd = (await this.snapshot(AgentDoc, id, context))?.cwd;
  return build({ conversationId: id, ...(cwd === undefined ? {} : { cwd }), read: this }, context);
}
```

The environment carries the working directory, the document reader, and any host-specific context a tool needs. It is built at each use, never cached, and never called on the Session line.

An `ExecutionEnv` is a filesystem plus a shell.

```ts
// packages/durable/src/env/index.ts -- the environment (verbatim)
export interface ExecutionEnv extends FileSystem, Shell {}
```

The filesystem returns `Result` values instead of throwing. A missing file is a `{ ok: false, error }` result, not an exception.

```ts
// packages/durable/src/env/index.ts -- a result (verbatim)
export type Result<TValue, TError> = { ok: true; value: TValue } | { ok: false; error: TError };
```

The Aha is the seam. The harness defines what a tool needs. The host decides how to provide it. A tool asks for an environment; it does not reach into the host. This is how codemode gets a working directory without durable knowing how codemode works.

## Prompt Sections

An extension can add a prompt section. A section injects text into the system prompt.

```ts
// packages/durable/src/harness/types.ts -- a prompt section (verbatim)
export type PromptSection<Tool extends ToolRegistration = ToolRegistration> = {
  readonly key: string;
  render(input: PromptInput<Tool>, context: Context): string | undefined | Promise<string | undefined>;
  readonly tag?: boolean;
};
```

Sections run at request build time. They read the current tools, the agent, the environment, and the committed documents, then render instructions. The default wraps the text as `<key>\n...\n</key>`.

The Aha is that sections are rendered fresh for every request, so the model's instructions stay in sync with the tools actually installed. A section cannot drift from the registry, because it reads the registry at render time.

## Where to Go Next

Tools run in the harness. The codemode tool runs user code in a sandbox. The next document explains that sandbox. Read [10-codemode.md](./10-codemode.md). For how generation uses tools, read [08-durable-generation.md](./08-durable-generation.md).
