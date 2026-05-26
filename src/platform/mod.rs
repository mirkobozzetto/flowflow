#[cfg(target_os = "ios")]
pub mod ios;

#[cfg(target_os = "ios")]
pub fn detect_system_language() -> String {
    ios::detect_system_language()
}

#[cfg(not(target_os = "ios"))]
pub fn detect_system_language() -> String {
    "en".to_string()
}
