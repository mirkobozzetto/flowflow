use objc2_avf_audio::{AVAudioSession, AVAudioSessionCategoryPlayAndRecord};
use std::path::PathBuf;

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
        let category = AVAudioSessionCategoryPlayAndRecord
            .expect("PlayAndRecord category unavailable");
        session
            .setCategory_error(category)
            .expect("Failed to set audio session category");
        session
            .setActive_error(true)
            .expect("Failed to activate audio session");
    }
    eprintln!("[ios] AVAudioSession ready");
}

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, DefinedClass, MainThreadOnly};
use objc2_foundation::{
    MainThreadMarker, NSArray, NSObject, NSObjectProtocol, NSString, NSURL,
};
use objc2_ui_kit::{
    UIApplication, UIDocumentPickerDelegate, UIDocumentPickerViewController,
};
use objc2_uniform_type_identifiers::UTType;
use std::cell::Cell;
use std::ops::Deref;

struct PickerDelegateIvars {
    is_done: Cell<bool>,
    was_cancelled: Cell<bool>,
    picked_paths: Cell<Vec<PathBuf>>,
}

impl PickerDelegateIvars {
    fn new() -> Self {
        Self {
            is_done: Cell::new(false),
            was_cancelled: Cell::new(false),
            picked_paths: Cell::new(Vec::new()),
        }
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "FlowFlowPickerDelegate"]
    #[ivars = PickerDelegateIvars]
    struct PickerDelegate;

    unsafe impl NSObjectProtocol for PickerDelegate {}

    unsafe impl UIDocumentPickerDelegate for PickerDelegate {
        #[unsafe(method(documentPicker:didPickDocumentsAtURLs:))]
        fn document_picker_did_pick_documents_at_urls(
            &self,
            _picker: &UIDocumentPickerViewController,
            urls: &NSArray<NSURL>,
        ) {
            let mut paths: Vec<PathBuf> = Vec::with_capacity(urls.count());
            for i in 0..urls.count() {
                let url = unsafe { urls.objectAtIndex(i) };
                if let Some(path) = unsafe { url.path() } {
                    paths.push(PathBuf::from(path.to_string()));
                }
            }
            self.ivars().picked_paths.set(paths);
            self.ivars().is_done.set(true);
        }

        #[unsafe(method(documentPickerWasCancelled:))]
        fn document_picker_was_cancelled(
            &self,
            _picker: &UIDocumentPickerViewController,
        ) {
            self.ivars().was_cancelled.set(true);
            self.ivars().is_done.set(true);
        }
    }
);

impl PickerDelegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(PickerDelegateIvars::new());
        unsafe { msg_send![super(this), init] }
    }
}

pub async fn open_file_picker(extensions: &[&str]) -> Option<Vec<PathBuf>> {
    let mtm = MainThreadMarker::new().unwrap();
    let app = UIApplication::sharedApplication(mtm);

    unsafe {
        let ut_types: Vec<Retained<UTType>> = extensions
            .iter()
            .filter_map(|ext| {
                UTType::typeWithFilenameExtension(&NSString::from_str(ext))
            })
            .collect();
        let ut_refs: Vec<&UTType> =
            ut_types.iter().map(|t| t.deref()).collect();
        let types_array = NSArray::from_slice(&ut_refs);

        let picker = UIDocumentPickerViewController::alloc(mtm);
        let picker =
            UIDocumentPickerViewController::initForOpeningContentTypes_asCopy(
                picker,
                &types_array,
                true,
            );

        let delegate = PickerDelegate::new(mtm);
        picker.setDelegate(Some(ProtocolObject::from_ref(delegate.deref())));
        picker.setAllowsMultipleSelection(false);

        let window = app.keyWindow().unwrap();
        let current_vc = window.rootViewController().unwrap();
        current_vc
            .presentViewController_animated_completion(&picker, true, None);

        while !delegate.ivars().is_done.get() {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        if delegate.ivars().was_cancelled.get() {
            None
        } else {
            Some(delegate.ivars().picked_paths.take())
        }
    }
}

pub fn read_file_as_text(path: &std::path::Path) -> Result<String, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "txt" | "md" | "csv" => std::fs::read_to_string(path)
            .map_err(|e| format!("Erreur lecture: {e}")),
        _ => Err(format!("Format non supporté: .{ext}")),
    }
}
