# Channels: How Messages Flow In and Out

This document explains how Moltis connects to WhatsApp, Telegram, Slack, Discord, Signal, Matrix, MS Teams, Nostr, and CalDAV — the ChannelPlugin trait, inbound modes, outbound delivery, webhook verification, and the event sink that connects channels to the agent loop.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/crates/channels/src/`

---

## Channel Event Flow

```
User sends message on WhatsApp
    ↓
WhatsApp Linked Devices → Event:PairingQrCode → Event:Connected → Event:Message
    ↓
handlers.rs:handle_event()
    ↓
ChannelEvent::Message { channel_type, account_id, sender, kind, meta, attachments }
    ↓
event_sink.emit(event)
    ↓
Gateway channel handler → LiveChatService::send()
    ↓
run_with_tools() → agent loop → LLM response
    ↓
Assistant response text
    ↓
deliver_channel_replies(session_key, final_text)
    ↓
WhatsAppOutbound.send_message(account_id, target, text)
    ↓
WhatsApp client.send_text_message(target, text)
    ↓
User sees response on WhatsApp
```

---

## The ChannelPlugin Trait

**Source:** `plugin.rs`

Every channel implements this trait:

```rust
#[async_trait]
pub trait ChannelPlugin: Send + Sync {
    fn channel_type(&self) -> ChannelType;

    fn register_account(&self, account_id: String, config: serde_json::Value)
        -> ChannelResult<()>;

    fn start_account(&self, account_id: &str, config: serde_json::Value)
        -> ChannelResult<()>;

    fn stop_account(&self, account_id: &str) -> ChannelResult<()>;

    fn outbound(&self) -> Arc<dyn ChannelOutbound>;

    fn status(&self, account_id: &str) -> ChannelResult<ChannelStatus>;

    fn probe(&self, account_id: &str) -> ChannelResult<ChannelHealthSnapshot>;

    fn supports_otp(&self) -> bool;

    fn supports_pairing(&self) -> bool;

    fn otp_provider(&self) -> Option<Arc<dyn ChannelOtpProvider>>;
}
```

### ChannelOutbound

```rust
#[async_trait]
pub trait ChannelOutbound: Send + Sync {
    async fn send_message(&self, account_id: &str, target: &str, text: &str) -> ChannelResult<()>;

    async fn send_media(&self, account_id: &str, target: &str, media: ChannelAttachment)
        -> ChannelResult<()>;

    async fn send_location(&self, account_id: &str, target: &str, lat: f64, lon: f64)
        -> ChannelResult<()>;

    async fn send_document(&self, account_id: &str, target: &str, doc: ChannelDocumentFile)
        -> ChannelResult<()>;
}
```

### ChannelStreamOutbound (for streaming responses)

```rust
#[async_trait]
pub trait ChannelStreamOutbound: Send + Sync {
    async fn start_stream(&self, account_id: &str, target: &str, stream_id: &str)
        -> ChannelResult<()>;

    async fn send_stream_chunk(&self, account_id: &str, target: &str, stream_id: &str, text: &str)
        -> ChannelResult<()>;

