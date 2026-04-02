# Ollama Provider Deep Dive

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/openclaude/ollama_provider.py`

---

## Overview

The Ollama Provider is a native Python integration that enables OpenClaude to route requests to locally-running Ollama models. This provides:

- **Zero-cost inference** (no API fees)
- **Complete privacy** (data never leaves your machine)
- **Low latency** (no network round-trip)
- **Offline capability** (works without internet)

---

## Architecture

### Integration Points

```
OpenClaude CLI
      │
      ▼
┌─────────────────┐
│  openaiShim.ts  │  TypeScript layer
└─────────────────┘
      │
      ▼ (HTTP to localhost:11434)
┌─────────────────┐
│  ollama_server  │  Ollama daemon
└─────────────────┘
      │
      ▼
┌─────────────────┐
│  llama3.3:70b   │  Loaded model
└─────────────────┘
```

### Python Integration

The `ollama_provider.py` module provides:

1. **Health checking** — Verify Ollama is running
2. **Model listing** — Discover available models
3. **Chat completion** — Non-streaming and streaming
4. **Message conversion** — Anthropic ↔ Ollama format

---

## Core Components

### Configuration

```python
import os

logger = logging.getLogger(__name__)
OLLAMA_BASE_URL = os.getenv("OLLAMA_BASE_URL", "http://localhost:11434")
```

**Environment Variables:**

| Variable | Default | Description |
|----------|---------|-------------|
| `OLLAMA_BASE_URL` | `http://localhost:11434` | Ollama server endpoint |

---

## Health Checking

### Check if Ollama is Running

```python
async def check_ollama_running() -> bool:
    """
    Verify Ollama server is reachable.
    
    Returns:
        bool: True if Ollama is running and responsive
    """
    try:
        async with httpx.AsyncClient(timeout=3.0) as client:
            resp = await client.get(f"{OLLAMA_BASE_URL}/api/tags")
            return resp.status_code == 200
    except Exception:
        return False
```

**Usage:**
```python
is_running = await check_ollama_running()
if not is_running:
    print("Ollama is not running. Start with: ollama serve")
```

### List Available Models

```python
async def list_ollama_models() -> list[str]:
    """
    Get list of installed Ollama models.
    
    Returns:
        list[str]: Model names (e.g., ["llama3.3:70b", "mistral:7b"])
    """
    try:
        async with httpx.AsyncClient(timeout=5.0) as client:
            resp = await client.get(f"{OLLAMA_BASE_URL}/api/tags")
            resp.raise_for_status()
            data = resp.json()
            return [m["name"] for m in data.get("models", [])]
    except Exception as e:
        logger.warning(f"Could not list Ollama models: {e}")
        return []
```

**Example Response:**
```json
{
  "models": [
    {"name": "llama3.3:70b", "size": 42363168466, "digest": "abc123"},
    {"name": "mistral:7b", "size": 4108906629, "digest": "def456"},
    {"name": "codellama:34b", "size": 19132192384, "digest": "ghi789"}
  ]
}
```

---

## Message Conversion

### Normalize Model Name

```python
def normalize_ollama_model(model_name: str) -> str:
    """
    Remove 'ollama/' prefix if present.
    
    Args:
        model_name: Model name (e.g., "ollama/llama3.3:70b")
    
    Returns:
        str: Normalized model name (e.g., "llama3.3:70b")
    """
    if model_name.startswith("ollama/"):
        return model_name[len("ollama/"):]
    return model_name
```

### Anthropic to Ollama Messages

