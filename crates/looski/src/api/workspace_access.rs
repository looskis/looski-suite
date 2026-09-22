// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::{
    application::ActorContext,
    workspace_access::{
        self, AccessQuery, EditMember, Member, MemberPage, Permission, Role, RolePage, SaveRole,
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
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/access/permissions",operation_id="access_permissions",tag="access",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path)),responses((status=200,body=Vec<Permission>),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn permissions(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Vec<Permission>>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        workspace_access::permissions(&state.db, &actor).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/access/roles",operation_id="access_roles",tag="access",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),AccessQuery),responses((status=200,body=RolePage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn roles(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<AccessQuery>,
) -> Result<Json<RolePage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        workspace_access::roles(&state.db, &actor, query).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/access/roles/{role_id}",operation_id="access_role",tag="access",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("role_id"=Uuid,Path)),responses((status=200,body=Role),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn role(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Role>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(workspace_access::role(&state.db, &actor, id).await?))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/access/roles",operation_id="access_create_role",tag="access",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveRole,responses((status=201,body=Role),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn create_role(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SaveRole>,
) -> Result<(StatusCode, Json<Role>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok((
        StatusCode::CREATED,
        Json(workspace_access::save_role(&state.db, &actor, None, key(&headers)?, input).await?),
    ))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/access/roles/{role_id}",operation_id="access_edit_role",tag="access",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("role_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveRole,responses((status=200,body=Role),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn edit_role(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SaveRole>,
) -> Result<Json<Role>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        workspace_access::save_role(&state.db, &actor, Some(id), key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/access/members",operation_id="access_members",tag="access",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),AccessQuery),responses((status=200,body=MemberPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn members(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<AccessQuery>,
) -> Result<Json<MemberPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        workspace_access::members(&state.db, &actor, query).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/access/members/{member_id}",operation_id="access_member",tag="access",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("member_id"=Uuid,Path)),responses((status=200,body=Member),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn member(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Member>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(workspace_access::member(&state.db, &actor, id).await?))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/access/members/{member_id}",operation_id="access_edit_member",tag="access",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("member_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=EditMember,responses((status=200,body=Member),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn edit_member(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<EditMember>,
) -> Result<Json<Member>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        workspace_access::edit_member(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
pub(super) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(permissions))
        .routes(routes!(roles, create_role))
        .routes(routes!(role, edit_role))
        .routes(routes!(members))
        .routes(routes!(member, edit_member))
}
