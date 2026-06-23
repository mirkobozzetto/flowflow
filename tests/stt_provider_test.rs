use flowflow::infrastructure::persistence::Database;
use flowflow::infrastructure::transcription::{
    SttProvider, TranscriptionClient,
};
use tempfile::tempdir;

#[test]
fn facade_dispatch_chain_from_default_to_ready_whisper() {
    let data_dir = tempdir().expect("data dir");
    std::env::set_var("FLOWFLOW_DATA_DIR", data_dir.path());
    let db_dir = tempdir().expect("db dir");
    let db = Database::open_at(db_dir.path().join("flowflow_test.db"))
        .expect("open_at");
    db.set_setting("language", "fr").unwrap();

    assert_eq!(
        TranscriptionClient::provider_from_db(&db),
        SttProvider::Soniox,
        "absent settings keys must default to Soniox"
    );

    db.set_setting("stt_provider", "whisper_local").unwrap();
    assert_eq!(
        TranscriptionClient::provider_from_db(&db),
        SttProvider::WhisperLocal
    );

    let err = TranscriptionClient::from_db(&db).err().expect("no consent");
    assert!(err.contains("Consentement"), "got: {err}");

    db.set_setting("ai_consent", "true").unwrap();
    let err = TranscriptionClient::from_db(&db).err().expect("no model");
    assert!(err.contains("Aucun modèle"), "got: {err}");

    db.set_setting("whisper_model", "tiny").unwrap();
    let err = TranscriptionClient::from_db(&db)
        .err()
        .expect("not downloaded");
    assert!(err.contains("non téléchargé"), "got: {err}");

    let models_dir = data_dir.path().join("models/whisper");
    std::fs::create_dir_all(&models_dir).unwrap();
    std::fs::write(models_dir.join("ggml-tiny.bin"), b"fake").unwrap();
    let client = TranscriptionClient::from_db(&db).expect("ready whisper");
    assert_eq!(client.provider(), SttProvider::WhisperLocal);

    db.set_setting("stt_provider", "soniox").unwrap();
    db.set_setting("soniox_api_key", "test-key").unwrap();
    let client = TranscriptionClient::from_db(&db).expect("soniox ready");
    assert_eq!(client.provider(), SttProvider::Soniox);
}
