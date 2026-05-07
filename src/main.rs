mod db;
mod models;
mod platform;
mod services;
mod ui;

fn main() {
    dotenvy::dotenv().ok();
    #[cfg(target_os = "ios")]
    platform::ios::configure_audio_session();
    dioxus::launch(ui::App);
}
