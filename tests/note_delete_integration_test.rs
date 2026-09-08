//! Diagnostic coverage: real menu components, Dioxus events and isolated SQLite.
#![allow(dead_code)]

pub use flowflow::{application, domain, infrastructure};
#[path = "../src/ui/clipboard.rs"]
pub mod clipboard;
#[path = "../src/ui/delete_confirm.rs"]
pub mod delete_confirm;
#[path = "../src/ui/kit.rs"]
pub mod kit;
mod ui {
    pub use super::{clipboard, delete_confirm, kit};
    pub use flowflow::ui::{icons, AppState, NoteMenuPage, RowMenu, View};
}
#[path = "../src/ui/notes/menu.rs"]
mod note_menu;
#[path = "../src/ui/notes/detail/hooks/persistence.rs"]
mod note_persistence_hooks;
#[path = "../src/ui/notes/row_menu.rs"]
mod row_menu;

use dioxus::core::{ElementId, Mutation, Mutations};
use dioxus::prelude::*;
use flowflow::domain::{NewFolder, NewTextNote, NewThread};
use flowflow::infrastructure::persistence::Database;
use flowflow::infrastructure::sync::engine::SyncEngine;
use std::{any::Any, rc::Rc, sync::Arc};

fn make_note(db: &Database) -> String {
    db.create_text_note(&NewTextNote {
        title: Some("Deletion diagnostic".into()),
        content: "Synthetic test data".into(),
        tags: vec![],
    })
    .unwrap()
    .id
}

#[test]
fn sqlite_delete_cleans_folder_and_thread_membership() {
    let temp = tempfile::tempdir().unwrap();
    let db = Database::open_at(temp.path().join("test.db")).unwrap();
    let parent = db
        .create_folder(&NewFolder {
            name: "Theme".into(),
            description: None,
            parent_id: None,
        })
        .unwrap();
    let child = db
        .create_folder(&NewFolder {
            name: "Subtheme".into(),
            description: None,
            parent_id: Some(parent.id.clone()),
        })
        .unwrap();
    for (label, folder, detach) in [
        ("all notes", None, false),
        ("theme + thread", Some(parent.id.as_str()), false),
        ("subtheme + thread", Some(child.id.as_str()), false),
        ("theme after detaching", Some(parent.id.as_str()), true),
        ("subtheme after detaching", Some(child.id.as_str()), true),
    ] {
        let target = make_note(&db);
        let sibling = make_note(&db);
        let thread = db
            .create_thread(&NewThread {
                title: label.into(),
                folder_id: folder.map(str::to_string),
            })
            .unwrap();
        db.add_note_to_thread(&target, &thread.id).unwrap();
        db.add_note_to_thread(&sibling, &thread.id).unwrap();
        if let Some(folder) = folder {
            db.add_note_to_folder(&target, folder).unwrap();
            assert!(db
                .list_notes_in_folder(folder)
                .unwrap()
                .iter()
                .any(|n| n.id == target));
        }
        if detach {
            db.remove_note_from_thread(&target).unwrap();
        }
        db.with_tx(|tx| {
            application::note_persistence::delete_note_rows(tx, &target)
        })
        .unwrap();
        assert!(db.get_note(&target).unwrap().is_none(), "{label}");
        assert!(db.folders_for_note(&target).unwrap().is_empty(), "{label}");
        let members = db.list_thread_notes(&thread.id).unwrap();
        assert_eq!(
            members.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(),
            vec![sibling.as_str()],
            "{label}"
        );
        assert!(db.list_notes().unwrap().iter().all(|n| n.id != target));
        println!(
            "SQL {label}: deleted, folder links cleared, sibling preserved"
        );
    }
}

#[derive(Clone)]
struct Fixture {
    db: Arc<Database>,
    engine: Arc<SyncEngine>,
    note_id: String,
    folder_id: Option<String>,
    detail: bool,
    keep_mounted: bool,
    state: Rc<std::cell::RefCell<Option<ui::AppState>>>,
    marked_deleted: Rc<std::cell::Cell<bool>>,
    editor: Rc<std::cell::RefCell<Option<Signal<String>>>>,
}

