# Nanobot Production-Grade Considerations

## Overview

This document analyzes nanobot's production-readiness, identifies gaps, and provides recommendations for enterprise deployment. While nanobot is designed as a lightweight personal assistant, many production patterns are already in place.

---

## Production Readiness Assessment

### Current Strengths

| Area | Status | Notes |
|------|--------|-------|
| Configuration Management | ✅ Good | Pydantic validation, env var support |
| Error Handling | ✅ Good | Try/catch with graceful degradation |
| Logging | ✅ Good | Loguru integration |
| Async Architecture | ✅ Good | Proper asyncio patterns |
| Security Basics | ✅ Good | Access control, command guards |
| Resource Limits | ✅ Good | Timeouts, iteration limits |
| Session Persistence | ✅ Good | File-based JSONL storage |

### Areas for Improvement

| Area | Priority | Effort |
|------|----------|--------|
| Observability/Telemetry | High | Medium |
| Rate Limiting | High | Low |
| Circuit Breakers | High | Medium |
| Horizontal Scaling | Medium | High |
| Database Backend | Medium | High |
| Secrets Management | High | Low |
| Health Checks | Medium | Low |
| Graceful Degradation | Medium | Medium |

---

## 1. Observability & Telemetry

### Current State

nanobot uses Loguru for logging but lacks:
- Structured logging (JSON format)
- Distributed tracing
- Metrics collection
- Request/response tracking

### Recommended Implementation

#### Structured Logging

```python
# nanobot/utils/logging.py
import json
from loguru import logger
import sys

def setup_production_logging():
    """Configure structured JSON logging for production."""

    class JsonFormatter:
        def __call__(self, record):
            log_entry = {
                "timestamp": record["time"].isoformat(),
                "level": record["level"].name,
                "message": record["message"],
                "module": record["module"],
                "function": record["function"],
                "line": record["line"],
            }

            # Add extra context
            if record["extra"]:
                log_entry["context"] = record["extra"]

            # Add exception info if present
            if record["exception"]:
                log_entry["exception"] = record["exception"]

            return json.dumps(log_entry) + "\n"

    logger.remove()  # Remove default handler
    logger.add(
        sys.stdout,
        format=JsonFormatter(),
        level="INFO",
    )
    logger.add(
        "/var/log/nanobot/app.log",
        format=JsonFormatter(),
        level="DEBUG",
        rotation="100 MB",
        retention="30 days",
    )
```

#### Request Tracking

```python
# nanobot/utils/tracing.py
import uuid
from contextvars import ContextVar

# Context-var for request ID (async-safe)
request_id_ctx: ContextVar[str] = ContextVar("request_id", default="")

def generate_request_id() -> str:
    return str(uuid.uuid4())[:8]

def get_request_id() -> str:
    return request_id_ctx.get()

class RequestContext:
    """Context manager for request tracking."""

    def __init__(self, channel: str, chat_id: str):
        self.request_id = generate_request_id()
        self.channel = channel
        self.chat_id = chat_id
        self._token = None

    def __enter__(self):
        self._token = request_id_ctx.set(self.request_id)
        logger.bind(
            request_id=self.request_id,
            channel=self.channel,
            chat_id=self.chat_id
        )
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        if self._token:
            request_id_ctx.reset(self._token)
```

#### Metrics Collection

