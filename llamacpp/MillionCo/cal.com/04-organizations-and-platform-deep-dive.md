# Organizations and Platform Deep Dive

Cal.com's enterprise features transform it from a personal scheduling tool into a multi-tenant platform capable of serving large organizations with complex access control, provisioning, and white-label requirements.

## Organization Hierarchy

```mermaid
graph TB
    subgraph "Organization (isOrganization=true)"
        ORG[Organization<br/>Team entity with isOrganization=true]
        ORG_SETTINGS[OrganizationSettings<br/>Domain, auto-accept, policies]
        ORG_PROFILES[Org Profiles<br/>Username namespacing]
    end

    subgraph "Teams"
        TEAM_A[Team A<br/>Engineering]
        TEAM_B[Team B<br/>Sales]
        TEAM_C[Team C<br/>Support]
    end

    subgraph "Members"
        U1[User 1<br/>ADMIN]
        U2[User 2<br/>MEMBER]
        U3[User 3<br/>OWNER]
        U4[User 4<br/>MEMBER]
    end

    ORG --> ORG_SETTINGS
    ORG --> ORG_PROFILES
    ORG -->|parentId| TEAM_A
    ORG -->|parentId| TEAM_B
    ORG -->|parentId| TEAM_C

    TEAM_A --> U1
    TEAM_A --> U2
    TEAM_B --> U2
    TEAM_B --> U3
    TEAM_C --> U3
    TEAM_C --> U4

    ORG -.->|scope| U1
    ORG -.->|scope| U2
    ORG -.->|scope| U3
    ORG -.->|scope| U4
```

### The Team/Organization Duality

Cal.com uses the same `Team` model for both teams and organizations. The distinction:

| Aspect | Team | Organization |
|--------|------|-------------|
| `isOrganization` | `false` | `true` |
| `parentId` | Points to org | `null` |
| Has `OrganizationSettings` | No | Yes |
| Has `Profile` records | No | Yes (per-user profiles) |
| Can have sub-teams | No | Yes |
| Has domain routing | No | Yes |
| SCIM provisioning | No | Yes |
| SSO/SAML | No | Yes |

### Managed Organizations

An organization can manage other organizations:

```mermaid
graph TB
    PARENT[Manager Organization<br/>Enterprise Platform] --> MO1[Managed Org 1<br/>Client A]
    PARENT --> MO2[Managed Org 2<br/>Client B]
    PARENT --> MO3[Managed Org 3<br/>Client C]

    MO1 --> T1A[Team 1A]
    MO1 --> T1B[Team 1B]
    MO2 --> T2A[Team 2A]
```

The `ManagedOrganization` model links a child org to its parent manager org. This enables:
- Central billing
- Cross-org administration
- Template propagation

## Profile System

### Username Namespacing

When users join an organization, they get a `Profile` within that org's namespace:

```
Before org: user visits cal.com/john
After joining Acme org: user visits acme.cal.com/john

Profile record:
{
  userId: 1,
  organizationId: 5,
  username: "john",
  uid: "...",
}
```

A user can have profiles in multiple organizations, each with potentially different usernames.

### Domain Routing

Organizations can configure custom domains:

```mermaid
flowchart TD
    A[Request: acme.cal.com/john] --> B{Org domain lookup}
    B --> C[Find org: Acme]
    C --> D[Find profile: john in Acme]
    D --> E[Serve john's booking page<br/>with Acme branding]
```

The `orgAutoAcceptEmail` setting auto-approves users with matching email domains (e.g., `@acme.com` users auto-join the Acme org).

## Permission System (PBAC)

### Built-in Roles

```mermaid
graph TB
    subgraph "Organization Level"
        ORG_OWNER[OWNER<br/>Full org control]
        ORG_ADMIN[ADMIN<br/>Manage teams and members]
        ORG_MEMBER[MEMBER<br/>Basic access]
    end

    subgraph "Team Level"
        TEAM_OWNER[OWNER<br/>Full team control]
        TEAM_ADMIN[ADMIN<br/>Manage team settings]
        TEAM_MEMBER[MEMBER<br/>View and participate]
    end

    ORG_OWNER --> TEAM_OWNER
    ORG_ADMIN --> TEAM_ADMIN
    ORG_MEMBER --> TEAM_MEMBER
```

### Custom Roles (PBAC)

The Permission-Based Access Control system allows defining granular custom roles:

```typescript
// Role model
model Role {
  id          String       @id @default(uuid())
  name        String
  description String?
  teamId      Int
  team        Team         @relation(fields: [teamId], references: [id], onDelete: Cascade)
  permissions Permission[]
  memberships Membership[]
}

model Permission {
  id     String @id @default(uuid())
  action String // e.g., "event_type.create", "booking.cancel"
  roleId String
  role   Role   @relation(fields: [roleId], references: [id], onDelete: Cascade)
}
```

