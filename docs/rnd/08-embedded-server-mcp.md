# 08 — Embedded Server & MCP Mobile

FlowFlow runs in Rust with Tokio on iOS. We can expose a local HTTP server from the app and turn the phone into an MCP Server for desktop tools (Claude, Cursor, etc.).

## 1. Embedded HTTP Server on iOS

### How it works

The app binds a `TcpListener` on a local port (e.g. 8080). Any device on the same WiFi network can reach `http://192.168.x.x:8080`. In Rust with Tokio, this is native.

```rust
// Minimal example with axum
let router = axum::Router::new()
    .route("/api/notes", get(list_notes))
    .route("/api/search", get(search_notes));

let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
axum::serve(listener, router).await?;
```

### iOS Limitations

| Constraint | Detail |
|-----------|--------|
| Foreground only | iOS kills the socket as soon as the app goes to background |
| Background task | `beginBackgroundTask()` gives ~10 min max after backgrounding |
| Auto-suspend | GCDWebServer handles this automatically (suspend/resume) |
| No public IP | Same WiFi network required (or tunnel via ngrok/Cloudflare) |

### Existing libs (Swift/Obj-C, for reference)

| Lib | Stars | Features |
|-----|-------|----------|
| GCDWebServer | 5.3k | Auto background handling, file upload, WebDAV, Bonjour |
| FlyingFox | ~500 | Swift async/await, BSD sockets, iOS 13+ |
| Swifter | 4k | Tiny, WebSocket included |
| Telegraph | ~450 | WebSocket + HTTP, modern |

For FlowFlow (100% Rust): no need for these libs. We already have Tokio + can add axum.

### Apps that already do this

- **ComicFlow** (iPad) — upload/download comics via WiFi from a web browser
- **PocketServer** — persistent HTTP/WebDAV server (~1MB)
- **mycellm** — LLM inference on iPad, serves `/v1/chat/completions` on local network
- **Locally AI** — Llama/Gemma on App Store, OpenAI-compatible server on LAN

### Required iOS config (Info.plist)

```xml
<key>NSLocalNetworkUsageDescription</key>
<string>To share your notes with your devices on the local network</string>

<key>NSBonjourServices</key>
<array>
    <string>_http._tcp</string>
    <string>_flowflow._tcp</string>
</array>

<key>NSAppTransportSecurity</key>
<dict>
    <key>NSAllowsLocalNetworking</key>
    <true/>
</dict>
```

## 2. MCP Mobile

### MCP Protocol

Model Context Protocol = open standard for connecting AI tools to data sources. Transport-agnostic (JSON-RPC 2.0).

| Transport | Mobile-friendly | Detail |
|-----------|----------------|--------|
| Streamable HTTP | Yes | POST for requests, GET+SSE for streaming. Ideal for local WiFi |
| stdio | No | Subprocess model, not possible on iOS |
| WebSocket | Yes | Persistent connection, works on LAN |

### Existing MCP Mobile projects

| Project | Platform | Type | Status | Link |
|---------|----------|------|--------|------|
| PocketMCP | Android | MCP Server (app) | Open Source | axonixtools.com/blog/pocketmcp |
| Mobile-MCP Framework | Android | Framework | Active | github.com/Mobile-MCP/Mobile-MCP |
| Swift SDK | iOS | Official SDK | Active | github.com/modelcontextprotocol/swift-sdk |
| Capacitor Mobile Claw | iOS/Android | Framework | Active | github.com/rogelioRuiz/capacitor-mobile-claw |
| ChatMCP | iOS/Android | MCP Client | TestFlight | github.com/mcp-syndicate/mcp-mobile |
| Android MCP SDK | Android | Native SDK | Active | github.com/kaeawc/android-mcp-sdk |
| Drengr | Android/iOS | Automation | Production | drengr.dev |

### Official Swift SDK (iOS 16+)

```swift
// MCP Client on iOS
let client = Client(name: "FlowFlow", version: "1.0.0")
let transport = HTTPClientTransport(url: URL(string: "http://localhost:8080/mcp")!)
try await client.connect(transport: transport)
let tools = try await client.listTools()
```

Supports: iOS, watchOS, tvOS, visionOS. Transports: StdioTransport, HTTPClientTransport, StatefulHTTPServerTransport.

### PocketMCP (Android, for reference)

- Runs as an Android app, exposes MCP Server via WebSocket on WiFi
- mDNS for discovery (zero config)
- Exposed tools: SMS, battery, GPS, notifications
- Granular permissions per tool

## 3. Use cases for FlowFlow

### Local search API

From the laptop, open a browser or a script:
```
GET http://192.168.1.42:8080/api/search?q=budget+Q3
→ [{ "id": "note-1", "title": "Budget meeting", "score": 0.92, "preview": "..." }]
```

