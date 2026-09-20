use crate::ui::chat::tools_menu::ToolsMenu;
use dioxus::prelude::*;

#[component]
pub fn NoteToolsMenu() -> Element {
    rsx! { ToolsMenu { note: true } }
}
