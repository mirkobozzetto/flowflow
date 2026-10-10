//! Hermes' scheduled tasks, read and steered from FlowFlow. Hermes keeps its
//! schedules on its own clock; the phone shows them on the phone's.

use crate::application::i18n::{month_abbr, t, t_args, weekday_name};
use crate::infrastructure::hermes::{
    HermesClient, HermesError, HermesJob, JobSchedule,
};
use crate::infrastructure::persistence::Database;
use chrono::{DateTime, Datelike, Offset, TimeZone, Timelike, Weekday};

const DAY_MINUTES: i32 = 24 * 60;
// Cron numbers its weekdays from Sunday, 7 being Sunday again.
const CRON_WEEK: [Weekday; 8] = [
    Weekday::Sun,
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];

pub async fn list(db: &Database) -> Result<Vec<HermesJob>, HermesError> {
    HermesClient::from_db(db)?.jobs().await
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JobAction {
    Pause,
    Resume,
    RunNow,
    Delete,
}

/// One action on a job, done on Hermes; the list is read again after it.
pub async fn act(
    db: &Database,
    job_id: &str,
    action: JobAction,
) -> Result<(), HermesError> {
    let client = HermesClient::from_db(db)?;
    match action {
        JobAction::Pause => client.job_action(job_id, "pause").await,
        JobAction::Resume => client.job_action(job_id, "resume").await,
        JobAction::RunNow => client.job_action(job_id, "run").await,
        JobAction::Delete => client.delete_job(job_id).await,
    }
}

/// What Hermes answered on the job's last run; None before its first.
pub async fn latest_result(
    db: &Database,
    job_id: &str,
) -> Result<Option<String>, HermesError> {
    let client = HermesClient::from_db(db)?;
    let Some(session) = client.last_run_session(job_id).await? else {
        return Ok(None);
    };
    Ok(client
        .messages(&session)
        .await?
        .iter()
        .rev()
        .filter(|m| m.role == "assistant")
        .map(|m| m.text())
        .find(|t| !t.trim().is_empty()))
}

/// Minutes the phone's clock is ahead of Hermes', read at a job's next run:
/// Hermes writes next runs in its own zone, and its API says nothing else
/// of that zone.
pub fn clock_shift<Tz: TimeZone>(jobs: &[HermesJob], tz: &Tz) -> i32 {
    jobs.iter()
        .find_map(|j| {
            DateTime::parse_from_rfc3339(j.next_run_at.as_deref()?).ok()
        })
        .map_or(0, |at| {
            let here = at.with_timezone(tz).offset().fix().local_minus_utc();
            (here - at.offset().local_minus_utc()) / 60
        })
}

/// "Du lundi au vendredi à 7 h" on the phone's clock, `shift` minutes ahead
/// of Hermes'; Hermes' own words for anything rarer.
pub fn schedule_text(lang: &str, schedule: &JobSchedule, shift: i32) -> String {
    let said = match schedule.kind.as_str() {
        "interval" => schedule.minutes.map(|m| every_text(lang, m)),
        "cron" => schedule
            .expr
            .as_deref()
            .and_then(|e| cron_text(lang, e, shift)),
        _ => None,
    };
    said.unwrap_or_else(|| schedule.display.clone())
}

/// "Demain à 7 h" on the phone's clock; None while the task is paused.
pub fn next_run_text<Tz: TimeZone>(
    lang: &str,
    job: &HermesJob,
    now: &DateTime<Tz>,
) -> Option<String> {
    if job.paused() {
        return None;
    }
    let at = DateTime::parse_from_rfc3339(job.next_run_at.as_deref()?)
        .ok()?
        .with_timezone(&now.timezone());
    let time = clock(lang, (at.hour() * 60 + at.minute()) as i32);
    let on = |when: String| {
        t_args(lang, "hermes-job-on", &[("when", &when), ("time", &time)])
    };
    Some(match (at.date_naive() - now.date_naive()).num_days() {
        ..=0 => t_args(lang, "hermes-job-today", &[("time", &time)]),
        1 => t_args(lang, "hermes-job-tomorrow", &[("time", &time)]),
        2..=6 => on(capitalize(weekday_name(lang, at.weekday()))),
        _ => on(format!("{} {}", at.day(), month_abbr(lang, at.month()))),
    })
}

/// How the last run went.
pub fn status_text(lang: &str, job: &HermesJob) -> String {
    match job.last_status.as_deref() {
        None => t(lang, "hermes-job-never"),
        Some("ok") => t(lang, "hermes-job-ok"),
        Some(_) => t(lang, "hermes-job-failed"),
    }
}

fn every_text(lang: &str, minutes: u32) -> String {
    match minutes {
        60 => t(lang, "hermes-job-every-hour"),
        m if m % 60 == 0 => t_args(
            lang,
            "hermes-job-every-hours",
            &[("hours", &(m / 60).to_string())],
        ),
        m => t_args(
            lang,
            "hermes-job-every-minutes",
            &[("minutes", &m.to_string())],
        ),
    }
}

// A fixed minute, listed hours, any day of the month, chosen weekdays: what
// people schedule. A time pushed across midnight would move the days too.
fn cron_text(lang: &str, expr: &str, shift: i32) -> Option<String> {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    let [minute, hours, "*", "*", days] = fields.as_slice() else {
        return None;
    };
    let minute: i32 = minute.parse().ok()?;
    let times = hours
        .split(',')
        .map(|h| {
            let at = h.parse::<i32>().ok()? * 60 + minute + shift;
            (0..DAY_MINUTES).contains(&at).then(|| clock(lang, at))
        })
        .collect::<Option<Vec<_>>>()?;
    let times = join(lang, &times);
    let day = |d: &str| {
        let w = CRON_WEEK.get(d.parse::<usize>().ok()?)?;
        Some(weekday_name(lang, *w).to_string())
    };
    Some(match *days {
        "*" => t_args(lang, "hermes-job-daily", &[("times", &times)]),
        d if d.contains('-') => {
            let (from, to) = d.split_once('-')?;
            t_args(
                lang,
                "hermes-job-range",
                &[("from", &day(from)?), ("to", &day(to)?), ("times", &times)],
            )
        }
        d if d.contains(',') => {
            let names = d.split(',').map(day).collect::<Option<Vec<_>>>()?;
            t_args(
                lang,
                "hermes-job-days",
                &[("days", &capitalize(&join(lang, &names))), ("times", &times)],
            )
        }
        d => t_args(
            lang,
            "hermes-job-weekly",
            &[("day", &day(d)?), ("times", &times)],
        ),
    })
}

// "7 h 30" in French, "7:30 AM" in English.
fn clock(lang: &str, at: i32) -> String {
    let (h, m) = (at / 60, at % 60);
    if lang == "fr" {
        return if m == 0 {
            format!("{h} h")
        } else {
            format!("{h} h {m:02}")
        };
    }
    let half = if h < 12 { "AM" } else { "PM" };
    let h = if h % 12 == 0 { 12 } else { h % 12 };
    if m == 0 {
        format!("{h} {half}")
    } else {
        format!("{h}:{m:02} {half}")
    }
}

fn join(lang: &str, items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => {
            format!("{} {} {last}", rest.join(", "), t(lang, "hermes-job-and"))
        }
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}
