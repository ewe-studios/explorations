---
title: "Production-Grade Bubble Tea: Deployment and Operations"
subtitle: "Comprehensive guide for deploying Bubble Tea TUI applications to production"
based_on: "Bubble Tea + Bubbles + Lip Gloss"
---

# Production-Grade Bubble Tea

## 1. Overview

### 1.1 What This Guide Covers

This document provides patterns and practices for building production-ready TUI applications with Bubble Tea. While Bubble Tea is designed for simplicity, these patterns help scale to enterprise deployments.

**Target Audience:**
- Developers building TUI tools for production use
- Teams deploying SSH-based applications
- Engineers optimizing TUI performance

**Prerequisites:**
- Understanding of Bubble Tea basics
- Experience with Go programming
- Familiarity with terminal concepts

---

## 2. Architecture Patterns

### 2.1 Layered Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Presentation Layer                    │
│  - Bubble Tea Models                                    │
│  - View rendering                                       │
│  - User input handling                                  │
└─────────────────────────┬───────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                    Application Layer                     │
│  - Business logic                                       │
│  - State management                                     │
│  - Command orchestration                                │
└─────────────────────────┬───────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                     Domain Layer                         │
│  - Domain models                                        │
│  - Business rules                                       │
│  - Validation                                           │
└─────────────────────────┬───────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                   Infrastructure Layer                   │
│  - Database access                                      │
│  - HTTP clients                                         │
│  - File I/O                                             │
└─────────────────────────────────────────────────────────┘
```

### 2.2 Separation of Concerns

```go
// ✓ Good: Separated concerns
package main

// Domain
type Task struct {
    ID          string
    Title       string
    Description string
    Status      TaskStatus
    CreatedAt   time.Time
}

type TaskStatus string

const (
    TaskStatusPending   TaskStatus = "pending"
    TaskStatusCompleted TaskStatus = "completed"
)

// Application
type TaskService interface {
    ListTasks(ctx context.Context) ([]Task, error)
    CreateTask(ctx context.Context, title, desc string) (*Task, error)
    CompleteTask(ctx context.Context, id string) error
}

// Infrastructure
type TaskRepository struct {
    db *sql.DB
}

func (r *TaskRepository) ListTasks(ctx context.Context) ([]Task, error) {
    // Database implementation
}

// Presentation (Bubble Tea)
type Model struct {
    tasks      []Task
    selected   int
    service    TaskService
    loading    bool
    err        error
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    // TUI logic only
}
```

### 2.3 Dependency Injection

```go
// Interface for dependencies
type DataProvider interface {
    FetchData(ctx context.Context) ([]Item, error)
    SaveItem(ctx context.Context, item Item) error
}

// Production implementation
type SQLProvider struct {
    db *sql.DB
}

func (p *SQLProvider) FetchData(ctx context.Context) ([]Item, error) {
    // SQL implementation
}

// Mock for testing
type MockProvider struct {
    Items []Item
}

func (m *MockProvider) FetchData(ctx context.Context) ([]Item, error) {
    return m.Items, nil
}

// Model with injected dependency
type Model struct {
    provider DataProvider
    items    []Item
}

func NewModel(provider DataProvider) Model {
    return Model{
        provider: provider,
        items:    []Item{},
    }
}

func (m Model) Init() tea.Cmd {
    return fetchDataCmd(m.provider)
}
```

---

## 3. Performance Optimization

### 3.1 Rendering Optimization

```go
// Cache expensive computations
type Model struct {
    items        []Item
    renderedList string
    listHash     uint64
    dirty        bool
}

func computeHash(items []Item) uint64 {
    h := fnv.New64a()
    for _, item := range items {
        h.Write([]byte(item.ID))
    }
    return h.Sum64()
}

func (m *Model) invalidate() {
    m.dirty = true
}

func (m *Model) renderList() string {
    if !m.dirty {
        return m.renderedList
    }

    var b strings.Builder
    for i, item := range m.items {
        if i == m.selected {
            b.WriteString("> ")
        } else {
            b.WriteString("  ")
        }
        b.WriteString(item.Title)
        b.WriteString("\n")
    }

    m.renderedList = b.String()
    m.listHash = computeHash(m.items)
    m.dirty = false
    return m.renderedList
}

