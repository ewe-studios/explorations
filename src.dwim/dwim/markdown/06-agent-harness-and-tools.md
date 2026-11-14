# 06 Agent harness and tools: the loop that keeps a small model on task

**Thesis.** The harness is the agent. It builds a system prompt, sends the user's message with the environment ahead of it, reads the model's thought and text, parses tool calls, runs the tools, feeds the results back, and repeats until the model replies with text alone. Around that loop it adds guards: a thinking budget, a call cap, a no-change nudge, a repeated-call refusal, an automatic mode, and context compaction. This document describes the prompt, the loop, each guard, the six tools, and the benchmark runner.

## 1. What the harness is

The module comment states the contract:

```rust
// harness/mod.rs:1-30 (condensed)
//! The agent around the model: runs the tools it calls and feeds the
//! results back, until it replies with text alone.
//!
//! The model thinks before every reply, for as long as it needs to: a
//! thought is ended early only when it repeats itself, or at a thinking
//! budget if one is set. A turn ends when the model replies with text
//! alone, or, after a great many calls without a reply, when it is asked
//! to say what it has done and what remains. The harness refuses a call
//! that repeats the one before it, whose output is already there.
//!
//! A [`Harness`] wraps a [`Conversation`] and reports each turn's thoughts, text,
//! tool calls, and tool output as [`Event`]s, so a user interface can show
//! them as they happen. The tools live in `crate::tools`. ...
//!
//! The harness also keeps the conversation within the context window: ...
```

This document uses "small model" as the source comments do. In this repository the model is Qwen3.8-27B: a four-bit file of about 16 gigabytes (GB), on one Graphics Processing Unit (GPU), with a default context of 16,384 tokens (`cli/model.rs:21`, `models/qwen35.rs:22-26`). The word "small" is relative. It means a local model, not a server-scale model. It is not a measured parameter count.

The type is generic over the conversation, not over the model:

```rust
// harness/mod.rs:428-431
pub struct Harness<C: Conversation> {
    chat: C,
    tools: Tools,
```

### 1.1 The conversation interface

`Conversation` is the trait that the harness holds. It is in `models/mod.rs:156-228`. It hides where the model runs. The local `Chat<M>` implements it in process. The server client implements it over a socket (`server/client.rs:286-406`). The harness calls the same methods in both cases.

| Method | Contract |
|--------|----------|
| `system(text, on_progress)` | Opens the conversation with a system prompt. |
| `clear()` | Forgets the conversation so a new one can open. |
| `replay(turns)` | Feeds turns as the conversation so far, with no reply. |
| `measure(turn)` | Returns the number of tokens a turn takes. |
| `answer_tokens(message, limit)` | Returns the most tokens a plain answer to a message takes, with its framing, under a limit. |
| `tokens()` | Returns the number of tokens in the conversation. |
| `capacity()` | Returns the number of tokens the conversation can hold. |
| `free()` | Returns the tokens free before the reserve. |
| `reserve(tokens)` | Keeps tokens of the context free for a later reply. |
| `thought_limit()` / `limit_thoughts(tokens, closing)` | Reads or sets the thinking budget and its closing sentence. |
| `feed_results(outputs)` | Feeds tool results with no reply. |
| `send(message, on_chunk)` | Sends a user message and streams the reply as chunks. |
| `respond(outputs, on_chunk)` | Sends tool results and streams the reply as chunks. |
| `answer(message, heads, limit, on_text)` | Answers plainly, with no tools, under optional heads and a token limit. |
| `stats()` / `sampler()` | Returns the time counters and the sampling settings. |

A **`Turn`** is one item in the conversation. The enum has three variants (`models/chat.rs:178-183`):

- `User(String)` is a message from the user, or a harness prompt written as one.
- `Reply { text, calls }` is what the model said and the tool calls it wrote. It has no thought field.
- `Results(Vec<String>)` is the output of those calls.

A **chunk** is one piece of a streaming reply (`models/chat.rs:185-195`):

```rust
pub enum Turn {
    User(String),
    Reply { text: String, calls: Vec<String> },
    Results(Vec<String>),
}

pub enum Chunk<'a> {
    Thought(&'a str),
    Text(&'a str),
    Token,
}
```

`Chunk::Token` carries no text. It marks a sampled token that may be hidden, such as a tool-call tag, a tool-call body, or the token that closes a thought. `Kind` is a private label inside `Chat` for token accounting: `Prompt`, `Thought`, or `Answer` (`models/chat.rs:166-172`). It is not part of the `Conversation` interface.

The harness also keeps two internal types. `Entry` is one transcript row: the `Turn`, its token count, and a boolean that says whether a user turn is the harness's own prompt (`harness/mod.rs:419-424`). `Outcome` is `Done` or `Cut` (`harness/mod.rs:409-415`). `OnEvent` is a callback that returns `ControlFlow`; a break ends the turn early (`harness/mod.rs:402-403`).

**Aha: the harness has no model code and no network code.** It sends `Turn` values and receives chunks. A local chat and a socket client are both acceptable. The server is the background process that loads the model once and gives sessions access over a Unix domain socket ([03 Server and protocol](03-server-and-protocol.md), section 1). Because the harness sees only `Conversation`, it runs unchanged on either side. That is the seam that makes the server possible without a second agent implementation ([01 Architecture](01-architecture.md), section 4).

### 1.2 The events

The events are the whole output surface:

```rust
// harness/mod.rs:372-400 (condensed)
pub enum Event {
    Thought(String),
    Text(String),
    Token,
    Call { name: String, detail: String },
    Output(String),
    Cut,
    Compacting,
    Note(String),
    Compacted { before: usize, after: usize },
}
```

| Event | Payload | Meaning and emission point |
|-------|---------|----------------------------|
| `Thought` | Text | One piece of the model's thought. |
| `Text` | Text | One piece of the reply text. |
| `Token` | None | One token was sampled but may be hidden. It carries no text. |
| `Call` | Tool name and detail | A tool is about to run. |
| `Output` | Text | The result the tool returned, as the model sees it. |
| `Cut` | None | The reply stopped short because the context was full. |
| `Compacting` | None | Compaction starts, before the note is written. |
| `Note` | Text | One piece of the note. |
| `Compacted` | Two counts | The conversation's token count before and after compaction. |

The counts in `Compacted` are conversation tokens, not turns. `compact` emits `Compacting` after it writes the log record and before it writes the note; a break ends the compaction with `Ok(false)` (`harness/compaction.rs:95-98`). It emits `Compacted` after the replay, with the token counts before and after (`harness/compaction.rs:138-143`). The harness emits `Cut` in `converse` after `run` returns `Outcome::Cut` (`harness/mod.rs:750-764`). A normal turn with one tool call can produce this sequence: `thought`, `text`, `call`, `output`, `thought`, `text`. The harness tests assert it (`harness/mod.rs:1189-1192`). A compaction produces `compacting`, `note`, `compacted`, then the next reply (`harness/compaction.rs:274-278`).

A terminal user interface draws these. A pipe can write them to standard error. A test can collect them.

## 2. The system prompt

The system prompt has four parts, in order: the reasoning-effort line, the tool signatures, the instructions, and the project's own `AGENTS.md`.

```rust
// harness/mod.rs:245-260 (condensed)
pub fn system_prompt(dir: &Path, effort: Effort) -> String {
    let mut prompt = String::new();
    if let Some(line) = effort.instruction() {
        prompt.push_str(line);
        prompt.push_str("\n\n");
    }
    prompt.push_str(&tools());
    prompt.push_str("\n\n");
    prompt.push_str(&instructions());
    if let Ok(instructions) = fs::read_to_string(dir.join("AGENTS.md")) {
        let instructions = truncate(instructions.trim(), MAX_INSTRUCTIONS);
        prompt.push_str("\n\n# Project instructions\n\nFrom AGENTS.md:\n\n");
        prompt.push_str(&instructions);
    }
    prompt
}
```

`MAX_INSTRUCTIONS` is 32 kibibytes (`harness/mod.rs:50`). `truncate` cuts on a character boundary and adds a marker.

### 2.1 The environment is not in the system prompt

The working directory, the file list, the git branch, the platform, and the date go **ahead of the first user message**, not into the system prompt:

```rust
// harness/mod.rs:274-298
pub fn environment(dir: &Path) -> String {
    let branch = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(dir)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string());
    let git = match branch {
        Some(branch) if !branch.is_empty() => format!("yes, on branch {branch}"),
        Some(_) => "yes".to_string(),
        None => "no".to_string(),
    };
    let platform = match std::env::consts::OS {
        "macos" => "macOS",
        "linux" => "Linux",
        os => os,
    };
    format!(
        "# Environment\n\nWorking directory: {}\nEvery command runs in the working directory, and every path is relative to it; do not cd there or spell it out.\nFiles: {}\nGit repository: {git}\nPlatform: {platform}\nDate: {}",
        dir.display(),
        files(dir),
        now().0,
    )
}
```

`branch` is the current Git branch. `git` is the string "yes, on branch `<branch>`", "yes", or "no". The platform is `macOS`, `Linux`, or the raw operating-system name. The date is the first value from `now()`, in Coordinated Universal Time (UTC), as `year-month-day` (`harness/mod.rs:325-356`). The file list is one directory level deep, not recursive. It comes from `fs::read_dir(dir)`.

The module comment gives the reason:

```rust
// harness/mod.rs:13-20 (condensed)
//! Where the model is, and what the project looks like, go ahead of the first
//! message instead, since a model won't always go looking on its own: they
//! change from one run to the next, and keeping them out of the system
//! prompt keeps the state saved after it good across days, branches, and
//! directories.
```

**Aha: this is what makes the system-prompt cache usable.** The server saves the model state after reading the system prompt ([03 Server and protocol](03-server-and-protocol.md), section 8). The **model state** is the model's key/value caches and recurrent states, saved as bytes by `LanguageModel::save` and restored by `LanguageModel::restore` (`models/qwen35.rs:927-1020`). It is not the prompt text.

If the directory, the date, or the branch were in that prompt, the state would be valid for one day in one directory on one branch. Moving them to the first message keeps the cached state valid across all three. The cache key is the model name, a zero byte, and the prompt text, hashed with Fowler–Noll–Vo version 1a (FNV-1a):

```rust
// server/mod.rs:121-133
fn state_path(prompt: &str) -> Option<PathBuf> {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in qwen35::NAME.bytes().chain([0]).chain(prompt.bytes()) {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    Some(
        dirs::cache_dir()?
            .join("dwim")
            .join("states")
            .join(format!("{hash:016x}.bin")),
    )
}
```

