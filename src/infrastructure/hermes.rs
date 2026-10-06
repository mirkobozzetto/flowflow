//! FlowFlow as a Hermes channel, through Hermes' own API server. A Hermes
//! session holds the conversation history; a run keeps going on the server
//! while the phone sleeps and can be followed again from its event log.

use crate::infrastructure::persistence::Database;
use serde::Deserialize;
use std::time::Duration;

pub const URL_SETTING: &str = "hermes_url";
pub const KEY_SETTING: &str = "hermes_api_key";

const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
// Session ids FlowFlow mints, so its sessions read apart from Telegram's.
const SESSION_PREFIX: &str = "flowflow_";

#[derive(Debug, Clone, PartialEq)]
pub enum HermesError {
    NotConfigured,
    Unreachable,
    KeyRefused,
    NotFound,
    Server(String),
}

/// One event of a run, as the chat needs it.
#[derive(Debug, Clone, PartialEq)]
pub enum RunEvent {
    Delta(String),
    ToolStarted { tool: String, preview: String },
    ToolDone { tool: String, failed: bool },
    Completed(String),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RunState {
    Running,
    Completed(String),
    Failed(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct HermesMessage {
    pub role: String,
    #[serde(default)]
    pub content: Option<serde_json::Value>,
    #[serde(default)]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolCall {
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    #[serde(default)]
    pub arguments: serde_json::Value,
}

impl HermesMessage {
    pub fn text(&self) -> String {
        match &self.content {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Array(parts)) => parts
                .iter()
                .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join(""),
            _ => String::new(),
        }
    }
}

/// Address as typed in Settings, made a base URL: https by default, no
/// trailing slash.
pub fn normalize_base(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() || trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    }
}

/// Takes every complete SSE frame out of `buf` and returns its `data`
/// payloads; comments (keepalives) and partial frames are left alone.
pub fn drain_sse_data(buf: &mut String) -> Vec<String> {
    let mut out = Vec::new();
    while let Some(end) = buf.find("\n\n") {
        let frame: String = buf.drain(..end + 2).collect();
        let data: Vec<&str> = frame
            .lines()
            .filter_map(|l| l.strip_prefix("data:"))
            .map(|d| d.strip_prefix(' ').unwrap_or(d))
            .collect();
        if !data.is_empty() {
            out.push(data.join("\n"));
        }
    }
    out
}

/// A run event payload to its sequence number and meaning; events the chat
/// does not show (reasoning, interim commentary, approvals) are None.
pub fn parse_run_event(data: &str) -> Option<(i64, RunEvent)> {
    let v: serde_json::Value = serde_json::from_str(data).ok()?;
    let seq = v.get("seq").and_then(|s| s.as_i64()).unwrap_or(-1);
    let s = |k: &str| {
        v.get(k)
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string()
    };
    let event = match v.get("event")?.as_str()? {
        "message.delta" => RunEvent::Delta(s("delta")),
        "tool.started" => RunEvent::ToolStarted {
            tool: s("tool"),
            preview: s("preview"),
        },
        "tool.completed" => RunEvent::ToolDone {
            tool: s("tool"),
            failed: v.get("error").and_then(|e| e.as_bool()).unwrap_or(false),
        },
        "tool.failed" => RunEvent::ToolDone {
            tool: s("tool"),
            failed: true,
        },
        "run.completed" => RunEvent::Completed(s("output")),
        "run.failed" | "run.cancelled" | "run.interrupted" => {
            RunEvent::Failed(s("error"))
        }
        _ => return None,
    };
    Some((seq, event))
}

/// A provider the user is signed in to, with the models it offers.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelProvider {
    pub slug: String,
    pub name: String,
    pub models: Vec<String>,
}

/// Hermes' default (provider, model) and every usable alternative.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModelOptions {
    pub provider: String,
    pub model: String,
    pub providers: Vec<ModelProvider>,
}

/// `/api/model/options`, keeping only providers that can answer.
pub fn parse_model_options(v: &serde_json::Value) -> ModelOptions {
    let s = |v: &serde_json::Value, k: &str| {
        v.get(k)
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string()
    };
    let providers = v
        .get("providers")
        .and_then(|p| p.as_array())
        .into_iter()
        .flatten()
        .filter(|p| {
            p.get("authenticated").and_then(|a| a.as_bool()) == Some(true)
        })
        .map(|p| ModelProvider {
            slug: s(p, "slug"),
            name: s(p, "name"),
            models: p
                .get("models")
                .and_then(|m| m.as_array())
                .into_iter()
                .flatten()
                .filter_map(|m| m.as_str().map(String::from))
                .collect(),
        })
        .filter(|p| !p.models.is_empty())
        .collect();
    ModelOptions {
        provider: s(v, "provider"),
        model: s(v, "model"),
        providers,
    }
}

/// A skill installed on Hermes.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Skill {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub category: String,
    /// How often Hermes used it; only the dashboard list counts.
    #[serde(default)]
    pub usage: u32,
}

