---
source: /home/darkvoid/Boxxed/@formulas/src.Kobweb
repository: github.com/varabyte/kobweb
explored_at: 2026-04-05
focus: KSP annotation processing, compile-time route generation, page discovery, API endpoint registration
---

# Deep Dive: KSP Processing and Routing

## Overview

This deep dive examines how Kobweb uses Kotlin Symbol Processing (KSP) to analyze `@Page`, `@Api`, and `@Layout` annotations at compile time, generating route registration code without runtime reflection overhead.

## KSP Architecture

```mermaid
flowchart LR
    A[Kotlin Source Files] --> B[KSP Processor]
    B --> C[Symbol Resolution]
    C --> D[Annotation Analysis]
    D --> E[Code Generation]
    E --> F[Generated Kotlin Files]
    F --> G[Kotlin Compiler]
    G --> H[JS + JVM Output]
```

## Annotation Definitions

### @Page Annotation

```kotlin
// kobweb-core/src/commonMain/kotlin/com/varabyte/kobweb/core/Page.kt

@Target(AnnotationTarget.FUNCTION)
@Retention(AnnotationRetention.BINARY)
annotation class Page(
    val value: String = "",  // Optional route override
)
```

### @Layout Annotation

```kotlin
// kobweb-core/src/commonMain/kotlin/com/varabyte/kobweb/core/Layout.kt

@Target(AnnotationTarget.FUNCTION)
@Retention(AnnotationRetention.BINARY)
annotation class Layout(
    val value: String = "",  // Optional layout name
)
```

### @Api Annotation

```kotlin
// kobweb-api/src/commonMain/kotlin/com/varabyte/kobweb/api/Api.kt

@Target(AnnotationTarget.FUNCTION)
@Retention(AnnotationRetention.BINARY)
annotation class Api(
    val value: String = "",  // Route path
    val method: HttpMethod = HttpMethod.Get,
)
```

## KSP Processor Implementation

### Page Processor

```kotlin
// gradle-plugins/src/main/kotlin/com/varabyte/kobweb/gradle/ksp/PageProcessor.kt

class PageProcessor(
    private val codeGenerator: CodeGenerator,
    private val logger: KSPLogger
) : SymbolProcessor {
    
    override fun process(resolver: Resolver): List<KSAnnotated> {
        // Find all @Page annotated functions
        val pageSymbols = resolver.getSymbolsWithAnnotation(
            Page::class.qualifiedName!!
        )
        
        val pages = mutableListOf<PageSymbol>()
        
        for (symbol in pageSymbols) {
            if (symbol !is KSFunctionDeclaration) {
                logger.error("@Page must be on function", symbol)
                continue
            }
            
            // Validate function signature
            if (!symbol.returnType.resolve().isUnit()) {
                logger.error("@Page function must return Unit", symbol)
                continue
            }
            
            // Check for @Composable annotation
            val isComposable = symbol.annotations.any {
                it.annotationType.resolve().declaration.qualifiedName
                    ?.asString() == "androidx.compose.runtime.Composable"
            }
            
            if (!isComposable) {
                logger.error("@Page function must be @Composable", symbol)
                continue
            }
            
            // Extract page metadata
            val pageAnnotation = symbol.annotations.find {
                it.annotationType.resolve().declaration.qualifiedName
                    ?.asString() == Page::class.qualifiedName!!
            }
            
            val routeOverride = pageAnnotation
                ?.arguments
                ?.find { it.name?.asString() == "value" }
                ?.value as? String
            
            // Get file path for route calculation
            val filePath = symbol.containingFile?.filePath ?: continue
            val packageName = symbol.packageName.asString()
            
            pages.add(PageSymbol(
                function = symbol,
                routeOverride = routeOverride,
                filePath = filePath,
                packageName = packageName,
            ))
        }
        
        // Generate route registration code
        generateRouteCode(pages)
        
        return emptyList()
    }
    
    private fun generateRouteCode(pages: List<PageSymbol>) {
        codeGenerator.createNewFile(
            dependencies = Dependencies.ALL_FILES,
            packageName = "kobweb.generated",
            fileName = "RouteRegistry",
        ).writer().use { writer ->
            writer.write("""
                package kobweb.generated
                
                import com.varabyte.kobweb.core.Router
                import com.varabyte.kobweb.core.PageContext
                
                fun Router.registerAllPages() {
            """.trimIndent())
            
            for (page in pages) {
                val route = calculateRoute(page)
                val layoutId = calculateLayoutId(page)
                val functionName = page.function.simpleName.asString()
                val packageName = page.function.packageName.asString()
                
                writer.write("""
                    
                    // ${page.filePath}
                    register("$route", layoutId = "$layoutId") { ctx ->
                        $packageName.$functionName(ctx)
                    }
                """.trimIndent())
            }
            
            writer.write("\n}\n")
        }
    }
    
    private fun calculateRoute(symbol: PageSymbol): String {
        // If route override exists, use it
        if (!symbol.routeOverride.isNullOrBlank()) {
            return symbol.routeOverride
        }
        
        // Calculate route from file path
        val relativePath = symbol.filePath
            .removePrefix("src/jsMain/kotlin/")
            .removePrefix(symbol.packageName.replace('.', '/') + "/")
            .removePrefix("pages/")
            .removeSuffix(".kt")
        
        // Convert path segments
        val segments = relativePath.split("/")
            .filter { it.isNotBlank() }
            .map { segment ->
                // Handle dynamic routes: [userId] -> {userId}
                if (segment.startsWith("[") && segment.endsWith("]")) {
                    "{${segment.removeSurrounding("[", "]")}}"
                } else {
                    // Convert PascalCase to kebab-case
                    segment.toKebabCase()
                }
            }
        
        // Handle Index.kt specially
        val routeSegments = segments.filter { it != "index" }
        
        return "/" + routeSegments.joinToString("/")
    }
}
```

