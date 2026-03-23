# Rust Revision: Building MCP Apps UI in Rust

## Overview

This guide explains how to reproduce MCP Apps UI functionality in Rust. While the official SDKs are TypeScript/Python/Ruby-based, the protocol is language-agnostic JSON-RPC over postMessage.

## Architecture for Rust Implementation

### Two Components Needed

1. **Server-Side (Rust MCP Server)**
   - Generate HTML resources
   - Register tools with `_meta.ui`
   - Serve resource content via `resources/read`

2. **Client-Side (Rust Host)**
   - Render sandboxed iframes
   - Proxy JSON-RPC messages
   - Handle tool calls and resources

## Server-Side Implementation

### Using Rust MCP SDK

```rust
use mcp_core::{
    ServerCapabilities, Tool, Resource,
    types::{Content, TextContent, ResourceContent},
};
use mcp_server::{Server, Handler};
use serde_json::{json, Value};

// Define your MCP server
struct MyMcpServer {
    // Server state
}

impl Handler for MyMcpServer {
    async fn list_tools(&self) -> Result<Vec<Tool>, Error> {
        Ok(vec![
            Tool {
                name: "show_dashboard".to_string(),
                description: "Show interactive dashboard".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"}
                    }
                }),
                // MCP Apps metadata
                annotations: Some(json!({
                    "ui": {
                        "resourceUri": "ui://dashboard/view"
                    }
                })),
            }
        ])
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: &Value,
    ) -> Result<Vec<Content>, Error> {
        match name {
            "show_dashboard" => {
                let query = arguments["query"].as_str().unwrap_or("default");

                // Generate HTML for the dashboard
                let html = self.generate_dashboard_html(query)?;

                // Return text result + UI resource
                Ok(vec![
                    Content::Text(TextContent {
                        text: format!("Dashboard for: {}", query),
                    }),
                    Content::Resource(ResourceContent {
                        uri: "ui://dashboard/view".to_string(),
                        mime_type: "text/html;profile=mcp-app".to_string(),
                        text: Some(html),
                        blob: None,
                    }),
                ])
            }
            _ => Err(Error::ToolNotFound(name.to_string())),
        }
    }

    async fn read_resource(&self, uri: &str) -> Result<Resource, Error> {
        match uri {
            "ui://dashboard/view" => {
                let html = self.generate_dashboard_html("default")?;
                Ok(Resource {
                    uri: uri.to_string(),
                    name: "Dashboard View".to_string(),
                    mime_type: "text/html;profile=mcp-app".to_string(),
                    text: Some(html),
                    blob: None,
                    annotations: Some(json!({
                        "ui": {
                            "csp": {
                                "connectDomains": ["https://api.example.com"],
                                "resourceDomains": ["https://cdn.jsdelivr.net"]
                            }
                        }
                    })),
                })
            }
            _ => Err(Error::ResourceNotFound(uri.to_string())),
        }
    }
}

impl MyMcpServer {
    fn generate_dashboard_html(&self, query: &str) -> Result<String, Error> {
        Ok(format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Dashboard</title>
    <style>
        body {{
            font-family: system-ui, sans-serif;
            padding: 20px;
            background: var(--color-background-primary, #fff);
            color: var(--color-text-primary, #000);
        }}
    </style>
</head>
<body>
    <h1>Dashboard: {}</h1>
    <div id="data">Loading...</div>
    <button onclick="refreshData()">Refresh</button>

    <script>
        // Receive tool input from host
        window.addEventListener('message', (event) => {
            if (event.data.type === 'ui-lifecycle-iframe-render-data') {{
                const {{ toolInput, toolOutput }} = event.data.payload.renderData;
                document.getElementById('data').textContent =
                    JSON.stringify({{ toolInput, toolOutput }}, null, 2);
            }}
        }});

        // Signal ready
        window.parent.postMessage({{ type: 'ui-lifecycle-iframe-ready' }}, '*');

        // Call server tool
        async function refreshData() {{
            window.parent.postMessage({{
                type: 'tool',
                payload: {{
                    toolName: 'fetch_data',
                    params: {{ query: '{}' }}
                }}
            }}, '*');
        }}
    </script>
</body>
</html>"#,
            query, query
        ))
    }
}
```

