use dioxus::prelude::*;

// Apple's SF Symbols rendered to masks (assets/sf), for the webview: the
// native menus draw the same symbols themselves.
fn mask(name: &str) -> Asset {
    match name {
        "xmark" => asset!("/assets/sf/xmark.png"),
        "photo.on.rectangle.angled" => {
            asset!("/assets/sf/photo.on.rectangle.angled.png")
        }
        "camera" => asset!("/assets/sf/camera.png"),
        "doc" => asset!("/assets/sf/doc.png"),
        "cpu" => asset!("/assets/sf/cpu.png"),
        "briefcase" => asset!("/assets/sf/briefcase.png"),
        "paintpalette" => asset!("/assets/sf/paintpalette.png"),
        "server.rack" => asset!("/assets/sf/server.rack.png"),
        "envelope" => asset!("/assets/sf/envelope.png"),
        "chevron.left.forwardslash.chevron.right" => {
            asset!("/assets/sf/chevron.left.forwardslash.chevron.right.png")
        }
        "play.rectangle" => asset!("/assets/sf/play.rectangle.png"),
        "brain" => asset!("/assets/sf/brain.png"),
        "note.text" => asset!("/assets/sf/note.text.png"),
        "person.crop.circle" => asset!("/assets/sf/person.crop.circle.png"),
        "checklist" => asset!("/assets/sf/checklist.png"),
        "magnifyingglass" => asset!("/assets/sf/magnifyingglass.png"),
        "house" => asset!("/assets/sf/house.png"),
        "bubble.left.and.bubble.right" => {
            asset!("/assets/sf/bubble.left.and.bubble.right.png")
        }
        "hammer" => asset!("/assets/sf/hammer.png"),
        "globe" => asset!("/assets/sf/globe.png"),
        _ => asset!("/assets/sf/sparkles.png"),
    }
}

/// An SF Symbol tinted by the text colour.
#[component]
pub fn SfIcon(name: String, #[props(default = 20)] size: u32) -> Element {
    let url = mask(&name);
    rsx! {
        span {
            class: "sf-icon",
            "aria-hidden": "true",
            style: "-webkit-mask-image: url({url}); mask-image: url({url}); width: {size}px; height: {size}px;",
        }
    }
}
