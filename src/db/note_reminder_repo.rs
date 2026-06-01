use crate::db::{now_iso, Database};
use crate::models::{NewNoteReminder, NoteReminder, REMINDER_STATE_ACTIVE};

const SELECT_COLS: &str = "id, note_id, reminder_id, backend, intent_hash, \
    due_year, due_month, due_day, due_hour, due_minute, is_all_day, tz_id, \
    recurrence, state, created_at";

fn map_reminder(row: &rusqlite::Row) -> rusqlite::Result<NoteReminder> {
    let is_all_day: i64 = row.get(10)?;
    Ok(NoteReminder {
        id: row.get(0)?,
        note_id: row.get(1)?,
        reminder_id: row.get(2)?,
        backend: row.get(3)?,
        intent_hash: row.get(4)?,
        due_year: row.get(5)?,
        due_month: row.get(6)?,
        due_day: row.get(7)?,
        due_hour: row.get(8)?,
        due_minute: row.get(9)?,
        is_all_day: is_all_day != 0,
        tz_id: row.get(11)?,
        recurrence: row.get(12)?,
        state: row.get(13)?,
        created_at: row.get(14)?,
    })
}

impl Database {
    pub fn add_note_reminder(
        &self,
        new: &NewNoteReminder,
    ) -> Result<NoteReminder, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let created_at = now_iso();
        let conn = self.conn();
        conn.execute(
            "INSERT INTO note_reminders
                (id, note_id, reminder_id, backend, intent_hash,
                 due_year, due_month, due_day, due_hour, due_minute,
                 is_all_day, tz_id, recurrence, state, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                     ?11, ?12, ?13, ?14, ?15)",
            rusqlite::params![
                id,
                new.note_id,
                new.reminder_id,
                new.backend,
                new.intent_hash,
                new.due_year,
                new.due_month,
                new.due_day,
                new.due_hour,
                new.due_minute,
                new.is_all_day as i32,
                new.tz_id,
                new.recurrence,
                REMINDER_STATE_ACTIVE,
                created_at,
            ],
        )
        .map_err(|e| format!("Add note reminder: {e}"))?;
        Ok(NoteReminder {
            id,
            note_id: new.note_id.clone(),
            reminder_id: new.reminder_id.clone(),
            backend: new.backend.clone(),
            intent_hash: new.intent_hash.clone(),
            due_year: new.due_year,
            due_month: new.due_month,
            due_day: new.due_day,
            due_hour: new.due_hour,
            due_minute: new.due_minute,
            is_all_day: new.is_all_day,
            tz_id: new.tz_id.clone(),
            recurrence: new.recurrence.clone(),
            state: REMINDER_STATE_ACTIVE.to_string(),
            created_at,
        })
    }

    pub fn reminders_for_note(&self, note_id: &str) -> Vec<NoteReminder> {
        let conn = self.conn();
        let sql = format!(
            "SELECT {SELECT_COLS} FROM note_reminders \
             WHERE note_id = ?1 ORDER BY created_at"
        );
        let mut stmt = match conn.prepare(&sql) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        stmt.query_map([note_id], map_reminder)
            .map(|rows| rows.flatten().collect())
            .unwrap_or_default()
    }

    pub fn reminder_exists_by_intent_hash(
        &self,
        note_id: &str,
        intent_hash: &str,
    ) -> bool {
        let conn = self.conn();
        conn.query_row(
            "SELECT COUNT(*) FROM note_reminders
             WHERE note_id = ?1 AND intent_hash = ?2 AND state = 'active'",
            rusqlite::params![note_id, intent_hash],
            |row| row.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false)
    }

    pub fn note_has_active_reminder(&self, note_id: &str) -> bool {
        let conn = self.conn();
        conn.query_row(
            "SELECT COUNT(*) FROM note_reminders
             WHERE note_id = ?1 AND state = 'active'",
            [note_id],
            |row| row.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false)
    }

    pub fn set_reminder_state(
        &self,
        id: &str,
        state: &str,
    ) -> Result<(), String> {
        let conn = self.conn();
        conn.execute(
            "UPDATE note_reminders SET state = ?1 WHERE id = ?2",
            rusqlite::params![state, id],
        )
        .map_err(|e| format!("Set reminder state: {e}"))?;
        Ok(())
    }

    pub fn delete_note_reminder(&self, id: &str) -> Result<(), String> {
        let conn = self.conn();
        conn.execute("DELETE FROM note_reminders WHERE id = ?1", [id])
            .map_err(|e| format!("Delete note reminder: {e}"))?;
        Ok(())
    }

    pub fn delete_reminders_for_note(
        &self,
        note_id: &str,
    ) -> Result<(), String> {
        let conn = self.conn();
        conn.execute(
            "DELETE FROM note_reminders WHERE note_id = ?1",
            [note_id],
        )
        .map_err(|e| format!("Delete reminders for note: {e}"))?;
        Ok(())
    }
}
