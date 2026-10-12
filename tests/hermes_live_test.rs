use flowflow::application::hermes_live;
use flowflow::infrastructure::hermes::{parse_jobs, HermesJob};

/// `GET /api/jobs` as Mirko's Hermes answered it on 2026-10-10.
const JOBS: &str = include_str!("fixtures/hermes_jobs.json");

fn jobs() -> Vec<HermesJob> {
    parse_jobs(&serde_json::from_str(JOBS).unwrap())
}

#[test]
fn only_running_tasks_delivered_to_flowflow_get_a_live_activity() {
    let mut jobs = jobs();
    jobs[0].deliver = "origin,ntfy".into();
    jobs[1].deliver = "ntfy".into();
    jobs[1].enabled = false;
    jobs[2].deliver = "telegram,ntfy".into();
    jobs[3].deliver = "ntfy".into();
    jobs[3].next_run_at = None;

    let scheduled = hermes_live::scheduled(&jobs);

    let got: Vec<(&str, &str, i64)> = scheduled
        .iter()
        .map(|j| (j.id.as_str(), j.name.as_str(), j.next_run_at))
        .collect();
    assert_eq!(
        got,
        vec![
            ("7455526871f4", jobs[0].name.as_str(), 1_791_781_200),
            ("de8dae5d216d", jobs[2].name.as_str(), 1_791_628_508),
        ]
    );
}

#[test]
fn tasks_left_on_telegram_alone_get_none() {
    assert!(hermes_live::scheduled(&jobs()).is_empty());
}
