# Executor UI & React Client — Deep Dive Exploration

**Package:** `@executor/ui`, `@executor/clients/react`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/executor/packages/ui` + `/packages/clients/react`  
**Total Files:** 62 files (59 UI components + 3 lib files + 7 React client files)  

---

## 1. Module Overview

### UI Package (`@executor/ui`)

The UI package provides **React component library** for the Executor system:

- **Base components** — Button, Input, Card, Dialog, etc. (shadcn/ui pattern)
- **Code highlighting** — Shiki-based syntax highlighting with Streamdown integration
- **Markdown rendering** — Prose-styled markdown with code block support
- **Utility functions** — `cn()` for class name merging

### React Client Package (`@executor/clients/react`)

The React client provides **Effect Atom-based reactive client**:

- **Typed API client** — AtomHttpApi wrapper around Executor API
- **Query atoms** — Reactive queries for tools, sources, secrets
- **Mutation atoms** — Create, update, delete operations
- **Scope context** — React context for scope management
- **Plugin contracts** — Interface for source/secret provider plugins

### Key Responsibilities

1. **Component Library** — Reusable UI components
2. **API Integration** — Effect Atom reactive queries/mutations
3. **Code Display** — Syntax-highlighted code blocks
4. **Markdown Rendering** — Streamdown-based markdown parsing
5. **Plugin Architecture** — Extensible source/secret provider UI

---

## 2. File Inventory

### UI Package (62 files)

#### Core Utilities (3 files)
| # | File | Lines | Description |
|---|------|-------|-------------|
| 1 | `src/lib/utils.ts` | 6 | `cn()` class name merger |
| 2 | `src/lib/shiki.ts` | 189 | Shiki syntax highlighting |
| 3 | `src/hooks/use-mobile.ts` | 20 | Mobile breakpoint detection |

#### Components (59 files)
| Component | Purpose |
|-----------|---------|
| `accordion.tsx` | Collapsible content panels |
| `alert-dialog.tsx` | Confirmation dialogs |
| `alert.tsx` | Status alerts |
| `avatar.tsx` | User avatar with fallback |
| `badge.tsx` | Status badges |
| `breadcrumb.tsx` | Navigation breadcrumbs |
| `button.tsx` | Button with variants |
| `calendar.tsx` | Date picker calendar |
| `card.tsx` | Content cards |
| `carousel.tsx` | Image/content carousel |
| `chart.tsx` | Recharts charts |
| `checkbox.tsx` | Checkbox inputs |
| `code-block.tsx` | Syntax-highlighted code |
| `collapsible.tsx` | Toggle content |
| `combobox.tsx` | Searchable dropdown |
| `command.tsx` | Command palette |
| `context-menu.tsx` | Right-click menus |
| `dialog.tsx` | Modal dialogs |
| `drawer.tsx` | Slide-out drawer |
| `dropdown-menu.tsx` | Dropdown menus |
| `empty.tsx` | Empty state placeholder |
| `field.tsx` | Form field wrapper |
| `form.tsx` | Form container |
| `hover-card.tsx` | Hover popups |
| `input.tsx` | Text inputs |
| `input-group.tsx` | Grouped inputs |
| `input-otp.tsx` | OTP input |
| `item.tsx` | List items |
| `kbd.tsx` | Keyboard shortcuts |
| `label.tsx` | Form labels |
| `markdown.tsx` | Markdown rendering |
| `menubar.tsx` | Menu bar |
| `native-select.tsx` | Native select |
| `navigation-menu.tsx` | Nav navigation |
| `pagination.tsx` | Pagination controls |
| `popover.tsx` | Popover menus |
| `progress.tsx` | Progress indicators |
| `radio-group.tsx` | Radio buttons |
| `resizable.tsx` | Resizable panels |
| `scroll-area.tsx` | Scroll containers |
| `select.tsx` | Custom select |
| `separator.tsx` | Dividers |
| `sheet.tsx` | Side sheets |
| `sidebar.tsx` | App sidebar |
| `skeleton.tsx` | Loading skeletons |
| `slider.tsx` | Range sliders |
| `sonner.tsx` | Toast notifications |
| `spinner.tsx` | Loading spinners |
| `switch.tsx` | Toggle switches |
| `table.tsx` | Data tables |
| `tabs.tsx` | Tab panels |
| `textarea.tsx` | Text areas |
| `toggle.tsx` | Toggle buttons |
| `toggle-group.tsx` | Toggle groups |
| `tooltip.tsx` | Tooltips |

### React Client Package (7 files)

| # | File | Lines | Description |
|---|------|-------|-------------|
| 1 | `src/client.ts` | 21 | AtomHttpApi client |
| 2 | `src/atoms.ts` | 79 | Query/mutation atoms |
| 3 | `src/atoms.test.ts` | — | Atom tests |
| 4 | `src/scope-context.tsx` | 56 | Scope React context |
| 5 | `src/provider.tsx` | 10 | Root provider |
| 6 | `src/secret-picker.tsx` | 137 | Secret selection UI |
| 7 | `src/source-plugin.ts` | 79 | Source plugin contract |
| 8 | `src/secret-provider-plugin.ts` | 27 | Secret provider contract |
| 9 | `src/use-scope.ts` | 2 | Scope hook export |
| 10 | `src/base-url.ts` | 13 | Base URL config |
| 11 | `src/index.ts` | 55 | Public exports |

---

## 3. Key Exports

### UI Package

```typescript
// lib/utils.ts
export function cn(...inputs: ClassValue[]): string;