/// `/v1/skills` (`{"data": [...]}`) or the dashboard's `/api/skills` (a bare
/// array); a skill switched off in Hermes is left out.
pub fn parse_skills(v: &serde_json::Value) -> Vec<Skill> {
    v.get("data")
        .unwrap_or(v)
        .as_array()
        .into_iter()
        .flatten()
        .filter(|s| s.get("enabled").and_then(|e| e.as_bool()) != Some(false))
        .filter_map(|s| serde_json::from_value::<Skill>(s.clone()).ok())
        .filter(|s| !s.name.trim().is_empty())
        .collect()
}

// The dashboard sits on the API host's default HTTPS port (the kit's
// `tailscale serve`); its skills list works where the API's /v1/skills fails
// on current Hermes releases (upstream issue 132317).
// ponytail: port assumed; add a setting if a setup serves it elsewhere.
pub fn dashboard_base(api_base: &str) -> Option<String> {
    let mut url = url::Url::parse(api_base).ok()?;
    url.set_port(None).ok()?;
    Some(url.as_str().trim_end_matches('/').to_string())
}

/// The session token the dashboard writes into its own page for its web app.
pub fn dashboard_token(page: &str) -> Option<String> {
    let rest = page.split("__HERMES_SESSION_TOKEN__=\"").nth(1)?;
    let token = rest.split('"').next()?;
    (!token.is_empty()).then(|| token.to_string())
}

fn run_state(v: &serde_json::Value) -> RunState {
    let text = |k: &str| {
        v.get(k)
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string()
    };
    match v.get("status").and_then(|s| s.as_str()).unwrap_or_default() {
        "completed" => RunState::Completed(text("output")),
        "failed" | "cancelled" | "interrupted" => {
            RunState::Failed(text("error"))
        }
        _ => RunState::Running,
    }
}

fn transport(e: reqwest::Error) -> HermesError {
    if e.is_connect() || e.is_timeout() || e.is_request() {
        HermesError::Unreachable
    } else {
        HermesError::Server(e.to_string())
    }
}

async fn checked(
    sent: Result<reqwest::Response, reqwest::Error>,
) -> Result<reqwest::Response, HermesError> {
    let resp = sent.map_err(transport)?;
    match resp.status().as_u16() {
        200..=299 => Ok(resp),
        401 | 403 => Err(HermesError::KeyRefused),
        404 => Err(HermesError::NotFound),
        code => {
            let body = resp.text().await.unwrap_or_default();
            Err(HermesError::Server(format!("{code}: {body}")))
        }
    }
}

pub struct HermesClient {
    base: String,
    key: String,
    http: reqwest::Client,
}

impl HermesClient {
    pub fn new(base: &str, key: &str) -> Result<Self, HermesError> {
        let base = normalize_base(base);
        let key = key.trim().to_string();
        if base.is_empty() || key.is_empty() {
            return Err(HermesError::NotConfigured);
        }
        let http = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .map_err(|e| HermesError::Server(e.to_string()))?;
        Ok(Self { base, key, http })
    }