### Layout Processor

```kotlin
// gradle-plugins/src/main/kotlin/com/varabyte/kobweb/gradle/ksp/LayoutProcessor.kt

class LayoutProcessor(
    private val codeGenerator: CodeGenerator,
    private val logger: KSPLogger
) : SymbolProcessor {
    
    override fun process(resolver: Resolver): List<KSAnnotated> {
        val layoutSymbols = resolver.getSymbolsWithAnnotation(
            Layout::class.qualifiedName!!
        )
        
        val layouts = mutableMapOf<String, LayoutSymbol>()
        
        for (symbol in layoutSymbols) {
            if (symbol !is KSFunctionDeclaration) {
                logger.error("@Layout must be on function", symbol)
                continue
            }
            
            val layoutAnnotation = symbol.annotations.find {
                it.annotationType.resolve().declaration.qualifiedName
                    ?.asString() == Layout::class.qualifiedName!!
            }
            
            val layoutName = layoutAnnotation
                ?.arguments
                ?.find { it.name?.asString() == "value" }
                ?.value as? String
                ?: calculateLayoutName(symbol)
            
            // Validate LayoutContext parameter
            val hasLayoutContext = symbol.parameters.any {
                it.type.resolve().declaration.qualifiedName
                    ?.asString() == "com.varabyte.kobweb.core.LayoutContext"
            }
            
            if (!hasLayoutContext) {
                logger.error("@Layout function must have LayoutContext parameter", symbol)
                continue
            }
            
            layouts[layoutName] = LayoutSymbol(
                function = symbol,
                name = layoutName,
            )
        }
        
        generateLayoutCode(layouts)
        
        return emptyList()
    }
    
    private fun calculateLayoutName(symbol: KSFunctionDeclaration): String {
        // Default layout naming from filename
        val fileName = symbol.containingFile?.fileName ?: "default"
        return fileName.removeSuffix(".kt").toKebabCase()
    }
    
    private fun generateLayoutCode(layouts: Map<String, LayoutSymbol>) {
        codeGenerator.createNewFile(
            dependencies = Dependencies.ALL_FILES,
            packageName = "kobweb.generated",
            fileName = "LayoutRegistry",
        ).writer().use { writer ->
            writer.write("""
                package kobweb.generated
                
                import com.varabyte.kobweb.core.LayoutContext
                import androidx.compose.runtime.Composable
                
                object LayoutRegistry {
            """.trimIndent())
            
            for ((name, layout) in layouts) {
                val functionName = layout.function.simpleName.asString()
                val packageName = layout.function.packageName.asString()
                
                writer.write("""
                    
                    @Composable
                    fun "$name"(ctx: LayoutContext, content: @Composable () -> Unit) {
                        $packageName.$functionName(ctx, content)
                    }
                """.trimIndent())
            }
            
            writer.write("\n}\n")
        }
    }
}
```

