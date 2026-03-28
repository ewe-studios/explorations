# Fragment Architecture Deep Dive: Type System and defineFragment Factory

## Overview

The Fragment type system in `@formulas/src.rust/src.deployAnywhere/fragment/src/fragment.ts` provides a unified abstraction for representing entities (agents, channels, files, toolkits, etc.) with template literals, reference tracking, and type-safe construction. This deep dive explores the core `Fragment` type, the `defineFragment` factory, and how template interpolation, context rendering, and TUI rendering work together.

---

## 1. The Fragment Type

### 1.1 Core Structure

The `Fragment` interface is the foundation of the entire system:

```typescript
export interface Fragment<
  Type extends string,
  Name extends string,
  References extends any[],
> {
  readonly type: Type;
  readonly id: Name;
  readonly template: TemplateStringsArray;
  readonly references: References;
}
```

**Properties:**

| Property | Type | Description |
|----------|------|-------------|
| `type` | `Type extends string` | Discriminator for the fragment kind (e.g., `"agent"`, `"channel"`) |
| `id` | `Name extends string` | Unique identifier for this fragment instance |
| `template` | `TemplateStringsArray` | Raw template strings from tagged template literal |
| `references` | `References extends any[]` | Interpolated values captured from template |

### 1.2 How References Are Captured

References are captured via TypeScript's tagged template literal typing:

```typescript
// Usage pattern:
const agent = Agent("coordinator")`
  I am ${Channel("general")} channel coordinator
  working with ${otherAgent}
`;

// TypeScript captures:
// - template: ["\n  I am ", " channel coordinator\n  working with ", "\n"]
// - references: [Channel("general")(), otherAgent]
```

The `references` array preserves the order of interpolation, matching the gaps in the `template` array.

### 1.3 Type Inference for IDs and References

TypeScript infers types automatically:

```typescript
// Agent builder infers ID type
const myAgent = Agent("my-agent")`...`;
// myAgent has type Fragment<"agent", "my-agent", [...]>

// References are inferred from interpolation
const channel = Channel("dev")`...`;
const agent = Agent("bot")`Monitor ${channel}`;
// agent.references has type [Fragment<"channel", "dev", unknown[]>]
```

### 1.4 FragmentClass: The Runtime Representation

The runtime uses a class-like structure with static properties:

```typescript
export interface FragmentClass<
  Type extends string,
  ID extends string,
  References extends any[],
  Extra extends object = {},
  Render extends FragmentRender<any> | undefined = undefined,
> extends Fragment<Type, ID, References> {
  new (_: never): Fragment<Type, ID, References> & Extra;
  readonly render?: Render;
}
```

Key insight: **The "class" is actually a constructor function with static properties**. The `_: never` parameter prevents actual instantiation—it's used as a type-level container.

---

## 2. defineFragment Factory

### 2.1 Triple-Nested Function Structure

`defineFragment` uses a triple-nested function pattern for fluent builder construction:

```
defineFragment(type)(options)(id, props?)(template, ...references)
   │              │           │             └─ Step 4: Returns FragmentClass
   │              │           └─ Step 3: Optional props for Extra fields
   │              └─ Step 2: Optional render config
   └─ Step 1: Fragment type discriminator
```

### 2.2 Full Implementation Breakdown

