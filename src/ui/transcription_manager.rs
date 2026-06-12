use crate::db::Database;
use crate::models::UpdateNote;
use crate::services::transcription::{
    clean_hesitations, SonioxClient, SttProvider, TranscriptionClient,
    WhisperLocal,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

const MAX_ELAPSED_S: u32 = 5 * 60 * 60;
const POLL_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, PartialEq)]
pub enum JobStatus {
    Queued,
    Uploading,
    Polling { elapsed_s: u32 },
    Done(String),
    Failed(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Job {
    pub id: String,
    pub note_id: String,
    pub file_path: PathBuf,
    pub status: JobStatus,
    pub provider: SttProvider,
    pub transcription_id: Option<String>,
    pub soniox_file_id: Option<String>,
}

#[derive(Default)]
struct Registry {
    queues: HashMap<String, VecDeque<Job>>,
    active: HashSet<String>,
}

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
        if crate::services::backup::restore_lock_active() {
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
        if crate::services::backup::restore_lock_active() {
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
        if crate::services::backup::restore_lock_active() {
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

fn front_job(reg: &Mutex<Registry>, note_id: &str) -> Option<Job> {
    reg.lock()
        .unwrap()
        .queues
        .get(note_id)
        .and_then(|q| q.front())
        .cloned()
}

fn set_status(
    reg: &Mutex<Registry>,
    note_id: &str,
    job_id: &str,
    status: JobStatus,
) {
    let mut g = reg.lock().unwrap();
    if let Some(q) = g.queues.get_mut(note_id) {
        if let Some(j) = q.iter_mut().find(|j| j.id == job_id) {
            j.status = status;
        }
    }
}

fn set_transcription_ids(
    reg: &Mutex<Registry>,
    note_id: &str,
    job_id: &str,
    transcription_id: &str,
    soniox_file_id: &str,
) {
    let mut g = reg.lock().unwrap();
    if let Some(q) = g.queues.get_mut(note_id) {
        if let Some(j) = q.iter_mut().find(|j| j.id == job_id) {
            j.transcription_id = Some(transcription_id.to_string());
            j.soniox_file_id = Some(soniox_file_id.to_string());
        }
    }
}

fn cleanup_file(path: &Path) {
    if path.as_os_str().is_empty() {
        return;
    }
    let _ = std::fs::remove_file(path);
}

fn client_from_db(db: &Database) -> Result<SonioxClient, String> {
    let lang = crate::services::i18n::ui_lang(db);
    if db.get_setting("ai_consent") != Some("true".to_string()) {
        return Err(crate::services::i18n::t(&lang, "error-ai-consent"));
    }
    let key = db
        .get_setting("soniox_api_key")
        .or_else(|| std::env::var("SONIOX_API_KEY").ok())
        .or_else(|| option_env!("SONIOX_API_KEY").map(String::from))
        .unwrap_or_default();
    if key.is_empty() || key == "your_key_here" {
        return Err(crate::services::i18n::t(&lang, "stt-error-soniox-key"));
    }
    Ok(SonioxClient::new(key).with_lang(lang))
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
            PathBuf::from(crate::services::audio::output_dir()).join(name);
        if fallback.is_file() {
            return fallback;
        }
    }
    p
}

async fn process_front(reg: &Mutex<Registry>, db: &Database, note_id: &str) {
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
            JobStatus::Failed(crate::services::i18n::t(
                &crate::services::i18n::ui_lang(db),
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

pub fn append_transcription_to_note(db: &Database, note_id: &str, text: &str) {
    let note = match db.get_note(note_id) {
        Ok(Some(n)) => n,
        _ => return,
    };
    let new_content = if note.content.is_empty() {
        text.to_string()
    } else {
        format!("{}\n{}", note.content, text)
    };
    let _ = db.update_note(
        note_id,
        &UpdateNote {
            title: None,
            content: Some(new_content.clone()),
            tags: None,
        },
    );
    crate::services::embed::embed_note(
        note_id.to_string(),
        note.title.unwrap_or_default(),
        new_content,
        note.tags,
        note.created_at,
    );
}
