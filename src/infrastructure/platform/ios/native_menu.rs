//! UIKit owns presentation and press-drag-release tracking. Actions remain in
//! the existing Dioxus menus; the bridge mirrors their labels/disabled states
//! and dispatches a click only if the originating view is still mounted.
use block2::RcBlock;
use objc2::{msg_send, rc::Retained, runtime::AnyObject};
use objc2_foundation::{MainThreadMarker, NSArray, NSString};
use objc2_ui_kit::{
    NSObjectUIAccessibility, UIAction, UIButton,
    UIContextMenuConfigurationElementOrder, UIImage, UIMenu, UIMenuElement,
    UIMenuElementAttributes, UIMenuElementState, UIMenuOptions,
};
use serde::Deserialize;
use std::ptr::NonNull;

// A symbol name the web can send for the app's own chat icon, which has no
// SF Symbol equivalent.
const CHAT_AI_SYMBOL: &str = "ff.chat.ai";

#[derive(Deserialize)]
pub(super) struct Menu {
    pub id: String,
    context: String,
    #[serde(default)]
    label: String,
    items: Vec<Item>,
}

/// An action, or with `children` a submenu (a separated group when `inline`).
#[derive(Deserialize)]
struct Item {
    #[serde(default)]
    id: String,
    title: String,
    #[serde(default)]
    symbol: String,
    #[serde(default)]
    subtitle: String,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    destructive: bool,
    #[serde(default)]
    checked: bool,
    #[serde(default)]
    inline: bool,
    #[serde(default)]
    children: Vec<Item>,
}

fn image(symbol: &str, mtm: MainThreadMarker) -> Option<Retained<UIImage>> {
    match symbol {
        "" => None,
        CHAT_AI_SYMBOL => Some(super::glass_burger::chat_ai(mtm, 22.0)),
        name => UIImage::systemImageNamed(&NSString::from_str(name)),
    }
}

fn element(
    item: Item,
    menu_id: &str,
    context: &str,
    mtm: MainThreadMarker,
) -> Retained<UIMenuElement> {
    let image = image(&item.symbol, mtm);
    // Actions carry an id; a submenu or a group has none.
    if item.id.is_empty() {
        let children: Vec<Retained<UIMenuElement>> = item
            .children
            .into_iter()
            .map(|c| element(c, menu_id, context, mtm))
            .collect();
        let options = if item.inline {
            UIMenuOptions::DisplayInline
        } else {
            UIMenuOptions::empty()
        };
        let menu = UIMenu::menuWithTitle_image_identifier_options_children(
            &NSString::from_str(&item.title),
            image.as_deref(),
            None,
            options,
            &NSArray::from_retained_slice(&children),
            mtm,
        );
        return Retained::into_super(menu);
    }
    // JSON-encode arguments, never interpolate note/user text as JS.
    let args = serde_json::to_string(&(menu_id, context, &item.id))
        .expect("menu arguments are strings");
    let callback = RcBlock::new(move |_action: NonNull<UIAction>| {
        evaluate(&format!("window.__ffMenuPick?.(...{args});"));
    });
    let action = unsafe {
        UIAction::actionWithHandler(&*callback as *const _ as *mut _, mtm)
    };
    action.setTitle(&NSString::from_str(&item.title));
    action.setImage(image.as_deref());
    if !item.subtitle.is_empty() {
        action.setSubtitle(Some(&NSString::from_str(&item.subtitle)));
    }
    if item.checked {
        action.setState(UIMenuElementState::On);
    }
    let mut attributes = UIMenuElementAttributes::empty();
    if item.disabled {
        attributes |= UIMenuElementAttributes::Disabled;
    }
    if item.destructive {
        attributes |= UIMenuElementAttributes::Destructive;
    }
    action.setAttributes(attributes);
    Retained::into_super(action)
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

/// Menu shown or dismissed: the web turns its "+" into a cross and back.
pub(super) fn report_open(open: bool) {
    evaluate(&format!("window.__ffMenuOpen?.({open});"));
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
    let children: Vec<Retained<UIMenuElement>> = menu
        .items
        .into_iter()
        .map(|item| element(item, &menu.id, &menu.context, mtm))
        .collect();
    let native =
        UIMenu::menuWithChildren(&NSArray::from_retained_slice(&children), mtm);
    button.setMenu(Some(&native));
    button.setPreferredMenuElementOrder(
        UIContextMenuConfigurationElementOrder::Fixed,
    );
    button.setShowsMenuAsPrimaryAction(true);
}