    pub fn from_db(db: &Database) -> Result<Self, HermesError> {
        Self::new(
            &db.get_setting(URL_SETTING).unwrap_or_default(),
            &db.get_setting(KEY_SETTING).unwrap_or_default(),
        )
    }

    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.http
            .get(format!("{}{path}", self.base))
            .bearer_auth(&self.key)
    }

    fn post(&self, path: &str) -> reqwest::RequestBuilder {
        self.http
            .post(format!("{}{path}", self.base))
            .bearer_auth(&self.key)
    }

    /// Reachable and the key accepted.
    pub async fn check(&self) -> Result<(), HermesError> {
        checked(self.get("/v1/models").timeout(REQUEST_TIMEOUT).send().await)
            .await
            .map(|_| ())
    }

    pub async fn create_session(&self) -> Result<String, HermesError> {
        let id = format!("{SESSION_PREFIX}{}", uuid::Uuid::new_v4().simple());
        checked(
            self.post("/api/sessions")
                .json(&serde_json::json!({ "id": id }))
                .timeout(REQUEST_TIMEOUT)
                .send()
                .await,
        )
        .await?;
        Ok(id)
    }

    pub async fn messages(
        &self,
        session_id: &str,
    ) -> Result<Vec<HermesMessage>, HermesError> {
        #[derive(Deserialize)]
        struct Page {
            data: Vec<HermesMessage>,
        }
        let resp = checked(
            self.get(&format!("/api/sessions/{session_id}/messages"))
                .timeout(REQUEST_TIMEOUT)
                .send()
                .await,
        )
        .await?;
        let page: Page = resp
            .json()
            .await
            .map_err(|e| HermesError::Server(e.to_string()))?;
        Ok(page.data)
    }

    async fn json_at(
        &self,
        path: &str,
    ) -> Result<serde_json::Value, HermesError> {
        checked(self.get(path).timeout(REQUEST_TIMEOUT).send().await)
            .await?
            .json()
            .await
            .map_err(|e| HermesError::Server(e.to_string()))
    }

    pub async fn model_options(&self) -> Result<ModelOptions, HermesError> {
        Ok(parse_model_options(
            &self.json_at("/api/model/options").await?,
        ))
    }

    /// Length of a list endpoint (`data` array or bare array).
    pub async fn count(&self, path: &str) -> Result<usize, HermesError> {
        let v = self.json_at(path).await?;
        Ok(v.get("data")
            .or_else(|| v.get("jobs"))
            .unwrap_or(&v)
            .as_array()
            .map_or(0, Vec::len))
    }

    /// Installed skills, in Hermes' own order: the API's list, else the
    /// dashboard's, read the way its web app reads it.
    pub async fn skills(&self) -> Result<Vec<Skill>, HermesError> {
        match self.json_at("/v1/skills").await {
            Ok(v) => Ok(parse_skills(&v)),
            Err(api) => self.dashboard_skills().await.map_err(|_| api),
        }
    }

    async fn dashboard_skills(&self) -> Result<Vec<Skill>, HermesError> {
        let base = dashboard_base(&self.base).ok_or(HermesError::NotFound)?;
        let page = checked(
            self.http
                .get(format!("{base}/"))
                .timeout(REQUEST_TIMEOUT)
                .send()
                .await,
        )
        .await?
        .text()
        .await
        .map_err(|e| HermesError::Server(e.to_string()))?;
        let token = dashboard_token(&page).ok_or(HermesError::KeyRefused)?;
        let list: serde_json::Value = checked(
            self.http
                .get(format!("{base}/api/skills"))
                .header("X-Hermes-Session-Token", token)
                .timeout(REQUEST_TIMEOUT)
                .send()
                .await,
        )
        .await?
        .json()
        .await
        .map_err(|e| HermesError::Server(e.to_string()))?;
        Ok(parse_skills(&list))
    }

    /// Starts a turn on the session, on the chosen (provider, model) and
    /// reasoning effort or Hermes' own defaults; it runs on the server
    /// whatever happens to this connection. Images (data URLs) ride along
    /// as parts of the user message.
    pub async fn start_run(
        &self,
        session_id: &str,
        input: &str,
        images: &[String],
        model: Option<(&str, &str)>,
        effort: Option<&str>,
    ) -> Result<String, HermesError> {
        let input = if images.is_empty() {
            serde_json::json!(input)
        } else {
            let mut parts =
                vec![serde_json::json!({ "type": "text", "text": input })];
            parts.extend(images.iter().map(|url| {
                serde_json::json!({ "type": "image_url", "image_url": { "url": url } })
            }));
            serde_json::json!([{ "role": "user", "content": parts }])
        };
        let mut body = serde_json::json!({
            "input": input,
            "session_id": session_id,
        });
        if let Some((provider, model)) = model {
            body["provider"] = provider.into();
            body["model"] = model.into();
        }
        if let Some(effort) = effort {
            body["model_options"] = serde_json::json!({ "reasoning": { "enabled": true, "effort": effort } });
        }
        let resp = checked(
            self.post("/v1/runs")
                .json(&body)
                .timeout(REQUEST_TIMEOUT)
                .send()
                .await,
        )
        .await?;
        let v: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| HermesError::Server(e.to_string()))?;
        v.get("run_id")
            .and_then(|r| r.as_str())
            .map(String::from)
            .ok_or_else(|| HermesError::Server("no run_id".into()))
    }

    pub async fn run_state(
        &self,
        run_id: &str,
    ) -> Result<RunState, HermesError> {
        let resp = checked(
            self.get(&format!("/v1/runs/{run_id}"))
                .timeout(REQUEST_TIMEOUT)
                .send()
                .await,
        )
        .await?;
        let v: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| HermesError::Server(e.to_string()))?;
        Ok(run_state(&v))
    }

    /// Streams the run's events after `after_seq` (-1: from the start, the
    /// server replays what it kept) until the server closes the stream.
    pub async fn follow_run(
        &self,
        run_id: &str,
        after_seq: i64,
        mut on_event: impl FnMut(i64, RunEvent),
    ) -> Result<(), HermesError> {
        use futures::StreamExt;
        let mut req = self.get(&format!("/v1/runs/{run_id}/events"));
        if after_seq >= 0 {
            req = req.header("Last-Event-ID", after_seq.to_string());
        }
        let resp = checked(req.send().await).await?;
        let stream = resp.bytes_stream();
        tokio::pin!(stream);
        let mut buf = String::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(transport)?;
            buf.push_str(
                &String::from_utf8_lossy(&chunk).replace("\r\n", "\n"),
            );
            for data in drain_sse_data(&mut buf) {
                if let Some((seq, event)) = parse_run_event(&data) {
                    on_event(seq, event);
                }
            }
        }
        Ok(())
    }
}