### API Processor

```kotlin
// gradle-plugins/src/main/kotlin/com/varabyte/kobweb/gradle/ksp/ApiProcessor.kt

class ApiProcessor(
    private val codeGenerator: CodeGenerator,
    private val logger: KSPLogger
) : SymbolProcessor {
    
    override fun process(resolver: Resolver): List<KSAnnotated> {
        val apiSymbols = resolver.getSymbolsWithAnnotation(
            Api::class.qualifiedName!!
        )
        
        val apis = mutableListOf<ApiSymbol>()
        
        for (symbol in apiSymbols) {
            if (symbol !is KSFunctionDeclaration) {
                logger.error("@Api must be on function or property", symbol)
                continue
            }
            
            val apiAnnotation = symbol.annotations.find {
                it.annotationType.resolve().declaration.qualifiedName
                    ?.asString() == Api::class.qualifiedName!!
            }
            
            val routePath = apiAnnotation
                ?.arguments
                ?.find { it.name?.asString() == "value" }
                ?.value as? String
                ?: calculateApiRoute(symbol)
            
            val httpMethod = apiAnnotation
                ?.arguments
                ?.find { it.name?.asString() == "method" }
                ?.value as? HttpMethod
                ?: HttpMethod.Get
            
            // Validate ApiContext parameter
            val hasApiContext = symbol.parameters.any {
                it.type.resolve().declaration.qualifiedName
                    ?.asString() == "com.varabyte.kobweb.api.ApiContext"
            }
            
            if (!hasApiContext) {
                logger.error("@Api function must have ApiContext parameter", symbol)
                continue
            }
            
            apis.add(ApiSymbol(
                function = symbol,
                route = routePath,
                method = httpMethod,
            ))
        }
        
        generateApiCode(apis)
        
        return emptyList()
    }
    
    private fun calculateApiRoute(symbol: KSFunctionDeclaration): String {
        // Calculate API route from package structure
        val packageName = symbol.packageName.asString()
        val functionName = symbol.simpleName.asString()
        
        // Convert package to route path
        val packagePath = packageName
            .removePrefix("api.")
            .replace('.', '/')
        
        return "$packagePath/${functionName.toKebabCase()}"
    }
    
    private fun generateApiCode(apis: List<ApiSymbol>) {
        codeGenerator.createNewFile(
            dependencies = Dependencies.ALL_FILES,
            packageName = "kobweb.generated",
            fileName = "ApiRegistry",
        ).writer().use { writer ->
            writer.write("""
                package kobweb.generated
                
                import com.varabyte.kobweb.api.ApiContext
                import com.varabyte.kobweb.api.HttpMethod
                import io.ktor.http.*
                
                object ApiRegistry {
                    val routes = mapOf<String, ApiHandler>()
                    
                    init {
            """.trimIndent())
            
            for (api in apis) {
                val functionName = api.function.simpleName.asString()
                val packageName = api.function.packageName.asString()
                val fullRoute = "${api.method.value.lowercase()}:${api.route}"
                
                writer.write("""
                        
                        routes["$fullRoute"] = ApiHandler(
                            method = HttpMethod.${api.method},
                            path = "${api.route}",
                            handler = { ctx ->
                                $packageName.$functionName(ctx)
                            }
                        )
                """.trimIndent())
            }
            
            writer.write("""
                    }
                }
            """.trimIndent())
        }
    }
}
```

## Route Calculation Algorithm

### File Path to Route Conversion

