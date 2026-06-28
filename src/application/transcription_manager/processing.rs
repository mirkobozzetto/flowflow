use super::job::{
    cleanup_file, front_job, set_status, set_transcription_ids, Job, JobStatus,
    Registry,
};
use crate::infrastructure::persistence::Database;
use crate::infrastructure::transcription::{
    clean_hesitations, SonioxClient, SttProvider, TranscriptionClient,
    WhisperLocal,
};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

const MAX_ELAPSED_S: u32 = 5 * 60 * 60;
const POLL_INTERVAL: Duration = Duration::from_secs(2);

pub(super) async fn process_front(
    reg: &Mutex<Registry>,
    db: &Database,
    note_id: &str,
) {
    let job = match front_job(reg, note_id) {
        Some(j) => j,
        None => return,
    };
    if matches!(job.status, JobStatus::Done(_) | JobStatus::Failed(_)) {
        return;
    }
    match job.provider {
        SttProvider::Soniox => process_soniox(reg, db, note_id, job).await,
        SttProvider::WhisperLocal => process_local(reg, db, note_id, job).await,
    }
}

async fn process_local(
    reg: &Mutex<Registry>,
    db: &Database,
    note_id: &str,
    job: Job,
) {
    let whisper: WhisperLocal = match TranscriptionClient::whisper_from_db(db) {
        Ok(w) => w,
        Err(e) => {
            set_status(reg, note_id, &job.id, JobStatus::Failed(e));
            return;
        }
    };
    let path = job.file_path.clone();
    if path.as_os_str().is_empty() || !path.is_file() {
        let _ = db.delete_pending_transcription(note_id);
        set_status(
            reg,
            note_id,
            &job.id,
            JobStatus::Failed(crate::application::i18n::t(
                &crate::application::i18n::ui_lang(db),
                "stt-error-file-missing",
            )),
        );
        return;
    }
    let _ =
        db.add_pending_local_transcription(note_id, &path.to_string_lossy());
    set_status(reg, note_id, &job.id, JobStatus::Polling { elapsed_s: 0 });
    let started = SystemTime::now();
    let fut = whisper.transcribe(&path, None);
    tokio::pin!(fut);
    loop {
        tokio::select! {
            res = &mut fut => {
                let _ = db.delete_pending_transcription(note_id);
                match res {
                    Ok(text) => {
                        cleanup_file(&path);
                        set_status(
                            reg,
                            note_id,
                            &job.id,
                            JobStatus::Done(text),
                        );
                    }
                    Err(e) => {
                        set_status(
                            reg,
                            note_id,
                            &job.id,
                            JobStatus::Failed(e),
                        );
                    }
                }
                return;
            }
            _ = tokio::time::sleep(POLL_INTERVAL) => {
                let elapsed = started
                    .elapsed()
                    .map(|d| d.as_secs() as u32)
                    .unwrap_or(0);
                set_status(
                    reg,
                    note_id,
                    &job.id,
                    JobStatus::Polling { elapsed_s: elapsed },
                );
            }
        }
    }
}

async fn process_soniox(
    reg: &Mutex<Registry>,
    db: &Database,
    note_id: &str,
    job: Job,
) {
    let client = match client_from_db(db) {
        Ok(c) => c,
        Err(e) => {
            set_status(reg, note_id, &job.id, JobStatus::Failed(e));
            return;
        }
    };

    set_status(reg, note_id, &job.id, JobStatus::Uploading);

    let (tr_id, file_id) = match job.transcription_id.clone() {
        Some(tid) => (tid, job.soniox_file_id.clone()),
        None => match client.start_transcription(&job.file_path, None).await {
            Ok((tid, fid)) => {
                let _ = db.add_pending_transcription(note_id, &tid, Some(&fid));
                set_transcription_ids(reg, note_id, &job.id, &tid, &fid);
                (tid, Some(fid))
            }
            Err(e) => {
                set_status(reg, note_id, &job.id, JobStatus::Failed(e));
                return;
            }
        },
    };

    let started = SystemTime::now();
    let mut transient = 0u32;
    loop {
        let elapsed =
            started.elapsed().map(|d| d.as_secs() as u32).unwrap_or(0);
        if elapsed > MAX_ELAPSED_S {
            if let Some(fid) = &file_id {
                let _ = client.delete_file(fid).await;
            }
            let _ = db.delete_pending_transcription(note_id);
            set_status(
                reg,
                note_id,
                &job.id,
                JobStatus::Failed("Transcription timeout (5 h)".to_string()),
            );
            return;
        }
        match client.check_status(&tr_id).await {
            Ok(Some(text)) => {
                if let Some(fid) = &file_id {
                    let _ = client.delete_file(fid).await;
                }
                let _ = db.delete_pending_transcription(note_id);
                cleanup_file(&job.file_path);
                let clean = clean_hesitations(&text);
                set_status(reg, note_id, &job.id, JobStatus::Done(clean));
                return;
            }
            Ok(None) => {
                transient = 0;
                set_status(
                    reg,
                    note_id,
                    &job.id,
                    JobStatus::Polling { elapsed_s: elapsed },
                );
                tokio::time::sleep(POLL_INTERVAL).await;
            }
            Err(e) => {
                let server_failed = e == "Transcription failed on server";
                transient += 1;
                if server_failed || transient >= 10 {
                    if let Some(fid) = &file_id {
                        let _ = client.delete_file(fid).await;
                    }
                    let _ = db.delete_pending_transcription(note_id);
                    set_status(reg, note_id, &job.id, JobStatus::Failed(e));
                    return;
                }
                eprintln!("[soniox] poll transient {transient}/10: {e}");
                set_status(
                    reg,
                    note_id,
                    &job.id,
                    JobStatus::Polling { elapsed_s: elapsed },
                );
                tokio::time::sleep(POLL_INTERVAL).await;
            }
        }
    }
}

fn client_from_db(db: &Database) -> Result<SonioxClient, String> {
    let lang = crate::application::i18n::ui_lang(db);
    if db.get_setting("ai_consent") != Some("true".to_string()) {
        return Err(crate::application::i18n::t(&lang, "error-ai-consent"));
    }
    let key = db
        .get_setting("soniox_api_key")
        .or_else(|| std::env::var("SONIOX_API_KEY").ok())
        .or_else(|| option_env!("SONIOX_API_KEY").map(String::from))
        .unwrap_or_default();
    if key.is_empty() || key == "your_key_here" {
        return Err(crate::application::i18n::t(&lang, "stt-error-soniox-key"));
    }
    Ok(SonioxClient::new(key).with_lang(lang))
}
