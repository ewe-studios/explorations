# 17 -- RSJS Overview: Islands, Signals, Memos, Handlers, Queries

RSJS is a Rust-to-JavaScript compiler for interactive islands, but the deeper idea is that it is
not a runtime library at all. Every reactive type you touch in source -- `Signal<T>`, `Memo<T>`,
`Query<T>`, `Mutation<Args, Result>`, `NodeRef<T>` -- is a compile-time witness. On the server these
types are inert placeholders that carry an initial value and then do nothing; the `rsjs` compiler
recognises them by name, lowers their call sites to JavaScript, and the real behavior lives in the
generated client code plus the embedded `rsjs-runtime.js`. The author writes one ordinary Rust
`WebRenderSync`/`WebRender` impl, and RSJS produces both the server-rendered HTML and the small,
route-planned JavaScript that makes it interactive. An analyzed component with no reachable client
roots stays server-only and emits no island JavaScript at all.

## The Five Crates

RSJS is one conceptual system split across five workspace members. The split is deliberate: the
intermediate representation must be testable without a proc-macro expansion, and the proc macro must
not depend on the runtime in a way that creates a dependency cycle.

```mermaid
flowchart LR
    A[rsjs<br/>runtime + authoring surface] --> B[rsjs_macros<br/>#[rsjs] proc macro]
    B --> C[rsjs_ir<br/>side-table IR + JS emitter]
    A --> D[rsjs_web<br/>DOM/event facade]
    A --> E[rsjs_transport<br/>descriptor + wire records]
    C --> D
    B --> E
    A --> E
```

- **`rsjs`** (`rsjs/src/lib.rs`) is the crate applications depend on. It re-exports the proc-macro
  entry points (`#[rsjs(component...)]`, `#[rsjs::client]`, `#[derive(rsjs::ClientProps)]`,
  `#[derive(rsjs::SharedState)]`), defines the public authoring primitives (`signal`, `memo`,
  `handler`, `query_with_initial`, `mutation`, `node_ref`, the `console`/`window`/`document`/...
  capability modules), and owns the runtime manifest plus the embedded browser runtime sources.
- **`rsjs_ir`** (`rsjs_ir/`) is the side-table intermediate representation. It owns the identifier
  types, the constrained Client IR v3 expression enum, the reactive declarations (`SignalDecl`,
  `EventHandler`, `QueryDecl`, `MutationDecl`), the per-`NodeId` bindings tables, the `Island`
  container, optimization passes, and the JavaScript emitter. It is deliberately independent of
  `syn` and `proc_macro`.
- **`rsjs_macros`** (`rsjs_macros/`) is the proc-macro frontend. It lifts an `html_sync!`/`html!`
  parse tree, builds an `rsjs_ir` `Island` from it, rewrites the authored render body into ordinary
  server code, and emits the descriptor records that the linker consumes.
- **`rsjs_web`** (`rsjs_web/`) is a generated, zero-cost browser type facade: DOM marker types
  (`elem::Button`, `elem::Input`, ...), event-interface vocabulary, and the `currentTarget` handle.
- **`rsjs_transport`** (`rsjs_transport/`) is the wire format: versioned descriptor and definition
  records (`ComponentDefinitionRecord`, `HelperEdgeRecord`, `PayloadRecord`, ...) that carry
  component and helper edges through the staged build.

## The Island Is the Unit of Compilation

One struct plus one `#[rsjs(component...)]` render-trait impl lowers to exactly one `Island`
(`rsjs_ir/src/island.rs`). The `Island` is a bag of side-tables, not a tree of reactive nodes. It
owns vectors of `SignalDecl`s, `EventHandler`s, `QueryDecl`s, `MutationDecl`s, `NodeRefDecl`s, a
`Bindings` table keyed by `NodeId`, an `ExprArena` of Client IR v3 expressions, and a `ViewTree`
that supplies stable `NodeId`s for the bindings to key off of.

```rust
// rsjs_ir/src/island.rs (shape, abbreviated)
pub struct Island {
    pub shared_stores: Vec<SharedStoreDecl>,
    pub signals: Vec<SignalDecl>,      // signals[i].id == SignalId(i as u32)
    pub handlers: Vec<EventHandler>,
    pub callback_slots: Vec<CallbackSlot>,
    pub node_refs: Vec<NodeRefDecl>,
    pub bindings: Bindings,            // per-NodeId side-tables
    pub exprs: ExprArena,              // every reactive expression by ExprId
    pub view_tree: ViewTreeRef,        // stable NodeIds
    pub is_async: bool,
    pub hydration_strategy: HydrationStrategy,
    pub queries: Vec<QueryDecl>,
    pub mutations: Vec<MutationDecl>,
    pub intervals: Vec<IntervalDecl>,
    pub lists: Vec<ListDecl>,
}
```

