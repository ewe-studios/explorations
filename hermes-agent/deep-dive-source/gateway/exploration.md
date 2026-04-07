# gateway/ Deep Dive Exploration

**Source Directory:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/hermes-agent/gateway/`

**Status:** complete

---

## Module Overview

The `gateway/` module is the multi-platform messaging integration layer for Hermes Agent. This ~32,629 line module provides connectivity to various messaging platforms (Telegram, Discord, WhatsApp, Slack, Signal, etc.) with unified session management, context injection, delivery routing, and platform-specific toolsets.

Key features:
- **Multi-platform support** - 15+ messaging platforms from a single codebase
- **Session management** - Persistent conversations with configurable reset policies
- **Dynamic context injection** - Agent knows where messages come from (platform, chat, thread)
- **Delivery routing** - Cron job outputs routed to appropriate channels
- **Platform-specific toolsets** - Different capabilities per platform
- **Unified configuration** - Single YAML config for all platforms
- **Pairing system** - Secure user onboarding via pairing codes

The gateway runs as a daemon service (`hermes gateway`) and ticks the cron scheduler every 60 seconds.

---

## Directory Structure

### Core Files

| File | Lines | Purpose |
|------|-------|---------|
| `__init__.py` | 35 | Package exports |
| `run.py` | 7,381 | Main gateway entry point |
| `config.py` | 943 | Configuration management |
| `session.py` | 1,081 | Session store and reset policies |
| `delivery.py` | 351 | Delivery routing |
| `hooks.py` | 170 | Gateway hooks |
| `channel_directory.py` | 272 | Human-friendly channel names |
| `mirror.py` | 132 | Message mirroring |
| `pairing.py` | 309 | User pairing system |
| `status.py` | 391 | Gateway status reporting |
| `sticker_cache.py` | 111 | Sticker/emoji caching |
| `stream_consumer.py` | 234 | Stream processing |

### Platforms

| File | Lines | Purpose |
|------|-------|---------|
| `platforms/__init__.py` | 17 | Platforms package |
| `platforms/base.py` | 1,666 | Base platform class |
| `platforms/telegram.py` | 2,532 | Telegram messenger |
| `platforms/discord.py` | 2,255 | Discord messenger |
| `platforms/slack.py` | 1,080 | Slack messenger |
| `platforms/whatsapp.py` | 867 | WhatsApp messenger |
| `platforms/signal.py` | 867 | Signal messenger |
| `platforms/matrix.py` | 2,048 | Matrix protocol |
| `platforms/mattermost.py` | 742 | Mattermost |
| `platforms/homeassistant.py` | 449 | Home Assistant |
| `platforms/email.py` | 621 | Email integration |
| `platforms/sms.py` | 276 | SMS gateway |
| `platforms/dingtalk.py` | 340 | DingTalk (China) |
| `platforms/feishu.py` | 3,445 | Feishu/Lark (China) |
| `platforms/wecom.py` | 1,338 | WeCom/WeChat Work |
| `platforms/webhook.py` | 629 | Generic webhook |
| `platforms/api_server.py` | 1,638 | REST API server |
| `platforms/telegram_network.py` | 248 | Telegram network routing |

### Built-in Hooks

| File | Lines | Purpose |
|------|-------|---------|
| `builtin_hooks/__init__.py` | 1 | Hooks package |
| `builtin_hooks/boot_md.py` | 86 | Boot message rendering |

**Total:** ~32,629 lines across 30+ files

---

## Key Components

### 1. Configuration (`config.py`)

Handles loading and validating gateway configuration.

**Key Classes:**
```python
class Platform(Enum):
    """Supported messaging platforms."""
    TELEGRAM = "telegram"
    DISCORD = "discord"
    WHATSAPP = "whatsapp"
    SLACK = "slack"
    SIGNAL = "signal"
    MATRIX = "matrix"
    MATTERMOST = "mattermost"
    HOMEASSISTANT = "homeassistant"
    EMAIL = "email"
    SMS = "sms"
    DINGTALK = "dingtalk"
    API_SERVER = "api_server"
    WEBHOOK = "webhook"
    FEISHU = "feishu"
    WECOM = "wecom"

@dataclass
class HomeChannel:
    """Default destination for a platform."""
    platform: Platform
    chat_id: str
    name: str  # Human-readable name

@dataclass
class SessionResetPolicy:
    """Controls when sessions reset (lose context)."""
    mode: str = "both"  # "daily", "idle", "both", "none"
    at_hour: int = 4  # Hour for daily reset
    idle_minutes: int = 1440  # 24 hours
    notify: bool = True

@dataclass
class PlatformConfig:
    """Configuration for a single platform."""
    enabled: bool = False
    token: Optional[str] = None
    api_key: Optional[str] = None
    home_channel: Optional[HomeChannel] = None
    reset_policy: SessionResetPolicy = field(default_factory=SessionResetPolicy)

