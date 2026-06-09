use objc2_foundation::{
    NSDictionary, NSFileManager,
    NSFileProtectionCompleteUntilFirstUserAuthentication, NSFileProtectionKey,
    NSNotificationCenter, NSString,
};
use std::path::Path;
use std::sync::Once;

static CHECKPOINT_OBSERVER: Once = Once::new();

// RFC 0004 T16: NSFileProtection class CompleteUntilFirstUserAuthentication
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

// RFC 0004 T16: checkpoint the WAL when the app moves to the background so a
// later lock/eviction never catches a fat WAL mid-flight. The work runs on a
// short-lived thread; iOS grants a few seconds of grace after this
// notification, enough for a passive checkpoint of our small DB.
pub fn observe_background_checkpoint() {
    CHECKPOINT_OBSERVER.call_once(|| unsafe {
        let name =
            NSString::from_str("UIApplicationDidEnterBackgroundNotification");
        let block = block2::RcBlock::new(
            |_n: std::ptr::NonNull<objc2_foundation::NSNotification>| {
                std::thread::spawn(|| match crate::db::Database::open() {
                    Ok(db) => match db.checkpoint_wal() {
                        Ok(()) => {
                            eprintln!("[sync] wal checkpoint on background")
                        }
                        Err(e) => eprintln!("[sync] wal checkpoint: {e}"),
                    },
                    Err(e) => eprintln!("[sync] checkpoint db open: {e}"),
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
