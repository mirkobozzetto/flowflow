//! UIKit owns presentation and press-drag-release tracking. Actions remain in
//! the existing Dioxus menus; the bridge mirrors their labels/disabled states
//! and dispatches a click only if the originating view is still mounted.
use block2::RcBlock;
use objc2::{msg_send, rc::Retained, runtime::AnyObject};
use objc2_foundation::{MainThreadMarker, NSArray, NSString};
use objc2_ui_kit::{
    NSObjectUIAccessibility, UIAction, UIButton,
    UIContextMenuConfigurationElementOrder, UIImage, UIMenu, UIMenuElement,
    UIMenuElementAttributes,
};
use serde::Deserialize;
use std::ptr::NonNull;

#[derive(Deserialize)]
pub(super) struct Menu {
    pub id: String,
    context: String,
    #[serde(default)]
    label: String,
    items: Vec<Item>,
}

#[derive(Deserialize)]
struct Item {
    id: String,
    title: String,
    symbol: String,
    disabled: bool,
    destructive: bool,
}

fn evaluate(js: &str) {
    let Some(web) = super::web_view() else { return };
    let js = NSString::from_str(js);
    let done: Option<
        &block2::DynBlock<dyn Fn(*mut AnyObject, *mut AnyObject)>,
    > = None;
    unsafe {
        let _: () = msg_send![&*web, evaluateJavaScript: &*js,
            completionHandler: done];
    }
}

pub(super) fn prepare(id: &str) {
    // Run the anchor's pointer-down handler, notably closing the theme picker,
    // without its click handler (which would open a second, web menu).
    evaluate(&format!(
        "document.querySelector('[data-glass=\"{id}\"]')?.dispatchEvent(\
         new PointerEvent('pointerdown', {{bubbles:true}}));"
    ));
}

pub(super) fn attach(button: &UIButton, menu: Menu, mtm: MainThreadMarker) {
    button.setAccessibilityLabel(Some(&NSString::from_str(&menu.label)), mtm);
    if menu.items.is_empty() {
        button.setMenu(None);
        button.setShowsMenuAsPrimaryAction(false);
        return;
    }
    let mut children: Vec<Retained<UIMenuElement>> = Vec::new();
    for item in menu.items {
        // JSON-encode arguments, never interpolate note/user text as JS.
        let args = serde_json::to_string(&(&menu.id, &menu.context, &item.id))
            .expect("menu arguments are strings");
        let callback = RcBlock::new(move |_action: NonNull<UIAction>| {
            evaluate(&format!("window.__ffMenuPick?.(...{args});"));
        });
        let action = unsafe {
            UIAction::actionWithHandler(&*callback as *const _ as *mut _, mtm)
        };
        action.setTitle(&NSString::from_str(&item.title));
        action.setImage(
            UIImage::systemImageNamed(&NSString::from_str(&item.symbol))
                .as_deref(),
        );
        let mut attributes = UIMenuElementAttributes::empty();
        if item.disabled {
            attributes |= UIMenuElementAttributes::Disabled;
        }
        if item.destructive {
            attributes |= UIMenuElementAttributes::Destructive;
        }
        action.setAttributes(attributes);
        children.push(Retained::into_super(action));
    }
    let native =
        UIMenu::menuWithChildren(&NSArray::from_retained_slice(&children), mtm);
    button.setMenu(Some(&native));
    button.setPreferredMenuElementOrder(
        UIContextMenuConfigurationElementOrder::Fixed,
    );
    button.setShowsMenuAsPrimaryAction(true);
}
