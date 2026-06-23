use objc2::msg_send;
use objc2::runtime::AnyObject;
use objc2_foundation::{
    NSDictionary, NSFileManager,
    NSFileProtectionCompleteUntilFirstUserAuthentication, NSFileProtectionKey,
    NSNotificationCenter, NSString,
};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Once};

static CHECKPOINT_OBSERVER: Once = Once::new();
static RESTORE_FOREGROUND_OBSERVER: Once = Once::new();

// UIBackgroundTaskInvalid (UIKit, linked): the sentinel UIApplication returns
// when no background time can be granted, and the value endBackgroundTask
// must never be called with.
fn bg_task_invalid() -> usize {
    unsafe { objc2_ui_kit::UIBackgroundTaskInvalid }
}

// A short grace window so a sync session survives the user
// backgrounding the app mid-transfer. Begin/end are documented thread-safe,
// but the objc2 typed UIApplication API is gated on MainThreadMarker, so the
// two calls go through the dynamic runtime instead. If the window expires
// before the session ends, iOS fires the expiration handler: the task is
// ended there (mandatory, the watchdog kills the app otherwise) and the cut
// transfer resumes at the next session (T17 watermark).
pub struct BackgroundTaskGuard {
    task_id: Arc<AtomicUsize>,
}

impl BackgroundTaskGuard {
    pub fn begin() -> Self {
        let task_id = Arc::new(AtomicUsize::new(bg_task_invalid()));
        // Covers the expiry-vs-begin race: the handler can fire on the main
        // queue BEFORE the worker thread stored the returned id. The handler
        // then sees `invalid`, raises `expired` instead of ending nothing,
        // and begin() ends the task itself right after the store. Every
        // interleaving ends the task exactly once (end_bg_task swaps to
        // invalid), so the watchdog never catches an unended assertion.
        let expired = Arc::new(AtomicBool::new(false));
        let for_expiry = task_id.clone();
        let expired_flag = expired.clone();
        unsafe {
            let block = block2::RcBlock::new(move || {
                expired_flag.store(true, Ordering::SeqCst);
                end_bg_task(&for_expiry);
            });
            let app: *mut AnyObject =
                msg_send![objc2::class!(UIApplication), sharedApplication];
            if !app.is_null() {
                let id: usize = msg_send![
                    &*app,
                    beginBackgroundTaskWithExpirationHandler: &*block
                ];
                task_id.store(id, Ordering::SeqCst);
                if expired.load(Ordering::SeqCst) {
                    end_bg_task(&task_id);
                }
            }
        }
        Self { task_id }
    }
}

impl Drop for BackgroundTaskGuard {
    fn drop(&mut self) {
        end_bg_task(&self.task_id);
    }
}

// Swap-to-invalid guarantees end-once even when the expiration handler and
// the guard drop race each other.
fn end_bg_task(task_id: &Arc<AtomicUsize>) {
    let invalid = bg_task_invalid();
    let id = task_id.swap(invalid, Ordering::SeqCst);
    if id == invalid {
        return;
    }
    unsafe {
        let app: *mut AnyObject =
            msg_send![objc2::class!(UIApplication), sharedApplication];
        if !app.is_null() {
            let _: () = msg_send![&*app, endBackgroundTask: id];
        }
    }
}

// NSFileProtection class CompleteUntilFirstUserAuthentication
// (NOT Complete) on the SQLite file family. Complete would make the mmapped
// -wal/-shm unreadable once the screen locks and corrupt a foreground sync
// that survives the lock; CompleteUntilFirstUserAuthentication keeps them
// readable after first unlock while still encrypting at rest.
pub fn protect_db_files(db_path: &Path) {
    let base = db_path.to_string_lossy().to_string();
    for path in [base.clone(), format!("{base}-wal"), format!("{base}-shm")] {
        if Path::new(&path).exists() {
            protect_path(&path);
        }
    }
}

fn protect_path(path: &str) {
    unsafe {
        let fm = NSFileManager::defaultManager();
        let ns_path = NSString::from_str(path);
        let dict = NSDictionary::from_slices(
            &[NSFileProtectionKey],
            &[NSFileProtectionCompleteUntilFirstUserAuthentication],
        );
        let attrs = &*(core::ptr::from_ref(&*dict)
            as *const NSDictionary<NSString, objc2::runtime::AnyObject>);
        match fm.setAttributes_ofItemAtPath_error(attrs, &ns_path) {
            Ok(()) => eprintln!("[sync] file protection set on {path}"),
            Err(e) => {
                eprintln!("[sync] file protection failed on {path}: {e}")
            }
        }
    }
}

pub fn observe_restore_foreground() {
    RESTORE_FOREGROUND_OBSERVER.call_once(|| unsafe {
        let name =
            NSString::from_str("UIApplicationWillEnterForegroundNotification");
        let block = block2::RcBlock::new(
            |_n: std::ptr::NonNull<objc2_foundation::NSNotification>| {
                if crate::application::backup::pending_restore_dir()
                    .join(crate::application::backup::DB_ENTRY_PATH)
                    .exists()
                {
                    crate::application::backup::activate_restore_lock();
                }
                crate::infrastructure::sync::engine::sync_now_if_live();
            },
        );
        let center = NSNotificationCenter::defaultCenter();
        center.addObserverForName_object_queue_usingBlock(
            Some(&name),
            None,
            None,
            &block,
        );
        eprintln!("[backup] restore foreground observer registered");
    });
}

// Checkpoint the WAL when the app moves to the background so a
// later lock/eviction never catches a fat WAL mid-flight. The work runs on a
// short-lived thread; iOS grants a few seconds of grace after this
// notification, enough for a passive checkpoint of our small DB.
pub fn observe_background_checkpoint() {
    CHECKPOINT_OBSERVER.call_once(|| unsafe {
        let name =
            NSString::from_str("UIApplicationDidEnterBackgroundNotification");
        let block = block2::RcBlock::new(
            |_n: std::ptr::NonNull<objc2_foundation::NSNotification>| {
                std::thread::spawn(|| {
                    match crate::infrastructure::persistence::Database::open() {
                        Ok(db) => match db.checkpoint_wal() {
                            Ok(()) => {
                                eprintln!("[sync] wal checkpoint on background")
                            }
                            Err(e) => eprintln!("[sync] wal checkpoint: {e}"),
                        },
                        Err(e) => eprintln!("[sync] checkpoint db open: {e}"),
                    }
                });
            },
        );
        let center = NSNotificationCenter::defaultCenter();
        center.addObserverForName_object_queue_usingBlock(
            Some(&name),
            None,
            None,
            &block,
        );
        eprintln!("[sync] background checkpoint observer registered");
    });
}