```python
def anthropic_to_ollama_messages(messages: list[dict]) -> list[dict]:
    """
    Convert Anthropic-format messages to Ollama format.
    
    Handles:
    - Text content (string or array)
    - Image content (marked as [image] placeholder)
    - Multi-part messages
    
    Args:
        messages: Anthropic-format messages
    
    Returns:
        list[dict]: Ollama-format messages
    """
    ollama_messages = []
    
    for msg in messages:
        role = msg.get("role", "user")
        content = msg.get("content", "")
        
        # Simple string content
        if isinstance(content, str):
            ollama_messages.append({"role": role, "content": content})
        
        # Array of content blocks
        elif isinstance(content, list):
            text_parts = []
            
            for block in content:
                if isinstance(block, dict):
                    if block.get("type") == "text":
                        text_parts.append(block.get("text", ""))
                    elif block.get("type") == "image":
                        text_parts.append("[image]")
                elif isinstance(block, str):
                    text_parts.append(block)
            
            ollama_messages.append({
                "role": role,
                "content": "\n".join(text_parts)
            })
    
    return ollama_messages
```

**Example Conversion:**

Anthropic format:
```python
[
    {
        "role": "user",
        "content": [
            {"type": "text", "text": "What is this?"},
            {"type": "image", "source": {...}}
        ]
    }
]
```

Ollama format:
```python
[
    {
        "role": "user",
        "content": "What is this?\n[image]"
    }
]
```

---

## Chat Completion

### Non-Streaming Chat

```python
async def ollama_chat(
    model: str,
    messages: list[dict],
    system: str | None = None,
    max_tokens: int = 4096,
    temperature: float = 1.0,
) -> dict:
    """
    Complete a chat conversation (non-streaming).
    
    Args:
        model: Model name (e.g., "llama3.3:70b")
        messages: Conversation messages
        system: Optional system prompt
        max_tokens: Maximum tokens to generate
        temperature: Sampling temperature (0.0-2.0)
    
    Returns:
        dict: Anthropic-format response
    """
    # Normalize and convert
    model = normalize_ollama_model(model)
    ollama_messages = anthropic_to_ollama_messages(messages)
    
    # Prepend system message
    if system:
        ollama_messages.insert(0, {"role": "system", "content": system})
    
    # Build request
    payload = {
        "model": model,
        "messages": ollama_messages,
        "stream": False,
        "options": {
            "num_predict": max_tokens,
            "temperature": temperature,
        },
    }
    
    # Make request
    async with httpx.AsyncClient(timeout=120.0) as client:
        resp = await client.post(
            f"{OLLAMA_BASE_URL}/api/chat",
            json=payload
        )
        resp.raise_for_status()
        data = resp.json()
    
    # Extract response
    assistant_text = data.get("message", {}).get("content", "")
    
    # Convert to Anthropic format
    return {
        "id": f"msg_ollama_{data.get('created_at', 'unknown')}",
        "type": "message",
        "role": "assistant",
        "content": [{"type": "text", "text": assistant_text}],
        "model": model,
        "stop_reason": "end_turn",
        "stop_sequence": None,
        "usage": {
            "input_tokens": data.get("prompt_eval_count", 0),
            "output_tokens": data.get("eval_count", 0),
        },
    }
```

**Example Response:**
```python
{
    "id": "msg_ollama_2026-04-02T12:00:00.000Z",
    "type": "message",
    "role": "assistant",
    "content": [{"type": "text", "text": "Hello! How can I help you?"}],
    "model": "llama3.3:70b",
    "stop_reason": "end_turn",
    "usage": {
        "input_tokens": 25,
        "output_tokens": 12,
    }
}
```

### Streaming Chat

