use crate::application::hermes_message::Attachment;
use crate::application::i18n::t;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::platform;
use crate::ui::AppState;
use dioxus::prelude::*;

/// The names of the skills already attached to the next question.
pub(crate) fn taken_skills(app: AppState) -> Vec<String> {
    app.hermes_attachments
        .read()
        .iter()
        .filter_map(|a| match a {
            Attachment::Skill { name } => Some(name.clone()),
            _ => None,
        })
        .collect()
}

fn close(mut app: AppState) {
    app.show_tools_menu.set(false);
    app.hermes_attach_error.set(None);
}

/// A new Hermes conversation with the note attached, the field ready.
pub(crate) fn send_note(mut app: AppState, db: &Database, note_id: &str) {
    let Some(note) = db.get_note(note_id).ok().flatten() else {
        return;
    };
    let lang = (app.current_lang)();
    let title = note
        .title
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| t(&lang, "note-card-untitled"));
    app.show_note_menu.set(false);
    app.show_note_tools_menu.set(false);
    app.sidebar_tab.set(crate::ui::SidebarTab::Chats);
    app.hermes_attach_error.set(None);
    app.hermes_attachments.set(vec![Attachment::Note {
        title,
        text: note.content,
    }]);
    app.previous_view.set(Some(crate::ui::View::NoteDetail {
        note_id: note_id.to_string(),
    }));
    app.view
        .set(crate::ui::View::HermesChat { session_id: None });
    document::eval(
        "requestAnimationFrame(() => document.querySelector('.composer-field')?.focus());",
    );
}

/// Adds what is not attached yet; the same name is never attached twice.
pub(crate) fn attach(mut app: AppState, items: Vec<Attachment>) {
    let mut list = app.hermes_attachments.write();
    for item in items {
        if !list.iter().any(|a| a.label() == item.label()) {
            list.push(item);
        }
    }
}

// The pickers outlive the menu that opened them.
pub(crate) fn pick_photos(app: AppState) {
    close(app);
    dioxus::core::spawn_forever(async move {
        let photos = platform::pick_photos().await;
        attach(
            app,
            photos
                .into_iter()
                .map(|(name, jpeg)| Attachment::Photo { name, jpeg })
                .collect(),
        );
    });
}

pub(crate) fn take_photo(app: AppState) {
    close(app);
    dioxus::core::spawn_forever(async move {
        if let Some((name, jpeg)) = platform::take_photo().await {
            attach(app, vec![Attachment::Photo { name, jpeg }]);
        }
    });
}

/// A file's text goes with the question; one whose text cannot be read
/// says so and stays out.
pub(crate) fn pick_file(mut app: AppState) {
    close(app);
    let lang = (app.current_lang)();
    dioxus::core::spawn_forever(async move {
        use crate::ui::notes::menu::import_file_content;
        match import_file_content(&lang).await {
            Ok(None) => {}
            Ok(Some(f)) if !f.content.trim().is_empty() => attach(
                app,
                vec![Attachment::File {
                    name: f.filename,
                    text: f.content,
                }],
            ),
            Ok(Some(_)) | Err(_) => app
                .hermes_attach_error
                .set(Some(t(&lang, "hermes-file-unreadable"))),
        }
    });
}

/// A picked skill comes off again when picked a second time.
pub(crate) fn toggle_skill(mut app: AppState, name: &str) {
    close(app);
    let attached = app
        .hermes_attachments
        .peek()
        .iter()
        .any(|a| matches!(a, Attachment::Skill { name: n } if n == name));
    if attached {
        app.hermes_attachments.write().retain(
            |a| !matches!(a, Attachment::Skill { name: n } if n == name),
        );
    } else {
        attach(
            app,
            vec![Attachment::Skill {
                name: name.to_string(),
            }],
        );
    }
}