fn menu_host() -> Element {
    let fixture: Fixture = use_context();
    let db = fixture.db.clone();
    let engine = fixture.engine.clone();
    use_context_provider(|| Signal::new(db));
    use_context_provider(|| Signal::new(engine));
    let app = use_context_provider(|| {
        let mut app = ui::AppState::new(Some(true), "en".into());
        app.selected_folder_id.set(fixture.folder_id.clone());
        app.detail_folder_id.set(fixture.folder_id.clone());
        app.current_note_id.set(Some(fixture.note_id.clone()));
        app.view.set(ui::View::NoteDetail {
            note_id: fixture.note_id.clone(),
        });
        app.show_note_menu.set(true);
        app.row_menu.set(Some(ui::RowMenu::Note {
            note_id: fixture.note_id.clone(),
            page: ui::NoteMenuPage::Actions,
        }));
        app
    });
    *fixture.state.borrow_mut() = Some(app);
    rsx! {
        row_menu::NoteDeleteStatus {}
        if fixture.detail {
            if (app.view)() == (ui::View::NoteDetail { note_id: fixture.note_id.clone() }) {
                detail_host {}
            }
        } else {
            row_menu::NoteRowMenu {}
        }
    }
}

fn detail_host() -> Element {
    let fixture: Fixture = use_context();
    let app: ui::AppState = use_context();
    let import_requested = use_signal(|| false);
    let deleted = use_signal(|| false);
    let observed = fixture.marked_deleted.clone();
    use_effect(move || observed.set(deleted()));
    let db: Signal<Arc<Database>> = use_context();
    let engine: Signal<Arc<SyncEngine>> = use_context();
    let note = use_hook(|| db().get_note(&fixture.note_id).unwrap().unwrap());
    let title = use_signal(|| note.title.clone().unwrap_or_default());
    let content = use_signal(|| note.content.clone());
    let tags = use_signal(|| note.tags.clone());
    let base_title = use_signal(|| title());
    let base_content = use_signal(|| content());
    let base_tags = use_signal(|| tags());
    let note_id = use_signal(|| fixture.note_id.clone());
    let audio = use_signal(|| None);
    *fixture.editor.borrow_mut() = Some(content);
    note_persistence_hooks::use_delete_exit(app, note_id, deleted);
    note_persistence_hooks::use_save_on_drop(
        app,
        db,
        engine,
        title,
        content,
        tags,
        note_id,
        deleted,
        audio,
        base_title,
        base_content,
        base_tags,
        fixture.folder_id.clone(),
    );
    rsx! {
        if (app.show_note_menu)() || fixture.keep_mounted {
            note_menu::NoteMenu { note_id: fixture.note_id, import_requested, deleted }
        }
    }
}

fn click_listeners(mutations: &Mutations) -> Vec<ElementId> {
    mutations
        .edits
        .iter()
        .filter_map(|m| match m {
            Mutation::NewEventListener { name, id } if name == "click" => {
                Some(*id)
            }
            _ => None,
        })
        .collect()
}

fn click(dom: &VirtualDom, id: ElementId) {
    let event = Event::new(
        Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default()))
            as Rc<dyn Any>,
        true,
    );
    dom.runtime().handle_event("click", event, id);
}

fn confirmation(dom: &mut VirtualDom) -> Vec<ElementId> {
    let initial = dom.rebuild_to_vec();
    click(dom, *click_listeners(&initial).last().unwrap());
    click_listeners(&dom.render_immediate_to_vec())
}

async fn pump(dom: &mut VirtualDom) {
    dom.render_immediate_to_vec();
    let _ = tokio::time::timeout(
        std::time::Duration::from_millis(10),
        dom.wait_for_work(),
    )
    .await;
}

