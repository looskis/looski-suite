// SPDX-License-Identifier: AGPL-3.0-only
use crate::client::ApiClient;
use rmcp::{
    ErrorData, ServerHandler,
    handler::server::router::tool::ToolRouter,
    model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig},
    tool, tool_handler, tool_router,
};

#[derive(Clone)]
pub struct McpServer {
    client: ApiClient,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl McpServer {
    #[tool(
        description = "List workspaces allowed by the configured first-party session and current suite permissions."
    )]
    async fn list_workspaces(&self) -> Result<CallToolResult, ErrorData> {
        match self.client.workspaces().await {
            Ok(workspaces) => Ok(CallToolResult::success(vec![ContentBlock::text(
                serde_json::to_string(&workspaces)
                    .map_err(|_| ErrorData::internal_error("Could not encode workspaces", None))?,
            )])),
            Err(_) => Ok(CallToolResult::error(vec![ContentBlock::text(
                "Workspace access failed. Check the API and the session in LOOSKI_ACCESS_TOKEN_FILE.",
            )])),
        }
    }
    pub fn new(client: ApiClient) -> Self {
        Self {
            client,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Read public Looski installation version and scaffold status. No business data or login access."
    )]
    async fn system_status(&self) -> Result<CallToolResult, ErrorData> {
        match self.client.system().await {
            Ok(info) => Ok(CallToolResult::success(vec![ContentBlock::text(
                serde_json::to_string(&info)
                    .map_err(|_| ErrorData::internal_error("Could not encode status", None))?,
            )])),
            Err(_) => Ok(CallToolResult::error(vec![ContentBlock::text(
                "Looski API is unavailable",
            )])),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("looski-suite", env!("CARGO_PKG_VERSION")))
            .with_instructions("system_status is public. list_workspaces uses LOOSKI_ACCESS_TOKEN_FILE and the same authorization as the HTTP API. No credentials are accepted as tool arguments.")
    }
}
