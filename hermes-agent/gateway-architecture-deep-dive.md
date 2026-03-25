---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.NousResearch/hermes-agent
repository: git@github.com:NousResearch/hermes-agent.git
explored_at: 2026-03-25
---

# Gateway Architecture Deep Dive

The Hermes Gateway is a multi-platform messaging gateway that allows users to interact with the agent from Telegram, Discord, Slack, WhatsApp, Signal, Email, SMS, and Home Assistant. This document explores the gateway architecture, each platform adapter, connection flows, and message handling.

## Gateway Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     Gateway Runner                               │
│                    (gateway/run.py)                              │
├─────────────────────────────────────────────────────────────────┤
│  Session Store  │  Delivery Router  │  Agent Cache             │
│  (session.py)   │  (delivery.py)    │  (per-session AIAgent)    │
├─────────────────────────────────────────────────────────────────┤
│                     Platform Adapters                            │
│  ┌───────┐ ┌────────┐ ┌───────┐ ┌─────────┐ ┌────────────────┐ │
│  │Telegram│ │Discord │ │ Slack │ │WhatsApp │ │Home Assistant  │ │
│  └───────┘ └────────┘ └───────┘ └─────────┘ └────────────────┘ │
│  ┌───────┐ ┌────────┐ ┌───────┐ ┌─────────┐                    │
│  │ Signal │ │ Email  │ │  SMS  │ │ Matrix  │                    │
│  └───────┘ └────────┘ └───────┘ └─────────┘                    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    AIAgent Core                                  │
│                   (run_agent.py)                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Gateway Runner

### Configuration Loading

```python
# gateway/run.py

class GatewayRunner:
    """Main gateway controller managing platform adapter lifecycles."""

    def __init__(self, config: Optional[GatewayConfig] = None):
        self.config = config or load_gateway_config()
        self.adapters: Dict[Platform, BasePlatformAdapter] = {}

        # Load ephemeral config from config.yaml / env vars
        self._prefill_messages = self._load_prefill_messages()
        self._ephemeral_system_prompt = self._load_ephemeral_system_prompt()
        self._reasoning_config = self._load_reasoning_config()
        self._show_reasoning = self._load_show_reasoning()
        self._provider_routing = self._load_provider_routing()
        self._fallback_model = self._load_fallback_model()

        # Session store with process registry integration
        from tools.process_registry import process_registry
        self.session_store = SessionStore(
            self.config.sessions_dir,
            self.config,
            has_active_processes_fn=lambda key: process_registry.has_active_for_session(key),
        )

        self.delivery_router = DeliveryRouter(self.config)
        self._running = False
        self._shutdown_event = asyncio.Event()

        # Track running agents per session for interrupt support
        self._running_agents: Dict[str, Any] = {}
        self._pending_messages: Dict[str, str] = {}

        # Cache AIAgent instances per session to preserve prompt caching
        self._agent_cache: Dict[str, tuple] = {}
        self._agent_cache_lock = threading.Lock()

        # DM pairing store for code-based user authorization
        from gateway.pairing import PairingStore
        self.pairing_store = PairingStore()

        # Event hook system
        from gateway.hooks import HookRegistry
        self.hooks = HookRegistry()

        # SQLite session database for session_search tool support
        try:
            from hermes_state import SessionDB
            self._session_db = SessionDB()
        except Exception:
            pass
```

### Agent Instance Caching

This is critical for **prompt caching preservation**:

```python
def _get_or_create_agent(self, session_key: str,
                          event: MessageEvent) -> AIAgent:
    """Get cached AIAgent or create new one.

    Caching preserves Anthropic prompt cache — without it,
    every message rebuilds system prompt (including memory)
    breaking prefix cache and costing ~10x more.
    """
    from hermes_cli.config import compute_config_signature

    with self._agent_cache_lock:
        if session_key in self._agent_cache:
            agent, cached_sig = self._agent_cache[session_key]
            current_sig = compute_config_signature()

            if cached_sig == current_sig:
                return agent  # Cache hit

            # Config changed — evict and recreate
            logger.info("Config changed, recreating agent for %s", session_key)

        # Create new agent
        agent = AIAgent(
            model=self.current_model,
            platform="gateway",
            session_id=event.session_id,
            # ... other params
        )

        self._agent_cache[session_key] = (agent, compute_config_signature())
        return agent
```

