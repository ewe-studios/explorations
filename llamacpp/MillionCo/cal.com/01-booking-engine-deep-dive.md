# Booking Engine Deep Dive

The booking engine is the central nervous system of Cal.com. It handles everything from validating incoming booking requests to creating calendar events, processing payments, sending notifications, and managing the full booking lifecycle.

## Architecture Overview

```mermaid
graph TB
    subgraph "Entry Points"
        WEB_BOOK[Web Booker Component]
        API_V1[API v1 Endpoint]
        API_V2[API v2 NestJS Module]
        EMBED[Embed Widget]
    end

    subgraph "Booking Handlers"
        HNB[handleNewBooking<br/>packages/features/bookings/lib/handleNewBooking/]
        CIB[create-instant-booking.ts]
        CRB[create-recurring-booking.ts]
    end

    subgraph "Core Pipeline"
        VALIDATE[Validate Input<br/>bookingCreateBodySchema]
        CONFLICT[Conflict Detection<br/>conflictChecker/]
        ASSIGN[Host Assignment<br/>getLuckyUser.ts]
        CREATE[Create Booking<br/>create-booking.ts]
        EVENTS[Calendar Events<br/>EventManager.ts]
        PAYMENT[Payment Processing<br/>handlePayment.ts]
        NOTIFY[Notifications<br/>BookingEmailSmsHandler.ts]
        WEBHOOK[Webhook Triggers<br/>handleWebhookTrigger.ts]
    end

    WEB_BOOK --> HNB
    API_V1 --> HNB
    API_V2 --> HNB
    EMBED --> HNB

    HNB --> VALIDATE
    CIB --> VALIDATE
    CRB --> HNB

    VALIDATE --> CONFLICT
    CONFLICT --> ASSIGN
    ASSIGN --> CREATE
    CREATE --> EVENTS
    EVENTS --> PAYMENT
    PAYMENT --> NOTIFY
    NOTIFY --> WEBHOOK
```

## The handleNewBooking Flow

The primary booking handler lives at `packages/features/bookings/lib/handleNewBooking/`. This is a multi-file module that orchestrates the entire booking creation process.

### Step 1: Input Validation

The booking request is validated against `bookingCreateBodySchema.ts`:

```typescript
// Simplified schema structure
const bookingCreateBody = z.object({
  start: z.string(),          // ISO datetime of slot start
  end: z.string(),            // ISO datetime of slot end
  eventTypeId: z.number(),
  eventTypeSlug: z.string(),
  timeZone: z.string(),
  language: z.string(),
  responses: z.record(z.any()), // Dynamic booking field responses
  metadata: z.record(z.any()).optional(),
  recurringEventId: z.string().optional(),
  hasHashedBookingLink: z.boolean().optional(),
  seatReferenceUid: z.string().optional(),
  orgSlug: z.string().optional(),
});
```

The `getBookingDataSchema` function dynamically constructs the validation schema based on the event type's configured booking fields, ensuring required fields are present and correctly typed.

### Step 2: Event Type Loading and Verification

The handler loads the full event type with all relationships:
- Hosts and their credentials
- Team membership
- Workflow configurations
- Payment settings
- Schedule and availability rules

It verifies the event type exists, is not hidden (unless accessed via hashed link), and belongs to the correct team/user.

### Step 3: Conflict Detection

The conflict checker (`packages/features/bookings/lib/conflictChecker/`) verifies the requested slot is still available:

```mermaid
flowchart TD
    A[Requested Slot] --> B{Check existing bookings}
    B --> C{Any overlapping<br/>ACCEPTED bookings?}
    C -->|Yes| D[CONFLICT - Slot taken]
    C -->|No| E{Check calendar busy times}
    E --> F{External calendar<br/>conflicts?}
    F -->|Yes| D
    F -->|No| G{Check booking limits}
    G --> H{Exceeds daily/weekly/<br/>monthly limit?}
    H -->|Yes| D
    H -->|No| I[AVAILABLE]
```

For **collective events**, all hosts must be available. For **round-robin**, at least one eligible host must be free.

### Step 4: Host Assignment (Round-Robin)

For round-robin events, `getLuckyUser.ts` implements a sophisticated assignment algorithm:

```mermaid
flowchart TD
    A[All Team Hosts] --> B[Filter by availability]
    B --> C[Filter by segment<br/>attribute matching]
    C --> D{Priority-based<br/>grouping}
    D --> E[Highest priority<br/>available hosts]
    E --> F{Weight-based<br/>selection}
    F --> G[Calculate target ratio<br/>weight / total_weight]
    G --> H[Calculate actual ratio<br/>bookings / total_bookings]
    H --> I[Select host with<br/>largest gap below target]
    I --> J[Tiebreaker:<br/>least recent booking]
    J --> K[Assigned Host]
```

The algorithm ensures fair distribution according to configured weights while respecting priority tiers:

```typescript
// Conceptual algorithm
function getLuckyUser(hosts: Host[], existingBookings: Booking[]) {
  // 1. Group by priority - highest priority first
  const priorityGroups = groupBy(hosts, h => h.priority);
  const topPriority = max(Object.keys(priorityGroups));
  const candidates = priorityGroups[topPriority];

  // 2. Calculate weighted fair share
  const totalWeight = sum(candidates.map(h => h.weight));
  const totalBookings = existingBookings.length;

  // 3. Find most "under-booked" host relative to their weight
  return candidates.reduce((lucky, host) => {
    const targetRatio = host.weight / totalWeight;
    const actualRatio = countBookings(host) / totalBookings;
    const deficit = targetRatio - actualRatio;
    return deficit > lucky.deficit ? { host, deficit } : lucky;
  });
}
```

Additional considerations:
- **No-show tracking**: `includeNoShowInRRCalculation` optionally counts no-shows in the fairness calculation
- **Calibration**: Dynamically adjusts for weight changes mid-cycle
- **RR Reset Interval**: Booking counts reset monthly (configurable)
- **Timestamp basis**: Can use `CREATED_AT` or `START_TIME` for counting

### Step 5: Booking Creation

The `create-booking.ts` module persists the booking to the database:

```typescript
// Simplified booking creation
const booking = await prisma.booking.create({
  data: {
    uid: generateUniqueId(),
    title: eventType.title,
    startTime: slot.start,
    endTime: slot.end,
    userId: assignedHost.id,
    eventTypeId: eventType.id,
    status: requiresConfirmation ? "PENDING" : "ACCEPTED",
    responses: validatedResponses,
    attendees: {
      create: attendeeData,
    },
    metadata: bookingMetadata,
  },
});
```

An **idempotency key** is generated from `(start, end, email, eventTypeId)` to prevent duplicate bookings from network retries.

### Step 6: Calendar Event Creation (EventManager)

The `EventManager.ts` orchestrates creating events on all relevant calendars:

```mermaid
sequenceDiagram
    participant EM as EventManager
    participant DC as Destination Calendar
    participant CC as Connected Calendars
    participant VID as Video Provider

    EM->>VID: Create video meeting (Daily/Zoom/etc)
    VID-->>EM: Meeting URL + credentials
    EM->>DC: Create event on destination calendar
    DC-->>EM: Calendar event ID
    EM->>CC: Create event on additional calendars
    CC-->>EM: Additional event IDs
    EM-->>EM: Store BookingReferences
```

The EventManager:
1. Creates the video conferencing link (if applicable)
2. Creates the event on the host's **destination calendar** (primary output calendar)
3. Optionally creates on additional connected calendars
4. Stores all external IDs as `BookingReference` records for later updates/deletion

### Step 7: Payment Processing

If the event type has a price > 0:

```mermaid
flowchart TD
    A[Booking Created<br/>status=PENDING] --> B{Payment required?}
    B -->|No| C[Set status=ACCEPTED]
    B -->|Yes| D[Create Payment record]
    D --> E[Redirect to Stripe Checkout]
    E --> F{Payment successful?}
    F -->|Yes| G[Set status=ACCEPTED<br/>paid=true]
    F -->|No| H[Keep status=PENDING<br/>Auto-cancel after timeout]
    G --> I[Continue to notifications]
```

### Step 8: Notification Dispatch

`BookingEmailSmsHandler.ts` handles all notification channels:

- **Email**: Confirmation to attendee, notification to host
- **SMS**: Via configured SMS provider (Twilio)
- **Workflows**: Trigger any configured workflow automations
- **Webhooks**: Fire `BOOKING_CREATED` webhook to all subscribers

## Cancellation Flow

`handleCancelBooking.ts` handles booking cancellations:

```mermaid
flowchart TD
    A[Cancel Request] --> B[Load booking + references]
    B --> C[Delete calendar events<br/>via EventManager]
    C --> D[Cancel video meeting]
    D --> E[Process refund<br/>if paid]
    E --> F[Update booking status<br/>to CANCELLED]
    F --> G[Send cancellation emails]
    G --> H[Fire BOOKING_CANCELLED webhook]
    H --> I{Has cancellation reason?}
    I -->|Yes| J[Store reason]
    I -->|No| K[Done]
```