```typescript
export const defineFragment =
  <Type extends string>(type: Type) =>           // Step 1: Type discriminator
  <Extra extends object = {}, Options extends DefineFragmentOptions = {}>(
    options?: Options,
  ): FragmentBuilder<Type, Extra, Options["render"]> => {
    type Render = Options["render"];

    const builder = <ID extends string, Props extends Extra>(
      id: ID,
      ...args: keyof Extra extends never ? [] : [props: Props]  // Conditional args
    ) => {
      const props = (args[0] ?? {}) as Props;
      return <References extends any[]>(
        template: TemplateStringsArray,
        ...references: References
      ): FragmentClass<Type, ID, References, Props, Render> => {
        const cls = class {
          static readonly type = type;
          static readonly id = id;
          static readonly template = template;
          static readonly references = references;
          constructor(_: never) {}
        };
        // Copy options as statics (preserves getters/methods)
        if (options) {
          Object.defineProperties(
            cls,
            Object.getOwnPropertyDescriptors(options),
          );
        }
        // Copy per-instance props
        Object.assign(cls, props);
        return cls as FragmentClass<Type, ID, References, Props, Render>;
      };
    };

    // Add type guard method
    builder.is = <T extends Fragment<Type, string, any[]>>(
      x: any
    ): x is T => x?.type === type;

    builder.type = type;
    builder.render = options?.render as Render;

    return builder as FragmentBuilder<Type, Extra, Render>;
  };
```

### 2.3 Type Parameters Explained

| Parameter | Purpose | Example |
|-----------|---------|---------|
| `Type` | Fragment discriminator | `"agent"`, `"channel"` |
| `Extra` | Additional instance properties | `{ language: string }` for File |
| `Render` | Render configuration | `{ context: (f) => \`@${f.id}\` }` |
| `ID` | Specific instance identifier | `"my-channel"` |
| `Props` | Runtime property values | `{ owner: "sam", repo: "repo" }` |

### 2.4 Static Properties on Fragment Classes

The factory attaches properties at multiple levels:

```typescript
// Level 1: Builder statics (from defineFragment)
Agent.type    // "agent"
Agent.render  // { context: ..., tui: ... }
Agent.is      // Type guard function

// Level 2: Instance statics (from Agent("id"))
myAgent.type       // "agent"
myAgent.id         // "coordinator"
myAgent.template   // TemplateStringsArray
myAgent.references // [...]
myAgent.render     // Inherited from builder
```

---

## 3. Template Literal Interpolation

### 3.1 How ${Reference} Capture Works

When you write:

```typescript
const agent = Agent("bot")`
  Monitor ${Channel("alerts")} for errors
  Send reports to ${Channel("reports")}
`;
```

TypeScript captures:

```typescript
template = [
  "\n  Monitor ",
  " for errors\n  Send reports to ",
  "\n"
]
references = [
  Channel("alerts")(),
  Channel("reports")()
]
```

### 3.2 Reference Resolution: Thunks vs Values

**Thunks** are zero-argument functions enabling lazy resolution:

```typescript
// Thunk definition
export type Thunk<T = unknown> = () => T;

export const isThunk = (value: unknown): value is Thunk =>
  typeof value === "function" &&
  (value as Function).length === 0 &&
  !isFragment(value) &&
  !S.isSchema(value);

export const resolveThunk = <T>(value: T | Thunk<T>): T =>
  isThunk(value) ? value() : value;
```

**Why thunks matter:** Forward references and circular dependencies.

```typescript
// Forward reference pattern
const agentA = Agent("a")`Works with ${() => agentB}`;
const agentB = Agent("b")`Works with ${() => agentA}`;

// Resolution happens later in collectReferences()
```

### 3.3 References Array Construction

The `references` array is built during template literal capture:

```typescript
// Source (from agent.ts):
Agent = defineFragment("agent")({
  render: { context: (agent) => `@${agent.id}` }
});

// Usage:
const channel = Channel("dev")`Development channel`;
const agent = Agent("bot")`Monitor ${channel}`;

// Result:
agent.references === [channel]  // Array with captured Channel fragment
```

### 3.4 Circular Reference Handling

The `context.ts` uses a visited set to prevent infinite loops:

```typescript
const visited = new Set<string>();

const collect = (rawRef: any, depth: number): void => {
  const ref = resolveThunk(rawRef);
  if (!ref) return;

  const key = `${ref.type}:${ref.id}`;
  if (visited.has(key)) return;  // Cycle detection
  visited.add(key);

  // Process reference and recurse
  ref.references.forEach((r: any) => collect(r, depth));
};
```

---

## 4. Context Rendering

### 4.1 The Context Render Function

