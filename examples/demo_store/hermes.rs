// A fake Hermes API server: the endpoints the app calls, canned answers.
// Every question gets the whiteboard answer, streamed with its steps, so a
// preview can be recorded live.
use crate::{HERMES_URL, SESSIONS};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

// Pace of the streamed answer: fast enough for a 30 s preview, slow enough
// to read as typing.
const STEP_PAUSE: Duration = Duration::from_millis(900);
const WORD_PAUSE: Duration = Duration::from_millis(35);

struct Text {
    launch_q: &'static str,
    photo: &'static str,
    vision_q: &'static str,
    todo_title: &'static str,
    launch_a: &'static str,
    plan_q: &'static str,
    note_title: &'static str,
    note_body: &'static str,
    memory: &'static str,
    plan_a: &'static str,
    lisbon_q: &'static str,
    lisbon_search: &'static str,
    lisbon_a: &'static str,
    /// (category, name, description, times used)
    skills: &'static [(&'static str, &'static str, &'static str, u32)],
}

const EN: Text = Text {
    launch_q: "What's left before the launch? Turn this board into a plan for my week.",
    photo: "whiteboard.jpg",
    vision_q: "What is written on the board?",
    todo_title: "Launch week",
    launch_a: "Here is your week, from the board:\n\n**Monday and Tuesday**\n1. Finish the **home page copy** and the **contact form**.\n\n**Wednesday**\n2. Gather the **client photos**.\n\n**Thursday**\n3. **Call Sarah**, and ask for **testimonials** on the same call.\n\n**Friday**\n4. The **pricing page**, circled as the priority.\n5. **Send the invoice** once the page is live.\n\nDomain and logo are done. Launch on **October 20**.",
    plan_q: "Anything missing compared to my launch plan?",
    note_title: "Launch plan for the October release",
    note_body: "Beta opens on the 6th for the waitlist. Press kit ready by Friday, landing page copy reviewed by Sarah. Keep the pricing announcement for the keynote, not before.",
    memory: "Launch on October 20, pricing announced at the keynote",
    plan_a: "Two things from your note are not on the board:\n\n- **Press kit**, also due Friday.\n- **Pricing**: your note keeps the announcement for the keynote. Build the pricing page by Friday, publish it after the keynote.\n\nBoth are now in this week's plan.",
    lisbon_q: "Plan a 3-day weekend in Lisbon, with a day trip to Sintra.",
    lisbon_search: "Sintra day trip from Lisbon by train",
    lisbon_a: "**Friday**: Alfama and the castle at sunset, dinner by the river.\n\n**Saturday**: early train to Sintra from Rossio, 40 minutes. Pena Palace first, before the crowds.\n\n**Sunday**: tram 28 in the morning, pastries in Belém, flight in the evening.",
    skills: &[
        ("productivity", "weekly-planner", "Turns notes and tasks into a plan for the week", 42),
        ("productivity", "meeting-recap", "Decisions and next steps from a meeting", 31),
        ("productivity", "inbox-zero", "Sorts the inbox and drafts the replies", 12),
        ("writing", "proofread", "Spelling, grammar and tone, nothing else", 25),
        ("writing", "launch-email", "The announcement email for a release", 18),
        ("writing", "linkedin-post", "A short post from a note", 9),
        ("finance", "invoice-builder", "An invoice from a client and a list of hours", 14),
        ("finance", "expense-sorter", "Receipts sorted by category", 6),
        ("research", "market-scan", "Who else does this, and at what price", 7),
        ("research", "competitor-watch", "What changed on the competitors' sites", 5),
        ("travel", "trip-planner", "A day by day plan for a trip", 11),
        ("health", "training-plan", "A running or training plan by week", 4),
    ],
};

