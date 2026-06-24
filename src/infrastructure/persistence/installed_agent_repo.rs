use crate::domain::agent_manifest::{InstalledAgent, VerifiedAgent};
use crate::infrastructure::persistence::Database;

impl Database {
    /// Pin a verified agent. Upsert by id so a re-install (e.g. an update) repins the digest and
    /// manifest in place. Callers verify the signature/digest BEFORE this; the repo only stores.
    pub fn install_agent(&self, agent: &VerifiedAgent) -> Result<(), String> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO installed_agents
                 (id, version, content_digest, manifest_json, active)
             VALUES (?1, ?2, ?3, ?4, 1)
             ON CONFLICT(id) DO UPDATE SET
                 version = excluded.version,
                 content_digest = excluded.content_digest,
                 manifest_json = excluded.manifest_json",
            rusqlite::params![
                agent.manifest.id,
                agent.manifest.version,
                agent.content_digest,
                agent.manifest_json,
            ],
        )
        .map_err(|e| format!("install agent: {e}"))?;
        Ok(())
    }

    pub fn get_installed_agent(&self, id: &str) -> Option<InstalledAgent> {
        let conn = self.conn();
        conn.query_row(
            "SELECT id, version, content_digest, manifest_json, installed_at, active
             FROM installed_agents WHERE id = ?1",
            [id],
            row_to_installed_agent,
        )
        .ok()
    }

    pub fn list_installed_agents(&self) -> Vec<InstalledAgent> {
        let conn = self.conn();
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, version, content_digest, manifest_json, installed_at, active
             FROM installed_agents ORDER BY installed_at",
        ) else {
            return Vec::new();
        };
        let rows = stmt
            .query_map([], row_to_installed_agent)
            .map(|r| r.flatten().collect());
        rows.unwrap_or_default()
    }

    pub fn set_agent_active(
        &self,
        id: &str,
        active: bool,
    ) -> Result<(), String> {
        let conn = self.conn();
        conn.execute(
            "UPDATE installed_agents SET active = ?2 WHERE id = ?1",
            rusqlite::params![id, active as i64],
        )
        .map_err(|e| format!("set agent active: {e}"))?;
        Ok(())
    }

    /// Pin the per-install bound resource the user armed (e.g. `{"spreadsheet_id": "1Ab.."}`). The
    /// builder merges it over the manifest's `governance.bound_resource`. Pass None to clear it.
    pub fn set_agent_binding(
        &self,
        id: &str,
        bound_json: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.conn();
        conn.execute(
            "UPDATE installed_agents SET bound_json = ?2 WHERE id = ?1",
            rusqlite::params![id, bound_json],
        )
        .map_err(|e| format!("set agent binding: {e}"))?;
        Ok(())
    }

    /// The armed bound resource for an agent, parsed. None when unbound or the row is absent.
    pub fn get_agent_binding(&self, id: &str) -> Option<serde_json::Value> {
        let conn = self.conn();
        let raw: Option<String> = conn
            .query_row(
                "SELECT bound_json FROM installed_agents WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .ok()?;
        serde_json::from_str(&raw?).ok()
    }
}

fn row_to_installed_agent(
    row: &rusqlite::Row,
) -> rusqlite::Result<InstalledAgent> {
    Ok(InstalledAgent {
        id: row.get(0)?,
        version: row.get(1)?,
        content_digest: row.get(2)?,
        manifest_json: row.get(3)?,
        installed_at: row.get(4)?,
        active: row.get::<_, i64>(5)? != 0,
    })
}
