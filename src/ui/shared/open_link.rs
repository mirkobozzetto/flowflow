// Entry point for a share link (proposal 0001, Q3 app-only): paste a
// flowflow://share/... link, land on the read-only view. Lives in the
// sidebar drawer so the path is always visible.

use crate::application::i18n::t;
use crate::domain::share::parse_share_link;
use crate::domain::space::parse_space_link;
use crate::infrastructure::persistence::Database;
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::sync::Arc;

// Say what actually went wrong. Every one of these sends the user somewhere
// different, and a single generic line sends them nowhere.
fn join_error(lang: &str, e: &crate::application::space::SpaceError) -> String {
    use crate::application::space::SpaceError as E;
    let key = match e {
        E::Gone => "space-join-dead-code",
        E::Offline => "space-join-offline",
        E::Refused => "space-join-no-account",
        E::NoBackend => "space-join-no-backend",
        E::Limit(_) => "space-join-full",
        E::ReadOnly | E::Other(_) => "space-join-failed",
    };
    t(lang, key)
}

#[component]
pub fn OpenShareLink() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();

    let mut open = use_signal(|| false);
    let mut input = use_signal(String::new);
    // The reason the link did not open, in the user's words. A join can fail
    // for half a dozen reasons that have nothing to do with the link being
    // malformed, and telling them "this is not a link" sends them hunting for
    // a problem that is not theirs.
    let mut error: Signal<Option<String>> = use_signal(|| None);

    rsx! {
        div {
            if open() {
                div { class: "bg-warm-white border border-stone-200 rounded-xl p-2.5",
                    input {
                        class: "w-full text-xs font-mono outline-none text-stone-900 placeholder:text-stone-400",
                        placeholder: t(&lang, "share-open-link-placeholder"),
                        value: "{input}",
                        oninput: move |evt| {
                            error.set(None);
                            input.set(evt.value());
                        },
                    }
                    if let Some(ref msg) = error() {
                        p { class: "text-[11px] text-ios-red mt-1", "{msg}" }
                    }
                    div { class: "flex justify-end gap-2 mt-1.5",
                        button {
                            class: "h-7 px-2.5 rounded-lg bg-stone-100 text-stone-900 text-[11px] font-medium",
                            onclick: move |_| {
                                open.set(false);
                                input.set(String::new());
                                error.set(None);
                            },
                            {t(&lang, "share-cancel")}
                        }
                        button {
                            class: "h-7 px-2.5 rounded-lg bg-ios-orange text-white text-[11px] font-medium",
                            onclick: move |_| {
                                let raw = input();
                                // A share link opens a read-only snapshot; a
                                // space link JOINS a live space. Same field,
                                // because the user pastes whatever they were
                                // sent and should not have to know which.
                                if let Some(code) = parse_share_link(&raw) {
                                    let code = code.to_string();
                                    open.set(false);
                                    input.set(String::new());
                                    app.sidebar_open.set(false);
                                    app.previous_view.set(Some(View::NotesList));
                                    app.view.set(View::SharedView { code });
                                } else if let Some(code) = parse_space_link(&raw) {
                                    let code = code.to_string();
                                    let lang = lang.clone();
                                    spawn(async move {
                                        match crate::application::space::join(&db(), &code).await {
                                            Ok(_) => {
                                                open.set(false);
                                                input.set(String::new());
                                                error.set(None);
                                                app.sidebar_open.set(false);
                                                app.folders_version.set((app.folders_version)() + 1);
                                                app.notes_version.set((app.notes_version)() + 1);
                                            }
                                            Err(e) => error.set(Some(join_error(&lang, &e))),
                                        }
                                    });
                                } else {
                                    error.set(Some(t(&lang, "share-open-invalid")));
                                }
                            },
                            {t(&lang, "share-open-link")}
                        }
                    }
                }
            } else {
                button {
                    class: "flex items-center gap-2.5 w-full px-2 py-3 text-sm font-medium text-stone-600 rounded-lg min-h-[44px] hover:bg-stone-100 transition-colors duration-150",
                    onclick: move |_| open.set(true),
                    span { class: "text-ios-orange-dark", "→" }
                    {t(&lang, "share-open-link")}
                }
            }
        }
    }
}
