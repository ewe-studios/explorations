# Example 1: Building a Simple Chat Client

This example shows how to build a minimal chat client that communicates with an AI API.

## Code

```rust
use reqwest::blocking::Client;
use serde::{Serialize, Deserialize};

// Request types
#[derive(Serialize)]
struct MessageRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

// Response types
#[derive(Deserialize)]
struct MessageResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: String,
}

// Simple chat client
struct ChatClient {
    client: Client,
    api_key: String,
}

impl ChatClient {
    fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    fn send(&self, messages: Vec<Message>) -> Result<String, Box<dyn std::error::Error>> {
        let request = MessageRequest {
            model: "claude-sonnet-4-6".to_string(),
            messages,
            max_tokens: 1000,
        };

        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()?
            .json::<MessageResponse>()?;

        Ok(response.content[0].text.clone())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .expect("Set ANTHROPIC_API_KEY environment variable");

    let client = ChatClient::new(api_key);

    let messages = vec![
        Message {
            role: "user".to_string(),
            content: "Hello! Can you help me with Rust?".to_string(),
        }
    ];

    let response = client.send(messages)?;
    println!("Response: {}", response);

    Ok(())
}
```

## Key Concepts

1. **HTTP Client**: Using `reqwest` for HTTP communication
2. **Serialization**: Using `serde` for JSON encoding/decoding
3. **Error Handling**: Using `Result` with trait objects for flexibility
4. **Environment Variables**: Securely loading API keys

## Run It

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
cargo run
```

---

*Generated: 2026-04-02*
