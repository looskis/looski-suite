// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::{application::ActorContext, crm::coordination};
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
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/coordination-templates",operation_id="crm_coordination_templates",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),coordination::CoordinationQuery),responses((status=200,body=coordination::CoordinationTemplatePage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_coordination_templates(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<coordination::CoordinationQuery>,
) -> Result<Json<coordination::CoordinationTemplatePage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = coordination::templates(&state.db, &actor, query).await?;
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/coordination-templates/{id}",operation_id="crm_coordination_template",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path)),responses((status=200,body=coordination::CoordinationTemplate),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_coordination_template(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<coordination::CoordinationTemplate>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = coordination::template(&state.db, &actor, id).await?;
    Ok(Json(result))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/crm/coordination-templates",operation_id="crm_create_coordination_template",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=coordination::SaveCoordinationTemplate,responses((status=201,body=coordination::CoordinationTemplate),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_create_coordination_template(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<coordination::SaveCoordinationTemplate>,
) -> Result<(StatusCode, Json<coordination::CoordinationTemplate>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = coordination::save(&state.db, &actor, None, key(&headers)?, input).await?;
    Ok((StatusCode::CREATED, Json(result)))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/crm/coordination-templates/{id}",operation_id="crm_edit_coordination_template",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=coordination::SaveCoordinationTemplate,responses((status=200,body=coordination::CoordinationTemplate),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_edit_coordination_template(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<coordination::SaveCoordinationTemplate>,
) -> Result<Json<coordination::CoordinationTemplate>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = coordination::save(&state.db, &actor, Some(id), key(&headers)?, input).await?;
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/coordination-templates/{id}/revisions",operation_id="crm_coordination_revisions",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),coordination::CoordinationPageQuery),responses((status=200,body=coordination::CoordinationRevisionPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_coordination_revisions(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<coordination::CoordinationPageQuery>,
) -> Result<Json<coordination::CoordinationRevisionPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = coordination::revisions(&state.db, &actor, id, query).await?;
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/deals/{id}/coordination",operation_id="crm_deal_coordination",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),coordination::CoordinationPageQuery),responses((status=200,body=coordination::CoordinationInstancePage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_deal_coordination(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<coordination::CoordinationPageQuery>,
) -> Result<Json<coordination::CoordinationInstancePage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = coordination::instances(&state.db, &actor, id, query).await?;
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/crm/deals/{id}/coordination/{instance_id}",operation_id="crm_deal_coordination_instance",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("instance_id"=Uuid,Path)),responses((status=200,body=coordination::CoordinationInstance),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_deal_coordination_instance(
    State(state): State<AppState>,
    Path((workspace, id, instance_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<coordination::CoordinationInstance>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = coordination::instance(&state.db, &actor, id, instance_id).await?;
    Ok(Json(result))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/crm/deals/{id}/coordination",operation_id="crm_apply_coordination",tag="crm",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=coordination::ApplyCoordination,responses((status=201,body=coordination::CoordinationInstance),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn crm_apply_coordination(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<coordination::ApplyCoordination>,
) -> Result<(StatusCode, Json<coordination::CoordinationInstance>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = coordination::apply(&state.db, &actor, id, key(&headers)?, input).await?;
    Ok((StatusCode::CREATED, Json(result)))
}
pub(super) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(crm_coordination_templates))
        .routes(routes!(crm_coordination_template))
        .routes(routes!(crm_create_coordination_template))
        .routes(routes!(crm_edit_coordination_template))
        .routes(routes!(crm_coordination_revisions))
        .routes(routes!(crm_deal_coordination))
        .routes(routes!(crm_deal_coordination_instance))
        .routes(routes!(crm_apply_coordination))
}
