// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::{
    application::{ActorContext, OperationError},
    time::{self, *},
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
type Failure = (StatusCode, Json<ApiError>);
fn key(h: &HeaderMap) -> Result<&str, OperationError> {
    h.get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .ok_or(OperationError::Invalid("Idempotency-Key is required"))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/time/entries",operation_id="time_list",tag="time",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),TimeQuery),responses((status=200,body=TimeEntryPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn list(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Query(query): Query<TimeQuery>,
) -> Result<Json<TimeEntryPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(time::list(&state.db, &actor, query).await?))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/time/entries",operation_id="time_create",tag="time",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveTimeEntry,responses((status=200,body=TimeEntry),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn create(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Json(input): Json<SaveTimeEntry>,
) -> Result<Json<TimeEntry>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        time::save(&state.db, &actor, None, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/time/entries/{id}",operation_id="time_get",tag="time",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path)),responses((status=200,body=TimeEntry),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn get(
    State(state): State<AppState>,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
) -> Result<Json<TimeEntry>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(time::get(&state.db, &actor, id).await?))
}
#[utoipa::path(put,path="/api/v1/workspaces/{workspace_id}/time/entries/{id}",operation_id="time_update",tag="time",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveTimeEntry,responses((status=200,body=TimeEntry),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn update(
    State(state): State<AppState>,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Json(input): Json<SaveTimeEntry>,
) -> Result<Json<TimeEntry>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        time::save(&state.db, &actor, Some(id), key(&headers)?, input).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/time/entries/{id}/transition",operation_id="time_transition",tag="time",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=TimerTransition,responses((status=200,body=TimeEntry),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn transition(
    State(state): State<AppState>,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Json(input): Json<TimerTransition>,
) -> Result<Json<TimeEntry>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        time::transition(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/time/report",operation_id="time_report",tag="time",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),TimeQuery),responses((status=200,body=TimeReport),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn report(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Query(query): Query<TimeQuery>,
) -> Result<Json<TimeReport>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(time::report(&state.db, &actor, query).await?))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/time/budgets",operation_id="time_budgets",tag="time",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),TimeQuery),responses((status=200,body=TimeBudgetPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn budgets(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Query(query): Query<TimeQuery>,
) -> Result<Json<TimeBudgetPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(time::budgets(&state.db, &actor, query).await?))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/time/budgets",operation_id="time_create_budget",tag="time",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveTimeBudget,responses((status=200,body=TimeBudget),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn create_budget(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Json(input): Json<SaveTimeBudget>,
) -> Result<Json<TimeBudget>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        time::save_budget(&state.db, &actor, None, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(put,path="/api/v1/workspaces/{workspace_id}/time/budgets/{id}",operation_id="time_update_budget",tag="time",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveTimeBudget,responses((status=200,body=TimeBudget),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn update_budget(
    State(state): State<AppState>,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Json(input): Json<SaveTimeBudget>,
) -> Result<Json<TimeBudget>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        time::save_budget(&state.db, &actor, Some(id), key(&headers)?, input).await?,
    ))
}
pub(super) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(get, update))
        .routes(routes!(transition))
        .routes(routes!(report))
        .routes(routes!(budgets, create_budget))
        .routes(routes!(update_budget))
}
