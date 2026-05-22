# FlowFlow

100% Rust mobile app for recording voice notes, transcribing, generating tags/titles via AI, organizing in folders, and chatting with notes via local RAG.

Built with Dioxus 0.7.9, targeting iOS (iPhone).

## Features

- Voice recording with background audio support
- Speech-to-text via Soniox REST API
- AI-powered tags and titles (OpenAI / Anthropic)
- Semantic search via LanceDB embeddings
- RAG chat with agent tools (search, create, summarize)
- Document import (TXT, MD, CSV, PDF, DOCX)
- Dynamic Island Live Activity with recording timer
- AVAudioSession interruption handling (auto-pause on calls)
- Multi-provider LLM support (OpenAI + Anthropic via rig-core)

## Commands

```bash
make all            # build + sign widget + icon + install (auto-renews expired profiles)
make ddev           # dx serve --ios --device (hot reload, no widget signing)
make dev            # dx serve --ios (simulator)
make desktop        # dx serve --desktop (Mac window, real mic)
make build          # cargo build --features mobile
make format         # cargo fmt
make check          # fmt check + clippy
make deploy         # dx build device + icon injection
make appstore       # release build + distribution signing + IPA
make renew          # regenerate iOS provisioning profiles (xcodebuild)
make check-profiles # show profile expiration dates
make logs           # open Console.app (select iPhone, filter "FlowFlow")
make clean          # rm target/dx
```

## iOS Provisioning

Personal Team (free Apple ID) profiles expire every 7 days. `make all` auto-detects expiry < 24h and runs `make renew` before building. Renewal uses a minimal Xcode project template under `tools/provision-renew/` invoked via `xcodebuild -allowProvisioningUpdates`. Zero manual Xcode GUI required.

Once your paid Apple Developer Program is active, profiles last 1 year and `make renew` becomes a once-a-year operation.

## Stack

- Rust 1.94.1, Dioxus 0.7.9, Tailwind CSS V4
- cpal 0.17 (audio), hound 3.5 (WAV)
- rig-core 0.36 (LLM, OpenAI + Anthropic)
- LanceDB 0.27.2 (vector search)
- rusqlite 0.34 (SQLite, bundled)
- ActivityKit (Dynamic Island via Swift FFI)
- iOS 16.0+, aarch64-apple-ios

## Setup

```bash
cp .env.example .env
# Fill in: SONIOX_API_KEY, OPENAI_API_KEY, ANTHROPIC_API_KEY
make all
```

## License

EUPL 1.2
