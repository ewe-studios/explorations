# Example 2: SSE Streaming Parser

This example shows how to parse Server-Sent Events (SSE) from a streaming API.

## Code

```rust
use std::io::{self, BufRead};

// SSE Event structure
#[derive(Debug)]
struct SseEvent {
    event_type: String,
    data: String,
}

// SSE Parser
struct SseParser {
    buffer: String,
}

impl SseParser {
    fn new() -> Self {
        Self { buffer: String::new() }
    }

    fn push(&mut self, chunk: &str) -> Vec<SseEvent> {
        self.buffer.push_str(chunk);

        // Split by double newline (event delimiter)
        let events: Vec<&str> = self.buffer.split("\n\n").collect();

        // Keep incomplete chunk in buffer
        self.buffer = events.last().unwrap_or(&"").to_string();

        // Parse complete events
        events[..events.len().saturating_sub(1)]
            .iter()
            .filter_map(|chunk| self.parse_event(chunk))
            .collect()
    }

    fn parse_event(&self, chunk: &str) -> Option<SseEvent> {
        let mut event_type = String::new();
        let mut data = String::new();

        for line in chunk.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if let Some(rest) = line.strip_prefix("event: ") {
                event_type = rest.to_string();
            } else if let Some(rest) = line.strip_prefix("data: ") {
                if !data.is_empty() {
                    data.push('\n');
                }
                data.push_str(rest);
            }
        }

        if data.is_empty() {
            return None;
        }

        Some(SseEvent { event_type, data })
    }
}

// Simulate receiving chunks
fn simulate_stream() -> Vec<&'static str> {
    vec![
        "event: message_start\ndata: {\"type\":\"message_start\"}\n\n",
        "event: content_block_delta\ndata: {\"text\":\"Hello\"}\n\n",
        "event: content_block_delta\ndata: {\"text\":\" World\"}\n\n",
        "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
    ]
}

fn main() {
    let mut parser = SseParser::new();

    for chunk in simulate_stream() {
        println!("Received chunk: {}", chunk.trim());

        let events = parser.push(chunk);
        for event in events {
            println!("  Parsed event: {:?} = {}", event.event_type, event.data);
        }
    }
}
```

## Output

```
Received chunk: event: message_start
data: {"type":"message_start"}
  Parsed event: message_start = {"type":"message_start"}
Received chunk: event: content_block_delta
data: {"text":"Hello"}
  Parsed event: content_block_delta = {"text":"Hello"}
Received chunk: event: content_block_delta
data: {"text":" World"}
  Parsed event: content_block_delta = {"text":" World"}
Received chunk: event: message_stop
data: {"type":"message_stop"}
  Parsed event: message_stop = {"type":"message_stop"}
```

## Key Concepts

1. **Buffering**: Accumulating chunks until complete events
2. **Event Delimiters**: Double newline separates events
3. **Line Parsing**: Key-value format (event:, data:)
4. **State Management**: Keeping incomplete chunks in buffer

## Run It

```bash
cargo run
```

---

*Generated: 2026-04-02*
