// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::{
    application::ActorContext,
    exports::{self, CollectionSpec, ExportPage, ExportRequest},
};
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
type Failure = (StatusCode, Json<ApiError>);
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/exports/collections",operation_id="exportCollections",tag="exports",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path)),responses((status=200,body=Vec<CollectionSpec>),(status=401,body=ApiError),(status=403,body=ApiError),(status=503,body=ApiError)))]
async fn catalog(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Vec<CollectionSpec>>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(exports::catalog(&state.db, &actor).await?))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/exports/page",operation_id="exportPage",tag="exports",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path)),request_body=ExportRequest,responses((status=200,body=ExportPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn page(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<ExportRequest>,
) -> Result<Json<ExportPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(exports::page(&state.db, &actor, input).await?))
}
pub(super) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(catalog, page))
}