The bindings are what connect the DOM to the reactive state. `text_bindings` say which text nodes
display which expression; `event_bindings` say which DOM events on which nodes invoke which handler;
`conditional_sections` hold `{ if cond { then } else { else } }` predicates; `input_bindings` pair a
form input with a `SignalId` for `bind_value`/`bind_checked`/`bind_selected`; `attr_bindings` pair a
dynamic attribute with an expression. Conditional rendering is a binding concern, not a structural
one: both branches are statically present in the view tree, and the runtime shows one and hides the
other.

```mermaid
flowchart TD
    SRC[one struct + one impl<br/>#[rsjs(component...)] WebRender] --> MACRO[rsjs_macros lifts html_sync!]
    MACRO --> IR[Island side-tables]
    IR --> SIG[signals / memos]
    IR --> HND[handlers]
    IR --> QRY[queries / mutations]
    IR --> BIND[Bindings by NodeId]
    IR --> ARENA[ExprArena Client IR v3]
    BIND --> ARENA
    HND --> ARENA
    IR --> OPT[optimize: HandlerDedup, DeadSignalElimination]
    OPT --> JS[emit_js per-island module]
    MACRO --> RUST[emit_rust rewritten server render]
    JS --> HYDRATE[hydrate at HydrationStrategy]
    RUST --> SSR[server-rendered HTML + seed blob]
```

## The Build Model

Deployable applications use the linked `cargo proa` lifecycle, not plain `cargo build`. The four
stages are described in the crate root (`rsjs/src/lib.rs`):

1. **Descriptor compilation** preserves typed SSR and emits component/helper edges plus latent client
   crossings. It runs as ordinary Rust type-checking with `#[used]` link sections carrying relocation
   records.
2. **Semantic linking** resolves Rust identities, reachability, final owners, and activation
   strategies.
3. **Final compilation** emits wrappers, fragments, serializers, client modules, and the exact
   manifest for the linked plan.
4. The **response planner** includes only the artifacts recorded by the rendered response and orders
   runtime assets and seed tables.

`cargo check` and rust-analyzer remain useful for ordinary typing; `cargo proa check` gives full
linked diagnostics and `cargo proa build` produces a deployable artifact.

## Authoring an Island

The user writes the struct and the trait impl explicitly; `#[rsjs(component)]` rewrites the render
body but does not generate the trait signature. `client` is added only when the component originates
writable state, handlers, queries, mutations, shared-state acquisition, node refs, timers, effects,
or browser capabilities.

```rust
// rsjs/src/lib.rs (crate-root example, abbreviated)
pub struct Counter {
    pub initial: i32,
}

#[rsjs(component, client)]
impl<L: DataLoader> WebRenderSync<L> for Counter {
    fn render(self, cx: &mut WebContext<L>) -> Result<(), WriteError> {
        let count = signal::<i32>(self.initial);
        let decrement = event_handler(
            |_: MouseEvent<elem::Button>| count.update(|value| value - 1),
        );
        let increment = event_handler(
            |_: MouseEvent<elem::Button>| count.update(|value| value + 1),
        );

        html_sync! {
            <section data-counter>
                <button type="button" on_click={decrement()}>"-"</button>
                <span>{count.get()}</span>
                <button type="button" on_click={increment()}>"+"</button>
            </section>
        }
        .render(cx)
    }
}
```

Sync vs async is the user's choice: `impl WebRenderSync<L>` for sync islands, or the real
`impl WebRender<L>` returning a `RenderOutcome<impl Future<...>>` for components that need to await.
There is no `rsjs!` body macro -- the body is a normal `html_sync!` or `html!` template.

## Signals and Memos

A `Signal<T>` is a reactive cell: `.get()` tracks the current effect as a subscriber, `.set()` and
`.update()` notify subscribers. `signal(initial)` constructs one. A `Memo<T>` is a read-only derived
cell that caches its value, derives its inputs from the closure's reads (no manual dependency array),
recomputes on a batched input write, and notifies downstream subscribers only when the result
changes. On the server both are placeholders: `signal` stores the initial value and `memo` runs the
closure once over initial values for SSR.

