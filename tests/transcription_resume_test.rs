use flowflow::application::transcription_manager::{
    job_from_pending, JobStatus,
};
use flowflow::infrastructure::persistence::pending_transcription_repo::PendingTranscription;
use flowflow::infrastructure::transcription::SttProvider;

fn local_row(audio_id: Option<&str>) -> PendingTranscription {
    PendingTranscription {
        note_id: "note-1".into(),
        transcription_id: None,
        soniox_file_id: None,
        provider: SttProvider::WhisperLocal.as_str().to_string(),
        file_path: Some("/tmp/a.wav".into()),
        audio_id: audio_id.map(str::to_string),
    }
}

fn soniox_row(
    transcription_id: Option<&str>,
    audio_id: Option<&str>,
) -> PendingTranscription {
    PendingTranscription {
        note_id: "note-2".into(),
        transcription_id: transcription_id.map(str::to_string),
        soniox_file_id: Some("fid".into()),
        provider: SttProvider::Soniox.as_str().to_string(),
        file_path: None,
        audio_id: audio_id.map(str::to_string),
    }
}

#[test]
fn a_local_row_resumes_as_a_queued_whisper_job() {
    let job = job_from_pending(&local_row(Some("aud-1"))).expect("job");

    assert_eq!(job.provider, SttProvider::WhisperLocal);
    assert_eq!(job.status, JobStatus::Queued);
    assert_eq!(job.audio_id.as_deref(), Some("aud-1"));
    assert_eq!(job.file_path.to_string_lossy(), "/tmp/a.wav");
}

#[test]
fn a_soniox_row_resumes_as_a_polling_job_without_a_local_file() {
    let job = job_from_pending(&soniox_row(Some("tr-1"), Some("aud-2")))
        .expect("job");

    assert_eq!(job.provider, SttProvider::Soniox);
    assert_eq!(job.status, JobStatus::Polling { elapsed_s: 0 });
    assert_eq!(job.transcription_id.as_deref(), Some("tr-1"));
    assert_eq!(job.audio_id.as_deref(), Some("aud-2"));
    assert!(job.file_path.as_os_str().is_empty());
}

#[test]
fn a_soniox_row_without_a_server_id_is_unusable() {
    assert!(job_from_pending(&soniox_row(None, Some("aud-3"))).is_none());
}

#[test]
fn a_missing_audio_id_stays_missing() {
    assert!(job_from_pending(&local_row(None))
        .expect("job")
        .audio_id
        .is_none());
    assert!(job_from_pending(&soniox_row(Some("tr-1"), None))
        .expect("job")
        .audio_id
        .is_none());
}

#[test]
fn each_resume_mints_a_fresh_job_id() {
    let row = local_row(Some("aud-1"));

    let first = job_from_pending(&row).expect("job");
    let second = job_from_pending(&row).expect("job");

    assert_ne!(first.id, second.id);
}