```python
# nanobot/utils/metrics.py
from dataclasses import dataclass, field
from typing import dict, Optional
import time
from prometheus_client import Counter, Histogram, Gauge

# Metrics definitions
MESSAGES_RECEIVED = Counter(
    "nanobot_messages_received_total",
    "Total messages received",
    ["channel"]
)

MESSAGES_PROCESSED = Counter(
    "nanobot_messages_processed_total",
    "Total messages processed",
    ["channel", "status"]
)

TOOL_EXECUTIONS = Counter(
    "nanobot_tool_executions_total",
    "Total tool executions",
    ["tool_name", "status"]
)

LLM_LATENCY = Histogram(
    "nanobot_llm_latency_seconds",
    "LLM call latency",
    ["model"],
    buckets=[0.5, 1.0, 2.0, 5.0, 10.0, 30.0]
)

TOOL_LATENCY = Histogram(
    "nanobot_tool_latency_seconds",
    "Tool execution latency",
    ["tool_name"],
    buckets=[0.01, 0.05, 0.1, 0.5, 1.0, 5.0]
)

AGENT_ITERATIONS = Histogram(
    "nanobot_agent_iterations",
    "Agent loop iterations per message",
    buckets=[1, 2, 5, 10, 15, 20]
)

ACTIVE_SESSIONS = Gauge(
    "nanobot_active_sessions",
    "Number of active sessions"
)

@dataclass
class MetricsCollector:
    """Helper for recording metrics."""

    def record_message_received(self, channel: str):
        MESSAGES_RECEIVED.labels(channel=channel).inc()

    def record_message_processed(self, channel: str, status: str):
        MESSAGES_PROCESSED.labels(channel=channel, status=status).inc()

    def record_tool_execution(self, tool_name: str, status: str, duration: float):
        TOOL_EXECUTIONS.labels(tool_name=tool_name, status=status).inc()
        TOOL_LATENCY.labels(tool_name=tool_name).observe(duration)

    def record_llm_call(self, model: str, duration: float):
        LLM_LATENCY.labels(model=model).observe(duration)

    def record_agent_iterations(self, iterations: int):
        AGENT_ITERATIONS.observe(iterations)
```

#### Integration with Agent Loop

```python
# nanobot/agent/loop.py (modified)
from nanobot.utils.metrics import MetricsCollector
from nanobot.utils.tracing import RequestContext

class AgentLoop:
    def __init__(self, ...):
        self.metrics = MetricsCollector()
        ...

    async def _process_message(self, msg: InboundMessage) -> OutboundMessage:
        with RequestContext(msg.channel, msg.chat_id):
            start_time = time.time()
            iteration = 0

            try:
                self.metrics.record_message_received(msg.channel)

                while iteration < self.max_iterations:
                    iteration += 1

                    # LLM call with timing
                    llm_start = time.time()
                    response = await self.provider.chat(...)
                    llm_duration = time.time() - llm_start
                    self.metrics.record_llm_call(self.model, llm_duration)

                    # Tool execution
                    if response.has_tool_calls:
                        for tool_call in response.tool_calls:
                            tool_start = time.time()
                            result = await self.tools.execute(...)
                            tool_duration = time.time() - tool_start
                            self.metrics.record_tool_execution(
                                tool_call.name,
                                "success" if result and not result.startswith("Error") else "error",
                                tool_duration
                            )

                self.metrics.record_message_processed(msg.channel, "success")
                self.metrics.record_agent_iterations(iteration)

            except Exception as e:
                self.metrics.record_message_processed(msg.channel, "error")
                logger.error(f"Error processing message: {e}")
                raise

            finally:
                total_duration = time.time() - start_time
                logger.info(
                    f"Message processed in {total_duration:.2f}s",
                    extra={"duration_ms": total_duration * 1000}
                )
```

---

## 2. Rate Limiting

### Current State

No rate limiting exists. Production deployments need protection against:
- API rate limits (LLM providers)
- Channel rate limits (Telegram, Discord, etc.)
- Resource exhaustion

### Recommended Implementation

#### Token Bucket Rate Limiter

