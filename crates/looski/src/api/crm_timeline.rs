// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::{application::ActorContext, crm::timeline};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/timeline",operation_id="crm_timeline",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),timeline::TimelineQuery),responses((status=200,body=timeline::TimelinePage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn crm_timeline(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<timeline::TimelineQuery>,
) -> Result<Json<timeline::TimelinePage>, (StatusCode, Json<ApiError>)> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(timeline::list(&state.db, &actor, query).await?))
}
pub(super) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(crm_timeline))
}
