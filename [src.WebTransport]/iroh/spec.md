---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.WebTransport/src.n0-computer/iroh
repository: git@github.com:n0-computer/iroh
revised_at: 2026-06-03T00:00:00Z
workspace: iroh
---

# iroh — Spec File

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.WebTransport/src.n0-computer/iroh`
- **Language:** Rust
- **Edition:** 2024
- **Rust Version:** 1.91
- **License:** MIT OR Apache-2.0
- **Version:** 1.0.0-rc.1
- **Remote:** `git@github.com:n0-computer/iroh`
- **Workspace Members:** `iroh-base`, `iroh-dns`, `iroh-dns-server`, `iroh`, `iroh/bench`, `iroh-relay`

## What the Project Is

Iroh is a Rust library for dialing peer-to-peer QUIC connections by public key. It provides hole-punching with relay server fallback, using raw public key TLS (RFC 7250) with Ed25519 identities. The endpoint manager discovers, maintains, and selects the fastest network path regardless of NAT, firewalls, or network topology changes.

## Documentation Goal

After reading this documentation, a reader should understand:

1. How iroh's public-key addressing model works (EndpointId = Ed25519 public key)
2. The Endpoint lifecycle: bind → discover → connect → relay fallback → direct upgrade
3. The Router/ProtocolHandler pattern for registering ALPN-based protocol handlers
4. How AddressLookup services (DNS, Pkarr, Memory) resolve endpoint addresses
5. How net_report probes determine NAT status, relay latencies, and preferred relay
6. The Socket layer: transports (IP, Relay, Custom), RemoteMap, and path selection
7. The TLS layer: raw public key certificates, RFC 7250, Ed25519 verification
8. The relay server architecture and ACME certificate management
9. How data flows from application protocol → QUIC stream → Socket → Transport → Network
10. Cross-cutting concerns: WASM support, portmapping, metrics, runtime management

## Documentation Structure

```
iroh/
├── spec.md
├── markdown/
│   ├── README.md              ← Index / table of contents
│   ├── 00-overview.md         ← What iroh is, why it exists, architecture at a glance
│   ├── 01-architecture.md     ← Full dependency graph, layer diagram, module map
│   ├── 02-endpoint.md         ← Endpoint: the main connection manager
│   ├── 03-protocol.md         ← Router and ProtocolHandler: ALPN-based protocol dispatch
│   ├── 04-address-lookup.md   ← DNS, Pkarr, and Memory address resolution
│   ├── 05-net_report.md       ← Network condition reporting: probes, reports, relay selection
│   ├── 06-tls.md              ← Raw public key TLS: RFC 7250, Ed25519 verification
│   ├── 07-socket.md           ← Socket layer: transports, RemoteMap, path selection
│   ├── 08-iroh-relay.md       ← Relay server and client architecture
│   ├── 09-data-flow.md        ← End-to-end flows with sequence diagrams
│   └── 10-cross-cutting.md    ← WASM, portmapper, metrics, runtime, custom transports
├── html/
│   ├── index.html
│   ├── styles.css
│   └── *.html
└── build.py
```

## Tasks

| Phase | Document | Status |
|-------|----------|--------|
| Foundation | README.md | DONE |
| Foundation | 00-overview.md | DONE |
| Architecture | 01-architecture.md | DONE |
| Deep Dive | 02-endpoint.md | DONE |
| Deep Dive | 03-protocol.md | DONE |
| Deep Dive | 04-address-lookup.md | DONE |
| Deep Dive | 05-net_report.md | DONE |
| Deep Dive | 06-tls.md | DONE |
| Deep Dive | 07-socket.md | DONE |
| Deep Dive | 08-iroh-relay.md | DONE |
| Cross-Cutting | 09-data-flow.md | DONE |
| Cross-Cutting | 10-cross-cutting.md | DONE |
| Grandfather Review | All documents | DONE |
| HTML Generation | build.py | DONE |

## Build System

```bash
cd iroh && python3 build.py          # build all iroh docs
```

Script: `build.py` (zero dependencies, Python 3.12+ stdlib only). Converts markdown to HTML with mermaid diagrams, dark/light theme, prev/next navigation.

## Quality Requirements

1. **Detailed Sections with Code Snippets** — Every concept grounded in actual source code with file:line references
2. **Teach Key Facts Quickly** — First sentence of each section is its thesis
3. **Clear Articulation** — Non-overly-complex sentences, one idea per sentence
4. **Mermaid Diagrams** — Minimum 2 per document, all pass grandfather review
5. **Good Visual Assets** — Tables, ASCII art, mermaid for complex flows
6. **Generated HTML** — Well-aligned headers, breadcrumbs, prev/next navigation
7. **Cross-References** — Every document links to 2+ others
8. **Source Path References** — Actual file paths and line numbers
9. **Aha Moments** — At least 1 per document: non-obvious design decisions
10. **Navigation** — Index button, prev/next, theme toggle on every page

## Expected Outcome

After reading, a developer can:
- Build a production iroh application with protocol handlers
- Debug connectivity issues using net_report data
- Deploy and operate a relay server
- Extend iroh with custom transports or address lookup services
- Understand the path selection and hole-punching mechanics

## Resume Point

If interrupted, resume by writing documents in order: 00 → 01 → 02 → ... → 10. Each document should be written from the actual source code, then run through grandfather review before marking DONE. After all documents are written, run `python3 build.py` and verify HTML output.
