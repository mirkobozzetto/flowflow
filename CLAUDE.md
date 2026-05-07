# CLAUDE.md — FlowFlow

## Project

**FlowFlow** — 100% Rust mobile app for recording voice notes, transcribing them, generating tags/titles via AI, organizing in folders, and chatting with notes via local RAG.

Inspired by SuperPowerNotes (Next.js/TypeScript app by Mirko). See `ANALYSIS.md` for full source codebase analysis.

## Owner

Mirko Bozzetto — freelance full-stack developer, Brussels.

## Technical Constraints

- **Language**: 100% Rust. Zero JavaScript, zero TypeScript.
- **UI**: Dioxus (native mobile support)
- **Target**: iOS only (Android later)
- **Vector DB**: LanceDB (local semantic search)
- **Metadata**: SQLite (rusqlite)
- **Transcription**: Soniox REST API (async)
- **Embeddings**: ONNX Runtime (ort) on-device (all-MiniLM-L6-v2)
- **HTTP**: reqwest
- **Async**: tokio

## Work Methodology

- One step at a time. After each step → STOP → show result → wait for Mirko's validation.
- No predefined file structure beyond current step.
- If it doesn't compile or work → fix before moving forward.
- No assumptions — test everything.
- When unsure between two approaches → present options with pros/cons → Mirko decides.
- Git: commit after each validated step, descriptive messages.
- File names, structure, architecture evolve as needed. Nothing is set in stone.

## Tracks (indicative order, Mirko decides)

| Track | Description | Status |
|-------|-------------|--------|
| A | Minimal Dioxus iOS scaffold (hello world on simulator) | Done |
| B | Audio capture iOS mic (cpal + hound, save WAV) | Done |
| C | Soniox REST (upload WAV → transcription) | Done |
| D | SQLite storage + UI refactor + Tailwind | Done |
| E | On-device embeddings (ONNX, all-MiniLM-L6-v2) | — |
| F | RAG + Chat (embed → search → context → LLM → response) | — |

## Architecture (Clean Architecture)

```
src/
  main.rs              entry point
  models/              domain entities (Note, Folder, NoteType)
  db/                  persistence (Database, migrations, CRUD repos)
  services/            business logic (AudioRecorder, SonioxClient)
  platform/            OS-specific (iOS AVAudioSession, documents_dir)
  ui/                  Dioxus components + Tailwind CSS
    mod.rs             App, AppState, View enum, Stylesheet
    layout.rs          TopBar, Sidebar drawer, FAB
    notes.rs           NotesList, NoteCard, NoteDetail
    recording.rs       RecordingView, RecordButton, StatusLine
```

## Data Entities

### Note (global entity — voice and text are input modes, not types)
- id (UUID), note_type (voice/text), title, content, tags[], duration, audio_file_path
- Auto-titled with date+time when created on the fly
- Speech-to-text available from any note
- Future: embedding vector (LanceDB), summary (LLM)

### Folder (hierarchy)
- id, name, description, parent_id (self-ref), created_at
- N:N relation with Note via junction table (notes_folders)
- ON DELETE SET NULL for parent (children become root)

## Styling

- **Tailwind CSS V4** via Dioxus 0.7 auto-detection
- `tailwind.css` at project root = input file
- `dx serve` auto-compiles to `assets/tailwind.css`
- Custom colors: `ios-green` (#34c759), `ios-red` (#ff3b30), `ios-blue` (#007aff)
- Mobile-first: touch targets 44px, safe area insets, scroll fix on empty pages

## Main Pipeline

```
Mic capture → WAV/audio
    → Soniox REST API → transcription
    → LLM API → title + tags
    → ONNX (ort) → embedding vector
    → SQLite (metadata) + LanceDB (vector)
```

## Stack Versions

- Dioxus 0.7 (CLI dx 0.7.7)
- cpal 0.17 (audio I/O via CoreAudio on iOS)
- hound 3.5 (WAV file writing)
- reqwest 0.12 (HTTP client, multipart + JSON)
- tokio 1 (async runtime)
- serde 1.0 + serde_json 1.0 (JSON serialization)
- dotenvy 0.15 (.env loader)
- rusqlite 0.34 (SQLite, bundled for iOS cross-compile)
- uuid 1 (UUID v4 generation)
- chrono 0.4 (ISO 8601 timestamps)
- Tailwind CSS V4 (auto-detected by dx)
- Rust 1.94.1
- iOS targets: aarch64-apple-ios, aarch64-apple-ios-sim

## Commands (use Makefile)

```bash
make build    # cargo build --features mobile
make format   # cargo fmt (80-char max)
make check    # fmt check + clippy
make dev      # dx serve --ios (simulator)
make ddev     # dx serve --ios --device (physical iPhone)
make desktop  # dx serve --desktop (Mac window, real mic)
make logs     # iPhone device logs (idevicesyslog)
```

## Environment Variables

Create a `.env` file at the project root (never committed):

```
SONIOX_API_KEY=your_soniox_api_key
```

Get your API key at https://console.soniox.com

### Manual commands (if needed)

```bash
# Simulator management
open /Applications/Xcode.app/Contents/Developer/Applications/Simulator.app
xcrun simctl boot "iPhone 17 Pro"
xcrun simctl shutdown all

# Device management
xcrun devicectl list devices
xcrun devicectl manage pair --device <DEVICE_ID>
```

## Physical Device Setup (one-time)

1. iPhone: Settings → Privacy & Security → Developer Mode → enable → restart
2. Connect via USB, accept "Trust This Computer"
3. `xcrun devicectl manage pair --device <ID>` (fixes the "no DDI" error)
4. Xcode → Settings → Apple Accounts → click account → Manage Certificates → + → Apple Development
5. If certificate not recognized by codesigning, install Apple WWDR intermediate cert:
   `curl -sO https://www.apple.com/certificateauthority/AppleWWDRCAG3.cer && security add-certificates AppleWWDRCAG3.cer && rm AppleWWDRCAG3.cer`
6. Verify: `security find-identity -v -p codesigning` must show "Apple Development"
7. Create provisioning profile (required for free Apple account):
   WORKAROUND: this method is cumbersome. Looking for a simpler approach.
   No existing Dioxus issue on this — worth opening one if dx doesn't improve this.
   Ref: https://github.com/DioxusLabs/dioxus/issues/3817 (related App Store issue)
   For now, create a TEMPORARY Swift/SwiftUI project in Xcode:
   - Xcode → File → New → Project → iOS → App
   - Product Name: `flowflow`, Organization Identifier: `com.mirkobozzetto`, Team: Personal Team
   - Interface: SwiftUI, Language: Swift (doesn't matter, it's temporary)
   - Save to /tmp
   - Select iPhone as destination at the top of Xcode
   - Cmd+R to build — Xcode creates the provisioning profile automatically
   - Trust the dev profile on iPhone: Settings → General → VPN & Device Management → Trust
   - Close the Xcode project (profile stays in ~/Library/Developer/Xcode/UserData/Provisioning Profiles/)
   The profile is tied to the bundle ID (com.mirkobozzetto.flowflow), not the language.
   Once created, delete the Xcode project — the profile persists and `dx serve` uses it for the Rust app.
8. After first pairing, Wi-Fi works (same network)

## References

- `ANALYSIS.md`: full SuperPowerNotes analysis
- `INSTRUCTIONS.md`: first session startup brief
- SuperPowerNotes source: `/Users/mirkobozzetto/stuffs/superpowernotes`
