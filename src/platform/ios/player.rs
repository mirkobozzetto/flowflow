use objc2::rc::{Id, Owned};
use objc2::{msg_send, msg_send_id};
use objc2_avf_audio::AVAudioPlayer;
use objc2_foundation::{NSError, NSString, NSURL};
use std::sync::Mutex;
use std::sync::OnceLock;

static PLAYER: OnceLock<Mutex<Option<Id<AVAudioPlayer, Owned>>>> =
    OnceLock::new();

fn player_lock() -> &'static Mutex<Option<Id<AVAudioPlayer, Owned>>> {
    PLAYER.get_or_init(|| Mutex::new(None))
}

pub fn play_audio(path: &str) {
    stop_audio();

    unsafe {
        let file_path = NSString::from_str(path);
        let url = NSURL::fileURLWithPath(&file_path);

        let mut error: *mut NSError = std::ptr::null_mut();
        let player: *mut AVAudioPlayer = msg_send_id![
            msg_send_id![AVAudioPlayer::class(), alloc],
            initWithContentsOfURL: &url,
            error: &mut error,
            _: ()
        ];

        if !error.is_null() {
            eprintln!("[player] failed to init AVAudioPlayer");
            return;
        }

        if let Ok(p) = Id::try_retain(player) {
            let _: () = msg_send![&p, play];
            if let Ok(mut guard) = player_lock().lock() {
                *guard = Some(p);
            }
        }
    }
}

pub fn stop_audio() {
    if let Ok(mut guard) = player_lock().lock() {
        if let Some(ref player) = *guard {
            unsafe {
                let _: () = msg_send![player, stop];
            }
        }
        *guard = None;
    }
}

pub fn is_playing() -> bool {
    if let Ok(guard) = player_lock().lock() {
        if let Some(ref player) = *guard {
            unsafe {
                let playing: bool = msg_send![player, isPlaying];
                return playing;
            }
        }
    }
    false
}