Context rendering converts fragments to text for agent prompts:

```typescript
export interface FragmentRender<T = unknown> {
  context?: (fragment: ResolvedFragment<T>, config?: RenderConfig) => string;
  // ...
}
```

**Key:** The fragment's references are pre-resolved, so type guards work directly.

### 4.2 Rendering Agents as @id

From `agent.ts`:

```typescript
export const Agent = defineFragment("agent")({
  render: {
    context: (agent: Agent) => `@${agent.id}`,
    // ...
  }
});

// Result:
const agent = Agent("coordinator")`...`;
agent.render?.context(agent)  // "@coordinator"
```

### 4.3 Rendering Channels as #id

From `chat/channel.ts` (pattern shown in context.ts):

```typescript
// Channel rendering
channels.map((c) => `### #${c.id}\n\n${c.content}`)
```

### 4.4 Custom Context Renderers

Complex rendering for compound fragments:

```typescript
// GroupChat rendering (from context.ts usage)
context: (groupChat) => {
  const members = groupChat.references
    .filter(isAgent)
    .map(a => a.id);
  return members.length > 0
    ? `@{${members.join(", ")}}`
    : `@{${groupChat.id}}`;
}
```

### 4.5 The renderTemplate Pipeline

From `render-template.ts`:

```typescript
export function renderTemplate(
  template: TemplateStringsArray,
  references: any[],
  config?: RenderConfig,
): string {
  let result = template[0];
  for (let i = 0; i < references.length; i++) {
    const ref = references[i];
    result += stringify(ref, config) + template[i + 1];
  }
  return result;
}

function stringify(rawValue: unknown, config?: RenderConfig): string {
  const value = resolveThunk(rawValue);

  if (isFragment(value)) {
    return stringifyFragment(value, config);  // Uses render.context
  }

  // Handle primitives, arrays, objects
  if (typeof value !== "object") return String(value);
  return "\n" + yaml.stringify(serialize(value, config)).trimEnd();
}

function stringifyFragment(
  fragment: Fragment<string, string, any[]>,
  config?: RenderConfig,
): string {
  const frag = fragment as any;
  if (frag.render?.context) {
    const resolved = resolveFragmentReferences(fragment);
    return frag.render.context(resolved, config);
  }
  return `{${frag.type}:${frag.id}}`;  // Fallback
}
```

---

## 5. TUI Rendering

### 5.1 FragmentRenderTui Interface

```typescript
export interface FragmentRenderTui<T = unknown> {
  sidebar?: (props: {
    fragments: T[];
    selectedId?: string;
    onSelect?: (id: string, type: string) => void;
  }) => JSX.Element;

  content?: (props: ContentViewProps<T>) => JSX.Element;

  chat?: (props: { fragment: T; content: string }) => JSX.Element;

  focusable?: boolean;
  icon?: string;
  color?: string;
  sectionTitle?: string;
}
```

### 5.2 Sidebar Components

Sidebar receives all fragments of a type for list rendering:

```typescript
// Example sidebar pattern
sidebar: (props) => (
  <div>
    {props.fragments.map(f => (
      <Item
        key={f.id}
        selected={f.id === props.selectedId}
        onClick={() => props.onSelect?.(f.id, f.type)}
      >
        {f.render?.tui?.icon} {f.id}
      </Item>
    ))}
  </div>
)
```

### 5.3 Content View Components

Content view replaces the default ChatView:

```typescript
export interface ContentViewProps<T = unknown> {
  fragment: T;
  focused: boolean;
  onBack: () => void;
  onExit: () => void;
}

// Usage in Agent (from agent.ts):
content: AgentContent
```

### 5.4 Focusable Fragments

```typescript
// Agent configuration
tui: {
  content: AgentContent,
  focusable: true,  // Pressing Enter focuses the content
}

