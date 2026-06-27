pub enum Slide {
    Left,
    Right,
}

pub fn slide_style(dir: Slide, sliding_out: bool) -> &'static str {
    if cfg!(target_os = "macos") {
        return "";
    }
    match (dir, sliding_out) {
        (Slide::Left, true) => {
            "animation: slideOutToLeft 0.15s ease-in forwards;"
        }
        (Slide::Left, false) => "animation: slideInFromLeft 0.15s ease-out;",
        (Slide::Right, true) => {
            "animation: slideOutRight 0.15s ease-in forwards;"
        }
        (Slide::Right, false) => "animation: slideInRight 0.15s ease-out;",
    }
}
