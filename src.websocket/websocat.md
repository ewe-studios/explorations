# Websocat: WebSocket CLI Tool

## Overview

Websocat is a versatile command-line WebSocket client and server, often described as "netcat/curl/socat for WebSockets". It provides extensive functionality for WebSocket testing, debugging, and proxying.

**Key Features:**
- Connect to and serve WebSockets from CLI
- Protocol bridging (WebSocket ↔ TCP/UDP/stdio/process)
- Text and binary modes
- Auto-reconnect and connection reuse
- Broadcast and multiplexing capabilities
- SOCKS5 proxy support
- TLS/SSL support

## Architecture

```
websocat/
├── src/
│   ├── main.rs             # CLI entry point, option parsing
│   ├── lib.rs              # Library exports
│   ├── options.rs          # Command-line options
│   ├── specparse.rs        # Address specifier parser
│   ├── specifier.rs        # Address type definitions
│   ├── sessionserve.rs     # Session handling
│   ├── my_copy.rs          # Bidirectional data copying
│   ├── net_peer.rs         # TCP networking
│   ├── ssl_peer.rs         # TLS support
│   ├── http_peer.rs        # HTTP connections
│   ├── exec_peer.rs        # Process execution
│   ├── file_peer.rs        # File I/O
│   ├── broadcast_reuse_peer.rs  # Broadcast mode
│   ├── reconnect_peer.rs   # Auto-reconnect
│   ├── crypto_peer.rs      # Encryption
│   ├── jsonrpc_peer.rs     # JSON-RPC formatting
│   ├── prometheus_peer.rs  # Metrics
│   └── ...                 # Various peer types
```

## Installation

### From Package Manager

```bash
# Fedora
dnf copr enable atim/websocat -y && dnf install websocat

# FreeBSD
pkg install websocat

# macOS (Homebrew)
brew install websocat

# macOS (MacPorts)
sudo port install websocat
```

### From Source

```bash
cargo install --features=ssl websocat
# Or move binary from target/release/websocat to PATH
```

## Basic Usage

### Simple Client

```bash
# Connect to echo server
websocat ws://ws.vi-server.org/mirror
123
123
ABC
ABC

# Secure WebSocket
websocat wss://echo.websocket.org
```

### Simple Server

```bash
# Listen on port
websocat -s 1234
# In another terminal:
websocat ws://127.0.0.1:1234
```

### Advanced Mode

```bash
# WebSocket to TCP proxy
websocat ws-l:127.0.0.1:8080 tcp:127.0.0.1:5678

# TCP to WebSocket (reverse)
websocat tcp-l:127.0.0.1:8080 ws://backend:9000/
```

## Address Types

### WebSocket Addresses

| Type | Syntax | Description |
|------|--------|-------------|
| Client | `ws://host:port/path` | Insecure WebSocket client |
| Client | `wss://host:port/path` | Secure WebSocket client |
| Server | `ws-l:host:port` | WebSocket server (listen) |
| Server | `wss-l:host:port` | Secure WebSocket server |
| Unix | `ws-u:path` | WebSocket over Unix socket |

### Transport Addresses

| Type | Syntax | Description |
|------|--------|-------------|
| TCP | `tcp:host:port` | TCP client |
| TCP | `tcp-l:host:port` | TCP server |
| UDP | `udp:host:port` | UDP client |
| UDP | `udp-l:host:port` | UDP server |
| Unix | `unix:path` | Unix socket client |
| Unix | `unix-l:path` | Unix socket server |

### Special Addresses

| Type | Syntax | Description |
|------|--------|-------------|
| Stdio | `-` | Standard input/output |
| Mirror | `mirror:` | Echo input back |
| Literal | `literal:text` | Output text, discard input |
| Exec | `exec:command` | Execute command |
| File | `readfile:path` | Read file |
| File | `writefile:path` | Write file |

## Overlays (Modifiers)

Overlays modify data passing through them. Syntax: `overlay:address`

### WebSocket Overlays

