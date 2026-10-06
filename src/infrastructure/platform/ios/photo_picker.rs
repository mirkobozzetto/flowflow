//! Photos for Hermes: the system photo picker (several at once) and the
//! camera, each photo redrawn as a JPEG small enough for one request.
//! PHPicker is reached through the runtime: the PhotosUI bindings only
//! expose it on macOS.
use block2::RcBlock;
use objc2::rc::{Allocated, Retained};
use objc2::runtime::{AnyClass, AnyObject};
use objc2::{define_class, msg_send, DefinedClass, MainThreadOnly, Message};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{
    MainThreadMarker, NSArray, NSData, NSDictionary, NSError, NSObject,
    NSObjectProtocol, NSString,
};
use objc2_ui_kit::{
    UIApplication, UIGraphicsImageRenderer, UIGraphicsImageRendererContext,
    UIGraphicsImageRendererFormat, UIImage, UIImagePickerController,
    UIImagePickerControllerDelegate, UIImagePickerControllerOriginalImage,
    UIImagePickerControllerSourceType, UINavigationControllerDelegate,
    UIViewController,
};
use std::cell::Cell;
use std::ptr::NonNull;
use tokio::sync::oneshot;

// Longest side sent: what vision models read in full, and several photos
// still fit under the server's 10 MB request cap.
const MAX_SIDE: f64 = 1568.0;
const JPEG_QUALITY: f64 = 0.8;
const PICK_LIMIT: isize = 6;

#[link(name = "PhotosUI", kind = "framework")]
extern "C" {}

type Picked<T> = Cell<Option<oneshot::Sender<T>>>;

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "FlowFlowPhotosDelegate"]
    #[ivars = Picked<Vec<Retained<AnyObject>>>]
    struct PhotosDelegate;

    unsafe impl NSObjectProtocol for PhotosDelegate {}

    impl PhotosDelegate {
        #[unsafe(method(picker:didFinishPicking:))]
        fn did_finish(&self, picker: &UIViewController, results: &NSArray<AnyObject>) {
            picker.dismissViewControllerAnimated_completion(true, None);
            let providers = results
                .iter()
                .map(|r| unsafe { msg_send![&*r, itemProvider] })
                .collect();
            if let Some(tx) = self.ivars().take() {
                let _ = tx.send(providers);
            }
        }
    }
);

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "FlowFlowCameraDelegate"]
    #[ivars = Picked<Option<Retained<UIImage>>>]
    struct CameraDelegate;

    unsafe impl NSObjectProtocol for CameraDelegate {}
    unsafe impl UINavigationControllerDelegate for CameraDelegate {}

    unsafe impl UIImagePickerControllerDelegate for CameraDelegate {
        #[unsafe(method(imagePickerController:didFinishPickingMediaWithInfo:))]
        fn did_pick(&self, picker: &UIImagePickerController, info: &NSDictionary<NSString, AnyObject>) {
            picker.dismissViewControllerAnimated_completion(true, None);
            let image = info
                .objectForKey(unsafe { UIImagePickerControllerOriginalImage })
                .and_then(|o| o.downcast::<UIImage>().ok());
            if let Some(tx) = self.ivars().take() {
                let _ = tx.send(image);
            }
        }

        #[unsafe(method(imagePickerControllerDidCancel:))]
        fn did_cancel(&self, picker: &UIImagePickerController) {
            picker.dismissViewControllerAnimated_completion(true, None);
            if let Some(tx) = self.ivars().take() {
                let _ = tx.send(None);
            }
        }
    }
);

// The top view controller, to present over whatever is already shown.
#[allow(deprecated)]
fn presenter(mtm: MainThreadMarker) -> Option<Retained<UIViewController>> {
    let app = UIApplication::sharedApplication(mtm);
    let mut top = app.keyWindow()?.rootViewController()?;
    while let Some(next) = top.presentedViewController() {
        top = next;
    }
    Some(top)
}

// Redrawn at scale 1 so the pixel size is the size asked for; drawing also
// applies the photo's orientation.
fn reduce(image: &UIImage, mtm: MainThreadMarker) -> Vec<u8> {
    let size = unsafe { image.size() };
    let k = (MAX_SIDE / size.width.max(size.height)).min(1.0);
    let target =
        CGSize::new((size.width * k).round(), (size.height * k).round());
    let format = UIGraphicsImageRendererFormat::preferredFormat();
    format.setScale(1.0);
    let renderer = UIGraphicsImageRenderer::initWithSize_format(
        mtm.alloc(),
        target,
        &format,
    );
    let image = image.retain();
    let draw =
        RcBlock::new(move |_ctx: NonNull<UIGraphicsImageRendererContext>| {
            image.drawInRect(CGRect::new(CGPoint::new(0.0, 0.0), target));
        });
    let data = unsafe {
        renderer.JPEGDataWithCompressionQuality_actions(
            JPEG_QUALITY,
            &*draw as *const _ as *mut _,
        )
    };
    data.to_vec()
}

