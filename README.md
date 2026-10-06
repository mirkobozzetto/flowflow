<p align="center">
  <img src="docs/logo.png" alt="FlowFlow" width="160" />
</p>

<h1 align="center">FlowFlow</h1>

<p align="center">
  Voice notes that think with you - iPhone and Mac, mainly Rust, built with <a href="https://github.com/DioxusLabs/dioxus">Dioxus</a>.<br/>
  Featured on <a href="https://dioxuslabs.com/awesome/">Awesome Dioxus</a>.
</p>

---

Most of my ideas come when I'm walking or between two tasks. And they vanish just as fast.

FlowFlow is a voice notes app that captures what you say, transcribes it, and lets you **chat with your notes** later. You ask "what was that pricing idea?" and the right passages come up - with links to the original notes.

No manual searching. No folders to dig through. Just talk, and find it later.

## Get it

- **iOS** - on the [App Store](https://apps.apple.com/app/id6773033233), available worldwide.
- **macOS (Apple Silicon)** - grab the DMG from the [latest release](https://github.com/mirkobozzetto/flowflow/releases/latest), drag FlowFlow to Applications. Signed and notarized by Apple: it opens like any Mac app.
- **From source** - see [Build](#build) below; one `make` installs on your own iPhone or Mac.

## Highlights

**Capture**

- One composer for notes and chat: type, or tap the mic and a dark voice
  capsule records, transcribes and hands the text back where you were typing
- Once there is something to send, the orange mic stretches into a pill with
  a send button, so you can dictate again and the words land after your text
- Live 60fps waveform, pause/resume, Dynamic Island live timer; the capsule
  stays on screen until the text is ready, with retry if a transcription fails
- Cloud (Soniox) or fully offline transcription with local Whisper models
- Long recordings finish: local Whisper works in one-minute chunks cut at a
  silence, saves after each one and resumes after a lock, crash or kill; on
  iOS 26 it keeps going with the screen off, its progress on the Lock Screen
- Soniox uploads go up compressed (AAC) and are sent again after a kill or lock
- Personal dictionary: your names and brands spelled right, everywhere

**Ask your notes**

- RAG chat with tappable sources: hybrid search (BM25 + vector + rerank)
- Chat through an OpenAI API key or a ChatGPT subscription; embeddings still
  use the OpenAI API
- Optional web search ([Exa](https://exa.ai)) fused into the same answer
- Save any answer or thread back as a note

**Organized for you**

- AI titles, tags and themes as you write; searchable chats
- Theme search from the menu or the title; every folder starts open and
  the ones you close stay closed
- Threads: related notes as one chronological story; right-click or long-press
  a note within a thread to copy its full text or share that note
- Smart filters in the search bar: dictated, reminder, document, thread

**Your devices, one brain**

- Encrypted LAN P2P sync (Noise protocol): no server, no cloud
- One account for up to 3 paired devices; premium follows the pairing automatically
- One-archive backup, validated crash-safe atomic restore

**Act, not just record**

- "Pick up the kids at 5pm" becomes a calendar event, one tap to confirm
- Notes as actions: the assistant executes with your connected tools, every write holds for approval
- One `+` menu in notes and chat, native on iOS 26: discuss this note, add
  it to a thread, tools, agents and connectors, with live connection state
  and each product's own icon
- Scoped agents: installed packages are verified and pinned, each capability
  checked against its owner, every native action confirmed before it runs
- Governed connectors (Google Sheets), each through its own MCP peer;
  groundwork for a signed-agent marketplace

**Talk to your Hermes Agent**

- Chats > New conversation > With Hermes: your own
  [Hermes Agent](https://github.com/NousResearch/hermes-agent) answers, its
  tool calls folded into one "N steps" row
- Link it by scanning one QR code; Hermes stays on your private Tailscale network
- Pick the model and reasoning level under the title; a reply keeps running
  when you leave the app and is there when you come back
- Attach photos (or take one), a PDF, Word or text file from the `+`: Hermes
  sees the photos, reads the file's text
- Your Hermes skills in the `+`, most used first, then by category; type `/`
  or a word that names a skill and it is suggested, one tap to call it
- Send to Hermes from any note: a new conversation opens with the note attached

**Share a space with Hermes Agent**

- Grant a [Hermes Agent](https://github.com/NousResearch/hermes-agent) scoped,
  revocable access to one shared space from the space menu
- Hermes reads folders, notes and threads, writes its own notes and threads,
  and every change lands on your devices through the normal space pull
- Everything is a plain note on arrival: searchable and chat-ready

**Native feel**

- A real Mac app: ⌘N, ⌘F, ⌘⌘, view history, native file dialogs
- iPhone: the menu lies under the screen, which slides aside as a card;
  swipe from anywhere to open it
- iOS 26 Liquid Glass controls and native options menus (press, slide,
  release); haptics on every commit, drag-to-dismiss sheets
- English + French, down to error messages; word-level transcript with tap-to-seek

The full tour, one paragraph per feature: [docs/FEATURES.md](docs/FEATURES.md).

## How it works

```
Talk   → Record → Transcribe (cloud or on-device) → Clean fillers → Apply dictionary → Auto-embed → Store → AI title

Ask    → Embed query → Hybrid search (BM25 + vector)  ∥  Web search (Exa, when enabled)
       → RRF fusion → LLM rerank → Temporal boost → Tag-enriched context → Agent with tools → Answer with sources

Sync   → Save → debounced trigger → Noise-encrypted LAN session → version-vector merge → UI refresh < 1 s

Hermes → question + photos (image parts) + files and notes (text) + skills (named)
  chat   → your Hermes API server (HTTPS, Tailscale only) → streamed run → steps + answer
         history stays in the Hermes session; FlowFlow keeps a pointer on the device

Hermes → mcps_ token (one space, read or read_write) → MCP tools on api.flowflow.be/v1/mcp-spaces
         → notes and threads written by the agent → pulled by every member device

Backup → Export scrubbed SQLite snapshot + WAV + manifest (zip) → share
         Import → read-only validation → atomic swap at next launch → vector index rebuilt offline
```

## Hermes Agent

**Chat with it.** On the Hermes host, with Tailscale on both machines:

```sh
./scripts/enable-flowflow-chat.sh
```

It turns on Hermes' API server, exposes it on your Tailscale network only and
prints a QR code. Scan it with the iPhone camera: FlowFlow opens Settings >
Connections > Hermes with the address and key filled in. Check the address,
then tap **Save and test**. Step by step: [flowflow.be/hermes](https://flowflow.be/hermes/).

**Share a space with it.** FlowFlow ships a Hermes skill in [`skills/flowflow-spaces`](skills/flowflow-spaces/).
Install it on the Hermes host, store the one-time token in `~/.hermes/.env`,
and add one MCP server per shared space:

```sh
./scripts/install-flowflow-hermes-skill.sh
```

```yaml
mcp_servers:
  flowflow_projects:
    url: "https://api.flowflow.be/v1/mcp-spaces"
    headers:
      Authorization: "Bearer ${FLOWFLOW_TOKEN_PROJECTS}"
```

The MCP server reports a `contract_version`; the skill states the version it
was written for and asks for an update when they diverge. Update with
`git pull && ./scripts/install-flowflow-hermes-skill.sh`, or track the skill
with `hermes skills install <raw SKILL.md url>` and `hermes skills update`.
Full walkthrough, safety rules, rotation and revocation:
[docs/guides/hermes-flowflow.md](docs/guides/hermes-flowflow.md).

## Built with

Mainly Rust: the app itself is Rust end to end, UI included, with a few
deliberate exceptions where another tool does the job better - a small
TypeScript layer for webview gestures and the native menu bridge, Swift for
the Live Activity and background transcription, and the web sites in Astro.

| Part | Stack |
| ---- | ----- |
| App (`src/`) | Rust: [Dioxus 0.8 alpha](https://github.com/DioxusLabs/dioxus), [rig](https://github.com/0xPlaygrounds/rig), [LanceDB](https://lancedb.com), [rusqlite](https://github.com/rusqlite/rusqlite), [snow](https://github.com/mcginty/snow) (Noise XXpsk3), [cpal](https://github.com/RustAudio/cpal), [whisper-rs](https://github.com/tazz4843/whisper-rs), [tokio](https://tokio.rs), Tailwind CSS v4 |
| Webview glue | ~32 KB of TypeScript (gestures in `src/ui/hooks/*.ts`, native menu bridge in `src/ui/app/glass_burger.ts`), compiled by `make js` |
| iOS extras | Swift (`src/ios/widget`, `src/ios/plugin`): Live Activity, background transcription task, bridged via FFI |
| Account site (`account/`) | Astro - passkeys, plan and devices at account.flowflow.be |
| Landing (`landing-page/`) | Astro - flowflow.be |
| Backend | Rust (separate repo) - accounts, entitlements, governed connector proxy |
| AI services | OpenAI (embeddings + API-key chat), ChatGPT subscription (chat), Anthropic (chat), Soniox (cloud STT), Exa (web search), your own Hermes Agent (chat) |
| Targets | iOS 16+ (aarch64-apple-ios), macOS (Apple Silicon) |

## Build

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
cargo install dioxus-cli
cp .env.example .env   # add your API keys (or set them in-app)
```

API keys can be set in-app via Settings and are stored in SQLite. ChatGPT
subscription tokens are stored in Apple Keychain. Chat can use either an
OpenAI API key or a ChatGPT subscription. An OpenAI API key remains required
for semantic search and embeddings. Transcription works with a Soniox key or a
downloaded local Whisper model.

```bash
make all          # build + sign + icon + install on iPhone
make ddev         # dx serve --ios --device (hot reload)
make desktop-app  # build + install the Mac app in /Applications
make dmg          # distributable Mac DMG in dist/
make release      # make dmg + notarize + publish as a GitHub release
make check        # fmt check + clippy
make appstore     # release build + signed IPA
```

Full command list in the [Makefile](Makefile).

## Documentation

| Where | What |
|-------|------|
| [docs/INDEX.md](docs/INDEX.md) | Product, architecture, dev guides, App Store |
| [docs/FEATURES.md](docs/FEATURES.md) | The full feature tour |
| [docs/HISTORY.md](docs/HISTORY.md) | Every milestone, chronologically |
| [flowflow.be/hermes](https://flowflow.be/hermes/) | Link your Hermes Agent to the chat by QR code |
| [Hermes Agent guide](docs/guides/hermes-flowflow.md) | Connect one FlowFlow space to Hermes through MCP |

## Tests

```bash
cargo test
cargo test -- --ignored    # API-key-gated integration tests
```

## Status

Actively developed. The codebase evolves constantly - new features, better architecture, and deeper platform integration land regularly.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Copyright 2026 Mirko Bozzetto - [EUPL v1.2](LICENSE)