Custom roles can be assigned to memberships via `customRoleId`, providing fine-grained control over what team members can do.

## Directory Sync (DSYNC)

DSYNC allows organizations to provision users from identity providers:

```mermaid
sequenceDiagram
    participant IDP as Identity Provider<br/>(Okta, Azure AD, etc.)
    participant SCIM as SCIM Endpoint
    participant CAL as Cal.com

    IDP->>SCIM: POST /Users (create user)
    SCIM->>CAL: Create user + add to org
    CAL-->>SCIM: 201 Created

    IDP->>SCIM: PATCH /Users/:id (update)
    SCIM->>CAL: Update user attributes
    CAL-->>SCIM: 200 OK

    IDP->>SCIM: DELETE /Users/:id (deprovision)
    SCIM->>CAL: Deactivate/remove user
    CAL-->>SCIM: 204 No Content

    IDP->>SCIM: POST /Groups (create group)
    SCIM->>CAL: Create team in org
    CAL-->>SCIM: 201 Created
```

The `DSyncTeamGroupMapping` model maps external group IDs to Cal.com teams.

## SSO / SAML

Single Sign-On support via SAML:

```mermaid
flowchart TD
    A[User visits org login] --> B[Redirect to SAML IdP]
    B --> C[User authenticates at IdP]
    C --> D[IdP sends SAML assertion]
    D --> E[Cal.com validates assertion]
    E --> F{User exists?}
    F -->|Yes| G[Login user]
    F -->|No| H[Create user + join org]
    G --> I[Dashboard]
    H --> I
```

SAML configuration is stored per-organization with IdP metadata, certificate, and login URL.

## Enterprise Features (`packages/ee/`)

```
packages/ee/
  api-keys/           - API key management for organizations
  billing/            - Stripe-based subscription billing
  common/             - Shared enterprise utilities
  deployment/         - Self-hosted deployment features
  dsync/              - Directory sync (SCIM)
  impersonation/      - Admin user impersonation
  integration-attribute-sync/ - Sync user attributes from integrations
  managed-event-types/ - Template event types that cascade to children
  organizations/      - Organization management
  payments/           - Payment processing
  round-robin/        - Advanced round-robin features
  sso/                - SAML/SSO authentication
  teams/              - Team management
  users/              - User management
  workflows/          - Workflow automation
```

### Managed Event Types

Organization admins can create **managed event types** that serve as templates:

```mermaid
flowchart TD
    A[Admin creates managed<br/>event type template] --> B[Template: 30min Meeting<br/>with specific settings]
    B --> C[Auto-creates child event types<br/>for assigned team members]
    C --> D[User 1: 30min Meeting]
    C --> E[User 2: 30min Meeting]
    C --> F[User 3: 30min Meeting]

    G[Admin updates template] --> H[Changes cascade to all children]
    H --> D
    H --> E
    H --> F
```

The `parentId` field on `EventType` links children to their managed parent.

### Impersonation

Organization admins can impersonate member users for debugging:

```mermaid
flowchart TD
    A[Admin User] --> B{isAdminReviewed?}
    B -->|Yes| C[Can impersonate org members]
    B -->|No| D[Impersonation blocked]
    C --> E{Target user allows?}
    E -->|disableImpersonation=false| F[Impersonation session created]
    E -->|disableImpersonation=true| D
```

The `isAdminReviewed` flag requires instance-level admin approval before org admins can impersonate.

## Platform API (API v2)

The NestJS API v2 serves platform consumers who embed Cal.com functionality:

```mermaid
graph TB
    subgraph "Platform Architecture"
        OAUTH[OAuth 2.0<br/>PlatformOAuthClient]
        APIV2[NestJS REST API<br/>apps/api/v2]
        ATOMS[Platform Atoms<br/>packages/platform]
    end

    subgraph "External Consumers"
        PARTNER_A[Partner App A]
        PARTNER_B[Partner App B]
        WHITE_LABEL[White-Label Product]
    end

    PARTNER_A --> OAUTH
    PARTNER_B --> OAUTH
    WHITE_LABEL --> ATOMS

    OAUTH --> APIV2
    ATOMS --> APIV2

    APIV2 --> FEATURES[Business Logic]
    APIV2 --> PRISMA[Database]
```

### OAuth Client Model

```typescript
model PlatformOAuthClient {
  id            String    @id @default(uuid())
  name          String
  secret        String
  permissions   Int       @default(0)
  logo          String?
  redirectUris  String[]
  organizationId Int
  organization  Team      @relation(fields: [organizationId], references: [id], onDelete: Cascade)
  createdById   Int?
  createdBy     User?     @relation(fields: [createdById], references: [id])
  // ... access tokens, refresh tokens, etc.
}
```