const FR: Text = Text {
    launch_q: "Que reste-t-il avant le lancement ? Fais-moi un plan pour la semaine à partir de ce tableau.",
    photo: "tableau.jpg",
    vision_q: "Qu'est-ce qui est écrit sur le tableau ?",
    todo_title: "Semaine du lancement",
    launch_a: "Voici ta semaine, d'après le tableau :\n\n**Lundi et mardi**\n1. Finir le **texte de l'accueil** et le **formulaire de contact**.\n\n**Mercredi**\n2. Réunir les **photos du client**.\n\n**Jeudi**\n3. **Appeler Sarah**, et lui demander des **avis** pendant l'appel.\n\n**Vendredi**\n4. La **page des tarifs**, entourée comme priorité.\n5. **Envoyer la facture** une fois la page en ligne.\n\nLe domaine et le logo sont faits. Lancement le **20 octobre**.",
    plan_q: "Il manque quelque chose par rapport à mon plan de lancement ?",
    note_title: "Plan de lancement de la version d'octobre",
    note_body: "La bêta ouvre le 6 pour la liste d'attente. Dossier de presse prêt vendredi, textes de la page d'accueil relus par Sarah. Garder l'annonce des prix pour la keynote, pas avant.",
    memory: "Lancement le 20 octobre, prix annoncés à la keynote",
    plan_a: "Deux points de ta note ne sont pas sur le tableau :\n\n- **Le dossier de presse**, prévu lui aussi vendredi.\n- **Les prix** : ta note garde l'annonce pour la keynote. Prépare la page des tarifs pour vendredi, publie-la après la keynote.\n\nLes deux sont maintenant dans le plan de la semaine.",
    lisbon_q: "Prépare-moi un week-end de 3 jours à Lisbonne, avec une journée à Sintra.",
    lisbon_search: "Sintra en train depuis Lisbonne",
    lisbon_a: "**Vendredi** : l'Alfama et le château au coucher du soleil, dîner au bord du fleuve.\n\n**Samedi** : train tôt pour Sintra depuis Rossio, 40 minutes. Le palais de Pena d'abord, avant la foule.\n\n**Dimanche** : tram 28 le matin, pâtisseries à Belém, vol en soirée.",
    skills: &[
        ("productivité", "planning-semaine", "Transforme notes et tâches en plan pour la semaine", 42),
        ("productivité", "compte-rendu-reunion", "Décisions et prochaines étapes d'une réunion", 31),
        ("productivité", "boite-zero", "Trie la boîte mail et prépare les réponses", 12),
        ("écriture", "relecture", "Orthographe, grammaire et ton, rien d'autre", 25),
        ("écriture", "email-lancement", "L'email d'annonce d'une sortie", 18),
        ("écriture", "post-linkedin", "Un post court à partir d'une note", 9),
        ("finances", "facture-express", "Une facture à partir d'un client et d'heures", 14),
        ("finances", "tri-depenses", "Les tickets classés par catégorie", 6),
        ("recherche", "veille-marche", "Qui d'autre fait ça, et à quel prix", 7),
        ("recherche", "veille-concurrents", "Ce qui a changé chez les concurrents", 5),
        ("voyage", "itineraire-voyage", "Un voyage jour par jour", 11),
        ("santé", "plan-entrainement", "Un plan de course ou de sport par semaine", 4),
    ],
};

fn user(text: String) -> Value {
    json!({ "role": "user", "content": text })
}

fn tool(name: &str, key: &str, value: &str) -> Value {
    let arguments = json!({ key: value }).to_string();
    json!({ "role": "assistant", "content": "", "tool_calls": [
        { "function": { "name": name, "arguments": arguments } }
    ] })
}

fn answer(text: &str) -> Value {
    json!({ "role": "assistant", "content": text })
}

fn photo_tag(name: &str) -> String {
    format!("<attachment kind=\"photo\" name=\"{name}\"/>")
}

/// The scripted answer to any question: its steps, then its text.
fn reply(t: &Text) -> (Vec<(&'static str, &'static str)>, &'static str) {
    (
        vec![("vision_analyze", t.vision_q), ("todo_list", t.todo_title)],
        t.launch_a,
    )
}

