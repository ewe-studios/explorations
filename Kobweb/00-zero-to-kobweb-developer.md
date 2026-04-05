---
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/Kobweb
source: github.com/varabyte/kobweb
explored_at: 2026-04-05
prerequisites: Kotlin basics, Compose familiarity, Web development concepts
---

# Zero to Kobweb Developer - Complete Fundamentals

## Table of Contents

1. [What is Kobweb?](#what-is-kobweb)
2. [Core Concepts](#core-concepts)
3. [Getting Started](#getting-started)
4. [Project Structure](#project-structure)
5. [Pages and Routing](#pages-and-routing)
6. [Components and Composition](#components-and-composition)
7. [Styling with Modifiers](#styling-with-modifiers)
8. [State Management](#state-management)
9. [Layouts](#layouts)
10. [Navigation](#navigation)
11. [API Routes](#api-routes)
12. [Static Export](#static-export)
13. [Full-Stack Mode](#full-stack-mode)
14. [Silk UI Library](#silk-ui-library)
15. [Deployment](#deployment)

## What is Kobweb?

**Kobweb** is an opinionated Kotlin web development framework built on top of JetBrains Compose HTML. It brings Next.js-style file-based routing and SSR capabilities to Kotlin, enabling full-stack web development with pure Kotlin code.

### The Problem Kobweb Solves

Traditional Kotlin web development:

```
1. Choose between Spring Boot (backend only) or Ktor (minimal backend)
2. Write frontend in JavaScript/TypeScript separately
3. Share types manually or via code generation
4. Maintain two language ecosystems
5. No unified developer experience
```

Kobweb approach:

```
1. Write everything in Kotlin
2. Share types between frontend and backend
3. File-based routing (no manual route registration)
4. Compose-style declarative UI
5. Single language, single build system
```

### Key Features

| Feature | Description |
|---------|-------------|
| **File-based Routing** | Pages discovered from `pages/` directory structure |
| **Compose HTML** | Declarative UI using Kotlin Compose runtime |
| **SSR Export** | Static site generation with hydration |
| **Full-Stack Mode** | Ktor server with API routes and websockets |
| **KSP Processing** | Compile-time code generation (no reflection) |
| **Silk UI** | Chakra UI-inspired component library |
| **Markdown Support** | Markdown files as pages with Kotlin interpolation |
| **Live Reload** | Development server with hot module replacement |

### Kobweb vs Alternatives

| Framework | Language | SSR | Routing | Bundle Size | Learning Curve |
|-----------|----------|-----|---------|-------------|----------------|
| **Kobweb** | Kotlin | Yes (export) | File-based | Medium | Medium |
| **Next.js** | TypeScript | Yes | File-based | Medium | Medium |
| **React** | TypeScript | No (manual) | Manual | Small | Low |
| **Spring Boot** | Kotlin/Java | Yes | Manual | Large | High |
| **Vaadin** | Kotlin/Java | Yes | Manual | Large | Medium |

## Core Concepts

### 1. Pages

Pages are Composable functions annotated with `@Page`:

```kotlin
@Page
@Composable
fun HomePage() {
    Column {
        H1 { Text("Welcome to Kobweb!") }
    }
}
```

### 2. Routing

File path determines URL route:
- `pages/Index.kt` → `/`
- `pages/about/Index.kt` → `/about`
- `pages/users/[userId]/Profile.kt` → `/users/{userId}`

### 3. Components

Reusable Composable functions:

```kotlin
@Composable
fun UserCard(user: User) {
    Card {
        Text(user.name)
        Text(user.email)
    }
}
```

### 4. Modifiers

Chainable styling objects:

```kotlin
Box(
    Modifier
        .fillMaxWidth()
        .padding(16.px)
        .backgroundColor(Color.Blue)
)
```

### 5. Layouts

Layout composable functions wrap pages:

```kotlin
@Layout
@Composable
fun DefaultLayout(ctx: LayoutContext, content: @Composable () -> Unit) {
    Column {
        Header()
        Box(Modifier.weight(1f)) { content() }
        Footer()
    }
}
```

### 6. API Routes

Server-side endpoints with `@Api`:

```kotlin
@Api
suspend fun fetchUser(ctx: ApiContext) {
    val userId = ctx.pathParams["userId"]
    val user = database.getUser(userId)
    ctx.respond(user)
}
```

### 7. State Management

Compose runtime state:

```kotlin
var count by remember { mutableStateOf(0) }
```

### 8. Silk UI

Pre-built component library:

```kotlin
import com.varabyte.kobweb.silk.components.*

Button(onClick = { }) {
    Text("Click me")
}
```

## Getting Started

### Prerequisites

```bash
# Install JDK 17+
java -version

# Install Kobweb CLI
# Option 1: Using SDKMAN
sdk install kobweb

# Option 2: Using Homebrew (macOS)
brew install varabyte/tap/kobweb

# Option 3: Download from GitHub releases
```

### Create New Project

```bash
# Create new application
kobweb create app my-app

# Navigate to project
cd my-app
```

### Project Template

```kotlin
// gradle/libs.versions.toml
[versions]
kobweb = "0.23.1"
kotlin = "2.2.10"
jetbrains-compose = "1.8.0"

[libraries]
kobweb = { module = "com.varabyte.kobweb:kobweb-core", version.ref = "kobweb" }
kobweb-silk = { module = "com.varabyte.kobweb:kobweb-silk", version.ref = "kobweb" }

// build.gradle.kts
plugins {
    id("com.varabyte.kobweb.application") version "0.23.1"
}

dependencies {
    implementation(libs.kobweb)
    implementation(libs.kobweb.silk)
}

kobweb {
    pagesPackage.set(".pages")
    apiPackage.set(".api")
}
```

### Run Development Server

```bash
# Start development server with hot reload
kobweb run

# Server runs on http://localhost:8080
```

### Your First Page

```kotlin
// src/jsMain/kotlin/pages/Index.kt
package myapp.pages

import androidx.compose.runtime.*
import com.varabyte.kobweb.core.*
import com.varabyte.kobweb.silk.components.*
import org.jetbrains.compose.web.css.*
import org.jetbrains.compose.web.dom.*

@Page
@Composable
fun IndexPage() {
    var count by remember { mutableStateOf(0) }
    
    Column(
        Modifier
            .fillMaxWidth()
            .padding(32.px),
        horizontalAlignment = androidx.compose.ui.Alignment.CenterHorizontally
    ) {
        H1 { Text("Hello, Kobweb!") }
        
        Text("Count: $count")
        
        Button(
            onClick = { count++ }
        ) {
            Text("Increment")
        }
    }
}
```

## Project Structure

```
my-app/
├── src/
│   ├── jsMain/
│   │   ├── kotlin/
│   │   │   ├── myapp/
│   │   │   │   ├── pages/           # Page components
│   │   │   │   │   ├── Index.kt     # Home page (/)
│   │   │   │   │   ├── about/
│   │   │   │   │   │   └── Index.kt # About page (/about)
│   │   │   │   │   └── users/
│   │   │   │   │       └── [userId]/
│   │   │   │   │           └── Profile.kt  # Dynamic route (/users/{userId})
│   │   │   │   ├── components/      # Reusable components
│   │   │   │   │   ├── UserCard.kt
│   │   │   │   │   └── Navigation.kt
│   │   │   │   ├── layouts/         # Layout components
│   │   │   │   │   └── DefaultLayout.kt
│   │   │   │   └── styles/          # Global styles
│   │   │   │       └── Theme.kt
│   │   │   └── resources/
│   │   │       └── public/          # Static assets
│   │   │           ├── images/
│   │   │           └── favicon.ico
│   │   └── resources/
├── src/jvmMain/
│   └── kotlin/
│       └── myapp/
│           └── api/                 # API routes
│               └── users/
│                   └── UserApi.kt
├── build.gradle.kts                 # Build configuration
├── settings.gradle.kts              # Project settings
├── gradle.properties                # Gradle properties
├── gradle/
│   └── libs.versions.toml           # Version catalog
├── public/                          # Additional public files
└── kobweb/                          # Generated files (GitIgnore)
```

## Pages and Routing

### Basic Page

```kotlin
@Page
@Composable
fun AboutPage() {
    Column {
        H1 { Text("About Us") }
        P { Text("Welcome to our website!") }
    }
}
```

### Route Naming

File path automatically determines route:

| File Path | Route |
|-----------|-------|
| `pages/Index.kt` | `/` |
| `pages/about/Index.kt` | `/about` |
| `pages/blog/Post.kt` | `/blog/post` |
| `pages/contact/Us.kt` | `/contact/us` |

### Route Overrides

```kotlin
// Custom slug
@Page("custom-path")
@Composable
fun MyPage() { ... }

// Make this the index for a directory
@Page("index")
@Composable
fun BlogIndex() { ... }

// Absolute path override
@Page("/special/path/")
@Composable
fun SpecialPage() { ... }
```

### Dynamic Routes

```kotlin
// File: pages/users/[userId]/Profile.kt
@Page
@Composable
fun UserProfilePage(ctx: PageContext) {
    // Access dynamic parameter
    val userId = ctx.route.params["userId"]
    
    Column {
        H1 { Text("User Profile: $userId") }
    }
}

// Multiple dynamic segments
// File: pages/orgs/[orgId]/users/[userId]/Index.kt
@Page
@Composable
fun UserDetailPage(ctx: PageContext) {
    val orgId = ctx.route.params["orgId"]
    val userId = ctx.route.params["userId"]
}
```

### Catch-all Routes

```kotlin
// File: pages/docs/{...}.kt
@Page
@Composable
fun DocsPage(ctx: PageContext) {
    // Matches /docs, /docs/intro, /docs/guide/advanced, etc.
    val pathSegments = ctx.route.params["..."]?.split("/") ?: emptyList()
}
```

### Query Parameters

```kotlin
@Page
@Composable
fun SearchPage(ctx: PageContext) {
    // Access query parameters
    val query = ctx.route.queryParams["q"] ?: ""
    val page = ctx.route.queryParams["page"]?.toIntOrNull() ?: 1
    
    Column {
        Text("Searching for: $query")
        Text("Page: $page")
    }
}
```

## Components and Composition

### Basic Component

```kotlin
@Composable
fun UserCard(
    user: User,
    onCardClick: () -> Unit = {}
) {
    Card(
        Modifier
            .padding(8.px)
            .onClick { onCardClick() }
    ) {
        Column(Modifier.padding(16.px)) {
            Text(user.name, fontSize = 18.px)
            Text(user.email, fontSize = 14.px, color = Color.Gray)
        }
    }
}
```

### Component with Children

```kotlin
@Composable
fun Section(
    title: String,
    content: @Composable () -> Unit
) {
    Section {
        H2 { Text(title) }
        content()
    }
}

// Usage
Section(title = "Features") {
    FeatureList(features)
}
```

### Slot APIs

```kotlin
@Composable
fun Card(
    header: @Composable () -> Unit = {},
    footer: @Composable () -> Unit = {},
    content: @Composable () -> Unit
) {
    Div(Modifier.className("card")) {
        Div(Modifier.className("card-header")) { header() }
        Div(Modifier.className("card-body")) { content() }
        Div(Modifier.className("card-footer")) { footer() }
    }
}

// Usage
Card(
    header = { Text("Title") },
    footer = { Button { Text("Action") } }
) {
    Text("Card content")
}
```

## Styling with Modifiers

### Modifier Basics

```kotlin
import org.jetbrains.compose.web.css.*

Box(
    Modifier
        .fillMaxWidth()           // width: 100%
        .height(200.px)           // height: 200px
        .padding(16.px)           // padding: 16px
        .margin(8.px)             // margin: 8px
        .backgroundColor(Color.Blue)
        .border(1.px, LineStyle.Solid, Color.Black)
        .borderRadius(8.px)
)
```

### Custom CSS Classes

```kotlin
// Define styles
object Style {
    val primaryButton = CssStyle {
        backgroundColor(Color.Blue)
        color(Color.White)
        padding(12.px, 24.px)
        borderRadius(4.px)
        fontSize(16.px)
    }
    
    val card = CssStyle {
        backgroundColor(Color.White)
        boxShadow(Color.Black.opacity(0.1), 0.px, 4.px, 16.px)
        borderRadius(8.px)
    }
}

// Apply styles
Button(
    onClick = { },
    attrs = {
        modifier(Style.primaryButton)
    }
) {
    Text("Primary Action")
}
```

### Responsive Design

```kotlin
val responsiveStyle = CssStyle {
    width(100.percent)
    
    // Tablet
    @MediaQuery("(min-width: 768px)") {
        width(50.percent)
    }
    
    // Desktop
    @MediaQuery("(min-width: 1024px)") {
        width(33.percent)
    }
}
```

### Extending Modifiers

```kotlin
// Extension function for common patterns
fun Modifier.cardStyle(): Modifier = then(
    Modifier
        .backgroundColor(Color.White)
        .borderRadius(8.px)
        .boxShadow(Color.Black.opacity(0.1), 0.px, 4.px, 16.px)
        .padding(16.px)
)

// Usage
Box(Modifier.cardStyle()) { ... }
```

## State Management

### Local State

```kotlin
@Composable
fun Counter() {
    var count by remember { mutableStateOf(0) }
    
    Column {
        Text("Count: $count")
        Button(onClick = { count++ }) {
            Text("Increment")
        }
    }
}
```

### State Hoisting

```kotlin
// Stateful component (convenient)
@Composable
fun SearchBox() {
    var query by remember { mutableStateOf("") }
    SearchBox(query = query, onQueryChange = { query = it })
}

// Stateless component (testable, reusable)
@Composable
fun SearchBox(
    query: String,
    onQueryChange: (String) -> Unit
) {
    Input(
        value = query,
        onValueChangedEvent = { onQueryChange(it.value) }
    )
}
```

### Remember with Saveable

```kotlin
@Composable
fun FormWithState() {
    // Survives configuration changes (in full-stack mode)
    var formData by rememberSaveable { 
        mutableStateOf(FormData()) 
    }
    
    // Form inputs...
}
```

### Derived State

```kotlin
@Composable
fun FilteredList(items: List<Item>, filter: String) {
    // Compute filtered list efficiently
    val filteredItems by remember(items, filter) {
        derivedStateOf {
            items.filter { it.name.contains(filter, ignoreCase = true) }
        }
    }
    
    // Render filteredItems...
}
```

## Layouts

### Basic Layout

```kotlin
@Layout
@Composable
fun DefaultLayout(
    ctx: LayoutContext,
    content: @Composable () -> Unit
) {
    Column(Modifier.fillMaxHeight()) {
        Header()
        Box(Modifier.weight(1f)) { content() }
        Footer()
    }
}
```

### Nested Layouts

```kotlin
// Root layout
@Layout("root")
@Composable
fun RootLayout(ctx: LayoutContext, content: @Composable () -> Unit) {
    HtmlDocument { content() }
}

// Blog layout (nested)
@Layout("blog")
@Composable
fun BlogLayout(ctx: LayoutContext, content: @Composable () -> Unit) {
    Column {
        BlogHeader()
        content()
    }
}

// Layout file determines inheritance
// pages/blog/Index.kt uses "blog" layout which uses "root" layout
```

### Layout Context

```kotlin
@Layout
@Composable
fun LayoutWithNav(ctx: LayoutContext, content: @Composable () -> Unit) {
    Row {
        Sidebar(
            currentRoute = ctx.route.path,
            onNavigate = { ctx.router.navigateTo(it) }
        )
        MainContent(Modifier.weight(1f)) { content() }
    }
}
```

## Navigation

### Programmatic Navigation

```kotlin
@Composable
fun NavigationExample() {
    val router = Router.current
    
    Column {
        Button(onClick = { router.navigateTo("/about") }) {
            Text("Go to About")
        }
        
        Button(onClick = { 
            router.navigateTo("/users/123", UpdateHistoryMode.REPLACE)
        }) {
            Text("Go to User 123 (replace history)")
        }
    }
}
```

### Navigation with Parameters

```kotlin
@Composable
fun UserList(users: List<User>) {
    val router = Router.current
    
    users.forEach { user ->
        TextLink(
            text = user.name,
            href = "/users/${user.id}"
        )
    }
}
```

### Route Interceptors

```kotlin
// In application initialization
router.addRouteInterceptor {
    // Check authentication before navigating
    if (path.startsWith("/admin") && !isLoggedIn()) {
        navigateTo("/login")
        preventDefault()
    }
}
```

### Link Components

```kotlin
// Standard link (uses client-side routing)
A(href = "/about") { Text("About") }

// External link (full page reload)
A(href = "https://external.com", target = "_blank") { 
    Text("External") 
}
```

## API Routes

### Basic API Route

```kotlin
// src/jvmMain/kotlin/myapp/api/users/UserApi.kt
package myapp.api.users

import com.varabyte.kobweb.api.*

@Api("users/fetch")
suspend fun fetchUser(ctx: ApiContext) {
    val userId = ctx.pathParams["userId"] ?: run {
        ctx.respondError("Missing userId")
        return
    }
    
    val user = database.getUser(userId)
    if (user != null) {
        ctx.respond(user)
    } else {
        ctx.respondError("User not found", 404)
    }
}
```

### HTTP Methods

```kotlin
import io.ktor.http.*

// GET (default)
@Api("users/list")
suspend fun listUsers(ctx: ApiContext) {
    ctx.respond(database.getAllUsers())
}

// POST
@Api(value = "users/create", method = HttpMethod.Post)
suspend fun createUser(ctx: ApiContext) {
    val userData = ctx.bodyAs<UserData>()
    val user = database.createUser(userData)
    ctx.respond(user)
}

// PUT
@Api(value = "users/update", method = HttpMethod.Put)
suspend fun updateUser(ctx: ApiContext) {
    val userData = ctx.bodyAs<UserData>()
    val user = database.updateUser(userData)
    ctx.respond(user)
}

// DELETE
@Api(value = "users/delete", method = HttpMethod.Delete)
suspend fun deleteUser(ctx: ApiContext) {
    val userId = ctx.pathParams["userId"]
    database.deleteUser(userId)
    ctx.respondSuccess()
}
```

### API Interceptors

```kotlin
// Global API interceptor
class AuthApiInterceptor : ApiInterceptor {
    override suspend fun intercept(ctx: ApiContext, next: () -> Unit) {
        val token = ctx.headers["Authorization"]?.removePrefix("Bearer ")
        
        if (token == null || !isValidToken(token)) {
            ctx.respondError("Unauthorized", 401)
            return
        }
        
        next()
    }
}
```

## Static Export

### Export Command

```bash
# Export static site
kobweb export -PkobwebExportLayout=STATIC

# Output: build/dist/js/site/
```

### Export Configuration

```kotlin
// build.gradle.kts
kobweb {
    export {
        // Define dynamic route values for export
        dynamicRouteValues.put("users/[userId]", listOf(
            mapOf("userId" to "1"),
            mapOf("userId" to "2"),
            mapOf("userId" to "3"),
        ))
    }
}
```

### Handling Export in Code

```kotlin
@Composable
fun PageWithClientOnlyFeature() {
    val isExporting = PageContext.current.isExporting
    
    // Skip client-only code during export
    if (!isExporting) {
        ClientOnlyFeature()
    }
    
    // Always render this
    StaticContent()
}
```

### Hydration

```kotlin
// State is preserved after hydration
@Composable
fun HydratedComponent() {
    // Initial state rendered on server
    // State becomes interactive after JS loads
    var count by remember { mutableStateOf(0) }
    
    Text("Count: $count")  // Clickable after hydration
}
```

## Full-Stack Mode

### Run Full-Stack Server

```bash
# Development
kobweb run

# Production export (full-stack)
kobweb export -PkobwebExportLayout=FULLSTACK
```

### Server Configuration

```kotlin
// src/jvmMain/kotlin/myapp/ServerConfig.kt
package myapp

import com.varabyte.kobweb.server.*
import io.ktor.server.*

class MyServerPlugin : KobwebServerPlugin {
    override fun Application.install() {
        // Install Ktor features
        install(ContentNegotiation) {
            json()
        }
        
        install(CORS) {
            anyHost()
        }
    }
}
```

### Database Integration

```kotlin
// src/jvmMain/kotlin/myapp/Database.kt
package myapp

import org.jetbrains.exposed.sql.*

object Users : Table() {
    val id = varchar("id", 36).primaryKey()
    val name = varchar("name", 100)
    val email = varchar("email", 255)
}

class Database {
    private val db = Database.connect(/* connection */)
    
    fun getUser(id: String): User? {
        return Users.select { Users.id eq id }.singleOrNull()?.let {
            User(it[Users.id], it[Users.name], it[Users.email])
        }
    }
}
```

## Silk UI Library

### Installation

```kotlin
// gradle/libs.versions.toml
[libraries]
kobweb-silk = { module = "com.varabyte.kobweb:kobweb-silk", version.ref = "kobweb" }

// build.gradle.kts
dependencies {
    implementation(libs.kobweb.silk)
}
```

### Basic Silk Components

```kotlin
import com.varabyte.kobweb.silk.components.*
import com.varabyte.kobweb.silk.theme.*

@Page
@Composable
fun SilkExample() {
    SilkApp {  // Required wrapper for Silk
        Column {
            // Buttons
            Button(onClick = { }) {
                Text("Primary")
            }
            
            Button(
                variant = ButtonVariant.OUTLINE,
                onClick = { }
            ) {
                Text("Outline")
            }
            
            // Input
            var text by remember { mutableStateOf("") }
            Input(
                value = text,
                onValueChangedEvent = { text = it.value },
                placeholder = "Enter text..."
            )
            
            // Card
            Card {
                CardBody {
                    Text("Card content")
                }
            }
            
            // Modal
            var showModal by remember { mutableStateOf(false) }
            if (showModal) {
                Modal(onClose = { showModal = false }) {
                    Text("Modal content")
                }
            }
            Button(onClick = { showModal = true }) {
                Text("Open Modal")
            }
        }
    }
}
```

### Theme Customization

```kotlin
// src/jsMain/kotlin/myapp/styles/Theme.kt
package myapp.styles

import com.varabyte.kobweb.silk.theme.*

@InitKobweb
fun initTheme() {
    SilkTheme.registerCustomTheme {
        // Override colors
        colors.put("primary", "#3B82F6")
        colors.put("secondary", "#10B981")
        
        // Override spacing
        spacing.put("sm", 8.px)
        spacing.put("md", 16.px)
        spacing.put("lg", 24.px)
    }
}
```

## Deployment

### Static Hosting

```bash
# Export static site
kobweb export -PkobwebExportLayout=STATIC

# Deploy to any static host
# Output in build/dist/js/site/
```

### Vercel Deployment

```yaml
# vercel.json
{
  "buildCommand": "kobweb export -PkobwebExportLayout=STATIC",
  "outputDirectory": "build/dist/js/site",
  "devCommand": "kobweb run",
  "installCommand": "chmod +x gradlew && ./gradlew build"
}
```

### Docker Deployment (Full-Stack)

```dockerfile
FROM gradle:8-jdk17 AS builder

WORKDIR /app
COPY . .
RUN gradle kobwebExport -PkobwebExportLayout=FULLSTACK

FROM eclipse-temurin:17-jre-alpine

WORKDIR /app
COPY --from=builder /app/build/dist/js/site .

EXPOSE 8080
CMD ["java", "-jar", "server.jar"]
```

### Production Checklist

1. [ ] Set `NODE_ENV=production`
2. [ ] Configure database connection
3. [ ] Set up SSL/TLS
4. [ ] Configure CORS for production domain
5. [ ] Enable compression
6. [ ] Set up logging
7. [ ] Configure health check endpoints
8. [ ] Set up monitoring (metrics, tracing)

## Conclusion

Kobweb provides:

1. **Kotlin-First Development**: Write full-stack web apps in pure Kotlin
2. **Compose HTML**: Declarative UI with familiar Compose patterns
3. **File-based Routing**: Next.js-style automatic routing
4. **SSR Export**: Static site generation with hydration
5. **Full-Stack Capabilities**: Ktor server with API routes
6. **Silk UI**: Production-ready component library
7. **KSP Processing**: Compile-time code generation

## Next Steps

- Deep dive into KSP processing and route generation
- Advanced Compose HTML patterns
- Silk UI customization
- API Streams for real-time features
- Production deployment strategies
