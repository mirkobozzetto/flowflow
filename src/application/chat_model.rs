//! The model and reasoning level the notes chat answers with. A pick is kept
//! per conversation, and the latest pick starts every new one, as Hermes
//! does (hermes_chat::choose_model). Only current-generation models.
use crate::application::chat_source;
use crate::application::constants::{
    ANTHROPIC_HAIKU_MODEL, ANTHROPIC_OPUS_MODEL, ANTHROPIC_SONNET_MODEL,
};
use crate::infrastructure::persistence::Database;
use crate::infrastructure::Provider;
use std::str::FromStr;

const LATEST: &str = "last";
const LEVELS: &[&str] = &["low", "medium", "high", "xhigh", "max"];
const LEVELS_WITH_NONE: &[&str] =
    &["none", "low", "medium", "high", "xhigh", "max"];

pub struct ChatModel {
    pub id: &'static str,
    pub levels: &'static [&'static str],
    pub default_level: &'static str,
}

// The first entry is each source's default.
const CLAUDE: &[ChatModel] = &[
    ChatModel {
        id: ANTHROPIC_HAIKU_MODEL,
        levels: LEVELS,
        default_level: "medium",
    },
    ChatModel {
        id: ANTHROPIC_SONNET_MODEL,
        levels: LEVELS,
        default_level: "high",
    },
    ChatModel {
        id: ANTHROPIC_OPUS_MODEL,
        levels: LEVELS,
        default_level: "medium",
    },
];
const OPENAI: &[ChatModel] = &[
    ChatModel {
        id: "gpt-6-luna",
        levels: LEVELS_WITH_NONE,
        default_level: "medium",
    },
    ChatModel {
        id: "gpt-6.1-sol",
        levels: LEVELS,
        default_level: "medium",
    },
];
// What the ChatGPT subscription serves, as Hermes lists it.
const CHATGPT: &[ChatModel] = &[ChatModel {
    id: "gpt-6.1-sol",
    levels: LEVELS,
    default_level: "medium",
}];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Family {
    Claude,
    OpenAi,
}

pub fn family(source: Provider) -> Family {
    match source {
        Provider::OpenAi | Provider::ChatGpt => Family::OpenAi,
        _ => Family::Claude,
    }
}

pub fn models(source: Provider) -> &'static [ChatModel] {
    match source {
        Provider::OpenAi => OPENAI,
        Provider::ChatGpt => CHATGPT,
        _ => CLAUDE,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChatPick {
    pub source: Provider,
    pub model: &'static str,
    pub level: &'static str,
}

/// The pick a conversation answers with: its own, else the latest, else the
/// chat source's first model. A pick whose source is gone falls back.
pub fn current(db: &Database, conversation: Option<&str>) -> ChatPick {
    let slot = conversation.unwrap_or(LATEST);
    let read = |kind: &str| {
        db.get_setting(&key(kind, slot))
            .or_else(|| db.get_setting(&key(kind, LATEST)))
    };
    let source = read("source")
        .and_then(|value| Provider::from_str(&value).ok())
        .filter(|source| chat_source::is_ready(db, *source))
        .unwrap_or_else(|| chat_source::active(db));
    let list = models(source);
    let model = read("model")
        .and_then(|id| list.iter().find(|m| m.id == id))
        .unwrap_or(&list[0]);
    let level = read("level")
        .and_then(|level| model.levels.iter().find(|l| **l == level))
        .copied()
        .unwrap_or(model.default_level);
    ChatPick {
        source,
        model: model.id,
        level,
    }
}

/// What the menu offers besides the current pick: per connected family, the
/// models of one source (the one in use, else the first ready one).
pub fn options(
    db: &Database,
    pick: &ChatPick,
) -> Vec<(Family, Provider, Vec<&'static ChatModel>)> {
    [Family::Claude, Family::OpenAi]
        .into_iter()
        .filter_map(|wanted| {
            let source = if family(pick.source) == wanted {
                pick.source
            } else {
                chat_source::all().into_iter().find(|source| {
                    family(*source) == wanted
                        && chat_source::is_ready(db, *source)
                })?
            };
            let others: Vec<_> = models(source)
                .iter()
                .filter(|m| !(source == pick.source && m.id == pick.model))
                .collect();
            (!others.is_empty()).then_some((wanted, source, others))
        })
        .collect()
}

/// Picking a model keeps the level when the model accepts it. The chat
/// source follows, so new conversations and the AI screen agree.
pub fn choose_model(
    db: &Database,
    conversation: Option<&str>,
    source: Provider,
    model: &str,
) -> Result<ChatPick, String> {
    let current = current(db, conversation);
    let chosen = models(source)
        .iter()
        .find(|m| m.id == model)
        .ok_or_else(|| format!("unknown model {model}"))?;
    let level = chosen
        .levels
        .iter()
        .find(|l| **l == current.level)
        .copied()
        .unwrap_or(chosen.default_level);
    let pick = ChatPick {
        source,
        model: chosen.id,
        level,
    };
    remember(db, conversation, &pick)?;
    chat_source::choose(db, source)?;
    Ok(pick)
}

pub fn choose_level(
    db: &Database,
    conversation: Option<&str>,
    level: &str,
) -> Result<ChatPick, String> {
    let mut pick = current(db, conversation);
    pick.level = models(pick.source)
        .iter()
        .find(|m| m.id == pick.model)
        .and_then(|m| m.levels.iter().find(|l| **l == level))
        .copied()
        .ok_or_else(|| format!("unknown level {level}"))?;
    remember(db, conversation, &pick)?;
    Ok(pick)
}

/// Pins a pick on a conversation (and as the next one's start).
pub fn remember(
    db: &Database,
    conversation: Option<&str>,
    pick: &ChatPick,
) -> Result<(), String> {
    let slots = [Some(LATEST), conversation];
    for slot in slots.into_iter().flatten() {
        db.set_setting(&key("source", slot), pick.source.as_str())?;
        db.set_setting(&key("model", slot), pick.model)?;
        db.set_setting(&key("level", slot), pick.level)?;
    }
    Ok(())
}

fn key(kind: &str, slot: &str) -> String {
    format!("chat_{kind}:{slot}")
}