```python
# nanobot/utils/rate_limiter.py
import asyncio
import time
from typing import Optional

class TokenBucket:
    """Token bucket rate limiter."""

    def __init__(self, rate: float, capacity: int):
        """
        Args:
            rate: Tokens per second to add
            capacity: Maximum bucket capacity
        """
        self.rate = rate
        self.capacity = capacity
        self.tokens = capacity
        self.last_update = time.monotonic()
        self._lock = asyncio.Lock()

    async def acquire(self, tokens: int = 1, timeout: Optional[float] = None) -> bool:
        """
        Acquire tokens from the bucket.

        Args:
            tokens: Number of tokens to acquire
            timeout: Maximum time to wait (None = no wait)

        Returns:
            True if tokens acquired, False if timeout
        """
        start = time.monotonic()

        while True:
            async with self._lock:
                now = time.monotonic()
                elapsed = now - self.last_update
                self.tokens = min(self.capacity, self.tokens + elapsed * self.rate)
                self.last_update = now

                if self.tokens >= tokens:
                    self.tokens -= tokens
                    return True

            if timeout is not None:
                elapsed = time.monotonic() - start
                if elapsed >= timeout:
                    return False

            # Wait before retrying
            wait_time = (tokens - self.tokens) / self.rate
            await asyncio.sleep(min(wait_time, 0.1))

    @property
    def available(self) -> float:
        """Current available tokens."""
        now = time.monotonic()
        elapsed = now - self.last_update
        return min(self.capacity, self.tokens + elapsed * self.rate)


class RateLimitedProvider:
    """Wrap LLM provider with rate limiting."""

    def __init__(
        self,
        provider: LLMProvider,
        requests_per_minute: int = 60,
        tokens_per_minute: int = 100000,
    ):
        self.provider = provider
        self.request_limiter = TokenBucket(
            rate=requests_per_minute / 60,
            capacity=requests_per_minute
        )
        self.token_limiter = TokenBucket(
            rate=tokens_per_minute / 60,
            capacity=tokens_per_minute
        )

    async def chat(self, messages: list[dict], **kwargs) -> LLMResponse:
        # Acquire request token
        acquired = await self.request_limiter.acquire(
            tokens=1, timeout=30.0
        )
        if not acquired:
            raise RateLimitExceeded("Request rate limit exceeded")

        # Estimate token usage (rough estimate: 4 chars per token)
        estimated_tokens = sum(len(m.get("content", "")) for m in messages) // 4
        estimated_tokens += kwargs.get("max_tokens", 4096)

        # Acquire token budget
        acquired = await self.token_limiter.acquire(
            tokens=estimated_tokens, timeout=30.0
        )
        if not acquired:
            raise RateLimitExceeded("Token budget exceeded")

        return await self.provider.chat(messages, **kwargs)


class RateLimitExceeded(Exception):
    """Rate limit exceeded."""
    pass
```

#### Channel-Specific Rate Limits

```python
# nanobot/channels/rate_limits.py

# Telegram limits (bot API)
TELEGRAM_LIMITS = {
    "messages_per_second": 30,
    "global_messages_per_second": 100,
}

# Discord limits
DISCORD_LIMITS = {
    "messages_per_second": 10,
    "messages_per_channel_per_second": 5,
}

# WhatsApp limits (via bridge)
WHATSAPP_LIMITS = {
    "messages_per_second": 20,
}
```

---

## 3. Circuit Breaker Pattern

### Current State

No circuit breaker protection. If LLM provider or external services fail, nanobot continues attempting calls.

### Recommended Implementation

```python
# nanobot/utils/circuit_breaker.py
import asyncio
import time
from enum import Enum
from typing import Callable, Any, Optional

class CircuitState(Enum):
    CLOSED = "closed"      # Normal operation
    OPEN = "open"          # Failing, reject calls
    HALF_OPEN = "half_open" # Testing recovery

class CircuitBreaker:
    """Circuit breaker for external service calls."""

    def __init__(
        self,
        name: str,
        failure_threshold: int = 5,
        recovery_timeout: float = 60.0,
        half_open_max_calls: int = 3,
    ):
        self.name = name
        self.failure_threshold = failure_threshold
        self.recovery_timeout = recovery_timeout
        self.half_open_max_calls = half_open_max_calls

        self._state = CircuitState.CLOSED
        self._failure_count = 0
        self._success_count = 0
        self._last_failure_time: Optional[float] = None
        self._half_open_calls = 0
        self._lock = asyncio.Lock()

    @property
    def state(self) -> CircuitState:
        return self._state

    async def call(self, func: Callable, *args, **kwargs) -> Any:
        """Execute function through circuit breaker."""
        async with self._lock:
            if not self._should_allow_request():
                raise CircuitOpenError(f"Circuit {self.name} is open")

            self._half_open_calls += 1

        try:
            result = await func(*args, **kwargs)
            await self._on_success()
            return result
        except Exception as e:
            await self._on_failure()
            raise

    def _should_allow_request(self) -> bool:
        if self._state == CircuitState.CLOSED:
            return True

        if self._state == CircuitState.OPEN:
            if (time.monotonic() - self._last_failure_time) >= self.recovery_timeout:
                self._state = CircuitState.HALF_OPEN
                self._half_open_calls = 0
                return True
            return False

        # HALF_OPEN
        return self._half_open_calls < self.half_open_max_calls

    async def _on_success(self):
        if self._state == CircuitState.HALF_OPEN:
            self._success_count += 1
            if self._success_count >= self.half_open_max_calls:
                self._state = CircuitState.CLOSED
                self._failure_count = 0
                self._success_count = 0
        else:
            self._failure_count = 0

    async def _on_failure(self):
        self._failure_count += 1
        self._last_failure_time = time.monotonic()

        if self._state == CircuitState.HALF_OPEN:
            self._state = CircuitState.OPEN
        elif self._failure_count >= self.failure_threshold:
            self._state = CircuitState.OPEN
            logger.warning(
                f"Circuit {self.name} opened after {self._failure_count} failures"
            )


class CircuitOpenError(Exception):
    """Circuit breaker is open."""
    pass


# Usage in Agent Loop
class AgentLoop:
    def __init__(self, ...):
        self.llm_circuit = CircuitBreaker(
            name="llm_provider",
            failure_threshold=5,
            recovery_timeout=60.0,
        )
        self.web_circuit = CircuitBreaker(
            name="web_tools",
            failure_threshold=3,
            recovery_timeout=30.0,
        )

    async def _process_message(self, msg: InboundMessage) -> OutboundMessage:
        try:
            response = await self.llm_circuit.call(
                self.provider.chat,
                messages=messages,
                tools=self.tools.get_definitions(),
                model=self.model
            )
        except CircuitOpenError:
            return OutboundMessage(
                channel=msg.channel,
                chat_id=msg.chat_id,
                content="Service temporarily unavailable. Please try again later."
            )
```

