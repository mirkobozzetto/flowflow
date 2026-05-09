# 04 — Media Attachments

How to implement photo capture, photo import, file attachments, and screenshots in FlowFlow (Dioxus 0.7, WKWebView, 100% Rust).

## The Good News

WKWebView natively supports `<input type="file">` which opens the iOS photo picker or camera without any objc2 code. This is the simplest path.

## Approach 1: HTML File Input (Recommended — Zero objc2)

### Photo Import from Library

```html
<input type="file" accept="image/*" />
```

In WKWebView on iOS, this opens the native PHPicker (Photos library picker). The user selects a photo, and the browser returns the file data via the `change` event.

Dioxus RSX:
```rust
input {
    r#type: "file",
    accept: "image/*",
    onchange: move |evt| {
        // evt.files() returns FileEngine with file data
        // Read bytes, save to Documents dir, store path in DB
    },
}
```

### Camera Capture

```html
<input type="file" accept="image/*" capture="camera" />
```

Opens the camera directly. Requires `NSCameraUsageDescription` in `Dioxus.toml`:
```toml
[ios.plist]
NSCameraUsageDescription = "FlowFlow needs camera access to attach photos to notes"
```

### Generic File Import

```html
<input type="file" accept="*/*" />
```

Opens the iOS Files app picker. User can select PDFs, documents, etc.

### Multiple Files

```html
<input type="file" accept="image/*" multiple />
```

Allows selecting multiple photos at once.

## Approach 2: objc2 PHPickerViewController (Fallback)

If `<input type="file">` doesn't work well in Dioxus WKWebView (needs testing), fall back to objc2:

```rust
// ~80 lines of objc2 code
// Uses PHPickerViewController (iOS 14+) or UIImagePickerController
// Pattern documented in Dioxus issue #3849
```

Effort: ~1 day. Same pattern as existing `hide_keyboard_accessory()` in `src/platform/ios.rs`.

## Data Model

### SQLite Schema (V2 migration)

```sql
CREATE TABLE IF NOT EXISTS attachments (
    id TEXT PRIMARY KEY,
    note_id TEXT NOT NULL,
    file_name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    width INTEGER,
    height INTEGER,
    thumbnail_path TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_attachments_note ON attachments(note_id);
```

### Rust Model

```rust
pub struct Attachment {
    pub id: String,
    pub note_id: String,
    pub file_name: String,
    pub file_path: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub thumbnail_path: Option<String>,
    pub created_at: String,
}

pub struct NewAttachment {
    pub note_id: String,
    pub file_name: String,
    pub file_path: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
}
```

## Storage

### File Location
- iOS: `~/Documents/flowflow/attachments/{note_id}/{uuid}.{ext}`
- Desktop: `/tmp/flowflow/attachments/{note_id}/{uuid}.{ext}`
- Same pattern as audio files in `src/services/audio.rs:output_dir()`

### Thumbnails
- Generate thumbnails for images (200x200px) using the `image` crate
- Store in `~/Documents/flowflow/thumbnails/{uuid}_thumb.jpg`
- Display thumbnails in NoteCard and NoteDetail attachment list
- Full-size on tap (lightbox or fullscreen view)

## Rust Crates Needed

| Crate | Purpose | Size |
|-------|---------|------|
| `image` | Decode/encode images, generate thumbnails | ~2MB |
| `mime_guess` | Detect MIME type from file extension | ~100KB |

Add to `Cargo.toml`:
```toml
image = { version = "0.25", default-features = false, features = ["jpeg", "png"] }
mime_guess = "2"
```

## UI Integration

### NoteDetail — Attachment Button
- Add paperclip/image icon button next to the mic button in the fixed bottom toolbar
- Tap → shows options: "Photo Library", "Camera", "File"
- Each option triggers the appropriate `<input type="file">` variant

### NoteDetail — Attachment Display
- Below the textarea, show attached images as a horizontal scrollable row
- Thumbnails: 80x80px rounded, tap for full-size
- Non-image files: icon + filename + size
- Delete button (X) on each attachment

### NoteCard — Attachment Badge
- Paperclip icon or image count badge on cards with attachments
- "3 photos" or paperclip icon in the metadata row (next to date and folder)

## Dioxus File Upload Flow

```
User taps "Photo" button
  → Hidden <input type="file" accept="image/*"> triggered via JS
  → iOS photo picker opens natively
  → User selects photo(s)
  → onchange fires with FileEngine data
  → Read file bytes in Rust
  → Generate UUID filename
  → Save to Documents/flowflow/attachments/
  → Generate thumbnail (image crate)
  → Insert into attachments table
  → Display in NoteDetail
  → Auto-save links attachment to note
```

## Testing Notes

- **Must test on real iOS device** — simulator may not have camera
- Photo picker works differently on simulator (uses Photos.app with sample images)
- Check file size limits (WKWebView may have memory constraints for large images)
- Consider image compression before storage (JPEG quality 80% saves ~60% space)

## Priority

Medium effort (1-2 days). Implement after pin notes, search, and AI titles. The `<input type="file">` approach needs validation on a real iOS device first — if it works in WKWebView, this becomes a quick win.