fn fixture(db: &Arc<Database>, engine: &Arc<SyncEngine>, id: &str) -> Fixture {
    Fixture {
        db: db.clone(),
        engine: engine.clone(),
        note_id: id.into(),
        folder_id: None,
        detail: true,
        keep_mounted: false,
        state: Rc::new(std::cell::RefCell::new(None)),
        marked_deleted: Rc::new(std::cell::Cell::new(false)),
        editor: Rc::new(std::cell::RefCell::new(None)),
    }
}

fn mount(fixture: Fixture) -> VirtualDom {
    let mut dom = VirtualDom::new(menu_host);
    dom.insert_any_root_context(Box::new(fixture));
    dom
}

// One test owns global environment overrides; no other test in this binary
// opens the global database, audio or vector stores.
#[tokio::test(flavor = "current_thread")]
async fn menu_confirmation_deletes_from_every_entry_point() {
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("FLOWFLOW_DATA_DIR", temp.path());
    std::env::set_var("FLOWFLOW_VECTORDB_PATH", temp.path().join("vectors"));
    std::env::set_var("FLOWFLOW_SYNC_ADVERTISE_ADDR", "127.0.0.1");
    // Reserve our own beacon socket instead of relying on the running app's port.
    let beacon = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    std::env::set_var(
        "FLOWFLOW_SYNC_PORT",
        (beacon.local_addr().unwrap().port() - 1).to_string(),
    );
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let db = Arc::new(Database::open_at(temp.path().join("menu.db")).unwrap());
    db.set_setting("backend_base_url", "").unwrap();
    let engine = SyncEngine::start_listener(db.clone(), 0);
    let parent = db
        .create_folder(&NewFolder {
            name: "Theme".into(),
            description: None,
            parent_id: None,
        })
        .unwrap();
    let child = db
        .create_folder(&NewFolder {
            name: "Subtheme".into(),
            description: None,
            parent_id: Some(parent.id.clone()),
        })
        .unwrap();
    let mut failures = Vec::new();
    for (label, detail, folder_id, thread_mode, keep_mounted, cancel) in [
        ("list/all", false, None, 0, false, false),
        (
            "list/theme",
            false,
            Some(parent.id.clone()),
            0,
            false,
            false,
        ),
        ("detail/all", true, None, 0, false, false),
        (
            "detail/theme",
            true,
            Some(parent.id.clone()),
            0,
            false,
            false,
        ),
        (
            "detail/subtheme/thread",
            true,
            Some(child.id.clone()),
            1,
            false,
            false,
        ),
        (
            "detail/subtheme/detached",
            true,
            Some(child.id.clone()),
            2,
            false,
            false,
        ),
        (
            "detail/mounted-control",
            true,
            Some(parent.id.clone()),
            0,
            true,
            false,
        ),
        (
            "detail/cancel",
            true,
            Some(child.id.clone()),
            1,
            false,
            true,
        ),
        (
            "list/cancel",
            false,
            Some(parent.id.clone()),
            0,
            false,
            true,
        ),
    ] {
        let id = make_note(&db);
        if let Some(ref fid) = folder_id {
            db.add_note_to_folder(&id, fid).unwrap();
        }
        let sibling = make_note(&db);
        let thread = db
            .create_thread(&NewThread {
                title: label.into(),
                folder_id: folder_id.clone(),
            })
            .unwrap();
        db.add_note_to_thread(&sibling, &thread.id).unwrap();
        if thread_mode != 0 {
            db.add_note_to_thread(&id, &thread.id).unwrap();
        }
        if thread_mode == 2 {
            db.remove_note_from_thread(&id).unwrap();
        }
        let mut setup = fixture(&db, &engine, &id);
        setup.detail = detail;
        setup.folder_id = folder_id.clone();
        setup.keep_mounted = keep_mounted;
        let mut dom = mount(setup);
        let buttons = confirmation(&mut dom);
        // The confirmation renders Cancel followed by Delete.
        click(&dom, buttons[buttons.len() - if cancel { 2 } else { 1 }]);
        for _ in 0..3 {
            pump(&mut dom).await;
        }
        let exists = db.get_note(&id).unwrap().is_some();
        println!("UI {label}: exists={exists}, expected={cancel}");
        if exists != cancel {
            failures.push(label.to_string());
        }
        assert!(db.get_note(&sibling).unwrap().is_some());
        assert!(db
            .list_thread_notes(&thread.id)
            .unwrap()
            .iter()
            .any(|n| n.id == sibling));
        if !exists {
            assert!(db.folders_for_note(&id).unwrap().is_empty());
        }
    }
    delayed_revoke_navigation(&db, &engine, &mut failures, false).await;
    delayed_revoke_navigation(&db, &engine, &mut failures, true).await;
    ui_failure_and_retry(&db, &engine).await;
    exit_animation_preserves_new_view(&db, &engine).await;
    sql_abort_and_retry(&db, &temp, &mut failures).await;
    menu_reopens_cleanly(&db, &engine, &mut failures);
    assert!(
        failures.is_empty(),
        "Deletion contract violations: {failures:?}"
    );
}

