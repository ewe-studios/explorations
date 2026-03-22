---
name: Robrix
description: Production-ready Matrix protocol chat client built with Makepad, demonstrating real-time messaging, room management, and reactive UI patterns
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/robrix/
---

# Robrix - Matrix Chat Client

## Overview

Robrix is a cross-platform Matrix protocol chat client built with Makepad. It demonstrates production-ready patterns for real-time messaging, including sliding sync, timeline management, encryption, and reactive UI updates. Robrix is part of Project Robius and serves as a reference implementation for building communication apps with Makepad.

## Repository Structure

```
robrix/
├── src/
│   ├── main.rs                   # Application entry point
│   ├── app.rs                    # Main application structure
│   ├── room_list_service.rs      # Room list management
│   ├── room_preview.rs           # Room preview component
│   ├── timeline/
│   │   ├── mod.rs                # Timeline module
│   │   ├── timeline_item.rs      # Timeline item types
│   │   ├── message.rs            # Message rendering
│   │   ├── event_item.rs         # Event handling
│   │   ├── virtual_item.rs       # Virtual list items
│   │   └── reactions.rs          # Message reactions
│   ├── authentication/
│   │   ├── mod.rs                # Auth module
│   │   ├── login.rs              # Login UI
│   │   ├── registration.rs       # Registration UI
│   │   └── sso.rs                # SSO handling
│   ├── profile/
│   │   ├── mod.rs                # Profile module
│   │   ├── user_profile.rs       # User profile display
│   │   └── avatar.rs             # Avatar rendering
│   ├── room_details/
│   │   ├── mod.rs                # Room details module
│   │   ├── members.rs            # Member list
│   │   └── settings.rs           # Room settings
│   ├── welcome_screen.rs         # Welcome/login screen
│   ├── loading_screen.rs         # Loading indicator
│   └── shared/
│       ├── avatar.rs             # Shared avatar component
│       ├── scrollbar.rs          # Custom scrollbar
│       └── text_input.rs         # Enhanced text input
│
├── Cargo.toml
├── build.rs
└── resources/
    └── images/
        └── icons/
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Robrix UI Layer (Makepad)                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │  Welcome     │  │  Room List   │  │  Timeline View       │  │
│  │  Screen      │  │  Panel       │  │  (Messages)          │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │  Login       │  │  Room        │  │  Message Input       │  │
│  │  Form        │  │  Preview     │  │  Composer            │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ Reactive Updates (eyeball)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                  State Management Layer                          │
│  ┌────────────────────┐  ┌────────────────────┐                │
│  │  RoomListService   │  │  TimelineService   │                │
│  │  (Observable)      │  │  (Observable)      │                │
│  └────────────────────┘  └────────────────────┘                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ Matrix SDK API
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│              matrix-rust-sdk Layer                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │  Client      │  │  Room        │  │  Encryption          │  │
│  │              │  │  List        │  │  (Olm/Megolm)        │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │  Sliding     │  │  Timeline    │  │  Authentication      │  │
│  │  Sync        │  │  Builder     │  │  (SSO, Password)     │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ HTTP + TLS
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Matrix Homeserver                              │
│  (Synapse / Dendrite / Conduit / Any Matrix Server)             │
└─────────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. Room List Service

```rust
// src/room_list_service.rs
use eyeball::SharedObservable;
use eyeball_im::ObservableVector;
use matrix_sdk::{
    room_list::{RoomList, RoomListService as SdkService, State},
    Client, Room, RoomState,
};
use futures_util::{StreamExt, pin_mut};

pub struct RoomListService {
    client: Client,
    rooms: SharedObservable<ObservableVector<RoomListItem>>,
    state: SharedObservable<RoomListState>,
}

#[derive(Clone, Debug)]
pub enum RoomListState {
    Initial,
    SettingUp,
    Recovering,
    Running,
    Error,
    Terminated,
}

#[derive(Clone, Debug)]
pub struct RoomListItem {
    pub id: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub latest_message: Option<String>,
    pub unread_count: u64,
    pub is_dm: bool,
    pub inviter: Option<Inviter>,
}

impl RoomListService {
    pub fn new(client: Client) -> Self {
        Self {
            client: client.clone(),
            rooms: SharedObservable::new(ObservableVector::new()),
            state: SharedObservable::new(RoomListState::Initial),
        }
    }

    /// Subscribe to room list changes
    pub fn subscribe_rooms(&self) -> impl Stream<Item = VectorDiff<RoomListItem>> {
        let vector = self.rooms.get();
        let subscriber = vector.subscribe();
        subscriber
    }

