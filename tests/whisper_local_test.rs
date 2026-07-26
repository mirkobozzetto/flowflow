use flowflow::infrastructure::transcription::WhisperLocal;
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
    let transcript = whisper
        .transcribe(Path::new(&wav), Some("fr"))
        .await
        .expect("transcribe");
    let text = transcript.text();
    eprintln!(
        "[whisper test] {} ms, {} words, transcript: {text}",
        start.elapsed().as_millis(),
        transcript.len()
    );
    for word in transcript.words.iter().take(12) {
        eprintln!(
            "[whisper test]   {:>6}..{:<6} p={:.3} {:?}",
            word.start_ms, word.end_ms, word.confidence, word.text
        );
    }
    assert!(!text.is_empty(), "transcript must not be empty");
    // The stored text is derived from the words, so the two can never disagree.
    assert_eq!(text.split_whitespace().count(), transcript.len());
    assert!(
        transcript.has_timings(),
        "max_len(1) + split_on_word must yield per-word timings"
    );
    assert!(
        transcript.words.iter().all(|w| w.end_ms >= w.start_ms),
        "no word may end before it starts"
    );
}
