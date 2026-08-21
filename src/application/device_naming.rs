use crate::infrastructure::persistence::settings_repo::DEVICE_NAME_KEY;
use crate::infrastructure::persistence::Database;
use std::hash::{Hash, Hasher};

// English on purpose: the name travels to peers whatever their app
// language, and hyphenated english combos stay short and readable.
const ADJECTIVES: [&str; 32] = [
    "amber", "bold", "brave", "bright", "calm", "clever", "cosmic", "crisp",
    "eager", "fleet", "gentle", "golden", "happy", "keen", "lively", "lucky",
    "mellow", "misty", "noble", "quick", "quiet", "rapid", "rustic", "silent",
    "smooth", "solar", "steady", "sunny", "swift", "vivid", "wild", "witty",
];
const ANIMALS: [&str; 32] = [
    "badger", "bison", "crane", "deer", "dolphin", "eagle", "falcon", "fox",
    "gazelle", "hawk", "heron", "ibis", "koala", "lemur", "lynx", "marmot",
    "otter", "owl", "panda", "puffin", "rabbit", "raven", "robin", "salmon",
    "seal", "sparrow", "swan", "tiger", "walrus", "weasel", "wolf", "wren",
];

/// The device's display name, never empty: the stored setting, or a
/// docker-style name derived from the sync device id ("quiet-otter-83"),
/// generated once and persisted so it survives hasher changes. The user's
/// own edit always wins.
pub fn ensure_device_name(db: &Database) -> String {
    if let Some(name) = db.get_setting(DEVICE_NAME_KEY) {
        if !name.is_empty() {
            return name;
        }
    }
    let seed = db.get_setting("sync_device_id").unwrap_or_default();
    let name = generate(&seed);
    let _ = db.set_setting(DEVICE_NAME_KEY, &name);
    name
}

fn generate(seed: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    seed.hash(&mut hasher);
    let h = hasher.finish();
    let adjective = ADJECTIVES[(h % 32) as usize];
    let animal = ANIMALS[((h >> 8) % 32) as usize];
    let digits = (h >> 16) % 100;
    format!("{adjective}-{animal}-{digits:02}")
}
