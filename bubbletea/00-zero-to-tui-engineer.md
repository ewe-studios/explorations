---
title: "Zero to TUI Engineer: A First-Principles Journey Through Bubble Tea"
subtitle: "Complete textbook-style guide from terminal fundamentals to ANSI escape sequences and event loops"
based_on: "Bubble Tea - The Elm Architecture for Terminal User Interfaces"
level: "Beginner to Intermediate - No prior TUI knowledge assumed"
---

# Zero to TUI Engineer: First-Principles Guide

## Table of Contents

1. [What Are Terminal User Interfaces?](#1-what-are-terminal-user-interfaces)
2. [Terminal Fundamentals](#2-terminal-fundamentals)
3. [ANSI Escape Sequences](#3-ansi-escape-sequences)
4. [Raw Mode vs Cooked Mode](#4-raw-mode-vs-cooked-mode)
5. [Event Loops](#5-event-loops)
6. [The Elm Architecture Overview](#6-the-elm-architecture-overview)
7. [Your First TUI Program](#7-your-first-tui-program)
8. [Your Learning Path](#8-your-learning-path)

---

## 1. What Are Terminal User Interfaces?

### 1.1 The Fundamental Question

**What is a TUI?**

A Terminal User Interface (TUI) is a user interface that operates within a text terminal, using characters, colors, and text formatting to create interactive applications.

```
┌─────────────────────────────────────────────────────────┐
│                    TUI Application                       │
│  ┌──────────────────────────────────────────────────┐   │
│  │  My Application                           [_][□] │   │
│  ├──────────────────────────────────────────────────┤   │
│  │                                                  │   │
│  │  Welcome to the TUI!                             │   │
│  │                                                  │   │
│  │  [ ] Option A    This is a TUI running           │   │
│  │  [✓] Option B    inside your terminal.           │   │
│  │  [ ] Option C    It uses text characters         │   │
│  │                to draw UI elements.              │   │
│  │                                                  │   │
│  │  Press 'q' to quit                               │   │
│  │                                                  │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

**TUIs vs GUIs vs CLIs:**

| Aspect | CLI | TUI | GUI |
|--------|-----|-----|-----|
| **Interaction** | Command + Enter | Keyboard/Mouse (real-time) | Mouse/Touch |
| **Display** | Text output only | Dynamic text + colors | Pixels, widgets |
| **Speed** | Fast (one-shot) | Fast (continuous) | Slower (rendering) |
| **Bandwidth** | Low | Very low | High |
| **Remote** | Excellent (SSH) | Excellent (SSH) | Requires X11/VNC |
| **Learning** | High (commands) | Medium (keys) | Low (visual) |

### 1.2 Why TUIs Matter in 2026

**TUIs are experiencing a renaissance:**

1. **SSH-First Development**: Cloud servers, containers, remote development
2. **Low Resource Usage**: Runs on minimal hardware, no GPU needed
3. **Keyboard-Centric**: Power users prefer keyboard over mouse
4. **Composability**: Pipe output, integrate with shell scripts
5. **Accessibility**: Screen readers, high contrast, terminal multiplexers

**Real-World TUI Applications:**

| Application | Purpose | Technology |
|-------------|---------|------------|
| `htop` | Process monitoring | ncurses (C) |
| `lazygit` | Git TUI | Go, tcell |
| `gum` | Shell scripting | Bubble Tea (Go) |
| `glow` | Markdown viewer | Bubble Tea (Go) |
| `bottom` | System monitor | ratatui (Rust) |
| `spotify-tui` | Spotify client | ratatui (Rust) |

---

## 2. Terminal Fundamentals

### 2.1 What Is a Terminal?

A **terminal** is a program that provides a text-based interface to a shell (command interpreter).

```
┌─────────────────────────────────────────────────────────┐
│                    User's Keyboard                       │
│                          │                               │
│                          ▼                               │
│  ┌─────────────────────────────────────────────────┐    │
│  │              Terminal Emulator                   │    │
│  │  (iTerm2, Kitty, Alacritty, Windows Terminal)   │    │
│  │                                                  │    │
│  │  ┌────────────────────────────────────────────┐ │    │
│  │  │            Pseudo-Terminal (PTY)           │ │    │
│  │  │     ┌────────────┐    ┌──────────────┐    │ │    │
│  │  │     │   Master   │◄──►│    Slave     │    │ │    │
│  │  │     └─────┬──────┘    └──────┬───────┘    │ │    │
│  │  │           │                  │            │ │    │
│  │  └───────────┼──────────────────┼────────────┘ │    │
│  │              │                  │              │    │
│  └──────────────┼──────────────────┼──────────────┘    │
│                 │                  │                    │
│                 ▼                  ▼                    │
│          Terminal Driver      Shell Process            │
│         (Kernel Space)       (bash, zsh, fish)         │
└─────────────────────────────────────────────────────────┘
```

### 2.2 The Terminal Grid

Terminals are organized as a **grid of character cells**:

```
┌─────────────────────────────────────────────────────────┐
│ (0,0)                                         (79,0)   │  Row 0
│  H e l l o ,       W o r l d !                          │
│                                                          │  Row 1
│  Current time: 14:32:45                                 │  Row 2
│  CPU: 45%  Memory: 2.1GB / 8GB                          │  Row 3
│  [████████████░░░░░░░░░░] 56%                           │  Row 4
│                                                          │  Row 5
│  Items:                                                  │  Row 6
│  > Item 1                                                │  Row 7
│    Item 2                                                │  Row 8
│    Item 3                                                │  Row 9
│                                                          │
│ (0,23)                                        (79,23)  │  Row 23
└─────────────────────────────────────────────────────────┘
        ▲                                      ▲
     Column 0                               Column 79

Standard terminal: 80 columns × 24 rows
Modern terminal: Often 120×40 or larger
```

**Key Properties:**

- **Cell**: Single character position (column × row)
- **Cursor**: Current position for text insertion (visible or hidden)
- **Viewport**: Visible portion of the terminal (may scroll)
- **Scrollback**: Buffer of lines that scrolled off the top

### 2.3 Character Encoding and Runes

Modern terminals use **Unicode (UTF-8)** encoding:

```go
// Go: runes are Unicode code points
var r rune = '猫'  // Chinese character for "cat"
var s string = "Hello, 世界"  // "Hello, World" in Chinese

// Important: Not all characters are 1 cell wide!
"a"   // 1 rune, 1 cell
"猫"  // 1 rune, 2 cells (wide character)
"é"   // Could be 1 or 2 runes depending on normalization
```

**Bubble Tea handles this automatically:**

```go
import "github.com/mattn/go-runewidth"

width := runewidth.StringWidth("Hello, 猫")
// Returns: 9 (7 for "Hello, " + 2 for "猫")
```

---

## 3. ANSI Escape Sequences

### 3.1 What Are Escape Sequences?

**ANSI escape sequences** are special character sequences that control terminal behavior.

```
Escape Sequence = ESC + '[' + Parameters + Command

Where:
- ESC is ASCII character 27 (0x1B, \e, \033)
- '[' is the Control Sequence Introducer (CSI)
- Parameters are numbers separated by ';'
- Command is a single letter
```

**Example:**

```bash
# Move cursor to row 10, column 20
echo -e "\e[10;20H"

# Set text color to red
echo -e "\e[31mRed Text\e[0m"

# Clear screen
echo -e "\e[2J"
```

### 3.2 Cursor Control

| Sequence | Name | Effect |
|----------|------|--------|
| `\e[H` or `\e[1;1H` | Cursor Home | Move to top-left |
| `\e[row;colH` | Cursor Position | Move to specific position |
| `\e[A` | Cursor Up | Move up 1 row |
| `\e[B` | Cursor Down | Move down 1 row |
| `\e[C` | Cursor Right | Move right 1 column |
| `\e[D` | Cursor Left | Move left 1 column |
| `\e[s` | Save Cursor | Save current position |
| `\e[u` | Restore Cursor | Restore saved position |
| `\e[?25h` | Show Cursor | Make cursor visible |
| `\e[?25l` | Hide Cursor | Hide cursor |

**Bubble Tea Usage:**

```go
import "github.com/charmbracelet/x/ansi"

// Hide cursor
fmt.Print(ansi.HideCursor)  // \e[?25l

// Move cursor to position
fmt.Print(ansi.CursorPosition(10, 20))  // \e[10;20H

// Clear screen
fmt.Print(ansi.EraseEntireScreen)  // \e[2J
```

### 3.3 Colors and Styles

**16 ANSI Colors (4-bit):**

| Code | Color | Code | Color (Bright) |
|------|-------|------|----------------|
| 30 | Black | 90 | Bright Black |
| 31 | Red | 91 | Bright Red |
| 32 | Green | 92 | Bright Green |
| 33 | Yellow | 93 | Bright Yellow |
| 34 | Blue | 94 | Bright Blue |
| 35 | Magenta | 95 | Bright Magenta |
| 36 | Cyan | 96 | Bright Cyan |
| 37 | White | 97 | Bright White |

**256 Colors (8-bit):**

```bash
# Set foreground to color 201 (hot pink)
echo -e "\e[38;5;201mHot Pink\e[0m"

# Set background to color 22 (dark green)
echo -e "\e[48;5;22mDark Green BG\e[0m"
```

**True Color (24-bit / 16.7 million colors):**

```bash
# RGB color: #FF5733 (orange-red)
echo -e "\e[38;2;255;87;51mTrue Color\e[0m"

# Background RGB: #2D46B9 (blue)
echo -e "\e[48;2;45;70;185mBlue BG\e[0m"
```

**Text Attributes:**

| Code | Name | Effect |
|------|------|--------|
| 0 | Reset | All attributes off |
| 1 | Bold | Bold or increased intensity |
| 2 | Faint | Dim or decreased intensity |
| 3 | Italic | Italic text |
| 4 | Underline | Underlined text |
| 5 | Slow Blink | Slow blink |
| 7 | Reverse | Swap foreground and background |
| 8 | Conceal | Hidden text |
| 9 | Strikethrough | Crossed-out text |

**Lip Gloss Makes This Easy:**

```go
import "github.com/charmbracelet/lipgloss"

var style = lipgloss.NewStyle().
    Bold(true).
    Italic(true).
    Foreground(lipgloss.Color("#FF5733")).
    Background(lipgloss.Color("22")).
    Strikethrough(true)

fmt.Println(style.Render("Styled Text"))
```

### 3.4 Screen Manipulation

| Sequence | Name | Effect |
|----------|------|--------|
| `\e[2J` | Clear Screen | Erase entire screen |
| `\e[K` or `\e[0K` | Clear to EOL | Erase from cursor to end of line |
| `\e[1K` | Clear to BOL | Erase from cursor to beginning of line |
| `\e[2K` | Clear Line | Erase entire line |
| `\e[2J` | Clear All | Erase entire screen |
| `\e[3J` | Clear Scrollback | Erase scrollback buffer |
| `\e[S` | Scroll Up | Scroll up N lines |
| `\e[T` | Scroll Down | Scroll down N lines |

### 3.5 Mouse Events

Modern terminals support **mouse tracking**:

```bash
# Enable mouse tracking
echo -e "\e[?1006;1000h"

# Mouse events are reported as:
# \e[<button;x;y{M|m}
# M = button press, m = button release
# button: 0=left, 1=middle, 2=right, 64=wheel up, 65=wheel down
```

**Bubble Tea Mouse Handling:**

```go
func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg := msg.(type) {
    case tea.MouseMsg:
        if msg.Action == tea.MouseActionPress {
            switch msg.Button {
            case tea.MouseButtonLeft:
                // Left click at (msg.X, msg.Y)
            case tea.MouseButtonWheelUp:
                // Scroll up
            }
        }
    }
    return m, nil
}
```

---

## 4. Raw Mode vs Cooked Mode

### 4.1 Cooked Mode (Canonical Mode)

**Cooked mode** is the default terminal mode:

```
┌─────────────────────────────────────────────────────────┐
│                    Cooked Mode                            │
│                                                          │
│  • Line buffering: Input sent on ENTER                  │
│  • Line editing: BACKSPACE, DELETE, arrow keys          │
│  • Signal handling: Ctrl+C (SIGINT), Ctrl+Z (SIGTSTP)  │
│  • Echo: Characters displayed as typed                  │
│  • Special chars: Ctrl+D (EOF), Ctrl+U (kill line)     │
└─────────────────────────────────────────────────────────┘
```

**Example:**

```bash
$ read name  # Waits for ENTER
Alice        # You type "Alice" and press Enter
$ echo $name
Alice
```

### 4.2 Raw Mode

**Raw mode** disables all terminal processing:

```
┌─────────────────────────────────────────────────────────┐
│                     Raw Mode                             │
│                                                          │
│  • No buffering: Every keystroke sent immediately       │
│  • No editing: BACKSPACE is just another character      │
│  • No signals: Ctrl+C is a regular keypress             │
│  • No echo: Program must display characters             │
│  • Full control: Program handles everything             │
└─────────────────────────────────────────────────────────┘
```

**Why TUIs Need Raw Mode:**

1. **Real-time input**: Games, interactive apps
2. **Custom keybindings**: Vim-style navigation
3. **Arrow keys**: Get escape sequences directly
4. **Mouse support**: Enable mouse tracking
5. **Custom rendering**: Control every character

### 4.3 Entering and Exiting Raw Mode

**Go (Bubble Tea):**

```go
import (
    "golang.org/x/term"
    "os"
)

// Save current state
oldState, err := term.MakeRaw(int(os.Stdin.Fd()))
if err != nil {
    // Handle error
}

// ... do TUI stuff ...

// Restore state (CRITICAL!)
term.Restore(int(os.Stdin.Fd()), oldState)
```

**Bubble Tea handles this automatically:**

```go
func (p *Program) initTerminal() error {
    // Save current terminal state
    p.previousOutputState, _ = term.MakeRaw(p.ttyOutput.Fd())
    p.previousTtyInputState, _ = term.MakeRaw(p.ttyInput.Fd())

    // ... configure renderer, hide cursor, etc. ...

    return nil
}

func (p *Program) restoreTerminalState() error {
    // Restore original terminal state
    term.Restore(p.ttyOutput.Fd(), p.previousOutputState)
    term.Restore(p.ttyInput.Fd(), p.previousTtyInputState)
    return nil
}
```

### 4.4 What Bubble Tea Configures

When Bubble Tea starts, it:

1. **Enters raw mode** for stdin and stdout
2. **Hides the cursor** (`\e[?25l`)
3. **Enables alternate screen buffer** (`\e[?1049h`)
4. **Enables mouse tracking** (if requested)
5. **Enables bracketed paste** (if supported)
6. **Sets up signal handlers** (SIGINT, SIGTERM, SIGWINCH)

When Bubble Tea exits, it:

1. **Restores terminal state** from saved values
2. **Shows the cursor** (`\e[?25h`)
3. **Exits alternate screen buffer** (`\e[?1049l`)
4. **Disables mouse tracking**
5. **Disables bracketed paste**

**Alternate Screen Buffer:**

```
┌─────────────────────────────────────────────────────────┐
│                  Main Screen Buffer                      │
│  (Your shell prompt, command history, previous output)  │
│  $ my-tui-app                                            │
│  ┌─────────────────────────────────────────────────────┐│
│  │           TUI Application Running                   ││
│  │                                                     ││
│  │   Press any key...                                  ││
│  └─────────────────────────────────────────────────────┘│
│                                                          │
└─────────────────────────────────────────────────────────┘
                          │
                          │ When TUI starts
                          ▼
┌─────────────────────────────────────────────────────────┐
│               Alternate Screen Buffer                    │
│  (Clean slate for TUI, main buffer preserved)           │
│  ┌─────────────────────────────────────────────────────┐│
│  │           TUI Application Running                   ││
│  │                                                     ││
│  │   Press any key...                                  ││
│  └─────────────────────────────────────────────────────┘│
│                                                          │
│                                                          │
└─────────────────────────────────────────────────────────┘
                          │
                          │ When TUI exits
                          ▼
┌─────────────────────────────────────────────────────────┐
│                  Main Screen Buffer                      │
│  (Everything restored exactly as before)                │
│  $ my-tui-app                                            │
│  $                                                       │  ← Ready for next command
│                                                          │
└─────────────────────────────────────────────────────────┘
```

---

## 5. Event Loops

### 5.1 What Is an Event Loop?

An **event loop** is a programming construct that waits for and dispatches events or messages.

```
┌─────────────────────────────────────────────────────────┐
│                    Event Loop                            │
│                                                          │
│  while running:                                          │
│      1. Wait for events (input, timer, network)         │
│      2. Read event                                       │
│      3. Process event (update state)                    │
│      4. Render updated state                            │
│      5. Go to step 1                                     │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### 5.2 Bubble Tea's Event Loop

**Simplified eventLoop:**

```go
func (p *Program) eventLoop(model Model, cmds chan Cmd) (Model, error) {
    for {
        select {
        case <-p.ctx.Done():
            return model, nil

        case err := <-p.errs:
            return model, err

        case msg := <-p.msgs:
            // 1. Filter message (optional)
            if p.filter != nil {
                msg = p.filter(model, msg)
            }
            if msg == nil {
                continue
            }

            // 2. Handle special internal messages
            switch msg := msg.(type) {
            case QuitMsg:
                return model, nil
            case WindowSizeMsg:
                // Handle resize
            case BatchMsg:
                // Handle batch commands
            }

            // 3. Update model
            var cmd Cmd
            model, cmd = model.Update(msg)

            // 4. Send command to executor
            cmds <- cmd

            // 5. Render view
            p.renderer.write(model.View())
        }
    }
}
```

### 5.3 Input Reading Loop

**Separate goroutine reads input:**

```go
func (p *Program) readEvents() {
    for {
        buf := make([]byte, 256)
        n, err := p.cancelReader.Read(buf)
        if err != nil {
            return
        }

        // Parse ANSI escape sequences
        msgs := p.parseInput(buf[:n])

        // Send parsed messages to main loop
        for _, msg := range msgs {
            p.msgs <- msg
        }
    }
}

func (p *Program) parseInput(buf []byte) []Msg {
    var msgs []Msg

    // Parse key sequences
    for len(buf) > 0 {
        msg, width := parseKeySequence(buf)
        if msg != nil {
            msgs = append(msgs, msg)
        }
        buf = buf[width:]
    }

    return msgs
}
```

### 5.4 Key Sequence Parsing

**Escape sequences for special keys:**

```go
// Common key sequences
var keySequences = map[string]KeyType{
    "\x1b[A": KeyUp,     // Up arrow
    "\x1b[B": KeyDown,   // Down arrow
    "\x1b[C": KeyRight,  // Right arrow
    "\x1b[D": KeyLeft,   // Left arrow
    "\x1b[3~": KeyDelete,
    "\x1b[1~": KeyHome,
    "\x1b[4~": KeyEnd,
    "\x1b[5~": KeyPgUp,
    "\x1b[6~": KeyPgDown,
    "\x1b\n": KeyEnter,  // Alt+Enter
    "\x1b ": KeySpace,   // Alt+Space
}

func parseKeySequence(buf []byte) (Msg, int) {
    // Check for escape sequences
    if buf[0] == 0x1b {
        // ESC character - might be escape key or sequence
        if len(buf) >= 3 {
            seq := string(buf[:3])
            if keyType, ok := keySequences[seq]; ok {
                return KeyMsg{Type: keyType}, 3
            }
        }
        // Just the escape key
        return KeyMsg{Type: KeyEscape}, 1
    }

    // Regular character
    r, width := utf8.DecodeRune(buf)
    return KeyMsg{Type: KeyRunes, Runes: []rune{r}}, width
}
```

### 5.5 Command Execution Loop

**Commands run concurrently:**

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
                // Run command in goroutine
                go func() {
                    msg := cmd()  // Execute command
                    p.Send(msg)   // Send result to main loop
                }()
            }
        }
    }()

    return ch
}
```

**Example Commands:**

```go
// HTTP request command
func fetchUserData(id string) tea.Cmd {
    return func() tea.Msg {
        resp, err := http.Get("/users/" + id)
        if err != nil {
            return errMsg{err}
        }
        defer resp.Body.Close()
        // Parse response...
        return userDataMsg{data}
    }
}

// Timer command
func tickAfter(d time.Duration) tea.Cmd {
    return func() tea.Msg {
        time.Sleep(d)
        return TickMsg{}
    }
}

// File read command
func readFile(path string) tea.Cmd {
    return func() tea.Msg {
        data, err := os.ReadFile(path)
        if err != nil {
            return errMsg{err}
        }
        return fileReadMsg{data}
    }
}
```

---

## 6. The Elm Architecture Overview

### 6.1 Core Concepts

The **Elm Architecture** consists of three parts:

```
┌─────────────────────────────────────────────────────────┐
│                   Elm Architecture                       │
│                                                          │
│  1. Model                                                │
│     - Application state                                  │
│     - Pure data structure                                │
│     - Immutable (new model on each update)              │
│                                                          │
│  2. View                                                 │
│     - Model → UI representation                          │
│     - Pure function (same input = same output)          │
│     - Returns string (ANSI-rendered)                    │
│                                                          │
│  3. Update                                               │
│     - (Model, Msg) → (Model, Cmd)                        │
│     - Pure function                                      │
│     - Returns new model and optional command            │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### 6.2 Data Flow

```
                    ┌──────────────┐
                    │    Model     │
                    │  (initial)   │
                    └──────┬───────┘
                           │ Init()
                           │ (optional Cmd)
                           ▼
                    ┌──────────────┐
                    │     View     │
                    │  (render UI) │
                    └──────┬───────┘
                           │ Display
                           ▼
              ┌────────────────────────┐
              │      Terminal UI       │
              └────────────┬───────────┘
                           │ User Input
                           ▼
                    ┌──────────────┐
                    │    Message   │
                    │  (Key, Click)│
                    └──────┬───────┘
                           │ Update()
                           ▼
              ┌────────────────────────┐
              │  (Model, Cmd) Result   │
              │  - New Model           │
              │  - Optional Command    │
              └────────────┬───────────┘
                           │
            ┌──────────────┴──────────────┐
            │                             │
            ▼                             ▼
     ┌──────────────┐            ┌──────────────┐
     │     View     │            │   Execute    │
     │  (re-render) │            │    Cmd       │
     └──────────────┘            └──────┬───────┘
                                        │ Returns Msg
                                        ▼
                                 ┌──────────────┐
                                 │    Message   │
                                 │  (Cmd Result)│
                                 └──────────────┘
                                        │
                                        └──────┐
                                               │
                                    (back to Update)
```

### 6.3 Example: Counter Application

```go
package main

import (
    "fmt"
    "os"
    "github.com/charmbracelet/bubbletea"
)

// 1. Model: Application state
type Model struct {
    counter int
}

// 2. Init: Initial command (none for counter)
func (m Model) Init() tea.Cmd {
    return nil
}

// 3. Update: Handle messages
func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg := msg.(type) {
    case tea.KeyMsg:
        switch msg.String() {
        case "q", "ctrl+c":
            return m, tea.Quit
        case "+", "up":
            m.counter++
        case "-", "down":
            m.counter--
        }
    }
    return m, nil
}

// 4. View: Render UI
func (m Model) View() string {
    return fmt.Sprintf(
        "\n  Counter: %d\n\n  +/up: increment\n  -/down: decrement\n  q: quit\n",
        m.counter,
    )
}

// 5. Main: Run program
func main() {
    p := tea.NewProgram(Model{})
    if _, err := p.Run(); err != nil {
        fmt.Printf("Error: %v\n", err)
        os.Exit(1)
    }
}
```

---

## 7. Your First TUI Program

### 7.1 Step 1: Setup

```bash
mkdir my-tui
cd my-tui
go mod init my-tui
go get github.com/charmbracelet/bubbletea
```

### 7.2 Step 2: Basic Structure

```go
package main

import (
    "github.com/charmbracelet/bubbletea"
)

type Model struct {
    // Add your state here
}

func (m Model) Init() tea.Cmd {
    return nil
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg := msg.(type) {
    case tea.KeyMsg:
        if msg.String() == "q" {
            return m, tea.Quit
        }
    }
    return m, nil
}

func (m Model) View() string {
    return "Hello, TUI!\nPress 'q' to quit.\n"
}

func main() {
    tea.NewProgram(Model{}).Run()
}
```

### 7.3 Step 3: Run It

```bash
go run main.go
```

You should see:

```
Hello, TUI!
Press 'q' to quit.
```

Press 'q' to exit.

### 7.4 Step 4: Add Interactivity

```go
type Model struct {
    message string
    cursor  int
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg := msg.(type) {
    case tea.KeyMsg:
        switch msg.String() {
        case "q", "ctrl+c":
            return m, tea.Quit
        case "up", "k":
            m.cursor--
        case "down", "j":
            m.cursor++
        case "enter":
            m.message = "You selected item " + string(rune(m.cursor+'0'))
        }
    }
    return m, nil
}

func (m Model) View() string {
    var b strings.Builder
    fmt.Fprintf(&b, "Selected: %s\n\n", m.message)
    for i := 0; i < 5; i++ {
        if i == m.cursor {
            fmt.Fprintf(&b, "> Item %d\n", i)
        } else {
            fmt.Fprintf(&b, "  Item %d\n", i)
        }
    }
    return b.String()
}
```

---

## 8. Your Learning Path

### 8.1 Beginner Path

1. **Understand terminals**: Read sections 2-4 of this document
2. **Build counter app**: Follow section 7
3. **Add styling**: Learn Lip Gloss basics
4. **Use bubbles**: Try textinput, spinner components

### 8.2 Intermediate Path

1. **Read deep dives**: [01-elm-architecture-deep-dive.md](01-elm-architecture-deep-dive.md)
2. **Build real app**: Clone a simple tool (todo list, calculator)
3. **Handle commands**: Learn async operations with tea.Cmd
4. **Compose components**: Build reusable sub-components

### 8.3 Advanced Path

1. **Study rendering**: [02-rendering-pipeline-deep-dive.md](02-rendering-pipeline-deep-dive.md)
2. **Optimize performance**: Diff rendering, minimize allocations
3. **Build complex app**: Multi-view, state persistence
4. **Translate to Rust**: Learn ratatui patterns

### 8.4 Recommended Resources

- [Bubble Tea Tutorials](https://github.com/charmbracelet/bubbletea/tree/master/tutorials)
- [Bubbles Examples](https://github.com/charmbracelet/bubbles/tree/master/examples)
- [ANSI Escape Codes](https://en.wikipedia.org/wiki/ANSI_escape_code)
- [Terminal Guide](https://terminalguide.namepad.de/)

---

## Key Takeaways

1. **Terminals are grids**: Character cells organized in rows × columns
2. **ANSI sequences control terminals**: Escape codes for cursor, colors, clearing
3. **Raw mode gives control**: Direct access to keystrokes, no buffering
4. **Event loops drive TUIs**: Read input → Update → Render → Repeat
5. **Elm Architecture is clean**: Model, View, Update with pure functions
6. **Bubble Tea handles complexity**: Terminal setup, input parsing, rendering

---

*This document is part of the complete Bubble Tea exploration. Continue to [01-elm-architecture-deep-dive.md](01-elm-architecture-deep-dive.md) for the core architecture.*
