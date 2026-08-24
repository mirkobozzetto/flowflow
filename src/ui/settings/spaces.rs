// Shared spaces panel (proposal 0002). The invitation SCREEN was out of the
// proposal's scope (brief task 3), but without an entry point none of the
// space plane is reachable from the app: this is the smallest surface that
// makes it usable and testable - create, invite, join, add a theme, leave.
//
// It orchestrates nothing itself: every action is one call into
// `application::space`, which owns the rules.

use crate::application::i18n::t;
use crate::application::space::{self, Departure, FolderRight, SpaceError};
use crate::domain::space::Space;
use crate::infrastructure::persistence::Database;
use crate::ui::clipboard::copy_text;
use crate::ui::kit;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn SpacesSettings() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let mut app: AppState = use_context();
    let lang = (app.current_lang)();

    let mut version = use_signal(|| 0u32);
    let mut status: Signal<Option<String>> = use_signal(|| None);
    let mut new_name = use_signal(String::new);
    let mut join_code = use_signal(String::new);

    let spaces = use_memo(move || {
        let _v = version();
        db().list_spaces().unwrap_or_default()
    });

    rsx! {
        div { class: "space-y-6 pb-20",
            h2 { class: "text-lg font-semibold text-stone-900",
                {t(&lang, "settings-section-spaces")}
            }
            if let Some(ref msg) = status() {
                p { class: "text-xs text-stone-600 bg-stone-100 rounded-lg px-3 py-2",
                    "{msg}"
                }
            }

            div {
                p { class: format!("{} mb-1.5", kit::SECTION_LABEL),
                    {t(&lang, "space-create-title")}
                }
                div { class: "flex gap-2",
                    input {
                        class: kit::INPUT,
                        placeholder: t(&lang, "space-name-placeholder"),
                        value: "{new_name}",
                        oninput: move |e| new_name.set(e.value()),
                    }
                    button {
                        class: kit::PILL_PRIMARY,
                        disabled: new_name().trim().is_empty(),
                        onclick: move |_| {
                            let name = new_name().trim().to_string();
                            spawn(async move {
                                let database = db();
                                match space::create_space(&database, &name).await {
                                    Ok(_) => {
                                        new_name.set(String::new());
                                        version.set(version() + 1);
                                        status.set(None);
                                    }
                                    Err(e) => status.set(Some(e.to_string())),
                                }
                            });
                        },
                        {t(&lang, "space-create-button")}
                    }
                }
            }

            div {
                p { class: format!("{} mb-1.5", kit::SECTION_LABEL),
                    {t(&lang, "space-join-title")}
                }
                div { class: "flex gap-2",
                    input {
                        class: kit::INPUT,
                        placeholder: t(&lang, "space-code-placeholder"),
                        value: "{join_code}",
                        oninput: move |e| join_code.set(e.value()),
                    }
                    button {
                        class: kit::PILL_PRIMARY,
                        disabled: join_code().trim().is_empty(),
                        onclick: move |_| {
                            let code = join_code().trim().to_string();
                            spawn(async move {
                                let database = db();
                                match space::join(&database, &code).await {
                                    Ok(_) => {
                                        join_code.set(String::new());
                                        version.set(version() + 1);
                                        status.set(None);
                                        app.folders_version
                                            .set((app.folders_version)() + 1);
                                        app.notes_version
                                            .set((app.notes_version)() + 1);
                                    }
                                    Err(e) => status.set(Some(e.to_string())),
                                }
                            });
                        },
                        {t(&lang, "space-join-button")}
                    }
                }
            }

            if spaces().is_empty() {
                p { class: "text-sm text-stone-400 text-center py-4",
                    {t(&lang, "space-no-spaces")}
                }
            } else {
                div { class: "space-y-4",
                    for sp in spaces() {
                        SpaceCard {
                            key: "{sp.id}",
                            space: sp,
                            version,
                            status,
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SpaceCard(
    space: Space,
    version: Signal<u32>,
    status: Signal<Option<String>>,
) -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let mut app: AppState = use_context();
    let lang = (app.current_lang)();

    let mut invite = use_signal(|| None::<String>);
    let mut leaving = use_signal(|| false);
    let mut theme_name = use_signal(String::new);
    let mut theme_collab = use_signal(|| true);

    let space_id = space.id.clone();
    let (id_invite, id_theme, id_pull, id_keep, id_withdraw) = (
        space_id.clone(),
        space_id.clone(),
        space_id.clone(),
        space_id.clone(),
        space_id.clone(),
    );

    // Themes of this space, with the right the user actually has in each: the
    // same call the sidebar badge makes, so both always say the same thing.
    let themes = use_memo(move || {
        let _v = (app.folders_version)();
        let database = db();
        database
            .space_folder_ids(&space_id)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|local_id| {
                let folder = database.get_folder(&local_id).ok().flatten()?;
                let right = space::folder_right(&database, &local_id);
                Some((folder.name, folder.remote_id?, right))
            })
            .collect::<Vec<_>>()
    });

    let last_pull = space
        .last_pull_at
        .clone()
        .map(|s| t_when(&lang, &s))
        .unwrap_or_else(|| t(&lang, "space-never-pulled"));

    rsx! {
        div { class: "rounded-xl bg-warm-white border border-stone-200 p-4 space-y-3",
            div { class: "flex items-center gap-2",
                span { class: "flex-1 text-sm font-medium text-stone-900",
                    "{space.name}"
                }
                if space.is_owner {
                    span { class: "text-[10px] px-1.5 py-0.5 rounded-md bg-stone-100 text-stone-500",
                        {t(&lang, "space-owner")}
                    }
                }
            }
            p { class: "text-xs text-stone-400", "{last_pull}" }

            div { class: "flex gap-2",
                button {
                    class: kit::PILL_GHOST,
                    onclick: move |_| {
                        let id = id_pull.clone();
                        spawn(async move {
                            let database = db();
                            match space::pull_space(&database, &id).await {
                                Ok(_) => {
                                    version.set(version() + 1);
                                    app.folders_version
                                        .set((app.folders_version)() + 1);
                                    app.notes_version
                                        .set((app.notes_version)() + 1);
                                }
                                Err(e) => status.set(Some(e.to_string())),
                            }
                        });
                    },
                    {t(&lang, "space-refresh")}
                }
                if space.is_owner {
                    button {
                        class: kit::PILL_GHOST,
                        onclick: move |_| {
                            let id = id_invite.clone();
                            spawn(async move {
                                let database = db();
                                match space::invite(&database, &id).await {
                                    // the LINK, not the bare code: what gets
                                    // pasted into a message has to be tappable
                                    Ok(code) => invite.set(Some(
                                        crate::domain::space::space_link(&code),
                                    )),
                                    Err(e) => status.set(Some(e.to_string())),
                                }
                            });
                        },
                        {t(&lang, "space-invite-button")}
                    }
                }
            }

            if let Some(code) = invite() {
                button {
                    class: "w-full text-left px-3 py-2 rounded-lg bg-stone-100 font-mono text-sm text-stone-800 break-all",
                    onclick: {
                        let code = code.clone();
                        let lang = lang.clone();
                        move |_| {
                            copy_text(&code);
                            status.set(Some(t(&lang, "space-invite-copied")));
                        }
                    },
                    "{code}"
                }
            }

            if !themes().is_empty() {
                div { class: "space-y-1",
                    for (name, _remote, right) in themes() {
                        div { class: "flex items-center gap-2 text-sm text-stone-700",
                            span { class: "flex-1 truncate", "{name}" }
                            span { class: "text-[10px] px-1.5 py-0.5 rounded-md bg-stone-100 text-stone-500",
                                {t(&lang, match right {
                                    FolderRight::SpaceReadOnly => "space-badge-readonly",
                                    _ => "space-badge-collab",
                                })}
                            }
                        }
                    }
                }
            }

            div { class: "flex gap-2",
                input {
                    class: kit::INPUT,
                    placeholder: t(&lang, "space-new-theme"),
                    value: "{theme_name}",
                    oninput: move |e| theme_name.set(e.value()),
                }
                button {
                    class: kit::PILL_GHOST,
                    onclick: move |_| theme_collab.set(!theme_collab()),
                    {t(&lang, if theme_collab() {
                        "space-mode-collab"
                    } else {
                        "space-mode-read"
                    })}
                }
                button {
                    class: kit::PILL_PRIMARY,
                    disabled: theme_name().trim().is_empty(),
                    onclick: move |_| {
                        let id = id_theme.clone();
                        let name = theme_name().trim().to_string();
                        let collab = theme_collab();
                        spawn(async move {
                            let database = db();
                            match space::create_folder(
                                &database, &id, None, &name, collab,
                            )
                            .await
                            {
                                Ok(_) => {
                                    theme_name.set(String::new());
                                    version.set(version() + 1);
                                    app.folders_version
                                        .set((app.folders_version)() + 1);
                                }
                                Err(e) => status.set(Some(e.to_string())),
                            }
                        });
                    },
                    {t(&lang, "space-create-button")}
                }
            }

            // The owner IS the space: leaving would strand it ownerless, so the
            // backend refuses it and the button is not offered.
            if !space.is_owner {
                if leaving() {
                    div { class: "flex gap-2",
                        button {
                            class: kit::CONFIRM_BTN_GHOST,
                            onclick: move |_| {
                                let id = id_keep.clone();
                                spawn(async move {
                                    leave(db(), &id, Departure::KeepMine, status)
                                        .await;
                                    leaving.set(false);
                                    version.set(version() + 1);
                                    app.folders_version
                                        .set((app.folders_version)() + 1);
                                    app.notes_version
                                        .set((app.notes_version)() + 1);
                                });
                            },
                            {t(&lang, "space-leave-keep")}
                        }
                        button {
                            class: kit::CONFIRM_BTN_DANGER,
                            onclick: move |_| {
                                let id = id_withdraw.clone();
                                spawn(async move {
                                    leave(
                                        db(),
                                        &id,
                                        Departure::WithdrawMine,
                                        status,
                                    )
                                    .await;
                                    leaving.set(false);
                                    version.set(version() + 1);
                                    app.folders_version
                                        .set((app.folders_version)() + 1);
                                    app.notes_version
                                        .set((app.notes_version)() + 1);
                                });
                            },
                            {t(&lang, "space-leave-withdraw")}
                        }
                    }
                } else {
                    button {
                        class: "w-full text-sm text-ios-red-dark py-2",
                        onclick: move |_| leaving.set(true),
                        {t(&lang, "space-leave-title")}
                    }
                }
            }
        }
    }
}

async fn leave(
    db: Arc<Database>,
    space_id: &str,
    departure: Departure,
    mut status: Signal<Option<String>>,
) {
    if let Err(e) = space::leave(&db, space_id, departure).await {
        // Gone means the membership was already revoked server-side; the local
        // cleanup ran anyway, which is the outcome the user asked for.
        if e != SpaceError::Gone {
            status.set(Some(e.to_string()));
        }
    }
}

// The raw timestamp is enough to tell a fresh pull from a stale one during a
// two-device test; a relative "3 minutes ago" needs a ticking clock this panel
// has no reason to own.
fn t_when(lang: &str, stamp: &str) -> String {
    crate::application::i18n::t_args(
        lang,
        "space-last-pull",
        &[("when", stamp)],
    )
}
