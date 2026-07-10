use crate::infrastructure::persistence::Database;

/// One stored link joined with the OTHER note's display fields (title/content
/// for the label fallback, created_at for the date line).
pub struct NoteLinkRow {
    pub other_note_id: String,
    pub score: f64,
    pub label: Option<String>,
    pub pinned: bool,
    pub title: String,
    pub content: String,
    pub note_created_at: String,
}

fn map_link_row(row: &rusqlite::Row) -> rusqlite::Result<NoteLinkRow> {
    let state: String = row.get(3)?;
    Ok(NoteLinkRow {
        other_note_id: row.get(0)?,
        score: row.get(1)?,
        label: row.get(2)?,
        pinned: state == "pinned",
        title: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
        content: row.get(5)?,
        note_created_at: row.get(6)?,
    })
}

impl Database {
    /// Recompute-time upsert: active links are replaced by the new candidate
    /// set; dismissed rows are untouched (a rejected link never comes back,
    /// the conflict clause only refreshes its score); pinned rows survive
    /// even when they drop out of the candidates.
    pub fn replace_note_links(
        &self,
        src: &str,
        candidates: &[(String, f64)],
    ) -> Result<(), String> {
        let conn = self.conn();
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Links tx: {e}"))?;
        // Only active rows that dropped out of the candidate set go away:
        // surviving rows keep their label ("computed once at creation").
        if candidates.is_empty() {
            tx.execute(
                "DELETE FROM note_links
                 WHERE src_note_id = ?1 AND state = 'active'",
                rusqlite::params![src],
            )
            .map_err(|e| format!("Delete stale links: {e}"))?;
        } else {
            let placeholders = vec!["?"; candidates.len()].join(",");
            let sql = format!(
                "DELETE FROM note_links
                 WHERE src_note_id = ?1 AND state = 'active'
                   AND dst_note_id NOT IN ({placeholders})"
            );
            let mut params: Vec<&dyn rusqlite::ToSql> = vec![&src];
            for (dst, _) in candidates {
                params.push(dst);
            }
            tx.execute(&sql, params.as_slice())
                .map_err(|e| format!("Delete stale links: {e}"))?;
        }
        for (dst, score) in candidates {
            if dst == src {
                continue;
            }
            tx.execute(
                "INSERT INTO note_links (src_note_id, dst_note_id, score)
                 SELECT ?1, ?2, ?3
                 WHERE EXISTS (SELECT 1 FROM notes WHERE id = ?2)
                 ON CONFLICT(src_note_id, dst_note_id)
                 DO UPDATE SET score = excluded.score",
                rusqlite::params![src, dst, score],
            )
            .map_err(|e| format!("Insert link: {e}"))?;
        }
        tx.commit().map_err(|e| format!("Links commit: {e}"))
    }

    /// True when the note has ANY stored row (any state): the UI then trusts
    /// the store and never falls back to the live search, so a fully
    /// dismissed section stays empty instead of resurrecting links.
    pub fn has_note_links(&self, src: &str) -> Result<bool, String> {
        let conn = self.conn();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM note_links WHERE src_note_id = ?1",
                rusqlite::params![src],
                |r| r.get(0),
            )
            .map_err(|e| format!("Count links: {e}"))?;
        Ok(n > 0)
    }

    pub fn note_links_for(
        &self,
        src: &str,
    ) -> Result<Vec<NoteLinkRow>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT nl.dst_note_id, nl.score, nl.label, nl.state,
                        n.title, n.content, n.created_at
                 FROM note_links nl
                 JOIN notes n ON n.id = nl.dst_note_id
                 WHERE nl.src_note_id = ?1 AND nl.state != 'dismissed'
                 ORDER BY (nl.state = 'pinned') DESC, nl.score DESC",
            )
            .map_err(|e| format!("Prepare links: {e}"))?;
        let rows = stmt
            .query_map(rusqlite::params![src], map_link_row)
            .map_err(|e| format!("Query links: {e}"))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| format!("Link row: {e}"))?);
        }
        Ok(out)
    }

    /// Reverse direction: notes whose stored links cite this one ("cited by").
    /// Reads the same rows, so dismissing a link hides it in both directions.
    pub fn note_backlinks_for(
        &self,
        dst: &str,
    ) -> Result<Vec<NoteLinkRow>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT nl.src_note_id, nl.score, nl.label, nl.state,
                        n.title, n.content, n.created_at
                 FROM note_links nl
                 JOIN notes n ON n.id = nl.src_note_id
                 WHERE nl.dst_note_id = ?1 AND nl.state != 'dismissed'
                 ORDER BY (nl.state = 'pinned') DESC, nl.score DESC",
            )
            .map_err(|e| format!("Prepare backlinks: {e}"))?;
        let rows = stmt
            .query_map(rusqlite::params![dst], map_link_row)
            .map_err(|e| format!("Query backlinks: {e}"))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| format!("Backlink row: {e}"))?);
        }
        Ok(out)
    }

    /// State applies to the PAIR, both directions: the merged UI list hides
    /// direction, and when each note computed its own row a one-sided update
    /// would let the surviving row resurface the link.
    pub fn set_note_link_pair_state(
        &self,
        note_a: &str,
        note_b: &str,
        state: &str,
    ) -> Result<(), String> {
        if !matches!(state, "active" | "dismissed" | "pinned") {
            return Err(format!("Invalid link state: {state}"));
        }
        self.conn()
            .execute(
                "UPDATE note_links SET state = ?3
                 WHERE (src_note_id = ?1 AND dst_note_id = ?2)
                    OR (src_note_id = ?2 AND dst_note_id = ?1)",
                rusqlite::params![note_a, note_b, state],
            )
            .map(|_| ())
            .map_err(|e| format!("Set link state: {e}"))
    }

    pub fn unlabeled_note_links(
        &self,
        src: &str,
    ) -> Result<Vec<String>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT dst_note_id FROM note_links
                 WHERE src_note_id = ?1 AND label IS NULL
                   AND state != 'dismissed'
                 ORDER BY score DESC",
            )
            .map_err(|e| format!("Prepare unlabeled: {e}"))?;
        let rows = stmt
            .query_map(rusqlite::params![src], |r| r.get::<_, String>(0))
            .map_err(|e| format!("Query unlabeled: {e}"))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| format!("Unlabeled row: {e}"))?);
        }
        Ok(out)
    }

    pub fn set_note_link_label(
        &self,
        src: &str,
        dst: &str,
        label: &str,
    ) -> Result<(), String> {
        self.conn()
            .execute(
                "UPDATE note_links SET label = ?3
                 WHERE src_note_id = ?1 AND dst_note_id = ?2",
                rusqlite::params![src, dst, label],
            )
            .map(|_| ())
            .map_err(|e| format!("Set link label: {e}"))
    }
}
