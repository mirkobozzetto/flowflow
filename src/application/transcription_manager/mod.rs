mod job;
mod processing;

pub use job::{Job, JobStatus};

use crate::infrastructure::persistence::pending_transcription_repo::PendingTranscription;

use job::{cleanup_file, Registry};
use processing::process_front;

use crate::domain::Transcript;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::transcription::{SttProvider, TranscriptionClient};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct TranscriptionManager {
    reg: Arc<Mutex<Registry>>,
    db: Arc<Database>,
}

impl TranscriptionManager {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            reg: Arc::new(Mutex::new(Registry::default())),
            db,
        }
    }

    pub fn snapshot(&self) -> HashMap<String, VecDeque<Job>> {
        self.reg.lock().unwrap().queues.clone()
    }

    pub fn enqueue(
        &self,
        note_id: String,
        file_path: PathBuf,
        audio_id: Option<String>,
    ) {
        if crate::application::backup::restore_lock_active() {
            eprintln!("[import] restore pending: transcription refused");
            return;
        }
        let job = Job {
            id: uuid::Uuid::new_v4().to_string(),
            note_id: note_id.clone(),
            file_path,
            status: JobStatus::Queued,
            provider: TranscriptionClient::provider_from_db(&self.db),
            transcription_id: None,
            soniox_file_id: None,
            audio_id,
        };
        {
            let mut g = self.reg.lock().unwrap();
            g.queues.entry(note_id.clone()).or_default().push_back(job);
        }
        self.kick(note_id);
    }

    /// The transcript of a finished job, with the clip it belongs to.
    ///
    /// Word timings are persisted by the caller whenever an `audio_id` comes
    /// back: that is the recording path, where the clip row exists. An import
    /// returns `None` there - it has no row to anchor timings to, so only the
    /// text survives.
    pub fn take_done(
        &self,
        note_id: &str,
    ) -> Option<(Transcript, Option<String>)> {
        let (transcript, audio_id) = {
            let mut g = self.reg.lock().unwrap();
            let q = g.queues.get_mut(note_id)?;
            let front = q.front()?;
            let transcript = match &front.status {
                JobStatus::Done(t) => t.clone(),
                _ => return None,
            };
            let audio_id = front.audio_id.clone();
            q.pop_front();
            if q.is_empty() {
                g.queues.remove(note_id);
            }
            (transcript, audio_id)
        };
        eprintln!(
            "[import] consumed note={note_id} words={}",
            transcript.len()
        );
        self.kick(note_id.to_string());
        Some((transcript, audio_id))
    }

    pub fn retry(&self, note_id: &str) {
        {
            let mut g = self.reg.lock().unwrap();
            if let Some(q) = g.queues.get_mut(note_id) {
                if let Some(j) = q.front_mut() {
                    if matches!(j.status, JobStatus::Failed(_)) {
                        j.status = JobStatus::Queued;
                        j.transcription_id = None;
                        j.soniox_file_id = None;
                    }
                }
            }
        }
        self.kick(note_id.to_string());
    }

    pub fn dismiss(&self, note_id: &str) {
        let path = {
            let mut g = self.reg.lock().unwrap();
            // A kept recording's clip is not the job's to delete: the audio
            // accordion still plays it and can retranscribe it.
            let path = g
                .queues
                .get(note_id)
                .and_then(|q| q.front())
                .filter(|j| j.audio_id.is_none())
                .map(|j| j.file_path.clone());
            if let Some(q) = g.queues.get_mut(note_id) {
                q.pop_front();
                if q.is_empty() {
                    g.queues.remove(note_id);
                }
            }
            path
        };
        if let Some(p) = path {
            cleanup_file(&p);
        }
        self.kick(note_id.to_string());
    }

    pub fn resume_pending(&self) {
        if crate::application::backup::restore_lock_active() {
            eprintln!("[import] restore pending: pending transcriptions held");
            return;
        }
        for row in self.db.list_pending_transcriptions() {
            let Some(job) = job_from_pending(&row) else {
                let _ = self.db.delete_pending_transcription(&row.note_id);
                continue;
            };
            {
                let mut g = self.reg.lock().unwrap();
                g.queues
                    .entry(row.note_id.clone())
                    .or_default()
                    .push_back(job);
            }
            self.kick(row.note_id);
        }
    }

    fn kick(&self, note_id: String) {
        if crate::application::backup::restore_lock_active() {
            return;
        }
        {
            let mut g = self.reg.lock().unwrap();
            if g.active.contains(&note_id) {
                return;
            }
            let actionable = matches!(
                g.queues
                    .get(&note_id)
                    .and_then(|q| q.front())
                    .map(|j| &j.status),
                Some(JobStatus::Queued)
                    | Some(JobStatus::Uploading)
                    | Some(JobStatus::Polling { .. })
            );
            if !actionable {
                return;
            }
            g.active.insert(note_id.clone());
        }
        let reg = self.reg.clone();
        let db = self.db.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                process_front(&reg, &db, &note_id).await;
                reg.lock().unwrap().active.remove(&note_id);
            });
        });
    }
}

/// The job a persisted pending row resumes into. Pure: no thread, no IO -
/// `resume_pending` is this plus a queue push. `None` means the row is unusable
/// (a soniox row with no server-side id) and must be dropped.
pub fn job_from_pending(row: &PendingTranscription) -> Option<Job> {
    if row.provider == SttProvider::WhisperLocal.as_str() {
        return Some(Job {
            id: uuid::Uuid::new_v4().to_string(),
            note_id: row.note_id.clone(),
            file_path: resolve_resume_path(row.file_path.as_deref()),
            status: JobStatus::Queued,
            provider: SttProvider::WhisperLocal,
            transcription_id: None,
            soniox_file_id: None,
            audio_id: row.audio_id.clone(),
        });
    }
    // Soniox holds the audio server-side; without its id there is nothing left
    // to poll.
    let tr_id = row.transcription_id.clone()?;
    Some(Job {
        id: uuid::Uuid::new_v4().to_string(),
        note_id: row.note_id.clone(),
        file_path: PathBuf::new(),
        status: JobStatus::Polling { elapsed_s: 0 },
        provider: SttProvider::Soniox,
        transcription_id: Some(tr_id),
        soniox_file_id: row.soniox_file_id.clone(),
        audio_id: row.audio_id.clone(),
    })
}

fn resolve_resume_path(stored: Option<&str>) -> PathBuf {
    let Some(stored) = stored else {
        return PathBuf::new();
    };
    let p = PathBuf::from(stored);
    if p.is_file() {
        return p;
    }
    if let Some(name) = p.file_name() {
        let fallback =
            PathBuf::from(crate::infrastructure::audio::output_dir())
                .join(name);
        if fallback.is_file() {
            return fallback;
        }
    }
    p
}
