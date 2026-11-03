//! Chapters of a recording: its pause paragraphs go to the model numbered and
//! timed, and the model answers where each topic starts. Anchored on a
//! paragraph number, a chapter always starts exactly on a paragraph.
use crate::application::constants::CHAPTERS_SYSTEM_PROMPT;
use crate::application::error::LlmError;
use crate::application::reminders_extract::extract_json_object;
use crate::domain::reading::{paragraph_ranges, Chapter, PAUSE_PARAGRAPH_MS};
use crate::domain::Word;
use crate::infrastructure::llm::LlmClient;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::platform::device_log;
use std::ops::Range;
use std::time::Instant;

/// Generate a clip's chapters and store them: the stored chapters, or `None`
/// when the store refused them (already there, or the words changed meanwhile).
/// Without AI consent or a connection this fails, and the note reads in
/// paragraphs only. Each run logs a start line and an end line, so a run cut
/// short shows as a start without an end.
// ponytail: the whole transcript in one call (about 15k tokens an hour);
// chunk by groups of paragraphs if recordings past 2 h hit the context.
pub async fn generate_chapters(
    db: &Database,
    audio_id: &str,
    words: &[Word],
) -> Result<Option<Vec<Chapter>>, LlmError> {
    let ranges = paragraph_ranges(words, PAUSE_PARAGRAPH_MS);
    device_log(&format!(
        "[chapters] {audio_id}: start, {} words in {} paragraphs",
        words.len(),
        ranges.len()
    ));
    let started = Instant::now();
    let outcome = ask_and_store(db, audio_id, words, &ranges).await;
    let ms = started.elapsed().as_millis();
    match &outcome {
        Ok(Some(chapters)) => device_log(&format!(
            "[chapters] {audio_id}: {} chapters kept in {ms} ms",
            chapters.len()
        )),
        Ok(None) => device_log(&format!(
            "[chapters] {audio_id}: refused in {ms} ms, chapters already there or words changed"
        )),
        Err(e) => {
            device_log(&format!("[chapters] {audio_id}: failed in {ms} ms: {e}"))
        }
    }
    outcome
}

async fn ask_and_store(
    db: &Database,
    audio_id: &str,
    words: &[Word],
    ranges: &[Range<usize>],
) -> Result<Option<Vec<Chapter>>, LlmError> {
    let client = LlmClient::from_db(db)?;
    let reply = client
        .chat_reading(CHAPTERS_SYSTEM_PROMPT, &numbered(words, ranges))
        .await?;
    let chapters = parse_chapters(&reply, ranges)?;
    let kept = db
        .set_audio_chapters(audio_id, words.len(), &chapters)
        .map_err(LlmError::Completion)?;
    Ok(kept.then_some(chapters))
}

/// The model's answer as chapters. A start outside the paragraphs, or not
/// after the previous one, is dropped rather than misplaced.
pub fn parse_chapters(
    reply: &str,
    paragraphs: &[Range<usize>],
) -> Result<Vec<Chapter>, LlmError> {
    #[derive(serde::Deserialize)]
    struct Envelope {
        chapters: Vec<Raw>,
    }
    #[derive(serde::Deserialize)]
    struct Raw {
        start: usize,
        title: String,
        #[serde(default)]
        summary: String,
    }
    let trimmed = reply.trim();
    let json = extract_json_object(trimmed).unwrap_or(trimmed);
    let envelope: Envelope = serde_json::from_str(json).map_err(|e| {
        LlmError::Completion(format!("Invalid chapters JSON: {e}"))
    })?;
    let mut chapters: Vec<Chapter> = Vec::new();
    for raw in envelope.chapters {
        let title = raw.title.trim();
        let Some(paragraph) =
            raw.start.checked_sub(1).and_then(|i| paragraphs.get(i))
        else {
            continue;
        };
        let after_previous = chapters
            .last()
            .is_none_or(|c| c.start_word < paragraph.start);
        if title.is_empty() || !after_previous {
            continue;
        }
        chapters.push(Chapter {
            start_word: paragraph.start,
            title: title.to_string(),
            summary: raw.summary.trim().to_string(),
        });
    }
    if chapters.is_empty() {
        return Err(LlmError::Completion("No chapters in the answer".into()));
    }
    Ok(chapters)
}

fn numbered(words: &[Word], paragraphs: &[Range<usize>]) -> String {
    paragraphs
        .iter()
        .enumerate()
        .map(|(i, range)| {
            let secs = words[range.start].start_ms / 1000;
            let text = words[range.clone()]
                .iter()
                .map(|w| w.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            format!("[{}] {}:{:02} {text}", i + 1, secs / 60, secs % 60)
        })
        .collect::<Vec<_>>()
        .join("\n")
}