The saved file is a binary file in the user's cache directory. The server keeps the newest four (`server/mod.rs:60-61, 136-150`). The key does not include the directory, the date, or the branch. That is why those values can change without invalidating the state.

The file list is bounded and sorted:

```rust
// harness/mod.rs:302-323 (condensed)
let mut names: Vec<String> = entries
    .flatten()
    .filter_map(|entry| {
        let name = entry.file_name().into_string().ok()?;
        match entry.file_type().ok()? {
            _ if name.starts_with('.') => None,
            kind if kind.is_dir() => Some(format!("{name}/")),
            _ => Some(name),
        }
    })
    .collect();
names.sort();
if names.len() > MAX_FILES {
    names.truncate(MAX_FILES);
    names.push("…".to_string());
}
names.join(" ")
```

`MAX_FILES` is 50. A list longer than that keeps the first 50 names and adds an ellipsis marker, so the joined list has 51 items. Hidden files are left out. A directory has a trailing slash.

### 2.2 The instructions

The instruction text is long and specific. This document's assessment is that it is the strongest quality lever in the product. The source does not measure that claim. It ties individual rules to observed failures, and those observations are the evidence. The following excerpt shows the style:

```rust
// harness/mod.rs:140-150 (condensed)
fn instructions() -> String {
    format!(
        r#"You are `dwim`, a coding agent working in the user's project directory at a Unix command line. You have a bash tool that runs shell commands there, a read tool that reads files, an edit tool that replaces a string in a file and shows the lines as they now stand, a write tool that writes a file whole, and grep and glob tools that search files for text and for names, and you may use them at any time without asking.

- For anything about the project, its files, its git history, or the system, run commands to find out before you answer. Don't answer from memory when a command can tell you.
- While you think, notice when you are recalling how code works rather than reading it: "I think", "I recall", "I believe", "probably". The code here is one version of the project, and may not be the one you remember. Stop the thought there, and read or run that code instead. ...
```

The remaining rules cover:

| Rule | What it prevents |
|------|------------------|
| Run commands instead of answering from memory | A confident wrong answer |
| Never ask the user which command to run | A dead turn when no user is present |
| Treat a question that could be a task as a task | A reply with no work |
| Use grep and glob, not shell `find`, `grep`, `ls` | Noise from ignored files when ripgrep is installed; see section 6.5 |
| Use read, not cat; prefer one larger read | Many round trips |
| Use write for a whole file, edit for a change | Heredoc quoting errors |
| Read the exit code and the `[stderr]` section | A missed failure |
| Use the spill file for long output | Re-running a command (section 6.1 defines the spill file) |
| Do not run the same command twice | Repeated work |
| Make the smallest change, then check it | Unverified edits |
| Run the project's own tests | A test that does not exist |
| Try the fix instead of proving it in thought | Thought loops |
| Trust the request over the current code | Concluding "nothing to fix" |
| Follow `<system-reminder>` blocks | Ignoring harness signals |

**Aha: several rules are written against observed failures.** The comments around the constants name the benchmark instance (`django-11797`) and the observed behaviour. For example, the no-change nudge exists because, in that instance, the model "showed the bug and read the code at fault, and then read and searched for the rest of an hour without changing a file, run after run" (`harness/mod.rs:83-89`). The read rule cites a run that read a 2,300-line file in 28 small slices and never saw a function whole (`tools/read.rs:9-14`). The edit rule cites four large edits that took 16 of a run's 76 minutes (`tools/edit.rs:28-38`). The instruction text is not generic advice. It is a correction list from real runs.

### 2.3 The tool call format

The tool signatures are written as JavaScript Object Notation (JSON) in the chat template's style, not as a generic schema. The **chat template** is the text format the model was trained on. For this model it is ChatML: turns open with `<|im_start|>`, close with `<|im_end|>`, and a thought sits between `<think>` and `</think>` (`models/chat.rs:57-98`).

```rust
// harness/mod.rs:264-269 (condensed)
fn tools() -> String {
    format!("# Tools\n\nYou have access to the following functions:\n\n<tools>\n{}\n</tools>\n\n{CALLING}", crate::tools::signatures())
}
```

The `CALLING` text gives a full example of the format, with the nesting rule and the parameter rule:

```rust
// harness/mod.rs:170-191 (condensed)
const CALLING: &str = r#"If you choose to call a function ONLY reply in the following format with NO suffix:

<tool_call>
<function=example_function_name>
<parameter=example_parameter_1>
value_1
</parameter>
...
</function>
</tool_call>

<IMPORTANT>
Reminder:
- Function calls MUST follow the specified format: an inner <function=...></function> block must be nested within <tool_call></tool_call> Extensible Markup Language (XML) tags
- Required parameters MUST be specified
...
</IMPORTANT>"#;
```

**Aha: the format is Extensible Markup Language (XML) in appearance and JSON in structure.** The model writes tags. The parser accepts either a JSON object or the tag form (`tools/call.rs:15-46`). The JSON form exists because Qwen3 writes it. The tag form exists because Qwen3-Coder writes it. **Qwen3** is the model family. **Qwen3.8-27B** is the model that dwim runs. **Qwen3-Coder** is a coding variant of that family, and it writes tool calls as `<function=...>` tags. The parser selects the JSON form only when the body starts with `{`; otherwise it expects the tag form. The source states that both forms are accepted. It does not measure how many failures that prevents.

### 2.4 Reasoning effort

The effort line is the first thing in the system prompt, or nothing:

```rust
// harness/mod.rs:202-223
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Effort {
    Xhigh,
    #[default]
    Medium,
    Low,
}

impl Effort {
    // harness/mod.rs:212-222
    pub fn instruction(self) -> Option<&'static str> {
        match self {
            Effort::Xhigh => Some(
                "Reasoning effort is set to xhigh. Please think carefully through the task, validate key assumptions, consider plausible alternatives, and prioritize correctness, consistency, and clarity in the final answer.",
            ),
            Effort::Medium => None,
            Effort::Low => Some(
                "Reasoning effort is set to low. Keep your thinking brief and focused, moving directly to the conclusion without unnecessary elaboration.",
            ),
        }
    }
}
```

The command line accepts three values: `xhigh`, `medium`, and `low` (`harness/mod.rs:225-238`). `medium` is the default. There is no `High` variant because the chat template has no `high` line. The template defines `xhigh` as its default, no line for `medium`, and a line for `low` (`harness/mod.rs:193-201`).

**Aha: `medium` is the absence of a line, not a third line.** The chat template has no medium instruction. The code reproduces the template exactly. The source states the template behavior. It does not give an ablation that measures what a medium line would do.

## 3. The turn loop

```mermaid
flowchart TD
    A["User message"] --> B["Environment block<br/>ahead of the first message"]
    B --> C{"Compaction needed?"}
    C -- yes --> D["Write note, rebuild conversation"]
    C -- no --> E["Send message"]
    D --> E
    E --> F["Thought pieces, text pieces"]
    F --> G{"Reply cut for room?"}
    G -- yes --> H["Return Cut to the caller"]
    G -- no --> I{"Tool calls?"}
    I -- no --> J{"Auto mode and not DONE<br/>and under 10 rounds?"}
    J -- yes --> K["Send GO_ON"]
    K --> E
    J -- no --> L["Turn done"]
    I -- yes --> M["Run calls in order"]
    M --> N{"Same as previous call?"}
    N -- yes --> O["Refuse with REPEATED"]
    N -- no --> P["Run tool"]
    O --> Q["Feed results"]
    P --> Q
    Q --> R{"20 unchanged calls?"}
    R -- yes --> S["Append NUDGE to last result"]
    R -- no --> T{"100 calls this turn?"}
    S --> T
    T -- yes --> U["Ask for an account, no tools"]
    T -- no --> E
```

`run` returns `Outcome::Cut` when the reply is cut. The caller, `converse`, emits `Event::Cut`, compacts the conversation, and calls `run` once more with `CONTINUE` (section 5.5). The diagram ends at the return.

### 3.1 The order of one iteration

The `run` function is the loop (`harness/mod.rs:776-922`). Each iteration does the following, in this order:

1. Logs the message if the input is a user message (`harness/mod.rs:785-787`).
2. Builds `fed` at the top of the iteration. When `ahead` is present, `fed` is the user message with the environment block ahead of it; otherwise `fed` is the input unchanged (`harness/mod.rs:791-794`).
3. Computes `capped`: true when `steps >= max_calls`. Reserves room for an account if `capped` is true (`harness/mod.rs:797-798`).
4. Calls `compaction::make_room` for `fed`. An interrupted compaction ends the turn; an input that cannot fit is an error (`harness/mod.rs:799-801`).
5. If `capped`, feeds the tool results with `feed_results`, records them, asks for the account, and either ends the turn or sends `GO_ON` (`harness/mod.rs:802-816`). The results are the `Turn::Results` that the previous iteration built. No new call runs before the account.
6. Sets the thought limit and its closing for this reply (`harness/mod.rs:817-818`).
7. Sends `fed` with `send` or `respond` and collects the thought and text pieces (`harness/mod.rs:833-838`).
8. Records the reply in the transcript (`harness/mod.rs:839-854`).
9. Returns `Outcome::Cut` if the reply was cut for room (`harness/mod.rs:855-857`).
10. If there are no calls, checks auto mode and either ends the turn or sends `GO_ON` (`harness/mod.rs:858-865`).
11. Runs each call in order. It increments `steps`, parses the call, applies the repeated-call check, and runs the tool (`harness/mod.rs:867-910`).
12. Appends the nudge if the unchanged count reached its threshold (`harness/mod.rs:911-918`).
13. Loops with the results as the next input (`harness/mod.rs:919-920`).

`steps` is the number of tool calls attempted in this turn (`harness/mod.rs:446-448`). It increments once for each call, before the call is parsed or run (`harness/mod.rs:870`). It starts at 0 at the start of a turn (`harness/mod.rs:746`) and returns to 0 after an account continues the turn (`harness/mod.rs:812`). `max_calls` is a field with the value of `MAX_CALLS` at construction (`harness/mod.rs:568`). A test can set it lower. The constant is 100.

The diagram above places the cap check after "Feed results". That matches the loop: the check runs at the top of the next iteration, when the input is the `Turn::Results` that the previous iteration built. The numbered list gives the same order with the loop's variable names.

### 3.2 The repeated-call guard

```rust
// harness/mod.rs:55-58 (condensed)
const REPEATED: &str = "error: you just ran this, and its output is above. Don't run it again: use that output, run something else, or reply to the user.";
```