```kotlin
// Utility functions for route calculation

fun calculateRouteFromPath(
    filePath: String,
    basePackage: String,
    pagesDir: String = "pages"
): String {
    // Example: src/jsMain/kotlin/com/example/pages/users/[userId]/Profile.kt
    
    // 1. Remove source directory prefix
    val relativePath = filePath
        .removePrefix("src/jsMain/kotlin/")
        .removePrefix(basePackage.replace('.', '/') + "/")
        .removePrefix("$pagesDir/")
        .removeSuffix(".kt")
    
    // Result: users/[userId]/Profile
    
    // 2. Split into segments
    val segments = relativePath.split("/")
    
    // 3. Process each segment
    val routeSegments = segments.mapIndexed { index, segment ->
        when {
            // Index.kt represents the directory index
            segment.equals("Index", ignoreCase = true) -> null
            
            // Dynamic segments: [userId] -> {userId}
            segment.startsWith("[") && segment.endsWith("]") -> {
                "{${segment.removeSurrounding("[", "]")}}"
            }
            
            // Catch-all: {...} -> {...}
            segment.startsWith("{...}") -> "{...}"
            
            // Named dynamic: {userId} -> {userId}
            segment.startsWith("{") && segment.endsWith("}") -> segment
            
            // Regular segment: PascalCase to kebab-case
            else -> segment.toKebabCase()
        }
    }.filterNotNull()
    
    // 4. Join with slashes
    return "/" + routeSegments.joinToString("/")
}

fun String.toKebabCase(): String {
    return fold(StringBuilder()) { acc, char ->
        if (char.isUpperCase()) {
            if (acc.isNotEmpty()) acc.append('-')
            acc.append(char.lowercaseChar())
        } else {
            acc.append(char)
        }
    }.toString()
}
```

### Route Override Handling

```kotlin
fun handleRouteOverride(
    annotation: KSAnnotation,
    defaultRoute: String
): String {
    val overrideValue = annotation.arguments
        .find { it.name?.asString() == "value" }
        ?.value as? String
    
    return when {
        // No override
        overrideValue.isNullOrBlank() -> defaultRoute
        
        // Absolute path override
        overrideValue.startsWith("/") -> overrideValue
        
        // Relative override (replace slug)
        else -> {
            val parentPath = defaultRoute.substringBeforeLast("/")
            "$parentPath/$overrideValue"
        }
    }
}
```

## Generated Code Example

### Input Source Files

```kotlin
// src/jsMain/kotlin/pages/Index.kt
package com.example.pages

import com.varabyte.kobweb.core.*
import androidx.compose.runtime.*

@Page
@Composable
fun IndexPage() {
    Text("Home Page")
}

// src/jsMain/kotlin/pages/about/Index.kt
package com.example.pages.about

@Page
@Composable
fun AboutPage() {
    Text("About Page")
}

// src/jsMain/kotlin/pages/users/[userId]/Profile.kt
package com.example.pages.users.profile

@Page
@Composable
fun UserProfilePage(ctx: PageContext) {
    val userId = ctx.route.params["userId"]
    Text("User Profile: $userId")
}

// src/jsMain/kotlin/layouts/DefaultLayout.kt
package com.example.layouts

import com.varabyte.kobweb.core.*

@Layout
@Composable
fun DefaultLayout(ctx: LayoutContext, content: @Composable () -> Unit) {
    Column {
        Header()
        content()
        Footer()
    }
}
```

### Generated RouteRegistry.kt

```kotlin
// build/generated/ksp/js/main/kotlin/kobweb/generated/RouteRegistry.kt

package kobweb.generated

import com.varabyte.kobweb.core.Router
import com.varabyte.kobweb.core.PageContext
import com.example.pages.IndexPage
import com.example.pages.about.AboutPage
import com.example.pages.users.profile.UserProfilePage

fun Router.registerAllPages() {
    // src/jsMain/kotlin/pages/Index.kt
    register("/", layoutId = "default") { ctx ->
        IndexPage(ctx)
    }
    
    // src/jsMain/kotlin/pages/about/Index.kt
    register("/about", layoutId = "default") { ctx ->
        AboutPage(ctx)
    }
    
    // src/jsMain/kotlin/pages/users/[userId]/Profile.kt
    register("/users/{userId}", layoutId = "default") { ctx ->
        UserProfilePage(ctx)
    }
}
```

### Generated LayoutRegistry.kt

```kotlin
// build/generated/ksp/js/main/kotlin/kobweb/generated/LayoutRegistry.kt

package kobweb.generated

import com.varabyte.kobweb.core.LayoutContext
import androidx.compose.runtime.Composable
import com.example.layouts.DefaultLayout

object LayoutRegistry {
    @Composable
    fun "default"(ctx: LayoutContext, content: @Composable () -> Unit) {
        DefaultLayout(ctx, content)
    }
}
```