@dataclass
class GatewayConfig:
    """Top-level gateway configuration."""
    platforms: Dict[Platform, PlatformConfig] = field(default_factory=dict)
    home_channels: Dict[Platform, HomeChannel] = field(default_factory=dict)
    reset_policy: SessionResetPolicy = field(default_factory=SessionResetPolicy)
    delivery: Dict[str, Any] = field(default_factory=dict)
```

**Config Loading:**
```python
def load_gateway_config() -> GatewayConfig:
    """Load configuration from ~/.hermes/config.yaml."""
```

### 2. Session Management (`session.py`)

Manages persistent conversations with reset policies.

**Key Classes:**
```python
@dataclass
class SessionContext:
    """Context for a platform session."""
    platform: str
    chat_id: str
    thread_id: Optional[str]
    user_name: Optional[str]
    created_at: datetime
    last_activity: datetime
    message_count: int = 0

class SessionStore:
    """Manages sessions across all platforms."""
    
    def get_or_create(
        self, 
        platform: str, 
        chat_id: str, 
        thread_id: Optional[str] = None
    ) -> SessionContext:
        """Get or create a session for a chat."""
    
    def should_reset(self, session: SessionContext) -> bool:
        """Check if session should be reset per policy."""
    
    def reset(self, session: SessionContext) -> None:
        """Reset a session (clear context)."""
```

**Reset Policies:**
| Mode | Behavior |
|------|----------|
| `daily` | Reset at configured hour (e.g., 4 AM) |
| `idle` | Reset after N minutes of inactivity |
| `both` | Whichever triggers first |
| `none` | Never auto-reset |

**Session Context Prompt:**
```python
def build_session_context_prompt(session: SessionContext) -> str:
    """Build platform context for system prompt.
    
    Example output:
    ```
    You are currently interacting via Telegram.
    User: @username
    Chat: "Family Group" (id: 12345)
    Thread: Reply to message about dinner plans
    ```
    """
```

### 3. Delivery Routing (`delivery.py`)

Routes cron job outputs and agent messages to appropriate channels.

**Key Classes:**
```python
@dataclass
class DeliveryTarget:
    """A delivery destination."""
    platform: str
    chat_id: str
    thread_id: Optional[str]

class DeliveryRouter:
    """Routes messages to delivery targets."""
    
    def resolve(
        self, 
        deliver_spec: str, 
        origin: Optional[dict] = None
    ) -> Optional[DeliveryTarget]:
        """Resolve delivery spec to concrete target.
        
        Specs:
            "local" -> None (no delivery)
            "origin" -> Original platform/chat
            "telegram" -> Telegram home channel
            "discord:channel_id" -> Specific channel
            "Alice (dm)" -> Resolved via channel directory
        """
```

### 4. Platform Base Class (`platforms/base.py`)

Abstract base class for all platform implementations.

**Key Class:**
```python
class PlatformConnector(ABC):
    """Base class for platform connectors."""
    
    def __init__(self, config: PlatformConfig, gateway: "Gateway"):
        self.config = config
        self.gateway = gateway
        self._connected = False
    
    @abstractmethod
    async def connect(self) -> None:
        """Establish connection to platform."""
    
    @abstractmethod
    async def disconnect(self) -> None:
        """Disconnect from platform."""
    
    @abstractmethod
    async def send_message(
        self, 
        chat_id: str, 
        content: str,
        thread_id: Optional[str] = None,
        reply_to: Optional[str] = None,
    ) -> None:
        """Send a message to a chat."""
    
    @abstractmethod
    async def send_file(
        self, 
        chat_id: str, 
        file_path: str,
        caption: Optional[str] = None,
    ) -> None:
        """Send a file to a chat."""
    
    @abstractmethod
    def get_toolset(self) -> List[str]:
        """Get platform-specific toolset."""
    
    def handle_incoming_message(
        self, 
        chat_id: str, 
        content: str,
        sender: str,
        thread_id: Optional[str] = None,
    ) -> None:
        """Handle incoming message from platform."""
