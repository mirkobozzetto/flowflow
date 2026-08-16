// On-device MCP client (RFC 0008). Opens one Streamable-HTTP connection to the backend's
// single fixed MCP proxy (`{backend}/v1/mcp`) authenticated with the device session as the
// bearer auth header, discovers the connector tools, and exposes them as rig `McpTool`s.
// The connection is dark unless a backend is configured (BackendClient::from_db == None).

use crate::infrastructure::backend::{BackendClient, BackendError};
use crate::infrastructure::persistence::Database;
use reqwest::header::{HeaderName, HeaderValue};
use rmcp::model::{ClientCapabilities, ClientInfo, Implementation, Tool};
use rmcp::service::{RunningService, ServerSink};
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::RoleClient;
use rmcp::ServiceExt;
use std::collections::HashMap;

// The agent-scoped proxy binds connector access to the entitled agent driving the call.
const AGENT_ID_HEADER: &str = "x-agent-id";

pub struct McpRegistry {
    tools: Vec<Tool>,
    peer: ServerSink,
    // Kept solely to keep the connection alive: the `peer` the agent's tools clone is only
    // valid while the running service lives. Dropping the registry tears down the session.
    #[allow(dead_code)]
    service: RunningService<RoleClient, ClientInfo>,
}

impl McpRegistry {
    /// Connect to the legacy single MCP proxy (`{backend}/v1/mcp`) with the device session
    /// bearer. The un-scoped route carries no agent context (the chat agent's path).
    pub async fn connect(
        db: &Database,
        backend: &BackendClient,
    ) -> Result<Self, BackendError> {
        Self::connect_inner(db, backend, backend.mcp_url(), None).await
    }

    /// Connect to the agent-scoped proxy (`{backend}/v1/connectors/{slug}/mcp`) with the
    /// device session bearer AND `x-agent-id`, so the backend applies that agent's
    /// governance gate. `slug` is the connector id (`google`), `agent_id` the entitled agent.
    pub async fn connect_agent(
        db: &Database,
        backend: &BackendClient,
        slug: &str,
        agent_id: &str,
    ) -> Result<Self, BackendError> {
        Self::connect_inner(
            db,
            backend,
            backend.connector_mcp_url(slug),
            Some(agent_id),
        )
        .await
    }

    /// `auth_header(token)` makes rmcp send `Authorization: Bearer {token}` on every leg
    /// (POST and the device's own SSE leg), which is the session the proxy validates. When
    /// `agent_id` is set, `x-agent-id` rides alongside it so the agent-scoped route resolves.
    async fn connect_inner(
        db: &Database,
        backend: &BackendClient,
        url: String,
        agent_id: Option<&str>,
    ) -> Result<Self, BackendError> {
        let token = backend.session(db).await?;
        let mut config = StreamableHttpClientTransportConfig::with_uri(url)
            .auth_header(token);
        if let Some(agent_id) = agent_id {
            let mut headers = HashMap::new();
            headers.insert(
                HeaderName::from_static(AGENT_ID_HEADER),
                HeaderValue::from_str(agent_id).map_err(|e| {
                    BackendError::Network(format!("x-agent-id header: {e}"))
                })?,
            );
            config = config.custom_headers(headers);
        }
        let transport = StreamableHttpClientTransport::from_config(config);

        let client_info = ClientInfo::new(
            ClientCapabilities::default(),
            Implementation::new("flowflow", env!("CARGO_PKG_VERSION")),
        );
        let service = client_info
            .serve(transport)
            .await
            .map_err(|e| BackendError::Network(format!("mcp serve: {e}")))?;
        let tools = service
            .list_tools(Default::default())
            .await
            .map_err(|e| BackendError::Network(format!("mcp list_tools: {e}")))?
            .tools;
        let peer = service.peer().to_owned();

        Ok(Self {
            tools,
            peer,
            service,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// The discovered tools, for `AgentBuilder::rmcp_tools`.
    pub fn tools(&self) -> Vec<Tool> {
        self.tools.clone()
    }

    /// The server sink the rig `McpTool`s forward calls through.
    pub fn peer(&self) -> ServerSink {
        self.peer.clone()
    }
}

/// The connectors one agent run talks to. Each entry keeps its own live session; dropping the
/// pool tears every one of them down, so it must outlive the run that mounted its tools.
pub struct McpPool {
    registries: Vec<McpRegistry>,
}

impl McpPool {
    /// Connect the agent-scoped proxy once per connector slug. A slug that fails to connect
    /// fails the whole pool: a partially connected agent would silently lose half its tools.
    pub async fn connect_agent(
        db: &Database,
        backend: &BackendClient,
        slugs: &[String],
        agent_id: &str,
    ) -> Result<Self, BackendError> {
        let mut registries = Vec::with_capacity(slugs.len());
        for slug in slugs {
            registries.push(
                McpRegistry::connect_agent(db, backend, slug, agent_id).await?,
            );
        }
        Ok(Self { registries })
    }

    /// One (tools, sink) pair per connector, for `AgentBuilder::rmcp_tools` - mounted
    /// separately so each tool carries the sink of the server that serves it.
    pub fn mounts(&self) -> Vec<(Vec<Tool>, ServerSink)> {
        self.registries
            .iter()
            .map(|r| (r.tools(), r.peer()))
            .collect()
    }

    /// Which server serves each tool name, for the hook's direct execution path.
    pub fn peers_by_tool(&self) -> Vec<(String, ServerSink)> {
        self.registries
            .iter()
            .flat_map(|r| {
                let peer = r.peer();
                r.tools()
                    .into_iter()
                    .map(move |t| (t.name.to_string(), peer.clone()))
            })
            .collect()
    }

    /// Tool names served by more than one connector. A duplicate makes routing ambiguous, so
    /// the caller refuses to run rather than guess which server owns the call.
    pub fn duplicate_tools(&self) -> Vec<String> {
        let mut seen = std::collections::BTreeSet::new();
        let mut dupes = std::collections::BTreeSet::new();
        for registry in &self.registries {
            for tool in registry.tools() {
                let name = tool.name.to_string();
                if !seen.insert(name.clone()) {
                    dupes.insert(name);
                }
            }
        }
        dupes.into_iter().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.registries.iter().all(McpRegistry::is_empty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Guards finding 12: the session must ride on the device->backend leg as the auth header.
    #[test]
    fn config_carries_bearer_auth_header() {
        let cfg =
            StreamableHttpClientTransportConfig::with_uri("https://x/v1/mcp")
                .auth_header("tok123");
        assert_eq!(cfg.auth_header.as_deref(), Some("tok123"));
        assert_eq!(&*cfg.uri, "https://x/v1/mcp");
    }
}
