use rusqlite::{params, Connection};

use crate::infrastructure::persistence::Database;

// One pinned connector manifest row. `manifest_json` is the canonical form the pinned
// `content_digest` was computed over; the gate reads it as data only (same rule as agents).
#[derive(Debug, Clone)]
pub struct PinnedConnector {
    pub slug: String,
    pub connector_type: String,
    pub version: i64,
    pub content_digest: String,
    pub manifest_json: String,
}

// The compiled Sheets manifest, kept ONLY as the v17 backfill + test fixture (RFC 0016: the
// distribution source of truth is the signed backend envelope). Must stay byte-identical to
// marketplace-flowflow/connectors/google-sheets.json until the backfill retires; the pinned
// digest test below the repo tests guards the drift.
pub const SHEETS_BACKFILL_MANIFEST_JSON: &str = r#"{
  "connector": "google-sheets",
  "version": 1,
  "type": "tabular_store",
  "server": "ghcr.io/klavis-ai/google-sheets-mcp-server",
  "mcp_prefix": "google_sheets_",
  "provides": ["search", "read", "create", "update"],
  "tools": [
    { "tool": "google_sheets_list_spreadsheets",  "resource": "spreadsheet", "action": "search", "risk": "read_only" },
    { "tool": "google_sheets_get_spreadsheet",    "resource": "spreadsheet", "action": "read",   "risk": "read_only" },
    { "tool": "google_sheets_list_sheets",        "resource": "sheet",       "action": "read",   "risk": "read_only" },
    { "tool": "google_sheets_create_spreadsheet", "resource": "spreadsheet", "action": "create", "risk": "read_write" },
    { "tool": "google_sheets_create_sheets",      "resource": "sheet",       "action": "create", "risk": "read_write" },
    { "tool": "google_sheets_write_to_cell",      "resource": "cell",        "action": "update", "risk": "read_write" }
  ]
}"#;

impl Database {
    pub fn pin_connector(&self, pin: &PinnedConnector) -> Result<(), String> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO installed_connectors
                 (slug, connector_type, version, content_digest, manifest_json)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(slug) DO UPDATE SET
                 connector_type = excluded.connector_type,
                 version = excluded.version,
                 content_digest = excluded.content_digest,
                 manifest_json = excluded.manifest_json,
                 pinned_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')",
            params![
                pin.slug,
                pin.connector_type,
                pin.version,
                pin.content_digest,
                pin.manifest_json
            ],
        )
        .map_err(|e| format!("pin_connector: {e}"))?;
        Ok(())
    }

    pub fn pinned_connector(&self, slug: &str) -> Option<PinnedConnector> {
        let conn = self.conn();
        conn.query_row(
            "SELECT slug, connector_type, version, content_digest, manifest_json
             FROM installed_connectors WHERE slug = ?1",
            [slug],
            |row| {
                Ok(PinnedConnector {
                    slug: row.get(0)?,
                    connector_type: row.get(1)?,
                    version: row.get(2)?,
                    content_digest: row.get(3)?,
                    manifest_json: row.get(4)?,
                })
            },
        )
        .ok()
    }

    // Lowest slug among pins of the type: the deterministic order every resolver shares
    // (device, backend loader, console - RFC 0016 F4).
    pub fn pinned_connector_for_type(
        &self,
        connector_type: &str,
    ) -> Option<PinnedConnector> {
        let conn = self.conn();
        conn.query_row(
            "SELECT slug, connector_type, version, content_digest, manifest_json
             FROM installed_connectors WHERE connector_type = ?1
             ORDER BY slug ASC LIMIT 1",
            [connector_type],
            |row| {
                Ok(PinnedConnector {
                    slug: row.get(0)?,
                    connector_type: row.get(1)?,
                    version: row.get(2)?,
                    content_digest: row.get(3)?,
                    manifest_json: row.get(4)?,
                })
            },
        )
        .ok()
    }

    pub fn remove_connector_pin(&self, slug: &str) -> Result<(), String> {
        let conn = self.conn();
        conn.execute(
            "DELETE FROM installed_connectors WHERE slug = ?1",
            [slug],
        )
        .map_err(|e| format!("remove_connector_pin: {e}"))?;
        Ok(())
    }

    pub fn list_pinned_connectors(&self) -> Vec<PinnedConnector> {
        let conn = self.conn();
        let Ok(mut stmt) = conn.prepare(
            "SELECT slug, connector_type, version, content_digest, manifest_json
             FROM installed_connectors ORDER BY slug",
        ) else {
            return Vec::new();
        };
        stmt.query_map([], |row| {
            Ok(PinnedConnector {
                slug: row.get(0)?,
                connector_type: row.get(1)?,
                version: row.get(2)?,
                content_digest: row.get(3)?,
                manifest_json: row.get(4)?,
            })
        })
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
    }
}

// v17 migrate hook: pin the compiled Sheets fixture so an existing install keeps working offline
// immediately after the update, before any backend fetch. Idempotent (INSERT OR IGNORE): a pin
// refreshed from a signed envelope is never clobbered by a later re-run.
pub fn backfill_sheets_pin(conn: &Connection) -> Result<(), String> {
    let manifest: serde_json::Value =
        serde_json::from_str(SHEETS_BACKFILL_MANIFEST_JSON)
            .map_err(|e| format!("sheets backfill fixture must parse: {e}"))?;
    let digest = crate::domain::agent_manifest::digest_of(&manifest);
    conn.execute(
        "INSERT OR IGNORE INTO installed_connectors
             (slug, connector_type, version, content_digest, manifest_json)
         VALUES ('google', 'tabular_store', 1, ?1, ?2)",
        params![digest, SHEETS_BACKFILL_MANIFEST_JSON],
    )
    .map_err(|e| format!("backfill sheets pin: {e}"))?;
    Ok(())
}