### Message Processing Flow

```python
async def _handle_message_event(self, event: MessageEvent):
    """Handle incoming message from any platform."""
    session_key = build_session_key(event)

    # Check for active agent (interrupt scenario)
    if session_key in self._running_agents:
        # Queue message for after current task completes
        self._pending_messages[session_key] = event.text
        await self._send_info_message(
            session_key, "Message queued — finishing current task..."
        )
        return

    # Check DM pairing requirement
    if self.config.require_dm_pairing and not self.pairing_store.is_paired(event.sender_id):
        pairing_code = self.pairing_store.generate_pairing_code(event.sender_id)
        await self._send_info_message(
            session_key,
            f"Please pair this chat by visiting: https://hermes-agent.nousresearch.com/pair/{pairing_code}"
        )
        return

    # Mark session as running (prevents duplicate processing)
    self._running_agents[session_key] = _AGENT_PENDING_SENTINEL

    try:
        # Get or create cached agent
        agent = self._get_or_create_agent(session_key, event)

        # Build session context
        session_context = build_session_context(
            source=event.platform.value,
            user_id=event.sender_id,
            chat_id=event.chat_id,
            thread_id=event.thread_id,
        )

        # Inject platform-specific context
        if event.platform == Platform.DISCORD:
            # Discord-specific context (mentions, channels)
            pass
        elif event.platform == Platform.TELEGRAM:
            # Telegram-specific context (forum topics, etc.)
            pass

        # Run agent with callbacks
        loop = asyncio.get_event_loop()
        response = await loop.run_in_executor(
            None,
            lambda: agent.chat(
                message=event.text,
                session_context=session_context,
                tool_progress_callback=lambda cb: asyncio.create_task(
                    self._handle_tool_callback(session_key, cb)
                ),
            )
        )

        # Deliver response via router
        await self.delivery_router.deliver(
            session_key=session_key,
            platform=event.platform,
            response=response,
            config=self.config,
        )

    finally:
        # Clear running status
        if session_key in self._running_agents:
            del self._running_agents[session_key]

        # Process any queued messages
        if session_key in self._pending_messages:
            pending = self._pending_messages.pop(session_key)
            await self._handle_message_event(
                MessageEvent(text=pending, ...)
            )
```

## Platform Adapters

### Base Platform Adapter

```python
# gateway/platforms/base.py

class BasePlatformAdapter(ABC):
    """Base class for all platform adapters."""

    def __init__(self, config: PlatformConfig, platform: Platform):
        self.config = config
        self.platform = platform
        self._message_callback: Optional[Callable] = None
        self._running = False

    @abstractmethod
    async def connect(self) -> bool:
        """Establish connection to platform."""

    @abstractmethod
    async def disconnect(self):
        """Disconnect from platform."""

    @abstractmethod
    async def send_message(self, chat_id: str, text: str,
                           thread_id: str = None) -> SendResult:
        """Send a message to the platform."""

    @abstractmethod
    async def send_media(self, chat_id: str, media_type: str,
                         media: bytes) -> SendResult:
        """Send media (image/audio/document)."""

    def set_message_callback(self, callback: Callable):
        """Set callback for incoming messages."""
        self._message_callback = callback
```

### Telegram Adapter