---

## 4. Secrets Management

### Current State

API keys stored in plaintext JSON config file (`~/.nanobot/config.json`).

### Recommended Improvements

#### Environment Variable Injection

```python
# nanobot/config/secrets.py
import os
from pathlib import Path

def load_secret(secret_name: str) -> str | None:
    """Load secret from environment or file."""
    # Try environment variable first
    env_value = os.environ.get(f"NANOBOT_{secret_name.upper()}")
    if env_value:
        return env_value

    # Try secrets directory
    secrets_dir = Path.home() / ".nanobot" / "secrets"
    secret_file = secrets_dir / secret_name.lower()
    if secret_file.exists():
        return secret_file.read_text().strip()

    return None

# Usage in config
class ProviderConfig(BaseModel):
    @classmethod
    def get_api_key(cls, provider: str) -> str | None:
        return load_secret(f"{provider}_api_key")
```

#### Integration with Secret Managers

```python
# nanobot/config/aws_secrets.py
import boto3
from functools import lru_cache

class AWSSecretsManager:
    def __init__(self, region: str = "us-east-1"):
        self.client = boto3.client("secretsmanager", region_name=region)
        self._cache: dict[str, str] = {}

    @lru_cache(maxsize=100)
    def get_secret(self, secret_name: str) -> str:
        """Get secret from AWS Secrets Manager."""
        try:
            response = self.client.get_secret_value(SecretId=secret_name)
            return response["SecretString"]
        except Exception as e:
            logger.warning(f"Failed to get secret {secret_name}: {e}")
            return ""

# Usage
secrets = AWSSecretsManager()
api_key = secrets.get_secret("nanobot/openrouter/api_key")
```

---

## 5. Database Backend

### Current State

File-based storage for:
- Sessions: JSONL files
- Memory: Markdown files
- Cron jobs: JSON file

### Recommended: SQLite for Production

