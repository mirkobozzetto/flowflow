//! Device/package-local selections. The backend accepts the change before the
//! local copy is stored; a failed write never grants a local execution binding.
use crate::application::agent_bindings::{
    BindingIdentity, ResolvedResourceRequirement, ResourceBinding,
};
use crate::domain::agent_manifest::digest_of_stored;
use crate::domain::scoped_agent_manifest::ScopedAgentManifest;
use crate::infrastructure::{backend::BackendClient, persistence::Database};
use serde_json::{json, Value};
use std::sync::Arc;

pub type AdmissionCheck = Arc<dyn Fn() -> Result<(), String> + Send + Sync>;

pub fn selection_key(identity: &BindingIdentity, requirement: &str) -> String {
    let scope = json!([
        identity.device_id,
        identity.agent_id,
        identity.package_digest,
        requirement
    ]);
    format!(
        "scoped_agent_selection:{}",
        crate::domain::agent_manifest::digest_of(&scope)
    )
}

pub fn context(
    db: &Database,
    id: &str,
) -> Result<(ScopedAgentManifest, BindingIdentity, AdmissionCheck), String> {
    let (manifest, digest, check) = db.load_scoped_execution_snapshot(id)?;
    let device_id = BackendClient::device_pubkey(db)
        .ok_or("backend device identity unavailable")?;
    Ok((
        manifest,
        BindingIdentity {
            device_id,
            agent_id: id.into(),
            package_digest: digest,
        },
        check,
    ))
}