    /// Subscribe to state changes
    pub fn subscribe_state(&self) -> impl Stream<Item = RoomListState> {
        let subscriber = self.state.subscribe();
        subscriber.map(|g| (*g).clone())
    }

    /// Start the room list sync
    pub async fn sync(&self) -> matrix_sdk::Result<()> {
        self.state.set(RoomListState::SettingUp);

        let room_list = self.client.room_list().await?;
        let (room_list_room_list, room_list_stream) = room_list.all_rooms().await?;

        // Pin the stream
        pin_mut!(room_list_stream);

        // Set up the sync loop
        while let Some(diff) = room_list_stream.next().await {
            match diff {
                VectorDiff::Append { values } => {
                    for room in values {
                        self.add_room(room).await;
                    }
                }
                VectorDiff::Insert { index, value } => {
                    self.insert_room(index, value).await;
                }
                VectorDiff::Remove { index } => {
                    self.remove_room(index).await;
                }
                VectorDiff::Update { index, value } => {
                    self.update_room(index, value).await;
                }
                VectorDiff::Clear => {
                    self.rooms.get().clear();
                }
                _ => {}
            }
        }

        self.state.set(RoomListState::Running);
        Ok(())
    }

    async fn add_room(&self, room: Room) {
        let item = RoomListItem::from_room(&room).await;
        self.rooms.get().push_back(item);
    }

    async fn insert_room(&self, index: usize, room: Room) {
        let item = RoomListItem::from_room(&room).await;
        self.rooms.get().insert(index, item);
    }

    async fn update_room(&self, index: usize, room: Room) {
        let item = RoomListItem::from_room(&room).await;
        self.rooms.get().set(index, item);
    }

    async fn remove_room(&self, index: usize) {
        self.rooms.get().remove(index);
    }
}

impl RoomListItem {
    pub async fn from_room(room: &Room) -> Self {
        let latest_message = room.latest_event().await.map(|e| {
            e.as_original()
                .and_then(|e| e.content.body().map(|b| b.to_string()))
                .unwrap_or_default()
        });

        Self {
            id: room.room_id().to_string(),
            name: room.name(),
            avatar_url: room.avatar_url().map(|u| u.to_string()),
            latest_message,
            unread_count: room.num_unread_messages(),
            is_dm: room.is_direct().await.unwrap_or(false),
            inviter: room.invite_details().await.ok().and_then(|d| d.inviter),
        }
    }
}
```

### 2. Timeline Service

```rust
// src/timeline/mod.rs
use eyeball_im::ObservableVector;
use eyeball::SharedObservable;
use matrix_sdk::room::Room;
use matrix_sdk_base::deserialized_responses::SyncTimelineEvent;

pub struct TimelineService {
    room: Room,
    items: SharedObservable<ObservableVector<TimelineItem>>,
    has_more: SharedObservable<bool>,
}

#[derive(Clone, Debug)]
pub enum TimelineItem {
    Event(EventTimelineItem),
    Virtual(VirtualTimelineItem),
}

#[derive(Clone, Debug)]
pub struct EventTimelineItem {
    pub event_id: OwnedEventId,
    pub sender: OwnedUserId,
    pub timestamp: MilliSecondsSinceUnixEpoch,
    pub content: TimelineItemContent,
    pub kind: EventTimelineItemKind,
}

#[derive(Clone, Debug)]
pub enum TimelineItemContent {
    Message(Message),
    RedactedMessage,
    State(StateEvent),
    MembershipChange(MembershipChange),
    ProfileChange(ProfileChange),
    Other(OtherState),
}

#[derive(Clone, Debug)]
pub struct Message {
    pub msgtype: MessageType,
    pub body: String,
    pub in_reply_to: Option<InReplyToDetails>,
    pub edited: bool,
    pub reactions: Vec<Reaction>,
}

#[derive(Clone, Debug)]
pub enum MessageType {
    Text(TextMessageContent),
    Image(ImageMessageContent),
    Video(VideoMessageContent),
    Audio(AudioMessageContent),
    File(FileMessageContent),
    Emote(EmoteMessageContent),
}

#[derive(Clone, Debug)]
pub struct TextMessageContent {
    pub body: String,
    pub formatted: Option<FormattedBody>,
}

#[derive(Clone, Debug)]
pub struct FormattedBody {
    pub body: String,
    pub format: MessageFormat,
}

#[derive(Clone, Debug)]
pub enum MessageFormat {
    Html(String),
    Markdown(String),
}

#[derive(Clone, Debug)]
pub enum VirtualTimelineItem {
    DateSeparator(String),
    ReadMarker,
}

