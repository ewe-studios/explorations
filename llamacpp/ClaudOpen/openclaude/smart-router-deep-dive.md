# Smart Router Deep Dive

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/openclaude/smart_router.py`

---

## Overview

The Smart Router is an intelligent multi-provider routing system that automatically selects the optimal LLM provider for each request based on latency, cost, and health metrics.

### Key Features

- **Automatic Provider Selection**: Chooses the best provider for each request
- **Real-time Health Monitoring**: Continuously tracks provider availability
- **Latency Learning**: Adapts based on actual request performance
- **Cost Optimization**: Can prioritize cheaper providers when appropriate
- **Automatic Failover**: Gracefully handles provider failures

---

## Architecture

### Component Diagram

```mermaid
graph TB
    Request[Incoming Request] --> Router[SmartRouter]
    Router --> Select[select_provider()]
    
    Select --> Providers[Provider Pool]
    Providers --> P1[OpenAI Provider]
    Providers --> P2[Gemini Provider]
    Providers --> P3[Ollama Provider]
    
    P1 --> Score1[Calculate Score]
    P2 --> Score2[Calculate Score]
    P3 --> Score3[Calculate Score]
    
    Score1 --> Compare[Compare Scores]
    Score2 --> Compare
    Score3 --> Compare
    
    Compare --> Winner[Select Winner]
    Winner --> Route[Route Request]
    Route --> Record[record_result()]
    
    Record --> Update[Update Metrics]
    Update --> Providers
```

### Data Flow

```
1. Request arrives with messages
2. Router checks provider health status
3. For each healthy provider:
   a. Calculate score based on strategy
   b. Apply error penalties
4. Select provider with lowest score
5. Route request to selected provider
6. Record result (success/failure, duration)
7. Update provider metrics
```

---

## Provider Model

### Provider Class Definition

```python
@dataclass
class Provider:
    """Represents an LLM provider with health and performance metrics."""
    
    name: str                        # e.g., "openai", "gemini", "ollama"
    ping_url: str                    # URL for health checks
    api_key_env: str                 # Environment variable for API key
    cost_per_1k_tokens: float        # Cost in USD per 1k tokens
    big_model: str                   # Model for complex requests
    small_model: str                 # Model for simple requests
    
    # Dynamic metrics (updated at runtime)
    latency_ms: float = 9999.0       # Last measured latency
    healthy: bool = True             # Current health status
    request_count: int = 0           # Total requests routed
    error_count: int = 0             # Total errors
    avg_latency_ms: float = 9999.0   # Rolling average latency
    
    @property
    def api_key(self) -> Optional[str]:
        """Get API key from environment."""
        return os.getenv(self.api_key_env)
    
    @property
    def is_configured(self) -> bool:
        """Check if provider has required configuration."""
        if self.name == "ollama":
            return True  # Ollama doesn't need an API key
        return bool(self.api_key)
    
    @property
    def error_rate(self) -> float:
        """Calculate current error rate."""
        if self.request_count == 0:
            return 0.0
        return self.error_count / self.request_count
```

### Default Provider Catalogue

```python
def build_default_providers() -> list[Provider]:
    """Build the default list of providers from environment."""
    
    big = os.getenv("BIG_MODEL", "gpt-4.1")
    small = os.getenv("SMALL_MODEL", "gpt-4.1-mini")
    ollama_url = os.getenv("OLLAMA_BASE_URL", "http://localhost:11434")
    
    return [
        Provider(
            name="openai",
            ping_url="https://api.openai.com/v1/models",
            api_key_env="OPENAI_API_KEY",
            cost_per_1k_tokens=0.002,
            big_model=big if "gpt" in big else "gpt-4.1",
            small_model=small if "gpt" in small else "gpt-4.1-mini",
        ),
        Provider(
            name="gemini",
            ping_url="https://generativelanguage.googleapis.com/v1/models",
            api_key_env="GEMINI_API_KEY",
            cost_per_1k_tokens=0.0005,
            big_model=big if "gemini" in big else "gemini-2.5-pro",
            small_model=small if "gemini" in small else "gemini-2.0-flash",
        ),
        Provider(
            name="ollama",
            ping_url=f"{ollama_url}/api/tags",
            api_key_env="",
            cost_per_1k_tokens=0.0,  # Free - local
            big_model=big if "gemini" not in big and "gpt" not in big else "llama3:8b",
            small_model=small if "gemini" not in small and "gpt" not in small else "llama3:8b",
        ),
    ]
