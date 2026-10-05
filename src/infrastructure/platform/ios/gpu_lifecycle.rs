use crate::infrastructure::transcription::gpu_gate;
use objc2_foundation::{NSNotification, NSNotificationCenter, NSString};
use std::ptr::NonNull;
use std::sync::Once;
use std::time::Duration;

static OBSERVERS: Once = Once::new();

// Metal work committed after the app returns from the background notification
// is rejected, and ggml aborts on the failed command buffer. The observer runs
// on the main thread, so blocking it holds the transition until the inference
// step in flight is done. iOS grants about five seconds there; the wait stays
// under that so the watchdog never kills the app over it.
const DRAIN_TIMEOUT: Duration = Duration::from_secs(3);

/// Pauses on background and reopens once the app is active again, the latest
/// and safest point of the return to the foreground.
pub fn observe_gpu_lifecycle() {
    super::continued_task::register();
    OBSERVERS.call_once(|| unsafe {
        let center = NSNotificationCenter::defaultCenter();
        let pause = block2::RcBlock::new(|_n: NonNull<NSNotification>| {
            if !gpu_gate::pause(DRAIN_TIMEOUT) {
                eprintln!("[whisper] GPU work still in flight on background");
            }
        });
        center.addObserverForName_object_queue_usingBlock(
            Some(&NSString::from_str(
                "UIApplicationDidEnterBackgroundNotification",
            )),
            None,
            None,
            &pause,
        );
        let resume = block2::RcBlock::new(|_n: NonNull<NSNotification>| {
            gpu_gate::resume();
        });
        center.addObserverForName_object_queue_usingBlock(
            Some(&NSString::from_str(
                "UIApplicationDidBecomeActiveNotification",
            )),
            None,
            None,
            &resume,
        );
    });
}
