//! #177: native iOS 26 Liquid Glass buttons laid over web anchors
//! (`[data-glass="<id>"]`): the burger, the chat pill, the new-note button.
//! The web anchor stays the source of truth: it keeps its place, its click
//! handler and, whenever the native button cannot show (before iOS 26, under
//! a web overlay, off its view), its own look. `src/ui/app/glass_burger.ts`
//! reports where each anchor is and how the card they all ride on moves.

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{
    define_class, msg_send, sel, AnyThread, ClassType, MainThreadOnly,
};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{
    MainThreadMarker, NSAttributedString, NSDictionary, NSObject, NSString,
};
use objc2_ui_kit::{
    NSDirectionalEdgeInsets, NSFontAttributeName, UIAction, UIButton,
    UIButtonConfiguration, UIButtonConfigurationCornerStyle, UIColor,
    UIControl, UIControlEvents, UIFont, UIFontWeightMedium,
    UIGraphicsImageRenderer, UIGraphicsImageRendererContext, UIImage,
    UIImageRenderingMode, UIImageSymbolConfiguration, UIImageSymbolWeight,
    UIResponder, UIUserInterfaceStyle, UIView, UIViewAnimating,
    UIViewAnimatingState, UIViewPropertyAnimator,
};

// Anchors whose native button opens a UIMenu mirrored from a hidden DOM menu.
fn has_menu(id: &str) -> bool {
    matches!(id, "note-more" | "chat-more" | "note-plus" | "chat-plus")
}

// The composer's "+": its web glyph stays visible and turns into a cross, so
// the native button over it is clear and only reports the menu opening.
fn is_plus(id: &str) -> bool {
    matches!(id, "note-plus" | "chat-plus")
}

define_class!(
    #[unsafe(super(UIButton, UIControl, UIView, UIResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "FlowFlowPlusMenuButton"]
    struct PlusMenuButton;

    impl PlusMenuButton {
        #[unsafe(method(contextMenuInteraction:willDisplayMenuForConfiguration:animator:))]
        fn will_display(
            &self,
            interaction: &AnyObject,
            configuration: &AnyObject,
            animator: *mut AnyObject,
        ) {
            unsafe {
                let _: () = msg_send![super(self),
                    contextMenuInteraction: interaction,
                    willDisplayMenuForConfiguration: configuration,
                    animator: animator];
            }
            super::native_menu::report_open(true);
        }

        #[unsafe(method(contextMenuInteraction:willEndForConfiguration:animator:))]
        fn will_end(
            &self,
            interaction: &AnyObject,
            configuration: &AnyObject,
            animator: *mut AnyObject,
        ) {
            unsafe {
                let _: () = msg_send![super(self),
                    contextMenuInteraction: interaction,
                    willEndForConfiguration: configuration,
                    animator: animator];
            }
            super::native_menu::report_open(false);
        }
    }
);
use std::cell::RefCell;
use std::ptr::NonNull;

struct Glassed {
    id: &'static str,
    button: Retained<UIButton>,
    badge: Retained<UIView>,
    base: CGRect,
}

struct Glass {
    web: Retained<UIView>,
    buttons: Vec<Glassed>,
    travel: f64,
    p: f64,
    animator: Option<Retained<UIViewPropertyAnimator>>,
}

thread_local! {
    static GLASS: RefCell<Option<Glass>> = const { RefCell::new(None) };
}

// Card travel: same duration and curve as .sb-card (460ms var(--ease-soft)).
const SETTLE_S: f64 = 0.46;
const EASE_SOFT: (CGPoint, CGPoint) =
    (CGPoint { x: 0.2, y: 0.8 }, CGPoint { x: 0.2, y: 1.0 });

fn rgb(r: f64, g: f64, b: f64) -> Retained<UIColor> {
    UIColor::colorWithRed_green_blue_alpha(r / 255.0, g / 255.0, b / 255.0, 1.0)
}
// stone-800, ios-orange-dark, ios-orange (tailwind.css, oklch -> sRGB).
fn stone_800() -> Retained<UIColor> {
    rgb(41.0, 37.0, 36.0)
}
fn orange_dark() -> Retained<UIColor> {
    rgb(160.0, 55.0, 0.0)
}
fn orange() -> Retained<UIColor> {
    rgb(223.0, 67.0, 0.0)
}

// The glass would otherwise tint every glyph with its own dark vibrant
// colour: pin each image to the web anchor's colour.
fn original(image: &UIImage, color: &UIColor) -> Retained<UIImage> {
    image.imageWithTintColor_renderingMode(
        color,
        UIImageRenderingMode::AlwaysOriginal,
    )
}

