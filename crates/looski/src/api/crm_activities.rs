// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::{application::ActorContext, crm::activities};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
type Failure = (StatusCode, Json<ApiError>);
fn key(headers: &HeaderMap) -> Result<&str, crate::application::OperationError> {
    headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .ok_or(crate::application::OperationError::Invalid(
            "Idempotency-Key is required for mutations",
        ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/activity-types",operation_id="crm_activity_types",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),activities::types::ActivityTypeQuery),responses((status=200,body=activities::types::ActivityTypePage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_activity_types(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<activities::types::ActivityTypeQuery>,
) -> Result<Json<activities::types::ActivityTypePage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = activities::types::list(&state.db, &actor, query).await?;
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/activity-types/{id}",operation_id="crm_activity_type",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path)),responses((status=200,body=activities::types::ActivityType),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_activity_type(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<activities::types::ActivityType>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = activities::types::get(&state.db, &actor, id).await?;
    Ok(Json(result))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/crm/activity-types",operation_id="crm_create_activity_type",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=activities::types::SaveActivityType,responses((status=201,body=activities::types::ActivityType),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_create_activity_type(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<activities::types::SaveActivityType>,
) -> Result<(StatusCode, Json<activities::types::ActivityType>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = activities::types::save(&state.db, &actor, None, key(&headers)?, input).await?;
    Ok((StatusCode::CREATED, Json(result)))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/crm/activity-types/{id}",operation_id="crm_edit_activity_type",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=activities::types::SaveActivityType,responses((status=200,body=activities::types::ActivityType),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_edit_activity_type(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<activities::types::SaveActivityType>,
) -> Result<Json<activities::types::ActivityType>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result =
        activities::types::save(&state.db, &actor, Some(id), key(&headers)?, input).await?;
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/activities",operation_id="crm_activities",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),activities::ActivityQuery),responses((status=200,body=activities::ActivityPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_activities(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<activities::ActivityQuery>,
) -> Result<Json<activities::ActivityPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = activities::list(&state.db, &actor, query).await?;
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/activities/{id}",operation_id="crm_activity",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path)),responses((status=200,body=activities::Activity),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_activity(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<activities::Activity>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = activities::get(&state.db, &actor, id).await?;
    Ok(Json(result))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/crm/activities",operation_id="crm_create_activity",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=activities::CreateActivity,responses((status=201,body=activities::Activity),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_create_activity(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<activities::CreateActivity>,
) -> Result<(StatusCode, Json<activities::Activity>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = activities::create(&state.db, &actor, key(&headers)?, input).await?;
    Ok((StatusCode::CREATED, Json(result)))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/crm/activities/{id}",operation_id="crm_edit_activity",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=activities::EditActivity,responses((status=200,body=activities::Activity),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_edit_activity(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<activities::EditActivity>,
) -> Result<Json<activities::Activity>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = activities::edit(&state.db, &actor, id, key(&headers)?, input).await?;
    Ok(Json(result))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/crm/activities/{id}/archive",operation_id="crm_archive_activity",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=activities::ArchiveActivity,responses((status=200,body=activities::Activity),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_archive_activity(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<activities::ArchiveActivity>,
) -> Result<Json<activities::Activity>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = activities::archive(&state.db, &actor, id, key(&headers)?, input).await?;
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/activities/{id}/history",operation_id="crm_activity_history",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),activities::ActivityHistoryQuery),responses((status=200,body=activities::ActivityHistoryPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_activity_history(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<activities::ActivityHistoryQuery>,
) -> Result<Json<activities::ActivityHistoryPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = activities::history(&state.db, &actor, id, query).await?;
    Ok(Json(result))
}
pub(super) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(crm_activity_types))
        .routes(routes!(crm_activity_type))
        .routes(routes!(crm_create_activity_type))
        .routes(routes!(crm_edit_activity_type))
        .routes(routes!(crm_activities))
        .routes(routes!(crm_activity))
        .routes(routes!(crm_create_activity))
        .routes(routes!(crm_edit_activity))
        .routes(routes!(crm_archive_activity))
        .routes(routes!(crm_activity_history))
}
