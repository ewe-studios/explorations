# Example 3: Terminal Spinner Animation

This example shows how to create an animated spinner for terminal output.

## Code

```rust
use std::io::{self, Write, stdout};
use std::time::Duration;
use std::thread;
use std::sync::atomic::{AtomicBool, Ordering};

// Spinner frames (Braille characters)
const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

// ANSI escape codes
const CLEAR_LINE: &str = "\x1b[2K";      // Clear entire line
const MOVE_TO_START: &str = "\r";        // Carriage return
const RESET: &str = "\x1b[0m";           // Reset all attributes
const BLUE: &str = "\x1b[34m";           // Blue foreground

pub struct Spinner {
    frame: usize,
    label: String,
}

impl Spinner {
    fn new(label: impl Into<String>) -> Self {
        Self {
            frame: 0,
            label: label.into(),
        }
    }

    fn tick(&mut self) -> io::Result<()> {
        let frame = FRAMES[self.frame % FRAMES.len()];
        self.frame += 1;

        let mut stdout = stdout();
        write!(stdout, "{}{}{} {} {}", MOVE_TO_START, CLEAR_LINE, frame, BLUE, self.label)?;
        write!(stdout, "{}", RESET)?;
        stdout.flush()?;

        Ok(())
    }

    fn finish(&mut self, success: bool) -> io::Result<()> {
        let mut stdout = stdout();
        let symbol = if success { "✔" } else { "✘" };
        let color = if success { "\x1b[32m" } else { "\x1b[31m" };  // Green or Red

        write!(stdout, "{}{}{} {} {}", MOVE_TO_START, CLEAR_LINE, symbol, color, self.label)?;
        write!(stdout, "{}\n", RESET)?;
        stdout.flush()?;

        Ok(())
    }
}

// Simulated long-running task
fn long_task() {
    thread::sleep(Duration::from_secs(3));
}

fn main() -> io::Result<()> {
    let mut spinner = Spinner::new("Processing...");

    // Animate for 3 seconds
    for _ in 0..30 {
        spinner.tick()?;
        thread::sleep(Duration::from_millis(100));
    }

    spinner.finish(true)
}
```

## Output (Animated)

```
⠋ Processing...
⠙ Processing...
⠹ Processing...
⠸ Processing...
⠼ Processing...
... (frames cycle)
✔ Processing...
```

## Key Concepts

1. **ANSI Escape Codes**: Control cursor and colors
2. **Frame Animation**: Cycle through characters
3. **Buffer Flushing**: Force immediate output
4. **Line Clearing**: Redraw same line repeatedly

## ANSI Codes Reference

| Code | Description |
|------|-------------|
| `\r` | Carriage return (move to start of line) |
| `\x1b[2K` | Clear entire line |
| `\x1b[34m` | Blue foreground |
| `\x1b[32m` | Green foreground |
| `\x1b[31m` | Red foreground |
| `\x1b[0m` | Reset all attributes |

## Run It

```bash
cargo run
```

## Advanced: Multi-threaded Spinner

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub fn spawn_spinner(label: &str) -> Arc<AtomicBool> {
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = Arc::clone(&running);
    let label = label.to_string();

    thread::spawn(move || {
        let mut spinner = Spinner::new(label);
        while running_clone.load(Ordering::Relaxed) {
            let _ = spinner.tick();
            thread::sleep(Duration::from_millis(100));
        }
        let _ = spinner.finish(true);
    });

    running
}

// Usage
let spinner = spawn_spinner("Loading...");

// Do work
long_task();

// Stop spinner
spinner.store(false, Ordering::Relaxed);
thread::sleep(Duration::from_millis(200));  // Let spinner finish
```

---

*Generated: 2026-04-02*