impl TimelineService {
    pub fn new(room: Room) -> Self {
        Self {
            room,
            items: SharedObservable::new(ObservableVector::new()),
            has_more: SharedObservable::new(true),
        }
    }

    /// Subscribe to timeline changes
    pub fn subscribe(&self) -> impl Stream<Item = VectorDiff<TimelineItem>> {
        let vector = self.items.get();
        let subscriber = vector.subscribe();
        subscriber
    }

    /// Load more timeline messages (pagination)
    pub async fn paginate_backwards(&self, num_events: u16) -> matrix_sdk::Result<bool> {
        if !*self.has_more.get() {
            return Ok(false);
        }

        let timeline = self.room.timeline().await;
        let (items, mut subscriber) = timeline.subscribe().await;

        // Paginate backwards
        match timeline.paginate_backwards(num_events).await {
            Ok(()) => {
                // Update items
                self.update_from_timeline(&items).await;
                Ok(true)
            }
            Err(matrix_sdk::Error::NoMoreEvents) => {
                self.has_more.set(false);
                Ok(false)
            }
            Err(e) => Err(e),
        }
    }

    /// Send a message
    pub async fn send_message(&self, content: RoomMessageEventContent) -> matrix_sdk::Result<()> {
        let timeline = self.room.timeline().await;
        timeline.send(content.into()).await;
        Ok(())
    }

    /// Toggle reaction to a message
    pub async fn toggle_reaction(
        &self,
        event_id: &EventId,
        key: &str,
    ) -> matrix_sdk::Result<()> {
        let timeline = self.room.timeline().await;
        timeline.toggle_reaction(event_id, key).await?;
        Ok(())
    }

    async fn update_from_timeline(&self, items: &Vector<Arc<TimelineItem>>) {
        let obs_vector = self.items.get();
        obs_vector.clear();

        for item in items {
            match item.as_ref() {
                matrix_sdk::timeline::TimelineItem::Event(e) => {
                    obs_vector.push_back(TimelineItem::Event(
                        EventTimelineItem::from_sdk(e)
                    ));
                }
                matrix_sdk::timeline::TimelineItem::Virtual(v) => {
                    obs_vector.push_back(TimelineItem::Virtual(
                        VirtualTimelineItem::from_sdk(v)
                    ));
                }
            }
        }
    }
}
```

### 3. Message Rendering

```rust
// src/timeline/message.rs
use makepad_widgets::*;

// Message rendering component
live_design! {
    MessageBubble = {{MessageBubble}} {
        container = {
            flow: Down,
            padding: { left: 10, right: 10, top: 5, bottom: 5 }
            spacing: 5

            sender_label = {
                text: ""
                draw_text: {
                    color: #0066cc
                    font_size: 12.0
                }
            }

            timestamp_label = {
                text: ""
                draw_text: {
                    color: #888888
                    font_size: 10.0
                }
            }

            content_label = {
                text: ""
                draw_text: {
                    color: #000000
                    font_size: 14.0
                }
            }

            reactions_view = {
                flow: Right,
                spacing: 5
            }
        }
    }
}

struct MessageBubble {
    message: Message,
    container: WidgetRef,
    sender_label: WidgetRef,
    timestamp_label: WidgetRef,
    content_label: WidgetRef,
    reactions_view: WidgetRef,
}

impl MessageBubble {
    fn set_message(&mut self, message: &Message) {
        self.message = message.clone();

        self.sender_label.set_text(&format!("@{}", message.sender));

        let time = chrono::DateTime::from_timestamp(
            (message.timestamp.0 / 1000) as i64,
            0
        ).unwrap();
        self.timestamp_label.set_text(
            &time.format("%H:%M").to_string()
        );

        match &message.msgtype {
            MessageType::Text(text) => {
                self.content_label.set_text(&text.body);
            }
            MessageType::Image(image) => {
                self.content_label.set_text(&format!("[Image: {}]", image.body));
            }
            _ => {
                self.content_label.set_text(&message.body);
            }
        }

        // Render reactions
        self.reactions_view.set_text("");
        for reaction in &message.reactions {
            // Create reaction widget
        }
    }

    fn render(&mut self, cx: &mut Cx2d, walk: Walk) -> DrawStep {
        self.container.draw_all(cx, walk)
    }
}

// HTML to Makepad text conversion
fn html_to_text(html: &str) -> String {
    // Simple HTML stripping
    html.replace("<br>", "\n")
        .replace("<p>", "")
        .replace("</p>", "\n")
        .replace("<strong>", "**")
        .replace("</strong>", "**")
        .replace("<em>", "*")
        .replace("</em>", "*")
}

