//! A meeting summary on demand. The note goes to the model in reading order,
//! each spoken paragraph numbered with where it starts; an item citing a
//! number points back to that moment of the audio.
use crate::application::constants::SUMMARY_SYSTEM_PROMPT;
use crate::application::error::LlmError;
use crate::application::reminders_extract::extract_json_object;
use crate::domain::reading::{locate_paragraphs, Segment, PAUSE_PARAGRAPH_MS};
use crate::domain::summary::{
    source_hash, Moment, NoteSummary, Summary, SummaryItem,
};
use crate::domain::Word;
use crate::infrastructure::llm::LlmClient;
use crate::infrastructure::persistence::Database;

/// Summarize the note as `content` reads now, in the app language `lang`,
/// and keep it with the note. Without AI consent or a connection this fails
/// and nothing is stored.
// ponytail: the whole note in one call (about 15k tokens an hour); chunk by
// groups of paragraphs if recordings past 2 h hit the context.
pub async fn summarize_note(
    db: &Database,
    note_id: &str,
    content: &str,
    lang: &str,
) -> Result<NoteSummary, LlmError> {
    let client = LlmClient::from_db(db)?;
    let clips: Vec<(String, Vec<Word>)> = db
        .list_audios(note_id)
        .map_err(LlmError::Completion)?
        .into_iter()
        .filter_map(|a| Some((a.id.clone(), db.words_for_audio(&a.id)?.words)))
        .collect();
    let (input, moments) = numbered_body(content, &clips);
    let lang_name = if lang == "fr" { "French" } else { "English" };
    let system =
        format!("{SUMMARY_SYSTEM_PROMPT}\n\nWrite every item in {lang_name}.");
    let reply = client.chat_reading(&system, &input).await?;
    let summary = NoteSummary {
        note_id: note_id.to_string(),
        body: parse_summary(&reply, &moments)?,
        lang: lang.to_string(),
        model: client.reading_model().to_string(),
        source_hash: source_hash(content),
    };
    db.save_summary(
        note_id,
        &summary.body,
        &summary.lang,
        &summary.model,
        &summary.source_hash,
    )
    .map_err(LlmError::Completion)?;
    Ok(summary)
}

/// The note in reading order: each spoken paragraph as `[n] m:ss text`, text
/// typed or corrected by hand as is. `moments[n - 1]` is where paragraph `n`
/// starts. `clips` are the note's recordings, as `(audio_id, words)`.
pub fn numbered_body(
    content: &str,
    clips: &[(String, Vec<Word>)],
) -> (String, Vec<Moment>) {
    let transcripts: Vec<&[Word]> =
        clips.iter().map(|(_, w)| w.as_slice()).collect();
    let mut moments = Vec::new();
    let lines: Vec<String> =
        locate_paragraphs(content, &transcripts, PAUSE_PARAGRAPH_MS)
            .into_iter()
            .map(|segment| match segment {
                Segment::Text(text) => text,
                Segment::Paragraph { audio, words } => {
                    let spoken = &clips[audio].1[words];
                    let ms = spoken[0].start_ms;
                    moments.push(Moment {
                        audio_id: clips[audio].0.clone(),
                        ms,
                    });
                    let text = spoken
                        .iter()
                        .map(|w| w.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" ");
                    let secs = ms / 1000;
                    format!(
                        "[{}] {}:{:02} {text}",
                        moments.len(),
                        secs / 60,
                        secs % 60
                    )
                }
            })
            .collect();
    (lines.join("\n"), moments)
}

/// The model's answer as a summary. A cited paragraph that does not exist
/// leaves the item without a time rather than with a wrong one.
pub fn parse_summary(
    reply: &str,
    moments: &[Moment],
) -> Result<Summary, LlmError> {
    #[derive(serde::Deserialize)]
    struct Raw {
        #[serde(default)]
        key_points: Vec<RawItem>,
        #[serde(default)]
        decisions: Vec<RawItem>,
        #[serde(default)]
        actions: Vec<RawItem>,
        #[serde(default)]
        questions: Vec<RawItem>,
    }
    #[derive(serde::Deserialize)]
    struct RawItem {
        text: String,
        #[serde(default)]
        at: Option<usize>,
    }
    let trimmed = reply.trim();
    let json = extract_json_object(trimmed).unwrap_or(trimmed);
    let raw: Raw = serde_json::from_str(json).map_err(|e| {
        LlmError::Completion(format!("Invalid summary JSON: {e}"))
    })?;
    let items = |raw: Vec<RawItem>| -> Vec<SummaryItem> {
        raw.into_iter()
            .filter(|r| !r.text.trim().is_empty())
            .map(|r| SummaryItem {
                text: r.text.trim().to_string(),
                at: r
                    .at
                    .and_then(|n| n.checked_sub(1))
                    .and_then(|i| moments.get(i))
                    .cloned(),
            })
            .collect()
    };
    let summary = Summary {
        key_points: items(raw.key_points),
        decisions: items(raw.decisions),
        actions: items(raw.actions),
        questions: items(raw.questions),
    };
    if summary.is_empty() {
        return Err(LlmError::Completion("Empty summary".into()));
    }
    Ok(summary)
}
