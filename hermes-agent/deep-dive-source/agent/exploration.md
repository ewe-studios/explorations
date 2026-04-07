# agent/ Deep Dive Exploration

**Source Directory:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/hermes-agent/agent/`

**Status:** complete

---

## Module Overview

The `agent/` module contains the core internals of the Hermes Agent system - 23 Python files (~14K lines total) that handle model communication, credential management, context handling, memory systems, and utility functions. These modules were extracted from the monolithic `run_agent.py` to provide separation of concerns and maintainability.

The module follows a clear architectural pattern:
- **Adapters** translate between internal formats and provider APIs
- **Clients** route requests to appropriate model providers
- **Memory** providers implement pluggable long-term memory systems
- **Context** systems handle references, compression, and building
- **Utilities** handle display, pricing, redaction, and other cross-cutting concerns

---

## Directory Structure

| File | Lines | Purpose |
|------|-------|---------|
| `__init__.py` | 6 | Package docstring |
| `anthropic_adapter.py` | 1,389 | Anthropic Messages API adapter |
| `auxiliary_client.py` | 2,127 | Auxiliary client router for side tasks |
| `builtin_memory_provider.py` | 113 | Built-in MEMORY.md/USER.md provider |
| `context_compressor.py` | 696 | Automatic context window compression |
| `context_references.py` | 492 | @-reference parsing and resolution |
| `copilot_acp_client.py` | 570 | GitHub Copilot ACP client |
| `credential_pool.py` | 1,157 | Multi-credential pool for failover |
| `display.py` | 1,084 | CLI spinner and tool preview formatting |
| `insights.py` | 799 | Turn-based insights extraction |
| `memory_manager.py` | 366 | Multi-provider memory orchestration |
| `memory_provider.py` | 231 | Abstract base class for memory providers |
| `model_metadata.py` | 941 | Model context length and pricing data |
| `models_dev.py` | 781 | Model routing configuration |
| `prompt_builder.py` | 960 | System prompt assembly |
| `prompt_caching.py` | 72 | Anthropic prompt caching helpers |
| `redact.py` | 181 | Sensitive data redaction |
| `skill_commands.py` | 368 | Skill-related command helpers |
| `skill_utils.py` | 442 | Skill utility functions |
| `smart_model_routing.py` | 194 | Intelligent model selection |
| `subdirectory_hints.py` | 219 | Working directory hints |
| `title_generator.py` | 125 | Session title generation |
| `trajectory.py` | 56 | Trajectory format conversion |
| `usage_pricing.py` | 656 | Token usage and cost calculation |

**Total:** ~14,025 lines across 24 files

---

## Key Components

### 1. Anthropic Adapter (`anthropic_adapter.py`)

Translates OpenAI-style messages to Anthropic Messages API format with support for multiple auth types.

**Key Exports:**
```python
def translate_to_anthropic(messages, model, **kwargs) -> dict
def _get_anthropic_max_output(model: str) -> int
def _supports_adaptive_thinking(model: str) -> bool
```

**Auth Types Supported:**
| Type | Token Pattern | Headers |
|------|--------------|---------|
| API Key | `sk-ant-api*` | `x-api-key` |
| OAuth | `sk-ant-oat*` | `Authorization: Bearer` + beta headers |
| Claude Code | `~/.claude.json` | Bearer auth |

**Beta Headers (OAuth):**
- `claude-code-20250219`
- `oauth-2025-04-20`
- `interleaved-thinking-2025-05-14`
- `fine-grained-tool-streaming-2025-05-14`

### 2. Auxiliary Client (`auxiliary_client.py`)

Routes side tasks (context compression, vision, web extraction) to appropriate model providers.

**Resolution Order (Text - Auto Mode):**
1. OpenRouter (`OPENROUTER_API_KEY`)
2. Nous Portal (`~/.hermes/auth.json`)
3. Custom endpoint (config.yaml `model.base_url`)
4. Codex OAuth (Responses API)
5. Native Anthropic
6. Direct API-key providers (z.ai, Kimi, MiniMax)

**Key Classes:**
```python
class AuxiliaryClientRouter:
    def get_client(self, task_type: str, auto: bool = True) -> Any
    def get_vision_client(self, main_provider_config: dict) -> Any