```python
# nanobot/storage/database.py
import aiosqlite
from pathlib import Path
from typing import Optional

class Database:
    """SQLite database for production storage."""

    def __init__(self, db_path: Path):
        self.db_path = db_path
        self._conn: Optional[aiosqlite.Connection] = None

    async def connect(self):
        """Initialize database connection."""
        self.db_path.parent.mkdir(parents=True, exist_ok=True)
        self._conn = await aiosqlite.connect(self.db_path)
        await self._init_tables()

    async def _init_tables(self):
        """Create tables if not exist."""
        await self._conn.executescript("""
            CREATE TABLE IF NOT EXISTS sessions (
                key TEXT PRIMARY KEY,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                metadata JSON
            );

            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_key TEXT,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                metadata JSON,
                FOREIGN KEY (session_key) REFERENCES sessions(key)
            );

            CREATE TABLE IF NOT EXISTS cron_jobs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                enabled BOOLEAN DEFAULT 1,
                schedule JSON NOT NULL,
                payload JSON NOT NULL,
                state JSON,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_messages_session
            ON messages(session_key);

            CREATE INDEX IF NOT EXISTS idx_cron_jobs_enabled
            ON cron_jobs(enabled, state);
        """)
        await self._conn.commit()

    async def close(self):
        """Close database connection."""
        if self._conn:
            await self._conn.close()

    # Session operations
    async def get_session(self, key: str) -> dict | None:
        async with self._conn.execute(
            "SELECT * FROM sessions WHERE key = ?", (key,)
        ) as cursor:
            row = await cursor.fetchone()
            return dict(row) if row else None

    async def save_session(self, key: str, metadata: dict):
        await self._conn.execute("""
            INSERT INTO sessions (key, metadata, updated_at)
            VALUES (?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(key) DO UPDATE SET
                metadata = excluded.metadata,
                updated_at = CURRENT_TIMESTAMP
        """, (key, json.dumps(metadata)))
        await self._conn.commit()

    async def get_messages(self, session_key: str, limit: int = 50):
        async with self._conn.execute("""
            SELECT role, content, timestamp, metadata
            FROM messages
            WHERE session_key = ?
            ORDER BY timestamp DESC
            LIMIT ?
        """, (session_key, limit)) as cursor:
            rows = await cursor.fetchall()
            return [dict(r) for r in reversed(rows)]

    async def add_message(self, session_key: str, role: str, content: str, metadata: dict):
        await self._conn.execute("""
            INSERT INTO messages (session_key, role, content, metadata)
            VALUES (?, ?, ?, ?)
        """, (session_key, role, content, json.dumps(metadata)))
        await self._conn.commit()
        # Update session timestamp
        await self.save_session(session_key, {})
```

---

## 6. Health Checks

### Current State

No health check endpoints.

### Recommended Implementation

```python
# nanobot/utils/health.py
from dataclasses import dataclass
from typing import Optional
import asyncio
import time

@dataclass
class HealthStatus:
    service: str
    healthy: bool
    message: str
    latency_ms: Optional[float] = None

class HealthChecker:
    """Service health checker."""

    def __init__(self, agent_loop: AgentLoop, db: Optional[Database] = None):
        self.agent_loop = agent_loop
        self.db = db
        self._last_check: dict[str, HealthStatus] = {}

    async def check_all(self) -> dict[str, HealthStatus]:
        """Run all health checks."""
        checks = {}

        # Agent loop check
        checks["agent"] = await self._check_agent()

        # Database check
        if self.db:
            checks["database"] = await self._check_database()

        # Provider check
        checks["provider"] = await self._check_provider()

        # Channel checks
        checks["channels"] = await self._check_channels()

        self._last_check = checks
        return checks

    async def _check_agent(self) -> HealthStatus:
        start = time.monotonic()
        try:
            if self.agent_loop._running:
                return HealthStatus(
                    service="agent",
                    healthy=True,
                    message="Agent loop running",
                    latency_ms=(time.monotonic() - start) * 1000
                )
            else:
                return HealthStatus(
                    service="agent",
                    healthy=False,
                    message="Agent loop not running"
                )
        except Exception as e:
            return HealthStatus(
                service="agent",
                healthy=False,
                message=str(e)
            )

    async def _check_database(self) -> HealthStatus:
        start = time.monotonic()
        try:
            await self.db._conn.execute("SELECT 1")
            return HealthStatus(
                service="database",
                healthy=True,
                message="Database connection OK",
                latency_ms=(time.monotonic() - start) * 1000
            )
        except Exception as e:
            return HealthStatus(
                service="database",
                healthy=False,
                message=str(e)
            )

    async def _check_provider(self) -> HealthStatus:
        start = time.monotonic()
        try:
            # Lightweight health check - just verify configuration
            if self.agent_loop.provider.api_key:
                return HealthStatus(
                    service="provider",
                    healthy=True,
                    message="Provider configured",
                    latency_ms=(time.monotonic() - start) * 1000
                )
            else:
                return HealthStatus(
                    service="provider",
                    healthy=False,
                    message="Provider not configured"
                )
        except Exception as e:
            return HealthStatus(
                service="provider",
                healthy=False,
                message=str(e)
            )

    async def _check_channels(self) -> HealthStatus:
        # Check enabled channels
        healthy_channels = []
        unhealthy_channels = []

        for name, channel in self.agent_loop.channels.channels.items():
            if channel.is_running:
                healthy_channels.append(name)
            else:
                unhealthy_channels.append(name)

        if unhealthy_channels:
            return HealthStatus(
                service="channels",
                healthy=len(unhealthy_channels) == 0,
                message=f"Healthy: {healthy_channels}, Unhealthy: {unhealthy_channels}"
            )

        return HealthStatus(
            service="channels",
            healthy=True,
            message=f"All channels running: {healthy_channels}"
        )


# HTTP health endpoint (optional, for container orchestration)
from aiohttp import web

async def health_endpoint(request):
    checker = request.app["health_checker"]
    checks = await checker.check_all()

    overall_healthy = all(c.healthy for c in checks.values())

    response_data = {
        "healthy": overall_healthy,
        "checks": {
            name: {
                "healthy": c.healthy,
                "message": c.message,
                "latency_ms": c.latency_ms,
            }
            for name, c in checks.items()
        }
    }

    status = 200 if overall_healthy else 503
    return web.json_response(response_data, status=status)
```

