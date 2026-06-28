use crate::infrastructure::transcription::SttProvider;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

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
pub(super) struct Registry {
    pub(super) queues: HashMap<String, VecDeque<Job>>,
    pub(super) active: HashSet<String>,
}

pub(super) fn front_job(reg: &Mutex<Registry>, note_id: &str) -> Option<Job> {
    reg.lock()
        .unwrap()
        .queues
        .get(note_id)
        .and_then(|q| q.front())
        .cloned()
}

pub(super) fn set_status(
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

pub(super) fn set_transcription_ids(
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

pub(super) fn cleanup_file(path: &Path) {
    if path.as_os_str().is_empty() {
        return;
    }
    let _ = std::fs::remove_file(path);
}
