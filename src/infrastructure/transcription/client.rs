use super::hesitations::clean_hesitations_words;
use crate::domain::transcript::words_from_span;
use crate::domain::{Dictionary, Transcript, Word};
use serde::Deserialize;
use std::path::Path;
use std::time::Duration;

const BASE_URL: &str = "https://api.soniox.com";
const POLL_INTERVAL: Duration = Duration::from_secs(2);
const MAX_POLLS: u32 = 9000;

#[derive(Deserialize)]
struct FileResponse {
    id: String,
}

#[derive(Deserialize)]
struct TranscriptionResponse {
    id: String,
}

#[derive(Deserialize)]
struct TranscriptionStatus {
    status: String,
}

#[derive(Deserialize)]
struct TranscriptResponse {
    text: String,
    /// Soniox returns timestamps by default, so no request change is needed.
    /// Defaulted rather than required: a response without them degrades to an
    /// empty word list and the UI falls back to today's paragraph.
    #[serde(default)]
    tokens: Vec<SonioxToken>,
}

#[derive(Deserialize)]
pub struct SonioxToken {
    pub text: String,
    pub start_ms: u32,
    pub end_ms: u32,
    pub confidence: f32,
}

/// Soniox tokens are sub-word ("Bon", "j", "our"). A **leading space** in a
/// token's text is what opens a new word; anything else continues the current
/// one, which is also how punctuation stays glued to the word it belongs to.
///
/// Verified against a real French response on 2026-07-26, captured as
/// `tests/fixtures/soniox_tokens.json`: applying this rule reproduces Soniox's
/// own `text` field exactly.
pub fn group_tokens_to_words(tokens: &[SonioxToken]) -> Vec<Word> {
    let mut words: Vec<Word> = Vec::new();
    let mut confidences: Vec<Vec<f32>> = Vec::new();

    for token in tokens {
        if token.text.trim().is_empty() {
            continue;
        }
        let starts_word = token.text.starts_with(' ') || words.is_empty();
        match words.last_mut() {
            Some(word) if !starts_word => {
                word.text.push_str(&token.text);
                word.end_ms = token.end_ms.max(word.end_ms);
                confidences
                    .last_mut()
                    .expect("a word always has a confidence bucket")
                    .push(token.confidence);
            }
            _ => {
                words.push(Word::new(
                    token.text.trim(),
                    token.start_ms,
                    token.end_ms,
                    token.confidence,
                ));
                confidences.push(vec![token.confidence]);
            }
        }
    }

    for (word, scores) in words.iter_mut().zip(confidences) {
        word.confidence = scores.iter().sum::<f32>() / scores.len() as f32;
    }
    // A token carrying an interior space would otherwise put whitespace inside a
    // word and break the one-word-one-span invariant.
    words
        .into_iter()
        .flat_map(|w| {
            words_from_span(&w.text, w.start_ms, w.end_ms, w.confidence)
        })
        .collect()
}

/// `context.terms` is Soniox's native exact-spelling vocabulary field. Omitted
/// entirely when the dictionary is empty, so an unused dictionary changes nothing
/// about the request.
pub fn transcription_request_body(
    file_id: &str,
    language: Option<&str>,
    terms: &[&str],
) -> serde_json::Value {
    let mut body = serde_json::json!({
        "model": "stt-async-v4",
        "file_id": file_id,
    });
    if let Some(lang) = language {
        body["language_hints"] = serde_json::json!([lang]);
    }
    if !terms.is_empty() {
        body["context"] = serde_json::json!({ "terms": terms });
    }
    body
}

pub struct SonioxClient {
    client: reqwest::Client,
    api_key: String,
    lang: String,
    dictionary: Dictionary,
}