### Using axum for HTTP-based MCP Server

```rust
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    // Shared state
}

#[derive(Deserialize)]
struct McpRequest {
    jsonrpc: String,
    method: String,
    #[serde(default)]
    id: Option<serde_json::Value>,
    #[serde(default)]
    params: serde_json::Value,
}

#[derive(Serialize)]
struct McpResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ErrorBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct ErrorBody {
    code: i32,
    message: String,
}

async fn handle_mcp(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(req): Json<McpRequest>,
) -> impl IntoResponse {
    match req.method.as_str() {
        "initialize" => Json(json!({
            "jsonrpc": "2.0",
            "id": req.id,
            "result": {
                "protocolVersion": "2026-01-26",
                "capabilities": {
                    "tools": {},
                    "resources": {}
                },
                "serverInfo": {
                    "name": "rust-mcp-server",
                    "version": "1.0.0"
                }
            }
        })),

        "tools/list" => Json(json!({
            "jsonrpc": "2.0",
            "id": req.id,
            "result": {
                "tools": [{
                    "name": "show_widget",
                    "description": "Show interactive widget",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": {"type": "string"}
                        }
                    },
                    "_meta": {
                        "ui": {
                            "resourceUri": "ui://widget/view"
                        }
                    }
                }]
            }
        })),

        "tools/call" => {
            let params: ToolCallParams = serde_json::from_value(req.params)
                .map_err(|e| McpResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(ErrorBody {
                        code: -32602,
                        message: e.to_string(),
                    }),
                    id: req.id,
                })?;

            match params.name.as_str() {
                "show_widget" => {
                    let html = generate_widget_html(&params.arguments);
                    Json(json!({
                        "jsonrpc": "2.0",
                        "id": req.id,
                        "result": {
                            "content": [
                                {
                                    "type": "text",
                                    "text": "Widget displayed"
                                },
                                {
                                    "type": "resource",
                                    "resource": {
                                        "uri": "ui://widget/view",
                                        "mimeType": "text/html;profile=mcp-app",
                                        "text": html
                                    }
                                }
                            ]
                        }
                    }))
                }
                _ => Json(json!({
                    "jsonrpc": "2.0",
                    "id": req.id,
                    "error": {
                        "code": -32601,
                        "message": "Tool not found"
                    }
                })),
            }
        }

        "resources/read" => {
            let params: ResourceReadParams = serde_json::from_value(req.params)
                .map_err(|_| McpResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(ErrorBody {
                        code: -32602,
                        message: "Invalid params".to_string(),
                    }),
                    id: req.id,
                })?;

            match params.uri.as_str() {
                "ui://widget/view" => {
                    let html = generate_widget_html(&serde_json::Map::new());
                    Json(json!({
                        "jsonrpc": "2.0",
                        "id": req.id,
                        "result": {
                            "contents": [{
                                "uri": params.uri,
                                "mimeType": "text/html;profile=mcp-app",
                                "text": html,
                                "_meta": {
                                    "ui": {
                                        "csp": {
                                            "connectDomains": ["https://api.example.com"]
                                        }
                                    }
                                }
                            }]
                        }
                    }))
                }
                _ => Json(json!({
                    "jsonrpc": "2.0",
                    "id": req.id,
                    "error": {
                        "code": -32001,
                        "message": "Resource not found"
                    }
                })),
            }
        }

        _ => Json(json!({
            "jsonrpc": "2.0",
            "id": req.id,
            "error": {
                "code": -32601,
                "message": "Method not found"
            }
        })),
    }
}

fn generate_widget_html(arguments: &serde_json::Value) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Widget</title>
    <style>
        :root {{
            --color-background-primary: light-dark(#ffffff, #171717);
            --color-text-primary: light-dark(#171717, #fafafa);
        }}
        body {{
            font-family: system-ui, sans-serif;
            padding: 20px;
            background: var(--color-background-primary);
            color: var(--color-text-primary);
        }}
        button {{
            padding: 8px 16px;
            border-radius: 6px;
            border: 1px solid var(--color-border-primary, #ccc);
            cursor: pointer;
        }}
    </style>
</head>
<body>
    <h1>Interactive Widget</h1>
    <p>Arguments: <code id="args">{arguments}</code></p>
    <button onclick="sendPrompt()">Send Follow-up</button>
    <button onclick="callTool()">Call Tool</button>

    <script>
        // Initialize MCP Apps protocol
        let nextId = 1;

        // Send initialize request
        function initialize() {{
            const id = nextId++;
            window.parent.postMessage({{
                jsonrpc: "2.0",
                id: id,
                method: "ui/initialize",
                params: {{
                    protocolVersion: "2026-01-26",
                    capabilities: {{}},
                    clientInfo: {{
                        name: "rust-widget",
                        version: "1.0.0"
                    }},
                    appCapabilities: {{
                        availableDisplayModes: ["inline"]
                    }}
                }}
            }}, '*');
        }}

        // Listen for responses
        window.addEventListener('message', (event) => {{
            const data = event.data;
            if (!data || !data.jsonrpc) return;

            if (data.method === 'ui/notifications/tool-input') {{
                console.log('Tool input:', data.params);
            }}
            if (data.method === 'ui/notifications/tool-result') {{
                console.log('Tool result:', data.params);
            }}
        }});

        // Send follow-up message
        function sendPrompt() {{
            const id = nextId++;
            window.parent.postMessage({{
                jsonrpc: "2.0",
                id: id,
                method: "ui/message",
                params: {{
                    role: "user",
                    content: [{{ type: "text", text: "Tell me more" }}]
                }}
            }}, '*');
        }}

        // Call another tool
        function callTool() {{
            const id = nextId++;
            window.parent.postMessage({{
                jsonrpc: "2.0",
                id: id,
                method: "tools/call",
                params: {{
                    name: "other_tool",
                    arguments: {{}}
                }}
            }}, '*');
        }}

        // Initialize on load
        initialize();

        // Signal ready (legacy MCP-UI)
        window.parent.postMessage({{ type: 'ui-lifecycle-iframe-ready' }}, '*');
    </script>
</body>
</html>"#
    )
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/mcp", post(handle_mcp))
        .with_state(Arc::new(Mutex::new(AppState {})));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

## Client-Side Implementation (Rust Host)

### Using Tauri for Desktop Host

```rust
// Tauri command to handle MCP communication
#[tauri::command]
async fn call_mcp_tool(
    window: Window,
    tool_name: String,
    arguments: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // Call MCP server
    let result = mcp_client::call_tool(&tool_name, &arguments)
        .await
        .map_err(|e| e.to_string())?;

    // Check if result contains UI resource
    for content in &result.content {
        if let Content::Resource(resource) = content {
            if resource.mime_type == "text/html;profile=mcp-app" {
                // Emit event to frontend with UI resource
                window.emit("mcp-ui-resource", resource).unwrap();
            }
        }
    }

    Ok(serde_json::to_value(result).unwrap())
}