---

## 7. Graceful Degradation

### Current State

Limited fallback behavior when services fail.

### Recommended Implementation

```python
# nanobot/utils/fallback.py
from typing import Optional, Callable, Any
import asyncio

class FallbackProvider:
    """Provider with fallback chain."""

    def __init__(self, providers: list[LLMProvider]):
        """
        Args:
            providers: List of providers in priority order
        """
        self.providers = providers
        self._current_index = 0

    async def chat(self, messages: list[dict], **kwargs) -> LLMResponse:
        """Try providers in order until one succeeds."""
        last_error = None

        for i, provider in enumerate(self.providers):
            try:
                logger.info(f"Trying provider {i + 1}/{len(self.providers)}")
                response = await provider.chat(messages, **kwargs)
                if i > 0:
                    logger.info(f"Switched to provider {i + 1}")
                    self._current_index = i
                return response
            except Exception as e:
                last_error = e
                logger.warning(f"Provider {i + 1} failed: {e}")
                continue

        raise FallbackExhaustedError(
            f"All {len(self.providers)} providers failed. Last error: {last_error}"
        )

    @property
    def current_provider(self) -> int:
        return self._current_index


class FallbackExhaustedError(Exception):
    """All fallback providers exhausted."""
    pass


# Usage
primary = LiteLLMProvider(
    api_key=config.providers.openrouter.api_key,
    default_model="anthropic/claude-opus-4-5"
)

fallback = LiteLLMProvider(
    api_key=config.providers.openai.api_key,
    default_model="gpt-4o-mini"
)

provider = FallbackProvider([primary, fallback])
```

---

## 8. Horizontal Scaling

### Current State

Single-instance architecture with in-memory state.

### Recommended Architecture for Scale

```
┌─────────────────────────────────────────────────────────────┐
│                     Load Balancer                            │
│                     (nginx, HAProxy)                         │
└────────────────────────┬────────────────────────────────────┘
                         │
        ┌────────────────┼────────────────┐
        │                │                │
        ▼                ▼                ▼
┌───────────────┐ ┌───────────────┐ ┌───────────────┐
│  Nanobot #1   │ │  Nanobot #2   │ │  Nanobot #3   │
│  (Stateless)  │ │  (Stateless)  │ │  (Stateless)  │
└───────┬───────┘ └───────┬───────┘ └───────┬───────┘
        │                │                │
        └────────────────┼────────────────┘
                         │
        ┌────────────────┼────────────────┐
        │                │                │
        ▼                ▼                ▼
┌───────────────┐ ┌───────────────┐ ┌───────────────┐
│   Redis       │ │   PostgreSQL  │ │  S3/Blob      │
│   (Cache/     │ │   (Primary    │ │  Storage      │
│    Sessions)  │ │    DB)        │ │  (Files)      │
└───────────────┘ └───────────────┘ └───────────────┘
```

### Stateless Agent Design

```python
# nanobot/agent/stateless.py
class StatelessAgent:
    """Agent designed for horizontal scaling."""

    def __init__(
        self,
        redis: Redis,  # Shared session store
        db: Database,
        provider: LLMProvider,
        instance_id: str,
    ):
        self.redis = redis
        self.db = db
        self.provider = provider
        self.instance_id = instance_id
        self._running = False

    async def process_message(self, msg: InboundMessage) -> OutboundMessage:
        # Acquire lock for this session (prevent concurrent processing)
        lock_key = f"session_lock:{msg.session_key}"
        lock = self.redis.lock(lock_key, timeout=30)

        if not await lock.acquire():
            # Another instance is processing - skip or queue
            logger.info(f"Session {msg.session_key} locked by another instance")
            return None

        try:
            # Load session from shared DB
            session = await self.db.get_session(msg.session_key)

            # Process message
            response = await self._do_process(msg, session)

            # Save session
            await self.db.save_session(msg.session_key, session)

            return response

        finally:
            await lock.release()
```

