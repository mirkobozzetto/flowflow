use crate::application::approvals::{
    self, ProposalView, UserDecision, APPROVAL_TIMEOUT,
};
use crate::application::i18n::t;
use crate::ui::chat::models::ProposalStatus;
use crate::ui::icons::IconShieldCheck;
use dioxus::prelude::*;
use uuid::Uuid;

// The approval card for a suspended connector write: payload rows, the TOCTOU caveat, and
// Approuver / Modifier / Refuser. Modifier opens an inline editor over the value field;
// every action delegates to `approvals::decide`/`touch` - the card never touches the registry.
// Once decided the buttons vanish and only a status pill remains. A decide that returns Expired
// (the run already died) has no hook left to emit the resolution, so the card asks its owner
// to freeze it: `on_expired`. That handler is the card's only tie to where it is mounted, which
// is why the same component serves the chat and a note action.
#[component]
pub fn ApprovalCard(
    view: ProposalView,
    status: ProposalStatus,
    edit_error: Option<String>,
    on_expired: EventHandler<()>,
    lang: String,
) -> Element {
    let mut editing = use_signal(|| false);
    let value_row = view
        .rows
        .iter()
        .find(|(k, _)| k == "value")
        .map(|(_, v)| v.clone());
    let has_value = value_row.is_some();
    let mut edit_value = use_signal(|| value_row.clone().unwrap_or_default());

    let pending = status == ProposalStatus::Pending;
    let card_id = Uuid::parse_str(&view.id).ok();

    let approve = move |_| {
        if let Some(uuid) = card_id {
            if approvals::decide(uuid, UserDecision::Approved).is_err() {
                on_expired.call(());
            }
        }
    };

    let open_edit = move |_| {
        if let Some(uuid) = card_id {
            approvals::touch(uuid, APPROVAL_TIMEOUT);
        }
        editing.set(true);
    };

    let reject = move |_| {
        if let Some(uuid) = card_id {
            if approvals::decide(uuid, UserDecision::Rejected).is_err() {
                on_expired.call(());
            }
        }
    };

    let submit_edit = {
        let raw = view.raw_args.clone();
        move |_| {
            let mut args = raw.clone();
            if let Some(obj) = args.as_object_mut() {
                obj.insert(
                    "value".to_string(),
                    serde_json::Value::String(edit_value()),
                );
            }
            if let Some(uuid) = card_id {
                if approvals::decide(uuid, UserDecision::Edited(args)).is_err()
                {
                    on_expired.call(());
                }
            }
            editing.set(false);
        }
    };

    let editing_now = editing();
    let display_rows: Vec<(String, String)> = if editing_now {
        view.rows
            .iter()
            .filter(|(k, _)| k != "value")
            .cloned()
            .collect()
    } else {
        view.rows.clone()
    };

    rsx! {
        div { class: "mt-2 p-3 bg-warm-white border border-stone-200 rounded-xl",
            div { class: "flex items-center gap-2 text-ios-orange-dark",
                IconShieldCheck { size: 18 }
                span { class: "text-sm font-semibold text-stone-800",
                    {t(&lang, "approval-required")}
                }
            }
            div { class: "mt-0.5 text-xs text-stone-500", "{view.action} · {view.tool}" }

            div { class: "mt-2 space-y-1",
                {display_rows.iter().enumerate().map(|(i, (k, v))| {
                    let label = row_label(&lang, k);
                    rsx! {
                        div { key: "{i}", class: "flex gap-2 text-sm",
                            span { class: "shrink-0 text-stone-400", "{label}" }
                            span { class: "text-stone-700 break-words", "{v}" }
                        }
                    }
                })}
            }

            div { class: "mt-2 text-xs italic text-stone-400",
                {t(&lang, "approval-caveat")}
            }

            if let Some(err) = edit_error.as_ref() {
                div { class: "mt-2 text-xs text-ios-red", "{err}" }
            }

            if pending && editing_now {
                div { class: "mt-2",
                    label { class: "block mb-1 text-xs text-stone-400",
                        {row_label(&lang, "value")}
                    }
                    input {
                        class: "w-full min-h-[44px] px-3 py-2 rounded-lg border border-stone-200 bg-white text-sm text-stone-800",
                        value: "{edit_value}",
                        oninput: move |e| edit_value.set(e.value()),
                    }
                    div { class: "mt-2 flex gap-2",
                        button {
                            class: "pressable flex-1 min-h-[44px] rounded-xl bg-ios-orange text-white font-medium text-sm hover:bg-ios-orange-dark active:scale-[0.98]",
                            onclick: submit_edit,
                            {t(&lang, "approval-confirm")}
                        }
                        button {
                            class: "pressable flex-1 min-h-[44px] rounded-xl bg-stone-100 text-stone-600 font-medium text-sm hover:bg-stone-200 active:bg-stone-300",
                            onclick: move |_| editing.set(false),
                            {t(&lang, "approval-cancel")}
                        }
                    }
                }
            } else if pending {
                div { class: "mt-3 flex gap-2",
                    button {
                        class: "pressable flex-1 min-h-[44px] rounded-xl bg-ios-orange text-white font-medium text-sm hover:bg-ios-orange-dark active:scale-[0.98]",
                        onclick: approve,
                        {t(&lang, "approval-approve")}
                    }
                    if has_value {
                        button {
                            class: "pressable flex-1 min-h-[44px] rounded-xl bg-stone-100 text-stone-700 font-medium text-sm hover:bg-stone-200 active:bg-stone-300",
                            onclick: open_edit,
                            {t(&lang, "approval-edit")}
                        }
                    }
                    button {
                        class: "pressable flex-1 min-h-[44px] rounded-xl bg-ios-red/10 text-ios-red font-medium text-sm hover:bg-ios-red/15 active:scale-[0.98]",
                        onclick: reject,
                        {t(&lang, "approval-reject")}
                    }
                }
            } else {
                div { class: "mt-3",
                    span {
                        class: "inline-flex items-center px-2.5 py-1 rounded-full text-xs font-medium {pill_class(&status)}",
                        {t(&lang, pill_key(&status))}
                    }
                }
            }
        }
    }
}

// Translate the arg keys the formatter leads with; anything else keeps its raw name.
fn row_label(lang: &str, key: &str) -> String {
    let k = match key {
        "spreadsheet_id" => "approval-row-spreadsheet",
        "cell" => "approval-row-cell",
        "value" => "approval-row-value",
        _ => return key.to_string(),
    };
    t(lang, k)
}

fn pill_key(status: &ProposalStatus) -> &'static str {
    match status {
        ProposalStatus::Approved => "approval-status-approved",
        ProposalStatus::Edited => "approval-status-edited",
        ProposalStatus::Rejected => "approval-status-rejected",
        // Pending never reaches the pill branch; treat it as expired defensively.
        ProposalStatus::Pending | ProposalStatus::Expired => {
            "approval-status-expired"
        }
    }
}

fn pill_class(status: &ProposalStatus) -> &'static str {
    match status {
        ProposalStatus::Approved | ProposalStatus::Edited => {
            "bg-ios-orange-50 text-ios-orange-dark"
        }
        ProposalStatus::Rejected => "bg-ios-red/10 text-ios-red",
        ProposalStatus::Pending | ProposalStatus::Expired => {
            "bg-stone-100 text-stone-500"
        }
    }
}