// If focusable: false, Enter key has no effect
```

### 5.5 Icon and Color Configuration

```typescript
// GitHub Repository pattern (from fragment.ts example)
render: {
  context: (frag) => `📦${frag.owner}/${frag.repo}`,
  tui: {
    icon: "📦",
    color: "blue",
    sectionTitle: "Repositories",
  }
}
```

---

## 6. Type Guards

### 6.1 The .is<T>() Method

Each fragment builder has a built-in type guard:

```typescript
// Generated by defineFragment
builder.is = <T extends Fragment<Type, string, any[]>>(
  x: any
): x is T => x?.type === type;

// Usage (from agent.ts):
export const isAgent = Agent.is<Agent>;

// Type signature:
// isAgent: (x: any) => x is Agent<string, any[]>
```

### 6.2 Runtime Type Checking

The generic `isFragment` function checks the shape:

```typescript
export function isFragment(
  value: unknown,
): value is Fragment<string, string, any[]> {
  if (value === null || value === undefined) return false;
  const v = value as any;
  return (
    (typeof value === "function" || typeof value === "object") &&
    "type" in v &&
    typeof v.type === "string" &&
    "id" in v &&
    "template" in v &&
    "references" in v
  );
}
```

### 6.3 Filtering References by Type

Pattern from `context.ts`:

```typescript
// Collect only toolkits from references
export const collectToolkits = (agent: Agent): Toolkit[] =>
  collectReferences(agent.references ?? [], {
    matches: isToolkit,
    shouldRecurse: (v) => isToolkit(v) || isFile(v) || isTool(v) || isRole(v),
  });

// In collectReferences:
const ref = resolveThunk(rawRef);
if (isAgent(ref)) { /* handle agent */ }
else if (isChannel(ref)) { /* handle channel */ }
else if (isFile(ref)) { /* handle file */ }
```

---

## 7. Complete Examples

### 7.1 Creating a Custom Fragment Type

```typescript
import { defineFragment } from "./fragment.ts";

// Define a Task fragment type
interface TaskProps {
  priority: "low" | "medium" | "high";
  assignee?: string;
}

export const Task = defineFragment("task")<TaskProps>({
  render: {
    context: (task) => {
      const priorityIcon = { low: "🟢", medium: "🟡", high: "🔴" }[task.priority];
      return `${priorityIcon}[${task.id}]${task.assignee ? ` -> @${task.assignee}` : ""}`;
    },
    tui: {
      icon: "📋",
      color: "yellow",
      sectionTitle: "Tasks",
      focusable: true,
    }
  }
});

export const isTask = Task.is;
```

### 7.2 Adding Custom Properties

```typescript
// File fragment with language property
export const File = defineFragment("file")<{ language: string }>({
  render: {
    context: (f) => `[${f.id}](file://${f.id})`
  }
});

// Usage:
const mainFile = File("src/main.ts", { language: "typescript" })`
  Main entry point for the application
  Exports ${function1} and ${function2}
`;

// Access custom property:
mainFile.language  // "typescript"
```

### 7.3 Rendering Configuration

```typescript
// Complete rendering setup
export const GitHubRepo = defineFragment("github-repository")<{
  owner: string;
  repo: string;
}>({
  render: {
    // Text rendering for agent context
    context: (repo) => `📦 ${repo.owner}/${repo.repo}`,

    // TUI rendering
    tui: {
      // Sidebar list
      sidebar: ({ fragments, selectedId, onSelect }) => (
        <VStack>
          {fragments.map(repo => (
            <Text
              key={repo.id}
              color={repo.id === selectedId ? "blue" : "white"}
              onClick={() => onSelect?.(repo.id, repo.type)}
            >
              📦 {repo.owner}/{repo.repo}
            </Text>
          ))}
        </VStack>
      ),

      // Detail view
      content: ({ fragment, focused, onBack }) => (
        <Box>
          <Text>Repository: {fragment.owner}/{fragment.repo}</Text>
          <Text>Template: {fragment.template.join(" [...] ")}</Text>
        </Box>
      ),

      icon: "📦",
      color: "blue",
      sectionTitle: "GitHub Repositories",
    }
  }
});
```

### 7.4 Usage Patterns

**Pattern 1: Simple agent with channel**

```typescript
const general = Channel("general")`
  Main discussion channel for the team
`;

