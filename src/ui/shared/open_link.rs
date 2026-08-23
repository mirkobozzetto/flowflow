// Entry point for a share link (proposal 0001, Q3 app-only): paste a
// flowflow://share/... link, land on the read-only view. Lives in the
// sidebar drawer so the path is always visible.

use crate::application::i18n::t;
use crate::domain::share::parse_share_link;
use crate::ui::{AppState, View};
use dioxus::prelude::*;

#[component]
pub fn OpenShareLink() -> Element {
    let mut app: AppState = use_context();
    let lang = (app.current_lang)();

    let mut open = use_signal(|| false);
    let mut input = use_signal(String::new);
    let mut invalid = use_signal(|| false);

    rsx! {
        div {
            if open() {
                div { class: "bg-warm-white border border-stone-200 rounded-xl p-2.5",
                    input {
                        class: "w-full text-xs font-mono outline-none text-stone-900 placeholder:text-stone-400",
                        placeholder: t(&lang, "share-open-link-placeholder"),
                        value: "{input}",
                        oninput: move |evt| {
                            invalid.set(false);
                            input.set(evt.value());
                        },
                    }
                    if invalid() {
                        p { class: "text-[11px] text-ios-red mt-1",
                            {t(&lang, "share-open-invalid")}
                        }
                    }
                    div { class: "flex justify-end gap-2 mt-1.5",
                        button {
                            class: "h-7 px-2.5 rounded-lg bg-stone-100 text-stone-900 text-[11px] font-medium",
                            onclick: move |_| {
                                open.set(false);
                                input.set(String::new());
                                invalid.set(false);
                            },
                            {t(&lang, "share-cancel")}
                        }
                        button {
                            class: "h-7 px-2.5 rounded-lg bg-ios-orange text-white text-[11px] font-medium",
                            onclick: move |_| {
                                match parse_share_link(&input()) {
                                    Some(code) => {
                                        let code = code.to_string();
                                        open.set(false);
                                        input.set(String::new());
                                        app.sidebar_open.set(false);
                                        app.previous_view.set(Some(View::NotesList));
                                        app.view.set(View::SharedView { code });
                                    }
                                    None => invalid.set(true),
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