```python
# gateway/platforms/telegram.py

class TelegramAdapter(BasePlatformAdapter):
    """Telegram bot adapter using python-telegram-bot."""

    MAX_MESSAGE_LENGTH = 4096
    MEDIA_GROUP_WAIT_SECONDS = 0.8

    def __init__(self, config: PlatformConfig):
        super().__init__(config, Platform.TELEGRAM)
        self._app: Optional[Application] = None
        self._bot: Optional[Bot] = None

        # Buffer rapid photo updates (album handling)
        self._pending_photo_batches: Dict[str, MessageEvent] = {}
        self._pending_photo_batch_tasks: Dict[str, asyncio.Task] = {}

        # Buffer rapid text messages (client-side splits)
        self._pending_text_batches: Dict[str, MessageEvent] = {}
        self._pending_text_batch_tasks: Dict[str, asyncio.Task] = {}

    async def connect(self) -> bool:
        """Initialize Telegram bot and start polling."""
        if not TELEGRAM_AVAILABLE:
            logger.error("Telegram dependencies not installed")
            return False

        token = os.getenv("TELEGRAM_BOT_TOKEN")
        if not token:
            logger.error("TELEGRAM_BOT_TOKEN not set")
            return False

        # Build application
        self._app = Application.builder().token(token).build()
        self._bot = self._app.bot

        # Register handlers
        self._app.add_handler(CommandHandler("start", self._on_start))
        self._app.add_handler(CommandHandler("help", self._on_help))
        self._app.add_handler(
            TelegramMessageHandler(
                self._on_message,
                filters=~filters.COMMAND
            )
        )

        # Start polling
        await self._app.initialize()
        await self._app.start()
        updater = self._app.updater
        await updater.start_polling(
            allowed_updates=["message", "edited_message"]
        )

        logger.info("Telegram bot started: @%s", (await self._bot.get_me()).username)
        return True

    async def _on_message(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        """Handle incoming Telegram message."""
        message = update.effective_message

        # Handle different message types
        if message.text:
            await self._handle_text_message(message)
        elif message.photo:
            await self._handle_photo_message(message)
        elif message.voice:
            await self._handle_voice_message(message)
        elif message.document:
            await self._handle_document_message(message)

    async def _handle_text_message(self, message: Message):
        """Buffer rapid text messages for aggregation."""
        chat_key = f"{message.chat.id}"

        # Check for existing batch
        if chat_key in self._pending_text_batches:
            # Append to existing batch
            event = self._pending_text_batches[chat_key]
            event.text += "\n" + message.text

            # Cancel existing timer
            if chat_key in self._pending_text_batch_tasks:
                self._pending_text_batch_tasks[chat_key].cancel()

        else:
            # Create new batch
            event = MessageEvent(
                platform=Platform.TELEGRAM,
                sender_id=str(message.from_user.id),
                chat_id=str(message.chat.id),
                text=message.text,
                timestamp=message.date.timestamp(),
            )
            self._pending_text_batches[chat_key] = event

        # Set timer to process batch
        task = asyncio.create_task(
            self._process_text_batch(chat_key)
        )
        self._pending_text_batch_tasks[chat_key] = task

    async def _process_text_batch(self, chat_key: str):
        """Process buffered text message after delay."""
        await asyncio.sleep(self._text_batch_delay_seconds)

        event = self._pending_text_batches.pop(chat_key, None)
        if event and self._message_callback:
            await self._message_callback(event)

    async def send_message(self, chat_id: str, text: str,
                           thread_id: str = None) -> SendResult:
        """Send message with MarkdownV2 formatting."""
        try:
            # Escape MarkdownV2 special characters
            escaped_text = _escape_mdv2(text)

            # Split long messages
            if len(escaped_text) > self.MAX_MESSAGE_LENGTH:
                parts = self._split_message(escaped_text)
                for part in parts:
                    await self._bot.send_message(
                        chat_id=chat_id,
                        text=part,
                        parse_mode=ParseMode.MARKDOWN_V2,
                        message_thread_id=thread_id,
                    )
            else:
                await self._bot.send_message(
                    chat_id=chat_id,
                    text=escaped_text,
                    parse_mode=ParseMode.MARKDOWN_V2,
                    message_thread_id=thread_id,
                )

            return SendResult(success=True)

        except Exception as e:
            logger.exception("Telegram send failed: %s", e)
            return SendResult(success=False, error=str(e))
```

### Discord Adapter