// Markdown rendering (if supported)
fn render_markdown(md: &str) -> RichText {
    // Use a markdown parser
    // Convert to Makepad text spans
}
```

### 4. Authentication

```rust
// src/authentication/login.rs
use makepad_widgets::*;
use matrix_sdk::{
    Client,
    config::SyncSettings,
    ruma::api::client::account::register::v3::Request as RegistrationRequest,
};

live_design! {
    LoginView = {{LoginView}} {
        container = {
            flow: Down,
            padding: 20,
            spacing: 15,

            homeserver_input = {
                text: "matrix.org"
                placeholder: "Homeserver"
            }

            username_input = {
                placeholder: "Username"
            }

            password_input = {
                placeholder: "Password"
                is_password: true
            }

            login_button = {
                text: "Login"
            }

            register_button = {
                text: "Register"
            }

            sso_button = {
                text: "Login with SSO"
            }

            error_label = {
                text: ""
                draw_text: {
                    color: #ff0000
                }
            }
        }
    }
}

struct LoginView {
    container: WidgetRef,
    homeserver_input: WidgetRef,
    username_input: WidgetRef,
    password_input: WidgetRef,
    login_button: WidgetRef,
    register_button: WidgetRef,
    sso_button: WidgetRef,
    error_label: WidgetRef,
}

impl LoginView {
    async fn login(&self) -> Result<Client, String> {
        let homeserver = self.homeserver_input.text();
        let username = self.username_input.text();
        let password = self.password_input.text();

        let client = Client::builder()
            .homeserver_url(format!("https://{}", homeserver))
            .build()
            .await
            .map_err(|e| e.to_string())?;

        let response = client
            .login_username(&username, &password)
            .initial_device_display_name("Robrix")
            .send()
            .await
            .map_err(|e| e.to_string())?;

        Ok(client)
    }

    async fn register(&self) -> Result<Client, String> {
        let homeserver = self.homeserver_input.text();
        let username = self.username_input.text();
        let password = self.password_input.text();

        let client = Client::builder()
            .homeserver_url(format!("https://{}", homeserver))
            .build()
            .await
            .map_err(|e| e.to_string())?;

        let request = RegistrationRequest::new();
        // ... set registration parameters

        client.register(request).await
            .map_err(|e| e.to_string())?;

        // Auto-login after registration
        self.login().await
    }

    async fn sso_login(&self) -> Result<Client, String> {
        let homeserver = self.homeserver_input.text();

        let client = Client::builder()
            .homeserver_url(format!("https://{}", homeserver))
            .build()
            .await
            .map_err(|e| e.to_string())?;

        // Get SSO URL
        let (url, login_token) = client
            .matrix_auth()
            .get_sso_login_url("Robrix")
            .map_err(|e| e.to_string())?;

        // Open browser
        webbrowser::open(&url).map_err(|e| e.to_string())?;

        // Wait for callback and complete login
        // ...

        Ok(client)
    }
}
```

## Performance Considerations

### Lazy Loading

```rust
// Only load visible timeline items
pub struct LazyTimeline {
    viewport_start: usize,
    viewport_end: usize,
    cached_items: ObservableVector<TimelineItem>,
}

impl LazyTimeline {
    pub fn set_viewport(&mut self, start: usize, end: usize) {
        self.viewport_start = start;
        self.viewport_end = end;

        // Load only items in viewport
        self.load_items(start.saturating_sub(10)..end + 10);
    }

    fn load_items(&mut self, range: Range<usize>) {
        // Fetch and cache items
    }
}
```

### Message Pagination

```rust
// Paginate timeline efficiently
pub async fn paginate_until(
    timeline: &Timeline,
    mut num_items: usize,
) -> matrix_sdk::Result<()> {
    while num_items > 0 {
        let batch_size = num_items.min(50);
        match timeline.paginate_backwards(batch_size).await {
            Ok(()) => {
                let items = timeline.items().await;
                if items.len() < num_items {
                    num_items -= items.len();
                } else {
                    break;
                }
            }
            Err(matrix_sdk::Error::NoMoreEvents) => break,
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
```

## Summary

Robrix demonstrates:
- **Matrix protocol integration** using matrix-rust-sdk
- **Sliding sync** for efficient room list updates
- **Timeline management** with pagination and lazy loading
- **Reactive UI** using eyeball observables
- **Message rendering** with support for various content types
- **Authentication flows** including password and SSO
- **Encryption support** via Olm/Megolm
- **Cross-platform** deployment on desktop, mobile, and web
