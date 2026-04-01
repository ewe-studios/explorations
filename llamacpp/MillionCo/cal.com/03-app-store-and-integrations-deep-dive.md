# App Store and Integrations Deep Dive

Cal.com's app store is one of its most distinguishing architectural features. With 150+ integrations, it provides a plugin system that connects scheduling with calendars, video conferencing, payments, CRMs, messaging platforms, analytics, and automation tools.

## App Store Architecture

```mermaid
graph TB
    subgraph "App Store Package"
        direction TB
        REGISTRY[Generated Registry<br/>_appRegistry.ts]
        META_GEN[apps.metadata.generated.ts]
        SCHEMAS_GEN[apps.schemas.generated.ts]
        SERVER_GEN[apps.server.generated.ts]
        CLI[App Store CLI<br/>packages/app-store-cli]
    end

    subgraph "App Structure"
        direction TB
        APP_META[_metadata.ts<br/>Name, category, icon]
        APP_API[api/<br/>OAuth callbacks, webhooks]
        APP_LIB[lib/<br/>Service implementations]
        APP_COMP[components/<br/>React UI]
        APP_ZOD[zod.ts<br/>Credential schemas]
    end

    subgraph "Integration Points"
        direction TB
        CAL_SVC[CalendarService<br/>getAvailability, createEvent]
        VIDEO_SVC[VideoApiAdapter<br/>createMeeting, deleteMeeting]
        PAY_SVC[PaymentService<br/>create, refund]
        CRM_SVC[CRMService<br/>createContact, updateDeal]
    end

    CLI -->|generates| REGISTRY
    CLI -->|generates| META_GEN
    CLI -->|generates| SCHEMAS_GEN
    CLI -->|generates| SERVER_GEN

    APP_META --> REGISTRY
    APP_LIB --> CAL_SVC
    APP_LIB --> VIDEO_SVC
    APP_LIB --> PAY_SVC
    APP_LIB --> CRM_SVC
```

## App Categories

### Calendar Integrations

| App | Protocol | Notes |
|-----|----------|-------|
| Google Calendar | OAuth 2.0 + Google Calendar API | Primary integration, supports push notifications |
| Outlook/Office 365 | OAuth 2.0 + Microsoft Graph API | Supports both personal and work accounts |
| Apple Calendar | App-specific password + CalDAV | Uses CalDAV protocol under the hood |
| CalDAV | CalDAV protocol | Generic support for any CalDAV server (Nextcloud, Radicale, etc.) |

Calendar apps implement the `CalendarService` interface:

```typescript
interface CalendarService {
  createEvent(event: CalendarEvent): Promise<NewCalendarEventType>;
  updateEvent(uid: string, event: CalendarEvent): Promise<NewCalendarEventType>;
  deleteEvent(uid: string, event: CalendarEvent): Promise<void>;
  getAvailability(
    dateFrom: string,
    dateTo: string,
    selectedCalendars: SelectedCalendar[]
  ): Promise<EventBusyDate[]>;
  listCalendars(): Promise<IntegrationCalendar[]>;
}
```

### Video Conferencing

| App | Notes |
|-----|-------|
| Daily.co | Default/built-in video provider, Cal Video branded |
| Zoom | OAuth integration, automatic meeting creation |
| Google Meet | Via Google Calendar event |
| Microsoft Teams | Via Microsoft Graph API |
| Webex | Cisco Webex integration |
| Jitsi | Self-hosted option |
| Campfire | 37signals video |
| Whereby | Embeddable video rooms |
| Tandem | Team video platform |

Video apps implement the `VideoApiAdapter`:

```typescript
interface VideoApiAdapter {
  createMeeting(event: CalendarEvent): Promise<VideoCallData>;
  updateMeeting(bookingRef: PartialReference, event: CalendarEvent): Promise<VideoCallData>;
  deleteMeeting(uid: string): Promise<void>;
  getAvailability(dateFrom?: string, dateTo?: string): Promise<EventBusyDate[]>;
}
```

### Payment

| App | Notes |
|-----|-------|
| Stripe | Primary payment provider, supports checkout sessions |
| PayPal | PayPal payment integration |
| Alby | Bitcoin Lightning payments |
| BtcPayServer | Self-hosted Bitcoin payments |

### CRM Integrations

