# Inngest SDKs Deep Dive

## Overview

Inngest provides SDKs for TypeScript, Python, Go, and Kotlin. This deep dive covers the Go SDK (`inngestgo/`) and Rust SDK (`inngest-rs/`) as they provide the clearest view of the SDK architecture.

---

## Go SDK (`inngestgo/`)

**Location**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.inngest/inngestgo`

### Directory Structure

```
inngestgo/
├── inngest/          # Core SDK types
├── macros/           # Helper macros
├── step/             # Step tool implementations
├── internal/         # Internal utilities
│   ├── sdkrequest/   # SDK request handling
│   └── types/        # Type utilities
├── errors/           # Error types
│   └── errors.go     # NoRetry, RetryAt errors
├── examples/         # Example applications
├── tests/            # SDK tests
├── handler.go        # HTTP handler
├── funcs.go          # Function definitions
├── event.go          # Event types
├── signature.go      # Request signing
└── README.md         # Documentation
```

### Core Types

**Handler** (`handler.go`):
```go
type Handler interface {
    http.Handler
    SetAppName(name string) Handler
    SetOptions(h HandlerOpts) Handler
    Register(...ServableFunction)
}

type HandlerOpts struct {
    Logger             *slog.Logger
    SigningKey         *string
    SigningKeyFallback *string
    Env                *string
    RegisterURL        *string
    MaxBodySize        int
    URL                *url.URL
    UseStreaming       bool
    AllowInBandSync    *bool
    Dev                *bool
}
```

**Event** (`event.go`):
```go
type Event struct {
    ID        *string         `json:"id,omitempty"`
    Name      string          `json:"name"`
    Data      map[string]any  `json:"data"`
    User      any             `json:"user,omitempty"`
    Timestamp int64           `json:"ts,omitempty"`
    Version   string          `json:"v,omitempty"`
}

func (e Event) Validate() error {
    if e.Name == "" {
        return fmt.Errorf("event name must be present")
    }
    if len(e.Data) == 0 {
        return fmt.Errorf("event data must be present")
    }
    return nil
}
```

**Generic Event** (type-safe):
```go
type GenericEvent[DATA any, USER any] struct {
    ID        *string `json:"id,omitempty"`
    Name      string  `json:"name"`
    Data      DATA    `json:"data"`
    User      USER    `json:"user,omitempty"`
    Timestamp int64   `json:"ts,omitempty"`
    Version   string  `json:"v,omitempty"`
}
```

### Function Registration

**ServableFunction**:
```go
type ServableFunction interface {
    Slug() string
    Name() string
    Config() FunctionConfig
    Trigger() Trigger
    Func() any
    ZeroEvent() any
}
```

**Function Configuration**:
```go
type FunctionConfig struct {
    ID           string
    Name         string
    Retries      *int
    Concurrency  []ConcurrencyLimit
    Cancel       []Cancel
    RateLimit    *RateLimit
    Debounce     *Debounce
    BatchEvents  *BatchEvents
    Timeouts     *Timeouts
    Throttle     *Throttle
    Idempotency  *string
    Priority     *Priority
}
```

**Create Function**:
```go
func CreateFunction[EVENT any](
    opts FunctionOpts,
    trigger EventTrigger[EVENT],
    fn func(context.Context, Input[EVENT]) (any, error),
) *standardFunction[EVENT]
```

### Step Tools (`step/`)

**Step Context Management**:
```go
// step.go
type ctxKey string

const (
    targetStepIDKey = ctxKey("stepID")
    ParallelKey     = ctxKey("parallelKey")
)

func SetTargetStepID(ctx context.Context, id string) context.Context {
    if id == "" || id == "step" {
        return ctx
    }
    return context.WithValue(ctx, targetStepIDKey, id)
}

