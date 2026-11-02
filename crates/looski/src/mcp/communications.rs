// SPDX-License-Identifier: AGPL-3.0-only
use super::{McpServer, business_result};
use crate::communications::*;
use rmcp::{
    ErrorData, handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;
#[derive(Deserialize, JsonSchema)]
pub(super) struct AccountsArgs {
    workspace_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ConnectArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    input: ConnectAccount,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct UpdateAccountArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    input: UpdateAccount,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct SyncArgs {
    workspace_id: Uuid,
    id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ListArgs {
    workspace_id: Uuid,
    #[serde(flatten)]
    query: ConversationQuery,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct GetArgs {
    workspace_id: Uuid,
    id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct UpdateArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    input: UpdateConversation,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct MessagesArgs {
    workspace_id: Uuid,
    id: Uuid,
    #[serde(flatten)]
    query: MessageQuery,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct SendArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    input: SendMessage,
}
#[tool_router(router=communications_tools,vis="pub(super)")]
impl McpServer {
    #[tool(description = "Access communications under current account and workspace permissions.")]
    async fn communications_accounts(
        &self,
        Parameters(a): Parameters<AccountsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.communications_accounts(a.workspace_id).await)
    }
    #[tool(
        description = "Connect an operator-provisioned connector. Accounts are private by default."
    )]
    async fn communications_connect(
        &self,
        Parameters(a): Parameters<ConnectArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .communications_connect(a.workspace_id, &a.idempotency_key, &a.input)
                .await,
        )
    }
    #[tool(description = "Access communications under current account and workspace permissions.")]
    async fn communications_update_account(
        &self,
        Parameters(a): Parameters<UpdateAccountArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .communications_update_account(a.workspace_id, a.id, &a.idempotency_key, &a.input)
                .await,
        )
    }
    #[tool(
        description = "Recover one page of incoming provider events for an authorized connection."
    )]
    async fn communications_sync(
        &self,
        Parameters(a): Parameters<SyncArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.communications_sync(a.workspace_id, a.id).await)
    }
    #[tool(description = "Access communications under current account and workspace permissions.")]
    async fn communications_list(
        &self,
        Parameters(a): Parameters<ListArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .communications_list(a.workspace_id, &a.query)
                .await,
        )
    }
    #[tool(description = "Access communications under current account and workspace permissions.")]
    async fn communications_get(
        &self,
        Parameters(a): Parameters<GetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.communications_get(a.workspace_id, a.id).await)
    }
    #[tool(description = "Access communications under current account and workspace permissions.")]
    async fn communications_update(
        &self,
        Parameters(a): Parameters<UpdateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .communications_update(a.workspace_id, a.id, &a.idempotency_key, &a.input)
                .await,
        )
    }
    #[tool(description = "Access communications under current account and workspace permissions.")]
    async fn communications_messages(
        &self,
        Parameters(a): Parameters<MessagesArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .communications_messages(a.workspace_id, a.id, &a.query)
                .await,
        )
    }
    #[tool(
        description = "Queue a real external reply through the conversation account. Idempotency key required; this sends a message."
    )]
    async fn communications_send(
        &self,
        Parameters(a): Parameters<SendArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .communications_send(a.workspace_id, a.id, &a.idempotency_key, &a.input)
                .await,
        )
    }
}