func (m Model) View() string {
    return m.renderList()
}
```

### 3.2 Memory Optimization

```go
// Reuse buffers
type Model struct {
    buf        bytes.Buffer
    styleCache map[string]lipgloss.Style
}

func NewModel() Model {
    m := Model{
        styleCache: make(map[string]lipgloss.Style),
    }
    // Pre-allocate buffer
    m.buf.Grow(4096)
    return m
}

func (m Model) View() string {
    m.buf.Reset()

    // Reuse buffer for rendering
    m.buf.WriteString(m.headerView())
    m.buf.WriteString(m.listView())
    m.buf.WriteString(m.footerView())

    return m.buf.String()
}

// Cache styles (don't create new each frame)
func (m *Model) getStyle(name string) lipgloss.Style {
    if style, ok := m.styleCache[name]; ok {
        return style
    }

    style := lipgloss.NewStyle()
    switch name {
    case "title":
        style = style.Bold(true).Foreground(lipgloss.Color("205"))
    case "error":
        style = style.Foreground(lipgloss.Color("196"))
    }

    m.styleCache[name] = style
    return style
}
```

### 3.3 Lazy Loading

```go
// Load data on demand
type Model struct {
    items      []Item
    loaded     bool
    loading    bool
    viewport   viewport.Model
    ready      bool
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg := msg.(type) {
    case tea.WindowSizeMsg:
        if !m.ready {
            m.viewport = viewport.New(msg.Width, msg.Height-4)
            m.ready = true
        } else {
            m.viewport.Width = msg.Width
            m.viewport.Height = msg.Height - 4
        }

        // Load content only when viewport is ready
        if !m.loaded && !m.loading {
            m.loading = true
            return m, loadContentCmd()
        }
    }

    var cmd tea.Cmd
    m.viewport, cmd = m.viewport.Update(msg)
    return m, cmd
}

func (m Model) View() string {
    if !m.ready {
        return "Initializing..."
    }
    return m.viewport.View()
}
```

### 3.4 Viewport High Performance Mode

```go
// Use alternate screen buffer for smooth scrolling
type Model struct {
    viewport viewport.Model
    content  string
    ready    bool
}

func (m Model) Init() tea.Cmd {
    return tea.EnterAltScreen
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg := msg.(type) {
    case tea.WindowSizeMsg:
        h, v := msg.Height - 4, msg.Width
        m.viewport = viewport.New(v, h)
        m.viewport.YPosition = 2
        m.viewport.HighPerformanceRendering = true  // Key optimization
        m.viewport.SetContent(m.content)
        m.ready = true
    }

    var cmd tea.Cmd
    m.viewport, cmd = m.viewport.Update(msg)
    return m, cmd
}
```

---

## 4. Error Handling and Recovery

### 4.1 Error States

```go
type ErrorState struct {
    Err      error
    RetryCmd tea.Cmd
    CanRetry bool
}

type Model struct {
    state      AppState
    errorState *ErrorState
    data       []Item
}

type AppState int

const (
    LoadingState AppState = iota
    ReadyState
    ErrorState
)

func (m Model) View() string {
    switch m.state {
    case LoadingState:
        return "Loading..."
    case ReadyState:
        return m.renderData()
    case ErrorState:
        return m.renderError()
    }
    return ""
}

