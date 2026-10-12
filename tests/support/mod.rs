//! A scripted HTTP server for client tests.

use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// A scripted Hermes: each accepted connection gets the next response whose
/// path matches; the request heads are kept for assertions.
pub struct Script {
    pub responses: Vec<(&'static str, String)>,
    pub seen: Arc<Mutex<Vec<String>>>,
}

pub fn json(status: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

// Head and body may arrive in separate reads: read up to Content-Length.
async fn read_request(stream: &mut tokio::net::TcpStream) -> String {
    let mut raw = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = stream.read(&mut chunk).await.unwrap();
        raw.extend_from_slice(&chunk[..n]);
        let text = String::from_utf8_lossy(&raw).to_string();
        if let Some(end) = text.find("\r\n\r\n") {
            let length = text[..end]
                .lines()
                .find_map(|l| {
                    l.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                })
                .unwrap_or(0);
            if raw.len() >= end + 4 + length || n == 0 {
                return text;
            }
        } else if n == 0 {
            return text;
        }
    }
}

pub async fn serve(script: Script) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        let mut responses = script.responses;
        while !responses.is_empty() {
            let (mut stream, _) = listener.accept().await.unwrap();
            let head = read_request(&mut stream).await;
            let line = head.lines().next().unwrap_or_default().to_string();
            script.seen.lock().unwrap().push(head);
            let at = responses
                .iter()
                .position(|(prefix, _)| line.starts_with(prefix))
                .unwrap_or_else(|| panic!("unexpected request {line}"));
            let (_, response) = responses.remove(at);
            stream.write_all(response.as_bytes()).await.unwrap();
            let _ = stream.shutdown().await;
        }
    });
    base
}