```

---

## Scoring Algorithm

### Score Calculation

The scoring algorithm is the heart of the smart router. Lower scores are better.

```python
def score(self, strategy: str = "balanced") -> float:
    """
    Calculate provider score. Lower = better.
    
    Args:
        strategy: 'latency' | 'cost' | 'balanced'
    
    Returns:
        float: Score (inf if unhealthy or unconfigured)
    """
    if not self.healthy or not self.is_configured:
        return float("inf")
    
    # Normalize latency to seconds (typical range: 0.1 - 2.0)
    latency_score = self.avg_latency_ms / 1000.0
    
    # Normalize cost to similar scale (typical range: 0.0 - 0.5)
    cost_score = self.cost_per_1k_tokens * 100
    
    # Heavy penalty for errors (typical range: 0.0 - 500+)
    error_penalty = self.error_rate * 500
    
    if strategy == "latency":
        # Prioritize speed above all
        return latency_score + error_penalty
    
    elif strategy == "cost":
        # Prioritize cost above all
        return cost_score + error_penalty
    
    else:  # balanced
        # Equal weighting of latency and cost
        return (latency_score * 0.5) + (cost_score * 0.5) + error_penalty
```

### Strategy Comparison

| Strategy | Formula | Best For |
|----------|---------|----------|
| `latency` | `latency + error_penalty` | Real-time applications |
| `cost` | `cost + error_penalty` | Budget-conscious workloads |
| `balanced` | `(latency * 0.5) + (cost * 0.5) + error_penalty` | General purpose |

### Example Scores

```
Provider: OpenAI
  avg_latency_ms: 250
  cost_per_1k: 0.002
  error_rate: 0.01
  
  latency_score = 250 / 1000 = 0.25
  cost_score = 0.002 * 100 = 0.2
  error_penalty = 0.01 * 500 = 5.0
  
  balanced_score = (0.25 * 0.5) + (0.2 * 0.5) + 5.0 = 5.225

Provider: Ollama (local)
  avg_latency_ms: 500
  cost_per_1k: 0.0
  error_rate: 0.0
  
  latency_score = 500 / 1000 = 0.5
  cost_score = 0.0 * 100 = 0.0
  error_penalty = 0.0 * 500 = 0.0
  
  balanced_score = (0.5 * 0.5) + (0.0 * 0.5) + 0.0 = 0.25

Result: Ollama wins (0.25 < 5.225) due to zero cost and no errors
```

---

## Health Checking

### Provider Ping

```python
async def _ping_provider(self, provider: Provider) -> None:
    """
    Measure latency to a provider's health endpoint.
    Updates provider.healthy and provider.latency_ms.
    """
    if not provider.is_configured:
        provider.healthy = False
        logger.debug(f"SmartRouter: {provider.name} skipped — no API key")
        return
    
    headers = {}
    if provider.api_key:
        headers["Authorization"] = f"Bearer {provider.api_key}"
    
    start = time.monotonic()
    try:
        async with httpx.AsyncClient(timeout=5.0) as client:
            resp = await client.get(provider.ping_url, headers=headers)
            elapsed_ms = (time.monotonic() - start) * 1000
            
            # 200 = OK, 400/401/403 = reachable (just auth issue)
            if resp.status_code in (200, 400, 401, 403):
                provider.healthy = True
                provider.latency_ms = elapsed_ms
                provider.avg_latency_ms = elapsed_ms
                logger.info(
                    f"SmartRouter: {provider.name} OK "
                    f"({elapsed_ms:.0f}ms, status={resp.status_code})"
                )
            else:
                provider.healthy = False
                logger.warning(
                    f"SmartRouter: {provider.name} unhealthy "
                    f"(status={resp.status_code})"
                )
    except Exception as e:
        provider.healthy = False
        logger.warning(f"SmartRouter: {provider.name} unreachable — {e}")