func (m Model) renderError() string {
    var b strings.Builder
    b.WriteString("\n")
    b.WriteString(lipgloss.NewStyle().
        Foreground(lipgloss.Color("196")).
        Bold(true).
        Render("Error"))
    b.WriteString("\n\n")
    b.WriteString(m.errorState.Err.Error())
    b.WriteString("\n\n")

    if m.errorState.CanRetry {
        b.WriteString("Press 'r' to retry")
    } else {
        b.WriteString("Press 'q' to quit")
    }

    return b.String()
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg := msg.(type) {
    case tea.KeyMsg:
        if m.state == ErrorState && msg.String() == "r" && m.errorState.CanRetry {
            m.state = LoadingState
            m.errorState = nil
            return m, m.errorState.RetryCmd
        }
    }
    return m, nil
}
```

### 4.2 Retry Logic

```go
func fetchDataWithRetry(url string, maxRetries int) tea.Cmd {
    return func() tea.Msg {
        var lastErr error

        for attempt := 0; attempt < maxRetries; attempt++ {
            resp, err := http.Get(url)
            if err == nil {
                defer resp.Body.Close()
                data, err := io.ReadAll(resp.Body)
                if err == nil {
                    return DataLoadedMsg{data}
                }
                lastErr = err
            } else {
                lastErr = err
            }

            // Exponential backoff
            if attempt < maxRetries-1 {
                time.Sleep(time.Duration(1<<uint(attempt)) * time.Second)
            }
        }

        return ErrorMsg{
            Err:      fmt.Errorf("failed after %d attempts: %w", maxRetries, lastErr),
            CanRetry: true,
        }
    }
}
```

### 4.3 Graceful Degradation

```go
// Degrade features when terminal doesn't support them
type Model struct {
    trueColor    bool
    mouseEnabled bool
    unicode      bool
}

func detectCapabilities() Model {
    m := Model{
        trueColor:    detectTrueColor(),
        mouseEnabled: true,  // Enable by default
        unicode:      true,  // Enable by default
    }

    // Check environment
    if os.Getenv("TERM") == "dumb" {
        m.trueColor = false
        m.mouseEnabled = false
        m.unicode = false
    }

    return m
}

func (m Model) render() string {
    if m.trueColor {
        return m.renderFullColor()
    } else if m.unicode {
        return m.renderAscii()
    } else {
        return m.renderBasic()
    }
}
```

---

## 5. Configuration Management

### 5.1 Configuration Structure

```go
type Config struct {
    // Display
    Theme       string `yaml:"theme"`
    ShowHelp    bool   `yaml:"show_help"`
    MouseEnable bool   `yaml:"mouse_enable"`

    // Network
    APIURL    string `yaml:"api_url"`
    Timeout   int    `yaml:"timeout_seconds"`

    // Paths
    DataDir   string `yaml:"data_dir"`
    LogFile   string `yaml:"log_file"`

    // Feature flags
    Experimental struct {
        NewRenderer bool `yaml:"new_renderer"`
        FastScroll  bool `yaml:"fast_scroll"`
    } `yaml:"experimental"`
}

func LoadConfig(path string) (*Config, error) {
    data, err := os.ReadFile(path)
    if err != nil {
        if os.IsNotExist(err) {
            return DefaultConfig(), nil
        }
        return nil, err
    }

    var cfg Config
    if err := yaml.Unmarshal(data, &cfg); err != nil {
        return nil, err
    }

    return &cfg, nil
}

func DefaultConfig() *Config {
    return &Config{
        Theme:       "default",
        ShowHelp:    true,
        MouseEnable: true,
        APIURL:      "https://api.example.com",
        Timeout:     30,
        DataDir:     "~/.myapp",
        LogFile:     "~/.myapp/app.log",
    }
}
```

### 5.2 Environment Variables

```go
func LoadConfigFromEnv() *Config {
    cfg := DefaultConfig()

    if theme := os.Getenv("MYAPP_THEME"); theme != "" {
        cfg.Theme = theme
    }

    if apiURL := os.Getenv("MYAPP_API_URL"); apiURL != "" {
        cfg.APIURL = apiURL
    }

    if timeout := os.Getenv("MYAPP_TIMEOUT"); timeout != "" {
        if t, err := strconv.Atoi(timeout); err == nil {
            cfg.Timeout = t
        }
    }

    if debug := os.Getenv("DEBUG"); debug == "1" || debug == "true" {
        cfg.LogFile = "/tmp/myapp-debug.log"
    }

    return cfg
}
```

---

## 6. Logging and Debugging

### 6.1 Debug Logging

```go
// logging.go
var (
    debugLog *os.File
    debugOn  bool
)

func init() {
    if os.Getenv("BUBBLETEA_DEBUG") == "1" {
        debugOn = true
        var err error
        debugLog, err = os.OpenFile("/tmp/bubbletea-debug.log",
            os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0600)
        if err != nil {
            return
        }
    }
}

