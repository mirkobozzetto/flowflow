use crate::domain::Transcript;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::transcription::TranscriptionClient;
use std::path::Path;

/// Save a Soniox key only once the service has confirmed it works.
///
/// Writing first and reporting "saved" made every typo look like a success. The
/// key is only persisted on a positive answer, so what is stored is always a key
/// that authenticated at least once.
pub async fn save_soniox_key(db: &Database, key: &str) -> Result<(), String> {
    let key = key.trim();
    crate::infrastructure::transcription::client::verify_key(key).await?;
    db.set_setting("soniox_api_key", key)
}

/// Transcribe a file that has no row of its own yet.
///
/// The recording bar owns the state machine (generation counter, error display);
/// this owns picking the engine and running it, so the view stops constructing an
/// infrastructure client. `cleanup` deletes the file afterwards, which is what the
/// dictation path wants and the keep-the-clip path does not.
pub async fn transcribe_file(
    db: &Database,
    path: &Path,
    cleanup: bool,
) -> Result<Transcript, String> {
    let client = match TranscriptionClient::from_db(db) {
        Ok(client) => client,
        Err(e) => {
            if cleanup {
                let _ = std::fs::remove_file(path);
            }
            return Err(e);
        }
    };
    let result = client.transcribe(path, None).await;
    if cleanup {
        let _ = std::fs::remove_file(path);
    }
    result
}

/// Re-transcribe a stored clip and persist the result against that exact clip.
///
/// The audio id is known here, so the words never go through the "last audio of
/// this note" guess. Unlike `persist_last_transcription` this deliberately
/// overwrites an existing transcription: the user asked for it again.
pub async fn retranscribe_audio(
    db: &Database,
    audio_id: &str,
    path: &Path,
) -> Result<Transcript, String> {
    let transcript = transcribe_file(db, path, false).await?;
    db.set_audio_transcript(audio_id, &transcript)?;
    Ok(transcript)
}