| App | Notes |
|-----|-------|
| Salesforce | Full CRM sync, lead/contact creation |
| HubSpot | Contact and deal management |
| Close | CRM sync with close.com |
| Zoho CRM | Zoho ecosystem integration |
| Attio | Modern CRM integration |
| Zoho Bigin | Small business CRM |

### Messaging & Notifications

| App | Notes |
|-----|-------|
| Slack | Channel notifications on bookings |
| Telegram | Bot notifications |
| WhatsApp | Message notifications |
| Signal | Secure messaging notifications |

### Analytics & Tracking

| App | Notes |
|-----|-------|
| Google Analytics 4 | Booking page tracking |
| Plausible | Privacy-focused analytics |
| Umami | Self-hosted analytics |
| Twipla | Website intelligence |

### Automation

| App | Notes |
|-----|-------|
| Zapier | Trigger Zaps on booking events |
| Make (Integromat) | Automation workflows |
| n8n | Self-hosted automation |

## App Lifecycle

### Installation Flow

```mermaid
sequenceDiagram
    participant U as User
    participant UI as App Store UI
    participant API as Cal.com API
    participant EXT as External Service

    U->>UI: Click "Install" on app
    UI->>API: POST /apps/install

    alt OAuth-based app
        API->>EXT: Redirect to OAuth consent
        EXT->>U: Show permissions dialog
        U->>EXT: Grant access
        EXT->>API: Callback with auth code
        API->>EXT: Exchange code for tokens
        API->>API: Store encrypted credential
    else API Key-based app
        API->>UI: Show API key form
        U->>UI: Enter API key
        UI->>API: Store encrypted credential
    end

    API->>UI: App installed successfully
```

### Credential Storage

```mermaid
erDiagram
    User ||--o{ Credential : has
    Credential }o--o| App : for

    Credential {
        int id
        string type
        json key
        string encryptedKey
        int userId
        string appId
        boolean invalid
    }

    App {
        string slug
        string dirName
        json keys
        string categories
        boolean enabled
    }
```

Credentials store OAuth tokens and API keys, encrypted with `CALENDSO_ENCRYPTION_KEY`. The `invalid` flag marks credentials that need re-authentication (expired refresh tokens, revoked access).

### Delegation Credentials

For enterprise organizations, **delegation credentials** allow a single OAuth connection to act on behalf of multiple users:

```mermaid
graph TB
    ORG[Organization] --> DC[Delegation Credential<br/>Service Account Token]
    DC --> U1[User 1's Calendar]
    DC --> U2[User 2's Calendar]
    DC --> U3[User 3's Calendar]

    style DC fill:#f96
```

This avoids requiring each user to individually connect their calendar - the organization admin sets up domain-wide access once.

## Code Generation Pipeline

The app store CLI (`packages/app-store-cli`) generates registry files that wire apps into the application:

```mermaid
flowchart TD
    A[App Store CLI] --> B[Scan all app directories]
    B --> C[Read _metadata.ts from each]
    C --> D[Generate _appRegistry.ts<br/>App slug -> import mapping]
    C --> E[Generate apps.metadata.generated.ts<br/>All metadata for client-side]
    C --> F[Generate apps.schemas.generated.ts<br/>Zod schemas for credentials]
    C --> G[Generate apps.server.generated.ts<br/>Server-side app constructors]
    C --> H[Generate calendar.services.generated.ts<br/>Calendar service factory]
    C --> I[Generate video.adapters.generated.ts<br/>Video adapter factory]
```

These generated files create a type-safe factory pattern where the application can instantiate the correct service based on the app slug.

## App Metadata Structure

Each app defines metadata:

```typescript
// packages/app-store/googlevideo/_metadata.ts (example)
export const metadata = {
  name: "Google Meet",
  description: "Video conferencing by Google",
  type: "google_video",
  variant: "conferencing",
  categories: ["conferencing"],
  logo: "icon.svg",
  publisher: "Cal.com",
  url: "https://meet.google.com",
  isOAuth: true,
  dirName: "googlevideo",
  appData: {
    location: {
      type: "integrations:google:meet",
      label: "Google Meet",
    },
  },
};
```

Key metadata fields:
- `type` - Unique identifier used in credential matching
- `variant` - Determines UI treatment (conferencing, calendar, payment, etc.)
- `categories` - For app store browsing/filtering
- `appData.location` - How the app appears as a meeting location option

## Integration with Booking Flow