// lib/shiki.ts
export const THEME = "vitesse-dark";
export function getHighlighter(): Promise<HighlighterCore>;
export function createCodeHighlighterPlugin(): CodeHighlighterPlugin;
export function resolveLang(lang: string): SupportedLang | null;
export function isSupportedLang(lang: string): boolean;

// components/code-block.tsx
export function CodeBlock(props: {
  code: string;
  lang?: string;
  title?: string;
  maxHeight?: string;
  className?: string;
}): JSX.Element;

// components/markdown.tsx
export function Markdown(props: {
  children: string;
  className?: string;
}): JSX.Element;

// components/button.tsx
export function Button(props: {
  variant?: "default" | "destructive" | "outline" | "secondary" | "ghost" | "link";
  size?: "default" | "xs" | "sm" | "lg" | "icon" | "icon-xs" | "icon-sm" | "icon-lg";
  asChild?: boolean;
}): JSX.Element;
export const buttonVariants: CVA;
```

### React Client Package

```typescript
// client.ts
export class ExecutorApiClient extends AtomHttpApi.Tag<ExecutorApiClient>()("ExecutorApiClient", {
  api: ExecutorApi,
  httpClient: FetchHttpClient.layer,
  baseUrl: getBaseUrl(),
});

// atoms.ts
export const scopeAtom: Atom<Result<ScopeInfo>>;
export const toolsAtom: (scopeId: ScopeId) => Atom<Result<Tool[]>>;
export const sourcesAtom: (scopeId: ScopeId) => Atom<Result<Source[]>>;
export const secretsAtom: (scopeId: ScopeId) => Atom<Result<Secret[]>>;
export const toolSchemaAtom: (scopeId, toolId) => Atom<Result<ToolSchema>>;

export const setSecret: MutationAtom;
export const removeSecret: MutationAtom;
export const removeSource: MutationAtom;
export const refreshSource: MutationAtom;
export const detectSource: MutationAtom;

// scope-context.tsx
export function ScopeProvider(props: PropsWithChildren): JSX.Element | null;
export function useScope(): ScopeId;
export function useScopeInfo(): ScopeInfo;

// provider.tsx
export function ExecutorProvider(props: PropsWithChildren): JSX.Element;

// secret-picker.tsx
export function SecretPicker(props: {
  value: string | null;
  onSelect: (secretId: string) => void;
  secrets: readonly SecretPickerSecret[];
  placeholder?: string;
}): JSX.Element;

// source-plugin.ts
export interface SourcePlugin {
  key: string;
  label: string;
  add: ComponentType<{ onComplete, onCancel, initialUrl?, initialPreset? }>;
  edit: ComponentType<{ sourceId, onSave }>;
  summary?: ComponentType<{ sourceId }>;
  presets?: readonly SourcePreset[];
}

// secret-provider-plugin.ts
export interface SecretProviderPlugin {
  key: string;
  label: string;
  settings: ComponentType<Record<string, never>>;
}
```

---

## 4. Line-by-Line Analysis

### Class Name Merger (`utils.ts:1-6`)

```typescript
import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
```

**Key patterns:**
1. **clsx** — Conditional class names
2. **tailwind-merge** — Resolve Tailwind conflicts
3. **Spread inputs** — Accept multiple class sources

### Shiki Highlighter (`shiki.ts:107-136`)

```typescript
const jsEngine = createJavaScriptRegexEngine({ forgiving: true });