```

### 3. Credential Pool (`credential_pool.py`)

Manages multi-credential failover for high-volume operations.

**Key Features:**
- **Strategies:** `fill_first`, `round_robin`, `random`, `least_used`
- **Exhaustion handling:** Cooldown periods (1hr for 429, 24hr for 402)
- **OAuth support:** sk-ant-oat* tokens with Bearer auth
- **Claude Code spoofing:** Version detection for OAuth routing

**Key Classes:**
```python
class CredentialPool:
    def __init__(self, provider: str, credentials: List[str], strategy: str = "round_robin")
    def get_next_credential(self) -> str
    def mark_exhausted(self, credential: str, status_code: int)
    def is_available(self) -> bool
```

### 4. Memory Provider System

**Abstract Base (`memory_provider.py`):**
```python
class MemoryProvider(ABC):
    @property
    @abstractmethod
    def name(self) -> str: ...
    
    @abstractmethod
    def is_available(self) -> bool: ...
    
    @abstractmethod
    def initialize(self, session_id: str, **kwargs) -> None: ...
    
    def system_prompt_block(self) -> str: ...
    def prefetch(self, query: str, *, session_id: str = "") -> str: ...
    def sync_turn(self, user_content: str, assistant_content: str) -> None: ...
    
    @abstractmethod
    def get_tool_schemas(self) -> List[Dict[str, Any]]: ...
```

**Built-in Provider (`builtin_memory_provider.py`):**
- Wraps `MEMORY.md` / `USER.md` files
- Always registered as first provider (cannot be disabled)
- Delegates to `MemoryStore` from `tools/memory_tool.py`

**Memory Manager (`memory_manager.py`):**
- Orchestrates multiple providers simultaneously
- Handles turn synchronization across providers

### 5. Context Compression (`context_compressor.py`)

Automatic context window compression using structured summarization.

**Algorithm:**
1. **Tool output pruning** - Pre-pass without LLM call
2. **Protect head messages** - System prompt + first exchange
3. **Protect tail messages** - Most recent ~20K tokens
4. **Summarize middle turns** - Structured LLM prompt
5. **Iterative updates** - Preserves info across compactions

**Token Budgets:**
```python
_MIN_SUMMARY_TOKENS = 2000
_SUMMARY_RATIO = 0.20  # 20% of compressed content
_SUMMARY_TOKENS_CEILING = 12_000
```

**Summary Template:**
```
[CONTEXT COMPACTION] Earlier turns were compacted...

Goal: [What was the original objective]
Progress: [What has been completed]
Decisions: [Key decisions made]
Files: [Files created/modified]
Next Steps: [What remains]
```

### 6. Context References (`context_references.py`)

Parses and resolves @-references in user messages.

**Reference Types:**
| Syntax | Kind | Target |
|--------|------|--------|
| `@diff` | diff | Git diff |
| `@staged` | staged | Staged changes |
| `@file:path/to/file.txt` | file | File content |
| `@file:path:10-20` | file | File lines 10-20 |
| `@folder:src/` | folder | Directory contents |
| `@git:HEAD~3` | git | Git history |
| `@url:https://...` | url | Web content |

**Security Features:**
- Path traversal protection (sandboxed to `allowed_root`)
- Sensitive file/directory filtering (`.ssh`, `.aws`, `.gnupg`, etc.)
- Token budget enforcement

### 7. Display Utilities (`display.py`)

CLI spinner, kawaii faces, and tool preview formatting.

**Key Functions:**
```python
def print_tool_preview(tool_name: str, args: dict, skin_config: dict)
def get_spinner(thinking: bool = False) -> Iterator[str]
def format_unified_diff(diff_text: str) -> str
def get_kawaii_face(emotion: str = "happy") -> str
```

