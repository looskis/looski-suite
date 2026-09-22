// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::{application::ActorContext, crm::status_jobs, jobs};
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
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/jobs",operation_id="jobs_list",tag="jobs",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),jobs::JobQuery),responses((status=200,body=jobs::JobPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn jobs_list(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<jobs::JobQuery>,
) -> Result<Json<jobs::JobPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = jobs::list(&state.db, &actor, query).await?;
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/jobs/{id}",operation_id="jobs_get",tag="jobs",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path)),responses((status=200,body=jobs::Job),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn jobs_get(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<jobs::Job>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = jobs::get(&state.db, &actor, id).await?;
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/jobs/{id}/history",operation_id="jobs_history",tag="jobs",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),jobs::JobQuery),responses((status=200,body=jobs::JobEventPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn jobs_history(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<jobs::JobQuery>,
) -> Result<Json<jobs::JobEventPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = jobs::history(&state.db, &actor, id, query).await?;
    Ok(Json(result))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/jobs/{id}",operation_id="jobs_change",tag="jobs",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=jobs::ChangeJob,responses((status=200,body=jobs::Job),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn jobs_change(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<jobs::ChangeJob>,
) -> Result<Json<jobs::Job>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = jobs::change(&state.db, &actor, id, key(&headers)?, input).await?;
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/coordination-rules",operation_id="crm_coordination_rules",tag="jobs",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),status_jobs::RuleQuery),responses((status=200,body=status_jobs::CoordinationRulePage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_coordination_rules(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<status_jobs::RuleQuery>,
) -> Result<Json<status_jobs::CoordinationRulePage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = status_jobs::list(&state.db, &actor, query).await?;
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/coordination-rules/{id}",operation_id="crm_coordination_rule",tag="jobs",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path)),responses((status=200,body=status_jobs::CoordinationRule),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_coordination_rule(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<status_jobs::CoordinationRule>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = status_jobs::get(&state.db, &actor, id).await?;
    Ok(Json(result))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/crm/coordination-rules",operation_id="crm_create_coordination_rule",tag="jobs",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=status_jobs::SaveCoordinationRule,responses((status=201,body=status_jobs::CoordinationRule),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_create_coordination_rule(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<status_jobs::SaveCoordinationRule>,
) -> Result<(StatusCode, Json<status_jobs::CoordinationRule>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = status_jobs::save(&state.db, &actor, None, key(&headers)?, input).await?;
    Ok((StatusCode::CREATED, Json(result)))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/crm/coordination-rules/{id}",operation_id="crm_edit_coordination_rule",tag="jobs",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=status_jobs::SaveCoordinationRule,responses((status=200,body=status_jobs::CoordinationRule),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_edit_coordination_rule(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<status_jobs::SaveCoordinationRule>,
) -> Result<Json<status_jobs::CoordinationRule>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = status_jobs::save(&state.db, &actor, Some(id), key(&headers)?, input).await?;
    Ok(Json(result))
}
pub(super) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(jobs_list))
        .routes(routes!(jobs_get))
        .routes(routes!(jobs_history))
        .routes(routes!(jobs_change))
        .routes(routes!(crm_coordination_rules))
        .routes(routes!(crm_coordination_rule))
        .routes(routes!(crm_create_coordination_rule))
        .routes(routes!(crm_edit_coordination_rule))
}