fn sessions(t: &Text) -> HashMap<String, Vec<Value>> {
    let launch = vec![
        user(format!("{}\n\n{}", t.launch_q, photo_tag(t.photo))),
        tool("vision_analyze", "question", t.vision_q),
        tool("todo_list", "title", t.todo_title),
        answer(t.launch_a),
        user(format!(
            "{}\n\n<attachment kind=\"note\" name=\"{}\">\n{}\n</attachment>",
            t.plan_q, t.note_title, t.note_body
        )),
        tool("memory", "content", t.memory),
        answer(t.plan_a),
    ];
    let lisbon = vec![
        user(t.lisbon_q.to_string()),
        tool("web_search", "query", t.lisbon_search),
        answer(t.lisbon_a),
    ];
    HashMap::from([
        (SESSIONS[0].to_string(), lisbon),
        (SESSIONS[1].to_string(), launch),
    ])
}

fn skills(t: &Text) -> Value {
    let data: Vec<Value> = t
        .skills
        .iter()
        .map(|(category, name, description, usage)| {
            json!({ "name": name, "description": description,
                    "category": category, "usage": usage, "enabled": true })
        })
        .collect();
    json!({ "data": data })
}

fn model_options() -> Value {
    json!({
        "provider": "anthropic",
        "model": "claude-opus-5-5",
        "providers": [
            { "slug": "anthropic", "name": "Anthropic", "authenticated": true,
              "models": ["claude-opus-5-5", "claude-sonnet-5-5", "claude-haiku-4-5"] },
            { "slug": "openai-codex", "name": "OpenAI Codex", "authenticated": true,
              "models": ["gpt-5.5", "gpt-5.5-mini"] },
        ],
    })
}

struct State {
    text: &'static Text,
    sessions: Mutex<HashMap<String, Vec<Value>>>,
    runs: Mutex<usize>,
}

#[tokio::main]
pub async fn serve(lang: &str) {
    let text = if lang == "fr" { &FR } else { &EN };
    let state = Arc::new(State {
        text,
        sessions: Mutex::new(sessions(text)),
        runs: Mutex::new(0),
    });
    let addr = HERMES_URL.trim_start_matches("http://");
    let listener = TcpListener::bind(addr).await.expect("bind fake Hermes");
    println!("fake Hermes ({lang}) on {HERMES_URL}");
    loop {
        let Ok((sock, _)) = listener.accept().await else {
            continue;
        };
        tokio::spawn(handle(sock, state.clone()));
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

async fn read_request(
    sock: &mut TcpStream,
) -> Option<(String, String, Vec<u8>)> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 16384];
    let head_end = loop {
        let n = sock.read(&mut chunk).await.ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(i) = find(&buf, b"\r\n\r\n") {
            break i;
        }
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let len = head
        .lines()
        .find_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.eq_ignore_ascii_case("content-length")
                .then(|| v.trim().parse::<usize>().unwrap_or(0))
        })
        .unwrap_or(0);
    while buf.len() < head_end + 4 + len {
        let n = sock.read(&mut chunk).await.ok()?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let mut first = head.lines().next()?.split_whitespace();
    let method = first.next()?.to_string();
    let path = first.next()?.split('?').next()?.to_string();
    Some((method, path, buf[head_end + 4..].to_vec()))
}

async fn send_json(sock: &mut TcpStream, status: &str, body: Value) {
    let body = body.to_string();
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = sock.write_all(head.as_bytes()).await;
    let _ = sock.write_all(body.as_bytes()).await;
}

/// The question's text, from a plain input or a message with image parts.
fn question(body: &[u8]) -> String {
    let v: Value = serde_json::from_slice(body).unwrap_or_default();
    match &v["input"] {
        Value::String(s) => s.clone(),
        Value::Array(messages) => messages
            .iter()
            .flat_map(|m| m["content"].as_array().cloned().unwrap_or_default())
            .filter_map(|p| p["text"].as_str().map(String::from))
            .collect(),
        _ => String::new(),
    }
}

