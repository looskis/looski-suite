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
            .with_instructions("Scaffold: only public system_status is available. Business operations and authentication are not implemented yet.")
    }
}
