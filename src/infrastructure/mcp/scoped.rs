use super::*;
use crate::application::agent_bindings::{
    BindingIdentity, ResolvedResourceRequirement,
};

impl McpPool {
    /// Caller validates the full plan before dialing. Every session then goes
    /// through server v2 admission; no partial pool is returned on failure.
    pub async fn connect_scoped(
        db: &Database,
        backend: &BackendClient,
        identity: &BindingIdentity,
        requirements: &[ResolvedResourceRequirement],
    ) -> Result<Self, BackendError> {
        if BackendClient::device_pubkey(db).as_deref()
            != Some(&identity.device_id)
        {
            return Err(BackendError::Network(
                "scoped device identity changed".into(),
            ));
        }
        let mut registries = Vec::new();
        for requirement in requirements {
            let url = backend.scoped_mcp_url(
                &identity.agent_id,
                &identity.package_digest,
                &requirement.key,
            )?;
            let registry =
                McpRegistry::connect_inner(db, backend, url, None).await?;
            // Discovery is not trusted to expand the signed owner's tool set.
            if registry.tools.iter().any(|tool| {
                requirement.manifest.tool(tool.name.as_ref()).is_none()
            }) {
                return Err(BackendError::Network(
                    "scoped peer advertised an unowned tool".into(),
                ));
            }
            registries.push(registry);
        }
        let pool = Self { registries };
        if !pool.duplicate_tools().is_empty() {
            return Err(BackendError::Network(
                "ambiguous scoped tool ownership".into(),
            ));
        }
        Ok(pool)
    }
}