/// Round-capped strokes `(x1, y1, x2, y2)` in a `view`-unit box drawn at
/// `size` points, `width` units thick: the web SVG glyphs, redrawn.
fn strokes(
    mtm: MainThreadMarker,
    size: f64,
    view: f64,
    width: f64,
    lines: &'static [(f64, f64, f64, f64)],
    color: Retained<UIColor>,
) -> Retained<UIImage> {
    let renderer = UIGraphicsImageRenderer::initWithSize(
        mtm.alloc(),
        CGSize::new(size, size),
    );
    let fill = color.clone();
    let draw = RcBlock::new(
        move |_ctx: NonNull<UIGraphicsImageRendererContext>| {
            fill.setFill();
            let k = size / view;
            let half = width * k / 2.0;
            for &(x1, y1, x2, y2) in lines {
                let (x, y) = (x1.min(x2) * k, y1.min(y2) * k);
                let (w, h) = ((x2 - x1).abs() * k, (y2 - y1).abs() * k);
                let rect = CGRect::new(
                    CGPoint::new(x - half, y - half),
                    CGSize::new(w + 2.0 * half, h + 2.0 * half),
                );
                objc2_ui_kit::UIBezierPath::bezierPathWithRoundedRect_cornerRadius(
                    rect, half,
                )
                .fill();
            }
        },
    );
    let image =
        unsafe { renderer.imageWithActions(&*draw as *const _ as *mut _) };
    original(&image, &color)
}

/// `IconChatAi` (icons.rs), redrawn: the bubble outline in orange-dark, its
/// two lines in the brighter orange, from the same 256-unit viewBox.
pub(super) fn chat_ai(mtm: MainThreadMarker, size: f64) -> Retained<UIImage> {
    let renderer = UIGraphicsImageRenderer::initWithSize(
        mtm.alloc(),
        CGSize::new(size, size),
    );
    let draw = RcBlock::new(
        move |_ctx: NonNull<UIGraphicsImageRendererContext>| {
            let k = size / 256.0;
            let p = |x: f64, y: f64| CGPoint::new(x * k, y * k);
            let pi = std::f64::consts::PI;
            let bubble = objc2_ui_kit::UIBezierPath::bezierPath();
            bubble.moveToPoint(p(40.0, 124.0));
            bubble.addArcWithCenter_radius_startAngle_endAngle_clockwise(
                p(132.0, 124.0),
                92.0 * k,
                pi,
                2.5 * pi,
                true,
            );
            bubble.addLineToPoint(p(48.0, 216.0));
            bubble.addArcWithCenter_radius_startAngle_endAngle_clockwise(
                p(48.0, 208.0),
                8.0 * k,
                0.5 * pi,
                pi,
                true,
            );
            bubble.closePath();
            bubble.setLineWidth(16.0 * k);
            orange_dark().setStroke();
            bubble.stroke();
            rgb(232.0, 106.0, 16.0).setFill();
            for y in [112.0, 144.0] {
                objc2_ui_kit::UIBezierPath::bezierPathWithRoundedRect_cornerRadius(
                    CGRect::new(p(88.0, y - 8.0), CGSize::new(84.0 * k, 16.0 * k)),
                    8.0 * k,
                )
                .fill();
            }
        },
    );
    let image =
        unsafe { renderer.imageWithActions(&*draw as *const _ as *mut _) };
    image.imageWithRenderingMode(UIImageRenderingMode::AlwaysOriginal)
}

fn symbol(name: &str, size: f64, color: &UIColor) -> Option<Retained<UIImage>> {
    let config = UIImageSymbolConfiguration::configurationWithPointSize_weight(
        size,
        UIImageSymbolWeight::Regular,
    );
    let image = UIImage::systemImageNamed_withConfiguration(
        &NSString::from_str(name),
        Some(&config),
    )?;
    Some(original(&image, color))
}

