//! #177: the burger as a native iOS 26 Liquid Glass button laid over the web
//! view. The web burger (`#burger`) stays the source of truth: it keeps its
//! place in the top bar and its click handler, and it shows itself whenever
//! this button cannot (before iOS 26, under a web overlay, on inner views).
//! `src/ui/app/glass_burger.ts` reports where it is and how the card moves.

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{msg_send, sel, ClassType};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{MainThreadMarker, NSString};
use objc2_ui_kit::{
    UIAction, UIButton, UIButtonConfiguration,
    UIButtonConfigurationCornerStyle, UIColor, UIControlEvents,
    UIGraphicsImageRenderer, UIGraphicsImageRendererContext,
    UIUserInterfaceStyle, UIView, UIViewAnimating, UIViewAnimatingState,
    UIViewPropertyAnimator,
};
use std::cell::RefCell;
use std::ptr::NonNull;

struct Glass {
    button: Retained<UIButton>,
    badge: Retained<UIView>,
    base: CGRect,
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

fn stone_800() -> Retained<UIColor> {
    UIColor::colorWithRed_green_blue_alpha(
        41.0 / 255.0,
        37.0 / 255.0,
        36.0 / 255.0,
        1.0,
    )
}

/// The two unequal strokes of the web glyph (28px viewBox drawn at 30px).
fn glyph(mtm: MainThreadMarker) -> Retained<objc2_ui_kit::UIImage> {
    let renderer = UIGraphicsImageRenderer::initWithSize(
        mtm.alloc(),
        CGSize::new(30.0, 30.0),
    );
    let color = stone_800();
    let draw = RcBlock::new(
        move |_ctx: NonNull<UIGraphicsImageRendererContext>| {
            color.setFill();
            let k = 30.0 / 28.0;
            let half = 1.2 * k;
            for (x2, y) in [(19.0, 10.0), (23.0, 18.0)] {
                let rect = CGRect::new(
                    CGPoint::new(5.0 * k - half, y * k - half),
                    CGSize::new((x2 - 5.0) * k + 2.0 * half, 2.0 * half),
                );
                objc2_ui_kit::UIBezierPath::bezierPathWithRoundedRect_cornerRadius(
                    rect, half,
                )
                .fill();
            }
        },
    );
    unsafe { renderer.imageWithActions(&*draw as *const _ as *mut _) }
}

/// Create the button once; false before iOS 26 or without a web view, in
/// which case the web burger simply stays visible.
pub fn install() -> bool {
    if GLASS.with(|g| g.borrow().is_some()) {
        return true;
    }
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };
    if UIButtonConfiguration::class()
        .class_method(sel!(glassButtonConfiguration))
        .is_none()
    {
        return false;
    }
    let Some(web) = super::web_view() else {
        return false;
    };

    let config = UIButtonConfiguration::glassButtonConfiguration(mtm);
    config.setCornerStyle(UIButtonConfigurationCornerStyle::Capsule);
    config.setImage(Some(&glyph(mtm)));
    config.setBaseForegroundColor(Some(&stone_800()));
    let button = UIButton::buttonWithConfiguration_primaryAction(&config, None);
    // The web UI is light-only: in system dark mode the glass would turn dark.
    button.setOverrideUserInterfaceStyle(UIUserInterfaceStyle::Light);
    button.setHidden(true);

    // A tap goes through the web burger's own click handler (open/close).
    let web_for_tap = web.clone();
    let tap = RcBlock::new(move |_a: NonNull<UIAction>| {
        let js = NSString::from_str(
            "var b=document.getElementById('burger');b&&b.click();",
        );
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
    let down = RcBlock::new(|_a: NonNull<UIAction>| {
        crate::infrastructure::platform::haptic_prepare("light");
    });
    unsafe {
        let tap = UIAction::actionWithHandler(&*tap as *const _ as *mut _, mtm);
        let down =
            UIAction::actionWithHandler(&*down as *const _ as *mut _, mtm);
        button.addAction_forControlEvents(&tap, UIControlEvents::TouchUpInside);
        button.addAction_forControlEvents(&down, UIControlEvents::TouchDown);
    }

    // Transcription-done dot, mirrored from the web burger.
    let badge = UIView::initWithFrame(
        mtm.alloc(),
        CGRect::new(CGPoint::new(30.0, 9.0), CGSize::new(8.0, 8.0)),
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

    GLASS.with(|g| {
        *g.borrow_mut() = Some(Glass {
            button,
            badge,
            base: CGRect::default(),
            travel: 0.0,
            p: 0.0,
            animator: None,
        })
    });
    true
}

fn frame(g: &Glass) -> CGRect {
    CGRect::new(
        CGPoint::new(g.base.origin.x + g.p * g.travel, g.base.origin.y),
        g.base.size,
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

/// Resting place of the web burger (card closed), the card travel, and
/// whether the button may show (false under an overlay or on inner views).
pub fn place(x: f64, y: f64, size: f64, travel: f64, visible: bool, dot: bool) {
    GLASS.with(|g| {
        let mut g = g.borrow_mut();
        let Some(g) = g.as_mut() else { return };
        g.base = CGRect::new(CGPoint::new(x, y), CGSize::new(size, size));
        g.travel = travel;
        g.button.setHidden(!visible);
        g.badge.setHidden(!dot);
        g.badge.setFrame(CGRect::new(
            CGPoint::new(size - 16.0, 8.0),
            CGSize::new(8.0, 8.0),
        ));
        if !g.animator.as_ref().is_some_and(|a| a.isRunning()) {
            g.button.setFrame(frame(g));
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
        g.button.setFrame(frame(g));
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
        let button = g.button.clone();
        let target = frame(g);
        let move_it = RcBlock::new(move || button.setFrame(target));
        let animator =
            UIViewPropertyAnimator::initWithDuration_controlPoint1_controlPoint2_animations(
                mtm.alloc(),
                SETTLE_S,
                EASE_SOFT.0,
                EASE_SOFT.1,
                Some(&*move_it),
            );
        animator.startAnimation();
        g.animator = Some(animator);
    });
}
