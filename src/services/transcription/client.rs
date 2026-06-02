use super::hesitations::clean_hesitations;
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
}

pub struct SonioxClient {
    client: reqwest::Client,
    api_key: String,
}

impl SonioxClient {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_secs(600))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { client, api_key }
    }

    pub fn from_db(db: &crate::db::Database) -> Result<Self, String> {
        if db.get_setting("ai_consent") != Some("true".to_string()) {
            return Err("Consentement IA requis".to_string());
        }
        let key = db
            .get_setting("soniox_api_key")
            .or_else(|| std::env::var("SONIOX_API_KEY").ok())
            .or_else(|| option_env!("SONIOX_API_KEY").map(String::from))
            .unwrap_or_default();
        if key.is_empty() || key == "your_key_here" {
            return Err("SONIOX_API_KEY not configured".to_string());
        }
        Ok(Self::new(key))
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
        let mut body = serde_json::json!({
            "model": "stt-async-v4",
            "file_id": file_id,
        });
        if let Some(lang) = language {
            body["language_hints"] = serde_json::json!([lang]);
        }
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
    ) -> Result<Option<String>, String> {
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
                Err("Transcription failed on server".to_string())
            }
            _ => Ok(None),
        }
    }

    pub async fn poll_transcript(
        &self,
        transcription_id: &str,
    ) -> Result<String, String> {
        eprintln!("[soniox] polling for {transcription_id}");
        for _ in 1..=MAX_POLLS {
            if let Some(text) = self.check_status(transcription_id).await? {
                return Ok(text);
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
        Err("Transcription timeout (5 h)".to_string())
    }

    async fn fetch_transcript(
        &self,
        transcription_id: &str,
    ) -> Result<String, String> {
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
        eprintln!("[soniox] transcript received ({} chars)", tr.text.len());
        Ok(tr.text)
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
    ) -> Result<String, String> {
        let (tr_id, file_id) = self.start_transcription(path, language).await?;
        let raw = self.poll_transcript(&tr_id).await;
        let _ = self.delete_file(&file_id).await;
        Ok(clean_hesitations(&raw?))
    }
}
