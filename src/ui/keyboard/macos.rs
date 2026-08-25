use crate::ui::{AppState, SidebarTab, View};
use dioxus::prelude::*;

pub fn use_macos_shortcuts(app: AppState) {
    use_future(move || {
        let mut app = app;
        async move {
            let mut eval = dioxus::document::eval(
                r#"
                var lastEsc = 0;
                var lastCtrl = 0;
                var lastMeta = 0;
                function inField() {
                    var el = document.activeElement;
                    return el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA');
                }
                document.addEventListener('keydown', function(e) {
                    if (e.ctrlKey && e.key !== 'Control') { lastCtrl = 0; }
                    if (e.metaKey && e.key !== 'Meta') { lastMeta = 0; }
                    if (e.key === 'Meta' && !e.ctrlKey && !e.altKey && !e.shiftKey) {
                        var nowM = Date.now();
                        if (nowM - lastMeta < 400) {
                            lastMeta = 0;
                            dioxus.send('tab-toggle');
                            return;
                        }
                        lastMeta = nowM;
                        return;
                    }
                    if (e.metaKey && e.shiftKey && e.key === 'Enter') { e.preventDefault(); dioxus.send('new-chat'); }
                    else if (e.ctrlKey && e.shiftKey && e.key === 'Enter') { e.preventDefault(); dioxus.send('new-chat'); }
                    else if (e.metaKey && e.key === 'n') { e.preventDefault(); dioxus.send('new-note'); }
                    else if (e.metaKey && e.key === 'f') { e.preventDefault(); dioxus.send('search'); }
                    else if (e.metaKey && e.key === ',') { e.preventDefault(); dioxus.send('settings'); }
                    else if (e.metaKey && e.key === '1') { e.preventDefault(); dioxus.send('tab-notes'); }
                    else if (e.metaKey && e.key === '2') { e.preventDefault(); dioxus.send('tab-chats'); }
                    else if (e.ctrlKey && e.key === 'ArrowLeft') { e.preventDefault(); dioxus.send('tab-notes'); }
                    else if (e.ctrlKey && e.key === 'ArrowRight') { e.preventDefault(); dioxus.send('tab-chats'); }
                    else if (e.metaKey && e.key === 'ArrowLeft' && !inField()) { e.preventDefault(); dioxus.send('nav-back'); }
                    else if (e.metaKey && e.key === 'ArrowRight' && !inField()) { e.preventDefault(); dioxus.send('nav-forward'); }
                    else if (e.key === 'Control' && !e.metaKey && !e.altKey && !e.shiftKey) {
                        var nowC = Date.now();
                        if (nowC - lastCtrl < 400) { lastCtrl = 0; dioxus.send('picker-toggle'); }
                        else { lastCtrl = nowC; }
                    }
                    else if (e.key === 'ArrowDown' && !inField() && !e.metaKey && !e.ctrlKey) { e.preventDefault(); dioxus.send('kb-down'); }
                    else if (e.key === 'ArrowUp' && !inField() && !e.metaKey && !e.ctrlKey) { e.preventDefault(); dioxus.send('kb-up'); }
                    else if (e.key === 'Enter' && !inField() && !e.metaKey && !e.ctrlKey && !e.shiftKey) { dioxus.send('kb-enter'); }
                    else if (e.key === 'Escape') {
                        var now = Date.now();
                        if (now - lastEsc < 400) {
                            lastEsc = 0;
                            dioxus.send('escape-home');
                        } else {
                            lastEsc = now;
                            dioxus.send('escape');
                        }
                    }
                });
                "#,
            );
            while let Ok(msg) = eval.recv::<String>().await {
                match msg.as_str() {
                    "new-note" => {
                        app.show_folder_picker.set(false);
                        if matches!((app.view)(), View::NoteDetail { .. }) {
                            app.view.set(View::NotesList);
                            spawn(async move {
                                futures_timer::Delay::new(
                                    std::time::Duration::from_millis(30),
                                )
                                .await;
                                app.view.set(View::NoteDetail {
                                    note_id: String::new(),
                                });
                            });
                        } else {
                            app.view.set(View::NoteDetail {
                                note_id: String::new(),
                            });
                        }
                    }
                    "search" => {
                        app.show_folder_picker.set(false);
                        app.view.set(View::NotesList);
                        dioxus::document::eval(
                            r#"
                            requestAnimationFrame(function() {
                                var el = document.getElementById('note-search');
                                if (el) el.focus();
                            });
                            "#,
                        );
                    }
                    "settings" => {
                        app.show_folder_picker.set(false);
                        if !matches!(
                            (app.view)(),
                            View::Settings | View::SettingsSection(_)
                        ) {
                            app.view.set(View::Settings);
                        }
                    }
                    "new-chat" => {
                        app.show_folder_picker.set(false);
                        app.sidebar_tab.set(SidebarTab::Chats);
                        app.chat_scope.set(None);
                        app.previous_view.set(Some(View::NotesList));
                        app.view.set(View::Chat {
                            conversation_id: None,
                        });
                    }
                    "tab-toggle" => {
                        let next = if (app.sidebar_tab)() == SidebarTab::Notes {
                            SidebarTab::Chats
                        } else {
                            SidebarTab::Notes
                        };
                        app.sidebar_tab.set(next);
                        app.sidebar_open.set(true);
                    }
                    "tab-notes" => {
                        app.sidebar_tab.set(SidebarTab::Notes);
                        app.sidebar_open.set(true);
                    }
                    "tab-chats" => {
                        app.sidebar_tab.set(SidebarTab::Chats);
                        app.sidebar_open.set(true);
                    }
                    "nav-back" => crate::ui::app::nav::nav_back(app),
                    "nav-forward" => {
                        let target = app.view_future.write().pop();
                        if let Some(target) = target {
                            app.view_history
                                .write()
                                .push(app.view.peek().clone());
                            app.history_nav.set(true);
                            app.sliding_out.set(false);
                            app.previous_view.set(None);
                            app.show_folder_picker.set(false);
                            app.show_note_menu.set(false);
                            app.show_chat_menu.set(false);
                            app.show_thread_menu.set(false);
                            app.view.set(target);
                        }
                    }
                    "picker-toggle" => {
                        if matches!(
                            (app.view)(),
                            View::NotesList
                                | View::NoteDetail { .. }
                                | View::Chat { .. }
                        ) {
                            let cur = (app.show_folder_picker)();
                            app.show_folder_picker.set(!cur);
                            if !cur {
                                dioxus::document::eval(
                                    "if (document.activeElement) document.activeElement.blur();",
                                );
                            }
                        }
                    }
                    "kb-down" => {
                        if (app.show_folder_picker)() {
                            let next = *app.picker_kb_down.peek() + 1;
                            app.picker_kb_down.set(next);
                        }
                    }
                    "kb-up" => {
                        if (app.show_folder_picker)() {
                            let next = *app.picker_kb_up.peek() + 1;
                            app.picker_kb_up.set(next);
                        }
                    }
                    "kb-enter" => {
                        if (app.show_folder_picker)() {
                            let next = *app.picker_kb_commit.peek() + 1;
                            app.picker_kb_commit.set(next);
                        }
                    }
                    "escape" => {
                        app.row_menu.set(None);
                        app.show_folder_picker.set(false);
                        app.show_note_menu.set(false);
                        app.show_chat_menu.set(false);
                        app.sidebar_open.set(false);
                        app.attachment_modal.set(None);
                    }
                    "escape-home" => {
                        app.show_folder_picker.set(false);
                        app.show_note_menu.set(false);
                        app.show_chat_menu.set(false);
                        app.sidebar_open.set(false);
                        app.attachment_modal.set(None);
                        app.previous_view.set(None);
                        app.view.set(View::NotesList);
                    }
                    _ => {}
                }
            }
        }
    });
}
