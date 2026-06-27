mod inset;
#[cfg(target_os = "macos")]
mod macos;

pub use inset::use_keyboard_inset;
#[cfg(target_os = "macos")]
pub use macos::use_macos_shortcuts;