```rust
// rsjs/src/lib.rs (server-side placeholder semantics)
pub fn signal<T>(initial: T) -> Signal<T> {
    Signal { _value: initial }
}

pub fn memo<T, F>(f: F) -> Memo<T>
where
    F: Fn() -> T,
{
    Memo { _value: f() }
}

impl<T> Signal<T> {
    pub fn get(&self) -> T where T: Clone { self._value.clone() }
    pub fn set(&self, _value: T) {}           // no-op on the server
    pub fn update<F>(&self, _f: F) where F: FnOnce(T) -> T {}  // no-op on the server
}
```

In the IR a memo is not a separate type. `SignalDecl` carries a `derived: Option<ExprId>` field: a
plain leaf signal has `None`, a memo has `Some(expr)`. Derived cells share the `SignalId` space with
plain signals so every existing consumer -- dependency collection, binding dep arrays, `__sig_<id>`
JS naming, component-inline signal remapping -- slots into the same wiring unchanged. The seed-blob
writers emit a `null` placeholder at a derived cell's index, because a derived value is never seeded;
it is recomputed from the seeded source signals.

## Handlers

An `EventHandler` is a freestanding Client IR v3 block invoked by name from the runtime when the
bound DOM event fires. The declared handler stores its body as a `ClientBlock` and an optional
trailing-edge debounce window in milliseconds. The event surface is a closed list (`EventName` in
`rsjs_ir/src/bindings.rs`): `on_click`, `on_input`, `on_change`, `on_paste`, `on_focus`, `on_blur`,
`on_keydown`, `on_keyup`, the full mouse/pointer/touch/drag families, `on_submit`, the document-level
`on_document_click`/`on_document_keydown`/`on_document_pointerdown`, and the window-level
`on_window_popstate`/`on_window_hashchange`.

The authoring surface offers two forms. `rsjs::handler(|payload| ...)` returns the closure unchanged
and is invoked as `on_save(())`. `rsjs::event_handler(|event, payload| ...)` is the IDE-friendly form
for non-trivial handlers; a closure taking only the event becomes callable as `handler()`, and a
closure taking event plus payload becomes `handler(payload)`. Event types carry a marker for the
handler's `currentTarget`: `rsjs::MouseEvent<elem::Button>` for a local binding, or
`rsjs::web::dom::Document`/`Window` for global bindings.

## Queries and Mutations

A query is a reactive cache cell keyed by a structural array (`["stock", id]`). The server computes
the initial value during SSR and embeds it in the page; the client fetcher runs after hydration on
the configured policy.

```rust
// rsjs/src/lib.rs (authoring shape)
let q: rsjs::Query<User> = rsjs::query_with_initial(
    QueryKey::new(["user", id]),
    cx.load_json::<User>(LoadKey::typed("user", id)).await?,
    FetchRequest { url: "/api/user", response: Some(ResponseKind::Json), ..Default::default() },
    QueryOptions { interval_ms: None, refetch_on_focus: true, ..Default::default() },
);
// q.data(), q.is_loading(), q.error(), q.refetch(), q.invalidate(), q.data_updated_at()
```

`QueryOptions` carries six knobs: `interval_ms` (polling), `refetch_on_focus`, `download_progress`,
`stream_text`, `retry_count`, and `stale_time_ms`. A `Mutation<Args, Result>` is the imperative
counterpart: it posts to its `client_runner` URL with user-supplied args and, on success, walks its
`invalidates` list and refetches (or resets) every cache entry whose key prefix-matches. The
`InvalidationStrategy` enum has `Refetch` (default) and `Reset`. Like signals, `Query<T>` and its
accessors are server-side stubs -- the real fetch behavior lives in `rsjs-runtime-query.js`.

## Shared State

`#[derive(rsjs::SharedState)]` produces a typed page-scoped shared-state schema with an explicit,
namespaced protocol identity. `rsjs::shared::<T>()` acquires the nearest `SharedStateProvider<T>` for
the island; a `SharedStateProvider` does not render a DOM wrapper -- it records a scope id and a
latent seed in the request-local page registry, and the seed is published only when a committed
reachable consumer acquires the provider.

