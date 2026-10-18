use crate::domain::Transcript;
use dioxus::prelude::*;

/// Fast enough that a word boundary is never missed at speech rate, cheap enough
/// that it only costs an atomic read: the position is counted in Rust, so the
/// tick touches no platform API.
const TICK_MS: u64 = 100;

#[derive(Clone, PartialEq)]
pub struct Playback {
    pub audio_id: String,
    pub path: String,
    pub transcript: Transcript,
    pub duration_secs: f64,
}

/// The note's one player. One clip plays at a time, so the reading view and
/// the recordings list drive the same state, which is also what lets the
/// highlighted word follow the audio.
#[derive(Clone, Copy, PartialEq)]
pub struct NotePlayer {
    pub playback: Signal<Option<Playback>>,
    /// Index into the playing clip's words.
    pub active_word: Signal<Option<usize>>,
    /// Where the current clip was paused, so playing it again resumes there.
    pub paused_at: Signal<Option<f64>>,
}

impl NotePlayer {
    /// The clip is loaded, playing or paused.
    pub fn is_current(&self, audio_id: &str) -> bool {
        self.playback
            .read()
            .as_ref()
            .is_some_and(|p| p.audio_id == audio_id)
    }

    pub fn is_playing(&self, audio_id: &str) -> bool {
        self.is_current(audio_id) && self.paused_at.read().is_none()
    }

    /// Play `clip` from `from_secs`, moving in place when it already plays.
    pub fn play(&mut self, clip: Playback, from_secs: f64) {
        if self.is_playing(&clip.audio_id) {
            seek_playback(&clip.path, from_secs);
            return;
        }
        start_playback(&clip.path, from_secs);
        self.paused_at.set(None);
        if !self.is_current(&clip.audio_id) {
            self.active_word.set(None);
            self.playback.set(Some(clip));
        }
    }

    /// Stops the audio but keeps the clip and its highlighted word.
    pub fn pause(&mut self) {
        if self.playback.peek().is_none() || self.paused_at.peek().is_some() {
            return;
        }
        let at = playhead_secs().unwrap_or(0.0);
        stop_playback();
        self.paused_at.set(Some(at));
    }

    /// A play/pause button: pauses `clip` while it plays, resumes it where it
    /// was paused, and otherwise plays it from `from_secs`.
    pub fn toggle(&mut self, clip: Playback, from_secs: f64) {
        if self.is_playing(&clip.audio_id) {
            self.pause();
            return;
        }
        let resume_at = (*self.paused_at.peek())
            .filter(|_| self.is_current(&clip.audio_id));
        self.play(clip, resume_at.unwrap_or(from_secs));
    }

    pub fn stop(&mut self) {
        stop_playback();
        self.playback.set(None);
        self.active_word.set(None);
        self.paused_at.set(None);
    }
}

pub fn use_note_player() -> NotePlayer {
    let mut playback: Signal<Option<Playback>> = use_signal(|| None);
    let mut active_word: Signal<Option<usize>> = use_signal(|| None);
    let paused_at: Signal<Option<f64>> = use_signal(|| None);

    use_future(move || async move {
        loop {
            futures_timer::Delay::new(std::time::Duration::from_millis(
                TICK_MS,
            ))
            .await;
            let Some(current) = playback.peek().clone() else {
                if active_word.peek().is_some() {
                    active_word.set(None);
                }
                continue;
            };
            if paused_at.peek().is_some() {
                continue;
            }
            let Some(elapsed) = playhead_secs() else {
                continue;
            };
            if current.duration_secs > 0.0 && elapsed >= current.duration_secs {
                playback.set(None);
                active_word.set(None);
                continue;
            }
            // Written only when the index changes: at normal speech that is two
            // or three re-renders a second rather than ten.
            let index = current
                .transcript
                .word_index_at_ms((elapsed * 1000.0) as u32);
            if *active_word.peek() != index {
                active_word.set(index);
            }
        }
    });

    NotePlayer {
        playback,
        active_word,
        paused_at,
    }
}

fn playhead_secs() -> Option<f64> {
    #[cfg(target_os = "ios")]
    {
        crate::infrastructure::platform::ios::current_time_secs()
    }
    // Desktop plays through a detached `afplay`, which exposes no position
    // (RFC 0024 non-goal). The transcript renders, inert.
    #[cfg(not(target_os = "ios"))]
    {
        None
    }
}

fn start_playback(path: &str, from_secs: f64) {
    #[cfg(target_os = "ios")]
    crate::infrastructure::platform::ios::play_audio_at(path, from_secs);
    #[cfg(target_os = "macos")]
    {
        let _ = from_secs;
        crate::infrastructure::platform::macos::play_audio(path);
    }
    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    {
        let _ = (path, from_secs);
    }
}

fn stop_playback() {
    #[cfg(target_os = "ios")]
    crate::infrastructure::platform::ios::stop_audio();
    #[cfg(target_os = "macos")]
    crate::infrastructure::platform::macos::stop_audio();
}

/// Move a running clip, restarting it when the platform cannot seek in place.
fn seek_playback(path: &str, to_secs: f64) {
    #[cfg(target_os = "ios")]
    {
        if crate::infrastructure::platform::ios::seek_to(to_secs) {
            return;
        }
    }
    start_playback(path, to_secs);
}