func debug(format string, args ...interface{}) {
    if !debugOn || debugLog == nil {
        return
    }
    fmt.Fprintf(debugLog, format+"\n", args...)
}

// Usage in Model
func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    debug("Update received: %T", msg)

    switch msg := msg.(type) {
    case tea.KeyMsg:
        debug("Key pressed: %s", msg.String())
        // Handle key
    }

    return m, nil
}
```

### 6.2 Structured Logging

```go
import "log/slog"

type Model struct {
    logger *slog.Logger
    // ...
}

func NewModel(cfg *Config) Model {
    var logger *slog.Logger

    if cfg.LogFile != "" {
        f, err := os.OpenFile(cfg.LogFile,
            os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0600)
        if err == nil {
            logger = slog.New(slog.NewJSONHandler(f, &slog.HandlerOptions{
                Level: slog.LevelInfo,
            }))
        }
    }

    if logger == nil {
        logger = slog.New(slog.NewDiscardHandler())
    }

    return Model{
        logger: logger,
        // ...
    }
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    m.logger.Debug("handling message", "type", fmt.Sprintf("%T", msg))

    // Log errors
    if result, err := m.doSomething(); err != nil {
        m.logger.Error("operation failed", "error", err)
        m.err = err
    } else {
        m.logger.Info("operation succeeded", "result", result)
    }

    return m, nil
}
```

---

## 7. Testing Strategies

### 7.1 Unit Testing Models

```go
func TestCounterIncrement(t *testing.T) {
    m := Model{counter: 5}

    // Simulate key press
    newModel, _ := m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'+'}})

    if newModel.(Model).counter != 6 {
        t.Errorf("expected counter to be 6, got %d", newModel.(Model).counter)
    }
}

func TestCounterDecrement(t *testing.T) {
    m := Model{counter: 5}

    newModel, _ := m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'-'}})

    if newModel.(Model).counter != 4 {
        t.Errorf("expected counter to be 4, got %d", newModel.(Model).counter)
    }
}

func TestQuitOnQ(t *testing.T) {
    m := Model{}

    _, cmd := m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'q'}})

    if cmd != tea.Quit {
        t.Errorf("expected Quit command, got %v", cmd)
    }
}
```

### 7.2 Testing with TeaTest

```go
import "github.com/charmbracelet/bubbles/testutil"

func TestListNavigation(t *testing.T) {
    m := Model{
        items:    []string{"A", "B", "C"},
        selected: 0,
    }

    // Create test program
    p := testutil.NewTestProgram(t, m)

    // Send key presses
    p.Send(tea.KeyMsg{Type: tea.KeyDown})
    p.Send(tea.KeyMsg{Type: tea.KeyDown})

    // Get final model
    final := p.FinalModel(t).(Model)

    if final.selected != 2 {
        t.Errorf("expected selected to be 2, got %d", final.selected)
    }
}
```

### 7.3 Integration Testing

```go
func TestFullWorkflow(t *testing.T) {
    // Setup mock provider
    mock := &MockProvider{
        Items: []Item{
            {ID: "1", Title: "Item 1"},
            {ID: "2", Title: "Item 2"},
        },
    }

    // Create model
    m := NewModel(mock)

    // Run initialization
    initCmd := m.Init()
    if initCmd == nil {
        t.Fatal("expected init command")
    }

    // Execute command
    msg := initCmd()

    // Verify data loaded
    if dataMsg, ok := msg.(DataLoadedMsg); ok {
        if len(dataMsg.Items) != 2 {
            t.Errorf("expected 2 items, got %d", len(dataMsg.Items))
        }
    } else {
        t.Errorf("expected DataLoadedMsg, got %T", msg)
    }
}
```

---

## 8. Deployment Patterns

### 8.1 Building for Multiple Platforms

```bash
#!/bin/bash
# build.sh

VERSION="1.0.0"
PLATFORMS="darwin/amd64 darwin/arm64 linux/amd64 linux/arm64 windows/amd64"