func getTargetStepID(ctx context.Context) *string {
    if v := ctx.Value(targetStepIDKey); v != nil {
        if c, ok := v.(string); ok {
            return &c
        }
    }
    return nil
}
```

**Preflight Check**:
```go
func preflight(ctx context.Context) sdkrequest.InvocationManager {
    if ctx.Err() != nil {
        // Another tool has already ran and context is closed
        panic(ControlHijack{})
    }
    mgr, ok := sdkrequest.Manager(ctx)
    if !ok {
        panic(ErrNotInFunction)
    }
    return mgr
}
```

**Run Step**:
```go
func Run[T any](ctx context.Context, id string, f func(ctx context.Context) (T, error)) (T, error) {
    mgr := preflight(ctx)
    hashedID := hashID(id)

    // Check if step already completed
    if val, ok := mgr.GetStepResult(hashedID); ok {
        return unmarshalResult[T](val)
    }

    // Check if this is the target step
    if targetID := getTargetStepID(ctx); targetID != nil && *targetID == hashedID {
        result, err := f(ctx)
        mgr.SetStepResult(hashedID, result, err)
        return result, err
    }

    // Yield opcode for this step
    mgr.Yield(state.GeneratorOpcode{
        Op: enums.OpcodeStepRun,
        ID: hashedID,
    })
    panic(ControlHijack{})
}
```

**Sleep**:
```go
func Sleep(ctx context.Context, id string, d time.Duration) {
    mgr := preflight(ctx)
    hashedID := hashID(id)

    if _, ok := mgr.GetStepResult(hashedID); ok {
        return  // Already slept
    }

    if targetID := getTargetStepID(ctx); targetID != nil && *targetID == hashedID {
        return  // Target step, just return
    }

    mgr.Yield(state.GeneratorOpcode{
        Op:   enums.OpcodeSleep,
        ID:   hashedID,
        Opts: map[string]any{"duration": d.String()},
    })
    panic(ControlHijack{})
}
```

**WaitForEvent**:
```go
func WaitForEvent[T any](
    ctx context.Context,
    id string,
    opts WaitForEventOpts,
) (*T, error) {
    mgr := preflight(ctx)
    hashedID := hashID(id)

    // Check if event already received
    if val, ok := mgr.GetStepResult(hashedID); ok {
        if val == nil {
            return nil, ErrEventNotReceived
        }
        return unmarshalEvent[T](val)
    }

    // Check if this is the target step
    if targetID := getTargetStepID(ctx); targetID != nil && *targetID == hashedID {
        return nil, nil  // Will be populated by executor
    }

    mgr.Yield(state.GeneratorOpcode{
        Op:   enums.OpcodeWaitForEvent,
        ID:   hashedID,
        Opts: map[string]any{
            "event":   opts.Event,
            "timeout": opts.Timeout.String(),
            "if":      opts.If,
        },
    })
    panic(ControlHijack{})
}
```

### Request/Response Handling

**SDK Request** (`internal/sdkrequest/`):
```go
type Request struct {
    CallCtx CallCtx                 `json:"ctx"`
    Event   json.RawMessage         `json:"event"`
    Events  []json.RawMessage       `json:"events"`
    Steps   map[string]*json.RawMessage `json:"steps"`
    UseAPI  bool                    `json:"use_api"`
    Version int                     `json:"version"`
}

