#[cfg(target_os = "ios")]
pub mod ios;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(any(target_os = "ios", target_os = "macos"))]
pub mod parsers;

#[cfg(any(target_os = "ios", target_os = "macos"))]
pub mod pdf;

#[cfg(target_os = "ios")]
pub fn detect_system_language() -> String {
    ios::detect_system_language()
}

#[cfg(not(target_os = "ios"))]
pub fn detect_system_language() -> String {
    "en".to_string()
}