```python
async def ollama_chat_stream(
    model: str,
    messages: list[dict],
    system: str | None = None,
    max_tokens: int = 4096,
    temperature: float = 1.0,
) -> AsyncIterator[str]:
    """
    Complete a chat conversation with streaming.
    
    Yields:
        str: Anthropic-format SSE events
    
    Events:
        - message_start
        - content_block_start
        - content_block_delta (multiple)
        - content_block_stop
        - message_delta
        - message_stop
    """
    import json
    
    model = normalize_ollama_model(model)
    ollama_messages = anthropic_to_ollama_messages(messages)
    
    if system:
        ollama_messages.insert(0, {"role": "system", "content": system})
    
    payload = {
        "model": model,
        "messages": ollama_messages,
        "stream": True,
        "options": {
            "num_predict": max_tokens,
            "temperature": temperature,
        },
    }
    
    # Emit message_start
    yield "event: message_start\n"
    yield f'data: {json.dumps({
        "type": "message_start",
        "message": {
            "id": "msg_ollama_stream",
            "type": "message",
            "role": "assistant",
            "content": [],
            "model": model,
            "stop_reason": None,
            "usage": {"input_tokens": 0, "output_tokens": 0}
        }
    })}\n\n'
    
    # Emit content_block_start
    yield "event: content_block_start\n"
    yield f'data: {json.dumps({
        "type": "content_block_start",
        "index": 0,
        "content_block": {"type": "text", "text": ""}
    })}\n\n'
    
    # Stream response
    async with httpx.AsyncClient(timeout=120.0) as client:
        async with client.stream(
            "POST",
            f"{OLLAMA_BASE_URL}/api/chat",
            json=payload
        ) as resp:
            resp.raise_for_status()
            
            async for line in resp.aiter_lines():
                if not line:
                    continue
                
                try:
                    chunk = json.loads(line)
                    delta_text = chunk.get("message", {}).get("content", "")
                    
                    if delta_text:
                        # Emit content_block_delta
                        yield "event: content_block_delta\n"
                        yield f'data: {json.dumps({
                            "type": "content_block_delta",
                            "index": 0,
                            "delta": {
                                "type": "text_delta",
                                "text": delta_text
                            }
                        })}\n\n'
                    
                    if chunk.get("done"):
                        # Emit content_block_stop
                        yield "event: content_block_stop\n"
                        yield f'data: {json.dumps({
                            "type": "content_block_stop",
                            "index": 0
                        })}\n\n'
                        
                        # Emit message_delta
                        yield "event: message_delta\n"
                        yield f'data: {json.dumps({
                            "type": "message_delta",
                            "delta": {
                                "stop_reason": "end_turn",
                                "stop_sequence": None
                            },
                            "usage": {
                                "output_tokens": chunk.get("eval_count", 0)
                            }
                        })}\n\n'
                        
                        # Emit message_stop
                        yield "event: message_stop\n"
                        yield f'data: {json.dumps({
                            "type": "message_stop"
                        })}\n\n'
                        
                        break
                        
                except json.JSONDecodeError:
                    continue
```

**Example Stream Output:**
```
event: message_start
data: {"type":"message_start","message":{...}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{...}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"!"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" How"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" can"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" I"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" help"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"?"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{...},"usage":{"output_tokens":8}}

event: message_stop
data: {"type":"message_stop"}
```

---

## Usage Examples

### Basic Usage

```python
from ollama_provider import ollama_chat

# Simple chat
response = await ollama_chat(
    model="llama3.3:70b",
    messages=[{"role": "user", "content": "Hello!"}]
)
print(response["content"][0]["text"])
```

### Streaming Usage

```python
from ollama_provider import ollama_chat_stream

# Streaming chat
async for event in ollama_chat_stream(
    model="llama3.3:70b",
    messages=[{"role": "user", "content": "Tell me a story"}]
):
    print(event, end="")
```

### With System Prompt

```python
response = await ollama_chat(
    model="codellama:34b",
    messages=[{"role": "user", "content": "def hello():"}],
    system="You are a coding assistant. Only respond with code."
)
```

### Model Selection

```python
# List available models
models = await list_ollama_models()
print(f"Available: {models}")

# Check if specific model is available
if "llama3.3:70b" in models:
    response = await ollama_chat(model="llama3.3:70b", messages=...)
elif "llama3.1:8b" in models:
    response = await ollama_chat(model="llama3.1:8b", messages=...)
else:
    print("No suitable model found")
```

