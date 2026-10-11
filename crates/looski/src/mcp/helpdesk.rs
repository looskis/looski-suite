// SPDX-License-Identifier: AGPL-3.0-only
use super::{McpServer, business_result};
use crate::helpdesk::*;
use rmcp::{
    ErrorData, handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;
#[derive(Deserialize, JsonSchema)]
pub(super) struct SettingsArgs {
    workspace_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ConfigureArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    input: ConfigureHelpdesk,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct DirectoryArgs {
    workspace_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ListArgs {
    workspace_id: Uuid,
    #[serde(flatten)]
    query: TicketQuery,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct GetArgs {
    workspace_id: Uuid,
    ticket_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct UpdateArgs {
    workspace_id: Uuid,
    ticket_id: Uuid,
    idempotency_key: String,
    input: UpdateTicket,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct LinkArgs {
    workspace_id: Uuid,
    ticket_id: Uuid,
    idempotency_key: String,
    input: LinkTicket,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct MessagesArgs {
    workspace_id: Uuid,
    ticket_id: Uuid,
    #[serde(flatten)]
    query: MessageQuery,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ReplyArgs {
    workspace_id: Uuid,
    ticket_id: Uuid,
    idempotency_key: String,
    input: ReplyTicket,
}
#[tool_router(router=helpdesk_tools,vis="pub(super)")]
impl McpServer {
    #[tool(description = "Settings helpdesk records under current staff permissions.")]
    async fn helpdesk_settings(
        &self,
        Parameters(a): Parameters<SettingsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.helpdesk_settings(a.workspace_id).await)
    }
    #[tool(description = "Configure helpdesk records under current staff permissions.")]
    async fn helpdesk_configure(
        &self,
        Parameters(a): Parameters<ConfigureArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .helpdesk_configure(a.workspace_id, &a.idempotency_key, &a.input)
                .await,
        )
    }
    #[tool(description = "Directory helpdesk records under current staff permissions.")]
    async fn helpdesk_directory(
        &self,
        Parameters(a): Parameters<DirectoryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.helpdesk_directory(a.workspace_id).await)
    }
    #[tool(description = "List helpdesk records under current staff permissions.")]
    async fn helpdesk_list(
        &self,
        Parameters(a): Parameters<ListArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.helpdesk_list(a.workspace_id, &a.query).await)
    }
    #[tool(description = "Get helpdesk records under current staff permissions.")]
    async fn helpdesk_get(
        &self,
        Parameters(a): Parameters<GetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.helpdesk_get(a.workspace_id, a.ticket_id).await)
    }
    #[tool(
        description = "Set status, assignee, archive or permanently revoke customer access; expected_version required."
    )]
    async fn helpdesk_update(
        &self,
        Parameters(a): Parameters<UpdateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .helpdesk_update(a.workspace_id, a.ticket_id, &a.idempotency_key, &a.input)
                .await,
        )
    }
    #[tool(description = "Set independently authorized contact and staff-only article references.")]
    async fn helpdesk_link(
        &self,
        Parameters(a): Parameters<LinkArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .helpdesk_link(a.workspace_id, a.ticket_id, &a.idempotency_key, &a.input)
                .await,
        )
    }
    #[tool(description = "Messages helpdesk records under current staff permissions.")]
    async fn helpdesk_messages(
        &self,
        Parameters(a): Parameters<MessagesArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .helpdesk_messages(a.workspace_id, a.ticket_id, &a.query)
                .await,
        )
    }
    #[tool(description = "Append a public reply or private internal note. Messages are immutable.")]
    async fn helpdesk_reply(
        &self,
        Parameters(a): Parameters<ReplyArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .helpdesk_reply(a.workspace_id, a.ticket_id, &a.idempotency_key, &a.input)
                .await,
        )
    }
}