const bot = Agent("coordinator")`
  I monitor ${general} and respond to questions
`;
```

**Pattern 2: Agent with multiple references**

```typescript
const db = Channel("db")`Database channel`;
const api = Channel("api")`API channel`;

const worker = Agent("worker", { role: "processor" })`
  I process messages from ${db} and publish to ${api}
  using ${File("src/processor.ts", { language: "typescript" })}
`;
```

**Pattern 3: Forward references with thunks**

```typescript
const agentA = Agent("a")`Collaborates with ${() => agentB}`;
const agentB = Agent("b")`Collaborates with ${() => agentA}`;

// Circular reference resolved at context building time
```

---

## Execution Trace Diagrams

### Diagram 1: Fragment Creation Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    defineFragment("agent")                       │
│                         Step 1: Type                             │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                    ({ render: {...} })                           │
│                      Step 2: Options                             │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                  builder = FragmentBuilder                       │
│  - builder.type = "agent"                                        │
│  - builder.render = { context, tui }                             │
│  - builder.is = type guard                                       │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                Agent("coordinator")                              │
│                   Step 3: ID + Props                             │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│  template`...references...`                                      │
│                  Step 4: Template Literal                        │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│  FragmentClass {                                                 │
│    static type = "agent"                                         │
│    static id = "coordinator"                                     │
│    static template = TemplateStringsArray                        │
│    static references = [Channel, File, ...]                      │
│    static render = { context, tui }                              │
│  }                                                               │
└─────────────────────────────────────────────────────────────────┘
```

### Diagram 2: Reference Resolution Flow

```
┌─────────────────────────────────────────────────────────────────┐
│  Agent("bot")`Monitor ${ch1} and ${() => ch2}`                  │
│  references = [ChannelFragment, Thunk<Channel>]                 │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                    renderTemplate()                              │
│  for each reference: resolveThunk(ref) → stringify(ref)         │
└─────────────────────────┬───────────────────────────────────────┘
                          │
          ┌───────────────┴───────────────┐
          │                               │
          ▼                               ▼
┌──────────────────┐            ┌──────────────────┐
│ Direct Fragment  │            │    Thunk () =>   │
│  isFragment=true │            │   resolveThunk() │
│  use render.ctx  │            │   → Fragment     │
└────────┬─────────┘            └────────┬─────────┘
         │                               │
         └───────────────┬───────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│              stringifyFragment(fragment)                         │
│  if (fragment.render?.context)                                   │
│    return fragment.render.context(resolvedFragment)              │
│  else                                                            │
│    return `{${type}:${id}}`                                      │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│  "Monitor #dev and #alerts"                                     │
└─────────────────────────────────────────────────────────────────┘
```

### Diagram 3: Type Guard Dispatch

```
                    ┌─────────────────┐
                    │  value: unknown │
                    └────────┬────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │  isFragment(v)  │──false──▶ false
                    └────────┬────────┘
                             │ true
                             ▼
                    ┌─────────────────┐
                    │  v.type: string │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
              ▼              ▼              ▼
     ┌────────────┐  ┌────────────┐  ┌────────────┐
     │ Agent.is() │  │Channel.is()│  │ File.is()  │
     │ v.type===  │  │ v.type===  │  │ v.type===  │
     │  "agent"   │  │ "channel"  │  │  "file"    │
     └─────┬──────┘  └─────┬──────┘  └─────┬──────┘
           │               │               │
           ▼               ▼               ▼
     x is Agent      x is Channel     x is File
```

---

## Type Flow Diagrams

### Diagram 4: Generic Type Propagation