let _promise: Promise<HighlighterCore> | null = null;

export function getHighlighter(): Promise<HighlighterCore> {
  if (!_promise) {
    _promise = createHighlighterCore({
      themes: [import("@shikijs/themes/vitesse-dark")],
      langs: Object.values(LANG_LOADERS).map((loader) => loader()),
      engine: jsEngine,
    });
  }
  return _promise;
}
```

**Key patterns:**
1. **Lazy singleton** — Created on first use
2. **JavaScript regex engine** — No WASM dependency
3. **Dynamic language loading** — Only bundle needed grammars

### Streamdown Plugin (`shiki.ts:147-188`)

```typescript
export function createCodeHighlighterPlugin(): CodeHighlighterPlugin {
  return {
    name: "shiki" as const,
    type: "code-highlighter" as const,
    getSupportedLanguages: () => [...SUPPORTED_LANGS] as string[] as never,
    getThemes: () => [THEME as ThemeInput, THEME as ThemeInput],
    supportsLanguage: (language: string) => isSupportedLang(language),
    highlight(options, callback) {
      const resolved = resolveLang(options.language);
      const lang = resolved ?? "json";
      const key = `${lang}:${options.code.length}:${options.code.slice(0, 128)}`;

      // Check cache
      const cached = tokensCache.get(key);
      if (cached) return cached as never;

      // Async highlight with callback support
      void getHighlighter().then((highlighter) => {
        const result = highlighter.codeToTokens(options.code, {
          lang,
          themes: { light: THEME, dark: THEME },
        });
        tokensCache.set(key, result);
        pendingCallbacks.get(key)?.forEach((cb) => cb(result));
      });

      return null; // Async, result via callback
    },
  };
}
```

**Key patterns:**
1. **Cache key** — Language + length + prefix
2. **Callback pattern** — Async result delivery
3. **Pending callbacks** — Multiple requests for same code

### Executor API Client (`client.ts:11-18`)

```typescript
class ExecutorApiClient extends AtomHttpApi.Tag<ExecutorApiClient>()(
  "ExecutorApiClient",
  {
    api: ExecutorApi,
    httpClient: FetchHttpClient.layer,
    baseUrl: getBaseUrl(),
  },
) {}
```

**Key patterns:**
1. **Tagged client** — Unique identifier for Atom registry
2. **API composition** — Uses ExecutorApi from @executor/api
3. **Fetch HTTP client** — Browser-native fetch
4. **Configurable base URL** — Environment-aware

### Query Atoms (`atoms.ts:18-54`)

```typescript
export const toolsAtom = (scopeId: ScopeId) =>
  ExecutorApiClient.query("tools", "list", {
    path: { scopeId },
    timeToLive: "30 seconds",
  });

export const sourceToolsAtom = (sourceId: string, scopeId: ScopeId) =>
  ExecutorApiClient.query("sources", "tools", {
    path: { scopeId, sourceId },
    timeToLive: "30 seconds",
  });

export const sourcesAtom = (scopeId: ScopeId) =>
  ExecutorApiClient.query("sources", "list", {
    path: { scopeId },
    timeToLive: "30 seconds",
  });

export const sourceAtom = (sourceId: string, scopeId: ScopeId) =>
  Atom.mapResult(
    sourcesAtom(scopeId),
    (sources) => sources.find((s) => s.id === sourceId) ?? null,
  );
