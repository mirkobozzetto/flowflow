use super::background;
use super::job::{
    cleanup_file, front_job, local_job_running, set_status,
    set_transcription_ids, Job, JobStatus, Registry,
};
use crate::infrastructure::persistence::Database;
use crate::infrastructure::transcription::whisper::wav_duration_ms;
use crate::infrastructure::transcription::{
    Checkpoint, SonioxClient, SttProvider, TranscriptionClient, WhisperLocal,
};
use std::path::Path;
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
        SttProvider::WhisperLocal => {
            let done = process_local(reg, db, note_id, job).await;
            if !local_job_running(&reg.lock().unwrap().queues) {
                background::end(done);
            }
        }
    }
}

/// `true` when the transcript is done.
async fn process_local(
    reg: &Mutex<Registry>,
    db: &Database,
    note_id: &str,
    job: Job,
) -> bool {
    let whisper: WhisperLocal = match TranscriptionClient::whisper_from_db(db) {
        Ok(w) => w,
        Err(e) => {
            set_status(reg, note_id, &job.id, JobStatus::Failed(e));
            return false;
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
        return false;
    }
    let from = resume_point(db, note_id, &path);
    let _ = db.add_pending_local_transcription(
        note_id,
        &path.to_string_lossy(),
        job.audio_id.as_deref(),
    );
    let total_ms = wav_duration_ms(&path).unwrap_or(0);
    let mut done_ms = from.done_ms;
    background::progress(
        db,
        note_id,
        done_ms,
        total_ms,
        percent(done_ms, total_ms).unwrap_or(0),
    );
    set_status(
        reg,
        note_id,
        &job.id,
        JobStatus::Polling {
            elapsed_s: 0,
            percent: percent(done_ms, total_ms),
        },
    );
    let started = SystemTime::now();
    let (progress_tx, mut progress_rx) =
        tokio::sync::mpsc::unbounded_channel::<Checkpoint>();
    let fut = whisper.transcribe_from(&path, None, from, move |progress| {
        let _ = progress_tx.send(progress.clone());
    });
    tokio::pin!(fut);
    loop {
        tokio::select! {
            Some(progress) = progress_rx.recv() => {
                let _ = db.save_local_progress(
                    note_id,
                    progress.done_ms,
                    &progress.words,
                );
                done_ms = progress.done_ms;
                background::progress(
                    db,
                    note_id,
                    done_ms,
                    total_ms,
                    percent(done_ms, total_ms).unwrap_or(0),
                );
            }
            res = &mut fut => {
                let _ = db.delete_pending_transcription(note_id);
                let done = res.is_ok();
                match res {
                    Ok(text) => {
                        // A clip with a `note_audios` row is the user's kept
                        // recording: deleting it would break the player and
                        // the retranscribe button.
                        if job.audio_id.is_none() {
                            cleanup_file(&path);
                        }
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
                return done;
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
                    JobStatus::Polling {
                        elapsed_s: elapsed,
                        percent: percent(done_ms, total_ms),
                    },
                );
            }
        }
    }
}

fn percent(done_ms: u32, total_ms: u32) -> Option<u8> {
    (total_ms > 0).then(|| {
        (u64::from(done_ms.min(total_ms)) * 100 / u64::from(total_ms)) as u8
    })
}

/// The recording is matched by file name: the app container path changes
/// across installs, the name does not.
fn resume_point(db: &Database, note_id: &str, path: &Path) -> Checkpoint {
    db.local_progress(note_id)
        .filter(|p| {
            p.file_path
                .as_deref()
                .map(Path::new)
                .and_then(Path::file_name)
                == path.file_name()
        })
        .map(|p| Checkpoint {
            done_ms: p.done_ms,
            words: p.words,
        })
        .unwrap_or_default()
}

async fn process_soniox(
    reg: &Mutex<Registry>,
    db: &Database,
    note_id: &str,
    job: Job,
) {
    let client = match SonioxClient::from_db(db) {
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
                let _ = db.add_pending_transcription(
                    note_id,
                    &tid,
                    Some(&fid),
                    job.audio_id.as_deref(),
                );
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
            Ok(Some(transcript)) => {
                if let Some(fid) = &file_id {
                    let _ = client.delete_file(fid).await;
                }
                let _ = db.delete_pending_transcription(note_id);
                if job.audio_id.is_none() {
                    cleanup_file(&job.file_path);
                }
                // This path drives the raw Soniox calls to stay resumable, so it
                // does not inherit the post-processing done inside `transcribe`.
                let clean = client.clean(transcript);
                set_status(reg, note_id, &job.id, JobStatus::Done(clean));
                return;
            }
            Ok(None) => {
                transient = 0;
                set_status(
                    reg,
                    note_id,
                    &job.id,
                    JobStatus::Polling {
                        elapsed_s: elapsed,
                        percent: None,
                    },
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
                    JobStatus::Polling {
                        elapsed_s: elapsed,
                        percent: None,
                    },
                );
                tokio::time::sleep(POLL_INTERVAL).await;
            }
        }
    }
}