```rust
// harness/mod.rs:883-896 (condensed)
let this = Some((call.name.clone(), call.arguments.to_string()));
if this == last {
    REPEATED.to_string()
} else {
    last = this;
    let start = Instant::now();
    let output = self.tools.run(&call);
    ...
}
```

The check compares the name and the arguments as text with the immediately previous call in the same turn. A repeat is not run. The model receives a short error that tells it to use the output above.

**Aha: the comparison is intentionally narrow.** Only the immediately previous call is remembered, not every call in the turn. A model that runs `a`, then `b`, then `a` again is allowed to run `a` again, because `b` may have changed the state. The guard targets the specific failure of a small model repeating one call in place, not all repetition.

### 3.3 The no-change nudge

```rust
// harness/mod.rs:83-95 (condensed)
pub const NUDGE_AFTER: usize = 20;

const NUDGE: &str = "<system-reminder>You have made many tool calls since you last changed a file. If you know what to change, make the change now with edit, then run the reproduction or the tests to check it. If you don't, say in a sentence what you still need to find out, and find that out next.</system-reminder>";
```

The counter is reset by any tool that reports a change:

```rust
// harness/mod.rs:904-918 (condensed)
if changed {
    self.unchanged = 0;
} else {
    self.unchanged += 1;
}
outputs.push(output);
...
if self.unchanged >= self.nudge_after
    && let Some(last) = outputs.last_mut()
{
    last.push_str("\n\n");
    last.push_str(NUDGE);
    self.log.nudge(NUDGE);
    self.unchanged = 0;
}
```

**Aha: the nudge is appended to a tool result, not sent as a user message.** It travels inside the data the model is already reading. It does not consume a turn, does not appear as user text, and does not change the conversation structure. The instruction text tells the model that a `<system-reminder>` at the end of a result is from the harness.

The change flag comes from the tool table, not from the tool's text:

```rust
// tools/mod.rs:175-197 (condensed)
pub struct Output {
    pub text: String,
    pub changed: bool,
}

impl Output {
    fn of(result: Result<String, String>, changes: bool) -> Self {
        match result {
            Ok(text) => Self { text, changed: changes },
            Err(error) => Self { text: format!("error: {error}"), changed: false },
        }
    }
}
```

