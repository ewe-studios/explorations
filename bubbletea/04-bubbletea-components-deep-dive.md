---
title: "Bubbles Components Deep Dive: Reusable TUI Elements"
subtitle: "Complete guide to the Bubbles component library for Bubble Tea applications"
based_on: "Bubbles - github.com/charmbracelet/bubbles"
---

# Bubbles Components Deep Dive

## Table of Contents

1. [Bubbles Component Architecture](#1-bubbles-component-architecture)
2. [Spinner Component](#2-spinner-component)
3. [Progress Bar Component](#3-progress-bar-component)
4. [Text Input Component](#4-text-input-component)
5. [Text Area Component](#5-text-area-component)
6. [List Component](#6-list-component)
7. [Table Component](#7-table-component)
8. [Viewport Component](#8-viewport-component)
9. [Paginator Component](#9-paginator-component)
10. [Help Component](#10-help-component)
11. [Key Binding System](#11-key-binding-system)
12. [File Picker Component](#12-file-picker-component)
13. [Timer and Stopwatch](#13-timer-and-stopwatch)
14. [Component Composition Patterns](#14-component-composition-patterns)

---

## 1. Bubbles Component Architecture

### 1.1 Component Interface

All Bubbles components follow a consistent interface:

```go
// Component interface (implicit)
type Component interface {
    // Update handles messages and returns optional command
    Update(tea.Msg) (Component, tea.Cmd)

    // View renders the component
    View() string
}

// Most components also have:
type Spinner struct {
    // Component state
}

// Constructor
func New() Spinner

// Initialization
func (s Spinner) Init() tea.Cmd  // Optional

// Update
func (s Spinner) Update(msg tea.Msg) (Spinner, tea.Cmd)

// View
func (s Spinner) View() string
```

### 1.2 Component State Pattern

```go
// Component with full state
type TextInput struct {
    // Value
    value string
    cursor int

    // State
    Focus bool
    Placeholder string

    // Style
    Prompt           string
    CursorStyle      lipgloss.Style
    TextStyle        lipgloss.Style
    PlaceholderStyle lipgloss.Style

    // Validation
    CharLimit int

    // Internal
    width int
}

// Constructor with defaults
func New() TextInput {
    t := TextInput{
        CursorStyle:  lipgloss.NewStyle().Foreground(lipgloss.Color("205")),
        TextStyle:    lipgloss.NewStyle(),
        PlaceholderStyle: lipgloss.NewStyle().Foreground(lipgloss.Color("240")),
        CharLimit:    400,
        Prompt:       "> ",
    }
    return t
}
```

### 1.3 Component Communication

```go
// Parent component manages children
type Model struct {
    spinner   spinner.Model
    textinput textinput.Model
    list      list.Model
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    var cmds []tea.Cmd
    var cmd tea.Cmd

    // Delegate to spinner
    m.spinner, cmd = m.spinner.Update(msg)
    cmds = append(cmds, cmd)

    // Delegate to textinput (when focused)
    if m.textinput.Focused() {
        m.textinput, cmd = m.textinput.Update(msg)
        cmds = append(cmds, cmd)
    }

    // Delegate to list
    m.list, cmd = m.list.Update(msg)
    cmds = append(cmds, cmd)

    // Handle global keys
    switch msg := msg.(type) {
    case tea.KeyMsg:
        switch msg.String() {
        case "tab":
            // Toggle focus
            m.textinput.Focus()
        case "esc":
            m.textinput.Blur()
        }
    }

    return m, tea.Batch(cmds...)
}
```

---

## 2. Spinner Component

### 2.1 Basic Usage

```go
import "github.com/charmbracelet/bubbles/spinner"

type Model struct {
    spinner spinner.Model
}

func NewModel() Model {
    s := spinner.New()
    s.Spinner = spinner.Dot
    s.Style = lipgloss.NewStyle().Foreground(lipgloss.Color("205"))
    return Model{spinner: s}
}

func (m Model) Init() tea.Cmd {
    return m.spinner.Tick  // Start animation
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    var cmd tea.Cmd
    m.spinner, cmd = m.spinner.Update(msg)
    return m, cmd
}

func (m Model) View() string {
    return m.spinner.View() + " Loading..."
}
```

### 2.2 Spinner Types

```go
// Built-in spinner types
spinner.Line      // │ / - \
spinner.Dot       // ⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏
spinner.MiniDot   // ⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏
spinner.Jump      // ⢄ ⢂ ⢁ ⡈ ⡐ ⡠
spinner.Pulse     // █ ▒ ▓ ░
spinner.Points    // ● ○
spinner.Globe     // 🌍 🌎 🌏

// Custom spinner
customSpinner := spinner.Spinner{
    Frames: []string{"🕐", "🕑", "🕒", "🕓", "🕔", "🕕"},
    FPS:    time.Second / 6,
}
```

### 2.3 Spinner Styling

```go
s := spinner.New()
s.Spinner = spinner.Dot

// Style with Lip Gloss
s.Style = lipgloss.NewStyle().
    Foreground(lipgloss.Color("205")).
    Bold(true)

// Change spinner type dynamically
s.Spinner = spinner.Pulse

// Access spinner state
isSpinning := s.Frame != ""
```

---

## 3. Progress Bar Component

### 3.1 Animated Progress

```go
import "github.com/charmbracelet/bubbles/progress"

type Model struct {
    progress progress.Model
}

func NewModel() Model {
    p := progress.New(
        progress.WithDefaultGradient(),
    )
    return Model{progress: p}
}

func (m Model) Init() tea.Cmd {
    return tickCmd()
}

func tickCmd() tea.Cmd {
    return tea.Tick(time.Second*5, func(t time.Time) tea.Msg {
        return progressMsg(1.0) // Complete in 5 seconds
    })
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg := msg.(type) {
    case progressMsg:
        m.progress.SetPercent(float64(msg))
        if msg >= 1.0 {
            return m, tea.Quit
        }
        return m, tickCmd()
    }

    var cmd tea.Cmd
    m.progress, cmd = m.progress.Update(msg)
    return m, cmd
}

func (m Model) View() string {
    return "\n" + m.progress.View() + "\n"
}
```

### 3.2 Progress Styling

```go
// Gradient fills
progress.WithDefaultGradient()
progress.WithGradient("#ff0000", "#00ff00")  // Red to green

// Solid fills
progress.WithScaledGradient("#ff7474")
progress.WithSolidFill(lipgloss.Color("#ff7474"))

// Custom styling
p := progress.New(
    progress.WithGradient("196", "205"),
    progress.WithoutPercentage(),  // Hide percentage
)

// Progress bar with empty/filled runes
p.EmptyFillerRune = '░'
p.FilledRune = '█'
p.Width = 40
```

### 3.3 Static Progress

```go
// One-time render (no animation)
func renderProgress(percent float64) string {
    p := progress.New(progress.WithSolidFill("#ff7474"))
    p.SetPercent(percent)
    p.Width = 50
    return p.View()
}

// Usage in View()
func (m Model) View() string {
    return fmt.Sprintf(
        "Download: %s (%.0f%%)\n",
        renderProgress(m.percent),
        m.percent*100,
    )
}
```

---

## 4. Text Input Component

### 4.1 Basic Usage

```go
import "github.com/charmbracelet/bubbles/textinput"

type Model struct {
    textinput textinput.Model
}

func NewModel() Model {
    ti := textinput.New()
    ti.Placeholder = "Enter your name"
    ti.Focus()
    ti.CharLimit = 156
    ti.Width = 20

    return Model{textinput: ti}
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    var cmd tea.Cmd
    m.textinput, cmd = m.textinput.Update(msg)
    return m, cmd
}

func (m Model) View() string {
    return fmt.Sprintf(
        "Name: %s\n\nPress Enter to submit",
        m.textinput.View(),
    )
}
```

### 4.2 Text Input Features

```go
ti := textinput.New()

// Placeholder text
ti.Placeholder = "Search..."

// Character limit
ti.CharLimit = 100

// Width (0 = auto)
ti.Width = 30

// Prompt
ti.Prompt = "> "

// Cursor modes
ti.Cursor.Mode = textinput.CursorBlink  // Blinking block
ti.Cursor.Mode = textinput.CursorStatic // Static block
ti.Cursor.Mode = textinput.CursorHide   // Hidden

// Echo mode (for passwords)
ti.EchoMode = textinput.EchoNormal      // Show text
ti.EchoMode = textinput.EchoPassword    // Show bullets
ti.Password = true                       // Alternative

// Validation
ti.Validate = func(s string) error {
    if len(s) < 3 {
        return fmt.Errorf("minimum 3 characters")
    }
    return nil
}

// Focus control
ti.Focus()   // Enable input
ti.Blur()    // Disable input
ti.Focused() // Check focus state
```

### 4.3 Multiple Text Inputs

```go
type Model struct {
    inputs  []textinput.Model
    focusIndex int
}

func NewModel() Model {
    inputs := make([]textinput.Model, 3)

    // Name input
    inputs[0] = textinput.New()
    inputs[0].Placeholder = "Name"
    inputs[0].CharLimit = 50

    // Email input
    inputs[1] = textinput.New()
    inputs[1].Placeholder = "Email"
    inputs[1].CharLimit = 100

    // Password input
    inputs[2] = textinput.New()
    inputs[2].Placeholder = "Password"
    inputs[2].EchoMode = textinput.EchoPassword

    // Focus first input
    inputs[0].Focus()

    return Model{inputs: inputs, focusIndex: 0}
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg := msg.(type) {
    case tea.KeyMsg:
        switch msg.Type {
        case tea.KeyEnter:
            // Move to next input
            if m.focusIndex < len(m.inputs)-1 {
                m.inputs[m.focusIndex].Blur()
                m.focusIndex++
                m.inputs[m.focusIndex].Focus()
            }
        case tea.KeyShiftTab, tea.KeyCtrlP:
            // Move to previous input
            if m.focusIndex > 0 {
                m.inputs[m.focusIndex].Blur()
                m.focusIndex--
                m.inputs[m.focusIndex].Focus()
            }
        }
    }

    // Update current input
    var cmd tea.Cmd
    m.inputs[m.focusIndex], cmd = m.inputs[m.focusIndex].Update(msg)
    return m, cmd
}

func (m Model) View() string {
    var b strings.Builder
    b.WriteString("Name: ")
    b.WriteString(m.inputs[0].View())
    b.WriteString("\nEmail: ")
    b.WriteString(m.inputs[1].View())
    b.WriteString("\nPassword: ")
    b.WriteString(m.inputs[2].View())
    return b.String()
}
```

---

## 5. Text Area Component

### 5.1 Basic Usage

```go
import "github.com/charmbracelet/bubbles/textarea"

type Model struct {
    textarea textarea.Model
}

func NewModel() Model {
    ta := textarea.New()
    ta.Placeholder = "Type your message..."
    ta.Focus()
    ta.CharLimit = 1000
    ta.SetWidth(40)
    ta.SetHeight(5)

    return Model{textarea: ta}
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    var cmd tea.Cmd
    m.textarea, cmd = m.textarea.Update(msg)
    return m, cmd
}

func (m Model) View() string {
    return m.textarea.View()
}
```

### 5.2 Text Area Features

```go
ta := textarea.New()

// Dimensions
ta.SetWidth(40)
ta.SetHeight(10)

// Character limit
ta.CharLimit = 5000

// Placeholder
ta.Placeholder = "Enter your story..."

// Line numbers
ta.ShowLineNumbers = true

// Key bindings
ta.KeyMap.InsertNewline.SetKeys("ctrl+j")  // Enter without submitting
ta.KeyMap.AcceptSuggestion.SetKeys("tab")

// Value access
content := ta.Value()
ta.SetValue("New content")

// Cursor position
row, col := ta.Cursor()
```

---

## 6. List Component

### 6.1 Basic Usage

```go
import "github.com/charmbracelet/bubbles/list"

type Model struct {
    list list.Model
}

// Implement list.Item interface
type Item struct {
    Title, Desc string
}

func (i Item) Title() string       { return i.Title }
func (i Item) Description() string { return i.Desc }
func (i Item) FilterValue() string { return i.Title }

func NewModel() Model {
    items := []list.Item{
        Item{Title: "Item 1", Desc: "Description 1"},
        Item{Title: "Item 2", Desc: "Description 2"},
        Item{Title: "Item 3", Desc: "Description 3"},
    }

    l := list.New(items, list.NewDefaultDelegate(), 0, 0)
    l.Title = "My List"

    return Model{list: l}
}

func (m Model) Init() tea.Cmd {
    return nil
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    var cmd tea.Cmd
    m.list, cmd = m.list.Update(msg)
    return m, cmd
}

func (m Model) View() string {
    return m.list.View()
}
```

### 6.2 List Delegates

```go
// Default delegate
delegate := list.NewDefaultDelegate()

// Custom delegate
type CustomDelegate struct{}

func (d CustomDelegate) Height() int { return 3 }
func (d CustomDelegate) Spacing() int { return 1 }
func (d CustomDelegate) Update(msg tea.Msg, m *list.Model) tea.Cmd { return nil }
func (d CustomDelegate) Render(w io.Writer, m list.Model, index int, item list.Item) {
    i, ok := item.(Item)
    if !ok {
        return
    }

    // Custom rendering
    if index == m.Index() {
        fmt.Fprintf(w, "> %s\n  %s", i.Title, i.Desc)
    } else {
        fmt.Fprintf(w, "  %s\n  %s", i.Title, i.Desc)
    }
}

// Use custom delegate
l := list.New(items, CustomDelegate{}, width, height)
```

### 6.3 List Features

```go
l := list.New(items, delegate, width, height)

// Title and description
l.Title = "My List"
l.SetFilteringEnabled(true)  // Enable search
l.SetShowStatusBar(true)
l.SetShowHelp(true)

// Pagination
l.SetPaginationEnabled(true)

// Spinner (for loading)
l.SetShowSpinner(true)
l.Spinner = spinner.Spinner{...}

// Status messages
l.SetStatusMessage("Loading...")

// Programmatic control
l.Select(5)  // Select item at index 5
l.InsertItem(0, newItem)  // Insert at index
l.RemoveItem(index)  // Remove item
l.SetItems(newItems)  // Replace all items

// Selected item
selected := l.SelectedItem()
```

### 6.4 Fancy List

```go
// Fancy delegate with styles
delegate := list.NewDefaultDelegate()

// Style the delegate
delegate.Styles.SelectedTitle = lipgloss.NewStyle().
    Foreground(lipgloss.Color("205")).
    Bold(true)
delegate.Styles.NormalTitle = lipgloss.NewStyle()
delegate.Styles.SelectedDesc = lipgloss.NewStyle().
    Foreground(lipgloss.Color("241"))

// Or use fancy list preset
l := list.New(items, list.NewDefaultDelegate(), width, height)
l.Styles.Title = lipgloss.NewStyle().
    Foreground(lipgloss.Color("205")).
    Bold(true)
```

---

## 7. Table Component

### 7.1 Basic Usage

```go
import "github.com/charmbracelet/bubbles/table"

type Model struct {
    table table.Model
}

func NewModel() Model {
    // Define columns
    columns := []table.Column{
        {Title: "ID", Width: 5},
        {Title: "Name", Width: 20},
        {Title: "Email", Width: 30},
        {Title: "Status", Width: 10},
    }

    // Define rows
    rows := []table.Row{
        {"1", "Alice", "alice@example.com", "Active"},
        {"2", "Bob", "bob@example.com", "Inactive"},
        {"3", "Charlie", "charlie@example.com", "Active"},
    }

    // Create table
    t := table.New(
        table.WithColumns(columns),
        table.WithRows(rows),
        table.WithFocused(true),
        table.WithHeight(7),
    )

    // Style the table
    s := table.DefaultStyles()
    s.Header = s.Header.
        Border(lipgloss.NormalBorder()).
        BorderBottom(true).
        Bold(true)
    s.Selected = s.Selected.
        Foreground(lipgloss.Color("205")).
        Bold(true)
    t.SetStyles(s)

    return Model{table: t}
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    var cmd tea.Cmd
    m.table, cmd = m.table.Update(msg)
    return m, cmd
}

func (m Model) View() string {
    return m.table.View()
}
```

### 7.2 Table Features

```go
// Sorting
t.SetSortColumn("Name")
t.SetSortDirection(table.SortAscending)

// Selection
selected := t.SelectedRow()
selectedIndex := t.Cursor()

// Programmatic navigation
t.MoveDown(5)   // Move down 5 rows
t.MoveUp(3)     // Move up 3 rows
t.GotoTop()     // Go to first row
t.GotoBottom()  // Go to last row

// Dynamic updates
t.SetRows(newRows)
t.SetColumns(newColumns)

// Key bindings
t.KeyMap.LineUp.SetKeys("k", "up")
t.KeyMap.LineDown.SetKeys("j", "down")
```

---

## 8. Viewport Component

### 8.1 Basic Usage

```go
import "github.com/charmbracelet/bubbles/viewport"

type Model struct {
    viewport viewport.Model
    ready    bool
}

func NewModel() Model {
    return Model{
        viewport: viewport.New(80, 24),
        ready:    false,
    }
}

func (m Model) Init() tea.Cmd {
    return nil
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg := msg.(type) {
    case tea.WindowSizeMsg:
        if !m.ready {
            // Initialize on first resize
            m.viewport = viewport.New(msg.Width, msg.Height)
            m.ready = true
        } else {
            m.viewport.Width = msg.Width
            m.viewport.Height = msg.Height
        }

        // Set content
        m.viewport.SetContent(longText)
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

### 8.2 Viewport Features

```go
v := viewport.New(width, height)

// Content
v.SetContent(longString)
v.SetContent(string(longBytes))

// Scrolling
v.GotoTop()
v.GotoBottom()
v.ScrollUp(1)
v.ScrollDown(1)
v.PageUp()
v.PageDown()
v.HalfPageUp()
v.HalfPageDown()

// Position
scrollTop := v.ScrollPercent()
v.SetYOffset(10)  // Scroll to position 10

// Mouse wheel support (enabled by default)
v.MouseWheelEnabled = true
v.MouseWheelDelta = 3  // Lines per scroll

// High performance mode
// Uses alternate screen buffer for smooth scrolling
```

---

## 9. Paginator Component

### 9.1 Basic Usage

```go
import "github.com/charmbracelet/bubbles/paginator"

type Model struct {
    paginator paginator.Model
    items     []string
}

func NewModel() Model {
    p := paginator.New()
    p.Type = paginator.Dots  // or paginator.Pages

    return Model{
        paginator: p,
        items:     []string{"Item 1", "Item 2", "Item 3"},
    }
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    var cmd tea.Cmd
    m.paginator, cmd = m.paginator.Update(msg)

    // Get current page items
    start, end := m.paginator.GetSliceBounds(len(m.items))
    currentPage := m.items[start:end]

    return m, cmd
}

func (m Model) View() string {
    var b strings.Builder
    for _, item := range m.currentPage {
        b.WriteString(item + "\n")
    }
    b.WriteString("\n")
    b.WriteString(m.paginator.View())
    return b.String()
}
```

### 9.2 Paginator Styles

```go
// Dot style (iOS-style)
p.Type = paginator.Dots
p.ActiveDot = lipgloss.NewStyle().Foreground(lipgloss.Color("205")).Render("•")
p.InactiveDot = lipgloss.NewStyle().Foreground(lipgloss.Color("241")).Render("•")

// Page number style
p.Type = paginator.Pages
p.KeyMap.NextPage.SetKeys("l", "right")
p.KeyMap.PrevPage.SetKeys("h", "left")
```

---

## 10. Help Component

### 10.1 Basic Usage

```go
import "github.com/charmbracelet/bubbles/help"

type Model struct {
    help     help.Model
    keymap   KeyMap
}

type KeyMap struct {
    Up       key.Binding
    Down     key.Binding
    Select   key.Binding
    Quit     key.Binding
}

var DefaultKeyMap = KeyMap{
    Up: key.NewBinding(
        key.WithKeys("up", "k"),
        key.WithHelp("↑/k", "move up"),
    ),
    Down: key.NewBinding(
        key.WithKeys("down", "j"),
        key.WithHelp("↓/j", "move down"),
    ),
    Select: key.NewBinding(
        key.WithKeys("enter"),
        key.WithHelp("enter", "select"),
    ),
    Quit: key.NewBinding(
        key.WithKeys("q", "ctrl+c"),
        key.WithHelp("q", "quit"),
    ),
}

func NewModel() Model {
    return Model{
        help:   help.New(),
        keymap: DefaultKeyMap,
    }
}

func (m Model) View() string {
    return "\n" + m.help.View(m.keymap)
}
```

### 10.2 Help Styles

```go
h := help.New()

// Compact mode (single line)
h.ShowAll = false

// Full mode (expanded)
h.ShowAll = true

// Styling
h.Styles.FullKey = lipgloss.NewStyle().Foreground(lipgloss.Color("205"))
h.Styles.FullDesc = lipgloss.NewStyle().Foreground(lipgloss.Color("241"))
```

---

## 11. Key Binding System

### 11.1 Key Bindings

```go
import "github.com/charmbracelet/bubbles/key"

// Define keymap
type KeyMap struct {
    Up       key.Binding
    Down     key.Binding
    Select   key.Binding
    Toggle   key.Binding
}

// Create bindings
var DefaultKeyMap = KeyMap{
    Up: key.NewBinding(
        key.WithKeys("up", "k", "ctrl+p"),
        key.WithHelp("↑/k", "move up"),
        key.WithDisabled(),  // Initially disabled
    ),
    Down: key.NewBinding(
        key.WithKeys("down", "j", "ctrl+n"),
        key.WithHelp("↓/j", "move down"),
    ),
    Select: key.NewBinding(
        key.WithKeys("enter", " "),
        key.WithHelp("enter", "select"),
    ),
    Toggle: key.NewBinding(
        key.WithKeys("t"),
        key.WithHelp("t", "toggle"),
    ),
}

// Check if key matches
func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg := msg.(type) {
    case tea.KeyMsg:
        switch {
        case key.Matches(msg, m.KeyMap.Up):
            // Handle up
        case key.Matches(msg, m.KeyMap.Down):
            // Handle down
        case key.Matches(msg, m.KeyMap.Select):
            // Handle select
        }
    }
    return m, nil
}

// Enable/disable bindings
m.KeyMap.Up.SetEnabled(true)
m.KeyMap.Up.SetEnabled(false)
m.KeyMap.Up.Enabled()  // Check if enabled
```

---

## 12. File Picker Component

### 12.1 Basic Usage

```go
import "github.com/charmbracelet/bubbles/filepicker"

type Model struct {
    filepicker filepicker.Model
}

func NewModel() Model {
    fp, err := filepicker.NewFilepicker()
    if err != nil {
        panic(err)
    }

    fp.CurrentDirectory, _ = os.UserHomeDir()
    fp.AllowedTypes = []string{".go", ".mod", ".sum"}

    return Model{filepicker: fp}
}

func (m Model) Init() tea.Cmd {
    return m.filepicker.Init()
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg := msg.(type) {
    case filepicker.FileSelectedMsg:
        // User selected a file
        fmt.Println("Selected:", msg.File)
        return m, tea.Quit
    }

    var cmd tea.Cmd
    m.filepicker, cmd = m.filepicker.Update(msg)
    return m, cmd
}

func (m Model) View() string {
    return m.filepicker.View()
}
```

---

## 13. Timer and Stopwatch

### 13.1 Timer

```go
import "github.com/charmbracelet/bubbles/timer"

type Model struct {
    timer timer.Model
}

func NewModel() Model {
    t := timer.NewWithInterval(5*time.Minute, time.Second)
    return Model{timer: t}
}

func (m Model) Init() tea.Cmd {
    return m.timer.Init()
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    var cmd tea.Cmd
    m.timer, cmd = m.timer.Update(msg)

    switch msg.(type) {
    case timer.TickMsg:
        if m.timer.Timedout() {
            return m, tea.Quit
        }
    }

    return m, cmd
}

func (m Model) View() string {
    return m.timer.View()  // Shows "04:59"
}
```

### 13.2 Stopwatch

```go
import "github.com/charmbracelet/bubbles/stopwatch"

type Model struct {
    stopwatch stopwatch.Model
}

func NewModel() Model {
    return Model{stopwatch: stopwatch.NewWithInterval(time.Millisecond)}
}

func (m Model) Init() tea.Cmd {
    return m.stopwatch.Init()
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    var cmd tea.Cmd
    m.stopwatch, cmd = m.stopwatch.Update(msg)
    return m, cmd
}

func (m Model) View() string {
    return m.stopwatch.View()  // Shows "00:00:01"
}
```

---

## 14. Component Composition Patterns

### 14.1 Full Application Example

```go
type Model struct {
    // State
    state     AppState
    selected  string

    // Components
    spinner   spinner.Model
    textinput textinput.Model
    list      list.Model
    viewport  viewport.Model
    help      help.Model

    // Keymap
    keymap KeyMap
}

func NewModel() Model {
    // Initialize spinner
    s := spinner.New()
    s.Spinner = spinner.Dot
    s.Style = lipgloss.NewStyle().Foreground(lipgloss.Color("205"))

    // Initialize textinput
    ti := textinput.New()
    ti.Placeholder = "Search..."
    ti.Width = 30

    // Initialize list
    items := []list.Item{/*...*/}
    l := list.New(items, list.NewDefaultDelegate(), 40, 10)
    l.Title = "Items"

    // Initialize viewport
    v := viewport.New(60, 15)

    return Model{
        spinner:   s,
        textinput: ti,
        list:      l,
        viewport:  v,
        help:      help.New(),
        keymap:    DefaultKeyMap,
    }
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    var cmds []tea.Cmd

    // Update spinner
    var cmd tea.Cmd
    m.spinner, cmd = m.spinner.Update(msg)
    cmds = append(cmds, cmd)

    // Update textinput (when focused)
    if m.textinput.Focused() {
        m.textinput, cmd = m.textinput.Update(msg)
        cmds = append(cmds, cmd)
    }

    // Update list
    m.list, cmd = m.list.Update(msg)
    cmds = append(cmds, cmd)

    // Update viewport
    m.viewport, cmd = m.viewport.Update(msg)
    cmds = append(cmds, cmd)

    // Handle global keys
    switch msg := msg.(type) {
    case tea.KeyMsg:
        switch {
        case key.Matches(msg, m.keymap.Quit):
            return m, tea.Quit
        case key.Matches(msg, m.keymap.Search):
            m.textinput.Focus()
        case key.Matches(msg, m.keymap.Escape):
            m.textinput.Blur()
        }
    }

    return m, tea.Batch(cmds...)
}

func (m Model) View() string {
    var b strings.Builder

    // Header with spinner
    b.WriteString(m.spinner.View())
    b.WriteString(" Loading...\n\n")

    // Search input
    b.WriteString(m.textinput.View())
    b.WriteString("\n\n")

    // List and viewport side by side
    body := lipgloss.JoinHorizontal(
        lipgloss.Top,
        m.list.View(),
        m.viewport.View(),
    )
    b.WriteString(body)
    b.WriteString("\n\n")

    // Help
    b.WriteString(m.help.View(m.keymap))

    return b.String()
}
```

---

## Key Takeaways

1. **Consistent interface**: All components implement Update/View
2. **State encapsulation**: Each component manages its own state
3. **Delegation pattern**: Parent delegates Update to children
4. **Batch commands**: Combine child commands with tea.Batch
5. **Focus management**: Track which component has focus
6. **Styling with Lip Gloss**: All components support custom styles
7. **Key bindings**: Use key.Binding for consistent key handling
8. **Composition**: Build complex UIs from simple components

---

*Continue to [rust-revision.md](rust-revision.md) for Rust/ratatui translation.*
