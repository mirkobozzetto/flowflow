use flowflow::application::constants::WhisperCatalogEntry;
use flowflow::infrastructure::transcription::models::fetch_part;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

// The first response promises the whole body and closes halfway, like iOS
// killing a suspended app's socket: the client must come back with a Range
// request and append the rest instead of starting over or failing.
#[tokio::test]
async fn cut_stream_resumes_with_range_and_completes() {
    let body: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
    let half = body.len() / 2;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let ranges = Arc::new(Mutex::new(Vec::new()));
    let (served, seen) = (body.clone(), ranges.clone());
    std::thread::spawn(move || {
        for (n, stream) in listener.incoming().enumerate() {
            let mut s = stream.unwrap();
            let mut buf = [0u8; 4096];
            let len = s.read(&mut buf).unwrap();
            let req = String::from_utf8_lossy(&buf[..len]).to_lowercase();
            let range = req.lines().find_map(|l| {
                l.strip_prefix("range: bytes=").map(|r| {
                    r.trim().trim_end_matches('-').parse::<usize>().unwrap()
                })
            });
            seen.lock().unwrap().push(range);
            let total = served.len();
            match (n, range) {
                (0, None) => {
                    write!(
                        s,
                        "HTTP/1.1 200 OK\r\nContent-Length: {total}\r\nConnection: close\r\n\r\n"
                    )
                    .unwrap();
                    s.write_all(&served[..half]).unwrap();
                }
                (_, Some(from)) => {
                    write!(
                        s,
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {from}-{}/{total}\r\nConnection: close\r\n\r\n",
                        total - from,
                        total - 1
                    )
                    .unwrap();
                    s.write_all(&served[from..]).unwrap();
                }
                _ => panic!("unexpected request #{n} with range {range:?}"),
            }
        }
    });

    let url: &'static str =
        Box::leak(format!("http://127.0.0.1:{port}/m.bin").into_boxed_str());
    let entry = WhisperCatalogEntry {
        id: "test",
        filename: "m.bin",
        url,
        bytes: body.len() as u64,
        sha256: "",
        quality_key: "",
    };
    let dir = tempfile::tempdir().unwrap();
    let part = dir.path().join("m.bin.part");

    fetch_part(&entry, &part, &mut |_, _| {}).await.unwrap();

    assert_eq!(std::fs::read(&part).unwrap(), body);
    assert_eq!(*ranges.lock().unwrap(), vec![None, Some(half)]);
}
