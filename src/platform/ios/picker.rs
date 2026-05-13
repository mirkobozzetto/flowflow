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
use std::path::PathBuf;

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
