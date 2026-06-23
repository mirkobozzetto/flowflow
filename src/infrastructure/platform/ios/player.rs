use objc2::rc::Retained;
use objc2::AnyThread;
use objc2_avf_audio::AVAudioPlayer;
use objc2_foundation::{NSString, NSURL};
use std::cell::RefCell;

thread_local! {
    static PLAYER: RefCell<Option<Retained<AVAudioPlayer>>> = RefCell::new(None);
}

pub fn play_audio(path: &str) {
    stop_audio();
    eprintln!("[player] play_audio path: {path}");
    let path_obj = std::path::Path::new(path);
    if !path_obj.exists() {
        eprintln!("[player] file not found: {path}");
        return;
    }
    unsafe {
        let ns_path = NSString::from_str(path);
        let url = NSURL::fileURLWithPath(&ns_path);
        match AVAudioPlayer::initWithContentsOfURL_error(
            AVAudioPlayer::alloc(),
            &url,
        ) {
            Ok(player) => {
                let _: bool = player.play();
                eprintln!("[player] playing OK");
                PLAYER.with(|p| {
                    *p.borrow_mut() = Some(player);
                });
            }
            Err(e) => {
                eprintln!("[player] AVAudioPlayer init failed: {e}");
            }
        }
    }
}

pub fn stop_audio() {
    PLAYER.with(|p| {
        if let Some(ref player) = *p.borrow() {
            unsafe {
                player.stop();
            }
        }
        *p.borrow_mut() = None;
    });
}

pub fn is_playing() -> bool {
    PLAYER.with(|p| {
        if let Some(ref player) = *p.borrow() {
            unsafe { player.isPlaying() }
        } else {
            false
        }
    })
}