```python
# gateway/platforms/discord.py

class DiscordAdapter(BasePlatformAdapter):
    """Discord bot adapter using discord.py."""

    def __init__(self, config: PlatformConfig):
        super().__init__(config, Platform.DISCORD)
        self._bot: Optional[commands.Bot] = None
        self._intents = Intents.default()
        self._intents.message_content = True
        self._intents.members = True

        # Voice receiver for voice messages
        self._voice_receiver: Optional[VoiceReceiver] = None

    async def connect(self) -> bool:
        """Initialize Discord bot and connect to gateway."""
        if not DISCORD_AVAILABLE:
            logger.error("Discord dependencies not installed")
            return False

        token = os.getenv("DISCORD_BOT_TOKEN")
        if not token:
            logger.error("DISCORD_BOT_TOKEN not set")
            return False

        # Build bot
        self._bot = commands.Bot(
            command_prefix="!",
            intents=self._intents,
        )

        # Register event handlers
        @self._bot.event
        async def on_ready():
            logger.info("Discord bot connected: %s", self._bot.user)

        @self._bot.event
        async def on_message(message: DiscordMessage):
            # Ignore bot's own messages
            if message.author == self._bot.user:
                return

            await self._handle_discord_message(message)

        # Start bot
        await self._bot.start(token)
        return True

    async def _handle_discord_message(self, message: DiscordMessage):
        """Handle incoming Discord message."""
        # Build message event
        event = MessageEvent(
            platform=Platform.DISCORD,
            sender_id=str(message.author.id),
            chat_id=str(message.channel.id),
            thread_id=str(message.thread.id) if message.thread else None,
            text=message.content,
            timestamp=message.created_at.timestamp(),
        )

        # Handle attachments (images, documents)
        for attachment in message.attachments:
            if attachment.content_type.startswith("image/"):
                image_path = await cache_image_from_url(attachment.url)
                event.attachments.append({
                    "type": "image",
                    "path": image_path,
                })
            elif attachment.content_type.startswith("audio/"):
                audio_path = await cache_audio_from_url(attachment.url)
                event.attachments.append({
                    "type": "audio",
                    "path": audio_path,
                })

        # Check for mentions (bot must be mentioned in non-DM channels)
        if not message.channel.dm and not self._bot.user.mentioned_in(message):
            # No mention in server channel — ignore
            return

        if self._message_callback:
            await self._message_callback(event)

    async def send_message(self, chat_id: str, text: str,
                           thread_id: str = None) -> SendResult:
        """Send message to Discord channel/thread."""
        try:
            channel = await self._bot.fetch_channel(int(chat_id))

            # Split long messages
            if len(text) > 2000:  # Discord limit
                parts = self._split_message(text)
                for part in parts:
                    await channel.send(content=part)
            else:
                await channel.send(content=text)

            return SendResult(success=True)

        except Exception as e:
            logger.exception("Discord send failed: %s", e)
            return SendResult(success=False, error=str(e))

    async def connect_voice(self, channel_id: int,
                            allowed_user_ids: set = None):
        """Connect to voice channel and start receiving audio."""
        channel = self._bot.get_channel(channel_id)
        if not channel:
            raise ValueError(f"Channel {channel_id} not found")

        voice_client = await channel.connect()

        self._voice_receiver = VoiceReceiver(
            voice_client,
            allowed_user_ids=allowed_user_ids,
        )
        self._voice_receiver.start()
```

### Slack Adapter

```python
# gateway/platforms/slack.py

class SlackAdapter(BasePlatformAdapter):
    """Slack bot adapter using slack_bolt."""

    def __init__(self, config: PlatformConfig):
        super().__init__(config, Platform.SLACK)
        self._app: Optional[App] = None

    async def connect(self) -> bool:
        """Initialize Slack app using Socket Mode."""
        token = os.getenv("SLACK_BOT_TOKEN")
        app_token = os.getenv("SLACK_APP_TOKEN")

        if not token or not app_token:
            logger.error("SLACK_BOT_TOKEN or SLACK_APP_TOKEN not set")
            return False

        self._app = App(token=token, app_token=app_token)

        # Register message handler
        @self._app.event("message")
        def handle_message(event, say):
            # Process message
            pass

        # Start Socket Mode connection
        await asyncio.to_thread(self._app.start)
        return True
```

### Home Assistant Adapter

```python
# gateway/platforms/homeassistant.py

class HomeAssistantAdapter(BasePlatformAdapter):
    """Home Assistant integration for smart home events."""

    def __init__(self, config: PlatformConfig):
        super().__init__(config, Platform.HOMEASSISTANT)
        self._hass: Optional[HomeAssistantClient] = None
        self._subscriptions: List[str] = []

    async def connect(self) -> bool:
        """Connect to Home Assistant WebSocket API."""
        url = os.getenv("HASS_URL")
        token = os.getenv("HASS_TOKEN")

        if not url or not token:
            logger.error("HASS_URL or HASS_TOKEN not set")
            return False

        from homeassistant_api import Client

        self._hass = Client(endpoint=url, token=token)

        # Subscribe to events
        await self._subscribe_to_events([
            "state_changed",
            "call_service",
        ])

        return True

    async def _subscribe_to_events(self, event_types: List[str]):
        """Subscribe to Home Assistant events."""
        for event_type in event_types:
            self._hass.subscribe(
                event_type,
                lambda event: self._handle_hass_event(event),
            )

    async def _handle_hass_event(self, event: dict):
        """Handle Home Assistant event (state change, etc.)."""
        if event.get("event_type") == "state_changed":
            entity_id = event["data"]["entity_id"]
            new_state = event["data"]["new_state"]

            # Format event for agent
            event_msg = MessageEvent(
                platform=Platform.HOMEASSISTANT,
                sender_id="homeassistant",
                chat_id="events",
                text=f"Entity {entity_id} changed to: {new_state['state']}",
                timestamp=time.time(),
            )

            if self._message_callback:
                await self._message_callback(event_msg)
```

