use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use base64::Engine;
use flowflow::infrastructure::chatgpt_auth;
use serde_json::json;
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tempfile::tempdir;

static AUTH_ENV_LOCK: Mutex<()> = Mutex::new(());

struct AuthEnv {
    _lock: MutexGuard<'static, ()>,
}

impl AuthEnv {
    fn new(path: &Path, base: Option<&str>) -> Self {
        let lock = AUTH_ENV_LOCK.lock().expect("auth env lock");
        std::env::set_var("FLOWFLOW_CHATGPT_AUTH_PATH", path);
        if let Some(base) = base {
            std::env::set_var("FLOWFLOW_CHATGPT_AUTH_BASE", base);
        } else {
            std::env::remove_var("FLOWFLOW_CHATGPT_AUTH_BASE");
        }
        Self { _lock: lock }
    }
}

impl Drop for AuthEnv {
    fn drop(&mut self) {
        chatgpt_auth::disconnect();
        std::env::remove_var("FLOWFLOW_CHATGPT_AUTH_PATH");
        std::env::remove_var("FLOWFLOW_CHATGPT_AUTH_BASE");
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_secs() as i64
}

fn fake_jwt(exp: i64, account_id: &str) -> String {
    let payload = json!({
        "exp": exp,
        "https://api.openai.com/auth": {
            "chatgpt_account_id": account_id,
        },
    });
    let encoded = BASE64_URL_SAFE_NO_PAD.encode(payload.to_string());
    format!("header.{encoded}.signature")
}

fn write_record(
    path: &Path,
    access_token: &str,
    refresh_token: Option<&str>,
    expires_at: i64,
    account_id: Option<&str>,
) {
    std::fs::write(
        path,
        serde_json::to_vec(&json!({
            "access_token": access_token,
            "refresh_token": refresh_token,
            "id_token": null,
            "expires_at": expires_at,
            "account_id": account_id,
        }))
        .expect("serialize token record"),
    )
    .expect("write token record");
}

struct MockResponse {
    status: u16,
    body: String,
}

fn spawn_mock(
    responses: Vec<MockResponse>,
) -> (String, Arc<Mutex<Vec<String>>>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    listener
        .set_nonblocking(true)
        .expect("set mock server nonblocking");
    let address = listener.local_addr().expect("mock address");
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = requests.clone();
    let handle = thread::spawn(move || {
        for response in responses {
            let deadline = Instant::now() + Duration::from_secs(10);
            let (mut stream, _) = loop {
                match listener.accept() {
                    Ok(connection) => break connection,
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {
                        assert!(
                            Instant::now() < deadline,
                            "mock request timed out"
                        );
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("mock accept failed: {error}"),
                }
            };
            let request = read_request(&mut stream);
            captured.lock().expect("capture request").push(request);
            write_response(&mut stream, response);
        }
    });
    (format!("http://{address}"), requests, handle)
}

fn read_request(stream: &mut TcpStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("request timeout");
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    let mut expected_len = None;

    loop {
        let count = stream.read(&mut buffer).expect("read request");
        assert!(count > 0, "request closed before body completed");
        bytes.extend_from_slice(&buffer[..count]);

        if expected_len.is_none() {
            if let Some(header_end) =
                bytes.windows(4).position(|part| part == b"\r\n\r\n")
            {
                let headers = String::from_utf8_lossy(&bytes[..header_end]);
                let content_len = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length").then(|| {
                            value
                                .trim()
                                .parse::<usize>()
                                .expect("content length")
                        })
                    })
                    .unwrap_or_default();
                expected_len = Some(header_end + 4 + content_len);
            }
        }

        if expected_len.is_some_and(|length| bytes.len() >= length) {
            break;
        }
    }

    String::from_utf8(bytes).expect("utf8 request")
}

fn write_response(stream: &mut TcpStream, response: MockResponse) {
    let reason = match response.status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        _ => "Error",
    };
    let reply = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        response.status,
        reason,
        response.body.len(),
        response.body
    );
    stream.write_all(reply.as_bytes()).expect("write response");
}

#[test]
fn jwt_helpers_extract_expiry_and_account() {
    let expiry = unix_now() + 600;
    let token = fake_jwt(expiry, "acc_1");

    assert_eq!(chatgpt_auth::extract_exp(&token), Some(expiry));
    assert_eq!(
        chatgpt_auth::extract_account_id(Some(&token)).as_deref(),
        Some("acc_1")
    );
    assert!(chatgpt_auth::decode_jwt_claims("invalid").is_null());
}