for platform in $PLATFORMS; do
    OS=${platform%/*}
    ARCH=${platform#*/}

    OUTPUT="bin/myapp-${OS}-${ARCH}"
    if [ "$OS" = "windows" ]; then
        OUTPUT="${OUTPUT}.exe"
    fi

    echo "Building for ${OS}/${ARCH}..."
    GOOS=$OS GOARCH=$ARCH go build -ldflags="-s -w" -o "$OUTPUT" .
done
```

### 8.2 Docker Deployment

```dockerfile
# Dockerfile
FROM golang:1.21-alpine AS builder

WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download

COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -ldflags="-s -w" -o myapp .

# Runtime image
FROM alpine:3.18

RUN apk add --no-cache ca-certificates

WORKDIR /app
COPY --from=builder /app/myapp .

ENTRYPOINT ["/app/myapp"]
```

### 8.3 SSH Deployment with Wish

```go
import (
    "github.com/charmbracelet/wish"
    "github.com/charmbracelet/wish/bubbletea"
)

func main() {
    s, err := wish.NewServer(
        wish.WithAddress(":2222"),
        wish.WithHostKeyPath(".ssh/term_info_ed25519"),
        wish.WithPasswordAuth(func(ctx ssh.Context, password string) bool {
            return password == "secret"
        }),
        wish.WithMiddleware(
            bubbletea.Middleware(teaHandler),
            wish.LoggingMiddleware(),
        ),
    )
    if err != nil {
        log.Fatal(err)
    }

    log.Println("Starting SSH server on :2222")
    if err := s.ListenAndServe(); err != nil {
        log.Fatal(err)
    }
}

func teaHandler(s ssh.Session) (tea.Model, []tea.ProgramOption) {
    return NewModel(), []tea.ProgramOption{
        tea.WithAltScreen(),
        tea.WithMouseCellMotion(),
    }
}
```

---

## 9. Observability

### 9.1 Metrics Collection

```go
import "github.com/prometheus/client_golang/prometheus"

var (
    commandsProcessed = prometheus.NewCounter(
        prometheus.CounterOpts{
            Name: "bubbletea_commands_processed_total",
            Help: "Total number of commands processed",
        },
    )
    updateDuration = prometheus.NewHistogram(
        prometheus.HistogramOpts{
            Name:    "bubbletea_update_duration_seconds",
            Help:    "Duration of update function calls",
            Buckets: prometheus.DefBuckets,
        },
    )
)

func init() {
    prometheus.MustRegister(commandsProcessed, updateDuration)
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    start := time.Now()
    defer func() {
        updateDuration.Observe(time.Since(start).Seconds())
    }()

    commandsProcessed.Inc()

    // ... update logic
    return m, nil
}
```

### 9.2 Health Checks

```go
// Expose health check endpoint
type HealthStatus struct {
    Status    string            `json:"status"`
    Timestamp time.Time         `json:"timestamp"`
    Checks    map[string]string `json:"checks"`
}

func (m Model) healthCheck() HealthStatus {
    checks := make(map[string]string)

    // Check database connection
    if err := m.db.Ping(); err != nil {
        checks["database"] = "unhealthy"
    } else {
        checks["database"] = "healthy"
    }

    // Check API connectivity
    if _, err := m.apiClient.Status(); err != nil {
        checks["api"] = "unhealthy"
    } else {
        checks["api"] = "healthy"
    }

    status := "healthy"
    for _, v := range checks {
        if v == "unhealthy" {
            status = "unhealthy"
            break
        }
    }

    return HealthStatus{
        Status:    status,
        Timestamp: time.Now(),
        Checks:    checks,
    }
}
```

---

## Key Takeaways

1. **Layer your architecture**: Separate domain, application, and presentation
2. **Cache aggressively**: Styles, rendered views, computations
3. **Handle errors gracefully**: Retry logic, degradation, clear messages
4. **Log strategically**: Debug logging, structured logs, observability
5. **Test thoroughly**: Unit tests for Update, integration tests for commands
6. **Build for deployment**: Multi-platform binaries, Docker, SSH support
7. **Monitor in production**: Metrics, health checks, tracing

---

*Continue to [05-valtron-integration.md](05-valtron-integration.md) for TUI backend patterns.*
