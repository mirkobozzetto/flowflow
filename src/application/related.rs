use crate::application::ai::char_prefix;
use crate::application::constants::{
    NOTE_LINK_LABEL_PROMPT, RELATED_FETCH_K, RELATED_GAP_FLOOR, RELATED_K,
    RELATED_KEYWORD_K, RELATED_MAX_DISTANCE, RELATED_MIN_GAP,
    RELATED_QUERY_CHUNKS, RELATED_STORE_K, RRF_K,
};
use crate::infrastructure::llm::LlmClient;
use crate::infrastructure::persistence::chunk_repo::ChunkRecord;
use crate::infrastructure::persistence::note_link_repo::NoteLinkRow;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::vectordb::{SearchResult, VectorStore};
use std::collections::HashMap;

/// One row of the "related notes" section on NoteDetail. `why` is the stored
/// LLM label (NULL-safe: absent on live-search rows and on label failures).
/// `actionable` is false for live-search rows: there is no note_links row to
/// pin or dismiss yet.
#[derive(Clone, Debug, PartialEq)]
pub struct LinkedNote {
    pub note_id: String,
    pub display: String,
    pub why: Option<String>,
    pub pinned: bool,
    pub created_at: String,
    pub actionable: bool,
}

/// Search candidate with its fused ranking score, consumed by both the live
/// UI fallback and the persistent note_links computation.
#[derive(Clone, Debug, PartialEq)]
pub struct RelatedCandidate {
    pub note_id: String,
    pub label: String,
    pub created_at: String,
    pub score: f32,
}