## Message Event Structure

```python
# gateway/platforms/base.py

@dataclass
class MessageEvent:
    """Normalized message event from any platform."""
    platform: Platform
    sender_id: str
    chat_id: str
    text: str
    timestamp: float

    # Optional fields
    thread_id: Optional[str] = None
    attachments: List[Dict] = field(default_factory=list)
    voice_audio: Optional[str] = None  # Path to transcribed audio
    is_reply: bool = False
    reply_to_message_id: Optional[str] = None

    # Platform-specific metadata
    raw_data: Optional[dict] = None
```

## Delivery Router

```python
# gateway/delivery.py

class DeliveryRouter:
    """Route agent responses back to the correct platform."""

    def __init__(self, config: GatewayConfig):
        self.config = config

    async def deliver(self, session_key: str, platform: Platform,
                      response: str, config: GatewayConfig):
        """Deliver response via the appropriate adapter."""
        # Get adapter for platform
        adapter = self.adapters.get(platform)
        if not adapter:
            logger.error("No adapter for platform %s", platform)
            return

        # Parse session key to get chat_id, thread_id
        chat_id, thread_id = parse_session_key(session_key)

        # Split long responses
        parts = self._split_response(response, platform)

        for part in parts:
            await adapter.send_message(chat_id, part, thread_id)

    def _split_response(self, text: str, platform: Platform) -> List[str]:
        """Split response into platform-appropriate chunks."""
        limits = {
            Platform.TELEGRAM: 4096,
            Platform.DISCORD: 2000,
            Platform.SLACK: 4000,
            Platform.WHATSAPP: 4096,
        }

        limit = limits.get(platform, 4096)
        return split_text_preserving_paragraphs(text, limit)
```

## Session Management

```python
# gateway/session.py

class SessionStore:
    """Manage session state and persistence."""

    def __init__(self, sessions_dir: Path, config: GatewayConfig,
                 has_active_processes_fn=None):
        self.sessions_dir = sessions_dir
        self.config = config
        self.has_active_processes_fn = has_active_processes_fn

        # In-memory cache
        self._sessions: Dict[str, SessionContext] = {}

    def get_or_create(self, event: MessageEvent) -> SessionContext:
        """Get existing session or create new one."""
        key = build_session_key(event)

        if key not in self._sessions:
            self._sessions[key] = SessionContext(
                source=event.platform.value,
                user_id=event.sender_id,
                chat_id=event.chat_id,
                thread_id=event.thread_id,
                started_at=datetime.now(),
            )

        return self._sessions[key]

    def reset(self, key: str, preserve_skills: bool = True):
        """Reset session state (for /new or /reset commands)."""
        if key in self._sessions:
            old_session = self._sessions[key]

            # Check for active background processes
            if self.has_active_processes_fn and self.has_active_processes_fn(key):
                raise SessionResetBlockedError(
                    "Cannot reset: background process still running"
                )

            # Create new session with parent reference
            self._sessions[key] = SessionContext(
                source=old_session.source,
                user_id=old_session.user_id,
                chat_id=old_session.chat_id,
                parent_session_id=old_session.id,
                started_at=datetime.now(),
            )
```

## Summary

The gateway architecture provides:

1. **8 platform adapters** (Telegram, Discord, Slack, WhatsApp, Signal, Email, SMS, Home Assistant)
2. **Agent instance caching** per session for prompt caching preservation
3. **Message batching** for rapid client-side splits
4. **DM pairing** for code-based user authorization
5. **Delivery router** for platform-appropriate formatting
6. **Session management** with background process protection
7. **Voice receiver** for Discord voice messages
8. **Event hook system** for extensibility