#[tokio::test(flavor = "current_thread")]
async fn token_store_round_trips_and_disconnects() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("auth.json");
    let _env = AuthEnv::new(&path, None);
    let expiry = unix_now() + 600;
    let token = fake_jwt(expiry, "acc_1");
    write_record(&path, &token, None, expiry, None);

    assert!(chatgpt_auth::is_connected());
    let (loaded_token, account_id) = chatgpt_auth::access_context()
        .await
        .expect("load access context");
    assert_eq!(loaded_token, token);
    assert_eq!(account_id.as_deref(), Some("acc_1"));

    let persisted: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&path).expect("read persisted record"),
    )
    .expect("parse persisted record");
    assert_eq!(persisted["account_id"], "acc_1");

    chatgpt_auth::disconnect();
    assert!(!path.exists());
    assert!(!chatgpt_auth::is_connected());
}

#[tokio::test(flavor = "current_thread")]
async fn device_flow_polls_then_refreshes_expired_token() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("auth.json");
    let expired = fake_jwt(unix_now() - 60, "acc_1");
    let refreshed = fake_jwt(unix_now() + 600, "acc_1");
    let (base, requests, server) = spawn_mock(vec![
        MockResponse {
            status: 200,
            body: json!({
                "device_auth_id": "device_1",
                "user_code": "ABCD-EFGH",
                "interval": 0,
            })
            .to_string(),
        },
        MockResponse {
            status: 403,
            body: "{}".into(),
        },
        MockResponse {
            status: 200,
            body: json!({
                "authorization_code": "authorization_1",
                "code_verifier": "verifier_1",
            })
            .to_string(),
        },
        MockResponse {
            status: 200,
            body: json!({
                "access_token": expired,
                "refresh_token": "refresh_1",
                "id_token": null,
            })
            .to_string(),
        },
        MockResponse {
            status: 200,
            body: json!({
                "access_token": refreshed,
                "refresh_token": "refresh_2",
                "id_token": null,
            })
            .to_string(),
        },
    ]);
    let _env = AuthEnv::new(&path, Some(&base));

    let login = chatgpt_auth::begin_device_login()
        .await
        .expect("begin device login");
    assert_eq!(login.user_code, "ABCD-EFGH");
    assert_eq!(login.verify_url, format!("{base}/codex/device"));
    chatgpt_auth::poll_device_login(&login)
        .await
        .expect("poll device login");
    assert!(chatgpt_auth::is_connected());

    let (access_token, account_id) = chatgpt_auth::access_context()
        .await
        .expect("refresh access context");
    println!("refreshed token round-trip: {access_token}");
    assert_eq!(access_token, refreshed);
    assert_eq!(account_id.as_deref(), Some("acc_1"));

    server.join().expect("mock server");
    let requests = requests.lock().expect("captured requests");
    assert_eq!(requests.len(), 5);
    assert!(requests[4].contains("grant_type=refresh_token"));
    assert!(requests[4].contains("refresh_token=refresh_1"));
}

#[tokio::test(flavor = "current_thread")]
async fn future_token_does_not_call_refresh_endpoint() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("auth.json");
    let listener =
        TcpListener::bind("127.0.0.1:0").expect("bind unused server");
    let base = format!("http://{}", listener.local_addr().expect("address"));
    drop(listener);
    let _env = AuthEnv::new(&path, Some(&base));
    let expiry = unix_now() + 600;
    let token = fake_jwt(expiry, "acc_1");
    write_record(&path, &token, Some("refresh_1"), expiry, Some("acc_1"));

    let (access_token, _) = chatgpt_auth::access_context()
        .await
        .expect("reuse future token");
    assert_eq!(access_token, token);
}

#[tokio::test(flavor = "current_thread")]
async fn invalid_grant_deletes_expired_record() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("auth.json");
    let (base, requests, server) = spawn_mock(vec![MockResponse {
        status: 400,
        body: json!({ "error": "invalid_grant" }).to_string(),
    }]);
    let _env = AuthEnv::new(&path, Some(&base));
    let expiry = unix_now() - 60;
    let token = fake_jwt(expiry, "acc_1");
    write_record(
        &path,
        &token,
        Some("expired_refresh"),
        expiry,
        Some("acc_1"),
    );

    let error = chatgpt_auth::access_context()
        .await
        .expect_err("invalid grant must require login");
    assert!(error.contains("not connected"));
    assert!(!path.exists());
    assert!(!chatgpt_auth::is_connected());

    server.join().expect("mock server");
    let requests = requests.lock().expect("captured requests");
    assert!(requests[0].contains("grant_type=refresh_token"));
}