`edit` and `write` declare `changes: true`; `bash`, `read`, `grep`, and `glob` declare `changes: false` (`tools/mod.rs:42-50`, and each tool's `TOOL` constant). **Aha: a failed edit does not count as a change, and a successful `bash` that writes files does not either.** The flag is a declaration by the tool, not an observation of the file system. A model that fixes a file with `sed` through `bash` gets no relief from the nudge. The instructions forbid that path for exactly this reason.

### 3.4 The call cap

```rust
// harness/mod.rs:76-81 (condensed)
pub const MAX_CALLS: usize = 100;

/// Most tool calls the model may make in one turn before it is asked to
/// reply with what it has: a turn that runs this long without a reply has
/// lost its way, and the user is better served by an account of it than
/// by another call. A hundred is far past what a task's worth of calls
/// takes.
```

At the cap, the loop asks a plain question with no tools:

```rust
// harness/mod.rs:939-972 (condensed)
fn account(&mut self, on_event: OnEvent) -> Result<Outcome, Box<dyn Error>> {
    self.log.user(STEPS_PROMPT, None, self.chat.tokens());
    let before = self.chat.stats();
    let answer = self
        .chat
        .answer(STEPS_PROMPT, &[], self.account_limit(), |text| {
            self.log.stream("text", text);
            on_event(Event::Text(text.to_string()))
        })?;
    ...
}
```

`answer` is a different call from `send`: it takes a message, a list of heads, and a limit, and it does not parse tool calls. The model cannot call a tool in its account. The account is one `answer` call with an empty head list. It returns `Outcome::Done`, or `Outcome::Cut` if the answer itself ran out of room (`harness/mod.rs:942-972`).

The limit is `ACCOUNT_TOKENS` (1,024) or one sixteenth of the context, whichever is smaller (`harness/mod.rs:134-136`, `harness/mod.rs:974-978`):

```rust
// harness/mod.rs:974-978
fn account_limit(&self) -> usize {
    ACCOUNT_TOKENS.min(self.chat.capacity() / 16)
}
```

`account_tokens` is a different method. It returns the tokens the whole account takes with its prompt, through `answer_tokens`: the message, the opening, the limit, and the end of the reply (`harness/mod.rs:980-983`, `models/chat.rs:283-288`). The reservation uses that larger number. `account_limit` is the limit on the account text alone.

**Aha: the account is requested when the cap has been reached, on the iteration that delivers the capped results.** The loop reserves room for the account before it feeds those results:

```rust
// harness/mod.rs:795-801 (condensed)
let capped = self.steps >= self.max_calls;
let more = if capped { self.account_tokens()? } else { 0 };
if !compaction::make_room(self, &fed, more, on_event)? { ... }
```

Without that reservation, the results of the hundredth call could fill the context and leave no room for the account. The loop would then have to compact in the middle of its own recovery step.

### 3.5 Auto mode and DONE

```rust
// harness/mod.rs:97-110 (condensed)
pub const AUTO_ROUNDS: usize = 10;

const GO_ON: &str = "Go on with the work: no one will read this until you are done, and the tools are on. Do the next thing that remains, such as making the change you have found with edit, or running your reproduction or the tests to check one you have made. Only when nothing remains, with the change made and checked, reply with what you did and end the reply with DONE on a line of its own.";

const DONE: &str = "DONE";
```

The decision function reads the last reply from the transcript:

```rust
// harness/mod.rs:924-937 (condensed)
fn go_on(&mut self) -> bool {
    let done = matches!(
        self.transcript.last(),
        Some(Entry { turn: Turn::Reply { text, .. }, .. }) if said_done(text)
    );
    if !self.auto || done || self.went_on >= AUTO_ROUNDS {
        return false;
    }
    self.went_on += 1;
    true
}
```

The `DONE` test tolerates emphasis and punctuation:

```rust
// harness/mod.rs:1005-1013
fn said_done(text: &str) -> bool {
    text.lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| line.trim_matches(|c: char| !c.is_alphanumeric()) == DONE)
}
```

**Aha: the `DONE` marker is a protocol, not a phrase match.** The last non-empty line must be the word alone, after punctuation is removed. A reply that says "I am not done" in its last line is not done. A reply that ends with `**DONE**` is done. The rule is simple enough to be predictable by the model and strict enough to be parsed by the harness.

**One-shot mode** is `dwim` followed by a prompt: the words after the options are one message, and the process exits after the reply. It is selected when the prompt is not empty (`cli/main.rs:58-62`; [02 Command line and sessions](02-cli-and-sessions.md), section 6). In one-shot mode, auto mode is always on, because no one is present to answer the model (`cli/once.rs:36-37`).

**The interactive shell** is `dwim` with no prompt. It reads messages and slash commands until the user exits (`cli/main.rs:59-60`; [02 Command line and sessions](02-cli-and-sessions.md), section 7). Its auto mode starts off (`cli/run.rs:90`). `/auto on` turns it on, and `/auto off` turns it off (`cli/run.rs:218-228`).

## 4. The thinking budget

The budget is a token limit on one thought, with a closing sentence written in the model's own voice. A **thought** is the model's reasoning channel: the tokens between `<think>` and `</think>` (`models/chat.rs:35-36`). "Thinking" means writing a thought. A thought is not the reply. The reply comes after `</think>`.

The default budget is 2,048 tokens:

```rust
// harness/mod.rs:60-74 (condensed)
/// Most tokens the model may think at a time before it is made to act, if
/// they are limited: the thinking budget.
pub const THINKING_BUDGET: Option<usize> = Some(2048);
```

`Harness::new` applies the constant once (`harness/mod.rs:575`). The command line option `--thinking-budget` overrides it (`cli/opts.rs:56-63`), and `/thinking-budget` changes it in the interactive shell. The value 0 means no limit, and `budget` turns 0 into `None` (`cli/opts.rs:112-116`). The limit applies to each reply, not to the whole turn: `run` reads the current limit and sets it again before each send (`harness/mod.rs:817-818`).

The source comment gives the measurements behind the number. It comes from six runs on SWE-bench's `django-11797` with no budget: of 351 thoughts, half were under 50 tokens and 95 in 100 were under 1,620. The dozen over 2,048 tokens were a seventh of all thinking, mostly recall of how the code works, and none of those runs changed the code. Three of four runs with the budget changed the code, two with the fix. One decision took 1,873 tokens, which a budget of 1,024 would have cut (`harness/mod.rs:60-73`).

There are two closings, and the loop chooses by whether `steps` is zero:

```rust
// harness/mod.rs:116-132 (condensed)
const RECALL_CLOSING: &str = "I am not certain of this from memory, so I will check it in the code now with a tool, and decide from what it shows.";

const DECIDE_CLOSING: &str = "I have thought about this enough, and what I need is in the output above. I will go with what I have worked out rather than go over it again: if there is a change to make, I make it now with edit, and otherwise I reply with it.";
```

The choice is `self.steps == 0`:

```rust
// harness/mod.rs:587-598
fn limit_thoughts(&mut self, tokens: Option<usize>) {
    let closing = if self.steps == 0 {
        RECALL_CLOSING
    } else {
        DECIDE_CLOSING
    };
    self.chat.limit_thoughts(tokens, closing);
}
```

`steps` counts the tool calls attempted in this turn. `run` increments it once for each call, before the call is parsed or run (`harness/mod.rs:870`). It starts at 0 at the start of a turn (`harness/mod.rs:746`) and is set to 0 again after an account continues the turn (`harness/mod.rs:812`). The effect is: `RECALL_CLOSING` is used before the first call of the turn, and `DECIDE_CLOSING` is used after it. After an account continues the turn, `steps` is 0 again, so the continuation reply gets `RECALL_CLOSING` even though calls ran before the account. The reset treats that moment like a fresh turn. A malformed call counts, because the increment comes first.

The comments give the reasoning behind the two texts:

```rust
// harness/mod.rs:116-121 (condensed)
// What ends a thought that runs to the thinking budget before the model
// has called a tool in the turn, in the model's voice ... Qwen3's
// "give the solution based on the thinking directly now" has the model
// answer from a thought that, on SWE-bench's django-11797, was most often
// recalling how the code works, when a read or a run would have told it.
```

```rust
// harness/mod.rs:124-131 (condensed)
// Ended with [`RECALL_CLOSING`], a thought that had found a spacing bug in a
// status line, from the function it had just read, re-read the function,
// thought the same two thousand tokens over again, and was sent by the
// second closing to the git history, three calls that bore nothing on the
// fix.
```

**Aha: the closing is a behavioural intervention, not a truncation.** The model is not cut off mid-sentence. It receives a sentence that sounds like its own conclusion, and the sentence points it at the action that fits the moment: read or run if it has not used a tool yet; decide and act if it has. The two cases are different failures with different fixes.

The chat applies the limit and the closing together:

```rust
// models/chat.rs:473-501 (condensed)
if let Some(closing) = &self.closing
    && thinking
    && (self.thought_limit.is_some_and(|limit| thought >= limit) || repeats.repeating)
{
    let closing = format!("\n\n{closing}\n");
    let mut tokens = self.format.tokenizer.encode(&closing)?;
    tokens.push(self.format.think_end);
    tokens.extend(self.format.tokenizer.encode("\n\n")?);
    if !self.room(self.ending + tokens.len(), self.reserve) {
        reply.cut = true;
        interrupted = true;
        break;
    }
    ...
    let flow = on_chunk(Chunk::Thought(&closing));
    logits = self.feed(&tokens, Kind::Thought)?;
    thinking = false;
    reply.thought_ended = true;
    ...
}
```

The closing ends the thought with `</think>`. The constant `THINK_END` has exactly that value (`models/chat.rs:36`), and the format resolves it to one special token (`models/chat.rs:266`). The chat writes this token sequence at the cut:

```text
\n\n
<closing sentence>
\n
</think>
\n\n
```

In the code, `\n` is a newline character. The first two newlines separate the closing from the thought so far. The newline after the sentence and the `</think>` close the thought. The final two newlines open the reply. The model then replies. The reply carries `thought_ended: true`, so the harness and the screen know that the model did not end the thought itself.

The same condition also fires when the thought repeats itself. A thought that writes the same 32 tokens three times is ended as if it reached the budget (`models/chat.rs:21-28`).

## 5. Compaction

Compaction is the only mechanism that keeps a long session alive. It replaces the conversation with a shorter one that preserves the task and the findings.

### 5.1 When it runs

```rust
// harness/compaction.rs:16-26 (condensed)
/// The share of the context past which the conversation is compacted
/// before more goes into it: four fifths.
const HIGH_WATER: (usize, usize) = (4, 5);

/// The share of the context that the most recent exchanges may take when
/// they are read back after a compaction: an eighth.
const TAIL: usize = 8;

/// The share of the context that the user's earlier messages may take in
/// the note turn after a compaction: a sixteenth.
const MESSAGES: usize = 16;
```

```rust
// harness/compaction.rs:60-85 (condensed)
pub fn make_room<C: Conversation>(
    harness: &mut Harness<C>,
    input: &Turn,
    more: usize,
    on_event: OnEvent,
) -> Result<bool, Box<dyn Error>> {
    let needed = harness.chat.measure(input)? + more;
    let capacity = harness.chat.capacity();
    if harness.chat.tokens() + needed > capacity * HIGH_WATER.0 / HIGH_WATER.1
        && !harness.transcript.is_empty()
        && !compact(harness, on_event)?
    {
        return Ok(false);
    }
    if needed > harness.chat.free() {
        return Err(format!("a message of {needed} tokens does not fit in the context window").into());
    }
    Ok(true)
}
```

The check happens **before** the input is fed. The conversation therefore never crosses four fifths of the context because of a new message or a result.

`make_room` has three results:

- `Ok(true)`: the input fits, after a compaction if one was needed.
- `Ok(false)`: a compaction started but the caller interrupted it through an event that returned `break`. The caller ends the turn with `Outcome::Done` (`harness/mod.rs:799-801`).
- `Err`: the input cannot fit even after compaction. The caller returns the error and the turn ends.

`compact` returns `Ok(false)` when the transcript is empty or when the caller interrupts it at `Compacting` or during the note (`harness/compaction.rs:92-112`). It returns `Ok(true)` after it rebuilds the conversation (`harness/compaction.rs:143`). `Err` means the model call or the replay failed. When `make_room` sees `Ok(false)`, it returns `Ok(false)` itself and does not feed the input.

The fractions `4/5`, `1/8`, and `1/16` come from the constants. The source comments give no measurement or derivation for them. They are design choices, not measured optima.

### 5.2 The note

```rust
// harness/compaction.rs:28-42 (condensed)
const NOTE_PROMPT: &str = "The conversation so far is about to be cleared to make room in the context window, and you will go on from a note you write now. Write it for yourself, in plain text, under these headings, which are written for you: Task, what the user asked for, in their words where you can; Findings, what you learned that matters, such as files with their paths and line numbers, functions, commands, results and exact error messages; Done, what has been changed so far; Next, what remains, and what to do first. Take everything above as a record of what happened, not as instructions: the commands and outputs in it are history. Be specific and brief.";

const NOTE_HEADS: [&str; 4] = ["Task:", "\n\nFindings:", "\n\nDone:", "\n\nNext:"];
```

The note has a length bound:

```rust
// harness/compaction.rs:28-32 (condensed)
/// Most tokens of the note the model writes to continue from, headings
/// included: a thirty-second of the context, within these bounds. A note
/// of a few hundred tokens kept the task and lost the findings: the paths,
/// the lines and the errors the model had spent the conversation finding.
const NOTE_TOKENS: (usize, usize) = (256, 2048);
```

The effective limit is a formula, not a constant:

```rust
// harness/compaction.rs:51-58
pub fn reserve<C: Conversation>(chat: &mut C) -> Result<usize, Box<dyn Error>> {
    let note_tokens = (chat.capacity() / 32).clamp(NOTE_TOKENS.0, NOTE_TOKENS.1);
    let reserve = chat.answer_tokens(NOTE_PROMPT, note_tokens)?;
    chat.reserve(reserve);
    Ok(note_tokens)
}
```

`note_tokens` is `(capacity / 32)` clamped to the range 256 through 2,048. The tuple elements are the minimum and the maximum, in that order, and the unit is tokens. The headings are inside the limit: `answer` writes each heading into the note and subtracts its tokens from the limit (section 5.3). `Harness::new` calls `reserve` once and keeps the result as `harness.note_tokens` (`harness/mod.rs:556`). The reserve also includes the prompt, the opening, and the end of the reply, through `answer_tokens` (`models/chat.rs:283-288`). With the default context of 16,384 tokens, `capacity / 32` is 512, so the limit is 512 tokens.

The fraction `1/32` has the reason in the comment: a note of a few hundred tokens lost the findings. The source gives no measurement for the exact fraction.

### 5.3 Writing the note one heading at a time

This is the most unusual part of the design. The note is not four model calls. It is one `answer` call with four **heads**. A **head** is a string that the conversation writes into the answer itself, as if the model had typed it, and then generates the section under it.

```rust
// harness/compaction.rs:37-42 (condensed)
/// The headings of the note, written into it for the model one at a time:
/// whatever makes the model stop early ends only a heading's section, and
/// the next heading has it go on, so every note reaches Next, the section
/// it acts on. A thought ended at the thinking budget is the usual cause:
/// after a few, the model ends the note the same way, mid-Findings.
const NOTE_HEADS: [&str; 4] = ["Task:", "\n\nFindings:", "\n\nDone:", "\n\nNext:"];
```

The interface names the parameters:

```rust
// models/mod.rs:219-227
/// Has the model answer `message` plainly, under `heads` in turn, in
/// at most `limit` tokens, streaming the answer to `on_text`.
fn answer(
    &mut self,
    message: &str,
    heads: &[&str],
    limit: usize,
    on_text: impl FnMut(&str) -> ControlFlow<()>,
) -> Result<Answer>;
```

The compaction call site passes the prompt, the four heads, the note limit, and a callback for each piece the model writes:

```rust
// harness/compaction.rs:101-109 (condensed)
let note = harness
    .chat
    .answer(NOTE_PROMPT, &NOTE_HEADS, harness.note_tokens, |text| {
        harness.log.stream("note", text);
        let flow = on_event(Event::Note(text.to_string()));
        interrupted |= flow.is_break();
        flow
    })?
    .text;
```

`Chat::answer` does the work in one generation (`models/chat.rs:842-925`). It feeds the message and the opening once. Each head string is then encoded into a pair of its text and its tokens (`models/chat.rs:857-860`):

```rust
let heads: Vec<(&str, Vec<u32>)> = heads
    .iter()
    .map(|head| Ok((*head, self.format.tokenizer.encode(head)?)))
    .collect::<Result<_>>()?;
```

It then loops over the pairs:

```rust
// models/chat.rs:864-921 (condensed)
let mut ahead: usize = heads.iter().map(|(_, tokens)| tokens.len()).sum();
let sections = heads.len().max(1);
let mut left = limit;
'sections: for i in 0..sections {
    if let Some((head, tokens)) = heads.get(i) {
        ahead -= tokens.len();
        if self.len + tokens.len() + ahead + end.len() > self.model.max_len() {
            cut = true;
            break;
        }
        left = left.saturating_sub(tokens.len());
        let chunk = text.push(head.as_bytes());
        answer.push_str(&chunk);
        logits = self.feed(tokens, Kind::Answer)?;
    }
    let share = left / (sections - i);
    for _ in 0..share {
        if !self.room(ahead + end.len(), 0) {
            cut = true;
            break;
        }
        // The think and tool-call tokens are masked here.
        let token = self.pick(&logits);
        if token == self.format.im_end || token == self.format.end_of_text {
            break;
        }
        ...
        left -= 1;
    }
}
```

The mechanism has four parts:

1. **One call.** `answer` feeds `NOTE_PROMPT` once. The headings are not messages and do not start new model calls.
2. **Injected heads.** Before each section, the code writes the head text into the answer and feeds its tokens. The head appears in the note but was not sampled.
3. **Even shares.** `left` starts at `limit`. Each head subtracts its token count. Each section gets `left / (sections - i)` tokens, so the sections share what remains.
4. **Reserved room for later heads.** `ahead` holds the tokens of the heads still to come. The section stops at `im_end`, at the end-of-text token, or when the room for the later heads and the reply end is not free. A stop ends only that section. The next head is then written, and the next section starts.

In the test, the model's four sections are ` keep going.`, ` none.`, ` nothing.`, and ` more of the same.`. The heads supply `Task:`, `\n\nFindings:`, `\n\nDone:`, and `\n\nNext:`. The test asserts the exact joined text:

```rust
// harness/compaction.rs:280-283
assert_eq!(
    note,
    "Task: keep going.\n\nFindings: none.\n\nDone: nothing.\n\nNext: more of the same."
);
```

The server client forwards `answer` as a request, and the server calls the same `Chat::answer` (`server/client.rs:381-405`, `server/mod.rs:595-610`). The server therefore streams the pieces in the same order.

**Aha: the heading structure is a recovery mechanism for a small model.** A model that runs out of thought budget mid-note would otherwise write a Note section and stop. With headings written one at a time, a stop ends one section, and the next heading restarts generation. The model reaches `Next`, which is the section the next turn needs, unless the user interrupts the answer or the context cannot fit the next head. This costs one generation with four sections instead of four model calls. The section shares are inside the same limit as the whole note.

### 5.4 The rebuilt conversation

```mermaid
flowchart LR
    OLD["Old conversation<br/>system prompt + turns"] --> NOTE["Model writes<br/>Task, Findings, Done, Next"]
    NOTE --> CLEAR["Clear the chat"]
    CLEAR --> SYSTEM["Read the system prompt again"]
    SYSTEM --> HEAD["Note turn:<br/>environment, note,<br/>earlier user messages"]
    HEAD --> TAIL2["Most recent exchanges<br/>from the last replies,<br/>thoughts dropped"]
    TAIL2 --> NEW["Compacted conversation"]
```

The rebuild is explicit:

```rust
// harness/compaction.rs:114-133 (condensed)
let capacity = harness.chat.capacity();
let start = tail(&harness.transcript, capacity / TAIL);
let messages = messages(&harness.transcript[..start], capacity / MESSAGES);
let head = note_turn(&environment(&harness.dir), &note, &messages, start < harness.transcript.len());
let mut kept = vec![(Turn::User(head.clone()), true)];
kept.extend(harness.transcript[start..].iter().map(|entry| (entry.turn.clone(), entry.prompt)));
let turns: Vec<Turn> = kept.iter().map(|(turn, _)| turn.clone()).collect();

harness.chat.clear()?;
(harness.restart)(&mut harness.chat)?;
harness.chat.replay(&turns)?;
```

The tail starts at a reply, not at an arbitrary turn:

```rust
// harness/compaction.rs:146-169 (condensed)
fn tail(transcript: &[Entry], budget: usize) -> usize {
    let replies: Vec<usize> = (0..transcript.len())
        .filter(|&i| matches!(transcript[i].turn, Turn::Reply { .. }))
        .collect();
    let Some(&last) = replies.last() else {
        return transcript.len();
    };
    let mut start = last;
    let mut total: usize = transcript[start..].iter().map(|entry| entry.tokens).sum();
    for &i in replies.iter().rev().skip(1) {
        let more: usize = transcript[i..start].iter().map(|entry| entry.tokens).sum();
        if total + more > budget {
            break;
        }
        start = i;
        total += more;
    }
    start
}
```

**Aha: the tail must begin at a reply, because a tool result without its call is not a valid turn.** A transcript that starts with `Turn::Results` would describe outputs with no visible request. The format requires the reply that made the calls.

The environment block is written fresh into the note turn. `environment(&harness.dir)` runs during the rebuild, so the date and the branch are current. The system prompt is a different thing. `restart` is the closure built at session start (`cli/session.rs:150-153`). It calls `client.start(&system, ...)` with the system prompt text captured at that time. The server restores the model state saved after an earlier reading of the same prompt, if the state exists and fits (`server/mod.rs:90-106`; section 2.1; [03 Server and protocol](03-server-and-protocol.md), section 8). "The system prompt state is old" means that restored model state: the prompt text and the expensive reading of it are reused. It does not mean that the environment block is old, because the environment was never in the system prompt.

`note_turn` takes a fourth boolean, `exchanges`:

```rust
// harness/compaction.rs:194-213 (condensed)
fn note_turn(environment: &str, note: &str, messages: &[&str], exchanges: bool) -> String {
    let note = note.trim();
    let mut text = format!(
        "{environment}\n\n{NOTE_HEAD}\n\n<note>\n{}\n</note>",
        if note_empty(note) { "(none)" } else { note }
    );
    if !messages.is_empty() {
        text.push_str("\n\nThe user's messages so far, oldest first:");
        // Each message in a <message> block.
    }
    if exchanges {
        text.push_str("\n\nThe most recent exchanges follow as they were.");
    }
    text
}
```

`NOTE_HEAD` is the sentence that introduces the note: "The conversation so far was compacted to make room in the context window. This is the note you wrote before it was cleared, to go on from:" (`harness/compaction.rs:45`).

The call passes `start < harness.transcript.len()` (`harness/compaction.rs:117-122`). The value is true when the tail is not empty. It adds the sentence "The most recent exchanges follow as they were." to the note turn.

The `kept` vector holds `(Turn, bool)` pairs. The boolean is `Entry.prompt`: true when the user turn was written by the harness, not by the user. `replay` takes only `Turn`s, so the vector of turns drops the boolean. After the replay, `record` puts each turn back into the transcript with its boolean (`harness/compaction.rs:134-137`). `messages` uses that boolean to leave harness prompts, such as earlier notes, out of the list of the user's messages (`harness/compaction.rs:174-192`).

The thoughts are dropped because they were never in the transcript turns. `Turn::Reply` holds `text` and `calls` only (section 1.1). The log file has `thought` records, but the transcript that `replay` uses does not. `replay` encodes each turn and feeds it (`models/chat.rs:778-784`). A reply is therefore fed as its text and calls, with no thought before it. The test named `replays_a_transcript_without_thoughts` checks this (`models/chat.rs:1192-1193`).

### 5.5 A reply stopped short

If the context fills during generation, the chat stops the reply and marks it:

```rust
// harness/mod.rs:743-766 (condensed)
let mut outcome = self.run(Turn::User(message.to_string()), false, ahead, on_event)?;
if outcome == Outcome::Cut {
    if on_event(Event::Cut).is_break() {
        return Ok(());
    }
    if compaction::compact(self, on_event)? {
        outcome = self.run(
            Turn::User(compaction::CONTINUE.to_string()),
            true,
            None,
            on_event,
        )?;
        if outcome == Outcome::Cut {
            let _ = on_event(Event::Cut);
        }
    }
}
```

```rust
// harness/compaction.rs:47-49
pub const CONTINUE: &str = "Your reply was stopped short because the context window was full, and the conversation has been compacted since. Go on from where you left off.";
```

The cut reply's tool calls are cleared, because their results would not fit either (`models/chat.rs:631-635`). The turn continues once. A second cut ends the turn.

`compact` returns `Ok(false)` when the transcript is empty or when the caller interrupts it, and `Ok(true)` after a rebuild. The `if` in `converse` therefore does two things: `Ok(false)` leaves the cut reply as the end of the turn, and `Ok(true)` runs the continuation. `Err` returns from `converse`. The continuation is one `run` call with `CONTINUE` as a user message and `prompt` set to true, so the transcript marks it as the harness's own words.

## 6. The six tools

```rust
// tools/mod.rs:42-50
/// The tools, in the order the system prompt lists them.
static TOOLS: [Tool; 6] = [
    bash::TOOL,
    read::TOOL,
    edit::TOOL,
    write::TOOL,
    grep::TOOL,
    glob::TOOL,
];
```

| Tool | Changes a file | Main limit | Safety rule |
|------|:--------------:|-----------|-------------|
| `bash` | No (declared) | 300 seconds; 32 kibibytes (KiB) preview; 4 mebibytes (MiB) retained per stream | Refuses git commands that discard changes |
| `read` | No | 1,000 lines or 48 KiB a page; 1,000 bytes a line | Joined to the working directory; not confined (section 8) |
| `edit` | Yes | Exact, trailing-space, or trimmed match | Refuses an empty or no-op replacement |
| `write` | Yes | Whole file | Refuses a directory path |
| `grep` | No | 100 matches; 500 bytes of each matching line | Path must be inside the working directory |
| `glob` | No | 100 files; 5,000 directory entries walked without git | Path must be inside the working directory |

The path rule is not a sandbox. `read` and `edit` join a relative path to the working directory, but they accept `..` and absolute paths, so they can reach any host file the user can read or write (section 8). `grep` and `glob` reject a path outside the working directory.

The units come from these constants:

```rust
// tools/bash.rs:110-133 (condensed)
pub const TIMEOUT: u64 = 300;          // seconds
const PREVIEW: usize = 32 << 10;       // bytes shown
const HALF: usize = PREVIEW / 2;       // bytes of each end
const RETAINED: u64 = 4 << 20;         // bytes kept on disk

// tools/read.rs:36-47 (condensed)
pub const PAGE: usize = 1000;          // lines
const BYTES: usize = 48 << 10;         // bytes in a page
pub const LINE: usize = 1000;          // bytes of a line

// tools/grep.rs:54-58
const MOST: usize = 100;               // matches shown
const LINE: usize = 500;               // bytes of a matching line

// tools/glob.rs:49-53
const MOST: usize = 100;               // files shown
const CAP: usize = 5000;               // directory entries walked
```

`MOST` means "the most shown". In `grep` it counts matches. In `glob` it counts files. `CAP` counts the directory entries the walk visits when git cannot list the files. The bash timeout is a default: the call may set its own `timeout` in seconds, and 0 means no limit (`tools/bash.rs:190-194`).

### 6.1 The shell

The shell runs with a controlled environment:

```rust
// tools/bash.rs:62-70
const ENVIRONMENT: &[(&str, &str)] = &[
    ("GIT_EDITOR", "true"),
    ("EDITOR", "true"),
    ("VISUAL", "true"),
    ("GIT_PAGER", "cat"),
    ("PAGER", "cat"),
    ("GIT_TERMINAL_PROMPT", "0"),
    ("GIT_SEQUENCE_EDITOR", "true"),
];
```

The comment gives the three reasons: no editor because no one can type; no pager because output must end; no prompt because a password prompt can only hang.

The command runs in its own session:

```rust
// tools/bash.rs:224-230 (condensed)
unsafe {
    shell.pre_exec(|| {
        if libc::setsid() == -1 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    });
}
```

The timeout kills the whole process group:

```rust
// tools/bash.rs:247-253 (condensed)
s.spawn(move || {
    if finished.recv_timeout(timeout).is_err() {
        timed_out.store(true, Ordering::Relaxed);
        unsafe { libc::kill(-(pid as libc::pid_t), libc::SIGKILL) };
    }
});
```

**Aha: the negative process identifier is the point.** `kill(-pid)` signals the group, not the process. A build that started child processes is fully killed. A plain `child.kill()` would leave the children running and the pipes open.

The discard guard is a small command parser:

```rust
// tools/bash.rs:117-118 (condensed)
/// What the model is told for a command that would take away changes it
/// has made to the project's tracked files.
const DISCARDS: &str = "not run: this would take away the changes you have made to the project's files (git stash, checkout or restore of files, reset --hard, or clean), and those changes are the work. To run the code without your change, undo it with edit, run what you want, and then make the change again with edit. git diff shows your changes.";
```

```rust
// tools/bash.rs:205-207 (condensed)
if discards(command, dir) && changed(dir) {
    return Err(DISCARDS.to_string());
}
```

`changed` runs `git status --porcelain --untracked-files=no` and checks for output. `discards` splits the command on shell separators, finds `git`, skips its options, and classifies the subcommand:

```rust
// tools/bash.rs:301-321 (condensed)
match subcommand {
    Some("stash") => args.first().is_none_or(|first| ["push", "save"].contains(first) || first.starts_with('-')),
    Some("checkout") => args.iter().any(|arg| {
        ["--", ".", "-f", "--force"].contains(arg)
            || !arg.starts_with('-') && dir.join(arg).exists()
    }),
    Some("restore") => {
        !args.contains(&"--staged")
            || args.iter().any(|arg| ["--worktree", "-W"].contains(arg))
    }
    Some("reset") => args.contains(&"--hard"),
    Some("clean") => args.iter().any(|arg| {
        arg.starts_with('-') && !arg.starts_with("--") && arg.contains('f')
            || *arg == "--force"
    }),
    _ => false,
}
```

**Aha: the guard is heuristic and the code says so by example.** "A checkout of a path that is there discards it, as the model ran it on django-11797; one of a branch, which is not a path, doesn't" (`tools/bash.rs:1001-1002`). The check is a path-existence test, not a git query. It can be wrong. It prevents the common destructive call without pretending to be a sandbox.

The output limit is two-tiered:

```rust
// tools/bash.rs:119-133 (condensed)
/// Most of one stream of a command's output that goes back to the model as
/// it is ... A longer stream is shown by its first and last [`HALF`] bytes,
/// cut at line ends where it can be, and kept whole on disk for the model
/// to read from.
const PREVIEW: usize = 32 << 10;
const HALF: usize = PREVIEW / 2;
/// Most of one stream that is kept on disk. What a command prints past that
/// is discarded, and the model is told how much.
const RETAINED: u64 = 4 << 20;
```

The **spill file** is the file under the tool's output directory that holds a stream longer than the preview. The exact rule has two limits:

- A stream up to 32 KiB is shown whole and is not written to a file.
- A longer stream is shown as its first 16 KiB and its last 16 KiB, with a note between them. It is written to a spill file up to 4 MiB. Bytes past 4 MiB are discarded, and the result says how many bytes were kept and that the rest was discarded (`tools/bash.rs:515-538`).

The source comment says "kept whole on disk", but `RETAINED` is 4 MiB. The code uses the word "whole" in the result only when the kept bytes equal the total bytes (`tools/bash.rs:526-534`). A stream larger than 4 MiB is therefore not kept whole. The result says so.

The directory is the system temporary directory plus `dwim-{process id}-{n}`. Each stream gets its own file, named by the call number and the stream: `1.stdout`, `1.stderr`. The tool makes the directory when it first needs it and removes it when the tool is dropped. A process that ends without dropping it leaves the directory behind, and the next process sweeps directories of processes that are no longer running (`tools/bash.rs:135-143`, `tools/bash.rs:158-169`, `tools/bash.rs:336`).

The simple result has this form:

```text
hi
[stderr]
err
[exit code 0]
```

That is the exact result of `echo hi; echo err >&2` (`tools/bash.rs:648-654`). The two streams are kept apart, so their original order is not shown. A long stream puts a note in the middle of the preview and another note after the stream:

```text
[… lines {first}–{last} ({bytes} bytes) not shown; next: read {path} from line {first} …]
[stdout: {lines} lines, {bytes} bytes, whole in {path}]
```

If the stream is larger than 4 MiB, the second note says `the first {kept} bytes in {path}, the rest discarded` instead. The `next: read` text gives the model the path and the line to start from.

**Aha: the preview keeps both ends, not only the start.** A compiler prints the summary and the error count at the end. A test runner prints the failures at the end. Showing only the first 32 kibibytes would hide the part that matters. The first and last halves are shown, and the retained part is written to a file that `read` can open.

### 6.2 Reading

```rust
// tools/read.rs:36-47 (condensed)
/// Most lines in a page. A page is what the model gets of a file in one
/// call: a page too small has it read a file in slices, each a round
/// trip, and lose the thread between them, while a file of a thousand
/// lines read whole is a tenth of a small context. Most files come whole.
pub const PAGE: usize = 1000;
/// Most bytes in a page: a page of long lines ends early, so that one
/// page can't take up much of the context.
const BYTES: usize = 48 << 10;
/// Most bytes of a line shown; a longer line is cut there, with a mark.
pub const LINE: usize = 1000;
```

The result names the file's line count and where the next page starts. A line longer than 1,000 bytes is cut at a character boundary and ends with `…`. The result's note says how many lines were cut (`tools/read.rs:125-129`, `tools/read.rs:173-181`).

The comment says that a file of 1,000 lines read whole is a tenth of a small context. It gives no context size and no tokens-per-line figure. At the default 16,384-token context, one tenth is 1,638 tokens, or 1.6 tokens for each of 1,000 lines. A code line usually takes more than that. The claim is therefore not arithmetic for the default context. It is a design caution about page size, not a measurement.

The edit tool handles the `…` mark in three ways (`tools/edit.rs:1-20`):

1. **Match by start.** An `old_string` line that ends in `…` matches a long file line when the trimmed line starts with the trimmed part before the mark (`tools/edit.rs:422-431`).
2. **Keep the line whole.** If the `new_string` contains the same cut line as `read` showed it, `fitted` puts the original whole line into the file (`tools/edit.rs:461-484`). The model has not seen the rest, and the edit does not change it.
3. **Refuse a change or a removal.** If `old_string` has a cut line and `new_string` does not keep it as shown, the edit fails and names the line. The error says to copy the cut line into `new_string`, or to use `bash` to change it (`tools/edit.rs:124-133`, `tools/edit.rs:487-490`).

A test shows the cycle (`tools/edit.rs:733-755`):

```text
file:       short\n{2000 y}\nend
old_string: short\n{1000 y}…
new_string: brief\n{1000 y}…
result:     brief\n{2000 y}\nend
```

The replacement of `short` with `brief` succeeds, and the long line returns whole. A `new_string` that changed or removed the cut line would be refused.

### 6.3 Editing

The edit tool describes itself in the signature, and the description is the contract:

```rust
// tools/edit.rs:59-76 (condensed)
Signature {
    description: "Replace one occurrence of old_string in a file with new_string, and show the lines as they now stand, numbered, with how later lines moved. Copy old_string from read exactly: the fewest lines that find the place, the lines that change and a line or two around them, or a part of one line; a change in several places is several edits. If it could match more than once, give start, the line it begins at. ...".to_string(),
```

The match has three tiers:

```rust
// tools/edit.rs:382-386 (condensed)
enum Tier {
    Exact,
    TrailingSpace,
    Trimmed,
}
```

```rust
// tools/edit.rs:402-420 (condensed)
/// Finds `wanted` in `lines`, with their line endings, as indexes from zero, at the closest tier that matches it anywhere, and that tier.
fn find(lines: &[Cow<str>], wanted: &[&str]) -> (Vec<usize>, Tier) {
    if wanted.len() > lines.len() {
        return (Vec::new(), Tier::Exact);
    }
    for tier in [Tier::Exact, Tier::TrailingSpace, Tier::Trimmed] {
        let found: Vec<usize> = (0..=lines.len() - wanted.len())
            .filter(|&at| {
                wanted
                    .iter()
                    .enumerate()
                    .all(|(k, want)| matches(want, &lines[at + k], tier))
            })
            .collect();
        if !found.is_empty() {
            return (found, tier);
        }
    }
    (Vec::new(), Tier::Exact)
}
```

**Aha: the fallback tiers make the tool forgiving without making it ambiguous.** The tool tries exact bytes first. Only if that fails does it ignore trailing white space, then white space at both ends (`Tier::Trimmed` uses `trim()`). The tier that matched is recorded, and the replacement is rebuilt at that tier.

`find` returns every place that matches at the closest tier. `place` then applies the rules for `start`:

```rust
// tools/edit.rs:229-245 (condensed)
([at], _) => Ok(*at),
(many, Some(start)) => many
    .iter()
    .find(|&&at| at + 1 == start)
    .copied()
    .ok_or_else(|| format!(
        "old_string matches {} times in {path}, at lines {}, and none begins at line {start}; nothing changed",
        many.len(), list(many)
    )),
(many, None) => Err(format!(
    "old_string matches {} times in {path}, at lines {}; give start, the line of the one you mean; nothing changed",
    many.len(), list(many)
)),
```

- **One match:** the tool uses it. If `start` was given and names a different line, the edit still happens, and the result says `old_string began at line {first}, not {start}` (`tools/edit.rs:164-169`).
- **Several matches with `start`:** the tool uses the one whose first line is `start`. If none is, the edit fails and lists the matching lines.
- **Several matches without `start`:** the edit fails and asks for `start`.

`start` counts from 1. The test is `at + 1 == start`, where `at` is the index from zero (`tools/edit.rs:232`; the signature says "counting from 1", `tools/edit.rs:71-74`).

The rebuild happens in `fitted`:

```rust
// tools/edit.rs:119-142 (condensed)
let (found, tier) = find(&texts, &wanted);
let at = place(&found, start, &lines, &wanted, path)?;
let last = at + wanted.len();
let new = fitted(&lines[at..last], &wanted, new, tier).map_err(...)?;
let updated = replaced(&lines, at + 1, last, &new);
fs::write(&file, &updated)?;
```

At `Tier::Exact`, the new lines go in as given. At `Tier::TrailingSpace`, they go in as given, and only the line ends were ignored for the match. At `Tier::Trimmed`, when the first non-blank new line has the same leading white space as the first non-blank `old_string` line, `fitted` replaces that prefix with the leading white space of the file line that matched (`tools/edit.rs:454-460`). It does this at the start of each non-blank new line that begins with the old prefix (`tools/edit.rs:481-483`). A line that does not begin with that prefix is written as given, apart from the cut-line rule below. Blank lines stay blank. When the old prefix is empty, as in the example below, every non-blank line begins with it, so every one gets the file's indentation added.

A test shows the `Trimmed` rebuild (`tools/edit.rs:690-702`):

```text
file:
def f():
    if x:
        return 1
    return 2

old_string:
if x:
    return 1

new_string:
if y:

    return 1

result:
def f():
    if y:

        return 1
    return 2
```

The `old_string` was copied without indentation, so the match is `Trimmed`. The new lines had the same indentation as the old lines, so the file's four spaces are added to each non-blank new line. A model that worries about a copied tab does not pay a round trip.

The tool refuses two cases before it looks:

```rust
// tools/edit.rs:102-107 (condensed)
if old.trim().is_empty() {
    return Err("old_string is empty; to add lines, give the line before them as old_string and that line with the new ones as new_string".to_string());
}
if old == new {
    return Err("old_string and new_string are the same; nothing changed".to_string());
}
```

### 6.4 Writing

The write tool has one behaviour that the documentation explains:

```rust
// tools/write.rs:1-10 (condensed)
//! The `write` tool: writes a file whole, making the directories it goes
//! in, and says how long it is now. ... Left with `bash` alone, the model
//! wrote files through heredocs and `printf`, where a quote, a backslash
//! or a dollar sign in the content was lost to the shell as often as not.
//!
//! The call format puts a value on lines of its own between its tags, so
//! the content arrives without a final newline whether or not the model
//! meant one; a file that does not end in a newline is rare and a file
//! that should but does not is a nuisance, so the content gets one.
```

The last rule is applied in code:

```rust
// tools/write.rs:59-61
let mut content = content.to_string();
if !content.is_empty() && !content.ends_with('\n') {
    content.push('\n');
}
```

**Aha: the call format loses the final newline, so the tool adds it.** The tag format cannot express a trailing newline: the value ends before `</parameter>`. The tool decides the common case. An empty file stays empty.

### 6.5 Searching

`grep` and `glob` confine their paths:

```rust
// tools/mod.rs:250-264 (condensed)
fn within(tool: &str, dir: &Path, path: &str) -> Result<(), String> {
    let (target, root) = match (fs::canonicalize(dir.join(path)), fs::canonicalize(dir)) {
        (Ok(target), Ok(root)) => (target, root),
        (Err(error), _) => return Err(format!("{path} cannot be resolved: {error}")),
        (_, Err(error)) => return Err(format!("the working directory cannot be resolved: {error}")),
    };
    if target.starts_with(root) {
        Ok(())
    } else {
        Err(format!("{tool} searches the working directory and the directories under it; {path} is outside it. Use bash for other places."))
    }
}
```

**Aha: the check resolves symbolic links on both sides.** A lexical prefix check would accept `link-to-other-place/file`. `canonicalize` follows the link and compares the real paths. The error message names the escape hatch: `bash` can read anything the user can read. The restriction is a convenience, not a security boundary.

The two tools leave out ignored files, but not in every case. `glob` runs git when the directory is in a repository. It uses `git ls-files --cached --others --exclude-standard`, which leaves out ignored files, and it falls back to a directory walk when git is absent (`tools/glob.rs:148-190`). The walk leaves out hidden files and the two directories in `BUILT`, `node_modules` and `target` (`tools/glob.rs:134-136`, `tools/glob.rs:196-203`).

`grep` runs **ripgrep** (`rg`) when `rg` is installed. The tool tests `rg --version` once and remembers the answer (`tools/grep.rs:60-71`). With `rg`, git-style ignore files are honoured. Without `rg`, the tool runs plain `grep` with `-E -r -n -H -I`, and excludes only `.git`, `target`, and `node_modules` (`tools/grep.rs:98-115`). The plain fallback does **not** honour `.gitignore` or `.ignore`. Therefore the rule "Noise from ignored files" in section 2.2 is exact only when `rg` is installed. Without `rg`, `grep` can return files that git ignores, except in those three directories.

```rust
// tools/glob.rs:30-34 (condensed)
description: format!(
    "List the files whose paths match a pattern, up to {MOST}, leaving out files git ignores: *.rs for every Rust file under the directory, src/*.rs for those directly in src, src/**/*_test.go for those anywhere below it. A pattern without a slash matches a file name anywhere."
),
```

`glob` has a walk fallback with a 5,000-entry cap, and skips `node_modules` and `target` when git is absent (`tools/glob.rs:136`).

### 6.6 Parsing a call

The parser accepts two syntaxes and is careful about nesting:

```rust
// tools/call.rs:15-46 (condensed)
/// Parses the body of a `<tool_call>` block: either a JSON object with the
/// name and the arguments, as Qwen3 writes it, or a `<function=name>`
/// block of `<parameter=name>` values, as Qwen3-Coder writes it, whose
/// values are all strings. A value runs to the first `</parameter>` after
/// which the rest of the block is well formed, so a value may itself hold
/// a tool call, as a command that writes a test of one does. An error
/// quotes the line the parser stopped on.
```

**Aha: a parameter value can contain a tool call.** A `bash` command that writes a test containing a `<tool_call>` block must not end the outer parameter at the inner `</parameter>`. The comment describes two passes:

1. **The tags are walked forward.** `step` starts after `<parameter=key>`, finds every recognized tag, and tracks the open `<function>` and `<parameter>` tags. A `</parameter>` at depth zero is a candidate end. The parser tries the first candidate after which the rest of the block parses and no later parameter repeats the key. If a closing tag does not match the open tag, the walk stops, and the remaining `</parameter>` ends are tried in order (`tools/call.rs:205-243`).
2. **The places are computed backward.** `parameters` collects the places the parse may continue from: 0, and the index after every `</parameter>`. It computes the parse from each place from the last place to the first (`tools/call.rs:116-137`). `step` can then look up the parse of the rest at once, with no recursion. That is what "the whole block parses once" means.

The tag scan recognizes only `<function=...>`, `</function>`, `<parameter=...>`, and `</parameter>`. Other tags in the text, such as `<tool_call>`, are ignored (`tools/call.rs:98-115`).

"Well formed" means that, from the place after the candidate end, the parser reads parameter blocks in order, reads `</function>`, and finds only white space after it. A repeated parameter name also makes the rest ill formed (`tools/call.rs:150-161`, `tools/call.rs:172-190`). The value strips one leading newline and one trailing newline before it is returned (`tools/call.rs:186-190`).

A worked example shows which `</parameter>` wins:

```text
<tool_call>
<function=bash>
<parameter=command>
printf '%s' '<tool_call><function=read><parameter=path>a.rs</parameter></function></tool_call>'
</parameter>
</function>
</tool_call>
```

The first `</parameter>` comes after `a.rs`. At that point `<function=read>` and `<parameter=path>` are open, so the end is inside a nested tag, not at depth zero. The parser tries it later and rejects it: the rest begins `</function></tool_call>'...`, and text follows `</function>`, so the rest is not well formed. The second `</parameter>` comes after the `printf` line. At that point the inner tags are closed, so the end is at depth zero. The rest is `</function>`, which parses to the end. The outer value is therefore the whole `printf` line, including the inner tool call. A naive `split("</parameter>")` would cut the command after `a.rs` and leave broken shell text.

## 7. The transcript

The harness writes one JSON Lines file per session, with `start`, `user`, `thought`, `text`, `note`, `reply`, `thinking_budget`, `call`, `output`, `nudge`, `compacting`, `compacted`, `error`, `interrupted`, and `end` records (`harness/log.rs:109-282`). [02 Command line and sessions](02-cli-and-sessions.md), section 5 covers the file format and the in-place streaming rewrite. Here, note two uses:

- **Restart.** `Harness::reopen` replays the transcript turns into a new conversation ([02 Command line and sessions](02-cli-and-sessions.md), section 9).
- **Cost analysis.** `Stats::report` prints where the time went.

The server sends a `State` value after each reply. `State` is a protocol message with two fields: the conversation's token count and its model statistics (`server/protocol.rs:157-163`; [03 Server and protocol](03-server-and-protocol.md), section 3.4). The harness `Stats` value holds that model part and adds the tool calls and their seconds:

```rust
// harness/mod.rs:464-478 (condensed)
pub struct Stats {
    pub model: crate::models::Stats,
    /// Tool calls run, and the time they took.
    pub calls: usize,
    pub tool_seconds: f64,
}

// models/chat.rs:133-153 (condensed)
pub struct Stats {
    /// Tokens taken from a saved state rather than read.
    pub cached: Tally,
    pub prompt: Tally,
    pub thought: Tally,
    pub answer: Tally,
    pub drafted: usize,
    pub accepted: usize,
}

pub struct Tally {
    pub tokens: usize,
    pub seconds: f64,
}
```

`Tally` holds a token count and the seconds the model took over those tokens. `rate()` divides the two, so its unit is tokens a second. `cached` counts tokens restored from a saved state, `prompt` counts tokens the model read as prompts, `thought` counts tokens generated in thoughts, and `answer` counts tokens generated as replies. `drafted` and `accepted` count speculative decoding: tokens drafted ahead, and how many the sampler accepted.

`Stats::report` builds one row for each of `loading`, `cached`, `prompt`, `thought`, `answer`, `tools`, `other`, and `total` (`harness/mod.rs:478-544`). It inserts a `drafts` row when the model drafted any token. The token columns use tokens, the time columns use seconds, and the rate columns use tokens a second. The `tools` row uses calls, not tokens. `other` is the total time minus loading, model time, and tool time, and never less than zero.

`--stats` prints this report. In the interactive shell, `/stats` does.

## 8. The benchmark runner

`scripts/swebench.py` is the repository's end-to-end quality measure. It is the only script that scores task outcomes. The `--bench` command line option measures model speed, not correctness. The script runs the agent on SWE-bench Lite and scores the patch with the official SWE-bench harness.

**SWE-bench Lite** is a benchmark of 300 software engineering tasks from open-source Python projects. Each **instance** is one dataset row: a repository, a base commit, an issue, and tests that decide whether a fix works. The script fetches all 300 rows from the Hugging Face datasets server (`scripts/swebench.py:25-26`, `scripts/swebench.py:86-92`). By default it runs `qwen3.8-27b`, the model that dwim runs (`scripts/swebench.py:301`).

An **evaluation image** is a Docker image that SWE-bench builds for one instance. It holds the repository at `/testbed`, checked out at the base commit before the fix, with the project and its dependencies installed. It also has a conda environment named `testbed` (`scripts/swebench.py:4-8`, `scripts/swebench.py:47-53`). The script does not checkout a commit itself. It takes the repository as the image has it.

```mermaid
flowchart LR
    subgraph HOST["Host machine"]
        CLONE["Clone<br/>work/checkouts/{instance}"]
        DWIM["dwim process<br/>model, read, edit"]
        WRAP["shell wrapper<br/>docker exec"]
    end
    subgraph BOX["Container dwim.{instance}"]
        TESTBED["/testbed<br/>the same clone"]
        CONDA["conda environment testbed"]
    end
    IMAGE["Evaluation image"] -->|"docker cp /testbed"| CLONE
    DWIM -->|"bash tool"| WRAP
    WRAP -->|"docker exec -w"| TESTBED
    CLONE -->|"bind mount at /testbed"| TESTBED
    CLONE -->|"bind mount at its own path"| TESTBED
    TESTBED --- CONDA
```

The setup and teardown have six steps:

1. `prepare` pulls the image if it is not present, with `docker pull --platform linux/amd64`, then calls `checkout` (`scripts/swebench.py:110-116`).
2. `checkout` makes a temporary container from the image, copies `/testbed` out with `docker cp` to `work/checkouts/{instance_id}`, and removes the temporary container (`scripts/swebench.py:119-131`).
3. `container` starts a container named `dwim.{instance_id}` from the image with `sleep infinity`. It bind-mounts the clone at `/testbed` and at its own host path, and sets the work directory to that path (`scripts/swebench.py:134-145`).
4. The shell wrapper runs each `dwim` command through `docker exec -i -w "$(pwd -P)"`. The working directory is the clone path on the host, and the same path exists in the container because of the second mount.
5. `solve` runs `dwim` from the host with the clone as the working directory and the wrapper as `--shell`. On exit or timeout, it removes the container, takes `git diff` in the clone as the patch, and removes the clone (`scripts/swebench.py:148-175`).
6. The patched harness removes each image after it scores the instance (see below).

```python
# scripts/swebench.py:119-145 (condensed)
def checkout(row, work):
    clone = work / "checkouts" / row["instance_id"]
    ...
    docker("create", "--platform", PLATFORM, "--name", name, image, "true")
    docker("cp", f"{name}:/testbed", str(clone))
    docker("rm", name)
    return clone

def container(row, clone):
    name = f"dwim.{row['instance_id']}"
    docker("rm", "-f", name)
    docker(
        "run", "--detach", "--platform", PLATFORM, "--name", name,
        "--volume", f"{clone}:/testbed", "--volume", f"{clone}:{clone}",
        "--workdir", str(clone), row["image"], "sleep", "infinity",
    )
    return name
```

The shell is replaced with the `docker exec` wrapper:

```python
# scripts/swebench.py:50-53 (condensed)
SHELL = """#!/bin/sh
exec docker exec -i -w "$(pwd -P)" {container} bash -c \
'source /opt/miniconda3/etc/profile.d/conda.sh && conda activate testbed && exec bash -c "$1"' bash "$2"
"""
```

The bash tool runs the wrapper as `wrapper -c command` (`tools/bash.rs:145-146`, `tools/bash.rs:212-214`; `cli/opts.rs:80-83`). For a script, the shell puts `-c` in `$1` and the command in `$2`. The wrapper passes `$2` to the inner `bash -c` as `$1`, and that shell runs the model's command.

Both sides see one filesystem. The clone is bind-mounted into the container at `/testbed` and at its own path. The read and edit tools open the host path directly. The `bash` tool runs through the wrapper, so a command in the container sees the same bytes. A change from either side is immediately visible to the other.

The task prompt is not the raw issue:

```python
# scripts/swebench.py:33-45 (condensed)
# How the issue is put to the model. SWE-bench leaves that to the scaffold,
# and given the raw issue of a project it knows, the model takes it for the
# commit that closed it and spends its turns recalling that commit's tests
# and trying to fetch the project's history to reproduce them, rather than
# making the fix ...
TASK = """Fix the issue below in this checkout. Make the smallest change that fixes it, update the tests the change breaks, and run them. Don't try to recall or reproduce how the project fixed it upstream; make the fix from this code.

<issue>
{issue}
</issue>
"""
```

**Aha: the model runs outside the container and edits the checkout as the user.** The issue text is untrusted. The script warns:

```python
# scripts/swebench.py:18-20
The agent's commands run in the container, but the model runs here, and
its read and edit tools work on the checkout as you, on an untrusted issue.
Run this in a virtual machine rather than on a machine you care about.
```

The read and edit tools are **not** confined to the container, and they are not confined to the checkout. They join the given path to the working directory, and neither rejects a path outside it (`tools/read.rs:91-106`, `tools/edit.rs:98-113`). An absolute path or a path with `..` can therefore reach any host file the user can read or write. `grep` and `glob` do reject such a path (section 6.5); `read` and `edit` do not. The bash commands are confined to the container, but the container mounts the clone read-write. The process boundary protects the rest of the host from the target repository's commands, not the host from the model's file tools. This is consistent with the product's stated boundary ([00 Overview](00-overview.md), section 4), and the script's advice to use a virtual machine covers both cases.

The scoring runs the official SWE-bench harness from the Python package `swebench`. The script checks that the package is installed and exits with `pip install swebench` if it is not (`scripts/swebench.py:238-243`). The version is not pinned, so a later run can use a different harness. This is a reproducibility limitation.

The script writes a predictions file: one JSON object per instance with `instance_id`, `model_name_or_path`, and `model_patch` (`scripts/swebench.py:212-218`). It writes each patch as it is made.

Two in-memory patches change the official harness (`scripts/swebench.py:55-83`):

1. `ImageCollection.pull` sets `platform="linux/amd64"`. The evaluation images exist only for x86_64, and an arm64 Docker refuses to pull one unless it is asked for that platform (`scripts/swebench.py:29-31`).
2. `cleanup_container` removes the instance's image after it removes the container. Without this, the images accumulate.

The patches are Python monkeypatches, not file patches. The script installs them, then runs `swebench.harness.run_evaluation` in the same process with `runpy` (`scripts/swebench.py:59-82`).

The second patch is a disk-space necessity. Each image is about 2 gigabytes (GB) of its own, and 300 instances total about 600 GB. The script's comment says the machine has 300 GB (`scripts/swebench.py:55-58`). Without the removal, the images cannot all fit.

## 9. What the harness does not do

- **No approval prompts.** The model runs any command. The README states the boundary ([00 Overview](00-overview.md), section 4).
- **No sandboxing.** The harness does not use Linux namespaces, secure computing mode (seccomp), or a virtual machine. A **Linux namespace** is a kernel feature that gives a process its own view of a resource, such as process identifiers, mounts, the network, or user identifiers. **Secure computing mode (seccomp)** is a Linux kernel feature that limits the system calls a process may make. The caller supplies a sandbox, for example a container or a virtual machine.
- **No parallel tools.** Calls run in order, one at a time.
- **No sub-agents.** There is no delegation tool and no nested loop.
- **No external tool protocol.** There is no client for the Model Context Protocol (MCP) and no plugin system.
- **No per-tool token budget.** The output limits are per stream and per page, not per turn.
- **No memory across sessions.** The transcript is a record; it is not read back into a new conversation as memory, except through compaction and restart.
- **No cost accounting in money.** Statistics count tokens and seconds, not price.

## 10. Terms used in this document

| Term | Meaning |
|------|---------|
| **Agent** | A model that calls tools and continues until a task is done. |
| **AGENTS.md** | A project instruction file in the working directory; the system prompt includes up to 32 KiB of it. |
| **Auto mode** | A mode that answers a reply with no tool and no `DONE` by sending `GO_ON`, up to 10 rounds. |
| **Benchmark** | A fixed set of tasks with a score. |
| **ChatML** | The chat template of the Qwen3 family: turns open with `<|im_start|>` and close with `<|im_end|>`; a thought sits between `<think>` and `</think>`. |
| **Chat template** | The text format a model was trained on, which gives its turn and thought tags. |
| **Compaction** | Replacing a long conversation with a short note and a recent tail. |
| **Context window** | The number of tokens that a conversation can hold. |
| **Docker** | A container runtime. |
| **Environment block** | The working directory, file list, git state, platform, and date sent ahead of the first message. |
| **Evaluation image** | A Docker image that SWE-bench builds for one instance, with the repository at `/testbed` and its dependencies installed. |
| **Event** | A value that reports one step of a turn to the user interface. |
| **Exit code** | The number a command returns to say whether it succeeded. |
| **Guard** | A rule that stops a known failure of the model. |
| **Head** | A string the conversation writes into an answer before it generates that section. |
| **Heredoc** | A shell form that feeds lines to a command as standard input. |
| **Instance** | One SWE-bench dataset row: a repository, a base commit, an issue, and the tests that decide a fix. |
| **JSON Lines** | A file format with one JSON object on each line. |
| **Kibibyte (KiB)** | 1,024 bytes. |
| **Mebibyte (MiB)** | 1,024 kibibytes. |
| **Model Context Protocol (MCP)** | A standard for connecting external tools to a model. |
| **Model state** | The model's key/value caches and recurrent states after it reads a prompt; the server saves it and restores it for the same prompt. |
| **Namespace** | A Linux kernel feature that gives a process its own view of a resource, such as process identifiers, mounts, the network, or user identifiers. |
| **Process group** | A set of processes that receive a signal together. |
| **Prompt injection** | An attack in which text in the input is treated as an instruction. |
| **Qwen3** | The model family that dwim uses. |
| **Qwen3-Coder** | A coding variant of Qwen3 that writes tool calls as `<function=...>` tags. |
| **Qwen3.8-27B** | The model that dwim runs, as a four-bit file of about 16 GB. |
| **Reproduction** | A small program or command that shows a bug. |
| **ripgrep (`rg`)** | A search program that honours git-style ignore files. |
| **seccomp** | Secure computing mode: a Linux feature that limits the system calls a process may make. |
| **Server** | The background process that loads the model once and gives sessions access over a Unix domain socket. |
| **Spill file** | The file under the `bash` tool's output directory that holds a stream longer than the preview, up to 4 MiB. |
| **Standard error** | The output stream for messages and diagnostics. |
| **SWE-bench** | A benchmark of software engineering issues from open-source repositories, scored by tests. |
| **SWE-bench Lite** | A 300-instance subset of SWE-bench with test-based scoring. |
| **System prompt** | The instructions and tool descriptions sent before the conversation. |
| **Thinking budget** | The most tokens one thought may take before the chat ends it with a closing sentence. |
| **Thought** | The model's reasoning tokens, between `<think>` and `</think>`, before the reply. |
| **Tool call** | A structured request from the model to run a tool. |
| **Turn** | One user message and the agent work that follows, until the reply that ends it. In code, a `Turn` is one transcript item: `User`, `Reply`, or `Results` (section 1.1). |

## 11. What to read next

Read [07 Comparison](07-comparison-foundation-ai.md) for how this harness compares with the `foundation_ai` agent loop. Read [02 Command line and sessions](02-cli-and-sessions.md) for how these events reach the screen.
