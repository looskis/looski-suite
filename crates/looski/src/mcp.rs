// SPDX-License-Identifier: AGPL-3.0-only
use crate::client::ApiClient;
use rmcp::{
    ErrorData, ServerHandler,
    handler::server::router::tool::ToolRouter,
    model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig},
    tool, tool_handler, tool_router,
};

use rmcp::handler::server::wrapper::Parameters;
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;
#[derive(Deserialize, JsonSchema)]
struct ContactListArgs {
    workspace_id: Uuid,
    #[serde(flatten)]
    query: crate::contacts::ListContacts,
}
#[derive(Deserialize, JsonSchema)]
struct ContactGetArgs {
    workspace_id: Uuid,
    contact_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
struct ContactCreateArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    contact: crate::contacts::ContactInput,
}
#[derive(Deserialize, JsonSchema)]
struct ContactEditArgs {
    workspace_id: Uuid,
    contact_id: Uuid,
    idempotency_key: String,
    edit: crate::contacts::EditContact,
}
#[derive(Deserialize, JsonSchema)]
struct ContactArchiveArgs {
    workspace_id: Uuid,
    contact_id: Uuid,
    idempotency_key: String,
    change: crate::contacts::ArchiveContact,
}
#[derive(Deserialize, JsonSchema)]
struct ContactMergeArgs {
    workspace_id: Uuid,
    survivor_id: Uuid,
    idempotency_key: String,
    merge: crate::contacts::MergeContacts,
}
fn business_result<T: serde::Serialize>(
    result: anyhow::Result<T>,
) -> Result<CallToolResult, ErrorData> {
    match result {
        Ok(value) => Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string(&value)
                .map_err(|_| ErrorData::internal_error("Could not encode result", None))?,
        )])),
        Err(error) => {
            let mut result = CallToolResult::error(vec![ContentBlock::text(error.to_string())]);
            if let Some(failure) = error.downcast_ref::<crate::client::ApiFailure>() {
                result.structured_content =
                    Some(serde_json::to_value(failure).map_err(|_| {
                        ErrorData::internal_error("Could not encode failure", None)
                    })?);
            }
            Ok(result)
        }
    }
}

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
    #[tool(
        description = "List permitted workspace contacts with bounded cursor pagination and optional name search."
    )]
    async fn list_contacts(
        &self,
        Parameters(args): Parameters<ContactListArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.contacts(args.workspace_id, &args.query).await)
    }
    #[tool(description = "Read a permitted contact by its stable ID.")]
    async fn get_contact(
        &self,
        Parameters(args): Parameters<ContactGetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .contact(args.workspace_id, args.contact_id)
                .await,
        )
    }
    #[tool(
        description = "Create a contact. Supply a stable idempotency key for retries; matching identifiers require review. allow_shared requires explicit reviewed sharing and permission."
    )]
    async fn create_contact(
        &self,
        Parameters(args): Parameters<ContactCreateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .create_contact(args.workspace_id, &args.idempotency_key, &args.contact)
                .await,
        )
    }
    #[tool(
        description = "Replace editable contact fields using expected_version and an idempotency key. Omitted identifiers are removed from active association, with history retained."
    )]
    async fn update_contact(
        &self,
        Parameters(args): Parameters<ContactEditArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .edit_contact(
                    args.workspace_id,
                    args.contact_id,
                    &args.idempotency_key,
                    &args.edit,
                )
                .await,
        )
    }
    #[tool(
        description = "Archive or restore a contact with expected_version and an idempotency key. Does not permanently delete records."
    )]
    async fn archive_contact(
        &self,
        Parameters(args): Parameters<ContactArchiveArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .archive_contact(
                    args.workspace_id,
                    args.contact_id,
                    &args.idempotency_key,
                    &args.change,
                )
                .await,
        )
    }
    #[tool(
        description = "Merge confirmed duplicate contacts into an explicitly chosen survivor. Requires both record versions and resolved fields; preserves identifiers/history and old-ID aliases. Never use shared email alone as proof of identity."
    )]
    async fn merge_contacts(
        &self,
        Parameters(args): Parameters<ContactMergeArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .merge_contacts(
                    args.workspace_id,
                    args.survivor_id,
                    &args.idempotency_key,
                    &args.merge,
                )
                .await,
        )
    }
    pub fn new(client: ApiClient) -> Self {
        Self {
            client: client.for_mcp(),
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
            .with_instructions("system_status is public. Business tools use LOOSKI_ACCESS_TOKEN_FILE and the same authorization as the HTTP API. No credentials are accepted as tool arguments.")
    }
}