```rust
// rsjs/src/lib.rs (shared-state authoring)
#[derive(rsjs::SharedState)]
#[rsjs(type_key = "acme.workbench")]
pub struct WorkbenchState {
    pub select_mode: bool,
    pub button_bg: String,
}
```

The protocol identity is the explicit `type_key` plus a transitive `schema_hash()`, computed as
`sha256:` over the type key, the ABI version, and the full recursive `ClientPropsMeta` (prefixed
`rsjs.shared-state.v2`). The hash lets a parent that fully inlines a child component prove the child's
Rust type names still denote the same states after re-resolution: the `IslandSharedStores` trait
records each store's `TYPE_KEY`, and the parent emits a const comparison between trait-resolved items,
so the check needs no registry and no name matching.

## Client Helpers and Client Props

`#[rsjs::client]` marks a free function as a reusable client-side helper, or defines a closed
value-object module. Its body must conform to the constrained v0.1 `Expr` IR: literals, parameter
reads, binary operators, `.is_non_empty()`, and calls to other `#[rsjs::client]` helpers. The macro
replaces the function with a metadata struct plus a re-emitted Rust function that delegates to the
server-side evaluator, so the same code runs on the server (for SSR) and lowers to JavaScript (for
the browser). `#[rsjs::client_module]` groups helpers into one client-loadable bundle.

`#[derive(rsjs::ClientProps)]` generates `ClientEncode`/`ClientDecode` impls and codec metadata for a
struct or fieldless enum. The closed vocabulary is `bool`, `i32`, `u32`, `i64`, `u64`, `f32`, `f64`,
`String`, `Option<T>`, `Vec<T>`, and nested client-props structs. Fieldless enums use their exact
Rust variant identifiers as stable JSON string tags; explicit numeric discriminants do not cross the
boundary. Wide integers (`i64`/`u64`) always use JavaScript `BigInt`, crossing the hydration boundary
as canonical decimal strings.

## Browser Capabilities

The `rsjs` crate exposes a set of built-in client-side capability modules whose Rust functions are
no-op stubs and whose real behavior is the JS the analyzer emits. They include `console` (`log`,
`info`, `warn`, `error`, `table`, `time`/`time_end`, `group`/`group_end`), `window` (`inner_width`,
`scroll_to`, `print`), `navigator` (`language`), `location` and `history` (URL and history
navigation), `url` (query-string reads and writes), `document` (custom events, title), `root` (the
`<html>` element's class), `clipboard` (read/write text), `webauthn` (WebAuthn ceremonies), `encoding`
(UTF-8 and base64/base64url), and `local_storage`/`session_storage`.

## The Aha Moment

**Aha:** The reactive graph has no edges stored anywhere. Dependencies are derived data, never
source-of-truth IR.

A `SignalDecl` does not store its dependents, and an `EventHandler` does not store the signals it
reads or writes. There is no `dependencies: Vec<SignalId>` field anywhere. Dependency information is
computed on demand by analyses that walk the `ClientExpr` tree, and binding dependency arrays are
derived from that same walk. Treating dependencies as derived rather than stored keeps the IR cheaper
to construct, harder to get out of sync, and easier for passes to mutate -- the optimization passes
(`HandlerDedup`, `DeadSignalElimination`) can rewrite the IR in place without first repairing a
parallel edge list, because there is no edge list to repair. The single source of truth is the
expression tree; everything the runtime needs to know about who depends on whom falls out of walking
it. The same "derive it, do not store it" discipline shows up again in the memo/signal shared
`SignalId` space: a derived cell is just a `SignalDecl` with a `derived: Some(ExprId)` body, so no
parallel reactive graph is needed for memos either.

## Cross-References

- The intermediate representation that owns the side-tables, identifiers, and JavaScript emitter:
  [18-rsjs-ir.md](./18-rsjs-ir.md).
- The `#[rsjs]` proc macro that lifts templates, builds the IR, and rewrites the render body:
  [19-rsjs-macros.md](./19-rsjs-macros.md).
- The DOM/event facade and descriptor wire transport: [20-rsjs-web-transport.md](./20-rsjs-web-transport.md).
- The linked build pipeline that runs descriptor compile, semantic link, and final compile:
  [15-build-tooling.md](./15-build-tooling.md).
- The `proa_ir` core IR that precedes RSJS in the overall pipeline: [11-ir.md](./11-ir.md).

Continue to [18-rsjs-ir.md](./18-rsjs-ir.md).
