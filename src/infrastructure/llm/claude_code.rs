//! A Claude subscription answers only requests shaped like Claude Code's:
//! bearer token, its betas and headers, and its identity as the first system
//! block (as Pi sends them).
use bytes::Bytes;
use rig::http_client::{
    self, HeaderMap, HeaderValue, HttpClientExt, LazyBody, MultipartForm,
    Request, Response, StreamingResponse,
};
use rig::providers::anthropic;
use rig::wasm_compat::WasmCompatSend;
use serde_json::{json, Value};
use std::future::Future;

const IDENTITY: &str =
    "You are Claude Code, Anthropic's official CLI for Claude.";
const BETAS: [&str; 2] = ["claude-code-20250219", "oauth-2025-04-20"];
const USER_AGENT: &str = "claude-cli/2.1.280";

pub type ClaudeCodeClient = anthropic::Client<ClaudeCodeHttp>;

pub fn client(access_token: &str) -> http_client::Result<ClaudeCodeClient> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "authorization",
        HeaderValue::from_str(&format!("Bearer {access_token}"))?,
    );
    headers.insert("user-agent", HeaderValue::from_static(USER_AGENT));
    headers.insert("x-app", HeaderValue::from_static("cli"));
    anthropic::Client::builder()
        .http_client(ClaudeCodeHttp::default())
        .http_headers(headers)
        .api_key(access_token)
        .anthropic_betas(&BETAS)
        .build()
}

/// rig always sends the token as `x-api-key` too: dropped here, and the
/// identity block goes in front of the app's own instructions.
#[derive(Clone, Debug, Default)]
pub struct ClaudeCodeHttp(reqwest::Client);

impl HttpClientExt for ClaudeCodeHttp {
    fn send<T, U>(
        &self,
        req: Request<T>,
    ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>>
           + WasmCompatSend
           + 'static
    where
        T: Into<Bytes>,
        T: WasmCompatSend,
        U: From<Bytes>,
        U: WasmCompatSend + 'static,
    {
        self.0.send(claude_code_request(req))
    }

    fn send_multipart<U>(
        &self,
        mut req: Request<MultipartForm>,
    ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>>
           + WasmCompatSend
           + 'static
    where
        U: From<Bytes>,
        U: WasmCompatSend + 'static,
    {
        req.headers_mut().remove("x-api-key");
        self.0.send_multipart(req)
    }

    fn send_streaming<T>(
        &self,
        req: Request<T>,
    ) -> impl Future<Output = http_client::Result<StreamingResponse>> + WasmCompatSend
    where
        T: Into<Bytes> + WasmCompatSend,
    {
        self.0.send_streaming(claude_code_request(req))
    }
}

pub fn claude_code_request<T: Into<Bytes>>(req: Request<T>) -> Request<Bytes> {
    let (mut parts, body) = req.into_parts();
    parts.headers.remove("x-api-key");
    parts.headers.remove("content-length");
    let body: Bytes = body.into();
    let body = with_identity(&body).map(Bytes::from).unwrap_or(body);
    Request::from_parts(parts, body)
}

fn with_identity(body: &[u8]) -> Option<Vec<u8>> {
    let mut request: Value = serde_json::from_slice(body).ok()?;
    let fields = request.as_object_mut()?;
    fields.get("messages")?;
    let identity = json!({ "type": "text", "text": IDENTITY });
    let system = match fields.remove("system") {
        Some(Value::Array(mut blocks)) => {
            blocks.insert(0, identity);
            blocks
        }
        Some(Value::String(text)) if !text.is_empty() => {
            vec![identity, json!({ "type": "text", "text": text })]
        }
        _ => vec![identity],
    };
    fields.insert("system".into(), Value::Array(system));
    serde_json::to_vec(&request).ok()
}
