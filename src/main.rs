mod audio;
#[cfg(target_os = "ios")]
mod ios;
mod soniox;
mod ui;

fn main() {
    dotenvy::dotenv().ok();
    #[cfg(target_os = "ios")]
    ios::configure_audio_session();
    dioxus::launch(ui::App);
}