```

### Initialization Flow

```python
async def initialize(self) -> None:
    """
    Ping all providers and build initial latency scores.
    Should be called once at application startup.
    """
    logger.info("SmartRouter: benchmarking providers...")
    
    # Ping all providers concurrently
    await asyncio.gather(
        *[self._ping_provider(p) for p in self.providers],
        return_exceptions=True,
    )
    
    # Log available providers
    available = [p for p in self.providers if p.healthy and p.is_configured]
    logger.info(
        f"SmartRouter ready. Available providers: "
        f"{[p.name for p in available]}"
    )
    
    if not available:
        logger.warning(
            "SmartRouter: no providers available! "
            "Check your API keys in .env"
        )
    
    self._initialized = True
```

---

## Request Routing

### Main Routing Method

```python
async def route(
    self,
    messages: list[dict],
    claude_model: str = "claude-sonnet",
    attempt: int = 0,
    exclude_providers: Optional[list[str]] = None,
) -> dict:
    """
    Route a request to the best provider.
    
    Args:
        messages: Conversation messages
        claude_model: Requested Claude model tier
        attempt: Retry attempt number
        exclude_providers: Providers to skip
    
    Returns:
        dict with provider, model, api_key, provider_object
    
    Raises:
        RuntimeError: If no providers available
    """
    if not self._initialized:
        await self.initialize()
    
    exclude = set(exclude_providers or [])
    large = self.is_large_request(messages)
    
    # Filter available providers
    available = [
        p for p in self.providers
        if p.healthy and p.is_configured and p.name not in exclude
    ]
    
    if not available:
        raise RuntimeError(
            "SmartRouter: no providers available. "
            "Check your API keys and provider health."
        )
    
    # Select best by score
    provider = min(available, key=lambda p: p.score(self.strategy))
    model = self.get_model_for_provider(provider, claude_model)
    
    logger.debug(
        f"SmartRouter: routing to {provider.name}/{model} "
        f"(strategy={self.strategy}, large={large}, attempt={attempt})"
    )
    
    return {
        "provider": provider.name,
        "model": model,
        "api_key": provider.api_key or "none",
        "provider_object": provider,
    }
```

### Model Mapping

```python
def get_model_for_provider(
    self, provider: Provider, claude_model: str
) -> str:
    """
    Map a Claude model tier to the provider's actual model.
    
    Args:
        provider: Selected provider
        claude_model: Claude model string (e.g., "claude-sonnet")
    
    Returns:
        Provider-specific model name
    """
    # Detect if this is a "large" request (opus/sonnet tier)
    is_large = any(
        keyword in claude_model.lower()
        for keyword in ["opus", "sonnet", "large", "big"]
    )
    
    # Return appropriate model for the tier
    return provider.big_model if is_large else provider.small_model
```

### Request Size Detection

```python
def is_large_request(self, messages: list[dict]) -> bool:
    """
    Estimate if request is "large" based on message content length.
    Large requests use big_model, small requests use small_model.
    
    Args:
        messages: Conversation messages
    
    Returns:
        bool: True if request appears to be large/complex
    """
    total_chars = sum(
        len(str(m.get("content", ""))) for m in messages
    )
    return total_chars > 2000  # >2000 chars = treat as large
```

---

## Learning from Results

### Result Recording

```python
async def record_result(
    self,
    provider_name: str,
    success: bool,
    duration_ms: float,
) -> None:
    """
    Record the outcome of a request.
    Updates provider latency and error metrics.
    
    Args:
        provider_name: Name of provider used
        success: Whether request succeeded
        duration_ms: Request duration in milliseconds
    """
    provider = next(
        (p for p in self.providers if p.name == provider_name), None
    )
    if not provider:
        return
    
    provider.request_count += 1
    
    if success:
        # Update rolling average latency (exponential moving average)
        self._update_latency(provider, duration_ms)
    else:
        provider.error_count += 1
        
        # Mark unhealthy after 3+ failures with >70% error rate
        recent_errors = provider.error_count
        recent_total = provider.request_count
        
        if recent_total >= 3 and (recent_errors / recent_total) > 0.7:
            logger.warning(
                f"SmartRouter: {provider.name} error rate high "
                f"({provider.error_rate:.0%}), marking unhealthy"
            )
            provider.healthy = False
            
            # Schedule re-check after 60 seconds
            asyncio.create_task(self._recheck_provider(provider, delay=60))
