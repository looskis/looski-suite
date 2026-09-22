// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::{
    application::ActorContext,
    contacts::{
        Contact,
        intake::{self, Intake, IntakePage, ListIntakes, ResolveIntake, SubmitIntake},
    },
};
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
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/contact-intakes",operation_id="listContactIntakes",tag="intake",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),ListIntakes),responses((status=200,body=IntakePage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn list(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(input): Query<ListIntakes>,
) -> Result<Json<IntakePage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(intake::list(&state.db, &actor, input).await?))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/contact-intakes/{intake_id}",operation_id="getContactIntake",tag="intake",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("intake_id"=Uuid,Path)),responses((status=200,body=Intake),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn get(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Intake>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(intake::get(&state.db, &actor, id).await?))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/contact-intakes",operation_id="submitContactIntake",tag="intake",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("Idempotency-Key"=String,Header)),request_body=SubmitIntake,responses((status=201,body=Intake),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn submit(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SubmitIntake>,
) -> Result<(StatusCode, Json<Intake>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok((
        StatusCode::CREATED,
        Json(intake::submit(&state.db, &actor, key(&headers)?, input).await?),
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/contact-intakes/{intake_id}/resolve",operation_id="resolveContactIntake",tag="intake",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("intake_id"=Uuid,Path), ("Idempotency-Key"=String,Header)),request_body=ResolveIntake,responses((status=200,body=Intake),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn resolve(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<ResolveIntake>,
) -> Result<Json<Intake>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        intake::resolve(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/contact-intakes/{intake_id}/candidates",operation_id="listIntakeCandidates",tag="intake",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("intake_id"=Uuid,Path)),responses((status=200,body=Vec<Contact>),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn candidates(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Vec<Contact>>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(intake::candidates(&state.db, &actor, id).await?))
}
pub(super) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list, submit))
        .routes(routes!(get))
        .routes(routes!(resolve))
        .routes(routes!(candidates))
}
