mod parsers;
mod picker;
mod player;

use objc2_avf_audio::{AVAudioSession, AVAudioSessionCategoryPlayAndRecord};
use std::path::PathBuf;

pub use parsers::read_file_as_text;
pub use picker::open_file_picker;
pub use player::{is_playing, play_audio, stop_audio};

pub fn hide_keyboard_accessory() {
    unsafe {
        let cls = objc2::ffi::objc_getClass(c"WKContentView".as_ptr());
        if cls.is_null() {
            eprintln!("[ios] WKContentView not found");
            return;
        }
        let Some(sel) =
            objc2::ffi::sel_registerName(c"inputAccessoryView".as_ptr())
        else {
            eprintln!("[ios] selector not found");
            return;
        };
        let method = objc2::ffi::class_getInstanceMethod(cls as *const _, sel);
        if method.is_null() {
            eprintln!("[ios] method not found");
            return;
        }
        unsafe extern "C-unwind" fn nil_view(
            _this: *mut std::ffi::c_void,
            _sel: *mut std::ffi::c_void,
        ) -> *mut std::ffi::c_void {
            std::ptr::null_mut()
        }
        let imp: unsafe extern "C-unwind" fn() =
            std::mem::transmute(nil_view as *const ());
        objc2::ffi::method_setImplementation(method, imp);
        eprintln!("[ios] keyboard accessory hidden");
    }
}

pub fn documents_dir() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME not set on iOS");
    PathBuf::from(home).join("Documents")
}

pub fn configure_audio_session() {
    eprintln!("[ios] configuring AVAudioSession");
    unsafe {
        let session = AVAudioSession::sharedInstance();
        let Some(category) = AVAudioSessionCategoryPlayAndRecord else {
            eprintln!("[ios] PlayAndRecord category unavailable");
            return;
        };
        if let Err(e) = session.setCategory_error(category) {
            eprintln!("[ios] setCategory failed: {e}");
            return;
        }
        if let Err(e) = session.setActive_error(true) {
            eprintln!("[ios] setActive failed (another app owns audio?): {e}");
            return;
        }
    }
    eprintln!("[ios] AVAudioSession ready");
}
