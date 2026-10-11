// SPDX-License-Identifier: AGPL-3.0-only
use super::{McpServer, business_result};
use crate::scheduling::*;
use rmcp::{
    ErrorData, handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;
#[derive(Deserialize, JsonSchema)]
pub(super) struct TypesArgs {
    workspace_id: Uuid,
    #[serde(flatten)]
    query: SchedulingQuery,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct CreateTypeArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    input: SaveAppointmentType,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct GetTypeArgs {
    workspace_id: Uuid,
    id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct UpdateTypeArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    input: SaveAppointmentType,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ListArgs {
    workspace_id: Uuid,
    #[serde(flatten)]
    query: SchedulingQuery,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct GetArgs {
    workspace_id: Uuid,
    id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ChangeArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    input: ChangeBooking,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct RetryArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    input: RetryBooking,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct EventsArgs {
    workspace_id: Uuid,
    id: Uuid,
    #[serde(flatten)]
    query: SchedulingQuery,
}
#[tool_router(router=scheduling_tools,vis="pub(super)")]
impl McpServer {
    #[tool(description = "Types scheduling records using current permissions.")]
    async fn scheduling_types(
        &self,
        Parameters(a): Parameters<TypesArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.scheduling_types(a.workspace_id, &a.query).await)
    }
    #[tool(description = "Create type scheduling records using current permissions.")]
    async fn scheduling_create_type(
        &self,
        Parameters(a): Parameters<CreateTypeArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .scheduling_create_type(a.workspace_id, &a.idempotency_key, &a.input)
                .await,
        )
    }
    #[tool(description = "Get type scheduling records using current permissions.")]
    async fn scheduling_get_type(
        &self,
        Parameters(a): Parameters<GetTypeArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.scheduling_get_type(a.workspace_id, a.id).await)
    }
    #[tool(description = "Update type scheduling records using current permissions.")]
    async fn scheduling_update_type(
        &self,
        Parameters(a): Parameters<UpdateTypeArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .scheduling_update_type(a.workspace_id, a.id, &a.idempotency_key, &a.input)
                .await,
        )
    }
    #[tool(description = "List scheduling records using current permissions.")]
    async fn scheduling_list(
        &self,
        Parameters(a): Parameters<ListArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.scheduling_list(a.workspace_id, &a.query).await)
    }
    #[tool(description = "Get scheduling records using current permissions.")]
    async fn scheduling_get(
        &self,
        Parameters(a): Parameters<GetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.scheduling_get(a.workspace_id, a.id).await)
    }
    #[tool(description = "Change scheduling records using current permissions.")]
    async fn scheduling_change(
        &self,
        Parameters(a): Parameters<ChangeArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .scheduling_change(a.workspace_id, a.id, &a.idempotency_key, &a.input)
                .await,
        )
    }
    #[tool(description = "Retry scheduling records using current permissions.")]
    async fn scheduling_retry(
        &self,
        Parameters(a): Parameters<RetryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .scheduling_retry(a.workspace_id, a.id, &a.idempotency_key, &a.input)
                .await,
        )
    }
    #[tool(description = "Events scheduling records using current permissions.")]
    async fn scheduling_events(
        &self,
        Parameters(a): Parameters<EventsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .scheduling_events(a.workspace_id, a.id, &a.query)
                .await,
        )
    }
}