### Generated ApiRegistry.kt

```kotlin
// build/generated/ksp/jvm/main/kotlin/kobweb/generated/ApiRegistry.kt

package kobweb.generated

import com.varabyte.kobweb.api.ApiContext
import com.varabyte.kobweb.api.HttpMethod
import com.example.api.users.fetchUser
import com.example.api.users.createUser

object ApiRegistry {
    val routes = mapOf<String, ApiHandler>()
    
    init {
        routes["get:users/fetch"] = ApiHandler(
            method = HttpMethod.Get,
            path = "users/fetch",
            handler = { ctx -> fetchUser(ctx) }
        )
        
        routes["post:users/create"] = ApiHandler(
            method = HttpMethod.Post,
            path = "users/create",
            handler = { ctx -> createUser(ctx) }
        )
    }
}
```

## Layout Inheritance

```kotlin
// Layout hierarchy calculation

data class LayoutNode(
    val name: String,
    val parent: String? = null,
    val children: MutableList<String> = mutableListOf()
)

fun buildLayoutHierarchy(files: List<KSFile>): Map<String, LayoutNode> {
    val layouts = mutableMapOf<String, LayoutNode>()
    
    for (file in files) {
        // Determine layout name from file path
        val relativePath = file.relativePath
        val segments = relativePath.split("/")
        
        // Each directory level can have a layout
        var parentLayout: String? = null
        var currentPath = ""
        
        for (segment in segments.dropLast(1)) {  // Exclude filename
            currentPath = "$currentPath/$segment"
            val layoutName = segment.toKebabCase()
            
            layouts.getOrPut(layoutName) {
                LayoutNode(layoutName, parentLayout)
            }.also { node ->
                parentLayout?.let { parent ->
                    layouts[parent]?.children?.add(layoutName)
                }
            }
            
            parentLayout = layoutName
        }
    }
    
    return layouts
}
```

## Error Handling

```kotlin
// Validation errors reported during KSP processing

class PageValidationErrors(
    private val logger: KSPLogger,
    private val symbol: KSAnnotated
) {
    fun reportMissingComposable() {
        logger.error("@Page function must be annotated with @Composable", symbol)
    }
    
    fun reportInvalidReturnType() {
        logger.error("@Page function must return Unit", symbol)
    }
    
    fun reportMissingPageContext() {
        logger.error("@Page function using route params must have PageContext parameter", symbol)
    }
    
    fun reportDuplicateRoute(route: String) {
        logger.error("Duplicate route: $route", symbol)
    }
    
    fun reportInvalidDynamicSegment() {
        logger.error("Dynamic segments must be in format [name]", symbol)
    }
}
```

## Performance Considerations

### Incremental Processing

```kotlin
// KSP supports incremental processing

class PageProcessor(
    private val codeGenerator: CodeGenerator,
    private val logger: KSPLogger
) : SymbolProcessor {
    
    override fun process(resolver: Resolver): List<KSAnnotated> {
        // Only process changed files
        val changedFiles = resolver.getNewFiles()
        
        // Regenerate only affected routes
        // ...
        
        return emptyList()
    }
}
```

### Caching Generated Code

```kotlin
// Gradle plugin caches generated code

class KobwebGradlePlugin : Plugin<Project> {
    override fun apply(target: Project) {
        val cacheDir = target.layout.buildDirectory.dir("ksp-cache")
        
        // Cache generated RouteRegistry, LayoutRegistry, ApiRegistry
        // Only regenerate if source files changed
    }
}
```

## Conclusion

Kobweb's KSP processing provides:

1. **Compile-Time Route Generation**: No runtime reflection overhead
2. **Type Safety**: Compiler validates page signatures
3. **Fast Startup**: Routes pre-registered at compile time
4. **Automatic Discovery**: No manual route registration needed
5. **Error Detection**: Route conflicts caught at compile time
6. **Incremental Builds**: Only regenerate changed routes

This approach mirrors Next.js's compile-time routing while leveraging Kotlin's powerful metaprogramming capabilities.