```

**Key patterns:**
1. **Parameterized atoms** — Scope ID as argument
2. **TTL caching** — Auto-refresh after expiry
3. **Derived atoms** — `sourceAtom` from `sourcesAtom`

### Mutation Atoms (`atoms.ts:68-79`)

```typescript
export const setSecret = ExecutorApiClient.mutation("secrets", "set");
export const resolveSecret = ExecutorApiClient.mutation("secrets", "resolve");
export const removeSecret = ExecutorApiClient.mutation("secrets", "remove");
export const removeSource = ExecutorApiClient.mutation("sources", "remove");
export const refreshSource = ExecutorApiClient.mutation("sources", "refresh");
export const detectSource = ExecutorApiClient.mutation("sources", "detect");
```

**Key patterns:**
1. **Direct mutation** — No config needed
2. **Type inference** — Request/response types from API
3. **Atomic operations** — One mutation per action

### Scope Provider (`scope-context.tsx:18-31`)

```typescript
export function ScopeProvider(props: React.PropsWithChildren) {
  const result = useAtomValue(scopeAtom);

  if (Result.isSuccess(result)) {
    return (
      <ScopeContext.Provider value={result.value}>
        {props.children}
      </ScopeContext.Provider>
    );
  }

  // Loading or error — don't render children
  return null;
}
```

**Key patterns:**
1. **Effect Atom integration** — `useAtomValue` for reactive value
2. **Result type** — Handle loading/error states
3. **Conditional rendering** — Only render on success

### Scope Hook (`scope-context.tsx:37-43`)

```typescript
export function useScope(): ScopeId {
  const scope = React.useContext(ScopeContext);
  if (scope === null) {
    throw new Error("useScope must be used inside a ScopeProvider");
  }
  return scope.id;
}
```

**Key patterns:**
1. **Context consumption** — React context API
2. **Null check** — Ensure provider exists
3. **Type-safe return** — ScopeId type

### Secret Picker (`secret-picker.tsx:35-136`)

```typescript
export function SecretPicker(props: {
  readonly value: string | null;
  readonly onSelect: (secretId: string) => void;
  readonly secrets: readonly SecretPickerSecret[];
  readonly placeholder?: string;
}) {
  const { value, onSelect, secrets, placeholder = "Search secrets…" } = props;
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");

  const selected = secrets.find((secret) => secret.id === value) ?? null;

  // Group by provider
  const grouped = new Map<string, SecretPickerSecret[]>();
  for (const secret of secrets) {
    const key = providerLabel(secret.provider);
    // ...
  }

  return (
    <Popover open={open} onOpenChange={setOpen} modal={false}>
      <PopoverAnchor asChild>
        <Input
          value={open ? query : (selected ? selected.name : "")}
          onChange={(e) => { setQuery(e.target.value); if (!open) setOpen(true); }}
          onFocus={() => setOpen(true)}
          placeholder={placeholder}
        />
      </PopoverAnchor>
      <PopoverContent className="w-(--radix-popover-trigger-width) p-0">
        <Command shouldFilter={false}>
          <CommandList>
            {groups.map(([label, items]) => (
              <CommandGroup key={label} heading={label}>
                {items.map((secret) => (
                  <CommandItem key={secret.id} value={`${secret.name} ${secret.id}`}
                    onSelect={() => { onSelect(secret.id); setOpen(false); }}>
                    {secret.name}
                  </CommandItem>
                ))}
              </CommandGroup>
            ))}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}
```

**Key patterns:**
1. **Popover + Command** — Radix primitives
2. **Provider grouping** — Organize by secret provider
3. **Search filtering** — Client-side filtering
4. **Modal=false** — Allow outside clicks

### Code Block (`code-block.tsx:66-145`)

```typescript
export function CodeBlock(props: { code, lang, title, maxHeight, className }) {
  const { code, lang: langHint, title, className } = props;
  const [expanded, setExpanded] = useState(false);
  const [copied, setCopied] = useState(false);

  const language = useMemo(() => detectLanguage(code, langHint), [code, langHint]);
  const highlighted = useHighlighted(code, language);
  const lines = code.split("\n");
  const isLong = lines.length > 24;
  const maxH = !expanded && isLong ? props.maxHeight ?? "24rem" : undefined;

  return (
    <div className={cn("rounded-lg border bg-card/60", className)}>
      {title && (
        <div className="flex items-center justify-between border-b px-3 py-2">
          <span className="text-[11px] font-medium uppercase">{title}</span>
          <button onClick={handleCopy}>{copied ? <CheckIcon /> : <CopyIcon />}</button>
        </div>
      )}
      <div className="group relative">
        <div className="overflow-auto text-[12px]" style={maxH ? { maxHeight: maxH } : undefined}>
          {highlighted ?? <pre className="p-3 font-mono">{code}</pre>}
        </div>
        {isLong && !expanded && (
          <div className="absolute bottom-0 ... bg-gradient-to-t">
            <button onClick={() => setExpanded(true)}>
              Show all ({lines.length} lines)
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
```

**Key patterns:**
1. **Auto language detection** — Fallback for unknown languages
2. **Expandable content** — Truncate long code blocks
3. **Copy to clipboard** — One-click copy with feedback
4. **Gradient fade** — Visual indicator for truncation

### Markdown Prose (`markdown.tsx:26-70`)

```typescript
const PROSE_CLASSES = [
  "text-[13px] leading-relaxed text-muted-foreground",
  // paragraphs
  "[&_p]:mb-[0.4em] [&_p:last-child]:mb-0",
  // bold
  "[&_strong]:text-foreground [&_strong]:font-semibold",
  // inline code
  "[&_code]:font-mono [&_code]:text-xs [&_code]:bg-muted [&_code]:border [&_code]:border-border",
  // links
  "[&_a]:text-primary [&_a]:underline [&_a]:underline-offset-2",
  // lists
  "[&_ul]:pl-5 [&_ul]:my-1.5 [&_ol]:pl-5 [&_ol]:my-1.5",
  // tables
  "[&_th]:border [&_th]:border-border [&_th]:bg-muted [&_th]:font-semibold",
  // ... more styles
].join(" ");

export function Markdown(props: { children: string; className?: string }) {
  return (
    <div className={props.className ? `${PROSE_CLASSES} ${props.className}` : PROSE_CLASSES}>
      <Streamdown
        linkSafety={{ enabled: false }}
        components={{ pre: PreBlock as never }}
      >
        {props.children}
      </Streamdown>
    </div>
  );
}
```

**Key patterns:**
1. **Prose styling** — Comprehensive markdown styles
2. **Tailwind arbitrary variants** — `[&_element]` syntax
3. **Streamdown integration** — Custom pre handler for code blocks
4. **Link safety disabled** — Trust internal content

### Button Variants (`button.tsx:7-39`)

```typescript
const buttonVariants = cva(
  "inline-flex shrink-0 items-center justify-center gap-2 rounded-md text-sm font-medium transition-all outline-none focus-visible:border-ring focus-visible:ring-[3px] disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground hover:bg-primary/90",
        destructive: "bg-destructive text-white hover:bg-destructive/90",
        outline: "border bg-background shadow-xs hover:bg-accent",
        secondary: "bg-secondary text-secondary-foreground hover:bg-secondary/80",
        ghost: "hover:bg-accent hover:text-accent-foreground",
        link: "text-primary underline-offset-4 hover:underline",
      },
      size: {
        default: "h-9 px-4 py-2 has-[>svg]:px-3",
        xs: "h-6 gap-1 rounded-md px-2 text-xs",
        sm: "h-8 gap-1.5 rounded-md px-3",
        lg: "h-10 rounded-md px-6",
        icon: "size-9",
      },
    },
    defaultVariants: { variant: "default", size: "default" },
  }
);
```

**Key patterns:**
1. **CVA (class-variance-authority)** — Variant-based class composition
2. **Has selector** — `has-[>svg]` for conditional padding
3. **SVG sizing** — Auto-size child icons
4. **Focus ring** — Accessibility focus styling

---

## 5. Component Relationships

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         UI + React Client                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    UI Package (@executor/ui)                         │   │
│  │                                                                       │   │
│  │  lib/                                                                 │   │
│  │  ├── utils.ts → cn() for Tailwind class merging                     │   │
│  │  └── shiki.ts → Syntax highlighting with caching                    │   │
│  │                                                                       │   │
│  │  components/                                                          │   │
│  │  ├── code-block.tsx → Shiki-powered code display                    │   │
│  │  ├── markdown.tsx → Streamdown with prose styles                    │   │
│  │  ├── button.tsx → CVA variants                                      │   │
│  │  ├── input.tsx → Styled text input                                  │   │
│  │  └── [55 more components]                                            │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│                              ▼                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                React Client (@executor/clients/react)                │   │
│  │                                                                       │   │
│  │  ExecutorApiClient (AtomHttpApi)                                    │   │
│  │    │                                                                 │   │
│  │    ├──> Query Atoms                                                 │   │
│  │    │   ├── toolsAtom(scopeId)                                      │   │
│  │    │   ├── sourcesAtom(scopeId)                                    │   │
│  │    │   ├── secretsAtom(scopeId)                                    │   │
│  │    │   └── toolSchemaAtom(scopeId, toolId)                         │   │
│  │    │                                                                 │   │
│  │    └──> Mutation Atoms                                              │   │
│  │        ├── setSecret, removeSecret                                 │   │
│  │        ├── removeSource, refreshSource, detectSource               │   │
│  │                                                                       │   │
│  │  ScopeProvider                                                       │   │
│  │    ├── useScope() → ScopeId                                        │   │
│  │    └── useScopeInfo() → { id, name }                               │   │
│  │                                                                       │   │
│  │  Plugin Contracts                                                    │   │
│  │    ├── SourcePlugin → add, edit, summary, presets                  │   │
│  │    └── SecretProviderPlugin → settings                             │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 6. Data Flow

### Query Atom Flow

```
React Component calls useAtomValue(toolsAtom(scopeId))
    │
    ▼
┌─────────────────────────────┐
│  AtomHttpApi.query()        │
│  - Checks cache             │
│  - TTL: 30 seconds          │
└───────────┬─────────────────┘
            │
            ▼
    ┌───────┴───────┐
    │               │
  HIT             MISS
    │               │
    │               ▼
    │       ┌───────────────┐
    │       │ HTTP GET      │
    │       │ /v1/scopes/:id/tools
    │       └───────┬───────┘
    │               │
    │               ▼
    │       ┌───────────────┐
    │       │ Effect Schema │
    │       │  validation   │
    │       └───────┬───────┘
    │               │
    │               ▼
    │       ┌───────────────┐
    │       │ Store in      │
    │       │ Result atom   │
    │       └───────┬───────┘
    │               │
    └───────────────┘
            │
            ▼
    React Component renders
```

### Mutation Flow

```
React Component calls useAtomSet(setSecret)({ secretId, value })
    │
    ▼
┌─────────────────────────────┐
│  AtomHttpApi.mutation()     │
│  - No cache (always POST)  │
└───────────┬─────────────────┘
            │
            ▼
    ┌───────────────┐
    │ HTTP POST     │
    │ /v1/secrets   │
    └───────┬───────┘
            │
            ▼
    ┌───────────────┐
    │ Invalidate    │
    │ related atoms │
    │ (secretsAtom) │
    └───────┬───────┘
            │
            ▼
    React components
    using secretsAtom
    auto-refresh
```

### Scope Provider Flow

```
App Root renders <ExecutorProvider>
    │
    ▼
┌─────────────────────────────┐
│  <RegistryProvider>         │
│  (Effect Atom registry)     │
└───────────┬─────────────────┘
            │
            ▼
┌─────────────────────────────┐
│  <ScopeProvider>            │
│    - useAtomValue(scopeAtom)│
│    - Fetches scope info    │
└───────────┬─────────────────┘
            │
            ▼
    ┌───────┴───────┐
    │               │
  Loading         Success
    │               │
    │               ▼
    │         <ScopeContext.Provider
    │           value={scopeInfo}>
    │         {children}
    │         </Provider>
    │
    ▼
render null    Children can now:
                - useScope() → ScopeId
                - useScopeInfo() → {id, name}
```

---

## 7. Key Patterns

### Effect Atom Integration

```typescript
// Query atom with TTL
export const toolsAtom = (scopeId: ScopeId) =>
  ExecutorApiClient.query("tools", "list", {
    path: { scopeId },
    timeToLive: "30 seconds",
  });

// Derived atom
export const sourceAtom = (sourceId, scopeId) =>
  Atom.mapResult(
    sourcesAtom(scopeId),
    (sources) => sources.find((s) => s.id === sourceId) ?? null,
  );
```

**Benefits:**
1. **Automatic caching** — TTL-based invalidation
2. **Dependency tracking** — Auto-refresh on change
3. **Type safety** — Inferred from API schema

### Singleton Highlighter

```typescript
let _promise: Promise<HighlighterCore> | null = null;

export function getHighlighter(): Promise<HighlighterCore> {
  if (!_promise) {
    _promise = createHighlighterCore({ ... });
  }
  return _promise;
}
```

**Benefits:**
1. **Lazy initialization** — Only created when needed
2. **Shared instance** — Single highlighter for app
3. **Promise memoization** — Avoid duplicate creation

### Language Detection

```typescript
function detectLanguage(code: string, hint?: string): string {
  if (hint) return resolveLang(hint) ?? "json";
  const trimmed = code.trimStart();
  if (trimmed.startsWith("{") || trimmed.startsWith("[")) return "json";
  if (trimmed.startsWith("<")) return "xml";
  if (trimmed.startsWith("---")) return "yaml";
  return "json";
}
```

**Benefits:**
1. **Hint priority** — Use provided hint if valid
2. **Content detection** — Infer from code content
3. **Safe fallback** — Default to JSON

### Plugin Contract Pattern

```typescript
export interface SourcePlugin {
  key: string;         // Matches SDK plugin key
  label: string;       // Display name
  add: ComponentType<...>;  // Add flow
  edit: ComponentType<...>; // Edit flow
  summary?: ComponentType<...>;
  presets?: readonly SourcePreset[];
}
```

**Benefits:**
1. **Type safety** — TypeScript enforces contract
2. **Separation of concerns** — Shell vs plugin ownership
3. **Extensibility** — Easy to add new plugins

### CVA for Variants

```typescript
const buttonVariants = cva("base-classes", {
  variants: {
    variant: { default: "...", destructive: "..." },
    size: { default: "...", xs: "...", sm: "..." },
  },
  defaultVariants: { variant: "default", size: "default" },
});
```

**Benefits:**
1. **Variant composition** — Mix and match variants
2. **Type inference** — TypeScript knows valid variants
3. **Centralized styles** — Single source of truth

---

## 8. Integration Points

### UI Package Dependencies

| Package | Purpose |
|---------|---------|
| `class-variance-authority` | Variant composition |
| `clsx` | Conditional classes |
| `tailwind-merge` | Tailwind conflict resolution |
| `shiki` | Syntax highlighting |
| `streamdown` | Markdown parsing |
| `hast-util-to-jsx-runtime` | HAST to JSX conversion |

### React Client Dependencies

| Package | Purpose |
|---------|---------|
| `@effect-atom/atom-react` | Effect Atom integration |
| `@effect/platform` | HTTP client |
| `@executor/api` | API definitions |
| `@executor/sdk` | SDK types |
| `@executor/ui` | UI components |

### Dependents

| Package | Relationship |
|---------|-------------|
| `@executor/apps/web` | Main web application |
| `@executor/apps/desktop` | Desktop application |
| Plugin packages | Source/secret provider UI |

---

## 9. Error Handling

### Scope Provider Error Handling

```typescript
export function ScopeProvider(props: PropsWithChildren) {
  const result = useAtomValue(scopeAtom);

  if (Result.isSuccess(result)) {
    return <ScopeContext.Provider value={result.value}>{props.children}</ScopeContext.Provider>;
  }

  // Loading or error — don't render children
  return null;
}
```

**Strategy:** Graceful loading state, hide children until ready.

### Fallback Rendering

```typescript
{highlighted ?? (
  <pre className="p-3 font-mono text-[12px] text-foreground/60">
    {code}
  </pre>
)}
```

**Strategy:** Show unhighlighted code if highlighting fails.

---

## 10. Testing Strategy

### Component Tests

UI components are tested with:
- **Vitest** — Unit test runner
- **React Testing Library** — Component testing
- **Mock providers** — Mock Effect Atom context

### Atom Tests

```typescript
// atoms.test.ts
describe("toolsAtom", () => {
  it("should fetch tools for scope", async () => {
    // Mock HTTP response
    // Verify atom returns correct data
  });
});
```

---

## 11. Design Decisions

### Why Effect Atom?

1. **Reactive** — Auto-refresh on data changes
2. **Type-safe** — End-to-end type inference from API
3. **Caching** — Built-in TTL and invalidation
4. **Effect integration** — Uses Effect Schema for validation

### Why Streamdown for Markdown?

1. **React-native** — Built for React, no DOM manipulation
2. **Customizable** — Component overrides for code blocks
3. **Lightweight** — Smaller than full markdown parsers

### Why Shiki for Syntax Highlighting?

1. **VSCode accuracy** — Same grammar as VSCode
2. **Theme support** — Consistent with editor
3. **JavaScript engine** — No WASM overhead

### Why Plugin Contracts?

1. **Separation** — Shell owns layout, plugins own flows
2. **Type safety** — TypeScript enforces interface
3. **Extensibility** — Easy to add new source types

---

## 12. Summary

The UI and React Client packages provide:

1. **Component Library** — 59 reusable React components
2. **Syntax Highlighting** — Shiki-based code display
3. **Markdown Rendering** — Streamdown with prose styles
4. **Reactive Client** — Effect Atom queries and mutations
5. **Scope Management** — React context for scope
6. **Plugin Architecture** — Source/secret provider contracts

The UI layer enables **consistent, accessible interfaces** while the React client provides **reactive, type-safe API integration**.