pub fn load(
    db: &Database,
    identity: &BindingIdentity,
    manifest: &ScopedAgentManifest,
) -> Result<(Vec<ResourceBinding>, Vec<ResolvedResourceRequirement>), String> {
    let mut bindings = Vec::new();
    let mut resolved = Vec::new();
    for requirement in &manifest.execution.required_connectors {
        let raw = db
            .get_setting(&selection_key(identity, &requirement.key))
            .ok_or_else(|| {
                format!("select an owner for `{}`", requirement.key)
            })?;
        let value: Value =
            serde_json::from_str(&raw).map_err(|error| error.to_string())?;
        let slug = value["connector_slug"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("invalid selected owner")?;
        let resource = value
            .get("resource")
            .ok_or("selection lacks a resource field")?
            .clone();
        let connector =
            checked_connector(db, manifest, &requirement.key, slug, &resource)?;
        bindings.push(ResourceBinding {
            identity: identity.clone(),
            requirement_key: requirement.key.clone(),
            connector_slug: slug.into(),
            resource,
        });
        resolved.push(ResolvedResourceRequirement {
            key: requirement.key.clone(),
            connector_slug: slug.into(),
            manifest: connector,
            resource_required: requirement.resource_required,
        });
    }
    Ok((bindings, resolved))
}

fn checked_connector(
    db: &Database,
    manifest: &ScopedAgentManifest,
    key: &str,
    slug: &str,
    resource: &Value,
) -> Result<crate::domain::governance::ConnectorManifest, String> {
    let requirement = manifest
        .execution
        .required_connectors
        .iter()
        .find(|r| r.key == key)
        .ok_or("unknown requirement")?;
    let pin = db
        .pinned_connector(slug)
        .ok_or("selected connector is not pinned")?;
    if digest_of_stored(&pin.manifest_json).map_err(|e| e.to_string())?
        != pin.content_digest
    {
        return Err("connector pin digest mismatch".into());
    }
    let connector: crate::domain::governance::ConnectorManifest =
        serde_json::from_str(&pin.manifest_json).map_err(|e| e.to_string())?;
    if connector.connector_type != requirement.connector_type
        || connector.connector_type != pin.connector_type
        || requirement
            .capabilities
            .iter()
            .any(|action| !connector.provides.contains(action))
    {
        return Err(
            "selected connector does not satisfy the requirement".into()
        );
    }
    if requirement.resource_required {
        if connector.connector != "google-sheets"
            || connector.connector_type != "tabular_store"
            || connector.mcp_prefix != "google_sheets_"
        {
            return Err("resource adapter unavailable".into());
        }
        let object =
            resource.as_object().ok_or("resource must be an object")?;
        let present = |value: &Value| {
            value.as_str().is_some_and(|s| !s.trim().is_empty())
        };
        if !object.get("spreadsheet_id").is_some_and(present)
            || object.keys().any(|k| k != "spreadsheet_id" && k != "sheet")
            || object.get("sheet").is_some_and(|v| !present(v))
        {
            return Err("invalid Sheets resource".into());
        }
    } else if !resource.is_null() {
        return Err("resource-free owner requires null".into());
    }
    Ok(connector)
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectionView {
    pub key: String,
    pub connector_type: String,
    pub resource_required: bool,
    pub compatible_slugs: Vec<String>,
    pub selected_slug: Option<String>,
    pub selected_resource: Option<Value>,
}

pub fn views(db: &Database, id: &str) -> Result<Vec<SelectionView>, String> {
    let (manifest, identity, _) = context(db, id)?;
    Ok(manifest
        .execution
        .required_connectors
        .iter()
        .map(|requirement| {
            let probe_resource = if requirement.resource_required {
                json!({"spreadsheet_id":"selection-probe"})
            } else {
                Value::Null
            };
            let compatible_slugs = db
                .list_pinned_connectors()
                .into_iter()
                .filter(|pin| {
                    checked_connector(
                        db,
                        &manifest,
                        &requirement.key,
                        &pin.slug,
                        &probe_resource,
                    )
                    .is_ok()
                })
                .map(|pin| pin.slug)
                .collect();
            let selected = db
                .get_setting(&selection_key(&identity, &requirement.key))
                .and_then(|raw| serde_json::from_str::<Value>(&raw).ok());
            SelectionView {
                key: requirement.key.clone(),
                connector_type: requirement.connector_type.clone(),
                resource_required: requirement.resource_required,
                compatible_slugs,
                selected_slug: selected
                    .as_ref()
                    .and_then(|value| value["connector_slug"].as_str())
                    .map(String::from),
                selected_resource: selected
                    .and_then(|value| value.get("resource").cloned()),
            }
        })
        .collect())
}

pub async fn select(
    db: &Database,
    id: &str,
    key: &str,
    slug: &str,
    resource: Value,
) -> Result<(), String> {
    let (manifest, identity, check) = context(db, id)?;
    checked_connector(db, &manifest, key, slug, &resource)?;
    let backend =
        BackendClient::from_db(db).ok_or("backend is not configured")?;
    backend
        .put_scoped_binding(
            db,
            id,
            &identity.package_digest,
            key,
            slug,
            &resource,
        )
        .await
        .map_err(|e| e.to_string())?;
    check()?;
    db.set_setting(
        &selection_key(&identity, key),
        &json!({"connector_slug":slug,"resource":resource}).to_string(),
    )
}

pub async fn unselect(
    db: &Database,
    id: &str,
    key: &str,
) -> Result<(), String> {
    let (manifest, identity, check) = context(db, id)?;
    if !manifest
        .execution
        .required_connectors
        .iter()
        .any(|r| r.key == key)
    {
        return Err("unknown requirement".into());
    }
    let backend =
        BackendClient::from_db(db).ok_or("backend is not configured")?;
    backend
        .delete_scoped_binding(db, id, &identity.package_digest, key)
        .await
        .map_err(|e| e.to_string())?;
    check()?;
    db.delete_setting(&selection_key(&identity, key))
}

/// Native and external approval gates share this immutable selection snapshot.
pub fn snapshot_check(
    db: &Database,
    identity: &BindingIdentity,
    bindings: &[ResourceBinding],
    pin_check: AdmissionCheck,
) -> Result<AdmissionCheck, String> {
    let entries: Vec<_> = bindings.iter().map(|binding| {
        let key = selection_key(identity, &binding.requirement_key);
        let raw = json!({"connector_slug":binding.connector_slug,"resource":binding.resource}).to_string();
        (key, raw)
    }).collect();
    let path = db
        .conn()
        .path()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .ok_or("persistent store required")?;
    let connection = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|e| e.to_string())?;
    let connection = std::sync::Mutex::new(connection);
    Ok(Arc::new(move || {
        pin_check()?;
        let conn = connection.lock().map_err(|_| "selection check poisoned")?;
        for (key, expected) in &entries {
            let raw: String = conn
                .query_row(
                    "SELECT value FROM settings WHERE key=?1",
                    [key],
                    |row| row.get(0),
                )
                .map_err(|_| "selection removed")?;
            if &raw != expected {
                return Err("selection changed during execution".into());
            }
        }
        Ok(())
    }))
}