// The provider calls back on a queue of its own: the bytes come back
// through a channel.
async fn load(provider: &AnyObject) -> Option<Vec<u8>> {
    let (tx, rx) = oneshot::channel::<Option<Vec<u8>>>();
    let tx = std::sync::Mutex::new(Some(tx));
    let done = RcBlock::new(move |data: *mut NSData, _error: *mut NSError| {
        let bytes = unsafe { data.as_ref() }.map(|d| d.to_vec());
        if let Some(tx) = tx.lock().ok().and_then(|mut t| t.take()) {
            let _ = tx.send(bytes);
        }
    });
    let kind = NSString::from_str("public.image");
    let _: *mut AnyObject = unsafe {
        msg_send![provider, loadDataRepresentationForTypeIdentifier: &*kind,
            completionHandler: &*done]
    };
    rx.await.ok().flatten()
}

/// Photos chosen in the library, as (name, JPEG); empty when cancelled.
pub async fn pick_photos() -> Vec<(String, Vec<u8>)> {
    let Some(mtm) = MainThreadMarker::new() else {
        return Vec::new();
    };
    let classes = (
        AnyClass::get(c"PHPickerConfiguration"),
        AnyClass::get(c"PHPickerFilter"),
        AnyClass::get(c"PHPickerViewController"),
    );
    let (Some(config_class), Some(filter_class), Some(picker_class)) = classes
    else {
        return Vec::new();
    };
    let Some(presenter) = presenter(mtm) else {
        return Vec::new();
    };
    let (tx, rx) = oneshot::channel();
    let delegate: Retained<PhotosDelegate> = {
        let this = PhotosDelegate::alloc(mtm).set_ivars(Cell::new(Some(tx)));
        unsafe { msg_send![super(this), init] }
    };
    let picker: Retained<UIViewController> = unsafe {
        let config: Retained<AnyObject> = msg_send![config_class, new];
        let _: () = msg_send![&*config, setSelectionLimit: PICK_LIMIT];
        let filter: *mut AnyObject = msg_send![filter_class, imagesFilter];
        let _: () = msg_send![&*config, setFilter: filter];
        let picker: Allocated<UIViewController> =
            msg_send![picker_class, alloc];
        let picker: Retained<UIViewController> =
            msg_send![picker, initWithConfiguration: &*config];
        let _: () = msg_send![&*picker, setDelegate: &*delegate];
        picker
    };
    presenter.presentViewController_animated_completion(&picker, true, None);
    let providers = rx.await.unwrap_or_default();
    let mut photos = Vec::new();
    for (i, provider) in providers.iter().enumerate() {
        let name: Option<Retained<NSString>> =
            unsafe { msg_send![&**provider, suggestedName] };
        let Some(bytes) = load(provider).await else {
            continue;
        };
        let Some(image) = UIImage::imageWithData(&NSData::with_bytes(&bytes))
        else {
            continue;
        };
        let name = name
            .map(|n| n.to_string())
            .unwrap_or_else(|| format!("Photo {}", i + 1));
        photos.push((format!("{name}.jpg"), reduce(&image, mtm)));
    }
    drop(delegate);
    photos
}

/// A photo taken now; None without a camera or when cancelled.
pub async fn take_photo() -> Option<(String, Vec<u8>)> {
    let mtm = MainThreadMarker::new()?;
    let camera = UIImagePickerControllerSourceType::Camera;
    if !UIImagePickerController::isSourceTypeAvailable(camera, mtm) {
        return None;
    }
    let presenter = presenter(mtm)?;
    let (tx, rx) = oneshot::channel();
    let delegate: Retained<CameraDelegate> = {
        let this = CameraDelegate::alloc(mtm).set_ivars(Cell::new(Some(tx)));
        unsafe { msg_send![super(this), init] }
    };
    let picker = UIImagePickerController::new(mtm);
    picker.setSourceType(camera);
    let as_delegate: &AnyObject = &delegate;
    unsafe { picker.setDelegate(Some(as_delegate)) };
    presenter.presentViewController_animated_completion(&picker, true, None);
    let image = rx.await.ok().flatten()?;
    let name = format!("Photo {}.jpg", chrono::Local::now().format("%H.%M"));
    Some((name, reduce(&image, mtm)))
}
