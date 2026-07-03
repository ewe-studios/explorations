# Voice Pipeline: STT → Agent → TTS

Complete source-level documentation of Moltis voice capabilities: 11 STT providers, 6 TTS providers, auto-speak modes, TTS directives with SSML support, and the full voice pipeline from audio input to speech output.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/crates/voice/src/`

## Configuration

**Source:** `config.rs`

```rust
pub struct VoiceConfig {
    pub tts: TtsConfig,
    pub stt: SttConfig,
}

pub struct TtsConfig {
    pub enabled: bool,                        // Enable TTS globally
    pub provider: Option<TtsProviderId>,      // None = auto-select first configured
    pub auto: TtsAutoMode,                    // When to auto-speak
    pub max_text_length: usize,               // Max chars before skipping (default 2000)
    pub elevenlabs: ElevenLabsConfig,
    pub openai: OpenAiTtsConfig,
    pub google: GoogleTtsConfig,
    pub piper: PiperTtsConfig,
    pub coqui: CoquiTtsConfig,
}

pub enum TtsAutoMode {
    Always,    // Speak all responses
    Off,       // Never auto-speak (default)
    Inbound,   // Only when user sent voice input
    Tagged,    // Only with explicit [[tts]] markup
}
```

## STT Providers (11)

### TranscribeRequest / Transcript

```rust
// stt/mod.rs
pub struct TranscribeRequest {
    pub audio: Bytes,                    // Raw audio data
    pub format: AudioFormat,             // mp3, opus, wav, webm, etc.
    pub language: Option<String>,        // ISO 639-1 hint ("en", "es", "fr")
    pub prompt: Option<String>,          // Context/terminology guide
}

pub struct Transcript {
    pub text: String,                    // Transcribed text
    pub language: Option<String>,        // Detected language
    pub confidence: Option<f32>,         // 0.0 - 1.0
    pub duration_seconds: Option<f32>,   // Audio duration
    pub words: Option<Vec<Word>>,        // Word-level timestamps
}

pub struct Word {
    pub word: String,
    pub start: f32,                      // Start time (seconds)
    pub end: f32,                        // End time (seconds)
}
```

### SttProvider Trait

```rust
// stt/mod.rs:75
#[async_trait]
pub trait SttProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    async fn transcribe(&self, req: TranscribeRequest) -> Result<Transcript>;
}
```

### Provider Details

| Provider | File | API/Method | Default Model |
|----------|------|------------|---------------|
| **Whisper (API)** | `stt/whisper.rs` | `POST /v1/audio/transcriptions` | whisper-1 |
| **Whisper CLI** | `stt/whisper_cli.rs` | Spawns `whisper` binary | — |
| **Whisper Local** | `stt/whisper_local.rs` | Direct `whisper.cpp` FFI | — |
| **Deepgram** | `stt/deepgram.rs` | `POST /v1/listen` | nova-2 |
| **ElevenLabs** | `stt/elevenlabs.rs` | `POST /v1/speech-to-text` | v1 |
| **Google** | `stt/google.rs` | Google Cloud Speech v2 | latest_long |
| **Groq** | `stt/groq.rs` | OpenAI-compatible `/v1/audio` | whisper-large-v3 |
| **Mistral** | `stt/mistral.rs` | OpenAI-compatible | mistral-stt |
| **Sherpa-ONNX** | `stt/sherpa_onnx.rs` | ONNX runtime, offline | — |
| **Voxtral Local** | `stt/voxtral_local.rs` | Local Voxtral model | — |
| **OpenAI-compat** | `stt/openai_compat.rs` | Any OpenAI-compatible endpoint | — |

### Whisper CLI Implementation

```rust
// stt/whisper_cli.rs
pub struct WhisperCliStt {
    binary_path: PathBuf,              // Path to whisper binary
    model: String,
    language: Option<String>,
}

async fn transcribe(&self, req: TranscribeRequest) -> Result<Transcript> {
    // Write audio to temp file
    let temp_path = temp_dir().join(format!("whisper_{}.wav", uuid()));
    tokio::fs::write(&temp_path, &req.audio).await?;

    // Spawn whisper binary
    let output = Command::new(&self.binary_path)
        .arg(&temp_path)
        .arg("--model")
        .arg(&self.model)
        .arg("--output-format")
        .arg("json")
        .output()
        .await?;

    // Parse JSON output
    let result: WhisperOutput = serde_json::from_slice(&output.stdout)?;
    Ok(Transcript { text: result.text, ... })
}
```

## TTS Providers (6)

### SynthesizeRequest / AudioOutput

```rust
// tts/mod.rs
pub struct SynthesizeRequest {
    pub text: String,
    pub voice: Option<String>,           // Voice ID
    pub format: AudioFormat,             // mp3, opus, wav, etc.
    pub speed: Option<f32>,              // Playback speed (0.5 - 2.0)
    pub directives: Option<TtsDirectives>, // SSML, pauses, emphasis
}

