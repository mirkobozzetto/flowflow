// Paste an invite or share link, in the drawer, next to the spaces it leads
// to. The field reads what was pasted as it is typed: the button only lights
// up once the text is a link FlowFlow knows, and says what it will do with it
// (join a space, or open a shared note) before it is pressed.

use crate::application::i18n::t;
use crate::application::space::SpaceError;
use crate::domain::share::parse_share_link;
use crate::domain::space::parse_space_link;
use crate::infrastructure::persistence::Database;
use crate::ui::icons::*;
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq)]
enum Link {
    None,
    Space,
    Share,
}

fn detect(raw: &str) -> Link {
    if parse_share_link(raw).is_some() {
        Link::Share
    } else if parse_space_link(raw).is_some() {
        Link::Space
    } else {
        Link::None
    }
}

// Say what actually went wrong. Every one of these sends the user somewhere
// different, and a single generic line sends them nowhere.
fn join_error(lang: &str, e: &SpaceError) -> String {
    let key = match e {
        SpaceError::Gone => "space-join-dead-code",
        SpaceError::Offline => "space-join-offline",
        SpaceError::Refused => "space-join-no-account",
        SpaceError::NoBackend => "space-join-no-backend",
        SpaceError::Limit(_) => "space-join-full",
        SpaceError::ReadOnly | SpaceError::Other(_) => "space-join-failed",
    };
    t(lang, key)
}

#[component]
pub fn JoinLink() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();

    let mut open = use_signal(|| false);
    let mut input = use_signal(String::new);
    let mut error: Signal<Option<String>> = use_signal(|| None);
    let kind = detect(&input());

    let go = use_callback(move |_: ()| {
        let raw = input();
        match detect(&raw) {
            // a share link opens a read-only snapshot; a space link JOINS a
            // live space. Same field: the user pastes whatever they were sent
            Link::Share => {
                let code =
                    parse_share_link(&raw).unwrap_or_default().to_string();
                open.set(false);
                input.set(String::new());
                app.sidebar_open.set(false);
                app.previous_view.set(Some(View::NotesList));
                app.view.set(View::SharedView { code });
            }
            Link::Space => {
                let code =
                    parse_space_link(&raw).unwrap_or_default().to_string();
                let lang = (app.current_lang)();
                spawn(async move {
                    match crate::application::space::join(&db(), &code).await {
                        Ok(_) => {
                            open.set(false);
                            input.set(String::new());
                            error.set(None);
                            app.folders_version
                                .set((app.folders_version)() + 1);
                            app.notes_version.set((app.notes_version)() + 1);
                        }
                        Err(e) => error.set(Some(join_error(&lang, &e))),
                    }
                });
            }
            Link::None => {
                error.set(Some(t(&(app.current_lang)(), "share-open-invalid")))
            }
        }
    });

    rsx! {
        div { class: "h-px bg-stone-200 my-3" }
        if open() {
            div {
                class: "flex items-center gap-1 bg-stone-100 rounded-xl pl-3 pr-1 py-1 my-1",
                style: "animation: popIn 0.16s ease-out;",
                span { class: "shrink-0 text-stone-400", IconLink { size: 16 } }
                input {
                    class: "flex-1 min-w-0 bg-transparent text-sm outline-none py-1.5 text-stone-900 placeholder-stone-400",
                    placeholder: t(&lang, "share-join-placeholder"),
                    value: "{input}",
                    autofocus: true,
                    oninput: move |evt| {
                        error.set(None);
                        input.set(evt.value());
                    },
                    onkeydown: move |evt| {
                        if evt.key() == Key::Escape {
                            open.set(false);
                            input.set(String::new());
                            error.set(None);
                        }
                    },
                    onkeypress: move |evt| {
                        if evt.key() == Key::Enter {
                            go(());
                        }
                    },
                }
                button {
                    class: "h-9 px-3 flex items-center justify-center rounded-lg text-xs font-medium transition-colors duration-150",
                    class: if kind == Link::None {
                        "text-stone-300"
                    } else {
                        "text-white bg-ios-orange active:opacity-70"
                    },
                    disabled: kind == Link::None,
                    onclick: move |_| go(()),
                    {t(&lang, match kind {
                        Link::Share => "share-open-link",
                        _ => "space-join-button",
                    })}
                }
            }
            if let Some(msg) = error() {
                p { class: "text-[11px] text-ios-red-dark px-2 pb-1", "{msg}" }
            }
        } else {
            button {
                class: "flex items-center gap-2.5 w-full px-2 py-3 text-sm font-medium text-stone-600 rounded-lg min-h-[44px] hover:bg-stone-100 transition-colors duration-150",
                onclick: move |_| open.set(true),
                span { class: "text-ios-orange-dark", IconLink { size: 16 } }
                {t(&lang, "share-join-link")}
            }
        }
    }
}