async fn handle(mut sock: TcpStream, state: Arc<State>) {
    let Some((method, path, body)) = read_request(&mut sock).await else {
        return;
    };
    let t = state.text;
    let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
    match (method.as_str(), parts.as_slice()) {
        ("GET", ["v1", "models"]) => {
            send_json(
                &mut sock,
                "200 OK",
                json!({ "data": [{ "id": "hermes" }] }),
            )
            .await
        }
        ("GET", ["v1", "skills"]) => {
            send_json(&mut sock, "200 OK", skills(t)).await
        }
        ("GET", ["api", "model", "options"]) => {
            send_json(&mut sock, "200 OK", model_options()).await
        }
        ("GET", ["api", "jobs"]) => {
            send_json(&mut sock, "200 OK", json!({ "jobs": [{}, {}] })).await
        }
        ("POST", ["api", "sessions"]) => {
            let v: Value = serde_json::from_slice(&body).unwrap_or_default();
            let id = v["id"].as_str().unwrap_or_default().to_string();
            state
                .sessions
                .lock()
                .unwrap()
                .entry(id.clone())
                .or_default();
            send_json(&mut sock, "200 OK", json!({ "id": id })).await
        }
        ("GET", ["api", "sessions", id, "messages"]) => {
            let data = state.sessions.lock().unwrap().get(*id).cloned();
            match data {
                Some(data) => {
                    send_json(&mut sock, "200 OK", json!({ "data": data }))
                        .await
                }
                None => send_json(&mut sock, "404 Not Found", json!({})).await,
            }
        }
        ("POST", ["v1", "runs"]) => {
            let v: Value = serde_json::from_slice(&body).unwrap_or_default();
            let session =
                v["session_id"].as_str().unwrap_or_default().to_string();
            let (steps, text) = reply(t);
            let mut turn = vec![user(question(&body))];
            turn.extend(steps.iter().map(|(name, detail)| {
                let key = if *name == "todo_list" {
                    "title"
                } else {
                    "question"
                };
                tool(name, key, detail)
            }));
            turn.push(answer(text));
            state
                .sessions
                .lock()
                .unwrap()
                .entry(session)
                .or_default()
                .extend(turn);
            let run = {
                let mut runs = state.runs.lock().unwrap();
                *runs += 1;
                format!("run_demo_{runs}")
            };
            send_json(&mut sock, "200 OK", json!({ "run_id": run })).await
        }
        ("GET", ["v1", "runs", _, "events"]) => {
            stream_reply(&mut sock, t).await
        }
        ("GET", ["v1", "runs", _]) => {
            let output = reply(t).1;
            send_json(
                &mut sock,
                "200 OK",
                json!({ "status": "completed", "output": output }),
            )
            .await
        }
        _ => send_json(&mut sock, "404 Not Found", json!({})).await,
    }
}

async fn stream_reply(sock: &mut TcpStream, t: &Text) {
    let head = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n";
    if sock.write_all(head.as_bytes()).await.is_err() {
        return;
    }
    let mut seq = 0;
    let mut events: Vec<(Value, Duration)> = Vec::new();
    let (steps, text) = reply(t);
    for (name, detail) in steps {
        events.push((
            json!({ "event": "tool.started", "tool": name, "preview": detail }),
            STEP_PAUSE,
        ));
        events.push((
            json!({ "event": "tool.completed", "tool": name }),
            Duration::ZERO,
        ));
    }
    for word in text.split_inclusive(' ') {
        events.push((
            json!({ "event": "message.delta", "delta": word }),
            WORD_PAUSE,
        ));
    }
    events.push((
        json!({ "event": "run.completed", "output": text }),
        Duration::ZERO,
    ));
    for (mut event, pause) in events {
        event["seq"] = json!(seq);
        seq += 1;
        let frame = format!("data: {event}\n\n");
        if sock.write_all(frame.as_bytes()).await.is_err() {
            return;
        }
        let _ = sock.flush().await;
        tokio::time::sleep(pause).await;
    }
}