### Video Integration

```mermaid
sequenceDiagram
    participant BH as Booking Handler
    participant EM as Event Manager
    participant VF as Video Factory
    participant DAILY as Daily.co API

    BH->>EM: createAllCalendarEvents()
    EM->>VF: getVideoAdapter(credential)
    VF-->>EM: DailyVideoApiAdapter
    EM->>DAILY: createMeeting(event)
    DAILY-->>EM: { url, id, password }
    EM->>EM: Store BookingReference
    EM-->>BH: Meeting URL in booking
```

### Calendar Integration

```mermaid
sequenceDiagram
    participant EM as Event Manager
    participant CF as Calendar Factory
    participant GCAL as Google Calendar API

    EM->>CF: getCalendarAdapter(credential)
    CF-->>EM: GoogleCalendarService
    EM->>GCAL: events.insert(calendarEvent)
    GCAL-->>EM: { eventId, iCalUID }
    EM->>EM: Store BookingReference
```

### CRM Integration

CRM integrations hook into the booking lifecycle via the `CrmManager`:

```mermaid
flowchart TD
    A[Booking Created] --> B[CRM Manager]
    B --> C{CRM connected?}
    C -->|Yes| D[Find or create contact]
    D --> E[Create activity/deal]
    E --> F[Sync booking details]
    C -->|No| G[Skip CRM sync]
```

### Webhook Apps (Zapier, Make)

These apps register as webhook subscribers and receive booking events:

```typescript
// Webhook payload structure
interface BookingWebhookPayload {
  triggerEvent: "BOOKING_CREATED" | "BOOKING_RESCHEDULED" | "BOOKING_CANCELLED";
  createdAt: string;
  payload: {
    title: string;
    startTime: string;
    endTime: string;
    attendees: { email: string; name: string }[];
    organizer: { email: string; name: string };
    location: string;
    metadata: Record<string, any>;
    responses: Record<string, any>;
  };
}
```

## App Configuration in Event Types

When creating an event type, users configure which apps apply:

```mermaid
graph TB
    ET[Event Type Configuration] --> LOC[Location Options]
    ET --> PAY[Payment Settings]
    ET --> HOOKS[Connected Apps]

    LOC --> L1[Cal Video - Daily.co]
    LOC --> L2[Zoom Meeting]
    LOC --> L3[Google Meet]
    LOC --> L4[In Person]
    LOC --> L5[Phone Call]

    PAY --> P1[Stripe - Price, Currency]

    HOOKS --> H1[Salesforce - Auto-create lead]
    HOOKS --> H2[Slack - Notify channel]
    HOOKS --> H3[Zapier - Trigger automation]
```

The event type's `metadata.apps` JSON stores per-app configuration that is merged with the app's defaults at booking time.

## Booker-Facing App Components

Some apps contribute UI to the booking page:

```mermaid
flowchart TD
    A[Booking Page] --> B[BookingPageTagManager]
    B --> C{Analytics apps installed?}
    C -->|GA4| D[Inject GA4 tracking script]
    C -->|Plausible| E[Inject Plausible script]
    C -->|Umami| F[Inject Umami script]

    A --> G[Location Display]
    G --> H[Show meeting link type]
    G --> I[Show address for in-person]

    A --> J[Payment Form]
    J --> K[Stripe Elements / PayPal button]
```

## Testing Infrastructure

App store tests use:
- `packages/app-store/tests/` - Shared test utilities
- `packages/app-store/test-setup.ts` - Test environment setup
- Individual app `__tests__/` directories
- Mock credential factories for testing without real API keys

The `delegationCredential.test.ts` at the app-store root tests the delegation credential flow that spans multiple apps.

## Creating a New Integration

The CLI provides scaffolding:

```bash
yarn app-store create
# Interactive prompts for:
# - App name
# - Category (calendar, conferencing, payment, etc.)
# - Auth type (OAuth, API key, none)
```

This generates the directory structure, metadata template, and placeholder files. The developer then implements:

1. `_metadata.ts` - App identity and configuration
2. `lib/CalendarService.ts` or `lib/VideoApiAdapter.ts` - Core service implementation
3. `api/callback.ts` - OAuth callback handler (if OAuth)
4. `zod.ts` - Credential validation schema
5. `components/` - Settings UI components

After implementation, running `yarn app-store:build` regenerates the registry files to include the new app.