    async fn end_stream(&self, account_id: &str, target: &str, stream_id: &str)
        -> ChannelResult<()>;
}
```

---

## Channel Types and Capabilities

**Source:** `plugin.rs:14-293`

### ChannelType Enum

```rust
pub enum ChannelType {
    Telegram, Whatsapp, MsTeams, Discord, Slack, Matrix, Nostr, Signal, Telephony,
}
```

### Inbound Modes

Each channel receives messages differently:

| Channel | Inbound Mode | How it works |
|---------|-------------|--------------|
| **Telegram** | Polling | Long-polling Telegram Bot API (`getUpdates`) |
| **WhatsApp** | GatewayLoop | WebSocket loop via `whatsapp_rust` Linked Devices |
| **Discord** | GatewayLoop | WebSocket via `serenity` gateway |
| **Signal** | GatewayLoop | Signal CLI event loop |
| **Matrix** | GatewayLoop | Matrix sync API loop |
| **Nostr** | GatewayLoop | Nostr relay subscription loop |
| **Slack** | SocketMode | WebSocket via Slack Socket Mode |
| **MS Teams** | Webhook | HTTP POST from Teams Bot Framework |
| **Telephony** | Webhook | HTTP POST from telephony provider |

### Channel Capabilities

| Channel | Streaming | Threads | Voice | OTP | Pairing | Location | Reactions |
|---------|-----------|---------|-------|-----|---------|----------|-----------|
| Telegram | ✓ | ✗ | ✓ | ✓ | ✗ | ✓ | ✗ |
| WhatsApp | ✓ | ✗ | ✓ | ✓ | ✓ | ✗ | ✗ |
| MS Teams | ✓ | ✓ | ✗ | ✗ | ✗ | ✓ | ✓ |
| Discord | ✓ | ✓ | ✓ | ✗ | ✗ | ✓ | ✗ |
| Slack | ✓ | ✓ | ✗ | ✗ | ✗ | ✗ | ✓ |
| Matrix | ✓ | ✓ | ✓ | ✓ | ✗ | ✓ | ✓ |
| Nostr | ✗ | ✗ | ✗ | ✓ | ✗ | ✗ | ✗ |
| Signal | ✗ | ✗ | ✗ | ✓ | ✗ | ✗ | ✗ |
| Telephony | ✗ | ✗ | ✓ | ✗ | ✗ | ✗ | ✗ |

### Chat Classification

Channels classify incoming chats for context:

```rust
// plugin.rs:63-85
pub fn classify_chat(&self, chat_id: &str) -> Option<String> {
    match self {
        Self::Telegram => {
            if chat_id.starts_with("-100") => Some("channel_or_supergroup")
            else if chat_id.starts_with('-') => Some("group")
            else => Some("private")
        },
        Self::Signal => {
            if chat_id.starts_with("group:") => Some("group")
            else => Some("direct")
        },
        Self::Nostr => Some("dm"),
        Self::Telephony => Some("call"),
        _ => None,
    }
}
```

### Secret Fields per Channel

```rust
// plugin.rs:88-99
pub fn secret_fields(&self) -> &'static [&'static str] {
    match self {
        Self::Telegram => &["token"],
        Self::MsTeams => &["app_password", "webhook_secret"],
        Self::Discord => &["token"],
        Self::Slack => &["bot_token", "app_token", "signing_secret"],
        Self::Matrix => &["access_token", "password"],
        Self::Nostr => &["secret_key"],
        Self::Telephony => &["auth_token"],
        Self::Whatsapp | Self::Signal => &[],  // secrets stored via QR/pairing
    }
}
```

---

## WhatsApp: Linked Devices Flow

**Source:** `crates/whatsapp/src/`

### Connection Setup

```rust
// connection.rs
pub async fn start_connection(
    account_id: String,
    config: WhatsAppAccountConfig,
    accounts: AccountStateMap,
    data_dir: PathBuf,
    message_log: Option<Arc<dyn MessageLog>>,
    event_sink: Option<Arc<dyn ChannelEventSink>>,
) -> Result<()> {
    // 1. Open persistent sled store at <data_dir>/whatsapp/<account_id>/
    let store_path = config.store_path.unwrap_or(data_dir.join("whatsapp").join(&account_id));
    let backend = Arc::new(SledStore::open(&store_path)?);

    // 2. Build WhatsApp bot
    let mut bot = whatsapp_rust::bot::Bot::builder()
        .with_backend(backend)
        .with_transport_factory(TokioWebSocketTransportFactory::new())
        .with_http_client(UreqHttpClient::new())
        .with_runtime(whatsapp_rust::TokioRuntime)
        .skip_history_sync()
        .with_device_props(Some("Moltis".to_string()), None, Some(PlatformType::Desktop))
        .with_push_name("Moltis")
        .on_event(move |event, client| {
            async move {
                if let Some(state) = state_ref.get() {
                    handlers::handle_event(event, client, state, accounts).await;
                }
            }
        })
        .build()
        .await?;
}
```

**Key detail:** `skip_history_sync()` means the bot doesn't download full message history on connect — only new messages.

### Event Handling

```rust
// handlers.rs
pub async fn handle_event(event: Event, client: Arc<Client>, state: Arc<AccountState>, accounts: AccountStateMap) {
    match event {
        Event::PairingQrCode { code, .. } => {
            // Store QR code for REST endpoint
            *state.latest_qr.write().unwrap() = Some(code);
            sink.emit(ChannelEvent::PairingQrCode { qr_data: code }).await;
        },
        Event::Connected(_) => {
            state.connected.store(true);
            // Auto-approve owner's phone number
            let own_pn = client.get_pn().await;
            auto_approve_owner_jid(&accounts, &state.account_id, own_pn.as_ref());
        },
        Event::Message { info, message } => {
            handle_message(info, message, client, state, accounts).await;
        },
        Event::Disconnected(_) => {
            state.connected.store(false);
        },
    }
}
```

### Message Handling

```rust
async fn handle_message(info: MessageInfo, message: Message, client: Arc<Client>, state: Arc<AccountState>, accounts: AccountStateMap) {
    // 1. Check access control (allowlist)
    access::check(&state.config.access, &info.sender)?;

    // 2. Handle OTP challenges
    if is_otp_challenge(&message) { handle_otp(&message, &accounts, &state.account_id).await; return; }

    // 3. Build channel event
    let event = ChannelEvent::Message {
        channel_type: ChannelType::Whatsapp,
        account_id: state.account_id.clone(),
        sender: info.sender.to_string(),
        kind: message_to_kind(&message),  // Text, Image, Voice, Video, Document, Location
        meta: ChannelMessageMeta { message_id: info.id, thread_id: info.chat.to_string(), timestamp: info.timestamp },
        reply_target: ChannelReplyTarget { ... },
        attachments: extract_attachments(&message),
    };

    // 4. Emit to event sink → gateway → agent loop
    state.event_sink.emit(event).await;
}
```

### Outbound Messaging

```rust
// outbound.rs
pub struct WhatsAppOutbound { accounts: AccountStateMap }