Search through voice notes from a Mac without touching the phone.

### MCP Server for Claude Desktop

Claude Desktop (or Cursor) connects to the phone via WiFi. The user types "What did I say about the budget?" → Claude calls `search_notes` on the phone → retrieves context → responds.

```json
{
  "tools": [
    {
      "name": "search_notes",
      "description": "Search user's voice notes and documents by semantic similarity",
      "inputSchema": {
        "type": "object",
        "properties": {
          "query": { "type": "string" },
          "scope": { "type": "string", "enum": ["global", "folder", "tag"] },
          "scope_value": { "type": "string" }
        },
        "required": ["query"]
      }
    },
    {
      "name": "ask_question",
      "description": "Ask a question answered from the user's knowledge base (RAG)",
      "inputSchema": {
        "type": "object",
        "properties": {
          "question": { "type": "string" },
          "folder": { "type": "string" },
          "tag": { "type": "string" }
        },
        "required": ["question"]
      }
    },
    {
      "name": "list_notes",
      "description": "List all notes, optionally filtered by folder",
      "inputSchema": {
        "type": "object",
        "properties": {
          "folder": { "type": "string" },
          "limit": { "type": "integer", "default": 20 }
        }
      }
    },
    {
      "name": "list_tags",
      "description": "List all tags with note count"
    },
    {
      "name": "create_note",
      "description": "Create a new note from text",
      "inputSchema": {
        "type": "object",
        "properties": {
          "content": { "type": "string" },
          "title": { "type": "string" },
          "folder": { "type": "string" },
          "tags": { "type": "array", "items": { "type": "string" } }
        },
        "required": ["content"]
      }
    }
  ]
}
```

### RAG from the desktop

The laptop sends a question, the phone:
1. Embeds the question (OpenAI)
2. Searches LanceDB (top-5 chunks)
3. Builds the prompt with context
4. Calls the LLM
5. Returns the answer with sources

The entire RAG pipeline runs on the phone. The laptop only displays the result.

### Voice-to-API bridge

1. Speak into the phone (mic recording)
2. Soniox transcribes
3. LLM structures the text (title, tags, cleanup)
4. The laptop polls `/api/notes/latest` and retrieves the structured result
5. Or WebSocket push in real time

Use case: in a meeting, the phone captures voice. The laptop displays notes live.

### P2P sync between Apple devices

Multipeer Connectivity (native Apple framework):
- Auto-discovery (WiFi + Bluetooth)
- Encrypted by default
- No server needed
- iPhone <-> iPad sync without cloud

Limitation: dies in background (same as HTTP).

### Bonjour / mDNS discovery

The app announces itself on the local network:
```
FlowFlow on iPhone Pro — _flowflow._tcp — 192.168.1.42:8080
```

The desktop (Claude Desktop, browser, script) automatically discovers the app without knowing the IP. Zero config.

## 4. Technical architecture for FlowFlow

### Dual-build pattern

The same `services/` code compiles for two targets:

```
┌─────────────────────────────┐
│         services/           │
│  db/ + vectordb/ + chat/    │
│  (SQLite + LanceDB + RAG)   │
│  No UI dependency           │
└──────────┬──────────────────┘
           │
     ┌─────┴─────┐
     │           │
┌────▼────┐ ┌───▼────┐
│  Mobile │ │ Server │
│ Dioxus  │ │  axum  │
│  iOS    │ │ VPS/Mac│
└─────────┘ └────────┘
```

Mobile = standalone app, offline, local-first.
Server = same intelligence, accessible via API/MCP.

### Dioxus fullstack mode

Dioxus 0.7 supports native fullstack mode with `dioxus/server` + built-in axum:

```rust
// Server functions callable from the client
#[server]
async fn search_notes(query: String) -> ServerFnResult<Vec<Note>> {
    let db = Database::open()?;
    Ok(db.search(&query)?)
}

// In production, point to the server
#[cfg(not(feature = "server"))]
dioxus::fullstack::set_server_url("https://flowflow.example.com");
```

`dx serve` automatically creates two builds: client (mobile) + server (axum).

### Planned REST endpoints

```
GET  /api/notes              → list of notes (pagination, filters)
GET  /api/notes/:id          → note detail
GET  /api/search?q=budget    → semantic search (LanceDB)
POST /api/chat               → RAG question { question, scope?, folder?, tag? }
POST /api/import             → document upload (multipart)
GET  /api/tags               → list of tags with count
GET  /api/folders             → folder tree
```

### Exposed MCP tools

```
search_notes(query, scope?, scope_value?)   → semantic results
ask_question(question, folder?, tag?)       → RAG answer + sources
list_notes(folder?, limit?)                 → filtered notes
list_tags()                                 → tags + counts
create_note(content, title?, folder?, tags?) → new note
import_document(file, folder?)              → file import
```