**Tool Preview Arguments:**
```python
PRIMARY_ARGS = {
    "terminal": "command",
    "web_search": "query",
    "read_file": "path",
    "write_file": "path",
    "patch": "path",
    "browser_navigate": "url",
    "image_generate": "prompt",
}
```

### 8. Prompt Caching (`prompt_caching.py`)

Anthropic prompt caching with `system_and_3` strategy (~75% input token reduction).

**Cache Breakpoints:**
Uses 4 `cache_control` breakpoints (Anthropic max):
1. System prompt (stable across all turns)
2. Last non-system message
3. Second-to-last non-system message
4. Third-to-last non-system message

**Implementation:**
```python
def apply_anthropic_cache_control(messages, cache_ttl="5m", native_anthropic=False):
    messages = copy.deepcopy(api_messages)
    marker = {"type": "ephemeral"}
    if cache_ttl == "1h":
        marker["ttl"] = "1h"
    
    # Place cache markers on system + last 3 non-system messages
    if messages[0].get("role") == "system":
        _apply_cache_marker(messages[0], marker)
    
    non_sys = [i for i in range(len(messages)) if messages[i].get("role") != "system"]
    for idx in non_sys[-3:]:
        _apply_cache_marker(messages[idx], marker)
    
    return messages
```

### 9. Model Metadata (`model_metadata.py`)

Model context length, pricing, and capability data.

**Key Data Structures:**
```python
MODEL_INFO = {
    "claude-opus-4-6": {
        "context_length": 200000,
        "input_price": 15.0,  # $/1M tokens
        "output_price": 75.0,
        "supports_vision": True,
        "supports_thinking": True,
    },
    # ... 50+ models
}
```

### 10. Usage Pricing (`usage_pricing.py`)

Token usage tracking and cost calculation.

**Key Functions:**
```python
def calculate_cost(model: str, input_tokens: int, output_tokens: int, 
                   cached_tokens: int = 0) -> dict
def format_usd(cost: float) -> str
def get_session_cost(session_id: str) -> float
```

---

## Integration Points

### With `run_agent.py` (AIAgent)
- `anthropic_adapter.py` - Called for Anthropic model requests
- `auxiliary_client.py` - Called for side tasks (compression, vision)
- `prompt_builder.py` - Called to assemble system prompts
- `context_compressor.py` - Called when context exceeds threshold
- `memory_manager.py` - Called each turn for memory sync

### With `hermes_cli/`
- `skill_commands.py` - CLI skill command handlers
- `model_metadata.py` - Model listing and info commands
- `usage_pricing.py` - Cost reporting

### With `tools/`
- `builtin_memory_provider.py` - Delegates to `MemoryStore`
- `context_references.py` - File/folder resolution uses tool utilities
- `display.py` - Tool preview formatting

---

## Related Files

**Individual File Explorations:**
- [anthropic_adapter.md](./anthropic_adapter.md)
- [auxiliary_client.md](./auxiliary_client.md)
- [context_compressor.md](./context_compressor.md)
- [context_references.md](./context_references.md)
- [credential_pool.md](./credential_pool.md)
- [display.md](./display.md)
- [insights.md](./insights.md)
- [memory_manager.md](./memory_manager.md)
- [model_metadata.md](./model_metadata.md)
- [models_dev.md](./models_dev.md)
- [prompt_builder.md](./prompt_builder.md)
- [skill_commands.md](./skill_commands.md)
- [skill_utils.md](./skill_utils.md)
- [subdirectory_hints.md](./subdirectory_hints.md)
- [title_generator.md](./title_generator.md)
- [usage_pricing.md](./usage_pricing.md)

**Module Documentation:**
- [agent-internals.md](./agent-internals.md) - Comprehensive agent analysis
- [00-module-overview.md](./00-module-overview.md) - Module overview

---

*Deep dive created: 2026-04-07*
