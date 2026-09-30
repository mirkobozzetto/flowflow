//! iOS terminates an app that submits Metal work from the background. The
//! platform layer pauses this gate when the app leaves the foreground and
//! waits for work in flight; GPU inference only starts while it is open.
//! While iOS keeps a background task alive for the job, the CPU takes over.

use std::sync::{Condvar, Mutex};
use std::time::Duration;

struct State {
    paused: bool,
    busy: u32,
    cpu_allowed: bool,
}

static STATE: Mutex<State> = Mutex::new(State {
    paused: false,
    busy: 0,
    cpu_allowed: false,
});
static CHANGED: Condvar = Condvar::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Engine {
    Gpu,
    Cpu,
}

/// Inference in flight; dropping GPU work lets a pending `pause` return.
pub struct Busy {
    pub engine: Engine,
}

impl Drop for Busy {
    fn drop(&mut self) {
        if self.engine == Engine::Gpu {
            STATE.lock().unwrap().busy -= 1;
            CHANGED.notify_all();
        }
    }
}

/// Blocks until an engine may run: the GPU in the foreground, the CPU in the
/// background while `allow_cpu` holds.
pub fn enter() -> Busy {
    let mut state = CHANGED
        .wait_while(STATE.lock().unwrap(), |s| s.paused && !s.cpu_allowed)
        .unwrap();
    if state.paused {
        return Busy {
            engine: Engine::Cpu,
        };
    }
    state.busy += 1;
    Busy {
        engine: Engine::Gpu,
    }
}

/// CPU work is not stopped by a return to the foreground: its chunk ends on
/// the CPU and the next one goes back to the GPU.
pub fn must_stop(engine: Engine) -> bool {
    let state = STATE.lock().unwrap();
    match engine {
        Engine::Gpu => state.paused,
        Engine::Cpu => !state.cpu_allowed,
    }
}

pub fn allow_cpu(allowed: bool) {
    STATE.lock().unwrap().cpu_allowed = allowed;
    CHANGED.notify_all();
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
