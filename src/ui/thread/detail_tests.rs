use super::*;
use dioxus::core::{ElementId, Mutation};
use dioxus::html::{
    set_event_converter, PlatformEventData, SerializedHtmlEventConverter,
    SerializedMouseData, SerializedPointerData,
};
use std::{cell::RefCell, rc::Rc, time::Duration};

#[derive(Clone)]
struct Fixture {
    notes: Vec<Note>,
    state: Rc<RefCell<Option<AppState>>>,
}

fn host() -> Element {
    let fixture: Fixture = use_context();
    let app = use_context_provider(|| {
        let mut app = AppState::new(Some(true), "en".into());
        app.view.set(View::ThreadDetail {
            thread_id: "thread".into(),
        });
        app
    });
    *fixture.state.borrow_mut() = Some(app);
    rsx! {
        if matches!((app.view)(), View::ThreadDetail { .. }) {
            for note in fixture.notes {
                ThreadNode { key: "{note.id}", thread_id: "thread", note }
            }
        }
    }
}

fn mount() -> (VirtualDom, AppState, ElementId) {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let notes = ["first", "second"].map(|id| {
        serde_json::from_value(serde_json::json!({
            "id": id, "note_type": "Text", "title": id, "content": "A thread note",
            "tags": [], "thread_id": "thread", "created_at": "2026-09-08T12:00:00Z",
            "modified_at": "2026-09-08T12:00:00Z"
        })).unwrap()
    }).to_vec();
    let state = Rc::new(RefCell::new(None));
    let mut dom = VirtualDom::new(host);
    dom.insert_any_root_context(Box::new(Fixture {
        notes,
        state: state.clone(),
    }));
    let edits = dom.rebuild_to_vec();
    let target = edits
        .edits
        .iter()
        .filter_map(|edit| match edit {
            Mutation::NewEventListener { name, id }
                if name == "contextmenu" =>
            {
                Some(*id)
            }
            _ => None,
        })
        .nth(1)
        .expect("second note context-menu listener");
    let app = state.borrow().unwrap();
    (dom, app, target)
}

fn mouse(dom: &VirtualDom, id: ElementId, name: &str) {
    dom.runtime().handle_event(
        name,
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(
                SerializedMouseData::default(),
            ))),
            true,
        ),
        id,
    );
}

fn pointer(dom: &VirtualDom, id: ElementId, name: &str, kind: &str, x: f64) {
    let mut data =
        serde_json::to_value(SerializedMouseData::default()).unwrap();
    data.as_object_mut().unwrap().extend(serde_json::json!({
        "pointer_id": 1, "width": 1.0, "height": 1.0, "pressure": 0.5,
        "tangential_pressure": 0.0, "tilt_x": 0, "tilt_y": 0, "twist": 0,
        "pointer_type": kind, "is_primary": true, "client_x": x, "client_y": 80.0
    }).as_object().unwrap().clone());
    let data: SerializedPointerData = serde_json::from_value(data).unwrap();
    dom.runtime().handle_event(
        name,
        Event::new(Rc::new(PlatformEventData::new(Box::new(data))), true),
        id,
    );
}

async fn pump(dom: &mut VirtualDom, duration: Duration) {
    let _ = tokio::time::timeout(duration, async {
        loop {
            dom.wait_for_work().await;
            dom.render_immediate_to_vec();
        }
    })
    .await;
}

fn selected_note() -> Option<RowMenu> {
    Some(RowMenu::ThreadNote {
        note_id: "second".into(),
        thread_id: "thread".into(),
    })
}

#[test]
fn right_click_targets_the_pressed_note_without_opening_it() {
    let (dom, app, target) = mount();
    mouse(&dom, target, "contextmenu");
    assert_eq!(*app.row_menu.peek(), selected_note());
    assert_eq!(
        *app.view.peek(),
        View::ThreadDetail {
            thread_id: "thread".into()
        }
    );
}

#[tokio::test(flavor = "current_thread")]
async fn long_press_opens_actions_but_release_does_not_open_the_note() {
    for kind in ["mouse", "touch"] {
        let (mut dom, mut app, target) = mount();
        pointer(&dom, target, "pointerdown", kind, 40.0);
        // Small finger jitter remains a hold, not a scroll.
        pointer(&dom, target, "pointermove", kind, 45.0);
        pump(&mut dom, Duration::from_millis(LONG_PRESS_MS + 80)).await;
        assert_eq!(*app.row_menu.peek(), selected_note(), "{kind}");
        pointer(&dom, target, "pointerup", kind, 45.0);
        mouse(&dom, target, "click");
        assert_eq!(*app.row_menu.peek(), selected_note());
        assert_eq!(
            *app.view.peek(),
            View::ThreadDetail {
                thread_id: "thread".into()
            }
        );

        dom.in_runtime(|| app.row_menu.set(None));
        pointer(&dom, target, "pointerdown", kind, 40.0);
        pointer(&dom, target, "pointerup", kind, 40.0);
        mouse(&dom, target, "click");
        assert_eq!(
            *app.view.peek(),
            View::NoteDetail {
                note_id: "second".into()
            }
        );
        assert_eq!(
            *app.previous_view.peek(),
            Some(View::ThreadDetail {
                thread_id: "thread".into()
            })
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn scroll_release_cancel_leave_and_unmount_do_not_open_a_menu() {
    for cancel in [
        "pointermove",
        "pointerup",
        "pointercancel",
        "pointerleave",
        "unmount",
    ] {
        let (mut dom, mut app, target) = mount();
        pointer(&dom, target, "pointerdown", "touch", 40.0);
        if cancel == "unmount" {
            dom.in_runtime(|| {
                app.view.set(View::NoteDetail {
                    note_id: "first".into(),
                })
            });
            dom.render_immediate_to_vec();
        } else {
            pointer(&dom, target, cancel, "touch", 40.0 + PRESS_SLOP + 1.0);
        }
        pump(&mut dom, Duration::from_millis(LONG_PRESS_MS + 80)).await;
        assert_eq!(*app.row_menu.peek(), None, "cancelled by {cancel}");
    }
}
