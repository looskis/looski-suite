// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::{
    application::ActorContext,
    contacts::tagging::{self, SetTags, Target},
    tags::{self, ListTags, SaveTag, Tag, TagPage},
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
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/tags",operation_id="listTags",tag="tags",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),ListTags),responses((status=200,body=TagPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn list(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<ListTags>,
) -> Result<Json<TagPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(tags::list(&state.db, &actor, query).await?))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/tags/{tag_id}",operation_id="getTag",tag="tags",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("tag_id"=Uuid,Path)),responses((status=200,body=Tag),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn get(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Tag>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(tags::get(&state.db, &actor, id).await?))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/tags",operation_id="createTag",tag="tags",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("Idempotency-Key"=String,Header)),request_body=SaveTag,responses((status=201,body=Tag),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn create(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SaveTag>,
) -> Result<(StatusCode, Json<Tag>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok((
        StatusCode::CREATED,
        Json(tags::save(&state.db, &actor, None, key(&headers)?, input).await?),
    ))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/tags/{tag_id}",operation_id="updateTag",tag="tags",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("tag_id"=Uuid,Path), ("Idempotency-Key"=String,Header)),request_body=SaveTag,responses((status=200,body=Tag),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn edit(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SaveTag>,
) -> Result<Json<Tag>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        tags::save(&state.db, &actor, Some(id), key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/contacts/{contact_id}/tags",operation_id="listContactTags",tag="tags",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("contact_id"=Uuid,Path)),responses((status=200,body=Vec<Tag>),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn list_contacts(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Vec<Tag>>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        tagging::list(&state.db, &actor, Target::Contact, id).await?,
    ))
}
#[utoipa::path(put,path="/api/v1/workspaces/{workspace_id}/contacts/{contact_id}/tags",operation_id="setContactTags",tag="tags",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("contact_id"=Uuid,Path), ("Idempotency-Key"=String,Header)),request_body=SetTags,responses((status=200,body=Vec<Tag>),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn set_contacts(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SetTags>,
) -> Result<Json<Vec<Tag>>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        tagging::set(
            &state.db,
            &actor,
            Target::Contact,
            id,
            key(&headers)?,
            input,
        )
        .await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/organizations/{organization_id}/tags",operation_id="listOrganizationTags",tag="tags",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("organization_id"=Uuid,Path)),responses((status=200,body=Vec<Tag>),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn list_organizations(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Vec<Tag>>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        tagging::list(&state.db, &actor, Target::Organization, id).await?,
    ))
}
#[utoipa::path(put,path="/api/v1/workspaces/{workspace_id}/organizations/{organization_id}/tags",operation_id="setOrganizationTags",tag="tags",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("organization_id"=Uuid,Path), ("Idempotency-Key"=String,Header)),request_body=SetTags,responses((status=200,body=Vec<Tag>),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn set_organizations(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SetTags>,
) -> Result<Json<Vec<Tag>>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        tagging::set(
            &state.db,
            &actor,
            Target::Organization,
            id,
            key(&headers)?,
            input,
        )
        .await?,
    ))
}
pub(super) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(get, edit))
        .routes(routes!(list_contacts, set_contacts))
        .routes(routes!(list_organizations, set_organizations))
}