impl SonioxClient {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_secs(600))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            api_key,
            lang: crate::infrastructure::platform::detect_system_language(),
            dictionary: Dictionary::default(),
        }
    }

    pub fn with_lang(mut self, lang: String) -> Self {
        self.lang = lang;
        self
    }

    pub fn with_dictionary(mut self, dictionary: Dictionary) -> Self {
        self.dictionary = dictionary;
        self
    }

    pub fn dictionary(&self) -> &Dictionary {
        &self.dictionary
    }

    pub fn from_db(
        db: &crate::infrastructure::persistence::Database,
    ) -> Result<Self, String> {
        let lang = crate::application::i18n::ui_lang(db);
        if db.get_setting("ai_consent") != Some("true".to_string()) {
            return Err(crate::application::i18n::t(&lang, "error-ai-consent"));
        }
        let key = db
            .get_setting("soniox_api_key")
            .or_else(|| std::env::var("SONIOX_API_KEY").ok())
            .or_else(|| option_env!("SONIOX_API_KEY").map(String::from))
            .unwrap_or_default();
        if key.is_empty() || key == "your_key_here" {
            return Err(crate::application::i18n::t(
                &lang,
                "stt-error-soniox-key",
            ));
        }
        let dictionary = crate::application::transcription_dictionary::load(db);
        Ok(Self::new(key).with_lang(lang).with_dictionary(dictionary))
    }

    async fn upload_file(&self, path: &Path) -> Result<String, String> {
        eprintln!("[soniox] uploading {}", path.display());
        let bytes = tokio::fs::read(path)
            .await
            .map_err(|e| format!("Read file error: {e}"))?;
        let filename = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        eprintln!("[soniox] upload size = {} bytes", bytes.len());
        let mut last_err = String::new();
        for attempt in 1..=3u32 {
            let part = reqwest::multipart::Part::bytes(bytes.clone())
                .file_name(filename.clone());
            let form = reqwest::multipart::Form::new().part("file", part);
            match self
                .client
                .post(format!("{BASE_URL}/v1/files"))
                .bearer_auth(&self.api_key)
                .multipart(form)
                .send()
                .await
            {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let file_resp: FileResponse = resp
                            .json()
                            .await
                            .map_err(|e| format!("Parse error: {e}"))?;
                        eprintln!(
                            "[soniox] uploaded, file_id={}",
                            file_resp.id
                        );
                        return Ok(file_resp.id);
                    }
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    return Err(format!("Upload failed ({status}): {body}"));
                }
                Err(e) => {
                    last_err = format!("Upload error: {e}");
                    eprintln!(
                        "[soniox] upload attempt {attempt}/3 failed: {e}"
                    );
                }
            }
            if attempt < 3 {
                tokio::time::sleep(Duration::from_secs(2 * attempt as u64))
                    .await;
            }
        }
        Err(last_err)
    }

    async fn create_transcription(
        &self,
        file_id: &str,
        language: Option<&str>,
    ) -> Result<String, String> {
        eprintln!("[soniox] creating transcription for {file_id}");
        let body = transcription_request_body(
            file_id,
            language,
            &self.dictionary.terms(),
        );
        let resp = self
            .client
            .post(format!("{BASE_URL}/v1/transcriptions"))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Create transcription error: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!(
                "Transcription create failed ({status}): {body}"
            ));
        }
        let tr: TranscriptionResponse =
            resp.json().await.map_err(|e| format!("Parse error: {e}"))?;
        eprintln!("[soniox] transcription_id={}", tr.id);
        Ok(tr.id)
    }

    pub async fn check_status(
        &self,
        transcription_id: &str,
    ) -> Result<Option<Transcript>, String> {
        let resp = self
            .client
            .get(format!("{BASE_URL}/v1/transcriptions/{transcription_id}"))
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| format!("Poll error: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Poll failed ({status}): {body}"));
        }
        let status: TranscriptionStatus =
            resp.json().await.map_err(|e| format!("Parse error: {e}"))?;
        eprintln!("[soniox] status={}", status.status);
        match status.status.as_str() {
            "completed" => {
                Ok(Some(self.fetch_transcript(transcription_id).await?))
            }
            "error" | "failed" => {
                Err(crate::application::i18n::t(&self.lang, "stt-error-server"))
            }
            _ => Ok(None),
        }
    }

    pub async fn poll_transcript(
        &self,
        transcription_id: &str,
    ) -> Result<Transcript, String> {
        eprintln!("[soniox] polling for {transcription_id}");
        for _ in 1..=MAX_POLLS {
            if let Some(transcript) =
                self.check_status(transcription_id).await?
            {
                return Ok(transcript);
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
        Err(crate::application::i18n::t(&self.lang, "stt-error-timeout"))
    }

    async fn fetch_transcript(
        &self,
        transcription_id: &str,
    ) -> Result<Transcript, String> {
        let resp = self
            .client
            .get(format!(
                "{BASE_URL}/v1/transcriptions/{transcription_id}/transcript"
            ))
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| format!("Fetch transcript error: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Fetch transcript failed ({status}): {body}"));
        }
        let tr: TranscriptResponse =
            resp.json().await.map_err(|e| format!("Parse error: {e}"))?;
        let words = group_tokens_to_words(&tr.tokens);
        eprintln!(
            "[soniox] transcript received ({} chars, {} words)",
            tr.text.len(),
            words.len()
        );
        // No tokens (an older or degraded response) still yields a usable
        // transcript: the text is rebuilt into untimed words, so the UI renders
        // today's paragraph instead of failing.
        if words.is_empty() {
            return Ok(Transcript::from_text(&tr.text));
        }
        Ok(Transcript::new(words))
    }

    pub async fn start_transcription(
        &self,
        path: &Path,
        language: Option<&str>,
    ) -> Result<(String, String), String> {
        let file_id = self.upload_file(path).await?;
        let tr_id = self.create_transcription(&file_id, language).await?;
        Ok((tr_id, file_id))
    }

    pub async fn delete_file(&self, file_id: &str) -> Result<(), String> {
        eprintln!("[soniox] deleting file {file_id}");
        let resp = self
            .client
            .delete(format!("{BASE_URL}/v1/files/{file_id}"))
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| format!("Delete file error: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Delete file failed ({status}): {body}"));
        }
        Ok(())
    }

    pub async fn transcribe(
        &self,
        path: &Path,
        language: Option<&str>,
    ) -> Result<Transcript, String> {
        let (tr_id, file_id) = self.start_transcription(path, language).await?;
        let raw = self.poll_transcript(&tr_id).await;
        let _ = self.delete_file(&file_id).await;
        Ok(self.clean(raw?))
    }

    /// The cleaning both entry points share: hesitations dropped, then declared
    /// terms corrected, word by word so every survivor keeps its timing.
    pub fn clean(&self, transcript: Transcript) -> Transcript {
        let cleaned = clean_hesitations_words(transcript.words);
        Transcript::new(self.dictionary.apply_words(cleaned))
    }
}