type CallCtx struct {
    Env        string `json:"env"`
    FunctionID string `json:"fnId"`
    RunID      string `json:"runId"`
    StepID     string `json:"stepId"`
    Attempt    int    `json:"attempt"`
}
```

**Invocation Manager**:
```go
type InvocationManager interface {
    GetStepResult(id string) (any, bool)
    SetStepResult(id string, result any, err error)
    Yield(opcode state.GeneratorOpcode)
    Err() error
    Ops() []state.GeneratorOpcode
}
```

### HTTP Handler Flow

**ServeHTTP** (`handler.go`):
```go
func (h *handler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
    switch r.Method {
    case http.MethodGet:
        h.inspect(w, r)  // SDK inspection
    case http.MethodPost:
        probe := r.URL.Query().Get("probe")
        if probe == "trust" {
            h.trust(w, r)  // Trust probe
        } else {
            h.invoke(w, r)  // Function invocation
        }
    case http.MethodPut:
        h.register(w, r)  // Function registration
    }
}
```

**Invoke Flow**:
```go
func (h *handler) invoke(w http.ResponseWriter, r *http.Request) error {
    // 1. Read and validate request body
    byt, _ := io.ReadAll(r.Body)

    // 2. Verify signature
    valid, _, _ := ValidateRequestSignature(ctx, sig, signingKey, byt, isDev)

    // 3. Parse request
    request := &sdkrequest.Request{}
    json.Unmarshal(byt, request)

    // 4. Find function
    fnID := r.URL.Query().Get("fnId")
    fn := h.findFunction(fnID)

    // 5. Extract target step ID
    stepID := r.URL.Query().Get("stepId")

    // 6. Invoke function
    resp, ops, err := invoke(ctx, fn, request, stepID)

    // 7. Handle response
    if len(ops) > 0 {
        w.WriteHeader(206)  // Partial content
        return json.NewEncoder(w).Encode(ops)
    }

    return json.NewEncoder(w).Encode(resp)
}
```

**Invoke Function**:
```go
func invoke(ctx context.Context, sf ServableFunction, input *sdkrequest.Request,
            stepID *string) (any, []state.GeneratorOpcode, error) {

    // Create cancellable context
    fCtx, cancel := context.WithCancel(context.Background())
    if stepID != nil {
        fCtx = step.SetTargetStepID(fCtx, *stepID)
    }

    // Create invocation manager
    mgr := sdkrequest.NewManager(cancel, input)
    fCtx = sdkrequest.SetManager(fCtx, mgr)

    // Build input with reflection
    fVal := reflect.ValueOf(sf.Func())
    inputVal := reflect.New(fVal.Type().In(1)).Elem()

    // Unmarshal event into input
    unmarshalEvent(input.Event, inputVal)

    // Call function
    var panickErr error
    defer func() {
        if r := recover(); r != nil {
            if _, ok := r.(step.ControlHijack); ok {
                return  // Expected
            }
            panickErr = fmt.Errorf("function panicked: %v", r)
        }
    }()

    res := fVal.Call([]reflect.Value{fCtx, inputVal})

    // Return response and opcodes
    return res[0].Interface(), mgr.Ops(), panickErr
}
```

### Error Types (`errors/errors.go`)

```go
// NoRetryError indicates permanent failure
type NoRetryError struct {
    Message string
}

func (e NoRetryError) Error() string { return e.Message }

// RetryAtError indicates retry after specific time
type RetryAtError struct {
    Message string
    RetryAt time.Time
}

func (e RetryAtError) Error() string { return e.Message }
func (e RetryAtError) RetryAfter() time.Time { return e.RetryAt }

// StepError wraps step execution errors
type StepError struct {
    Err error
}
```

### Signature Verification (`signature.go`)

```go
func ValidateRequestSignature(ctx context.Context, sigHeader string,
                              signingKey string, body []byte, isDev bool) (bool, string, error) {
    if isDev {
        return true, "dev-key", nil
    }

    // Parse signature header
    // Format: t=timestamp,v1=signature
    parts := strings.Split(sigHeader, ",")
    var timestamp, signature string
    for _, part := range parts {
        if strings.HasPrefix(part, "t=") {
            timestamp = strings.TrimPrefix(part, "t=")
        }
        if strings.HasPrefix(part, "v1=") {
            signature = strings.TrimPrefix(part, "v1=")
        }
    }

    // Create signed payload
    payload := fmt.Sprintf("%s.%s", timestamp, body)

    // Compute HMAC
    mac := hmac.New(sha256.New, []byte(signingKey))
    mac.Write([]byte(payload))
    expectedSig := hex.EncodeToString(mac.Sum(nil))

    // Constant-time comparison
    return hmac.Equal([]byte(signature), []byte(expectedSig)), signingKey, nil
}
```

---

## Rust SDK (`inngest-rs/`)

**Location**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.inngest/inngest-rs`

### Directory Structure

```
inngest-rs/
├── inngest/          # Core SDK
│   ├── src/
│   │   ├── lib.rs        # Client implementation
│   │   ├── function.rs   # Function definitions
│   │   ├── handler.rs    # HTTP handler
│   │   ├── step_tool.rs  # Step tools
│   │   ├── event.rs      # Event types
│   │   ├── result.rs     # Result types
│   │   ├── config.rs     # Configuration
│   │   ├── serve/        # Framework integrations
│   │   │   └── axum.rs   # Axum integration
│   │   └── sdk/          # SDK protocol
│   ├── examples/
│   │   ├── axum/         # Axum example
│   │   └── send_events/  # Event sending example
│   └── Cargo.toml
├── macros/           # Procedural macros
└── Cargo.toml        # Workspace config
```

### Core Types