fn menu_reopens_cleanly(
    db: &Arc<Database>,
    engine: &Arc<SyncEngine>,
    failures: &mut Vec<String>,
) {
    let a = make_note(db);
    let b = make_note(db);
    let folder = db
        .create_folder(&NewFolder {
            name: "Retained folder".into(),
            description: None,
            parent_id: None,
        })
        .unwrap();
    let thread = db
        .create_thread(&NewThread {
            title: "Unchanged thread".into(),
            folder_id: Some(folder.id.clone()),
        })
        .unwrap();
    for id in [&a, &b] {
        db.add_note_to_folder(id, &folder.id).unwrap();
        db.add_note_to_thread(id, &thread.id).unwrap();
    }
    for (page, button_index) in [("move", 0), ("delete", 3)] {
        for target in [&a, &b] {
            let mut setup = fixture(db, engine, &a);
            setup.detail = false;
            let observed = setup.state.clone();
            let mut dom = mount(setup);
            let initial = dom.rebuild_to_vec();
            click(&dom, click_listeners(&initial)[button_index]);
            dom.render_immediate_to_vec();
            let mut app = observed.borrow().unwrap();
            dom.in_runtime(|| app.row_menu.set(None));
            dom.render_immediate_to_vec();
            dom.in_runtime(|| {
                app.row_menu.set(Some(ui::RowMenu::Note {
                    note_id: target.clone(),
                    page: ui::NoteMenuPage::Actions,
                }))
            });
            let reopened = dom.render_immediate_to_vec();
            let copy = application::i18n::t("en", "note-menu-copy");
            let actions_visible = reopened.edits.iter().any(|edit| matches!(edit,
                Mutation::CreateTextNode { value, .. } | Mutation::SetText { value, .. } if *value == copy
            ));
            println!("MENU RESET: abandoned={page}, same_note={}, actions_visible={actions_visible}", target == &a);
            if !actions_visible {
                failures.push(format!(
                    "stale {page} page on reopen, same_note={}",
                    target == &a
                ));
            }
        }
    }
    for id in [&a, &b] {
        assert_eq!(
            db.get_note(id).unwrap().unwrap().thread_id.as_deref(),
            Some(thread.id.as_str())
        );
        assert_eq!(
            db.folders_for_note(id)
                .unwrap()
                .iter()
                .map(|f| f.id.as_str())
                .collect::<Vec<_>>(),
            vec![folder.id.as_str()]
        );
    }
}