```

### Latency Update (Exponential Moving Average)

```python
def _update_latency(self, provider: Provider, duration_ms: float) -> None:
    """
    Update provider's rolling average latency.
    Uses exponential moving average for smooth adaptation.
    
    Args:
        provider: Provider to update
        duration_ms: Observed latency
    """
    alpha = 0.3  # Weight for new observation (0.0-1.0)
    
    # EMA formula: new = alpha * current + (1 - alpha) * previous
    provider.avg_latency_ms = (
        alpha * duration_ms + (1 - alpha) * provider.avg_latency_ms
    )
```

### Provider Recovery

```python
async def _recheck_provider(
    self, provider: Provider, delay: float = 60
) -> None:
    """
    Re-ping a provider after a delay and restore if healthy.
    
    Args:
        provider: Provider to recheck
        delay: Seconds to wait before checking
    """
    await asyncio.sleep(delay)
    await self._ping_provider(provider)
    
    if provider.healthy:
        logger.info(
            f"SmartRouter: {provider.name} recovered, "
            f"re-adding to pool"
        )
```

---

## Status and Monitoring

### Status Report

```python
def status(self) -> list[dict]:
    """
    Return current provider status for monitoring.
    
    Returns:
        List of provider status dictionaries
    """
    return [
        {
            "provider": p.name,
            "healthy": p.healthy,
            "configured": p.is_configured,
            "latency_ms": round(p.avg_latency_ms, 1),
            "cost_per_1k": p.cost_per_1k_tokens,
            "requests": p.request_count,
            "errors": p.error_count,
            "error_rate": f"{p.error_rate:.1%}",
            "score": round(p.score(self.strategy), 3)
            if p.healthy and p.is_configured
            else "N/A",
        }
        for p in self.providers
    ]
```

### Example Status Output

```json
[
  {
    "provider": "openai",
    "healthy": true,
    "configured": true,
    "latency_ms": 234.5,
    "cost_per_1k": 0.002,
    "requests": 156,
    "errors": 2,
    "error_rate": "1.3%",
    "score": 5.223
  },
  {
    "provider": "gemini",
    "healthy": true,
    "configured": true,
    "latency_ms": 189.2,
    "cost_per_1k": 0.0005,
    "requests": 89,
    "errors": 0,
    "error_rate": "0.0%",
    "score": 0.145
  },
  {
    "provider": "ollama",
    "healthy": true,
    "configured": true,
    "latency_ms": 512.8,
    "cost_per_1k": 0.0,
    "requests": 342,
    "errors": 1,
    "error_rate": "0.3%",
    "score": 0.408
  }
]
```

---

## Usage Examples

### Basic Usage

```python
from smart_router import SmartRouter

# Create and initialize router
router = SmartRouter()
await router.initialize()

# Route a request
result = await router.route([
    {"role": "user", "content": "Hello, world!"}
])

print(f"Routed to: {result['provider']}/{result['model']}")

# Record the result
start = time.time()
# ... make API call ...
duration_ms = (time.time() - start) * 1000
await router.record_result(result['provider'], success=True, duration_ms=duration_ms)
```

### Custom Provider Configuration

```python
from smart_router import SmartRouter, Provider

