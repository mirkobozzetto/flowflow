use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Dominance {
    Equal,
    Local,
    Remote,
    Concurrent,
}

// A version vector that fails to parse is returned as (empty, corrupt=true).
// A corrupt vector must NEVER silently lose a comparison (an empty map is
// dominated by everything): the caller forces the Concurrent path instead, so
// the disagreement is resolved deterministically WITH an archive.
pub(super) fn parse_vv(s: &str) -> (BTreeMap<String, i64>, bool) {
    match serde_json::from_str(s) {
        Ok(m) => (m, false),
        Err(_) => (BTreeMap::new(), true),
    }
}

pub(super) fn vv_cmp(
    local: &BTreeMap<String, i64>,
    remote: &BTreeMap<String, i64>,
) -> Dominance {
    let local_ge = remote
        .iter()
        .all(|(k, v)| local.get(k).copied().unwrap_or(0) >= *v);
    let remote_ge = local
        .iter()
        .all(|(k, v)| remote.get(k).copied().unwrap_or(0) >= *v);
    match (local_ge, remote_ge) {
        (true, true) => Dominance::Equal,
        (true, false) => Dominance::Local,
        (false, true) => Dominance::Remote,
        (false, false) => Dominance::Concurrent,
    }
}

pub(super) fn vv_join(
    a: &BTreeMap<String, i64>,
    b: &BTreeMap<String, i64>,
) -> BTreeMap<String, i64> {
    let mut out = a.clone();
    for (k, v) in b {
        let e = out.entry(k.clone()).or_insert(0);
        if *v > *e {
            *e = *v;
        }
    }
    out
}
