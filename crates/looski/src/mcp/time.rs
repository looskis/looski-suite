// SPDX-License-Identifier: AGPL-3.0-only
use super::{McpServer, business_result};
use crate::time::*;
use rmcp::{
    ErrorData, handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;
#[derive(Deserialize, JsonSchema)]
pub(super) struct ListArgs {
    workspace_id: Uuid,
    #[serde(flatten)]
    query: TimeQuery,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct CreateArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    input: SaveTimeEntry,
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
    input: SaveTimeEntry,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct TransitionArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    input: TimerTransition,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ReportArgs {
    workspace_id: Uuid,
    #[serde(flatten)]
    query: TimeQuery,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct BudgetsArgs {
    workspace_id: Uuid,
    #[serde(flatten)]
    query: TimeQuery,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct CreateBudgetArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    input: SaveTimeBudget,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct UpdateBudgetArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    input: SaveTimeBudget,
}
#[tool_router(router=time_tools,vis="pub(super)")]
impl McpServer {
    #[tool(description = "List time records using current permissions.")]
    async fn time_list(
        &self,
        Parameters(a): Parameters<ListArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.time_list(a.workspace_id, &a.query).await)
    }
    #[tool(description = "Create time records using current permissions.")]
    async fn time_create(
        &self,
        Parameters(a): Parameters<CreateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .time_create(a.workspace_id, &a.idempotency_key, &a.input)
                .await,
        )
    }
    #[tool(description = "Get time records using current permissions.")]
    async fn time_get(
        &self,
        Parameters(a): Parameters<GetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.time_get(a.workspace_id, a.id).await)
    }
    #[tool(description = "Update time records using current permissions.")]
    async fn time_update(
        &self,
        Parameters(a): Parameters<UpdateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .time_update(a.workspace_id, a.id, &a.idempotency_key, &a.input)
                .await,
        )
    }
    #[tool(description = "TimerTransition time records using current permissions.")]
    async fn time_transition(
        &self,
        Parameters(a): Parameters<TransitionArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .time_transition(a.workspace_id, a.id, &a.idempotency_key, &a.input)
                .await,
        )
    }
    #[tool(description = "TimeReport time records using current permissions.")]
    async fn time_report(
        &self,
        Parameters(a): Parameters<ReportArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.time_report(a.workspace_id, &a.query).await)
    }
    #[tool(description = "Budgets time records using current permissions.")]
    async fn time_budgets(
        &self,
        Parameters(a): Parameters<BudgetsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.time_budgets(a.workspace_id, &a.query).await)
    }
    #[tool(description = "Create budget time records using current permissions.")]
    async fn time_create_budget(
        &self,
        Parameters(a): Parameters<CreateBudgetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .time_create_budget(a.workspace_id, &a.idempotency_key, &a.input)
                .await,
        )
    }
    #[tool(description = "Update budget time records using current permissions.")]
    async fn time_update_budget(
        &self,
        Parameters(a): Parameters<UpdateBudgetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .time_update_budget(a.workspace_id, a.id, &a.idempotency_key, &a.input)
                .await,
        )
    }
}