---

## Ollama Setup

### Installation

```bash
# macOS
curl -fsSL https://ollama.com/install.sh | sh

# Linux (wsl)
curl -fsSL https://ollama.com/install.sh | sh

# Windows
# Download from https://ollama.com/download/windows

# Docker
docker run -d -v ollama:/root/.ollama -p 11434:11434 --name ollama ollama/ollama
```

### Pull Models

```bash
# General purpose
ollama pull llama3.3:70b     # Best quality
ollama pull llama3.1:8b      # Fast, good quality
ollama pull mistral:7b       # Lightweight

# Coding
ollama pull codellama:34b    # Code specialist
ollama pull qwen2.5-coder:14b  # Alternative

# Verify
ollama list
```

### Start Server

```bash
# Start Ollama server
ollama serve

# Check status
ollama ps

# Test endpoint
curl http://localhost:11434/api/tags
```

---

## Performance Tuning

### Ollama Configuration

```bash
# ~/.ollama/config.json
{
  "host": "127.0.0.1:11434",
  "origins": ["*"],
  "noprunotelemetry": true
}
```

### Model Load Options

```bash
# Keep model loaded (faster subsequent requests)
ollama run llama3.3:70b

# Set GPU layers (NVIDIA)
OLLAMA_NUM_GPU=35 ollama run llama3.3:70b

# Set context size
OLLAMA_NUM_CTX=8192 ollama run llama3.3:70b
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `OLLAMA_HOST` | Bind address | `127.0.0.1:11434` |
| `OLLAMA_ORIGINS` | CORS origins | `*` |
| `OLLAMA_MODELS` | Model storage path | `~/.ollama/models` |
| `OLLAMA_NUM_GPU` | GPU layers to offload | Auto |
| `OLLAMA_NUM_CTX` | Context window size | Model default |

---

## Troubleshooting

### Connection Refused

**Problem:** `ConnectionRefusedError: All connection attempts failed`

**Solution:**
```bash
# Check if Ollama is running
ollama ps

# Start if needed
ollama serve

# Check port
netstat -an | grep 11434
```

### Model Not Found

**Problem:** `model 'llama3.3:70b' not found`

**Solution:**
```bash
# Pull the model
ollama pull llama3.3:70b

# List available
ollama list
```

### Slow Responses

**Problem:** Responses are very slow (>10 tokens/sec)

**Solution:**
```bash
# Check GPU utilization
ollama ps

# If running on CPU, try smaller model
ollama pull llama3.1:8b

# Or increase GPU offload
OLLAMA_NUM_GPU=35 ollama serve
```

### Out of Memory

**Problem:** `CUDA out of memory`

**Solution:**
```bash
# Use smaller model
ollama pull llama3.1:8b

# Or reduce context
OLLAMA_NUM_CTX=4096 ollama run llama3.3:70b

# Or reduce GPU layers
OLLAMA_NUM_GPU=20 ollama run llama3.3:70b
```

---

## Integration with OpenClaude

### Environment Setup

```bash
# Enable OpenAI provider
export CLAUDE_CODE_USE_OPENAI=1

# Point to Ollama
export OPENAI_BASE_URL=http://localhost:11434/v1

# Set model
export OPENAI_MODEL=llama3.3:70b

# No API key needed for local
unset OPENAI_API_KEY
```

### Profile Configuration

```json
{
  "profile": "ollama",
  "env": {
    "OPENAI_BASE_URL": "http://localhost:11434/v1",
    "OPENAI_MODEL": "llama3.3:70b"
  }
}
```

### Launch Command

```bash
bun run dev:ollama
```

---

## References

- [ollama_provider.py](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/openclaude/ollama_provider.py) — Source code
- [smart-router-deep-dive.md](./smart-router-deep-dive.md) — Smart router integration
- [Ollama Documentation](https://ollama.com/docs) — Official docs