// Frontend: Render UI resource in sandboxed iframe
fn render_ui_resource(app_handle: &AppHandle, resource: &Resource) {
    let main_window = app_handle.get_window("main").unwrap();

    // Create sandbox iframe
    let iframe_html = format!(
        r#"<iframe
            id="mcp-ui-frame"
            src="about:blank"
            sandbox="allow-scripts allow-same-origin allow-forms"
            style="width: 100%; height: 600px; border: none;"
        ></iframe>"#
    );

    main_window.eval(&iframe_html).unwrap();

    // Get iframe content and write HTML
    let write_html = format!(
        r#"
        const iframe = document.getElementById('mcp-ui-frame');
        const iframeDoc = iframe.contentDocument || iframe.contentWindow.document;
        iframeDoc.open();
        iframeDoc.write({html:?});
        iframeDoc.close();

        // Set up message relay
        window.addEventListener('message', (event) => {{
            const iframe = document.getElementById('mcp-ui-frame');
            if (iframe && iframe.contentWindow) {{
                iframe.contentWindow.postMessage(event.data, '*');
            }}
        }});

        // Relay messages from iframe to Rust
        iframe.addEventListener('message', (event) => {{
            window.__TAURI__.event.emit('mcp-ui-message', event.data);
        }});
        "#,
        html = resource.text.as_ref().unwrap()
    );

    main_window.eval(&write_html).unwrap();
}
```

### Wasm-based Host (Web)

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use web_sys::{window, HtmlIFrameElement, MessageEvent};

#[wasm_bindgen]
pub struct McpUiRenderer {
    iframe: HtmlIFrameElement,
    next_id: std::cell::Cell<u32>,
}

#[wasm_bindgen]
impl McpUiRenderer {
    pub fn new(sandbox_url: &str) -> Result<McpUiRenderer, JsValue> {
        let window = window().ok_or("No window")?;
        let document = window.document().ok_or("No document")?;

        let iframe = document
            .create_element("iframe")?
            .dyn_into::<HtmlIFrameElement>()?;

        iframe.set_sandbox("allow-scripts allow-same-origin allow-forms");
        iframe.set_src(sandbox_url);
        iframe.style().set_property("width", "100%").unwrap();
        iframe.style().set_property("height", "600px").unwrap();
        iframe.style().set_property("border", "none").unwrap();

        Ok(McpUiRenderer {
            iframe,
            next_id: std::cell::Cell::new(1),
        })
    }

    pub fn render(&self, html: &str) -> Result<(), JsValue> {
        let window = window().ok_or("No window")?;

        // Wait for iframe to load, then write HTML
        let iframe = self.iframe.clone();
        let html = html.to_string();

        let closure = Closure::once(move || {
            if let Some(doc) = iframe.content_document() {
                doc.write_with_str(&html).unwrap();
            }
        });

        self.iframe.add_event_listener_with_callback(
            "load",
            closure.as_ref().unchecked_ref()
        )?;

        closure.forget();

        // Set up message listener
        let closure = Closure::wrap(Box::new(move |event: MessageEvent| {
            let data = event.data();
            web_sys::console::log_1(&data);
        }) as Box<dyn FnMut(MessageEvent)>);

        window.add_event_listener_with_callback(
            "message",
            closure.as_ref().unchecked_ref()
        )?;

        closure.forget();

        Ok(())
    }

    pub fn send_message(&self, method: &str, params: &Value) -> Result<(), JsValue> {
        let window = window().ok_or("No window")?;
        let id = self.next_id.get();
        self.next_id.set(id + 1);

        let message = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        if let Some(iframe_window) = self.iframe.content_window() {
            iframe_window
                .post_message_with_target_origin(&message.into(), "*")
                .unwrap();
        }

        Ok(())
    }
}
```

