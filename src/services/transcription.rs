use serde::Deserialize;
use std::path::Path;
use std::time::Duration;

const BASE_URL: &str = "https://api.soniox.com";
const POLL_INTERVAL: Duration = Duration::from_secs(2);
const MAX_POLLS: u32 = 60;

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
        Self {
            client: reqwest::Client::new(),
            api_key,
        }
    }

    pub fn from_env() -> Result<Self, String> {
        let key = std::env::var("SONIOX_API_KEY")
            .ok()
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
        let part = reqwest::multipart::Part::bytes(bytes).file_name(filename);
        let form = reqwest::multipart::Form::new().part("file", part);
        let resp = self
            .client
            .post(format!("{BASE_URL}/v1/files"))
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("Upload error: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Upload failed ({status}): {body}"));
        }
        let file_resp: FileResponse =
            resp.json().await.map_err(|e| format!("Parse error: {e}"))?;
        eprintln!("[soniox] uploaded, file_id={}", file_resp.id);
        Ok(file_resp.id)
    }

    async fn create_transcription(
        &self,
        file_id: &str,
    ) -> Result<String, String> {
        eprintln!("[soniox] creating transcription for {file_id}");
        let body = serde_json::json!({
            "model": "stt-async-v4",
            "file_id": file_id,
            "language_hints": ["fr"],
            "language_hints_strict": true
        });
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

    async fn poll_transcript(
        &self,
        transcription_id: &str,
    ) -> Result<String, String> {
        eprintln!("[soniox] polling for {transcription_id}");
        for attempt in 1..=MAX_POLLS {
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
            eprintln!(
                "[soniox] poll {attempt}/{MAX_POLLS}: status={}",
                status.status
            );
            match status.status.as_str() {
                "completed" => {
                    return self.fetch_transcript(transcription_id).await
                }
                "error" | "failed" => {
                    return Err("Transcription failed on server".to_string());
                }
                _ => tokio::time::sleep(POLL_INTERVAL).await,
            }
        }
        Err("Transcription timeout (2 min)".to_string())
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

    pub async fn transcribe(&self, path: &Path) -> Result<String, String> {
        let file_id = self.upload_file(path).await?;
        let tr_id = self.create_transcription(&file_id).await?;
        self.poll_transcript(&tr_id).await
    }
}