pub struct AudioOutput {
    pub data: Bytes,                     // Raw audio data
    pub format: AudioFormat,
    pub duration_ms: Option<u64>,
}
```

### TtsProvider Trait

```rust
// tts/mod.rs
#[async_trait]
pub trait TtsProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    async fn list_voices(&self) -> Result<Vec<Voice>>;
    async fn synthesize(&self, req: SynthesizeRequest) -> Result<AudioOutput>;
}
```

### Provider Details

| Provider | File | API/Method | Default Voice | Default Model |
|----------|------|------------|---------------|---------------|
| **ElevenLabs** | `tts/elevenlabs.rs` | `POST /v1/text-to-speech/{id}` | Rachel | eleven_flash_v2_5 |
| **OpenAI** | `tts/openai.rs` | `POST /v1/audio/speech` | alloy | tts-1 |
| **Google** | `tts/google.rs` | Google Cloud TTS API | en-US-Neural2-F | — |
| **Coqui** | `tts/coqui.rs` | Local Coqui TTS | — | — |
| **Piper** | `tts/piper.rs` | Local Piper TTS | — | — |
| **Mock** | `tts/mock.rs` | Returns silence (testing) | — | — |

### ElevenLabs Implementation

```rust
// tts/elevenlabs.rs
pub struct ElevenLabsTts {
    client: Client,
    api_key: Option<Secret<String>>,
    default_voice_id: String,    // "21m00Tcm4TlvDq8ikWAM" (Rachel)
    default_model: String,       // "eleven_flash_v2_5" (~75ms latency)
}

// API: POST https://api.elevenlabs.io/v1/text-to-speech/{voice_id}
// Query params: output_format, model_id
// Body: { text, voice_settings }
async fn synthesize(&self, req: SynthesizeRequest) -> Result<AudioOutput> {
    let url = format!("{API_BASE}/text-to-speech/{}/audio", voice_id);
    let resp = self.client.post(&url)
        .header("xi-api-key", api_key)
        .json(&body)
        .send()
        .await?;
    let data = resp.bytes().await?;
    Ok(AudioOutput { data, format: req.format })
}
```

### OpenAI Implementation

```rust
// tts/openai.rs
pub struct OpenAiTts {
    client: Client,
    api_key: Option<Secret<String>>,
    base_url: String,              // Default: https://api.openai.com/v1
    default_voice: String,         // "alloy"
    default_model: String,         // "tts-1"
}

// Available voices: alloy, echo, fable, onyx, nova, shimmer
// Models: tts-1 (real-time), tts-1-hd (higher quality)
```

### AudioFormat

```rust
// tts/audio.rs
pub enum AudioFormat {
    Mp3,    // "audio/mpeg" — .mp3
    Opus,   // "audio/ogg"  — .ogg (Telegram voice notes)
    Aac,    // "audio/aac"  — .aac
    Pcm,    // "audio/pcm"  — .pcm (raw)
    Wav,    // "audio/wav"  — .wav
    Webm,   // "audio/webm" — .webm (browser MediaRecorder)
}
```

### TTS Directives and SSML

```rust
// tts/mod.rs
pub struct TtsDirectives {
    pub ssml: Option<String>,        // Raw SSML override
    pub pauses: Vec<PauseDirective>, // Insert pauses
    pub emphasis: Vec<EmphasisDirective>, // Emphasize text
}

pub fn contains_ssml(text: &str) -> bool;
pub fn strip_ssml_tags(text: &str) -> String;
pub fn parse_tts_directives(text: &str) -> TtsDirectives;
pub fn sanitize_text_for_tts(text: &str) -> String;
```

---

## WhatsApp: Linked Devices with QR Code Pairing

**Source:** `crates/whatsapp/src/`

### Full Connection Flow

1. **Plugin Creation**: `WhatsAppPlugin::new(data_dir)` creates empty account state
2. **Account Registration**: `register_account()` stores config
3. **Start Connection**: `start_account()` spawns bot with sled store
4. **QR Code Generation**: Bot emits `Event::PairingQrCode { code }` → stored in `state.latest_qr`
5. **User Scans QR**: WhatsApp pairs device
6. **Connected**: Bot emits `Event::Connected` → `state.connected = true`
7. **Auto-approve Owner**: `auto_approve_owner_jid()` adds owner's phone to allowlist
8. **Event Handling**: All messages go through `handlers::handle_event()`

### Event Handling

```rust
// handlers.rs:54+
pub async fn handle_event(
    event: Event,
    client: Arc<Client>,
    state: Arc<AccountState>,
    accounts: AccountStateMap,
) {
    match event {
        Event::PairingQrCode { code, .. } => {
            // Store QR code for REST endpoint
            *state.latest_qr.write().unwrap() = Some(code.clone());
            sink.emit(ChannelEvent::PairingQrCode { qr_data: code }).await;
        },
        Event::Connected(_) => {
            state.connected.store(true, Ordering::Relaxed);
            // Auto-approve owner's phone number
            let own_pn = state.client.get_pn().await;
            auto_approve_owner_jid(&accounts, &state.account_id, own_pn.as_ref());
        },
        Event::Message { info, message } => {
            handle_message(info, message, client, state, accounts).await;
        },
        Event::Disconnected(_) => {
            state.connected.store(false, Ordering::Relaxed);
        },
    }
}
```

### Message Handling

```rust
async fn handle_message(
    info: MessageInfo,
    message: Message,
    client: Arc<Client>,
    state: Arc<AccountState>,
    accounts: AccountStateMap,
) {
    // Check access control
    access::check(&state.config.access, &info.sender)?;

    // Handle OTP challenges
    if is_otp_challenge(&message) {
        handle_otp(&message, &accounts, &state.account_id).await;
        return;
    }

    // Build channel event
    let event = ChannelEvent::Message {
        channel_type: ChannelType::Whatsapp,
        account_id: state.account_id.clone(),
        sender: info.sender.to_string(),
        kind: message_to_kind(&message),
        meta: ChannelMessageMeta {
            message_id: info.id.clone(),
            thread_id: info.chat.to_string(),
            timestamp: info.timestamp,
        },
        reply_target: ChannelReplyTarget { ... },
        attachments: extract_attachments(&message),
    };

    // Emit to event sink
    state.event_sink.emit(event).await;
}
```

### Persistent Store

```rust
// sled_store.rs
pub struct SledStore {
    db: sled::Db,
}

