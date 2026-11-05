// SPDX-License-Identifier: AGPL-3.0-only
use super::{McpServer, business_result};
use crate::visitors::*;
use rmcp::{
    ErrorData, handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;
#[derive(Deserialize, JsonSchema)]
pub(super) struct SitesArgs {
    workspace_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct SaveSiteArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    input: SaveSite,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ListArgs {
    workspace_id: Uuid,
    #[serde(flatten)]
    query: Query,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct GetArgs {
    workspace_id: Uuid,
    id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct EventsArgs {
    workspace_id: Uuid,
    id: Uuid,
    #[serde(flatten)]
    query: EventsQuery,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ResolveArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    input: Resolve,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ReviewArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    input: Review,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct LinkArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
    input: Link,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct DeleteArgs {
    workspace_id: Uuid,
    id: Uuid,
    idempotency_key: String,
}
#[tool_router(router=visitors_tools,vis="pub(super)")]
impl McpServer {
    #[tool(
        description = "Visitor sites; workspace permissions apply. Matches never grant identity or access."
    )]
    async fn visitors_sites(
        &self,
        Parameters(a): Parameters<SitesArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.visitors_sites(a.workspace_id).await)
    }
    #[tool(
        description = "Visitor save site; workspace permissions apply. Matches never grant identity or access."
    )]
    async fn visitors_save_site(
        &self,
        Parameters(a): Parameters<SaveSiteArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .visitors_save_site(a.workspace_id, &a.idempotency_key, &a.input)
                .await,
        )
    }
    #[tool(
        description = "Visitor list; workspace permissions apply. Matches never grant identity or access."
    )]
    async fn visitors_list(
        &self,
        Parameters(a): Parameters<ListArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.visitors_list(a.workspace_id, &a.query).await)
    }
    #[tool(
        description = "Visitor get; workspace permissions apply. Matches never grant identity or access."
    )]
    async fn visitors_get(
        &self,
        Parameters(a): Parameters<GetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.visitors_get(a.workspace_id, a.id).await)
    }
    #[tool(
        description = "Visitor events; workspace permissions apply. Matches never grant identity or access."
    )]
    async fn visitors_events(
        &self,
        Parameters(a): Parameters<EventsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .visitors_events(a.workspace_id, a.id, &a.query)
                .await,
        )
    }
    #[tool(
        description = "Visitor resolve; workspace permissions apply. Matches never grant identity or access."
    )]
    async fn visitors_resolve(
        &self,
        Parameters(a): Parameters<ResolveArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .visitors_resolve(a.workspace_id, &a.idempotency_key, &a.input)
                .await,
        )
    }
    #[tool(
        description = "Visitor review; workspace permissions apply. Matches never grant identity or access."
    )]
    async fn visitors_review(
        &self,
        Parameters(a): Parameters<ReviewArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .visitors_review(a.workspace_id, a.id, &a.idempotency_key, &a.input)
                .await,
        )
    }
    #[tool(
        description = "Visitor link; workspace permissions apply. Matches never grant identity or access."
    )]
    async fn visitors_link(
        &self,
        Parameters(a): Parameters<LinkArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .visitors_link(a.workspace_id, a.id, &a.idempotency_key, &a.input)
                .await,
        )
    }
    #[tool(
        description = "Visitor delete; workspace permissions apply. Matches never grant identity or access."
    )]
    async fn visitors_delete(
        &self,
        Parameters(a): Parameters<DeleteArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .visitors_delete(a.workspace_id, a.id, &a.idempotency_key)
                .await,
        )
    }
}
