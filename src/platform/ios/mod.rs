pub mod live_activity;
mod parsers;
mod pdf;
mod picker;
mod player;
pub mod reminders;
mod share;
pub mod sync_ffi;

use objc2_avf_audio::{
    AVAudioSession, AVAudioSessionCategoryOptions,
    AVAudioSessionCategoryPlayAndRecord,
};
use std::path::PathBuf;
use std::sync::{mpsc::Sender, Mutex, Once};

static INTERRUPTION_TX: Mutex<
    Option<Sender<crate::services::audio::InterruptionEvent>>,
> = Mutex::new(None);
static OBSERVER_INIT: Once = Once::new();

pub fn set_interruption_sender(
    tx: Sender<crate::services::audio::InterruptionEvent>,
) {
    *INTERRUPTION_TX.lock().unwrap() = Some(tx);
}

pub fn observe_interruptions() {
    OBSERVER_INIT.call_once(|| unsafe {
        let name = objc2_foundation::NSString::from_str(
            "AVAudioSessionInterruptionNotification",
        );
        let block = block2::RcBlock::new(
            |notification: std::ptr::NonNull<
                objc2_foundation::NSNotification,
            >| {
                use crate::services::audio::InterruptionEvent;
                let n = notification.as_ptr();
                let type_key = objc2_foundation::NSString::from_str(
                    "AVAudioSessionInterruptionTypeKey",
                );
                let user_info: *const objc2::runtime::AnyObject =
                    objc2::msg_send![&*n, userInfo];
                if user_info.is_null() {
                    return;
                }
                let type_obj: *const objc2::runtime::AnyObject =
                    objc2::msg_send![&*user_info, objectForKey: &*type_key];
                if type_obj.is_null() {
                    return;
                }
                let type_val: usize =
                    objc2::msg_send![&*type_obj, unsignedIntegerValue];
                let event = if type_val == 1 {
                    InterruptionEvent::Began
                } else {
                    let option_key = objc2_foundation::NSString::from_str(
                        "AVAudioSessionInterruptionOptionKey",
                    );
                    let opt_obj: *const objc2::runtime::AnyObject =
                        objc2::msg_send![&*user_info, objectForKey: &*option_key];
                    let should_resume = if opt_obj.is_null() {
                        false
                    } else {
                        let opts: usize =
                            objc2::msg_send![&*opt_obj, unsignedIntegerValue];
                        opts & 1 != 0
                    };
                    InterruptionEvent::Ended { should_resume }
                };
                if let Some(tx) = INTERRUPTION_TX.lock().unwrap().as_ref() {
                    let _ = tx.send(event);
                }
            },
        );
        let center =
            objc2_foundation::NSNotificationCenter::defaultCenter();
        center.addObserverForName_object_queue_usingBlock(
            Some(&name),
            None,
            None,
            &block,
        );
        eprintln!("[ios] interruption observer registered");
    });
}

pub use parsers::read_file_as_text;
pub use pdf::extract as read_pdf_text;
pub use picker::{open_audio_picker, open_file_picker};
pub use player::{is_playing, play_audio, stop_audio};
pub use share::share_file;

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

pub fn detect_system_language() -> String {
    let result = std::panic::catch_unwind(|| unsafe {
        let cls = objc2::ffi::objc_getClass(c"NSLocale".as_ptr());
        if cls.is_null() {
            return None;
        }
        let locale: *mut objc2::runtime::AnyObject = objc2::msg_send![
            cls as *const objc2::runtime::AnyClass,
            currentLocale
        ];
        if locale.is_null() {
            return None;
        }
        let lang: *const objc2_foundation::NSString =
            objc2::msg_send![&*locale, languageCode];
        if lang.is_null() {
            return None;
        }
        Some((*lang).to_string())
    });
    match result {
        Ok(Some(code)) if code.starts_with("fr") => "fr".to_string(),
        _ => "en".to_string(),
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
        let options = AVAudioSessionCategoryOptions::DefaultToSpeaker
            | AVAudioSessionCategoryOptions::AllowBluetoothA2DP
            | AVAudioSessionCategoryOptions::MixWithOthers;
        if let Err(e) = session.setCategory_withOptions_error(category, options)
        {
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