**Inngest Client** (`lib.rs`):
```rust
#[derive(Clone)]
pub struct Inngest {
    app_id: String,
    api_origin: Option<String>,
    event_api_origin: Option<String>,
    event_key: Option<String>,
    env: Option<String>,
    is_dev: Option<bool>,
    http: reqwest::Client,
}

impl Inngest {
    pub fn new(app_id: &str) -> Self {
        Inngest {
            app_id: app_id.to_string(),
            api_origin: Config::api_origin(),
            event_api_origin: Config::event_api_origin(),
            event_key: Config::event_key(),
            env: Config::env(),
            is_dev: Config::is_dev(),
            http: reqwest::Client::new(),
        }
    }

    pub async fn send_event<T: InngestEvent>(&self, evt: &Event<T>) -> Result<(), DevError> {
        self.http
            .post("http://127.0.0.1:8288/e/test")
            .json(&evt)
            .send()
            .await
            .map(|_| ())
            .map_err(|err| DevError::Basic(format!("{}", err)))
    }
}
```

**Event** (`event.rs`):
```rust
pub trait InngestEvent: Serialize + for<'a> Deserialize<'a> + Debug + 'static {}
impl<T: Serialize + for<'a> Deserialize<'a> + Debug + 'static> InngestEvent for T {}

#[derive(Debug, Serialize, Deserialize)]
pub struct Event<T>
where
    T: 'static,
{
    pub id: Option<String>,
    pub name: String,
    pub data: T,
    pub timestamp: Option<i64>,
    pub version: Option<String>,
}

impl<T> Event<T>
where
    T: InngestEvent,
{
    pub fn new(name: &str, data: T) -> Self {
        Event {
            id: None,
            name: name.to_string(),
            data,
            timestamp: None,
            version: None,
        }
    }
}
```

**Function** (`function.rs`):
```rust
pub struct Input<T: 'static> {
    pub event: Event<T>,
    pub events: Vec<Event<T>>,
    pub ctx: InputCtx,
}

pub struct InputCtx {
    pub env: String,
    pub fn_id: String,
    pub run_id: String,
    pub step_id: String,
    pub attempt: u8,
}

pub struct FunctionOps {
    pub id: String,
    pub name: Option<String>,
    pub retries: u8,
}

pub struct ServableFn<T: 'static, E> {
    pub opts: FunctionOps,
    pub trigger: Trigger,
    pub func: Box<Func<T, E>>,
}

type Func<T, E> = dyn Fn(Input<T>, StepTool) -> BoxFuture<'static, Result<Value, E>>
    + Send + Sync + 'static;

pub fn create_function<T: 'static, E, F>(
    opts: FunctionOps,
    trigger: Trigger,
    func: impl Fn(Input<T>, StepTool) -> F + Send + Sync + 'static,
) -> ServableFn<T, E>
where
    F: Future<Output = Result<Value, E>> + Send + Sync + 'static,
{
    use futures::future::FutureExt;
    ServableFn {
        opts,
        trigger,
        func: Box::new(move |input, step| func(input, step).boxed()),
    }
}
```

**Triggers**:
```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Trigger {
    EventTrigger {
        event: String,
        expression: Option<String>,
    },
    CronTrigger {
        cron: String,
    },
}
```

### Step Tools (`step_tool.rs`)

**State Management**:
```rust
struct InnerState {
    app_id: String,
    state: HashMap<String, Option<Value>>,
    indices: HashMap<String, u64>,
    genop: Vec<GeneratorOpCode>,
    error: Option<StepError>,
}

impl InnerState {
    fn new_op(&mut self, id: &str) -> Op {
        let pos = self.indices.entry(id.to_string()).or_insert(0);
        *pos += 1;
        Op { id: id.to_string(), pos: *pos }
    }
}

#[derive(Clone)]
pub struct State {
    inner: Arc<RwLock<InnerState>>,
}
```