impl SledStore {
    pub fn open(path: &Path) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }
}
```

Session state persists across restarts at `<data_dir>/whatsapp/<account_id>/`.

### Outbound Messaging

```rust
// outbound.rs
pub struct WhatsAppOutbound {
    accounts: AccountStateMap,
}

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

## Slack: Events API with HMAC-SHA256 Verification

**Source:** `crates/slack/src/`

### Webhook Signature Verification

```rust
// webhook.rs:93-120
type HmacSha256 = Hmac<Sha256>;

pub fn verify_signature(
    signing_secret: &str,
    timestamp: &str,
    body: &[u8],
    signature: &str,
) -> bool {
    let sig_hex = signature.strip_prefix("v0=").unwrap_or(signature);
    let sig_bytes = hex::decode(sig_hex).ok()?;

    let mac_str = format!("v0:{timestamp}:{}", String::from_utf8_lossy(body));
    let mut mac = HmacSha256::new_from_slice(signing_secret.as_bytes()).ok()?;
    mac.update(mac_str.as_bytes());

    mac.verify_slice(&sig_bytes).is_ok()
}
```

Slack sends:
- `X-Slack-Signature`: `v0=<hex-hmac>`
- `X-Slack-Request-Timestamp`: epoch seconds

HMAC base string: `v0:{timestamp}:{body}`

### URL Verification Challenge

```rust
// webhook.rs
pub async fn handle_webhook(
    account_id: &str,
    body: &[u8],
    timestamp: &str,
    signature: &str,
    accounts: &AccountStateMap,
) -> Result<Option<String>> {
    // 1. Verify signature
    let config = current_config(accounts, account_id, ...);
    if !verify_signature(&config.signing_secret, timestamp, body, signature) {
        return Err("invalid signature");
    }

    // 2. Parse event
    let event: SlackEvent = serde_json::from_slice(body)?;

    // 3. Handle URL verification challenge
    if let SlackEvent::UrlVerification(challenge) = event {
        return Ok(Some(challenge.challenge));
    }

    // 4. Dispatch message events
    if let SlackEvent::Message(msg) = event {
        dispatch_message(account_id, msg, accounts).await;
    }

    Ok(None)
}
```

### Socket Mode vs Events API

| Mode | Connection | Setup |
|------|-----------|-------|
| Socket Mode | WebSocket to Slack | App token + bot token |
| Events API | HTTP webhook | Signing secret + bot token |

---

## Discord: Serenity Gateway with 6 Intents

**Source:** `crates/discord/src/`

### Required Gateway Intents

```rust
// handler/implementation.rs:41-48
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
        // Check access control
        if !access::check(&config.access, &msg.author.id).await {
            return;
        }

        // Build channel event
        let event = ChannelEvent::Message {
            channel_type: ChannelType::Discord,
            account_id: self.account_id.clone(),
            sender: msg.author.id.to_string(),
            kind: message_to_kind(&msg),
            meta: ChannelMessageMeta {
                message_id: msg.id.to_string(),
                thread_id: msg.channel_id.to_string(),
                timestamp: msg.timestamp.timestamp_millis(),
            },
            attachments: extract_attachments(&msg, &self.downloader).await,
        };

        // Emit to event sink
        if let Some(sink) = &state.event_sink {
            sink.emit(event).await;
        }
    }
}
```

### Bot Configuration

```rust
// config.rs
pub struct DiscordAccountConfig {
    pub bot_token: Secret<String>,
    pub activity_type: ActivityType,     // Playing, Watching, Listening
    pub activity_text: Option<String>,
    pub online_status: OnlineStatus,     // Online, Idle, DND, Invisible
    pub access: AccessConfig,
}
```

---

## Next

Continue to [05-agents-deep](05-agents-deep.md) for the agent loop implementation.