```

### 5. Platform Implementations

#### Telegram (`platforms/telegram.py`)
- Bot API integration
- Group and DM support
- Reply threading
- File/photo/voice support
- Markdown/HTML formatting

#### Discord (`platforms/discord.py`)
- discord.py library
- Guild/channel permissions
- Thread support
- Embed formatting
- Attachment handling

#### Slack (`platforms/slack.py`)
- Slack Bolt framework
- Channel/DM detection
- Thread replies
- Block Kit formatting

#### WhatsApp (`platforms/whatsapp.py`)
- WhatsApp Business API
- Group messaging
- Media support

#### Signal (`platforms/signal.py`)
- signal-cli integration
- DM and group support

#### Matrix (`platforms/matrix.py`)
- Matrix protocol (matrix-nio)
- Room-based conversations
- End-to-end encryption support

#### Home Assistant (`platforms/homeassistant.py`)
- Home Assistant webhook integration
- Event triggering
- Service calls

#### Email (`platforms/email.py`)
- IMAP/SMTP integration
- Email-to-message parsing
- HTML/text handling

#### API Server (`platforms/api_server.py`)
- REST API endpoints
- WebSocket support
- Custom client integration

### 6. Channel Directory (`channel_directory.py`)

Human-friendly channel name resolution.

**Key Function:**
```python
def resolve_channel_name(platform: str, name: str) -> Optional[str]:
    """Resolve human-friendly name to platform chat_id.
    
    Example:
        resolve_channel_name("telegram", "Alice (dm)") -> "12345"
        resolve_channel_name("discord", "dev-chat") -> "98765"
    
    Uses ~/.hermes/channels.yaml for mapping.
    """
```

### 7. Pairing System (`pairing.py`)

Secure user onboarding via pairing codes.

**Flow:**
1. User sends `/start` to bot
2. Bot generates pairing code
3. User enters code in authorized interface
4. Chat ID recorded as authorized user

**Key Functions:**
```python
def generate_pairing_code() -> str:
    """Generate a secure pairing code."""

def verify_pairing_code(code: str) -> Optional[str]:
    """Verify and consume a pairing code."""

def register_paired_chat(platform: str, chat_id: str) -> None:
    """Register a chat as paired/authorized."""
```

### 8. Gateway Runner (`run.py`)

Main gateway entry point and event loop.

**Key Class:**
```python
class Gateway:
    """Main gateway orchestrator."""
    
    def __init__(self, config: GatewayConfig):
        self.config = config
        self.session_store = SessionStore(config.reset_policy)
        self.delivery_router = DeliveryRouter(config)
        self.platforms: Dict[Platform, PlatformConnector] = {}
        self._cron_lock = Lock()
    
    async def start(self) -> None:
        """Start all platform connectors and cron scheduler."""
    
    async def stop(self) -> None:
        """Stop all connectors gracefully."""
    
    async def tick(self) -> None:
        """Cron scheduler tick (every 60 seconds)."""
    
    def handle_message(
        self,
        platform: str,
        chat_id: str,
        content: str,
        sender: str,
        thread_id: Optional[str] = None,
    ) -> None:
        """Route incoming message to agent."""
```

### 9. Stream Consumer (`stream_consumer.py`)

Processes streaming agent responses.

**Key Class:**
```python
class StreamConsumer:
    """Consumes streaming agent output for platform delivery."""
    
    async def consume(
        self,
        stream: AsyncIterator[dict],
        target: DeliveryTarget,
    ) -> None:
        """Process stream and send to platform.
        
        Handles:
        - Text chunks (concatenation)
        - Tool call previews
        - Thinking content (optional)
        - Final response formatting
        """
```

---

## Platform Toolsets

Different platforms have different tool capabilities:

| Platform | Terminal | Files | Browser | Memory |
|----------|----------|-------|---------|--------|
| Telegram | ✓ | ✓ | ✓ | ✓ |
| Discord | ✓ | ✓ | ✓ | ✓ |
| Slack | ✓ | ✓ | Limited | ✓ |
| WhatsApp | Limited | ✓ | No | ✓ |
| Signal | Limited | ✓ | No | ✓ |
| API Server | ✓ | ✓ | ✓ | ✓ |
| Webhook | No | No | No | No |

---

## Integration Points

### With Cron System
- `Gateway.tick()` calls `cron.scheduler.tick()` every 60 seconds
- Delivery router sends cron output to configured targets

### With Session Database
- Sessions persisted to `~/.hermes/state.db`
- Survives gateway restarts
- Searchable via session_search tool

### With Agent (`agent/`)
- Session context injected into system prompt
- Platform-specific toolsets activated
- Streaming response handling

### With CLI (`hermes_cli/`)
- `hermes gateway start/stop/status` commands
- `hermes gateway install` for systemd service

---

## Related Files

**Individual File Explorations:**
- [run.md](./gateway/run.md) - Main gateway
- [session.md](./gateway/session.md) - Session management
- [config.md](./gateway/config.md) - Configuration
- [delivery.md](./gateway/delivery.md) - Delivery routing
- [platforms/telegram.md](./gateway/platforms/telegram.md) - Telegram
- [platforms/discord.md](./gateway/platforms/discord.md) - Discord
- [platforms/slack.md](./gateway/platforms/slack.md) - Slack

**Related Modules:**
- [cron/exploration.md](../cron/exploration.md) - Cron scheduler
- [hermes_cli/gateway.md](../hermes_cli/gateway.md) - CLI gateway commands
- [tools/send_message_tool.md](../tools/send_message_tool.md) - Agent message tool

---

*Deep dive created: 2026-04-07*