## JSON-RPC Message Types

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

// MCP Apps specific types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiInitializeRequest {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(rename = "method")]
    pub method: String, // "ui/initialize"
    pub params: UiInitializeParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiInitializeParams {
    pub protocol_version: String,
    pub capabilities: Value,
    pub client_info: ClientInfo,
    #[serde(rename = "appCapabilities")]
    pub app_capabilities: AppCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
    #[serde(rename = "availableDisplayModes", skip_serializing_if = "Option::is_none")]
    pub available_display_modes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiInitializeResult {
    pub jsonrpc: String,
    pub id: Value,
    pub result: UiInitializeResultData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiInitializeResultData {
    pub protocol_version: String,
    pub capabilities: HostCapabilities,
    pub host_info: HostInfo,
    #[serde(rename = "hostContext")]
    pub host_context: HostContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub styles: Option<Styles>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_dimensions: Option<ContainerDimensions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Styles {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerDimensions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_height: Option<u32>,
}
```

## HTML Generation with Askama

For type-safe HTML generation, use Askama templates:

```rust
// templates/widget.html
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>{{ title }}</title>
    <style>
        :root {
            --color-background-primary: light-dark(#ffffff, #171717);
            --color-text-primary: light-dark(#171717, #fafafa);
        }
        body {
            font-family: system-ui, sans-serif;
            padding: 20px;
            background: var(--color-background-primary);
            color: var(--color-text-primary);
        }
    </style>
</head>
<body>
    <h1>{{ title }}</h1>
    <p>Query: {{ query }}</p>
    <button onclick="sendPrompt()">Ask Follow-up</button>

    <script>
        function sendPrompt() {
            window.parent.postMessage({
                jsonrpc: "2.0",
                id: 1,
                method: "ui/message",
                params: {
                    role: "user",
                    content: [{ type: "text", text: "Tell me more" }]
                }
            }, '*');
        }

        window.parent.postMessage({ type: 'ui-lifecycle-iframe-ready' }, '*');
    </script>
</body>
</html>
```

```rust
// In your Rust code
use askama::Template;

#[derive(Template)]
#[template(path = "widget.html")]
struct WidgetTemplate {
    title: String,
    query: String,
}

impl MyMcpServer {
    fn generate_widget_html(&self, query: &str) -> Result<String> {
        let template = WidgetTemplate {
            title: "Interactive Widget".to_string(),
            query: query.to_string(),
        };
        template.render().map_err(|e| Error::Template(e.to_string()))
    }
}
```

## Production Considerations

### 1. CSP Generation

```rust
fn build_csp_header(csp: &McpCsp) -> String {
    let mut directives = Vec::new();

    directives.push("default-src 'none'");
    directives.push("script-src 'self' 'unsafe-inline'");
    directives.push("style-src 'self' 'unsafe-inline'");
    directives.push("img-src 'self' data:");

    if !csp.connect_domains.is_empty() {
        directives.push(&format!("connect-src {}", csp.connect_domains.join(" ")));
    } else {
        directives.push("connect-src 'none'");
    }

    for domain in &csp.resource_domains {
        directives.push(&format!("script-src {}", domain));
        directives.push(&format!("style-src {}", domain));
        directives.push(&format!("img-src {}", domain));
    }

    directives.join("; ")
}
```

### 2. URL Validation for External URLs

```rust
use url::Url;
use std::net::IpAddr;

fn validate_external_url(url_str: &str) -> Result<Url, Error> {
    let url = Url::parse(url_str)?;

    // Must be http or https
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(Error::InvalidUrlScheme);
    }

    // Block private IPs
    if let Some(host) = url.host() {
        match host {
            url::Host::Domain(_) => {
                // Check for localhost
                if host.to_string() == "localhost" {
                    return Err(Error::PrivateNetworkBlocked);
                }
            }
            url::Host::Ipv4(ip) => {
                if ip.is_private() || ip.is_loopback() {
                    return Err(Error::PrivateNetworkBlocked);
                }
            }
            url::Host::Ipv6(ip) => {
                if ip.is_loopback() {
                    return Err(Error::PrivateNetworkBlocked);
                }
            }
        }
    }

    Ok(url)
}
```

### 3. Cargo Dependencies

```toml
[dependencies]
# Core MCP
mcp-core = "0.1"
mcp-server = "0.1"
mcp-client = "0.1"

# Web framework
axum = "0.7"
tokio = { version = "1", features = ["full"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Templates
askama = "0.12"

# URL handling
url = "2"

# For Wasm targets
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = [
    "Window",
    "Document",
    "HtmlIFrameElement",
    "MessageEvent"
]}

# Tauri (for desktop hosts)
tauri = "1"
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_list_tools_includes_ui_metadata() {
        let server = MyMcpServer {};
        let tools = server.list_tools().await.unwrap();

        let widget_tool = tools.iter()
            .find(|t| t.name == "show_widget")
            .expect("Widget tool not found");

        assert!(widget_tool.annotations.is_some());
        let annotations = widget_tool.annotations.as_ref().unwrap();
        assert_eq!(annotations["ui"]["resourceUri"], "ui://widget/view");
    }

    #[test]
    fn test_csp_generation() {
        let csp = McpCsp {
            connect_domains: vec!["https://api.example.com".to_string()],
            resource_domains: vec!["https://cdn.example.com".to_string()],
            frame_domains: vec![],
            base_uri_domains: vec![],
        };

        let header = build_csp_header(&csp);
        assert!(header.contains("connect-src https://api.example.com"));
        assert!(header.contains("script-src https://cdn.example.com"));
    }
}
```

## Related Resources

- [MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [Askama Templates](https://askama.cr/)
- [Tauri Framework](https://tauri.app/)
- [axum Web Framework](https://github.com/tokio-rs/axum)
