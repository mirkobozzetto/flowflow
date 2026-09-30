//! iOS terminates an app that submits Metal work from the background. The
//! platform layer pauses this gate when the app leaves the foreground and
//! waits for work in flight; inference only starts while it is open.

use std::sync::{Condvar, Mutex};
use std::time::Duration;

struct State {
    paused: bool,
    busy: u32,
}

static STATE: Mutex<State> = Mutex::new(State {
    paused: false,
    busy: 0,
});
static CHANGED: Condvar = Condvar::new();

/// GPU work in flight; dropping it lets a pending `pause` return.
pub struct Busy(());

impl Drop for Busy {
    fn drop(&mut self) {
        STATE.lock().unwrap().busy -= 1;
        CHANGED.notify_all();
    }
}

/// Blocks while paused, then marks GPU work as in flight.
pub fn enter() -> Busy {
    let mut state = CHANGED
        .wait_while(STATE.lock().unwrap(), |s| s.paused)
        .unwrap();
    state.busy += 1;
    Busy(())
}

pub fn is_paused() -> bool {
    STATE.lock().unwrap().paused
}

/// Closes the gate and waits up to `timeout` for work in flight to finish.
/// `false` means the wait timed out with GPU work still running.
pub fn pause(timeout: Duration) -> bool {
    let mut state = STATE.lock().unwrap();
    state.paused = true;
    let (_state, wait) = CHANGED
        .wait_timeout_while(state, timeout, |s| s.busy > 0)
        .unwrap();
    !wait.timed_out()
}

pub fn resume() {
    STATE.lock().unwrap().paused = false;
    CHANGED.notify_all();
}