### Embedded server in the mobile app

For the "laptop on same WiFi" case without a remote server:

```rust
// Spawn the HTTP server alongside the Dioxus UI
tokio::spawn(async {
    let router = axum::Router::new()
        .route("/api/search", get(search_handler))
        .route("/api/chat", post(chat_handler))
        .route("/mcp", post(mcp_handler));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, router).await.unwrap();
});
```

The server runs as long as the app is open. iOS kills it in background.

## 5. Local network security

### Identified risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| ATS bypass on local IPs | MitM possible | HTTPS self-signed (iOS 17+ fixes bug CVE-2023-38596) |
| Unauthorized access | Anyone on WiFi | Mandatory token auth |
| Root directory exposed | App data leak | Serve only API endpoints |
| Server start before auth | Passcode bypass | Start server after user unlock |
| WKWebView bypass | Local network access without permission | Known Apple issue, fix in progress |

### Best practices

1. Random token generated at startup, displayed in the app, required for every request
2. Never serve static files (no directory listing)
3. HTTPS self-signed for LAN traffic in production
4. Rate limiting on endpoints
5. Log incoming connections (IP, user-agent, endpoint)

### Simple authentication

```rust
async fn auth_middleware(req: Request, next: Next) -> Response {
    let token = req.headers().get("Authorization");
    match token {
        Some(t) if t == expected_token => next.run(req).await,
        _ => StatusCode::UNAUTHORIZED.into_response(),
    }
}
```

## 6. References and sources

### Embedded HTTP server on iOS

| Source | Type | Link |
|--------|------|------|
| GCDWebServer | iOS lib (5.3k stars) | github.com/feixue299/GCDWebServer |
| FlyingFox | Swift async lib | github.com/swhitty/FlyingFox |
| Swifter | Swift lib (4k stars) | github.com/glock45/swifter |
| Telegraph | Swift lib | Medium article (nikhiladigaz) |
| ComicFlow | iPad app (open source) | github.com/swisspol/ComicFlow |
| Running HTTP Server on iOS | Article | nikhiladigaz.medium.com |
| ATS Vulnerability CVE-2023-38596 | Security | blog.trailofbits.com |
| Web Servers in Apps Leak Data | Security | alesandroortiz.com |
| iOS Local Network Privacy TN3179 | Apple Doc | developer.apple.com/documentation/technotes/tn3179 |

### MCP Mobile

| Source | Type | Link |
|--------|------|------|
| MCP Swift SDK (official) | SDK | github.com/modelcontextprotocol/swift-sdk |
| PocketMCP | Android MCP Server | axonixtools.com/blog/pocketmcp |
| Mobile-MCP Framework | Android Framework | github.com/Mobile-MCP/Mobile-MCP |
| Android MCP SDK | Native SDK | github.com/kaeawc/android-mcp-sdk |
| Capacitor Mobile Claw | Hybrid framework | github.com/rogelioRuiz/capacitor-mobile-claw |
| ChatMCP | Cross-platform client | github.com/mcp-syndicate/mcp-mobile |
| Drengr | Mobile automation | drengr.dev |
| MCP in iOS Apps (blog) | Tutorial | artemnovichkov.com/blog/using-model-context-protocol-in-ios-apps |

### Dioxus Fullstack

| Source | Type | Link |
|--------|------|------|
| Dioxus Fullstack Native | Official doc | dioxuslabs.com/learn/0.7/essentials/fullstack/native/ |
| Dioxus Server Functions | Official doc | dioxuslabs.com/learn/0.6/guide/backend/ |
| dioxus-server crate | Crate | crates.io/crates/dioxus-server |

### Bonjour / Network discovery

| Source | Type | Link |
|--------|------|------|
| Ciao (mDNS Swift) | Lib | github.com/Ciao |
| Herald (Bonjour browser) | iOS app | github.com/jessedc/herald-ios-bonjour-browser |
| Apple Bonjour docs | Official doc | developer.apple.com/bonjour/ |

### P2P Sync

| Source | Type | Link |
|--------|------|------|
| Multipeer Connectivity | Apple Framework | developer.apple.com/documentation/multipeerconnectivity |
| PeerPlot | Couchbase Lite P2P example | github.com/rshankras/PeerPlot |

## 7. Potential roadmap

| Phase | Feature | Prerequisite |
|-------|---------|--------------|
| 1 | Local REST API (axum, foreground) | Track E + F (RAG working) |
| 2 | Bonjour discovery | Phase 1 |
| 3 | MCP Server (Streamable HTTP) | Phase 1 |
| 4 | Claude Desktop integration | Phase 3 |
| 5 | Dioxus fullstack (remote server) | Clean services/ architecture |
| 6 | P2P Sync (Multipeer) | Post-MVP |
