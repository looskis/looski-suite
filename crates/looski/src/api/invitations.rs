// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::{
    application::ActorContext,
    workspace_access::{
        AccessQuery, Member,
        invitations::{
            self, AcceptInvitation, CreateInvitation, Invitation, InvitationLink, InvitationPage,
            RevokeInvitation,
        },
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
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/access/invitations",operation_id="access_invitation_list",tag="access",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),AccessQuery),responses((status=200,body=InvitationPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn list(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<AccessQuery>,
) -> Result<Json<InvitationPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(invitations::list(&state.db, &actor, query).await?))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/access/invitations/{invitation_id}",operation_id="access_invitation_get",tag="access",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("invitation_id"=Uuid,Path)),responses((status=200,body=InvitationLink),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn get(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<InvitationLink>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        invitations::get(&state.db, &actor, &state.invitation_tokens, id).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/access/invitations",operation_id="access_invitation_create",tag="access",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=CreateInvitation,responses((status=201,body=InvitationLink),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn create(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<CreateInvitation>,
) -> Result<(StatusCode, Json<InvitationLink>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok((
        StatusCode::CREATED,
        Json(
            invitations::create(
                &state.db,
                &actor,
                &state.invitation_tokens,
                key(&headers)?,
                input,
            )
            .await?,
        ),
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/access/invitations/{invitation_id}/send",operation_id="access_invitation_send",tag="access",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("invitation_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=invitations::delivery::SendInvitation,responses((status=202,body=crate::jobs::Job),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn send(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<invitations::delivery::SendInvitation>,
) -> Result<(StatusCode, Json<crate::jobs::Job>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok((
        StatusCode::ACCEPTED,
        Json(
            invitations::delivery::send(
                &state.db,
                &actor,
                id,
                key(&headers)?,
                input,
                state.auth_mail_enabled,
            )
            .await?,
        ),
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/access/invitations/{invitation_id}/revoke",operation_id="access_invitation_revoke",tag="access",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("invitation_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=RevokeInvitation,responses((status=200,body=Invitation),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn revoke(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<RevokeInvitation>,
) -> Result<Json<Invitation>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        invitations::revoke(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/access/invitations/{invitation_id}/accept",operation_id="access_invitation_accept",tag="access",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("invitation_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=AcceptInvitation,responses((status=200,body=Member),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn accept(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<AcceptInvitation>,
) -> Result<Json<Member>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        invitations::accept(
            &state.db,
            &actor,
            &state.invitation_tokens,
            id,
            key(&headers)?,
            input,
        )
        .await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/access/invitations/{invitation_id}/preview",operation_id="access_invitation_preview",tag="access",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("invitation_id"=Uuid,Path)),request_body=invitations::PreviewInvitation,responses((status=200,body=invitations::InvitationPreview),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn preview(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<invitations::PreviewInvitation>,
) -> Result<Json<invitations::InvitationPreview>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        invitations::preview(&state.db, &actor, &state.invitation_tokens, id, input).await?,
    ))
}
pub(super) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(get))
        .routes(routes!(revoke))
        .routes(routes!(send))
        .routes(routes!(accept))
        .routes(routes!(preview))
}
