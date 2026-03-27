---
title: "Tea Executor Deep Dive: Program Loop, Commands, and Execution"
subtitle: "Complete guide to Bubble Tea's execution model, command system, and message handling"
based_on: "Bubble Tea - github.com/charmbracelet/bubbletea"
---

# Tea Executor Deep Dive

## Table of Contents

1. [Program Structure and Lifecycle](#1-program-structure-and-lifecycle)
2. [The Run Method](#2-the-run-method)
3. [Event Loop Internals](#3-event-loop-internals)
4. [Command Execution System](#4-command-execution-system)
5. [Batch and Sequence Commands](#5-batch-and-sequence-commands)
6. [Timer Commands: Tick and Every](#6-timer-commands-tick-and-every)
7. [Signal Handling](#7-signal-handling)
8. [Input Reading and Parsing](#8-input-reading-and-parsing)
9. [Terminal Management](#9-terminal-management)

---

## 1. Program Structure and Lifecycle

### 1.1 Program Structure

```go
// tea.go
type Program struct {
    // Initial model for the program
    initialModel Model

    // Message channel
    msgs chan Msg

    // Error channel
    errs chan error

    // Completion signal
    finished chan struct{}

    // Context for cancellation
    externalCtx context.Context
    ctx         context.Context
    cancel      context.CancelFunc

    // Configuration
    startupOptions startupOptions
    startupTitle   string
    inputType      inputType
    environ        []string

    // I/O
    output io.Writer
    ttyOutput term.File
    input    io.Reader
    ttyInput term.File

    // Terminal state
    previousOutputState   *term.State
    previousTtyInputState *term.State
    cancelReader          cancelreader.CancelReader

    // Renderer
    renderer renderer

    // Handlers for cleanup
    handlers channelHandlers

    // State tracking
    altScreenWasActive bool
    bpWasActive        bool
    reportFocus        bool
    ignoreSignals      uint32

    // Filter for messages
    filter func(Model, Msg) Msg

    // Framerate
    fps int

    // Windows mouse mode
    mouseMode bool
}
```

### 1.2 Program Lifecycle

```
┌─────────────────────────────────────────────────────────┐
│                   Program Lifecycle                      │
│                                                          │
│  1. NewProgram(Model, Options...) → *Program            │
│     - Create program structure                          │
│     - Apply options                                     │
│     - Initialize channels                               │
│                                                          │
│  2. Run() → (Model, error)                              │
│     - Initialize terminal (raw mode, alt screen)        │
│     - Start renderer                                    │
│     - Call Model.Init()                                 │
│     - Run event loop                                    │
│     - Handle signals (SIGINT, SIGTERM, SIGWINCH)        │
│                                                          │
│  3. Shutdown()                                          │
│     - Stop renderer                                     │
│     - Restore terminal state                            │
│     - Close channels                                    │
│     - Return final model                                │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### 1.3 Program Options

```go
// options.go

// WithAltScreen uses the alternate screen buffer
func WithAltScreen() ProgramOption {
    return func(p *Program) {
        p.startupOptions |= withAltScreen
    }
}

// WithMouseCellMotion enables mouse cell motion tracking
func WithMouseCellMotion() ProgramOption {
    return func(p *Program) {
        p.startupOptions |= withMouseCellMotion
    }
}

// WithMouseAllMotion enables mouse all motion tracking
func WithMouseAllMotion() ProgramOption {
    return func(p *Program) {
        p.startupOptions |= withMouseAllMotion
    }
}

// WithANSICompressor enables ANSI sequence compression
func WithANSICompressor() ProgramOption {
    return func(p *Program) {
        p.startupOptions |= withANSICompressor
    }
}

// WithoutSignalHandler disables signal handling
func WithoutSignalHandler() ProgramOption {
    return func(p *Program) {
        p.startupOptions |= withoutSignalHandler
    }
}

// WithoutCatchPanics disables panic recovery
func WithoutCatchPanics() ProgramOption {
    return func(p *Program) {
        p.startupOptions |= withoutCatchPanics
    }
}

// WithContext uses provided context instead of background
func WithContext(ctx context.Context) ProgramOption {
    return func(p *Program) {
        p.externalCtx = ctx
    }
}

// WithInput sets custom input reader
func WithInput(r io.Reader) ProgramOption {
    return func(p *Program) {
        p.input = r
        p.inputType = customInput
    }
}

// WithOutput sets custom output writer
func WithOutput(w io.Writer) ProgramOption {
    return func(p *Program) {
        p.output = w
    }
}

// WithFilter sets message filter function
func WithFilter(filter func(Model, Msg) Msg) ProgramOption {
    return func(p *Program) {
        p.filter = filter
    }
}

// WithFPS sets renderer framerate
func WithFPS(fps int) ProgramOption {
    return func(p *Program) {
        p.fps = fps
    }
}
```

---

## 2. The Run Method

### 2.1 Run Overview

```go
func (p *Program) Run() (returnModel Model, returnErr error) {
    // 1. Initialize channels
    p.handlers = channelHandlers{}
    cmds := make(chan Cmd)
    p.errs = make(chan error, 1)
    p.finished = make(chan struct{})

    defer func() {
        close(p.finished)
        p.cancel()
    }()

    // 2. Setup input
    switch p.inputType {
    case defaultInput:
        p.input = os.Stdin
        // Check if stdin is a terminal
        f, isFile := p.input.(term.File)
        if isFile && !term.IsTerminal(f.Fd()) {
            // Not a terminal, open TTY for input
            f, err := openInputTTY()
            if err != nil {
                return p.initialModel, err
            }
            defer f.Close()
            p.input = f
        }
    case ttyInput:
        f, err := openInputTTY()
        if err != nil {
            return p.initialModel, err
        }
        defer f.Close()
        p.input = f
    case customInput:
        // Use provided input
    }

    // 3. Setup signal handling
    if !p.startupOptions.has(withoutSignalHandler) {
        p.handlers.add(p.handleSignals())
    }

    // 4. Setup panic recovery
    if !p.startupOptions.has(withoutCatchPanics) {
        defer func() {
            if r := recover(); r != nil {
                returnErr = fmt.Errorf("%w: %w", ErrProgramKilled, ErrProgramPanic)
                p.recoverFromPanic(r)
            }
        }()
    }

    // 5. Setup renderer
    if p.renderer == nil {
        p.renderer = newRenderer(p.output, p.startupOptions.has(withANSICompressor), p.fps)
    }

    // 6. Initialize terminal
    if err := p.initTerminal(); err != nil {
        return p.initialModel, err
    }

    // 7. Apply startup options
    if p.startupTitle != "" {
        p.renderer.setWindowTitle(p.startupTitle)
    }
    if p.startupOptions&withAltScreen != 0 {
        p.renderer.enterAltScreen()
    }
    if p.startupOptions&withoutBracketedPaste == 0 {
        p.renderer.enableBracketedPaste()
    }
    if p.startupOptions&withMouseCellMotion != 0 {
        p.renderer.enableMouseCellMotion()
        p.renderer.enableMouseSGRMode()
    } else if p.startupOptions&withMouseAllMotion != 0 {
        p.renderer.enableMouseAllMotion()
        p.renderer.enableMouseSGRMode()
    }
    if p.startupOptions&withReportFocus != 0 {
        p.renderer.enableReportFocus()
    }

    // 8. Start renderer
    p.renderer.start()

    // 9. Initialize model
    model := p.initialModel
    if initCmd := model.Init(); initCmd != nil {
        ch := make(chan struct{})
        p.handlers.add(ch)
        go func() {
            defer close(ch)
            select {
            case cmds <- initCmd:
            case <-p.ctx.Done():
            }
        }()
    }

    // 10. Render initial view
    p.renderer.write(model.View())

    // 11. Setup input reading
    if p.input != nil {
        if err := p.initCancelReader(false); err != nil {
            return model, err
        }
    }

    // 12. Setup resize handling
    p.handlers.add(p.handleResize())

    // 13. Start command execution
    p.handlers.add(p.handleCommands(cmds))

    // 14. Run main event loop
    model, err := p.eventLoop(model, cmds)

    // 15. Handle errors
    killed := p.externalCtx.Err() != nil || p.ctx.Err() != nil || err != nil
    if killed {
        // Handle cancellation errors
        if err == nil && p.externalCtx.Err() != nil {
            err = fmt.Errorf("%w: %w", ErrProgramKilled, p.externalCtx.Err())
        } else if err == nil && p.ctx.Err() != nil {
            err = ErrProgramKilled
        } else {
            err = fmt.Errorf("%w: %w", ErrProgramKilled, err)
        }
    } else {
        // Graceful shutdown: render final state
        p.renderer.write(model.View())
    }

    // 16. Shutdown
    p.shutdown(killed)

    return model, err
}
```

---

## 3. Event Loop Internals

### 3.1 The Event Loop

```go
func (p *Program) eventLoop(model Model, cmds chan Cmd) (Model, error) {
    for {
        select {
        // Context cancellation
        case <-p.ctx.Done():
            return model, nil

        // Error from goroutines
        case err := <-p.errs:
            return model, err

        // Message processing
        case msg := <-p.msgs:
            // 1. Filter message (if filter set)
            if p.filter != nil {
                msg = p.filter(model, msg)
            }
            if msg == nil {
                continue // Filtered out
            }

            // 2. Handle internal messages
            switch msg := msg.(type) {
            case QuitMsg:
                return model, nil

            case InterruptMsg:
                return model, ErrInterrupted

            case SuspendMsg:
                if suspendSupported {
                    p.suspend()
                }

            case clearScreenMsg:
                p.renderer.clearScreen()

            case enterAltScreenMsg:
                p.renderer.enterAltScreen()

            case exitAltScreenMsg:
                p.renderer.exitAltScreen()

            case enableMouseCellMotionMsg:
                p.renderer.enableMouseCellMotion()
                p.renderer.enableMouseSGRMode()

            case enableMouseAllMotionMsg:
                p.renderer.enableMouseAllMotion()
                p.renderer.enableMouseSGRMode()

            case disableMouseMsg:
                p.disableMouse()

            case showCursorMsg:
                p.renderer.showCursor()

            case hideCursorMsg:
                p.renderer.hideCursor()

            case enableBracketedPasteMsg:
                p.renderer.enableBracketedPaste()

            case disableBracketedPasteMsg:
                p.renderer.disableBracketedPaste()

            case enableReportFocusMsg:
                p.renderer.enableReportFocus()

            case disableReportFocusMsg:
                p.renderer.disableReportFocus()

            case execMsg:
                p.exec(msg.cmd, msg.fn)

            case BatchMsg:
                // Execute batch of commands concurrently
                for _, cmd := range msg {
                    select {
                    case <-p.ctx.Done():
                        return model, nil
                    case cmds <- cmd:
                    }
                }
                continue

            case sequenceMsg:
                // Execute commands sequentially
                go func() {
                    for _, cmd := range msg {
                        if cmd == nil {
                            continue
                        }
                        msg := cmd()
                        if batchMsg, ok := msg.(BatchMsg); ok {
                            g, _ := errgroup.WithContext(p.ctx)
                            for _, cmd := range batchMsg {
                                cmd := cmd
                                g.Go(func() error {
                                    p.Send(cmd())
                                    return nil
                                })
                            }
                            g.Wait()
                            continue
                        }
                        p.Send(msg)
                    }
                }()

            case setWindowTitleMsg:
                p.SetWindowTitle(string(msg))

            case windowSizeMsg:
                go p.checkResize()
            }

            // 3. Handle renderer messages
            if r, ok := p.renderer.(*standardRenderer); ok {
                r.handleMessages(msg)
            }

            // 4. Update model
            var cmd Cmd
            model, cmd = model.Update(msg)

            // 5. Send command to executor
            select {
            case <-p.ctx.Done():
                return model, nil
            case cmds <- cmd:
            }

            // 6. Queue view for rendering
            p.renderer.write(model.View())
        }
    }
}
```

### 3.2 Message Flow

```
┌─────────────────────────────────────────────────────────┐
│                    Message Sources                       │
│                                                          │
│  Input Reader → KeyMsg, MouseMsg                        │
│  Command Result → Custom Msg                            │
│  Signal Handler → InterruptMsg, QuitMsg                 │
│  Resize Handler → WindowSizeMsg                         │
│  Program.Send() → Any Msg                               │
│                                                          │
└─────────────────────────┬───────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                   Message Channel                        │
│  p.msgs (chan Msg)                                      │
└─────────────────────────┬───────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                  Event Loop Select                       │
│  case msg := <-p.msgs                                   │
└─────────────────────────┬───────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                   Message Filter                         │
│  if p.filter != nil { msg = p.filter(model, msg) }      │
└─────────────────────────┬───────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│               Internal Message Handler                   │
│  switch msg.(type) { QuitMsg, BatchMsg, ... }           │
└─────────────────────────┬───────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                   Model Update                           │
│  model, cmd = model.Update(msg)                         │
└─────────────────────────┬───────────────────────────────┘
                          │
              ┌───────────┴───────────┐
              │                       │
              ▼                       ▼
┌─────────────────────┐   ┌─────────────────────┐
│   Renderer Write    │   │   Command Send      │
│  p.renderer.write() │   │   cmds <- cmd       │
└─────────────────────┘   └──────────┬──────────┘
                                     │
                                     ▼
                              ┌─────────────────────┐
                              │  Command Executor   │
                              │  (goroutine)        │
                              └──────────┬──────────┘
                                         │
                                         │ cmd()
                                         ▼
                              ┌─────────────────────┐
                              │  Returns Msg        │
                              └──────────┬──────────┘
                                         │
                                         │ p.Send()
                                         └──────────────┐
                                                        │
                                                        ▼
                                              (back to message channel)
```

---

## 4. Command Execution System

### 4.1 Command Definition

```go
// Cmd is a function that returns a message when complete
type Cmd func() Msg

// Nil command (no-op)
var nilCmd Cmd = nil

// Quit command
func Quit() Msg {
    return QuitMsg{}
}
```

### 4.2 Command Executor

```go
func (p *Program) handleCommands(cmds chan Cmd) chan struct{} {
    ch := make(chan struct{})

    go func() {
        defer close(ch)

        for {
            select {
            case <-p.ctx.Done():
                return

            case cmd := <-cmds:
                if cmd == nil {
                    continue
                }

                // Execute command in goroutine
                go func() {
                    // Recover from panics
                    if !p.startupOptions.has(withoutCatchPanics) {
                        defer func() {
                            if r := recover(); r != nil {
                                p.recoverFromGoPanic(r)
                            }
                        }()
                    }

                    // Execute command (can block)
                    msg := cmd()

                    // Send result to main loop
                    p.Send(msg)
                }()
            }
        }
    }()

    return ch
}
```

### 4.3 Command Patterns

**HTTP Request:**

```go
func fetchData(url string) tea.Cmd {
    return func() tea.Msg {
        resp, err := http.Get(url)
        if err != nil {
            return errorMsg{err}
        }
        defer resp.Body.Close()

        body, err := io.ReadAll(resp.Body)
        if err != nil {
            return errorMsg{err}
        }

        return dataMsg{body}
    }
}
```

**File I/O:**

```go
func loadFile(path string) tea.Cmd {
    return func() tea.Msg {
        data, err := os.ReadFile(path)
        if err != nil {
            return fileError{err}
        }
        return fileLoaded{string(data)}
    }
}

func saveFile(path, content string) tea.Cmd {
    return func() tea.Msg {
        err := os.WriteFile(path, []byte(content), 0644)
        if err != nil {
            return saveError{err}
        }
        return fileSaved{}
    }
}
```

**Time Delay:**

```go
func waitFor(d time.Duration) tea.Cmd {
    return func() tea.Msg {
        time.Sleep(d)
        return delayCompleteMsg{}
    }
}
```

**Process Execution:**

```go
func runCommand(name string, args ...string) tea.Cmd {
    return func() tea.Msg {
        output, err := exec.Command(name, args...).CombinedOutput()
        if err != nil {
            return commandError{err}
        }
        return commandComplete{string(output)}
    }
}
```

---

## 5. Batch and Sequence Commands

### 5.1 Batch (Concurrent)

```go
// commands.go

// Batch performs multiple commands concurrently
func Batch(cmds ...Cmd) Cmd {
    var validCmds []Cmd
    for _, c := range cmds {
        if c == nil {
            continue
        }
        validCmds = append(validCmds, c)
    }

    switch len(validCmds) {
    case 0:
        return nil
    case 1:
        return validCmds[0]
    default:
        return func() Msg {
            return BatchMsg(validCmds)
        }
    }
}

// BatchMsg is received by event loop to dispatch batch
type BatchMsg []Cmd

// Usage in Model.Init():
func (m Model) Init() tea.Cmd {
    return tea.Batch(
        loadDataCmd(),
        startSpinnerCmd(),
        tickCmd(),
    )
}

// Event loop handles BatchMsg:
case BatchMsg:
    for _, cmd := range msg {
        cmds <- cmd  // All commands run concurrently
    }
    continue
```

### 5.2 Sequence (Sequential)

```go
// Sequence runs commands one at a time, in order
func Sequence(cmds ...Cmd) Cmd {
    return func() Msg {
        return sequenceMsg(cmds)
    }
}

type sequenceMsg []Cmd

// Event loop handles sequenceMsg:
case sequenceMsg:
    go func() {
        for _, cmd := range msg {
            if cmd == nil {
                continue
            }

            msg := cmd()  // Wait for each command
            if batchMsg, ok := msg.(BatchMsg); ok {
                // Handle nested batch
                g, _ := errgroup.WithContext(p.ctx)
                for _, cmd := range batchMsg {
                    cmd := cmd
                    g.Go(func() error {
                        p.Send(cmd())
                        return nil
                    })
                }
                g.Wait()
                continue
            }

            p.Send(msg)  // Send result before next
        }
    }()

// Usage:
func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    if shouldSaveAndQuit {
        return m, tea.Sequence(
            saveDataCmd(),  // Wait for save
            tea.Quit,       // Then quit
        )
    }
    return m, nil
}
```

### 5.3 Batch vs Sequence Comparison

```go
// Use Batch when commands are independent:
tea.Batch(
    fetchUserCmd(),     // Can run in parallel
    fetchPostsCmd(),    // Can run in parallel
    fetchCommentsCmd(), // Can run in parallel
)

// Use Sequence when order matters:
tea.Sequence(
    validateDataCmd(),  // Must complete first
    saveDataCmd(),      // Only if validation passes
    notifySavedCmd(),   // After save completes
)

// Combine both:
tea.Sequence(
    tea.Batch(
        fetchUserCmd(),
        fetchSettingsCmd(),
    ),  // Both fetch in parallel
    renderDashboardCmd(),  // After both complete
)
```

---

## 6. Timer Commands: Tick and Every

### 6.1 Tick (Interval from Now)

```go
// Tick produces a command that fires after duration d
func Tick(d time.Duration, fn func(time.Time) Msg) Cmd {
    t := time.NewTimer(d)
    return func() Msg {
        ts := <-t.C
        t.Stop()
        for len(t.C) > 0 {
            <-t.C  // Drain channel
        }
        return fn(ts)
    }
}

// Usage: Animation loop
type TickMsg time.Time

func tick() tea.Cmd {
    return tea.Tick(time.Second/60, func(t time.Time) Msg {
        return TickMsg(t)
    })
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg.(type) {
    case TickMsg:
        m.frame++
        return m, tick()  // Reschedule
    }
    return m, nil
}
```

### 6.2 Every (Sync with Clock)

```go
// Every ticks in sync with the system clock
func Every(d time.Duration, fn func(time.Time) Msg) Cmd {
    n := time.Now()
    // Calculate time until next boundary
    dur := n.Truncate(d).Add(d).Sub(n)
    t := time.NewTimer(dur)

    return func() Msg {
        ts := <-t.C
        t.Stop()
        for len(t.C) > 0 {
            <-t.C
        }
        return fn(ts)
    }
}

// Usage: Update clock every minute
type ClockTick time.Time

func updateClock() tea.Cmd {
    return tea.Every(time.Minute, func(t time.Time) Msg {
        return ClockTick(t)
    })
}

func (m Model) Init() tea.Cmd {
    return updateClock()  // Start ticking
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg.(type) {
    case ClockTick:
        m.currentTime = time.Now()
        return m, updateClock()  // Reschedule
    }
    return m, nil
}
```

### 6.3 Tick vs Every

```
Tick Example (1 second interval):
12:00:00 → [Tick starts]
12:00:01 → Fires
12:00:02 → Fires
12:00:03 → Fires

Every Example (1 minute interval):
12:00:20 → [Every starts]
12:00:40 → [Waiting for next minute boundary]
12:01:00 → Fires (top of minute)
12:02:00 → Fires
12:03:00 → Fires

Tick is good for: Animations, countdowns
Every is good for: Clocks, periodic cleanup
```

---

## 7. Signal Handling

### 7.1 Signal Handler Setup

```go
func (p *Program) handleSignals() chan struct{} {
    ch := make(chan struct{})

    go func() {
        sig := make(chan os.Signal, 1)
        signal.Notify(sig, syscall.SIGINT, syscall.SIGTERM)
        defer func() {
            signal.Stop(sig)
            close(ch)
        }()

        for {
            select {
            case <-p.ctx.Done():
                return

            case s := <-sig:
                if atomic.LoadUint32(&p.ignoreSignals) == 0 {
                    switch s {
                    case syscall.SIGINT:
                        p.msgs <- InterruptMsg{}
                    default:
                        p.msgs <- QuitMsg{}
                    }
                    return
                }
            }
        }
    }()

    return ch
}
```

### 7.2 Signal Behavior

```
┌─────────────────────────────────────────────────────────┐
│                    Signal Handling                       │
│                                                          │
│  SIGINT (Ctrl+C):                                       │
│  - In raw mode: captured as KeyMsg (ctrl+c)             │
│  - Not in TTY: caught here, sends InterruptMsg          │
│  - Result: ErrInterrupted                               │
│                                                          │
│  SIGTERM (kill):                                        │
│  - Always caught here                                   │
│  - Sends QuitMsg                                        │
│  - Result: Clean shutdown                               │
│                                                          │
│  SIGWINCH (resize):                                     │
│  - Handled separately in handleResize()                 │
│  - Sends WindowSizeMsg                                  │
│                                                          │
│  SIGTSTP (Ctrl+Z):                                      │
│  - Not handled by default                               │
│  - Can be added for suspend support                     │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### 7.3 Window Resize

```go
func (p *Program) handleResize() chan struct{} {
    ch := make(chan struct{})

    if p.ttyOutput != nil {
        // Get initial size
        go p.checkResize()

        // Listen for resize events
        go p.listenForResize(ch)
    } else {
        close(ch)
    }

    return ch
}

func (p *Program) listenForResize(done chan struct{}) {
    // Platform-specific implementation
    // Unix: syscall.SIGWINCH
    // Windows: console API

    for {
        select {
        case <-p.ctx.Done():
            close(done)
            return

        case <-sigWinch:
            p.checkResize()
        }
    }
}

func (p *Program) checkResize() {
    if p.ttyOutput == nil {
        return
    }

    w, h, err := term.GetSize(p.ttyOutput.Fd())
    if err != nil {
        return
    }

    p.Send(WindowSizeMsg{Width: w, Height: h})
}
```

---

## 8. Input Reading and Parsing

### 8.1 Cancel Reader

```go
// github.com/muesli/cancelreader
// Allows canceling blocking reads

type CancelReader interface {
    Read(data []byte) (int, error)
    Cancel() error
    Close() error
}

func (p *Program) initCancelReader(reopen bool) error {
    if reopen {
        p.cancelReader.Close()
    }

    var err error
    p.cancelReader, err = cancelreader.NewReader(p.input)
    if err != nil {
        return err
    }

    // Start reading loop
    go p.readLoop()

    return nil
}
```

### 8.2 Read Loop

```go
func (p *Program) readLoop() {
    for {
        select {
        case <-p.ctx.Done():
            return

        default:
            buf := make([]byte, 256)
            n, err := p.cancelReader.Read(buf)

            if err != nil {
                if err == io.EOF {
                    return
                }
                if err == cancelreader.ErrCanceled {
                    return
                }
                p.errs <- err
                return
            }

            if n == 0 {
                continue
            }

            // Parse input into messages
            msgs := p.parseInput(buf[:n])

            // Send to main loop
            for _, msg := range msgs {
                p.msgs <- msg
            }
        }
    }
}
```

### 8.3 Key Sequence Parsing

```go
func (p *Program) parseInput(buf []byte) []tea.Msg {
    var msgs []tea.Msg

    for len(buf) > 0 {
        // Try to match known sequences
        msg, width := parseKeySequence(buf)

        if msg != nil {
            msgs = append(msgs, msg)
        }

        buf = buf[width:]
    }

    return msgs
}

func parseKeySequence(buf []byte) (tea.Msg, int) {
    // Escape character
    if buf[0] == 0x1b {
        // Check for longer sequences
        if len(buf) >= 3 {
            seq := string(buf[:3])
            if keyType, ok := sequences[seq]; ok {
                return tea.KeyMsg{Type: keyType}, 3
            }
        }

        // Just ESC key
        return tea.KeyMsg{Type: tea.KeyEscape}, 1
    }

    // UTF-8 rune
    r, width := utf8.DecodeRune(buf)
    return tea.KeyMsg{
        Type:  tea.KeyRunes,
        Runes: []rune{r},
    }, width
}

// Common sequences
var sequences = map[string]tea.KeyType{
    "\x1b[A": tea.KeyUp,
    "\x1b[B": tea.KeyDown,
    "\x1b[C": tea.KeyRight,
    "\x1b[D": tea.KeyLeft,
    "\x1b[3~": tea.KeyDelete,
    "\x1b[1~": tea.KeyHome,
    "\x1b[4~": tea.KeyEnd,
    "\x1b[5~": tea.KeyPgUp,
    "\x1b[6~": tea.KeyPgDown,
    "\x1b\r":  tea.KeyEnter,  // Alt+Enter
}
```

---

## 9. Terminal Management

### 9.1 Terminal Initialization

```go
func (p *Program) initTerminal() error {
    // Check if output is a TTY
    if p.ttyOutput != nil {
        // Enter raw mode
        state, err := term.MakeRaw(p.ttyOutput.Fd())
        if err != nil {
            return err
        }
        p.previousOutputState = state
    }

    // Check if input is a TTY
    if p.ttyInput != nil {
        state, err := term.MakeRaw(p.ttyInput.Fd())
        if err != nil {
            return err
        }
        p.previousTtyInputState = state
    }

    return nil
}
```

### 9.2 Terminal Restoration

```go
func (p *Program) restoreTerminalState() error {
    // Restore output state
    if p.previousOutputState != nil {
        err := term.Restore(p.ttyOutput.Fd(), p.previousOutputState)
        if err != nil {
            return err
        }
    }

    // Restore input state
    if p.previousTtyInputState != nil {
        err := term.Restore(p.ttyInput.Fd(), p.previousTtyInputState)
        if err != nil {
            return err
        }
    }

    return nil
}
```

### 9.3 Shutdown Sequence

```go
func (p *Program) shutdown(kill bool) {
    // Cancel context
    p.cancel()

    // Wait for all handlers to finish
    p.handlers.shutdown()

    // Cancel input reader
    if p.cancelReader != nil {
        if p.cancelReader.Cancel() {
            if !kill {
                p.waitForReadLoop()
            }
        }
        p.cancelReader.Close()
    }

    // Stop renderer
    if p.renderer != nil {
        if kill {
            p.renderer.kill()
        } else {
            p.renderer.stop()
        }
    }

    // Restore terminal
    p.restoreTerminalState()
}
```

---

## Key Takeaways

1. **Run() orchestrates lifecycle**: Init → Event Loop → Shutdown
2. **Event loop is select-based**: Concurrent message handling
3. **Commands run in goroutines**: Non-blocking I/O
4. **Batch = concurrent, Sequence = serial**: Choose based on needs
5. **Tick vs Every**: Interval vs clock-synchronized
6. **Signal handling**: SIGINT, SIGTERM, SIGWINCH
7. **Cancel reader**: Allows clean shutdown of blocking reads
8. **Terminal management**: Raw mode, alt screen, restore on exit

---

*Continue to [04-bubbletea-components-deep-dive.md](04-bubbletea-components-deep-dive.md) for the Bubbles component library.*
