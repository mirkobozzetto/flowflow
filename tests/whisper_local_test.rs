use flowflow::services::transcription::WhisperLocal;
use std::path::{Path, PathBuf};

#[tokio::test]
async fn whisper_local_transcribes_real_wav_when_fixture_present() {
    let Some(model) = std::env::var_os("FLOWFLOW_WHISPER_MODEL") else {
        eprintln!(
            "[whisper test] FLOWFLOW_WHISPER_MODEL not set, skipping \
             real-model test"
        );
        return;
    };
    let Some(wav) = std::env::var_os("FLOWFLOW_WHISPER_WAV") else {
        eprintln!(
            "[whisper test] FLOWFLOW_WHISPER_WAV not set, skipping \
             real-model test"
        );
        return;
    };
    let whisper = WhisperLocal::new(PathBuf::from(model));
    let start = std::time::Instant::now();
    let text = whisper
        .transcribe(Path::new(&wav), Some("fr"))
        .await
        .expect("transcribe");
    eprintln!(
        "[whisper test] {} ms, transcript: {text}",
        start.elapsed().as_millis()
    );
    assert!(!text.is_empty(), "transcript must not be empty");
}
