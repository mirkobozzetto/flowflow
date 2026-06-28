use super::super::log;
use super::super::wire::SyncRow;
use super::LocalMeta;
use crate::infrastructure::sync::conflict::MergeOutcome;
use crate::infrastructure::sync::vv::{parse_vv, vv_cmp, vv_join};

fn hlc_after(candidate: Option<&str>, baseline: Option<&str>) -> bool {
    match (candidate, baseline) {
        (Some(c), Some(b)) => c > b,
        (Some(_), None) => true,
        _ => false,
    }
}

pub(super) fn hlc_guard(
    outcome: MergeOutcome,
    local: &LocalMeta,
    row: &SyncRow,
) -> MergeOutcome {
    let dominated_but_newer = match &outcome {
        MergeOutcome::AlreadyCurrent => {
            hlc_after(row.updated_hlc.as_deref(), local.updated_hlc.as_deref())
        }
        MergeOutcome::TakeRemote => {
            hlc_after(local.updated_hlc.as_deref(), row.updated_hlc.as_deref())
        }
        MergeOutcome::Concurrent { .. } => false,
    };
    if !dominated_but_newer {
        return outcome;
    }
    let (local_vv, corrupt_local) = parse_vv(&local.version_vector);
    let (remote_vv, corrupt_remote) = parse_vv(&row.version_vector);
    if corrupt_local || corrupt_remote {
        return outcome;
    }
    if matches!(
        vv_cmp(&local_vv, &remote_vv),
        crate::infrastructure::sync::vv::Dominance::Equal
    ) {
        return outcome;
    }
    let remote_newer =
        hlc_after(row.updated_hlc.as_deref(), local.updated_hlc.as_deref());
    log(&format!(
        "hlc guard: dominated-but-newer on {}:{} -> Concurrent (archive + \
         tie-break) instead of silent {}",
        row.entity_kind,
        row.entity_id,
        if remote_newer { "skip" } else { "overwrite" }
    ));
    MergeOutcome::Concurrent {
        remote_wins: remote_newer
            || (row.updated_hlc.as_deref(), row.origin_device.as_str())
                > (local.updated_hlc.as_deref(), local.origin_device.as_str()),
        joined: vv_join(&local_vv, &remote_vv),
        corrupt_local: false,
        corrupt_remote: false,
    }
}