**Step Tool**:
```rust
#[derive(Clone)]
pub struct Step {
    state: State,
}

impl Step {
    pub fn new(app_id: impl Into<String>, state: &HashMap<String, Option<Value>>) -> Self {
        Step {
            state: State::new(app_id, state),
        }
    }

    pub async fn run<T, E, F>(&self, id: &str, f: impl FnOnce() -> F) -> Result<T, Error>
    where
        T: for<'a> Deserialize<'a> + Serialize,
        E: for<'a> UserProvidedError<'a>,
        F: Future<Output = Result<T, E>>,
    {
        let op = self.new_op(id);
        let hashed = op.hash();

        // Check if step already completed
        if let Some(Some(stored_value)) = self.remove(&hashed) {
            let run_result: StepRunResult<T, E> = serde_json::from_value(stored_value)?;
            match run_result {
                StepRunResult::Data(data) => return Ok(data),
                StepRunResult::Error(err) => return Err(err.into()),
            }
        }

        // Execute the function
        match f().await {
            Ok(result) => {
                let serialized = serde_json::to_value(&result)?;
                self.push_op(GeneratorOpCode {
                    op: Opcode::StepRun,
                    id: hashed,
                    name: id.to_string(),
                    display_name: id.to_string(),
                    data: serialized.into(),
                    opts: json!({}),
                });
                Err(Error::Interrupt(FlowControlError::step_generator()))
            }
            Err(err) => {
                let serialized_err = serde_json::to_value(&err)?;
                self.push_error(StepError {
                    name: "Step failed".to_string(),
                    message: err.to_string(),
                    stack: None,
                    data: Some(serialized_err),
                });
                Err(Error::Interrupt(FlowControlError::step_generator()))
            }
        }
    }

    pub fn sleep(&self, id: &str, dur: Duration) -> Result<(), Error> {
        let op = self.new_op(id);
        let hashed = op.hash();

        match self.get_hashed(&hashed) {
            Some(_) => Ok(()),  // Already slept
            None => {
                self.push_op(GeneratorOpCode {
                    op: Opcode::Sleep,
                    id: hashed,
                    name: id.to_string(),
                    data: None,
                    opts: json!({"duration": duration::to_string(dur)}),
                });
                Err(Error::Interrupt(FlowControlError::step_generator()))
            }
        }
    }

    pub fn wait_for_event<T: InngestEvent>(
        &self,
        id: &str,
        opts: WaitForEventOpts,
    ) -> Result<Option<Event<T>>, Error> {
        let op = self.new_op(id);
        let hashed = op.hash();

        match self.get_hashed(&hashed) {
            Some(evt) => match evt {
                None => Ok(None),
                Some(v) => {
                    let e = serde_json::from_value::<Event<T>>(v)?;
                    Ok(Some(e))
                }
            },
            None => {
                self.push_op(GeneratorOpCode {
                    op: Opcode::WaitForEvent,
                    id: hashed,
                    name: id.to_string(),
                    data: None,
                    opts: json!({
                        "event": &opts.event,
                        "timeout": duration::to_string(opts.timeout),
                    }),
                });
                Err(Error::Interrupt(FlowControlError::step_generator()))
            }
        }
    }

    pub fn invoke<T: for<'de> Deserialize<'de>>(
        &self,
        id: &str,
        opts: InvokeFunctionOpts,
    ) -> Result<T, Error> {
        let op = self.new_op(id);
        let hashed = op.hash();

        match self.get_hashed(&hashed) {
            Some(resp) => match resp {
                None => Err(Error::NoInvokeFunctionResponseError),
                Some(v) => Ok(serde_json::from_value::<T>(v)?),
            },
            None => {
                self.push_op(GeneratorOpCode {
                    op: Opcode::InvokeFunction,
                    id: hashed,
                    name: id.to_string(),
                    data: None,
                    opts: json!({
                        "function_id": &opts.function_id,
                        "payload": {"data": &opts.data},
                    }),
                });
                Err(Error::Interrupt(FlowControlError::step_generator()))
            }
        }
    }
}
```

**Op Code**:
```rust
#[derive(Serialize, Clone)]
enum Opcode {
    StepRun,
    Sleep,
    WaitForEvent,
    InvokeFunction,
}

#[derive(Serialize, Clone)]
struct GeneratorOpCode {
    op: Opcode,
    id: String,
    name: String,
    #[serde(rename(serialize = "displayName"))]
    display_name: String,
    data: Option<Value>,
    opts: Value,
}
```

**Op Hashing**:
```rust
struct Op {
    id: String,
    pos: u64,
}

impl Op {
    fn hash(&self) -> String {
        let key = if self.pos > 0 {
            format!("{}:{}", self.id, self.pos)
        } else {
            self.id.to_string()
        };
        let mut hasher = Sha1::new();
        hasher.update(key.as_bytes());
        let res = hasher.finalize();
        base16::encode_upper(res.as_slice())
    }
}
```

### Error Handling (`result.rs`)

