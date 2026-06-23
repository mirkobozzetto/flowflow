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

#[cfg(target_os = "ios")]
pub fn open_url(url: &str) {
    ios::open_url(url);
}

#[cfg(target_os = "macos")]
pub fn open_url(url: &str) {
    macos::open_url(url);
}

#[cfg(not(any(target_os = "ios", target_os = "macos")))]
pub fn open_url(_url: &str) {}