/// Collapse raw chunk hits to one row per note: drop the note itself (its own
/// note and attachment chunks are always the top hits), keep the best (lowest)
/// distance per note, sorted ascending.
pub fn best_per_note(
    self_id: &str,
    hits: Vec<SearchResult>,
) -> Vec<SearchResult> {
    let mut best: HashMap<String, SearchResult> = HashMap::new();
    for h in hits {
        if h.note_id == self_id || h.note_id.is_empty() {
            continue;
        }
        match best.get(&h.note_id) {
            Some(b) if b.distance <= h.distance => {}
            _ => {
                best.insert(h.note_id.clone(), h);
            }
        }
    }
    let mut out: Vec<SearchResult> = best.into_values().collect();
    out.sort_by(|a, b| {
        a.distance
            .partial_cmp(&b.distance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

/// How many candidates of a distance-ascending list to keep. Hard ceiling
/// first, floor candidates always kept, then cut at the widest distance jump
/// in the grey zone - a small corpus has no stable absolute threshold, but the
/// jump between "same topic" and "unrelated" is visible in the gaps. A jump
/// narrower than `min_gap` means one continuous cluster: keep everything.
pub fn gap_cutoff(
    distances: &[f32],
    floor: f32,
    ceiling: f32,
    min_gap: f32,
) -> usize {
    let n = distances.iter().take_while(|d| **d <= ceiling).count();
    if n == 0 {
        return 0;
    }
    let base = distances[..n].iter().take_while(|d| **d <= floor).count();
    let mut keep = n;
    let mut widest = min_gap;
    for i in base.max(1)..n {
        let gap = distances[i] - distances[i - 1];
        if gap > widest {
            widest = gap;
            keep = i;
        }
    }
    keep
}

/// FTS query text from the note's title and tags; empty when nothing usable.
pub fn keyword_query(title: &str, tags_json: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    let t = title.trim();
    if !t.is_empty() {
        parts.push(t.to_string());
    }
    if let Ok(tags) = serde_json::from_str::<Vec<String>>(tags_json) {
        for tag in tags {
            let tag = tag.trim().to_string();
            if !tag.is_empty() {
                parts.push(tag);
            }
        }
    }
    parts.join(" ")
}

/// Rank-based reciprocal-rank fusion of the vector leg and the keyword leg.
/// A note present in both legs sums its contributions; the vector-leg row
/// wins for display (its distance/created_at are the authentic ones).
pub fn rrf_fuse(
    vector_leg: Vec<SearchResult>,
    keyword_leg: Vec<SearchResult>,
    k: f32,
) -> Vec<(SearchResult, f32)> {
    let mut scores: HashMap<String, f32> = HashMap::new();
    let mut rows: HashMap<String, SearchResult> = HashMap::new();
    for (rank, r) in keyword_leg.into_iter().enumerate() {
        *scores.entry(r.note_id.clone()).or_default() +=
            1.0 / (k + rank as f32 + 1.0);
        rows.entry(r.note_id.clone()).or_insert(r);
    }
    for (rank, r) in vector_leg.into_iter().enumerate() {
        *scores.entry(r.note_id.clone()).or_default() +=
            1.0 / (k + rank as f32 + 1.0);
        rows.insert(r.note_id.clone(), r);
    }
    let mut out: Vec<(SearchResult, f32)> = rows
        .into_iter()
        .map(|(id, r)| {
            let s = scores.get(&id).copied().unwrap_or(0.0);
            (r, s)
        })
        .collect();
    out.sort_by(|a, b| {
        b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

fn display_label(title: &str, text: &str) -> Option<String> {
    let label = if title.trim().is_empty() {
        char_prefix(text.trim(), 60)
    } else {
        title.trim().to_string()
    };
    if label.is_empty() {
        None
    } else {
        Some(label)
    }
}

/// The v2 search engine: every stored chunk vector queries (multi-topic notes
/// link on all their topics), a keyword leg on title+tags catches shared
/// proper nouns, RRF fuses both, the gap cutoff trims the tail.
pub async fn related_candidates_with(
    store: &VectorStore,
    note_id: &str,
    chunks: &[ChunkRecord],
) -> Result<Vec<RelatedCandidate>, String> {
    let mut all_hits = Vec::new();
    for c in chunks.iter().take(RELATED_QUERY_CHUNKS) {
        all_hits.extend(
            store
                .search(c.vector.clone(), RELATED_FETCH_K, None)
                .await?,
        );
    }
    let vector_leg = best_per_note(note_id, all_hits);
    let dists: Vec<f32> = vector_leg.iter().map(|r| r.distance).collect();
    let keep = gap_cutoff(
        &dists,
        RELATED_GAP_FLOOR,
        RELATED_MAX_DISTANCE,
        RELATED_MIN_GAP,
    );
    let vector_leg: Vec<SearchResult> =
        vector_leg.into_iter().take(keep).collect();

    let mut keyword_leg = Vec::new();
    let first = &chunks[0];
    let kw = keyword_query(&first.title, &first.tags);
    if !kw.is_empty() {
        let hits = store
            .hybrid_search(&kw, first.vector.clone(), RELATED_FETCH_K, None)
            .await
            .unwrap_or_default();
        keyword_leg = best_per_note(note_id, hits);
        keyword_leg.truncate(RELATED_KEYWORD_K);
    }

    let fused = rrf_fuse(vector_leg, keyword_leg, RRF_K);
    let mut out = Vec::new();
    for (hit, score) in fused {
        let Some(label) = display_label(&hit.title, &hit.chunk_text) else {
            continue;
        };
        out.push(RelatedCandidate {
            note_id: hit.note_id,
            label,
            created_at: hit.created_at,
            score,
        });
    }
    Ok(out)
}

async fn related_candidates(
    note_id: &str,
) -> Result<Vec<RelatedCandidate>, String> {
    let db = Database::open()?;
    let chunks = db.chunks_for_owner(note_id, "note")?;
    if chunks.is_empty() {
        return Ok(vec![]);
    }
    let store = VectorStore::open().await?;
    related_candidates_with(&store, note_id, &chunks).await
}

fn row_to_linked(row: NoteLinkRow) -> Option<LinkedNote> {
    let display = display_label(&row.title, &row.content)?;
    Some(LinkedNote {
        note_id: row.other_note_id,
        display,
        why: row.label,
        pinned: row.pinned,
        created_at: row.note_created_at,
        actionable: true,
    })
}

/// What RelatedSection renders: ONE merged list. Similarity is symmetric, so
/// both stored directions fuse (each device computes per note at embed time,
/// the union covers not-yet-recomputed notes); live v2 search only fills in
/// when this note has no computed links yet. Dedup keeps the best row per
/// note: stored beats live, pinned beats active, then highest score.
pub async fn linked_notes(note_id: &str) -> Result<Vec<LinkedNote>, String> {
    let db = Database::open()?;
    let mut rows: Vec<(LinkedNote, f64)> = db
        .note_backlinks_for(note_id)?
        .into_iter()
        .filter_map(|r| {
            let score = r.score;
            row_to_linked(r).map(|l| (l, score))
        })
        .collect();
    if db.has_note_links(note_id)? {
        rows.extend(db.note_links_for(note_id)?.into_iter().filter_map(|r| {
            let score = r.score;
            row_to_linked(r).map(|l| (l, score))
        }));
    } else {
        rows.extend(
            related_candidates(note_id)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|c| {
                    let score = c.score as f64;
                    (
                        LinkedNote {
                            note_id: c.note_id,
                            display: c.label,
                            why: None,
                            pinned: false,
                            created_at: c.created_at,
                            actionable: false,
                        },
                        score,
                    )
                }),
        );
    }
    let mut best: HashMap<String, (LinkedNote, f64)> = HashMap::new();
    for (link, score) in rows {
        let replace = match best.get(&link.note_id) {
            None => true,
            Some((old, old_score)) => {
                (link.actionable, link.pinned, score)
                    > (old.actionable, old.pinned, *old_score)
            }
        };
        if replace {
            best.insert(link.note_id.clone(), (link, score));
        }
    }
    let mut merged: Vec<(LinkedNote, f64)> = best.into_values().collect();
    merged.sort_by(|a, b| {
        (b.0.pinned, b.1)
            .partial_cmp(&(a.0.pinned, a.1))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(merged.into_iter().take(RELATED_K).map(|(l, _)| l).collect())
}

/// UI action: dismiss ('dismissed', never shown again) or pin ('pinned',
/// always first). The merged list hides direction, and the pair may be stored
/// in both (each note computed its own row), so the state applies to BOTH
/// directions - otherwise the surviving row resurfaces the link.
pub fn set_link_state(
    note_a: &str,
    note_b: &str,
    state: &str,
) -> Result<(), String> {
    let db = Database::open()?;
    db.set_note_link_pair_state(note_a, note_b, state)
}

fn note_excerpt(db: &Database, note_id: &str) -> Option<String> {
    let note = db.get_note(note_id).ok()??;
    let title = note.title.unwrap_or_default();
    let body = char_prefix(note.content.trim(), 200);
    Some(format!("{title}\n{body}"))
}

async fn label_new_links(ai: &LlmClient, db: &Database, src: &str) {
    let pending = match db.unlabeled_note_links(src) {
        Ok(p) => p,
        Err(_) => return,
    };
    if pending.is_empty() {
        return;
    }
    let Some(src_text) = note_excerpt(db, src) else {
        return;
    };
    for dst in pending {
        let Some(dst_text) = note_excerpt(db, &dst) else {
            continue;
        };
        let user = format!("Note A:\n{src_text}\n\nNote B:\n{dst_text}");
        match ai.chat(NOTE_LINK_LABEL_PROMPT, &user).await {
            Ok(raw) => {
                let label = raw.trim();
                if !label.is_empty() && label.chars().count() <= 80 {
                    let _ = db.set_note_link_label(src, &dst, label);
                }
            }
            Err(e) => {
                // Label stays NULL; the link shows unlabeled.
                eprintln!("[related] label {src}->{dst}: {e}");
            }
        }
    }
}

/// Persist the note's links at embed time. Recompute replaces active links,
/// preserves dismissed (never comes back) and pinned (always kept), then
/// labels the newly created rows. Failures log and never break the embed.
pub(crate) async fn compute_note_links(
    store: &VectorStore,
    ai: &LlmClient,
    db: &Database,
    note_id: &str,
) {
    let chunks = match db.chunks_for_owner(note_id, "note") {
        Ok(c) => c,
        Err(_) => return,
    };
    if chunks.is_empty() {
        let _ = db.replace_note_links(note_id, &[]);
        return;
    }
    let cands = match related_candidates_with(store, note_id, &chunks).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[related] links {note_id}: {e}");
            return;
        }
    };
    let stored: Vec<(String, f64)> = cands
        .iter()
        .take(RELATED_STORE_K)
        .map(|c| (c.note_id.clone(), c.score as f64))
        .collect();
    if let Err(e) = db.replace_note_links(note_id, &stored) {
        eprintln!("[related] links store {note_id}: {e}");
        return;
    }
    label_new_links(ai, db, note_id).await;
}