Cancellation can be initiated by the host or attendee. The `cancelledBy` field tracks who initiated it.

## Rescheduling Flow

Rescheduling is essentially a cancel-and-rebook operation:

1. The original booking's `uid` is passed as `fromReschedule`
2. A new booking is created for the new time slot
3. The old booking is cancelled (linked via `fromReschedule`)
4. Calendar events are updated rather than deleted/recreated when possible
5. The `rescheduledBy` field tracks the initiator

For round-robin events, `rescheduleWithSameRoundRobinHost` can force keeping the same host.

## Seat Management

Seated events have special handling in `handleSeats/`:

```mermaid
flowchart TD
    A[Booking request for<br/>seated event] --> B{Existing booking<br/>for this slot?}
    B -->|No| C[Create new booking<br/>with first seat]
    B -->|Yes| D{Seats available?}
    D -->|No| E[Return FULL error]
    D -->|Yes| F[Add BookingSeat<br/>to existing booking]
    F --> G[Update attendee list]
    G --> H[Update calendar event<br/>with new attendee]
```

Each seat gets a `BookingSeat` record with:
- Unique `referenceUid` for individual management
- Separate `data` JSON for per-attendee responses
- Independent cancellation capability

## Confirmation Flow

When `requiresConfirmation` is true:

```mermaid
sequenceDiagram
    participant BOOKER as Booker
    participant SYSTEM as Cal.com
    participant HOST as Host

    BOOKER->>SYSTEM: Create booking
    SYSTEM->>SYSTEM: Set status=PENDING
    SYSTEM->>HOST: Email: "New booking request"
    SYSTEM->>BOOKER: Email: "Booking pending confirmation"

    HOST->>SYSTEM: Confirm booking
    SYSTEM->>SYSTEM: Set status=ACCEPTED
    SYSTEM->>SYSTEM: Create calendar events
    SYSTEM->>BOOKER: Email: "Booking confirmed"
    SYSTEM->>HOST: Email: "You confirmed booking"
```

`requiresConfirmationForFreeEmail` is a clever feature that only requires confirmation when the booker uses a free email domain (gmail, outlook, etc.) but auto-accepts company emails.

## Instant Meetings

Instant meetings (`isInstantEvent=true`) provide a "join now" experience:

1. A token is generated with short expiry (`instantMeetingExpiryTimeOffsetInSeconds`)
2. The booking is created with `status=AWAITING_HOST`
3. The booker waits on a holding page
4. When a host accepts, the video call starts immediately
5. If no host accepts before timeout, the booking is auto-cancelled

## Booking Limits

The limit system (`checkBookingLimits.ts`, `checkDurationLimits.ts`) enforces:

```typescript
interface BookingLimits {
  PER_DAY?: number;
  PER_WEEK?: number;
  PER_MONTH?: number;
  PER_YEAR?: number;
}

interface DurationLimits {
  PER_DAY?: number;    // max minutes per day
  PER_WEEK?: number;
  PER_MONTH?: number;
  PER_YEAR?: number;
}
```

Limits can be set at both event type and team level. Team-level limits with `includeManagedEventsInLimits` aggregate across all managed event types.

## Error Handling

The booking engine defines specific error codes:

- `NO_AVAILABLE_USERS_FOUND_ERROR` - No hosts available for the requested time
- `BOOKING_LIMIT_REACHED` - Booking count limit exceeded
- `DURATION_LIMIT_REACHED` - Duration limit exceeded
- `ALREADY_BOOKED` - Slot already taken (conflict)
- `PAYMENT_REQUIRED` - Payment needed but not completed
- `REQUIRES_CONFIRMATION` - Booking created but pending host approval

These error codes are essential for providing clear feedback in the booker UI and API responses.

## Performance Considerations

The booking engine must handle concurrent requests for the same slot. Key strategies:

1. **Idempotency keys** prevent duplicate bookings from retries
2. **Database-level unique constraints** on `Booking.uid` and `Booking.idempotencyKey`
3. **Optimistic concurrency**: The conflict checker runs just before creation, minimizing the race window
4. **`requiresConfirmationWillBlockSlot`**: When confirmation is required, this flag determines whether the slot is immediately blocked or remains available until confirmed

For high-traffic event types (popular webinars, etc.), the seat-based model with atomic seat counting prevents overselling.
