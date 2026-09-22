// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::{application::ActorContext, tasks};
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
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/tasks",operation_id="tasks_list",tag="tasks",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), tasks::TaskQuery),responses((status=200,body=tasks::TaskPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn tasks_list(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<tasks::TaskQuery>,
) -> Result<Json<tasks::TaskPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = tasks::list(&state.db, &actor, query).await?;
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/tasks/{id}",operation_id="tasks_get",tag="tasks",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("id"=Uuid,Path)),responses((status=200,body=tasks::Task),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn tasks_get(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<tasks::Task>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = tasks::get(&state.db, &actor, id).await?;
    Ok(Json(result))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/tasks",operation_id="tasks_create",tag="tasks",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("Idempotency-Key"=String,Header)),request_body=tasks::TaskInput,responses((status=201,body=tasks::Task),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn tasks_create(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<tasks::TaskInput>,
) -> Result<(StatusCode, Json<tasks::Task>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = tasks::save(&state.db, &actor, None, None, key(&headers)?, input).await?;
    Ok((StatusCode::CREATED, Json(result)))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/tasks/{id}",operation_id="tasks_edit",tag="tasks",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("id"=Uuid,Path), ("Idempotency-Key"=String,Header)),request_body=tasks::EditTask,responses((status=200,body=tasks::Task),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn tasks_edit(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<tasks::EditTask>,
) -> Result<Json<tasks::Task>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = tasks::save(
        &state.db,
        &actor,
        Some(id),
        Some(input.expected_version),
        key(&headers)?,
        input.fields,
    )
    .await?;
    Ok(Json(result))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/tasks/{id}/archive",operation_id="tasks_archive",tag="tasks",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("id"=Uuid,Path), ("Idempotency-Key"=String,Header)),request_body=tasks::ArchiveTask,responses((status=200,body=tasks::Task),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn tasks_archive(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<tasks::ArchiveTask>,
) -> Result<Json<tasks::Task>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = tasks::archive(&state.db, &actor, id, key(&headers)?, input).await?;
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/tasks/{id}/history",operation_id="tasks_history",tag="tasks",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("id"=Uuid,Path), tasks::TaskHistoryQuery),responses((status=200,body=tasks::TaskHistoryPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn tasks_history(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<tasks::TaskHistoryQuery>,
) -> Result<Json<tasks::TaskHistoryPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = tasks::history(&state.db, &actor, id, query).await?;
    Ok(Json(result))
}
pub(super) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(tasks_list))
        .routes(routes!(tasks_get))
        .routes(routes!(tasks_create))
        .routes(routes!(tasks_edit))
        .routes(routes!(tasks_archive))
        .routes(routes!(tasks_history))
}