---

## 9. Configuration for Production

### Production Config Template

```json
{
  "agents": {
    "defaults": {
      "workspace": "/var/nanobot/workspace",
      "model": "anthropic/claude-opus-4-5",
      "max_tokens": 8192,
      "temperature": 0.7,
      "max_tool_iterations": 15
    }
  },
  "providers": {
    "openrouter": {
      "api_key": "${OPENROUTER_API_KEY}",
      "api_base": "https://openrouter.ai/api/v1"
    },
    "anthropic": {
      "api_key": "${ANTHROPIC_API_KEY}"
    }
  },
  "channels": {
    "telegram": {
      "enabled": true,
      "token": "${TELEGRAM_BOT_TOKEN}",
      "allow_from": ["123456789"],
      "proxy": "${TELEGRAM_PROXY_URL:-}"
    }
  },
  "tools": {
    "restrict_to_workspace": true,
    "exec": {
      "timeout": 30
    },
    "web": {
      "search": {
        "api_key": "${BRAVE_API_KEY}",
        "max_results": 5
      }
    }
  },
  "production": {
    "rate_limit": {
      "requests_per_minute": 60,
      "tokens_per_minute": 100000
    },
    "circuit_breaker": {
      "failure_threshold": 5,
      "recovery_timeout": 60
    },
    "metrics": {
      "enabled": true,
      "port": 9090
    },
    "health_check": {
      "enabled": true,
      "port": 8080
    }
  }
}
```

---

## 10. Deployment Checklist

### Pre-Deployment

- [ ] API keys configured via environment variables or secrets manager
- [ ] Workspace directory created with correct permissions
- [ ] Database backend configured (SQLite or PostgreSQL)
- [ ] Rate limits configured for all providers
- [ ] Circuit breakers configured
- [ ] Logging configured for production (JSON format)
- [ ] Metrics collection enabled
- [ ] Health checks configured

### Deployment

- [ ] Container image built and scanned for vulnerabilities
- [ ] Resource limits set (CPU, memory)
- [ ] Persistent volumes mounted for data
- [ ] Network policies configured
- [ ] Liveness/readiness probes configured

### Post-Deployment

- [ ] Health endpoint responding correctly
- [ ] Metrics being collected
- [ ] Logs flowing to central system
- [ ] Alert rules configured
- [ ] Backup strategy for database

---

## 11. Monitoring & Alerting

### Key Metrics to Monitor

| Metric | Alert Threshold | Description |
|--------|-----------------|-------------|
| `nanobot_messages_processed_total{status="error"}` | > 10% error rate | Message processing failures |
| `nanobot_llm_latency_seconds` | p99 > 30s | Slow LLM responses |
| `nanobot_tool_latency_seconds` | p99 > 10s | Slow tool execution |
| `nanobot_active_sessions` | Sudden drop | Service availability issue |
| `nanobot_circuit_breaker_state` | OPEN | Circuit breaker triggered |

### Alerting Rules (Prometheus)

```yaml
groups:
  - name: nanobot
    rules:
      - alert: NanobotHighErrorRate
        expr: |
          rate(nanobot_messages_processed_total{status="error"}[5m])
          / rate(nanobot_messages_processed_total[5m]) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: High error rate in nanobot

      - alert: NanobotCircuitBreakerOpen
        expr: nanobot_circuit_breaker_state == 1
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: Circuit breaker open for {{ $labels.service }}

      - alert: NanobotHighLatency
        expr: |
          histogram_quantile(0.99,
            rate(nanobot_llm_latency_seconds_bucket[5m])
          ) > 30
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: High LLM latency (p99 > 30s)
```

---

## Summary

nanobot has a solid foundation for production use but needs additional investment in:

1. **Observability** - Structured logging, metrics, tracing
2. **Resilience** - Rate limiting, circuit breakers, fallbacks
3. **Security** - Secrets management, enhanced access control
4. **Scalability** - Stateless design, shared storage
5. **Operations** - Health checks, monitoring, alerting

With these additions, nanobot can support enterprise-grade deployments while maintaining its lightweight philosophy.