```bash
# WebSocket upgrade over custom transport
websocat ws-upgrade:tcp:example.com:80

# Raw WebSocket client (no HTTP upgrade)
websocat ws-lowlevel-client:tcp:example.com:80
```

### Data Transformation

```bash
# Line-to-message conversion (default)
websocat --no-line ws://server/

# Length-prefixed framing
websocat lengthprefixed:tcp:server:80

# Base64 encoding
websocat --base64 ws://server/

# JSON-RPC formatting
websocat --jsonrpc ws://server/
```

### Connection Management

```bash
# Auto-reconnect
websocat autoreconnect:ws://server/

# Connection reuse
websocat reuse-raw:ws-l:8080 tcp:backend:9000

# Broadcast to all clients
websocat broadcast:mirror:
```

## Common Use Cases

### Echo Server Testing

```bash
# Run autobahn test server
websocat -s 9001

# Connect as client
websocat ws://localhost:9001
```

### WebSocket to TCP Proxy

```bash
# Forward WebSocket connections to TCP service
websocat --oneshot -b ws-l:127.0.0.1:1234 tcp:127.0.0.1:22

# Reverse: TCP to WebSocket
websocat tcp-l:127.0.0.1:1236 ws://127.0.0.1:1234/
```

### Process Communication

```bash
# WebSocket to shell command
websocat ws-l:8080 exec:sh

# WebSocket to Python script
websocat ws-l:8080 exec:python3:- << 'EOF'
import sys
for line in sys.stdin:
    print(f"Processed: {line.strip()}")
EOF
```

### Broadcast Server

```bash
# All connected clients receive each other's messages
websocat -t ws-l:127.0.0.1:1234 broadcast:mirror:

# Multiple clients connect:
# Terminal A: websocat ws://127.0.0.1:1234
# Terminal B: websocat ws://127.0.0.1:1234
# Terminal C: websocat ws://127.0.0.1:1234
```

### Auto-Reconnect Client

```bash
# Reconnect on disconnect with delay
websocat autoreconnect:ws://unreliable-server/

# With custom delay
websocat --autoreconnect-delay-millis=1000 autoreconnect:ws://server/
```

### Debugging and Inspection

```bash
# Log all traffic
websocat -v ws://server/

# One message only
websocat -n1 ws://server/

# Unidirectional (receive only)
websocat -u ws://server/

# Exit on EOF
websocat -E ws://server/
```

## Advanced Examples

### Chromium DevTools

```bash
# Start Chromium with remote debugging
chromium --remote-debugging-port=9222 &

# Get WebSocket URL for new tab
curl -sg http://127.0.0.1:9222/json/new | \
    grep webSocketDebuggerUrl | cut -d'"' -f4

# Connect to DevTools protocol
echo 'Page.navigate {"url":"https://example.com"}' | \
    websocat -n1 --jsonrpc --jsonrpc-omit-jsonrpc \
    ws://127.0.0.1:9222/devtools/page/XXX
```

### Nginx Integration

```bash
# WebSocket server behind Nginx
# Nginx config:
# location /ws {
#     proxy_pass http://127.0.0.1:8080;
#     proxy_http_version 1.1;
#     proxy_set_header Upgrade $http_upgrade;
#     proxy_set_header Connection "upgrade";
# }

# Websocat backend
websocat ws-l:127.0.0.1:8080 exec:my_handler.sh
```

### SOCKS5 Proxy

```bash
# Connect through SOCKS5
websocat --socks5 127.0.0.1:9050 ws://hidden-service.onion/

# Bind mode ( SOCKS5 bind command)
websocat socks5-bind:127.0.0.1:9050 ws-l:8080
```

### SSL/TLS Termination

```bash
# Accept SSL connections
websocat wss-l:8080 --pkcs12-der=cert.pkcs12 --pkcs12-passwd=secret \
    tcp:backend:9000

# Connect with custom CA
websocat --cacert=ca.pem wss://server/
```

### Prometheus Metrics

```bash
# Expose connection metrics
websocat --prometheus 127.0.0.1:9090 ws-l:8080 mirror:

# Scrape metrics
curl http://127.0.0.1:9090/metrics
```

### Encryption