```rust
#[derive(Debug)]
pub enum DevError {
    Basic(String),
    RetryAt(RetryAfterError),
    NoRetry(NonRetryableError),
}

pub type DevResult<T> = Result<T, DevError>;
pub type InngestResult<T> = Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Dev(DevError),
    NoInvokeFunctionResponseError,
    Interrupt(FlowControlError),
}

#[derive(Debug)]
pub struct FlowControlError {
    acknowledged: bool,
    pub variant: FlowControlVariant,
}

impl FlowControlError {
    pub(crate) fn step_generator() -> Self {
        FlowControlError {
            acknowledged: false,
            variant: FlowControlVariant::StepGenerator,
        }
    }

    pub(crate) fn acknowledge(&mut self) {
        self.acknowledged = true;
    }
}

impl Drop for FlowControlError {
    fn drop(&mut self) {
        if !self.acknowledged {
            if std::thread::panicking() {
                println!("Flow control error was not acknowledged");
            } else {
                panic!("Flow control error was not acknowledged");
            }
        }
    }
}
```

### HTTP Handler (`handler.rs`)

```rust
pub struct Handler<T: 'static, E> {
    inngest: Inngest,
    signing_key: Option<String>,
    serve_origin: Option<String>,
    serve_path: Option<String>,
    funcs: HashMap<String, ServableFn<T, E>>,
}

impl<T, E> Handler<T, E> {
    pub fn new(client: Inngest) -> Self {
        Handler {
            signing_key: Config::signing_key(),
            serve_origin: Config::serve_origin(),
            serve_path: Config::serve_path(),
            inngest: client.clone(),
            funcs: HashMap::new(),
        }
    }

    pub fn register_fn(&mut self, func: ServableFn<T, E>) {
        self.funcs.insert(func.slug(), func);
    }

    pub async fn sync(&self, headers: &HashMap<String, String>,
                      framework: &str) -> Result<(), String> {
        let functions: Vec<Function> = self.funcs.iter().map(|(_, f)| {
            let mut steps = HashMap::new();
            steps.insert("step".to_string(), Step {
                id: "step".to_string(),
                name: "step".to_string(),
                runtime: StepRuntime {
                    url: format!("http://127.0.0.1:3000/api/inngest?fnId={}&step=step", f.slug()),
                    method: "http".to_string(),
                },
                retries: StepRetry { attempts: 3 },
            });
            Function {
                id: f.slug(),
                name: f.slug(),
                triggers: vec![f.trigger()],
                steps,
            }
        }).collect();

        let req = Request {
            app_name: self.inngest.app_id.clone(),
            framework: framework.to_string(),
            functions,
            url: "http://127.0.0.1:3000/api/inngest".to_string(),
            ..Default::default()
        };

        reqwest::Client::new()
            .post("http://127.0.0.1:8288/fn/register")
            .json(&req)
            .send()
            .await
            .map(|_| ())
            .map_err(|_| "error registering".to_string())
    }

    pub async fn run(&self, query: RunQueryParams, body: &Value) -> Result<SdkResponse, Error>
    where
        T: for<'de> Deserialize<'de> + Debug,
        E: Into<Error>,
    {
        let data = serde_json::from_value::<RunRequestBody<T>>(body.clone())?;

        let Some(func) = self.funcs.get(&query.fn_id) else {
            return Err(basic_error!("no function registered as ID: {}", &query.fn_id));
        };

        let input = Input {
            event: data.event,
            events: data.events,
            ctx: InputCtx {
                env: data.ctx.env.clone(),
                fn_id: query.fn_id.clone(),
                run_id: data.ctx.run_id.clone(),
                step_id: "step".to_string(),
                attempt: data.ctx.attempt,
            },
        };

        let step_tool = StepTool::new(&self.inngest.app_id, &data.steps);

        match std::panic::catch_unwind(AssertUnwindSafe(|| (func.func)(input, step_tool.clone()))) {
            Ok(fut) => {
                match AssertUnwindSafe(fut).catch_unwind().await {
                    Ok(v) => match v {
                        Ok(v) => Ok(SdkResponse { status: 200, body: v }),
                        Err(err) => match err.into() {
                            Error::Interrupt(mut flow) => {
                                flow.acknowledge();
                                // Handle step generator response
                                // ...
                            }
                            other => Err(other),
                        },
                    },
                    Err(panic_err) => Ok(SdkResponse {
                        status: 500,
                        body: Value::String(format!("panic: {:?}", panic_err)),
                    }),
                }
            }
            Err(panic_err) => Ok(SdkResponse {
                status: 500,
                body: Value::String(format!("panic: {:?}", panic_err)),
            }),
        }
    }
}
```

