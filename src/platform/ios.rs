use objc2_avf_audio::{AVAudioSession, AVAudioSessionCategoryPlayAndRecord};
use std::path::PathBuf;

pub fn documents_dir() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME not set on iOS");
    PathBuf::from(home).join("Documents")
}

pub fn configure_audio_session() {
    eprintln!("[ios] configuring AVAudioSession");
    unsafe {
        let session = AVAudioSession::sharedInstance();
        let category = AVAudioSessionCategoryPlayAndRecord
            .expect("PlayAndRecord category unavailable");
        session
            .setCategory_error(category)
            .expect("Failed to set audio session category");
        session
            .setActive_error(true)
            .expect("Failed to activate audio session");
    }
    eprintln!("[ios] AVAudioSession ready");
}
