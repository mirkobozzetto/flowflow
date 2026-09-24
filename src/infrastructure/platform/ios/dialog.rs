//! Native follow-up to a UIKit menu. Dioxus owns the awaiting task, so closing
//! its note/chat also dismisses the dialog and cancels the pending operation.
use block2::RcBlock;
use objc2::rc::Retained;
use objc2_foundation::{MainThreadMarker, NSString};
use objc2_ui_kit::{
    UIAlertAction, UIAlertActionStyle, UIAlertController,
    UIAlertControllerStyle, UIApplication, UIUserInterfaceStyle,
};
use std::{cell::RefCell, ptr::NonNull, rc::Rc};

struct PresentedAlert {
    alert: Retained<UIAlertController>,
    dismiss_on_drop: bool,
}

impl Drop for PresentedAlert {
    fn drop(&mut self) {
        if self.dismiss_on_drop
            && self.alert.presentingViewController().is_some()
        {
            self.alert
                .dismissViewControllerAnimated_completion(false, None);
        }
    }
}

/// None means cancel (or no available presenter). With `initial`, confirmation
/// returns the edited text; without it, confirmation returns an empty string.
#[allow(deprecated)]
pub async fn present(
    title: &str,
    message: Option<&str>,
    initial: Option<&str>,
    cancel_label: &str,
    confirm_label: &str,
    destructive: bool,
) -> Option<String> {
    let mtm = MainThreadMarker::new()?;
    let app = UIApplication::sharedApplication(mtm);
    let presenter = app.keyWindow()?.rootViewController()?;
    // Do not present over an unrelated modal. UIKit menu dismissal can still
    // be finishing when its action reaches Dioxus.
    for _ in 0..20 {
        let Some(current) = presenter.presentedViewController() else {
            break;
        };
        if !current.isBeingDismissed() {
            return None;
        }
        futures_timer::Delay::new(std::time::Duration::from_millis(25)).await;
    }
    if presenter.presentedViewController().is_some() {
        return None;
    }
    let message = message.map(NSString::from_str);
    let alert =
        UIAlertController::alertControllerWithTitle_message_preferredStyle(
            Some(&NSString::from_str(title)),
            message.as_deref(),
            UIAlertControllerStyle::Alert,
            mtm,
        );
    alert.setOverrideUserInterfaceStyle(UIUserInterfaceStyle::Light);
    let field = initial.map(|value| {
        alert.addTextFieldWithConfigurationHandler(None);
        let field = alert
            .textFields()
            .expect("added text field")
            .objectAtIndex(0);
        field.setText(Some(&NSString::from_str(value)));
        field
    });
    let (tx, rx) = tokio::sync::oneshot::channel();
    let tx = Rc::new(RefCell::new(Some(tx)));
    let cancel_tx = tx.clone();
    let cancel = RcBlock::new(move |_action: NonNull<UIAlertAction>| {
        if let Some(tx) = cancel_tx.borrow_mut().take() {
            let _ = tx.send(None);
        }
    });
    let confirm = RcBlock::new(move |_action: NonNull<UIAlertAction>| {
        let value = field
            .as_ref()
            .and_then(|f| f.text())
            .map(|s| s.to_string())
            .unwrap_or_default();
        if let Some(tx) = tx.borrow_mut().take() {
            let _ = tx.send(Some(value));
        }
    });
    let cancel = UIAlertAction::actionWithTitle_style_handler(
        Some(&NSString::from_str(cancel_label)),
        UIAlertActionStyle::Cancel,
        Some(&cancel),
        mtm,
    );
    let confirm = UIAlertAction::actionWithTitle_style_handler(
        Some(&NSString::from_str(confirm_label)),
        if destructive {
            UIAlertActionStyle::Destructive
        } else {
            UIAlertActionStyle::Default
        },
        Some(&confirm),
        mtm,
    );
    alert.addAction(&cancel);
    alert.addAction(&confirm);
    alert.setPreferredAction(Some(if destructive {
        &cancel
    } else {
        &confirm
    }));
    let mut guard = PresentedAlert {
        alert,
        dismiss_on_drop: true,
    };
    presenter.presentViewController_animated_completion(
        &guard.alert,
        true,
        None,
    );
    let result = rx.await.ok().flatten();
    // UIKit dismisses after an action; the guard only handles scope cancellation.
    guard.dismiss_on_drop = false;
    result
}