impl ChannelOutbound for WhatsAppOutbound {
    async fn send_message(&self, account_id: &str, target: &str, text: &str) -> Result<()> {
        let accounts = self.accounts.read()?;
        let state = accounts.get(account_id).ok_or(...)?;
        state.client.send_text_message(target, text).await?;
        Ok(())
    }
}
```

---

## Slack: HMAC-SHA256 Webhook Verification

**Source:** `crates/slack/src/`

### Socket Mode Registration

```rust
// socket.rs
pub async fn register_socket_mode_account(
    account_id: &str,
    config: SlackAccountConfig,
    accounts: AccountStateMap,
    message_log: Option<Arc<dyn MessageLog>>,
    event_sink: Option<Arc<dyn ChannelEventSink>>,
) -> Result<()> {
    // 1. Authenticate bot token via auth.test
    let client = SlackClient::new(SlackClientHyperConnector::new()?);
    let bot_token = SlackApiToken::new(config.bot_token.expose_secret().clone());
    let session = client.open_session(&bot_token);
    let auth_response = session.auth_test().await?;
    let bot_user_id = auth_response.user_id.to_string();

    // 2. Register account state
    accounts.insert(account_id, AccountState { bot_user_id: Some(bot_user_id), ... });
    Ok(())
}
```

### Events API Webhook

```rust
// webhook.rs
type HmacSha256 = Hmac<Sha256>;

pub fn verify_signature(signing_secret: &str, timestamp: &str, body: &[u8], signature: &str) -> bool {
    let sig_hex = signature.strip_prefix("v0=").unwrap_or(signature);
    let sig_bytes = hex::decode(sig_hex).ok()?;

    let mac_str = format!("v0:{timestamp}:{}", String::from_utf8_lossy(body));
    let mut mac = HmacSha256::new_from_slice(signing_secret.as_bytes()).ok()?;
    mac.update(mac_str.as_bytes());

    mac.verify_slice(&sig_bytes).is_ok()
}
```

Slack sends `X-Slack-Signature: v0=<hex-hmac>` and `X-Slack-Request-Timestamp`. The HMAC base string is `v0:{timestamp}:{body}`.

---

## Discord: Serenity Gateway

**Source:** `crates/discord/src/`

### Required Gateway Intents

```rust
// handler/implementation.rs
pub fn required_intents() -> GatewayIntents {
    GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::DIRECT_MESSAGE_REACTIONS
}
```

### Event Handler

```rust
// handler/implementation.rs
pub struct Handler {
    pub account_id: String,
    pub accounts: AccountStateMap,
    downloader: DiscordInboundMediaDownloader,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        // 1. Check access control
        if !access::check(&config.access, &msg.author.id).await { return; }

        // 2. Build channel event
        let event = ChannelEvent::Message {
            channel_type: ChannelType::Discord,
            account_id: self.account_id.clone(),
            sender: msg.author.id.to_string(),
            kind: message_to_kind(&msg),
            attachments: extract_attachments(&msg, &self.downloader).await,
        };

        // 3. Emit to event sink
        if let Some(sink) = &state.event_sink { sink.emit(event).await; }
    }
}
```

Uses the `serenity` crate for Discord gateway connections.

---

## Channel Registry

**Source:** `registry.rs`

```rust
pub struct ChannelRegistry {
    plugins: HashMap<ChannelType, Box<dyn ChannelPlugin>>,
}

impl ChannelRegistry {
    pub fn register(&mut self, plugin: Box<dyn ChannelPlugin>) {
        self.plugins.insert(plugin.channel_type(), plugin);
    }

    pub fn get(&self, channel_type: &ChannelType) -> Option<&dyn ChannelPlugin> {
        self.plugins.get(channel_type).map(|p| p.as_ref())
    }
}

pub struct RegistryOutboundRouter {
    registry: Arc<ChannelRegistry>,
}

impl ChannelOutbound for RegistryOutboundRouter {
    async fn send_message(&self, account_id: &str, target: &str, text: &str) -> Result<()> {
        // Route to correct channel's outbound
    }
}
```

---

## Webhook Middleware

**Source:** `channel_webhook_middleware.rs`

### Deduplication

```rust
pub struct ChannelWebhookDedupe {
    window: Duration,  // e.g., 5 seconds
    seen: HashMap<String, Instant>,  // event_id → timestamp
}
```

### Rate Limiting

```rust
pub enum ChannelWebhookRatePolicy {
    Allow,
    Throttle(Duration),
    Reject,
}
```

### Verification

```rust
pub trait ChannelWebhookVerifier: Send + Sync {
    async fn verify(&self, webhook: ...) -> Result<VerifiedChannelWebhook>;
}
```

---

## Next

Continue to [05-agents-deep](05-agents-deep.md) for the agent loop implementation.
