use crate::domain::ChatScope;
use crate::infrastructure::audio::RecordingState;
use crate::infrastructure::persistence::Database;
use crate::ui::chat::actions::{
    load_messages_from_db, send_question, update_proposal_status,
};
use crate::ui::chat::approval_card::ApprovalCard;
use crate::ui::chat::bot_bubble::BotBubble;
use crate::ui::chat::chat_input::ChatInputBar;
use crate::ui::chat::empty_state::ChatEmptyState;
use crate::ui::chat::mention_menu::MentionedNote;
use crate::ui::chat::menu::ChatMenu;
use crate::ui::chat::models::{ChatMsg, ProposalStatus};
use crate::ui::chat::typing_indicator::TypingIndicator;
use crate::ui::chat::user_bubble::UserBubble;
use crate::ui::state::View;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn ChatView() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();

    let initial_conv_id = match (app.view)() {
        View::Chat { conversation_id } => conversation_id,
        _ => None,
    };

    let mut conversation_id: Signal<Option<String>> =
        use_signal(|| initial_conv_id.clone());

    let scope_init = use_signal(|| {
        if let Some(cid) = initial_conv_id.as_deref() {
            let restored = db
                .peek()
                .chat_scope(cid)
                .filter(|s| scope_alive(&db.peek(), s));
            app.chat_scope.set(restored);
            app.chat_web.set(db.peek().chat_web(cid));
        } else {
            // Fresh mount with no conversation (new chat from a non-chat view):
            // start OFF so a prior chat's web toggle never leaks in.
            app.chat_web.set(false);
        }
        true
    });
    let _ = scope_init();

    let initial_msgs = if let Some(ref cid) = initial_conv_id {
        load_messages_from_db(&db(), cid)
    } else {
        vec![]
    };

    let mut messages: Signal<Vec<ChatMsg>> = use_signal(|| initial_msgs);
    let mut input = use_signal(String::new);
    let mut mentions: Signal<Vec<MentionedNote>> = use_signal(Vec::new);
    let mut loading = use_signal(|| false);
    let mut tool_status: Signal<Option<String>> = use_signal(|| None);
    let renaming = use_signal(|| false);
    let rename_input = use_signal(String::new);
    let confirm_delete = use_signal(|| false);

    use_effect(move || {
        if let RecordingState::Transcribed { transcript, .. } =
            (app.recording_state)()
        {
            // Dictation has no clip to seek into, so only the text is used.
            let text = transcript.text();
            let current = input();
            if current.is_empty() {
                input.set(text);
            } else {
                input.set(format!("{} {}", current, text));
            }
            app.recording_state.set(RecordingState::Idle);
            dioxus::document::eval(
                r#"
                requestAnimationFrame(() => {
                    var ta = document.querySelector('.chat-textarea');
                    if (ta) { ta.style.height = 'auto'; ta.style.height = ta.scrollHeight + 'px'; }
                });
                "#,
            );
        }
    });

    use_effect(move || {
        let _len = messages().len();
        dioxus::document::eval(
            r#"
            let el = document.getElementById('chat-messages');
            if (el) el.scrollTop = el.scrollHeight;
        "#,
        );
    });

    use_effect(move || {
        let _v = (app.sync_data_version)();
        if loading() {
            return;
        }
        let Some(cid) = conversation_id() else {
            return;
        };
        let fresh = load_messages_from_db(&db(), &cid);
        if fresh != *messages.peek() {
            messages.set(fresh);
        }
    });

    use_effect(move || {
        let view_cid = match (app.view)() {
            View::Chat {
                conversation_id: cid,
            } => cid,
            _ => return,
        };
        if (view_cid == *conversation_id.peek()
            && app.pending_chat_scope.peek().is_none())
            || *loading.peek()
        {
            return;
        }
        let msgs = match &view_cid {
            Some(cid) => load_messages_from_db(&db(), cid),
            None => vec![],
        };
        let restored = match &view_cid {
            Some(cid) => db
                .peek()
                .chat_scope(cid)
                .filter(|s| scope_alive(&db.peek(), s)),
            None => {
                // A new chat inherits a pending scope (e.g. opened from a
                // folder), which is then consumed.
                let pending = app.pending_chat_scope.peek().clone();
                app.pending_chat_scope.set(None);
                pending.filter(|s| scope_alive(&db.peek(), s))
            }
        };
        let web = match &view_cid {
            Some(cid) => db.peek().chat_web(cid),
            None => false,
        };
        conversation_id.set(view_cid);
        messages.set(msgs);
        input.set(String::new());
        tool_status.set(None);
        mentions.set(Vec::new());
        app.chat_scope.set(restored);
        app.chat_web.set(web);
    });

    use_effect(move || {
        let scope = (app.chat_scope)();
        if let Some(cid) = conversation_id.peek().clone() {
            let _ = db.peek().set_chat_scope(&cid, scope.as_ref());
        }
    });

    use_effect(move || {
        let web = (app.chat_web)();
        if let Some(cid) = conversation_id.peek().clone() {
            let _ = db.peek().set_chat_web(&cid, web);
        }
    });

    let is_empty = messages().is_empty();
    let show_menu = (app.show_chat_menu)();

    rsx! {
        if show_menu {
            ChatMenu {
                conversation_id: conversation_id,
                renaming: renaming,
                rename_input: rename_input,
                confirm_delete: confirm_delete,
            }
        }
        div {
            class: "overflow-hidden relative",
            style: "height: calc(100% - var(--keyboard-inset, 0px));",
            div { id: "chat-messages", class: "h-full overflow-y-auto px-4 pt-4 safe-pb-40 lg:px-[max(1rem,calc((100%-48rem)/2))]",
                if is_empty && !loading() {
                    ChatEmptyState {}
                } else {
                    div { class: "space-y-3",
                        {messages().iter().enumerate().map(|(i, msg)| {
                            match msg {
                                ChatMsg::User(text) => rsx! {
                                    div { key: "{i}",
                                        UserBubble { text: text.clone() }
                                    }
                                },
                                ChatMsg::Bot { text, sources, trace } => rsx! {
                                    div { key: "{i}",
                                        BotBubble {
                                            text: text.clone(),
                                            sources: sources.clone(),
                                            conversation_id: conversation_id(),
                                            trace: trace.clone(),
                                        }
                                    }
                                },
                                ChatMsg::Proposal { view, status, edit_error } => {
                                    let id = view.id.clone();
                                    rsx! {
                                        div { key: "{i}",
                                            ApprovalCard {
                                                view: view.clone(),
                                                status: status.clone(),
                                                edit_error: edit_error.clone(),
                                                on_expired: move |_| update_proposal_status(
                                                    &mut messages,
                                                    &id,
                                                    ProposalStatus::Expired,
                                                ),
                                                lang: (app.current_lang)(),
                                            }
                                        }
                                    }
                                },
                            }
                        })}
                        if loading() {
                            TypingIndicator { tool_status: tool_status() }
                        }
                    }
                }
            }
        }
        ChatInputBar {
            input: input,
            mentions: mentions,
            disabled: loading(),
            on_send: move |q: String| {
                if conversation_id().is_none() {
                    let title: String = q.chars().take(50).collect();
                    if let Ok(conv) = db().create_conversation(&title) {
                        let cid = conv.id.clone();
                        let _ = db().set_chat_scope(
                            &cid,
                            (app.chat_scope)().as_ref(),
                        );
                        let _ = db().set_chat_web(&cid, (app.chat_web)());
                        conversation_id.set(Some(cid.clone()));
                        app.view.set(View::Chat {
                            conversation_id: Some(cid),
                        });
                        // Refresh the sidebar Chats list: it slides via CSS (stays mounted),
                        // so a new conversation only shows if a watched signal changes.
                        app.invalidate_data();
                    }
                }
                let scope = (app.chat_scope)();
                let web = (app.chat_web)();
                let mention_ids: Vec<String> =
                    mentions().iter().map(|m| m.note_id.clone()).collect();
                let lang = (app.current_lang)();
                send_question(q, &mut messages, &mut loading, &mut tool_status, conversation_id, db, scope, web, mention_ids, lang);
                mentions.set(Vec::new());
            },
        }
    }
}

fn scope_alive(db: &Database, scope: &ChatScope) -> bool {
    match scope {
        ChatScope::Folder(id) => db.get_folder(id).ok().flatten().is_some(),
        ChatScope::Thread(id) => db.get_thread(id).ok().flatten().is_some(),
    }
}
