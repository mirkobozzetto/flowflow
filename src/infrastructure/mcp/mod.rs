// On-device MCP client (RFC 0008). Opens one Streamable-HTTP connection to the backend's
// single fixed MCP proxy (`{backend}/v1/mcp`) authenticated with the device session as the
// bearer auth header, discovers the connector tools, and exposes them as rig `McpTool`s.
// The connection is dark unless a backend is configured (BackendClient::from_db == None).

use crate::infrastructure::backend::{BackendClient, BackendError};
use crate::infrastructure::persistence::Database;
use rmcp::model::{ClientCapabilities, ClientInfo, Implementation, Tool};
use rmcp::service::{RunningService, ServerSink};
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::RoleClient;
use rmcp::ServiceExt;

pub struct McpRegistry {
    tools: Vec<Tool>,
    peer: ServerSink,
    // Kept solely to keep the connection alive: the `peer` the agent's tools clone is only
    // valid while the running service lives. Dropping the registry tears down the session.
    #[allow(dead_code)]
    service: RunningService<RoleClient, ClientInfo>,
}

impl McpRegistry {
    /// Connect to the backend MCP proxy with the device session bearer, list its tools.
    /// `auth_header(token)` makes rmcp send `Authorization: Bearer {token}` on every leg
    /// (POST and the device's own SSE leg), which is the session the proxy validates.
    pub async fn connect(
        db: &Database,
        backend: &BackendClient,
    ) -> Result<Self, BackendError> {
        let token = backend.session(db).await?;
        let config =
            StreamableHttpClientTransportConfig::with_uri(backend.mcp_url())
                .auth_header(token);
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