```bash
# Encrypted WebSocket traffic
websocat crypto:file:key.txt:ws://server/

# Reverse encryption on server
websocat ws-l:8080 crypto:file:key.txt:exec:handler.sh
```

## Command-Line Options Reference

### Connection Options

| Option | Short | Description |
|--------|-------|-------------|
| `--binary` | `-b` | Send as binary messages |
| `--text` | `-t` | Send as text messages |
| `--no-close` | `-n` | Don't send Close on EOF |
| `--one-message` | `-1` | Send/receive one message |
| `--exit-on-eof` | `-E` | Close on EOF |
| `--insecure` | `-k` | Accept invalid certificates |

### Server Options

| Option | Short | Description |
|--------|-------|-------------|
| `--server-mode` | `-s` | Simple server mode |
| `--oneshot` | | Serve only once |
| `--conncap=N` | | Max parallel connections |
| `--restrict-uri=/ws` | | Only accept this URI |

### Header Options

| Option | Short | Description |
|--------|-------|-------------|
| `--header=H: v` | `-H` | Add request header |
| `--server-header=H: v` | | Add response header |
| `--protocol=proto` | | Sec-WebSocket-Protocol |
| `--origin=http://x` | | Origin header |
| `--basic-auth=user:pass` | | Basic auth header |

### Heartbeat Options

| Option | Description |
|--------|-------------|
| `--ping-interval=30` | Send ping every N seconds |
| `--ping-timeout=60` | Close if no pong in N seconds |
| `--inhibit-pongs=N` | Stop replying after N pings |

### Size Limits

| Option | Default | Description |
|--------|---------|-------------|
| `--buffer-size=N` | 65536 | Max message size |
| `--max-ws-frame-length=N` | 104857600 | Max frame size |
| `--max-ws-message-length=N` | 209715200 | Max message size |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Connection refused/failed |
| 2 | Protocol error |
| 3 | TLS error |
| 4 | Timeout |

## Performance Tuning

```bash
# Increase buffer for large messages
websocat --buffer-size=1048576 ws://server/

# Disable line mode for binary protocols
websocat --no-line ws://server/

# Unidirectional for better throughput
websocat -u ws://server/
```

## Scripting Examples

### Bash Script

```bash
#!/bin/bash
# Automated WebSocket test

while true; do
    echo '{"type":"ping"}' | websocat -n1 ws://server/api
    sleep 5
done
```

### Python Integration

```python
import subprocess
import json

def ws_request(url, message):
    proc = subprocess.Popen(
        ['websocat', '-n1', url],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        text=True
    )
    stdout, _ = proc.communicate(json.dumps(message))
    return json.loads(stdout)
```

### Node.js Wrapper

```javascript
const { spawn } = require('child_process');

function wsClient(url) {
    const websocat = spawn('websocat', [url]);

    websocat.stdout.on('data', (data) => {
        console.log('Received:', data.toString());
    });

    return {
        send: (msg) => websocat.stdin.write(msg + '\n'),
        close: () => websocat.kill()
    };
}
```

## Troubleshooting

### Common Issues

**Connection refused:**
```bash
# Check if server is running
websocat -v ws://localhost:8080/

# Try with different TLS settings
websocat -k wss://localhost:8080/
```

**Handshake fails:**
```bash
# Check required headers
websocat -H 'Sec-WebSocket-Protocol: chat' ws://server/

# Debug with verbose output
websocat -v -H 'Origin: http://localhost' ws://server/
```

**Binary data corruption:**
```bash
# Force binary mode
websocat --binary ws://server/

# Or use base64
websocat --base64 ws://server/
```

## Related Tools

| Tool | Purpose |
|------|---------|
| wscat | Simple WebSocket client |
| websocketd | Run scripts over WebSocket |
| wstunnel | WebSocket tunneling |
| socat | General network tool (inspiration) |

## References

- [GitHub Repository](https://github.com/vi/websocat)
- [More Examples](https://github.com/vi/websocat/blob/master/moreexamples.md)
- [Full Documentation](https://github.com/vi/websocat/blob/master/doc.md)
