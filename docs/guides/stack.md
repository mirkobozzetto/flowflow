# Stack versions

Pinned versions and the reason each one is pinned. Update this file when a dependency
moves; `CLAUDE.md` links here rather than carrying the list, so the always-loaded
context stays small.

- Dioxus 0.7 (CLI dx 0.7.7)
- cpal 0.17 (audio I/O via CoreAudio on iOS)
- hound 3.5 (WAV file writing)
- reqwest 0.13 (HTTP client, multipart + JSON, unified with rig-core)
- rig-core 0.36 (LLM abstraction, agent + tools, rustls feature, OpenAI + Anthropic providers)
- Anthropic provider (Claude Sonnet 4.6 default, max_tokens 4096, chat + agent + tools)
- tokio 1 (async runtime)
- serde 1.0 + serde_json 1.0 (JSON serialization)
- dotenvy 0.15 (.env loader)
- rusqlite 0.34 (SQLite, bundled for iOS cross-compile)
- uuid 1 (UUID v4 generation)
- chrono 0.4 (ISO 8601 timestamps)
- Tailwind CSS V4 (auto-detected by dx)
- lancedb 0.27.2 (vector DB, default-features = false for iOS)
- arrow-array 57 + arrow-schema 57 (must match lancedb's arrow version)
- pulldown-cmark 0.12 (markdown to HTML for chat responses)
- pdf-extract 0.10 (PDF text extraction)
- zip 2 (DOCX archive reading)
- quick-xml 0.36 (DOCX word/document.xml parser)
- futures 0.3.32 (stream collect for LanceDB queries)
- whisper-rs 0.16 (local STT, whisper.cpp via cmake, metal feature on apple targets)
- fs4 0.13 (free disk space check for model downloads)
- libc 0.2 (getrusage peak RSS in the whisper bench)
- Rust 1.94.1
- iOS targets: aarch64-apple-ios, aarch64-apple-ios-sim
- IPHONEOS_DEPLOYMENT_TARGET=16.0 (required for lancedb/zstd-sys)
