use crate::domain::Dictionary;
use crate::infrastructure::persistence::settings_repo::TRANSCRIPTION_DICTIONARY_KEY;
use crate::infrastructure::persistence::Database;

pub fn load(db: &Database) -> Dictionary {
    db.get_setting(TRANSCRIPTION_DICTIONARY_KEY)
        .map(|raw| Dictionary::parse(&raw))
        .unwrap_or_default()
}

pub fn save(db: &Database, dict: &Dictionary) -> Result<(), String> {
    db.set_setting(TRANSCRIPTION_DICTIONARY_KEY, &dict.serialize())
}