/// The glass look of each known anchor; None for an unknown id.
fn configuration(
    id: &str,
    mtm: MainThreadMarker,
) -> Option<(&'static str, Retained<UIButtonConfiguration>)> {
    if is_plus(id) {
        let config = UIButtonConfiguration::plainButtonConfiguration(mtm);
        let id = if id == "note-plus" {
            "note-plus"
        } else {
            "chat-plus"
        };
        return Some((id, config));
    }
    let config = UIButtonConfiguration::glassButtonConfiguration(mtm);
    config.setCornerStyle(UIButtonConfigurationCornerStyle::Capsule);
    let id = match id {
        "burger" => {
            // top_bar.rs: 28px viewBox at 30px, stroke 1.8.
            config.setImage(Some(&strokes(
                mtm,
                30.0,
                28.0,
                1.8,
                &[(5.0, 10.0, 19.0, 10.0), (5.0, 18.0, 23.0, 18.0)],
                stone_800(),
            )));
            "burger"
        }
        "note-more" | "chat-more" => {
            config.setImage(symbol("ellipsis", 20.0, &stone_800()).as_deref());
            if id == "note-more" {
                "note-more"
            } else {
                "chat-more"
            }
        }
        "chat" => {
            config.setImage(Some(&chat_ai(mtm, 22.0)));
            config.setImagePadding(6.0);
            config.setBaseForegroundColor(Some(&orange_dark()));
            config.setContentInsets(NSDirectionalEdgeInsets {
                top: 0.0,
                leading: 12.0,
                bottom: 0.0,
                trailing: 16.0,
            });
            let font = UIFont::systemFontOfSize_weight(15.0, unsafe {
                UIFontWeightMedium
            });
            let value: &AnyObject = font.as_ref();
            let attrs = NSDictionary::from_slices(
                &[unsafe { NSFontAttributeName }],
                &[value],
            );
            let title = unsafe {
                NSAttributedString::initWithString_attributes(
                    NSAttributedString::alloc(),
                    &NSString::from_str("Chat"),
                    Some(&attrs),
                )
            };
            config.setAttributedTitle(Some(&title));
            "chat"
        }
        "fab" => {
            // fab.rs: the same thin orange plus (100 viewBox at 42px, stroke 4).
            config.setImage(Some(&strokes(
                mtm,
                42.0,
                100.0,
                4.0,
                &[(30.0, 50.0, 70.0, 50.0), (50.0, 30.0, 50.0, 70.0)],
                orange(),
            )));
            "fab"
        }
        _ => return None,
    };
    if id != "chat" {
        // A disc: the glyph centred in the square anchor, no default padding.
        config.setContentInsets(NSDirectionalEdgeInsets {
            top: 0.0,
            leading: 0.0,
            bottom: 0.0,
            trailing: 0.0,
        });
    }
    Some((id, config))
}

fn make(web: &UIView, id: &str, mtm: MainThreadMarker) -> Option<Glassed> {
    let (id, config) = configuration(id, mtm)?;
    let button: Retained<UIButton> = if is_plus(id) {
        unsafe {
            msg_send![PlusMenuButton::class(),
                buttonWithConfiguration: &*config,
                primaryAction: Option::<&UIAction>::None]
        }
    } else {
        UIButton::buttonWithConfiguration_primaryAction(&config, None)
    };
    // The web UI is light-only: in system dark mode the glass would turn dark.
    button.setOverrideUserInterfaceStyle(UIUserInterfaceStyle::Light);
    button.setHidden(true);

    // A tap goes through the web anchor's own click handler, which also owns
    // the haptic tick (one tick, whichever button the finger hit).
    let web_for_tap = objc2::Message::retain(web);
    let tap = RcBlock::new(move |_a: NonNull<UIAction>| {
        let js = NSString::from_str(&format!(
            "document.querySelector('[data-glass=\"{id}\"]')?.click();"
        ));
        let done: Option<
            &block2::DynBlock<dyn Fn(*mut AnyObject, *mut AnyObject)>,
        > = None;
        unsafe {
            let _: () = msg_send![
                &*web_for_tap,
                evaluateJavaScript: &*js,
                completionHandler: done
            ];
        }
    });
    let down = RcBlock::new(move |_a: NonNull<UIAction>| {
        crate::infrastructure::platform::haptic_prepare("light");
        if has_menu(id) {
            super::native_menu::prepare(id);
        }
    });
    unsafe {
        let tap = UIAction::actionWithHandler(&*tap as *const _ as *mut _, mtm);
        let down =
            UIAction::actionWithHandler(&*down as *const _ as *mut _, mtm);
        // These two controls use UIButton.menu instead of opening the web
        // popover as well. Before iOS 26 the web button remains the fallback.
        if !has_menu(id) {
            button.addAction_forControlEvents(
                &tap,
                UIControlEvents::TouchUpInside,
            );
        }
        button.addAction_forControlEvents(&down, UIControlEvents::TouchDown);
    }

    // Transcription-done dot, mirrored from the web anchor.
    let badge = UIView::initWithFrame(
        mtm.alloc(),
        CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(8.0, 8.0)),
    );
    badge.setBackgroundColor(Some(&UIColor::systemOrangeColor()));
    badge.setUserInteractionEnabled(false);
    badge.setHidden(true);
    unsafe {
        let layer: *mut AnyObject = msg_send![&*badge, layer];
        if let Some(layer) = layer.as_ref() {
            let _: () = msg_send![layer, setCornerRadius: 4.0f64];
        }
    }
    button.addSubview(&badge);
    web.addSubview(&button);
    Some(Glassed {
        id,
        button,
        badge,
        base: CGRect::default(),
    })
}