# Define custom providers
providers = [
    Provider(
        name="together",
        ping_url="https://api.together.xyz/v1/models",
        api_key_env="TOGETHER_API_KEY",
        cost_per_1k_tokens=0.0009,
        big_model="meta-llama/Llama-3.3-70B-Instruct-Turbo",
        small_model="meta-llama/Llama-3.1-8B-Instruct-Turbo",
    ),
    Provider(
        name="groq",
        ping_url="https://api.groq.com/openai/v1/models",
        api_key_env="GROQ_API_KEY",
        cost_per_1k_tokens=0.00089,
        big_model="llama-3.3-70b-versatile",
        small_model="llama-3.1-8b-instant",
    ),
]

# Create router with custom providers
router = SmartRouter(
    providers=providers,
    strategy="latency",  # Prioritize speed
    fallback_enabled=True,
)
```

### Strategy Switching

```python
# Cost-sensitive batch processing
router = SmartRouter(strategy="cost")
result = await router.route(batch_messages)

# Latency-sensitive real-time requests
router = SmartRouter(strategy="latency")
result = await router.route(realtime_messages)

# Balanced for general use
router = SmartRouter(strategy="balanced")
result = await router.route(general_messages)
```

---

## Configuration Reference

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `ROUTER_MODE` | `fixed` | `smart` or `fixed` |
| `ROUTER_STRATEGY` | `balanced` | `latency`, `cost`, or `balanced` |
| `ROUTER_FALLBACK` | `true` | Enable automatic failover |
| `BIG_MODEL` | `gpt-4.1` | Model for large requests |
| `SMALL_MODEL` | `gpt-4.1-mini` | Model for small requests |
| `OPENAI_API_KEY` | - | OpenAI API key |
| `GEMINI_API_KEY` | - | Google Gemini API key |
| `OLLAMA_BASE_URL` | `http://localhost:11434` | Ollama endpoint |

### Cost Reference

| Provider | Input Cost/1K | Output Cost/1K |
|----------|--------------|----------------|
| OpenAI GPT-4o | $0.005 | $0.015 |
| OpenAI GPT-4o-mini | $0.00015 | $0.0006 |
| Gemini 2.0 Flash | $0.000075 | $0.0003 |
| DeepSeek V3 | $0.00027 | $0.0011 |
| Ollama (local) | $0.00 | $0.00 |

---

## Troubleshooting

### No Providers Available

**Problem:** Router reports "no providers available"

**Solution:**
```bash
# Check API keys
echo $OPENAI_API_KEY
echo $GEMINI_API_KEY

# Test connectivity
curl https://api.openai.com/v1/models -H "Authorization: Bearer $OPENAI_API_KEY"

# Check router status
python -c "from smart_router import SmartRouter; r = SmartRouter(); print(r.status())"
```

### High Error Rate

**Problem:** Provider marked unhealthy due to errors

**Solution:**
1. Check provider status page
2. Verify API key hasn't expired
3. Check rate limits
4. Wait for automatic recovery (60s) or manually re-ping

### Unexpected Provider Selection

**Problem:** Router selects unexpected provider

**Solution:**
```python
# Check scoring
router = SmartRouter()
for p in router.providers:
    print(f"{p.name}: score={p.score(router.strategy)}")

# Try different strategy
router = SmartRouter(strategy="latency")  # or "cost"
```

---

## Performance Tuning

### Adjusting EMA Alpha

```python
# More responsive to changes (less smoothing)
alpha = 0.5

# More stable (more smoothing)
alpha = 0.1

# Default (balanced)
alpha = 0.3
```

### Error Threshold Tuning

```python
# Mark unhealthy after 50% errors (more tolerant)
if (recent_errors / recent_total) > 0.5:
    provider.healthy = False

# Mark unhealthy after 90% errors (less tolerant)
if (recent_errors / recent_total) > 0.9:
    provider.healthy = False
```

### Recheck Delay

```python
# Faster recovery check (30 seconds)
asyncio.create_task(self._recheck_provider(provider, delay=30))

# Slower recovery check (120 seconds)
asyncio.create_task(self._recheck_provider(provider, delay=120))
```

---

## References

- [smart_router.py](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/openclaude/smart_router.py) — Source code
- [ollama_provider.py](./ollama-provider-deep-dive.md) — Ollama provider implementation
- [01-openclaude-exploration.md](./01-openclaude-exploration.md) — Architecture overview
