mod append;
mod job;
mod processing;

pub use append::append_transcription_to_note;
pub use job::{Job, JobStatus};

use job::{cleanup_file, Registry};
use processing::process_front;

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

    pub fn enqueue(&self, note_id: String, file_path: PathBuf) {
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
        };
        {
            let mut g = self.reg.lock().unwrap();
            g.queues.entry(note_id.clone()).or_default().push_back(job);
        }
        self.kick(note_id);
    }

    pub fn take_done(&self, note_id: &str) -> Option<String> {
        let text = {
            let mut g = self.reg.lock().unwrap();
            let q = g.queues.get_mut(note_id)?;
            let text = match q.front().map(|j| j.status.clone()) {
                Some(JobStatus::Done(t)) => t,
                _ => return None,
            };
            q.pop_front();
            if q.is_empty() {
                g.queues.remove(note_id);
            }
            text
        };
        eprintln!("[import] consumed note={note_id} chars={}", text.len());
        self.kick(note_id.to_string());
        Some(text)
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
            let path = g
                .queues
                .get(note_id)
                .and_then(|q| q.front())
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
            let job = if row.provider == SttProvider::WhisperLocal.as_str() {
                Job {
                    id: uuid::Uuid::new_v4().to_string(),
                    note_id: row.note_id.clone(),
                    file_path: resolve_resume_path(row.file_path.as_deref()),
                    status: JobStatus::Queued,
                    provider: SttProvider::WhisperLocal,
                    transcription_id: None,
                    soniox_file_id: None,
                }
            } else {
                let Some(tr_id) = row.transcription_id else {
                    let _ = self.db.delete_pending_transcription(&row.note_id);
                    continue;
                };
                Job {
                    id: uuid::Uuid::new_v4().to_string(),
                    note_id: row.note_id.clone(),
                    file_path: PathBuf::new(),
                    status: JobStatus::Polling { elapsed_s: 0 },
                    provider: SttProvider::Soniox,
                    transcription_id: Some(tr_id),
                    soniox_file_id: row.soniox_file_id,
                }
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
