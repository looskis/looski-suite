// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::{application::ActorContext, crm::task_links};
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
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}/tasks",operation_id="crm_opportunity_tasks",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),task_links::TaskLinkQuery),responses((status=200,body=task_links::TaskLinkPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_opportunity_tasks(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<task_links::TaskLinkQuery>,
) -> Result<Json<task_links::TaskLinkPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = task_links::list(
        &state.db,
        &actor,
        task_links::Target::Opportunity,
        id,
        query,
    )
    .await?;
    Ok(Json(result))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}/tasks",operation_id="crm_link_opportunity_task",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=task_links::LinkTask,responses((status=201,body=task_links::TaskLink),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_link_opportunity_task(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<task_links::LinkTask>,
) -> Result<(StatusCode, Json<task_links::TaskLink>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = task_links::link(
        &state.db,
        &actor,
        task_links::Target::Opportunity,
        id,
        key(&headers)?,
        input,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(result)))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}/followups",operation_id="crm_create_opportunity_followup",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=task_links::CreateFollowup,responses((status=201,body=task_links::TaskLink),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_create_opportunity_followup(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<task_links::CreateFollowup>,
) -> Result<(StatusCode, Json<task_links::TaskLink>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = task_links::create(
        &state.db,
        &actor,
        task_links::Target::Opportunity,
        id,
        key(&headers)?,
        input,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(result)))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}/tasks/{link_id}",operation_id="crm_end_opportunity_task",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("link_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=task_links::EndTaskLink,responses((status=200,body=task_links::TaskLink),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_end_opportunity_task(
    State(state): State<AppState>,
    Path((workspace, id, link_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<task_links::EndTaskLink>,
) -> Result<Json<task_links::TaskLink>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = task_links::end(
        &state.db,
        &actor,
        task_links::Target::Opportunity,
        id,
        link_id,
        key(&headers)?,
        input,
    )
    .await?;
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}/tasks/{link_id}/history",operation_id="crm_opportunity_task_history",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("link_id"=Uuid,Path),crate::crm::HistoryQuery),responses((status=200,body=task_links::TaskLinkHistory),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_opportunity_task_history(
    State(state): State<AppState>,
    Path((workspace, id, link_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<crate::crm::HistoryQuery>,
) -> Result<Json<task_links::TaskLinkHistory>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = task_links::history(
        &state.db,
        &actor,
        task_links::Target::Opportunity,
        id,
        link_id,
        query,
    )
    .await?;
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/deals/{id}/tasks",operation_id="crm_deal_tasks",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),task_links::TaskLinkQuery),responses((status=200,body=task_links::TaskLinkPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_deal_tasks(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<task_links::TaskLinkQuery>,
) -> Result<Json<task_links::TaskLinkPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = task_links::list(&state.db, &actor, task_links::Target::Deal, id, query).await?;
    Ok(Json(result))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/crm/deals/{id}/tasks",operation_id="crm_link_deal_task",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=task_links::LinkTask,responses((status=201,body=task_links::TaskLink),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_link_deal_task(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<task_links::LinkTask>,
) -> Result<(StatusCode, Json<task_links::TaskLink>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = task_links::link(
        &state.db,
        &actor,
        task_links::Target::Deal,
        id,
        key(&headers)?,
        input,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(result)))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/crm/deals/{id}/followups",operation_id="crm_create_deal_followup",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=task_links::CreateFollowup,responses((status=201,body=task_links::TaskLink),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_create_deal_followup(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<task_links::CreateFollowup>,
) -> Result<(StatusCode, Json<task_links::TaskLink>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = task_links::create(
        &state.db,
        &actor,
        task_links::Target::Deal,
        id,
        key(&headers)?,
        input,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(result)))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/crm/deals/{id}/tasks/{link_id}",operation_id="crm_end_deal_task",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("link_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=task_links::EndTaskLink,responses((status=200,body=task_links::TaskLink),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_end_deal_task(
    State(state): State<AppState>,
    Path((workspace, id, link_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<task_links::EndTaskLink>,
) -> Result<Json<task_links::TaskLink>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = task_links::end(
        &state.db,
        &actor,
        task_links::Target::Deal,
        id,
        link_id,
        key(&headers)?,
        input,
    )
    .await?;
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/deals/{id}/tasks/{link_id}/history",operation_id="crm_deal_task_history",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("link_id"=Uuid,Path),crate::crm::HistoryQuery),responses((status=200,body=task_links::TaskLinkHistory),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_deal_task_history(
    State(state): State<AppState>,
    Path((workspace, id, link_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<crate::crm::HistoryQuery>,
) -> Result<Json<task_links::TaskLinkHistory>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = task_links::history(
        &state.db,
        &actor,
        task_links::Target::Deal,
        id,
        link_id,
        query,
    )
    .await?;
    Ok(Json(result))
}
pub(super) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(crm_opportunity_tasks))
        .routes(routes!(crm_link_opportunity_task))
        .routes(routes!(crm_create_opportunity_followup))
        .routes(routes!(crm_end_opportunity_task))
        .routes(routes!(crm_opportunity_task_history))
        .routes(routes!(crm_deal_tasks))
        .routes(routes!(crm_link_deal_task))
        .routes(routes!(crm_create_deal_followup))
        .routes(routes!(crm_end_deal_task))
        .routes(routes!(crm_deal_task_history))
}