async fn delayed_revoke_navigation(
    db: &Arc<Database>,
    engine: &Arc<SyncEngine>,
    failures: &mut Vec<String>,
    revoke_fails: bool,
) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    db.set_setting("backend_base_url", &url).unwrap();
    db.set_setting("backend_session_token", "synthetic-test-token")
        .unwrap();
    db.set_setting("backend_session_expires_at", "2099-01-01T00:00:00Z")
        .unwrap();
    let id = make_note(db);
    let other = make_note(db);
    db.upsert_share(
        &id,
        &domain::share::ShareKind::Note,
        "synthetic-code",
        "2099-01-01T00:00:00Z",
    )
    .unwrap();
    let (arrived_tx, mut arrived_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut buffer = [0; 1024];
        loop {
            let n = stream.read(&mut buffer).await.unwrap();
            assert!(n > 0, "request ended before HTTP headers");
            request.extend_from_slice(&buffer[..n]);
            if request.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        assert!(String::from_utf8_lossy(&request)
            .starts_with("POST /v1/shares/revoke "));
        arrived_tx.send(()).unwrap();
        release_rx.await.unwrap();
        // The client can stop awaiting after cancellation; the server still exits.
        let response = if revoke_fails {
            b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".as_slice()
        } else {
            b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".as_slice()
        };
        let _ = stream.write_all(response).await;
        tokio::time::timeout(
            std::time::Duration::from_millis(100),
            listener.accept(),
        )
        .await
        .is_ok()
    });
    let setup = fixture(db, engine, &id);
    let state = setup.state.clone();
    let marked_deleted = setup.marked_deleted.clone();
    let mut dom = mount(setup);
    let buttons = confirmation(&mut dom);
    click(&dom, *buttons.last().unwrap());
    // A second event can arrive before the menu removal is rendered.
    click(&dom, *buttons.last().unwrap());
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(2);
    let mut arrived = false;
    while std::time::Instant::now() < deadline {
        pump(&mut dom).await;
        if arrived_rx.try_recv().is_ok() {
            arrived = true;
            break;
        }
    }
    if !arrived {
        println!("DELAYED revoke: request never reached loopback server");
        failures.push("delayed revoke never started".into());
        server.abort();
        let _ = server.await;
        db.set_setting("backend_base_url", "").unwrap();
        return;
    }
    let premature = marked_deleted.get();
    println!(
        "DELAYED pending: SQL note exists={}, marked_deleted={premature}",
        db.get_note(&id).unwrap().is_some()
    );
    if premature {
        failures.push("marked deleted before SQL commit".into());
    }
    let mut app = state.borrow().unwrap();
    dom.in_runtime(|| {
        app.current_note_id.set(Some(other.clone()));
        app.view.set(ui::View::NoteDetail {
            note_id: other.clone(),
        });
    });
    dom.render_immediate_to_vec();
    release_tx.send(()).unwrap();
    assert!(
        !server.await.unwrap(),
        "duplicate confirmation sent another revoke"
    );
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(2);
    while db.get_note(&id).unwrap().is_some()
        && std::time::Instant::now() < deadline
    {
        pump(&mut dom).await;
    }
    let deleted = db.get_note(&id).unwrap().is_none();
    let kept_selection = *app.current_note_id.peek() == Some(other.clone());
    println!("DELAYED after navigation: deleted={deleted}, other_selection_preserved={kept_selection}");
    if !deleted {
        failures.push("deletion canceled by navigation".into());
    }
    if !kept_selection {
        failures.push("completion cleared another note selection".into());
    }
    assert!(db.get_note(&other).unwrap().is_some());
    assert_eq!(app.note_delete_error.peek().is_some(), revoke_fails);
    assert!(app.note_delete_pending.peek().is_none());
    println!("REVOKE failure={revoke_fails}: local deletion completed, warning correct, duplicate blocked");
    db.set_setting("backend_base_url", "").unwrap();
}

