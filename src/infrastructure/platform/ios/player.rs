use objc2::rc::Retained;
use objc2::AnyThread;
use objc2_avf_audio::AVAudioPlayer;
use objc2_foundation::{NSString, NSURL};
use std::cell::RefCell;
use std::sync::Mutex;
use std::time::Instant;

thread_local! {
    static PLAYER: RefCell<Option<Retained<AVAudioPlayer>>> = RefCell::new(None);
}

/// Playback position, counted here rather than read back off `AVAudioPlayer`.
///
/// `PLAYER` is a `thread_local`, so it is invisible to any thread that did not
/// start playback - and nothing guarantees the UI tick runs on that thread.
/// `AVAudioPlayer` plays in real time, so "wall clock since the last play or
/// seek, plus that seek's offset" is the same number, readable from anywhere and
/// with no unsafe: `Instant` and `f64` are both `Send`, the player is not.
struct Playhead {
    started: Instant,
    offset_secs: f64,
}

static PLAYHEAD: Mutex<Option<Playhead>> = Mutex::new(None);

fn set_playhead(offset_secs: f64) {
    *PLAYHEAD.lock().unwrap() = Some(Playhead {
        started: Instant::now(),
        offset_secs,
    });
}

pub fn play_audio(path: &str) {
    play_audio_at(path, 0.0);
}

pub fn play_audio_at(path: &str, start_secs: f64) {
    stop_audio();
    eprintln!("[player] play_audio path: {path} from {start_secs:.2}s");
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
                if start_secs > 0.0 {
                    player.setCurrentTime(start_secs);
                }
                let _: bool = player.play();
                eprintln!("[player] playing OK");
                set_playhead(start_secs);
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

/// Move a running player. Returns false when this thread cannot see the player,
/// which is the caller's cue to restart playback at the offset instead.
pub fn seek_to(secs: f64) -> bool {
    let moved = PLAYER.with(|p| {
        let borrowed = p.borrow();
        let Some(player) = borrowed.as_ref() else {
            return false;
        };
        unsafe { player.setCurrentTime(secs) };
        true
    });
    if moved {
        set_playhead(secs);
    }
    moved
}

pub fn current_time_secs() -> Option<f64> {
    let guard = PLAYHEAD.lock().unwrap();
    let head = guard.as_ref()?;
    Some(head.offset_secs + head.started.elapsed().as_secs_f64())
}

pub fn stop_audio() {
    *PLAYHEAD.lock().unwrap() = None;
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
