# 03 — Core UX

Essential UI patterns for FlowFlow's intelligence layer. Only patterns that serve document import, tags, and RAG chat.

## Chat UI

### Layout
- Separate view from note editor (new `View::Chat` variant)
- Accessible from: sidebar button, or contextual button in folder view
- Full-screen overlay with slide transition (same pattern as NoteDetail)

### Components
- **Message list**: scrollable, alternating user/AI bubbles
- **Input bar**: fixed bottom, text input + send button
- **Scope indicator**: top bar shows current context ("All notes", "Folder: Work", "#meeting")
- **Source citations**: each AI response shows linked sources (tap → opens note/document)

### Scoping UX
- From main view → chat defaults to global scope
- From folder view → chat defaults to that folder (no reference needed)
- Scope switcher: dropdown or chips to change scope without leaving chat
- `@folder-name` or `@#tag` typed in input → auto-scopes

### Message Format
```
┌─────────────────────────────────┐
│ 🔍 All notes                    │  ← scope indicator
├─────────────────────────────────┤
│                                 │
│  [User] What did we discuss     │
│         about the budget?       │
│                                 │
│  [AI] Based on your notes:      │
│       The Q3 budget was set     │
│       at 150K with...           │
│                                 │
│       Sources:                  │
│       · Réunion budget (8 mai)  │
│       · Budget_Q3.pdf           │
│                                 │
├─────────────────────────────────┤
│  [input field]          [Send]  │
└─────────────────────────────────┘
```

## Document Import UI

### Entry Points
1. **Inside note editor**: "Import" button in toolbar → attach file to current note
2. **Sidebar "Documents" section**: dedicated import area for standalone documents

### Import Flow
- Tap "Import" → `<input type="file">` → iOS Files picker opens natively
- Show progress indicator during text extraction
- After import: show document title + auto-generated tags
- Tags editable inline before confirming

### Documents List
- Similar to NotesList but for imported documents
- Shows: title, file type icon (PDF/TXT/DOC), tags, date, folder
- Tap → shows extracted text content (read-only)

## Tag Management UI

### In Note/Document Editor
- Tags displayed as chips below title
- Auto-generated tags appear after AI processing (editable)
- "Add tag" button or inline `#tag` typing
- Tap tag chip → remove or edit

### In Sidebar Drawer
- "Tags" section below "Folders"
- List of all tags with note/document count
- Tap tag → filter view to that tag (across all folders or within current folder)

### Tag + Folder Interaction
- Folder can have default tags (inherited by new notes created in it)
- Filter: "Show #meeting in Work folder" → combines folder + tag scope
- Same filtering available in chat scope

## View Architecture

Current views:
```rust
pub enum View {
    NotesList,
    NoteDetail { note_id: String },
}
```

Extended:
```rust
pub enum View {
    NotesList,
    NoteDetail { note_id: String },
    Chat { scope: ChatScope },
    DocumentsList,
    DocumentDetail { doc_id: String },
}

pub enum ChatScope {
    Global,
    Folder { folder_id: String },
    Tag { tag: String },
}
```

## Navigation

```
Sidebar Drawer
├── All notes (NotesList)
├── Folders
│   ├── Work → NotesList (filtered) + Chat (folder-scoped)
│   ├── Personal → ...
│   └── ...
├── Tags
│   ├── #meeting → NotesList (filtered by tag)
│   ├── #budget → ...
│   └── ...
├── Documents (DocumentsList)
└── Chat (global scope)

FAB (+) → New note (current behavior)
```
