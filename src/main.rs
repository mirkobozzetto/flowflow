fn main() {
    dotenvy::dotenv().ok();
    #[cfg(target_os = "ios")]
    flowflow::platform::ios::configure_audio_session();
    #[cfg(target_os = "ios")]
    flowflow::platform::ios::hide_keyboard_accessory();
    dioxus::launch(flowflow::ui::App);
}