/// iOS 26+: the native Liquid Glass button configuration exists.
pub fn supported() -> bool {
    UIButtonConfiguration::class()
        .class_method(sel!(glassButtonConfiguration))
        .is_some()
}

/// Ready the layer; false before iOS 26 or without a web view, in which case
/// the web anchors simply stay visible.
pub fn install() -> bool {
    if GLASS.with(|g| g.borrow().is_some()) {
        return true;
    }
    if !supported() {
        return false;
    }
    let Some(web) = super::web_view() else {
        return false;
    };
    GLASS.with(|g| {
        *g.borrow_mut() = Some(Glass {
            web,
            buttons: Vec::new(),
            travel: 0.0,
            p: 0.0,
            animator: None,
        })
    });
    true
}

/// The DOM menu is the source of truth for labels, availability and actions.
/// `place` creates the button before the matching menu message arrives.
pub fn set_menu(json: &str) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let Ok(menu) = serde_json::from_str::<super::native_menu::Menu>(json)
    else {
        return;
    };
    if !has_menu(&menu.id) {
        return;
    }
    GLASS.with(|cell| {
        let g = cell.borrow();
        if let Some(button) = g
            .as_ref()
            .and_then(|g| g.buttons.iter().find(|b| b.id == menu.id))
        {
            super::native_menu::attach(&button.button, menu, mtm);
        }
    });
}

fn frame(b: &Glassed, p: f64, travel: f64) -> CGRect {
    CGRect::new(
        CGPoint::new(b.base.origin.x + p * travel, b.base.origin.y),
        b.base.size,
    )
}

// Interrupt a running glide where it is (the next move starts from there).
fn stop(g: &mut Glass) {
    if let Some(animator) = g.animator.take() {
        if animator.state() == UIViewAnimatingState::Active {
            animator.stopAnimation(true);
        }
    }
}

/// Resting place of an anchor (card closed), the card travel, and whether
/// its button may show (false under an overlay or off its view).
#[allow(clippy::too_many_arguments)]
pub fn place(
    id: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    travel: f64,
    visible: bool,
    dot: bool,
) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    GLASS.with(|cell| {
        let mut guard = cell.borrow_mut();
        let Some(g) = guard.as_mut() else { return };
        g.travel = travel;
        let idx = match g.buttons.iter().position(|b| b.id == id) {
            Some(idx) => idx,
            None => {
                let Some(made) = make(&g.web, id, mtm) else {
                    return;
                };
                g.buttons.push(made);
                g.buttons.len() - 1
            }
        };
        let running = g.animator.as_ref().is_some_and(|a| a.isRunning());
        let (p, travel) = (g.p, g.travel);
        let b = &mut g.buttons[idx];
        // Never narrower than the native content (a wrapped "Ch/at" title):
        // grow to the left, the anchors that can grow sit on the right edge.
        // Only the pill: the discs keep their square anchor, or the glass
        // insets stretch them into a rounded rectangle.
        let need = b.button.intrinsicContentSize().width;
        let (x, w) = if visible && b.id == "chat" && need > w {
            (x - (need - w), need)
        } else {
            (x, w)
        };
        b.base = CGRect::new(CGPoint::new(x, y), CGSize::new(w, h));
        b.button.setHidden(!visible);
        b.badge.setHidden(!dot);
        b.badge.setFrame(CGRect::new(
            CGPoint::new(w - 16.0, 8.0),
            CGSize::new(8.0, 8.0),
        ));
        if !running {
            b.button.setFrame(frame(b, p, travel));
        }
    });
}

/// Follow the finger: card progress 0 (closed) ..= 1 (open), no animation.
pub fn drag(p: f64) {
    GLASS.with(|g| {
        let mut g = g.borrow_mut();
        let Some(g) = g.as_mut() else { return };
        stop(g);
        g.p = p;
        for b in &g.buttons {
            b.button.setFrame(frame(b, p, g.travel));
        }
    });
}

/// Glide to 0 or 1 with the card's own duration and curve.
pub fn settle(p: f64) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    GLASS.with(|cell| {
        let mut guard = cell.borrow_mut();
        let Some(g) = guard.as_mut() else { return };
        stop(g);
        g.p = p;
        let moves: Vec<(Retained<UIButton>, CGRect)> = g
            .buttons
            .iter()
            .map(|b| (b.button.clone(), frame(b, p, g.travel)))
            .collect();
        let move_all = RcBlock::new(move || {
            for (button, target) in &moves {
                button.setFrame(*target);
            }
        });
        let animator =
            UIViewPropertyAnimator::initWithDuration_controlPoint1_controlPoint2_animations(
                mtm.alloc(),
                SETTLE_S,
                EASE_SOFT.0,
                EASE_SOFT.1,
                Some(&*move_all),
            );
        animator.startAnimation();
        g.animator = Some(animator);
    });
}
