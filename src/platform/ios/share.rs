use objc2::runtime::AnyObject;
use objc2::MainThreadOnly;
use objc2_foundation::{MainThreadMarker, NSArray, NSString, NSURL};
use objc2_ui_kit::{UIActivityViewController, UIApplication};
use std::path::Path;

#[allow(deprecated)]
pub fn share_file(path: &Path) -> Result<(), String> {
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| "share_file must run on the main thread".to_string())?;
    if !path.is_file() {
        return Err(format!("share_file: missing file {}", path.display()));
    }
    unsafe {
        let ns_path = NSString::from_str(&path.to_string_lossy());
        let url = NSURL::fileURLWithPath(&ns_path);
        let item: &AnyObject = url.as_ref();
        let items = NSArray::from_slice(&[item]);
        let controller =
            UIActivityViewController::initWithActivityItems_applicationActivities(
                UIActivityViewController::alloc(mtm),
                &items,
                None,
            );
        let app = UIApplication::sharedApplication(mtm);
        let window = app
            .keyWindow()
            .ok_or_else(|| "share_file: no key window".to_string())?;
        let root = window
            .rootViewController()
            .ok_or_else(|| "share_file: no root view controller".to_string())?;
        root.presentViewController_animated_completion(&controller, true, None);
    }
    eprintln!("[backup] share sheet presented for {}", path.display());
    Ok(())
}