```
defineFragment<Type>(type)
       │
       │ Type = "agent"
       ▼
<Extra, Options>(options?)
       │
       │ Extra = {}
       │ Options = { render: ... }
       │ Render = Options["render"]
       ▼
<ID, Props>(id, props?)
       │
       │ ID = "coordinator"
       │ Props = {}
       ▼
<References>(template, ...references)
       │
       │ References = [Channel<"general">, ...]
       ▼
FragmentClass<"agent", "coordinator", References, Props, Render>
```

### Diagram 5: ResolvedFragment Type Transformation

```typescript
type ResolvedFragment<T> = T extends Fragment<infer Type, infer ID, infer _Refs>
  ? Omit<T, "references"> & Fragment<Type, ID, unknown[]>
  : T;

// Before: Fragment<"agent", "bot", [Channel<"dev">, File<"src.ts">]>
// After:  Fragment<"agent", "bot", unknown[]>

// References are resolved at runtime, so type guards work:
// const agents = fragment.references.filter(isAgent);
// agents has type Agent[] (not Thunk<Agent>[])
```

---

## Build It Yourself Exercises

### Exercise 1: Create a Custom Resource Fragment

Create a `Resource` fragment type for external API endpoints:

```typescript
// Your task:
// 1. Define ResourceProps interface with url and method fields
// 2. Create Resource using defineFragment
// 3. Add context renderer that outputs "METHOD url"
// 4. Add TUI config with icon and color

import { defineFragment, type FragmentRender } from "./fragment.ts";

interface ResourceProps {
  // Your code here
}

export const Resource = defineFragment("resource")<ResourceProps>({
  // Your code here
});

export const isResource = Resource.is;
```

### Exercise 2: Implement Circular Reference Detection

Extend the reference collector to detect and report cycles:

```typescript
// Modify collect() to track the reference path
const collect = (rawRef: any, depth: number, path: string[] = []): void => {
  const ref = resolveThunk(rawRef);
  const key = `${ref.type}:${ref.id}`;

  // Detect cycle
  if (path.includes(key)) {
    console.warn(`Circular reference detected: ${[...path, key].join(" -> ")}`);
    return;
  }

  // Your code: continue collection with updated path
};
```

### Exercise 3: Build a Custom Context Renderer

Create a YAML-formatted context renderer for debugging:

```typescript
function debugContextRenderer(fragment: any): string {
  return `
type: ${fragment.type}
id: ${fragment.id}
references:
${fragment.references.map((r: any) => `  - ${resolveThunk(r)}`).join("\n")}
template: ${fragment.template.length} parts
`.trim();
}

// Test with:
const agent = Agent("test")`Debug ${Channel("test")}`;
console.log(debugContextRenderer(agent));
```

### Exercise 4: Create a Fragment Composition Helper

Build a utility to merge multiple fragment references:

```typescript
function composeFragments<T extends Fragment<string, string, any[]>>(
  ...fragments: T[]
): {
  allReferences: any[];
  byType: <Type extends string>(type: Type) => Extract<T, Fragment<Type, any, any>>[];
} {
  // Your implementation here
}
```

---

## Key Takeaways

1. **Fragment is a type-level container**: The `Fragment` interface uses TypeScript's type system to track type, ID, and references at compile time.

2. **defineFragment is a factory factory**: The triple-nested function pattern enables both type inference and runtime configuration.

3. **Thunks enable lazy resolution**: Zero-argument functions in references allow forward references and prevent infinite recursion.

4. **render.context is self-describing**: Each fragment type knows how to render itself as text, with pre-resolved references for type guards.

5. **TUI rendering is opt-in**: Not all fragment types need TUI components; fallback behavior handles basic display.

6. **Type guards are first-class**: The `.is()` method is generated automatically and integrates with TypeScript's type narrowing.

7. **Static properties carry metadata**: All configuration lives on the constructor function, not instances.

---

## Related Files

- **Core implementation**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.deployAnywhere/fragment/src/fragment.ts`
- **Agent patterns**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.deployAnywhere/fragment/src/agent.ts`
- **Context rendering**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.deployAnywhere/fragment/src/context.ts`
- **Template utilities**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.deployAnywhere/fragment/src/util/render-template.ts`