### Axum Integration (`serve/axum.rs`)

```rust
pub async fn register<T, E>(
    State(handler): State<Arc<Handler<T, E>>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse
where
    T: for<'de> Deserialize<'de> + Debug,
    E: Into<Error>,
{
    let framework = headers
        .get("x-inngest-framework")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    match handler.sync(&headers, &framework).await {
        Ok(_) => StatusCode::OK,
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

pub async fn invoke<T, E>(
    State(handler): State<Arc<Handler<T, E>>>,
    Query(query): Query<RunQueryParams>,
    Json(body): Json<Value>,
) -> impl IntoResponse
where
    T: for<'de> Deserialize<'de> + Debug,
    E: Into<Error>,
{
    match handler.run(query, &body).await {
        Ok(response) => response.into_response(),
        Err(err) => err.into_response(),
    }
}
```

### Example Usage (`examples/axum/main.rs`)

```rust
#[tokio::main]
async fn main() {
    let client = Inngest::new("rust-dev");
    let mut inngest_handler = Handler::new(client);
    inngest_handler.register_fn(dummy_fn());
    inngest_handler.register_fn(step_run());

    let inngest_state = Arc::new(inngest_handler);

    let app = Router::new()
        .route("/api/inngest", put(serve::axum::register).post(serve::axum::invoke))
        .with_state(inngest_state);

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Serialize, Deserialize, Debug)]
struct TestData {
    name: String,
    data: u8,
}

fn dummy_fn() -> ServableFn<TestData, Error> {
    create_function(
        FunctionOps {
            id: "Dummy func".to_string(),
            ..Default::default()
        },
        Trigger::EventTrigger {
            event: "test/event".to_string(),
            expression: None,
        },
        |input: Input<TestData>, step: StepTool| async move {
            step.sleep("sleep-test", Duration::from_secs(3))?;

            let resp: Value = step.invoke(
                "test-invoke",
                InvokeFunctionOpts {
                    function_id: "other-fn".to_string(),
                    data: json!({ "name": "yolo", "data": 200 }),
                    timeout: None,
                },
            )?;

            let evt: Option<Event<Value>> = step.wait_for_event(
                "wait",
                WaitForEventOpts {
                    event: "test/wait".to_string(),
                    timeout: Duration::from_secs(60),
                    if_exp: None,
                },
            )?;

            Ok(json!({ "dummy": true }))
        },
    )
}
```

---

## Python SDK (`inngest-py/`)

**Location**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.inngest/inngest-py`

### Key Features

- Supports Flask, FastAPI, Django, Tornado, DigitalOcean Functions
- Both sync and async function support
- Step-based error handling

### Example

```python
@inngest_client.create_function(
    fn_id="fetch_ships",
    trigger=inngest.TriggerEvent(event="app/ships.find"),
)
def fetch_ships(ctx: inngest.Context, step: inngest.StepSync) -> dict:
    person_id = ctx.event.data["person_id"]

    person = step.run("fetch_person", lambda: requests.get(...).json())

    ship_names = []
    for ship_url in person["starships"]:
        ship = step.run("fetch_ship", lambda u: requests.get(u).json(), ship_url)
        ship_names.append(ship["name"])

    return {"person_name": person["name"], "ship_names": ship_names}
```

---

## Common Patterns Across SDKs

### 1. Step Position Hashing

All SDKs use the same hashing scheme:
```
hash = SHA1("step_id:position")
```

### 2. Control Flow Mechanism

- **Go**: Panic with `ControlHijack{}`
- **Rust**: Return `FlowControlError` with Drop guard
- **Python**: Raise exception internally

### 3. Response Codes

| Code | Meaning |
|------|---------|
| 200 | Function completed |
| 206 | Partial content (steps remaining) |
| 400 | Bad request |
| 401 | Unauthorized |
| 500 | Server error |

### 4. Registration Protocol

```json
{
  "url": "http://localhost:3000/api/inngest",
  "v": "1",
  "deployType": "ping",
  "sdk": "inngest-go:v1.0.0",
  "appName": "my-app",
  "functions": [...]
}
```
