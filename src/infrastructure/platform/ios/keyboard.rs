//! The keyboard's real height, from UIKit. The web side guessed it from
//! `visualViewport`, which third-party keyboards (Gboard) report late or not
//! at all, leaving the composer under the keys. UIKit posts the final frame
//! for every keyboard, so it is pushed to `window.__ffKeyboard` (inset.rs).
use objc2::msg_send;
use objc2::runtime::{AnyClass, AnyObject};
use objc2_core_foundation::CGRect;
use objc2_foundation::{NSNotification, NSNotificationCenter, NSString};
use std::ptr::NonNull;
use std::sync::Once;

static KEYBOARD_INIT: Once = Once::new();

fn push(height: f64) {
    let Some(web) = super::web_view() else { return };
    let js = NSString::from_str(&format!(
        "window.__ffKeyboard && window.__ffKeyboard({height:.0});"
    ));
    let done: Option<
        &block2::DynBlock<dyn Fn(*mut AnyObject, *mut AnyObject)>,
    > = None;
    unsafe {
        let _: () =
            msg_send![&*web, evaluateJavaScript: &*js, completionHandler: done];
    }
}

/// Height of the keyboard over the bottom of the screen, 0 when it is off
/// screen (hidden, undocked or moved away).
fn overlap(n: &NSNotification) -> f64 {
    unsafe {
        let info: *const AnyObject = msg_send![n, userInfo];
        let Some(info) = info.as_ref() else {
            return 0.0;
        };
        let key = NSString::from_str("UIKeyboardFrameEndUserInfoKey");
        let value: *const AnyObject = msg_send![info, objectForKey: &*key];
        let Some(value) = value.as_ref() else {
            return 0.0;
        };
        let frame: CGRect = msg_send![value, CGRectValue];
        let Some(screen_cls) = AnyClass::get(c"UIScreen") else {
            return 0.0;
        };
        let screen: *const AnyObject = msg_send![screen_cls, mainScreen];
        let Some(screen) = screen.as_ref() else {
            return 0.0;
        };
        let bounds: CGRect = msg_send![screen, bounds];
        (bounds.size.height - frame.origin.y).max(0.0)
    }
}

pub fn observe_keyboard() {
    KEYBOARD_INIT.call_once(|| {
        let center = NSNotificationCenter::defaultCenter();
        let change = block2::RcBlock::new(|n: NonNull<NSNotification>| {
            push(overlap(unsafe { n.as_ref() }));
        });
        let hide = block2::RcBlock::new(|_n: NonNull<NSNotification>| {
            push(0.0);
        });
        unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(&NSString::from_str(
                    "UIKeyboardWillChangeFrameNotification",
                )),
                None,
                None,
                &change,
            );
            center.addObserverForName_object_queue_usingBlock(
                Some(&NSString::from_str("UIKeyboardWillHideNotification")),
                None,
                None,
                &hide,
            );
        }
        eprintln!("[ios] keyboard observer registered");
    });
}
