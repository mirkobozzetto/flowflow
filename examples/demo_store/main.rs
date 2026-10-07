// Demo data for the App Store captures, no personal data anywhere:
//   cargo run --example demo_store -- /tmp/flowflow-demo-en.db en   (or fr)
//   cargo run --example demo_store -- hermes en                     (or fr)
// The first seeds a store; the second serves a fake Hermes the store points
// at, so no real Hermes address, key, skill or history reaches a frame.
mod hermes;
mod seed;

// The simulator shares the Mac's network: the app reaches this directly.
const HERMES_URL: &str = "http://127.0.0.1:8765";
const HERMES_KEY: &str = "demo";
// The sessions the store lists and the fake Hermes serves, newest last.
const SESSIONS: [&str; 2] = ["flowflow_demo_lisbon", "flowflow_demo_launch"];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let usage = "usage: demo_store <store.db> en|fr | demo_store hermes en|fr";
    let (first, lang) = match (args.get(1), args.get(2)) {
        (Some(first), Some(lang)) if lang == "en" || lang == "fr" => {
            (first.as_str(), lang.as_str())
        }
        _ => panic!("{usage}"),
    };
    if first == "hermes" {
        hermes::serve(lang);
    } else {
        seed::run(first.into(), lang);
    }
}