async fn exit_animation_preserves_new_view(
    db: &Arc<Database>,
    engine: &Arc<SyncEngine>,
) {
    let id = make_note(db);
    let other = make_note(db);
    let setup = fixture(db, engine, &id);
    let state = setup.state.clone();
    let mut dom = mount(setup);
    let buttons = confirmation(&mut dom);
    click(&dom, *buttons.last().unwrap());
    let mut app = state.borrow().unwrap();
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(2);
    while !*app.sliding_out.peek() && std::time::Instant::now() < deadline {
        pump(&mut dom).await;
    }
    assert!(
        *app.sliding_out.peek(),
        "committed deletion starts the exit animation"
    );
    assert!(db.get_note(&id).unwrap().is_none());
    dom.in_runtime(|| {
        app.view.set(ui::View::NoteDetail {
            note_id: other.clone(),
        });
        app.current_note_id.set(Some(other.clone()));
    });
    dom.render_immediate_to_vec();
    while *app.sliding_out.peek() && std::time::Instant::now() < deadline {
        pump(&mut dom).await;
    }
    assert!(!*app.sliding_out.peek());
    assert_eq!(
        *app.view.peek(),
        ui::View::NoteDetail {
            note_id: other.clone()
        }
    );
    assert_eq!(*app.current_note_id.peek(), Some(other));
    println!("EXIT RACE: newer view preserved, sliding_out cleared");
}

fn renders_delete_error(mutations: &Mutations) -> bool {
    let message = application::i18n::t("en", "note-delete-failed");
    mutations.edits.iter().any(|edit| match edit {
        Mutation::CreateTextNode { value, .. }
        | Mutation::SetText { value, .. } => value == &message,
        _ => false,
    })
}

async fn ui_failure_and_retry(db: &Arc<Database>, engine: &Arc<SyncEngine>) {
    for detail in [true, false] {
        let id = make_note(db);
        let mut setup = fixture(db, engine, &id);
        setup.detail = detail;
        let state = setup.state.clone();
        let editor = setup.editor.clone();
        let marked_deleted = setup.marked_deleted.clone();
        let mut dom = mount(setup);
        let buttons = confirmation(&mut dom);
        if detail {
            let mut content = editor.borrow().unwrap();
            dom.in_runtime(|| {
                content.set("Unsaved edit must survive failure".into())
            });
        }
        db.conn().execute_batch("CREATE TEMP TRIGGER ui_delete_failure BEFORE DELETE ON notes BEGIN SELECT RAISE(ABORT, 'ui deletion rejected'); END;").unwrap();
        click(&dom, *buttons.last().unwrap());
        let mut app = state.borrow().unwrap();
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_secs(2);
        let mut alert_rendered = false;
        while std::time::Instant::now() < deadline {
            let changes = dom.render_immediate_to_vec();
            alert_rendered |= renders_delete_error(&changes);
            if app.note_delete_error.peek().is_some() && alert_rendered {
                break;
            }
            let _ = tokio::time::timeout(
                std::time::Duration::from_millis(10),
                dom.wait_for_work(),
            )
            .await;
        }
        assert!(
            alert_rendered,
            "deletion failure must show an error to the user"
        );
        assert!(app.note_delete_error.peek().is_some());
        assert!(app.note_delete_pending.peek().is_none());
        assert!(!marked_deleted.get());
        assert!(db.get_note(&id).unwrap().is_some());
        assert!(!db
            .pending_purges()
            .unwrap()
            .contains(&(id.clone(), "note".into())));
        if detail {
            assert_eq!(
                &*editor.borrow().unwrap().peek(),
                "Unsaved edit must survive failure"
            );
            assert_eq!(
                *app.view.peek(),
                ui::View::NoteDetail {
                    note_id: id.clone()
                }
            );
        }
        db.conn()
            .execute_batch("DROP TRIGGER ui_delete_failure")
            .unwrap();
        dom.in_runtime(|| {
            app.note_delete_error.set(None);
            if detail {
                app.show_note_menu.set(true);
            } else {
                app.row_menu.set(Some(ui::RowMenu::Note {
                    note_id: id.clone(),
                    page: ui::NoteMenuPage::Actions,
                }));
            }
        });
        let opened = dom.render_immediate_to_vec();
        click(&dom, *click_listeners(&opened).last().unwrap());
        let confirmed = dom.render_immediate_to_vec();
        click(&dom, *click_listeners(&confirmed).last().unwrap());
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            pump(&mut dom).await;
            if db.get_note(&id).unwrap().is_none()
                && (!detail || matches!(*app.view.peek(), ui::View::NotesList))
            {
                break;
            }
        }
        assert!(db.get_note(&id).unwrap().is_none());
        assert!(app.note_delete_error.peek().is_none());
        assert!(!*app.sliding_out.peek());
        if detail {
            assert!(matches!(*app.view.peek(), ui::View::NotesList));
        }
        println!("UI ERROR/RETRY detail={detail}: alert rendered, content retained, retry deleted, exit completed");
    }
}

