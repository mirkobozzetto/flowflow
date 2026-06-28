use crate::domain::is_auto_title;
use crate::infrastructure::persistence::Database;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

pub fn use_auto_title(
    app: AppState,
    db: Signal<Arc<Database>>,
    content: Signal<String>,
    mut title: Signal<String>,
    mut generating_title: Signal<bool>,
    mut title_gen_done: Signal<bool>,
) {
    use_effect(move || {
        let c = content();
        let t = title();
        if title_gen_done() || generating_title() {
            return;
        }
        if c.len() <= 50 || !is_auto_title(&t) {
            return;
        }
        generating_title.set(true);
        let preview: String = c.chars().take(1500).collect();
        spawn(async move {
            let lang = (app.current_lang)();
            if let Ok(ai) =
                crate::infrastructure::llm::LlmClient::from_db(&db())
            {
                if let Ok(new_title) =
                    crate::application::titling::generate_title(
                        &ai, &preview, &lang,
                    )
                    .await
                {
                    title.set(new_title);
                }
            }
            generating_title.set(false);
            title_gen_done.set(true);
        });
    });
}