Partners register OAuth clients, authenticate users through OAuth flows, and access the API on behalf of users.

### Platform Atoms

The `packages/platform/` provides embeddable React components ("Atoms") for white-labeling:

- Booking widgets
- Availability selectors
- Calendar views
- Event type managers

These are published to npm and can be embedded in partner applications with custom styling and branding.

## Attributes and Routing

### Team Member Attributes

Organizations can define custom attributes for team members:

```typescript
model Attribute {
  id     String          @id @default(uuid())
  name   String
  slug   String
  type   AttributeType   // TEXT, NUMBER, SINGLE_SELECT, MULTI_SELECT
  teamId Int
  team   Team            @relation(fields: [teamId], references: [id], onDelete: Cascade)
  options AttributeOption[]
}

model AttributeToUser {
  id          String          @id @default(uuid())
  memberId    Int
  member      Membership      @relation(fields: [memberId], references: [id], onDelete: Cascade)
  attributeOptionId String
  attributeOption   AttributeOption @relation(fields: [attributeOptionId], references: [id], onDelete: Cascade)
}
```

Example attributes:
- Language: English, Spanish, French
- Expertise: Sales, Technical, Billing
- Region: NA, EMEA, APAC
- Seniority: Junior, Senior, Lead

### Attribute-Based Routing

Routing forms use attributes to match bookers with the right team member:

```mermaid
flowchart TD
    A[Routing Form] --> B[Booker selects: Spanish language]
    B --> C[Segment Query:<br/>language CONTAINS 'Spanish']
    C --> D[Filter team members<br/>with Spanish attribute]
    D --> E[Round-robin among<br/>Spanish-speaking hosts]
```

The `rrSegmentQueryValue` JSON on EventType stores these filter expressions.

### Integration Attribute Sync

The `IntegrationAttributeSync` model syncs attributes from external systems:

```mermaid
flowchart LR
    SF[Salesforce] --> SYNC[Attribute Sync]
    HUB[HubSpot] --> SYNC
    SYNC --> ATTRS[Cal.com Attributes]
    ATTRS --> ROUTING[Routing Decisions]
```

## Billing Architecture

### Team Billing

```typescript
model TeamBilling {
  id           String   @id @default(uuid())
  teamId       Int      @unique
  team         Team     @relation(fields: [teamId], references: [id], onDelete: Cascade)
  stripeCustomerId    String?
  stripeSubscriptionId String?
  plan         String?
  status       String?
}
```

### Organization Billing

Separate billing for organizations with:
- Per-seat pricing
- Feature tier unlocking
- Overage handling for credits (SMS, AI phone calls)
- Monthly proration tracking via `SeatChangeLog` and `MonthlyProration`

### Credit System

```mermaid
flowchart TD
    A[Credit Balance] --> B{Monthly allocation<br/>based on plan}
    B --> C[SMS sends consume credits]
    B --> D[AI phone calls consume credits]
    C --> E{Balance remaining?}
    D --> E
    E -->|Yes| F[Allow action]
    E -->|No| G{Additional credits?}
    G -->|Yes| H[Use additional credits]
    G -->|No| I[Block action + notify]
```

## Organization Onboarding

```typescript
model OrganizationOnboarding {
  id              String   @id @default(uuid())
  name            String
  slug            String?
  logo            String?
  bio             String?
  orgOwnerEmail   String
  invitedMembers  Json?
  teams           Json?
  // ... onboarding state fields
  stripeSubscriptionId    String?
  stripeCustomerId        String?
}
```

The onboarding flow guides new organizations through:
1. Organization details (name, slug, logo)
2. Team creation
3. Member invitation
4. Billing setup
5. Domain configuration
6. SSO setup (optional)

## Multi-Tenancy Data Isolation

Data isolation in Cal.com's multi-tenant model:

```mermaid
flowchart TD
    A[API Request] --> B[Auth Middleware]
    B --> C[Extract user + org context]
    C --> D{Query scope}
    D --> E[Prisma queries filtered by:<br/>organizationId, teamId, userId]
    E --> F[Results scoped to tenant]
```

Key isolation boundaries:
- Event types are scoped to user or team
- Bookings reference specific users and event types
- Credentials are per-user or per-team
- Webhooks can be user-level, team-level, or event-type-level
- Routing forms are team-scoped
- Attributes are organization-scoped

The `organizationId` on users and the `parentId` on teams form the primary scoping mechanism. tRPC middleware and NestJS guards enforce access control at the API layer.