async fn sql_abort_and_retry(
    db: &Database,
    temp: &tempfile::TempDir,
    failures: &mut Vec<String>,
) {
    use flowflow::infrastructure::vectordb::{Chunk, VectorStore};
    let id = make_note(db);
    let filename = "delete-diagnostic.wav";
    let audio = db.add_audio(&id, filename, 1.0).unwrap();
    let path = std::path::PathBuf::from(
        infrastructure::audio::resolve_audio_path(filename),
    );
    assert!(path.starts_with(temp.path()), "audio must be isolated");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"synthetic-audio").unwrap();
    let vector = VectorStore::open().await.unwrap();
    let chunk = || Chunk {
        id: format!("{id}-chunk"),
        note_id: id.clone(),
        chunk_text: "synthetic vector".into(),
        chunk_index: 0,
        vector: vec![0.1; application::constants::EMBEDDING_DIMS],
        title: "diagnostic".into(),
        tags: "[]".into(),
        created_at: "2026-09-08T00:00:00Z".into(),
    };
    vector.store_chunks(vec![chunk()]).await.unwrap();
    db.conn().execute_batch(
        "CREATE TEMP TRIGGER reject_note_delete BEFORE DELETE ON notes BEGIN SELECT RAISE(ABORT, 'diagnostic rejection'); END;"
    ).unwrap();
    let failure = db
        .with_tx(|tx| application::note_persistence::delete_note_rows(tx, &id))
        .unwrap_err();
    assert!(failure.contains("diagnostic rejection"));
    let failure =
        application::note_persistence::delete_note(db, &id).unwrap_err();
    assert!(failure.contains("diagnostic rejection"));
    let incorrectly_queued = db
        .pending_purges()
        .unwrap()
        .contains(&(id.clone(), "note".into()));
    assert!(db.get_note(&id).unwrap().is_some());
    assert!(db
        .list_audios(&id)
        .unwrap()
        .iter()
        .any(|a| a.id == audio.id));
    assert!(path.exists());
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(2);
    while !vector.fetch_note_rows(&id).await.unwrap().is_empty()
        && std::time::Instant::now() < deadline
    {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let vectors_preserved =
        !vector.fetch_note_rows(&id).await.unwrap().is_empty();
    println!("SQL ABORT: note_preserved=true, audio_preserved=true, purge_queued={incorrectly_queued}, vectors_preserved={vectors_preserved}");
    if incorrectly_queued || !vectors_preserved {
        failures.push("SQL rollback still purged live note vectors".into());
    }
    // Restore the synthetic vector so retry proves its cleanup and we can
    // wait for the detached deletion before releasing the temporary directory.
    vector.store_chunks(vec![chunk()]).await.unwrap();
    db.conn()
        .execute_batch("DROP TRIGGER reject_note_delete")
        .unwrap();
    application::note_persistence::delete_note(db, &id).unwrap();
    assert!(db.get_note(&id).unwrap().is_none());
    assert!(!path.exists());
    assert!(db.list_audios(&id).unwrap().is_empty());
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(2);
    while !vector.fetch_note_rows(&id).await.unwrap().is_empty()
        && std::time::Instant::now() < deadline
    {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(vector.fetch_note_rows(&id).await.unwrap().is_empty());
    println!(
        "SQL RETRY: note_deleted=true, audio_deleted=true, vector_deleted=true"
    );
}
