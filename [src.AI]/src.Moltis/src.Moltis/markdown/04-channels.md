# Channels: Message Routing and Plugin System

Moltis connects to multiple communication channels through a plugin system. Each channel implements the `ChannelPlugin` trait with sub-traits for config, auth, inbound/outbound messaging, and gateway lifecycle.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/crates/channels/src/`

## Module Structure

```
crates/channels/src/
├── lib.rs              ← Re-exports all public API
├── plugin.rs           ← ChannelPlugin trait + sub-traits
├── registry.rs         ← ChannelRegistry, RegistryOutboundRouter
├── store.rs            ← Channel storage
├── config_view.rs      ← ChannelConfigView
├── contract.rs         ← Channel contracts
├── error.rs            ← Error types
├── gating.rs           ← Message gating
├── media_download.rs   ← InboundMediaDownloader, InboundMediaSource
├── message_log.rs      ← Message logging
├── otp.rs              ← OTP handling
├── commands/           ← Channel commands
└── channel_webhook_middleware.rs ← Webhook dedupe, rate limiting, verification
```

## ChannelPlugin Trait

**Source:** `plugin.rs`

The core trait that all channels must implement:

```rust
// Re-exports from plugin.rs:28-36
pub use plugin::{
    ButtonRow, ButtonStyle, ChannelAttachment, ChannelCapabilities, ChannelDescriptor,
    ChannelDocumentFile, ChannelEvent, ChannelEventSink, ChannelHealthSnapshot,
    ChannelMessageKind, ChannelMessageMeta, ChannelOtpProvider, ChannelOutbound, ChannelPlugin,
    ChannelReplyTarget, ChannelStatus, ChannelStreamOutbound, ChannelThreadContext,
    ChannelType, InboundMode, InteractiveButton, InteractiveMessage, SavedChannelFile,
    StreamEvent, StreamReceiver, StreamSender, ThreadMessage, resolve_session_channel_binding,
    web_session_channel_binding,
};
```

### ChannelType

```rust
pub enum ChannelType {
    Telegram,
    Discord,
    Slack,
    WhatsApp,
    Signal,
    Matrix,
    MSTeams,
    Nostr,
    CalDav,
}
```

### ChannelMessageKind

```rust
pub enum ChannelMessageKind {
    Text,
    Image,
    Document,
    Voice,
    Video,
    Location,
    Contact,
    Sticker,
}
```

## ChannelRegistry

**Source:** `registry.rs`

```rust
pub struct ChannelRegistry {
    // Registered channel plugins
}

pub struct RegistryOutboundRouter {
    // Routes outbound messages to the correct channel
}
```

## ChannelWebhookMiddleware

**Source:** `channel_webhook_middleware.rs`

```rust
pub enum ChannelWebhookRatePolicy { ... }
pub enum ChannelWebhookRejection { ... }
pub struct ChannelWebhookDedupeResult { ... }
pub struct TimestampGuard { ... }
pub struct VerifiedChannelWebhook { ... }

pub trait ChannelWebhookVerifier: Send + Sync {
    async fn verify(&self, webhook: ...) -> Result<VerifiedChannelWebhook>;
}
```

Handles:
- **Deduplication** — Prevents duplicate webhook processing
- **Rate limiting** — Enforces per-channel rate limits
- **Verification** — Validates webhook signatures

## Media Download

**Source:** `media_download.rs`

```rust
pub trait InboundMediaSource: Send + Sync { ... }
pub struct InboundMediaDownloader { ... }
```

Downloads media from inbound channel messages.

## Message Logging

**Source:** `message_log.rs`

Logs all channel messages for audit and debugging.

## OTP Handling

**Source:** `otp.rs`

```rust
pub trait ChannelOtpProvider: Send + Sync { ... }
```

One-time password handling for channel authentication.

## Gating

**Source:** `gating.rs`

Message gating controls which messages are processed.

## Commands

**Source:** `commands/`

Channel-specific command handling.

## Next

Continue to [06-providers-deep](06-providers-deep.md) for LLM provider implementations.
