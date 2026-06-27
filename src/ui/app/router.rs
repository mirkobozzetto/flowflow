use super::animations::{slide_style, Slide};
use crate::ui::attachment_modal::AttachmentModal;
use crate::ui::chat::ChatView;
use crate::ui::fab::FloatingActionButton;
use crate::ui::folder_picker;
use crate::ui::note_list::NotesList;
use crate::ui::notes::NoteDetail;
use crate::ui::settings::{SettingsSectionView, SettingsView};
use crate::ui::sidebar::SidebarOverlay;
use crate::ui::sync::SyncView;
use crate::ui::thread::ThreadDetail;
use crate::ui::top_bar::TopBar;
use crate::ui::{AppState, View};
use dioxus::prelude::*;

#[component]
pub fn AppRouter(index_rebuilding: Signal<bool>) -> Element {
    let app = use_context::<AppState>();
    rsx! {
        div { class: "h-screen w-full overflow-hidden font-sans bg-stone-100 lg:flex lg:flex-row",
            SidebarOverlay {}
            AttachmentModal {}
            div { class: "flex flex-col h-screen safe-pt lg:flex-1 lg:min-w-0",
                TopBar {}
                if index_rebuilding() {
                    div { class: "bg-ios-orange/10 border-b border-ios-orange/20 px-4 py-1.5",
                        p { class: "text-xs text-ios-orange text-center",
                            {crate::application::i18n::t(&(app.current_lang)(), "restore-banner-rebuilding")}
                        }
                    }
                }
                div { class: "flex-1 overflow-hidden relative",
                    {
                        let is_bg = !matches!((app.view)(), View::NotesList);
                        let is_note = matches!((app.view)(), View::NoteDetail { .. } | View::ThreadDetail { .. });
                        let sliding_back = (app.sliding_out)();
                        let shifted = is_bg && !sliding_back;
                        let shift_dir = if is_note { "30%" } else { "-30%" };
                        let instant = cfg!(target_os = "macos");
                        rsx! {
                            div {
                                id: "notes-scroll",
                                class: "absolute inset-0 overflow-y-auto px-4 py-3 safe-pb-20",
                                class: if is_bg { "pointer-events-none" } else { "" },
                                style: if instant && shifted {
                                    format!("transform: translateX({shift_dir}); opacity: 0.5;")
                                } else if instant {
                                    "transform: translateX(0); opacity: 1;".to_string()
                                } else if shifted {
                                    format!("transform: translateX({shift_dir}); opacity: 0.5; transition: transform 0.15s ease, opacity 0.15s ease;")
                                } else {
                                    "transform: translateX(0); opacity: 1; transition: transform 0.15s ease, opacity 0.15s ease;".to_string()
                                },
                                div { class: "w-full lg:max-w-3xl lg:mx-auto",
                                    NotesList {}
                                }
                            }
                        }
                    }
                    if matches!((app.view)(), View::NoteDetail { .. }) {
                        div {
                            class: "absolute inset-0 flex flex-col min-h-0 bg-stone-100",
                            style: slide_style(Slide::Left, (app.sliding_out)()),
                            div { class: "w-full flex-1 flex flex-col min-h-0",
                                NoteDetail {}
                            }
                        }
                    }
                    if matches!((app.view)(), View::ThreadDetail { .. }) {
                        div {
                            class: "absolute inset-0 flex flex-col min-h-0 bg-stone-100",
                            style: slide_style(Slide::Left, (app.sliding_out)()),
                            div { class: "w-full flex-1 flex flex-col min-h-0",
                                ThreadDetail {}
                            }
                        }
                    }
                    if matches!((app.view)(), View::Chat { .. }) {
                        div {
                            class: "absolute inset-0 flex flex-col min-h-0 bg-stone-100",
                            style: slide_style(Slide::Right, (app.sliding_out)()),
                            div { class: "w-full flex-1 flex flex-col min-h-0",
                                ChatView {}
                            }
                        }
                    }
                    if matches!(
                        (app.view)(),
                        View::Settings | View::SettingsSection(_)
                    ) || (matches!((app.view)(), View::SyncPairing)
                        && (app.previous_view)() == Some(View::Settings))
                    {
                        div {
                            class: "absolute inset-0 flex flex-col min-h-0 px-4 safe-py-3 bg-stone-100 overflow-y-auto",
                            class: if !matches!((app.view)(), View::Settings)
                                && !(app.sliding_out)()
                            {
                                "pointer-events-none"
                            } else {
                                ""
                            },
                            style: {
                                let in_section =
                                    !matches!((app.view)(), View::Settings);
                                let sliding_back = (app.sliding_out)();
                                if cfg!(target_os = "macos") {
                                    // Desktop: opaque, no depth dimming. The dim+shift
                                    // left a ghost settings panel on transitions.
                                    String::new()
                                } else if sliding_back && !in_section {
                                    "animation: slideOutRight 0.15s ease-in forwards;".to_string()
                                } else if sliding_back && in_section {
                                    // Going back from a section: un-shift the list to 0 in
                                    // parallel with the section sliding out, instead of waiting
                                    // for the view flip (which animated them in sequence). Keep
                                    // the slideInRight token so it never replays here.
                                    "animation: slideInRight 0.15s ease-out; transform: translateX(0); opacity: 1; transition: transform 0.15s ease, opacity 0.15s ease;".to_string()
                                } else if in_section {
                                    "animation: slideInRight 0.15s ease-out; transform: translateX(-30%); opacity: 0.5; transition: transform 0.15s ease, opacity 0.15s ease;".to_string()
                                } else {
                                    "animation: slideInRight 0.15s ease-out; transform: translateX(0); opacity: 1; transition: transform 0.15s ease, opacity 0.15s ease;".to_string()
                                }
                            },
                            div { class: "w-full lg:max-w-2xl lg:mx-auto",
                                SettingsView {}
                            }
                        }
                    }
                    if matches!((app.view)(), View::SettingsSection(_)) {
                        div {
                            class: "absolute inset-0 flex flex-col min-h-0 px-4 safe-py-3 bg-stone-100 overflow-y-auto",
                            style: slide_style(Slide::Right, (app.sliding_out)()),
                            div { class: "w-full lg:max-w-2xl lg:mx-auto",
                                SettingsSectionView {}
                            }
                        }
                    }
                    if matches!((app.view)(), View::SyncPairing) {
                        div {
                            class: "absolute inset-0 flex flex-col min-h-0 px-4 safe-py-3 bg-stone-100 overflow-y-auto",
                            style: slide_style(Slide::Right, (app.sliding_out)()),
                            div { class: "w-full lg:max-w-2xl lg:mx-auto",
                                SyncView {}
                            }
                        }
                    }
                    if (app.show_folder_picker)() {
                        div {
                            class: "fixed inset-0 z-10",
                            onclick: move |_| {
                                let mut app = app;
                                app.show_folder_picker.set(false);
                            },
                        }
                        {
                            match (app.view)() {
                                View::NotesList => rsx! {
                                    folder_picker::FolderPicker { selected: app.selected_folder_id, on_pick: move |_| {} }
                                },
                                View::NoteDetail { .. } => rsx! {
                                    folder_picker::FolderPicker { selected: app.detail_folder_id, on_pick: move |_| {} }
                                },
                                View::Chat { .. } => rsx! {
                                    folder_picker::ChatScopePicker {}
                                },
                                _ => rsx! {},
                            }
                        }
                    }
                }
                if (app.view)() == View::NotesList {
                    FloatingActionButton {}
                }
            }
        }
    }
}
