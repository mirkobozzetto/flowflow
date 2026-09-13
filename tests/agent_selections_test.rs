use flowflow::application::agent_selections::list_sheet_resources;
use flowflow::infrastructure::persistence::installed_connector_repo::PinnedConnector;
use flowflow::infrastructure::persistence::Database;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Clone, Default)]
struct Fixture {
    requests: Arc<Mutex<Vec<(String, String)>>>,
}

async fn serve(listener: tokio::net::TcpListener, fixture: Fixture) {
    loop {
        let Ok((mut stream, _)) = listener.accept().await else {
            break;
        };
        let fixture = fixture.clone();
        tokio::spawn(async move {
            let mut bytes = Vec::new();
            let mut buf = [0u8; 8192];
            let (header_end, length) = loop {
                let n = stream.read(&mut buf).await.unwrap();
                if n == 0 {
                    return;
                }
                bytes.extend_from_slice(&buf[..n]);
                if let Some(end) =
                    bytes.windows(4).position(|window| window == b"\r\n\r\n")
                {
                    let headers = String::from_utf8_lossy(&bytes[..end]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|value| value.trim().parse().unwrap())
                        })
                        .unwrap_or(0);
                    break (end + 4, length);
                }
            };
            while bytes.len() < header_end + length {
                let n = stream.read(&mut buf).await.unwrap();
                if n == 0 {
                    return;
                }
                bytes.extend_from_slice(&buf[..n]);
            }
            let head = String::from_utf8_lossy(&bytes[..header_end]);
            let mut request = head.lines().next().unwrap().split_whitespace();
            let method = request.next().unwrap().to_string();
            let path = request.next().unwrap().to_string();
            fixture
                .requests
                .lock()
                .unwrap()
                .push((method.clone(), path.clone()));
            assert_eq!(path, "/v1/chat/connectors/google/mcp");
            if method == "GET" {
                stream.write_all(b"HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
                return;
            }
            assert_eq!(method, "POST");
            assert!(head.lines().any(|line| line
                .eq_ignore_ascii_case("authorization: Bearer local-session")));
            let body: Value =
                serde_json::from_slice(&bytes[header_end..header_end + length])
                    .unwrap();
            let rpc_method = body["method"].as_str().unwrap_or("");
            let (status, session, response) = match rpc_method {
                "initialize" => (
                    "200 OK",
                    Some("fixture-session"),
                    json!({"jsonrpc":"2.0","id":body["id"],"result":{"protocolVersion":"2025-03-26","capabilities":{"tools":{}},"serverInfo":{"name":"fixture","version":"1"}}}).to_string(),
                ),
                "notifications/initialized" => ("202 Accepted", None, String::new()),
                "tools/list" => (
                    "200 OK",
                    Some("fixture-session"),
                    json!({"jsonrpc":"2.0","id":body["id"],"result":{"tools":[{"name":"google_sheets_list_spreadsheets","description":"list","inputSchema":{"type":"object","properties":{}}}]}}).to_string(),
                ),
                "tools/call" => (
                    "200 OK",
                    Some("fixture-session"),
                    json!({"jsonrpc":"2.0","id":body["id"],"result":{"content":[{"type":"text","text":"{\"spreadsheets\":[{\"id\":\"sheet-a\",\"name\":\"Budget\"},{\"id\":\"sheet-b\",\"name\":\"Planning\"}]}"}],"isError":false}}).to_string(),
                ),
                other => panic!("unexpected MCP method {other}"),
            };
            let mut headers = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
                response.len()
            );
            if let Some(session) = session {
                headers.push_str(&format!("mcp-session-id: {session}\r\n"));
            }
            headers.push_str("\r\n");
            stream.write_all(headers.as_bytes()).await.unwrap();
            stream.write_all(response.as_bytes()).await.unwrap();
        });
    }
}

fn install_scoped_sheet_agent(db: &Database) {
    let manifest = json!({
        "schema_version":"2",
        "id":"agent-general-sheet",
        "version":"1.0.0",
        "name":"General sheet",
        "model":"test",
        "execution":{
            "required_connectors":[{"key":"table","type":"tabular_store","capabilities":["read","update","append"],"resource_required":true}],
            "native_tools":[],
            "governance":{"tools":[
                {"tool":"google_sheets_get_spreadsheet","mode":"read_only"},
                {"tool":"google_sheets_write_to_cell","mode":"read_write","approval":"require_approval"},
                {"tool":"google_sheets_append_rows","mode":"append_only","approval":"require_approval"}
            ],"read_before_write":true,"deny_destructive":true},
            "orchestration":{"chains":{}}
        }
    });
    let canonical = flowflow::domain::agent_manifest::canonical_json(&manifest);
    let digest = flowflow::domain::agent_manifest::digest_of(&manifest);
    db.conn().execute(
        "INSERT INTO installed_agents (id, version, content_digest, manifest_json, active) VALUES (?1, ?2, ?3, ?4, 1)",
        rusqlite::params!["agent-general-sheet", "1.0.0", digest, canonical],
    ).unwrap();
    let raw = json!({
        "connector":"google-sheets",
        "version":4,
        "type":"tabular_store",
        "server":"fixture",
        "mcp_prefix":"google_sheets_",
        "provides":["search","read","create","update","append","upsert"],
        "tools":[
            {"tool":"google_sheets_list_spreadsheets","resource":"spreadsheet","action":"search","risk":"read_only"},
            {"tool":"google_sheets_get_spreadsheet","resource":"spreadsheet","action":"read","risk":"read_only"},
            {"tool":"google_sheets_write_to_cell","resource":"cell","action":"update","risk":"read_write"},
            {"tool":"google_sheets_append_rows","resource":"row","action":"append","risk":"read_write"}
        ]
    }).to_string();
    db.pin_connector(&PinnedConnector {
        slug: "google".into(),
        connector_type: "tabular_store".into(),
        version: 4,
        content_digest: flowflow::domain::agent_manifest::digest_of_stored(
            &raw,
        )
        .unwrap(),
        manifest_json: raw,
    })
    .unwrap();
}

#[tokio::test]
async fn scoped_sheet_resources_are_discovered_without_the_legacy_crm_agent() {
    let temp = tempfile::tempdir().unwrap();
    let db = Database::open_at(temp.path().join("flowflow.db")).unwrap();
    let fixture = Fixture::default();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(serve(listener, fixture.clone()));
    db.set_setting("backend_base_url", &base).unwrap();
    db.set_setting("backend_session_token", "local-session")
        .unwrap();
    db.set_setting(
        "backend_session_expires_at",
        &(chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339(),
    )
    .unwrap();
    install_scoped_sheet_agent(&db);

    assert!(db.get_installed_agent("agent-crm").is_none());
    let resources =
        list_sheet_resources(&db, "agent-general-sheet", "table", "google")
            .await
            .unwrap();
    assert_eq!(resources.len(), 2);
    assert_eq!(resources[0].id, "sheet-a");
    assert_eq!(resources[0].name, "Budget");
    assert!(fixture.requests.lock().unwrap().iter().all(|(_, path)| {
        path == "/v1/chat/connectors/google/mcp" && !path.contains("/bindings/")
    }));
    server.abort();
}
