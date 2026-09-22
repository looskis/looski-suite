// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::{
    application::ActorContext,
    contacts::{
        ArchiveContact, ListContacts,
        organizations::{
            self, EditOrganization, Organization, OrganizationInput, OrganizationPage,
            Relationship, RelationshipInput,
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
fn key(headers: &HeaderMap) -> Result<&str, crate::contacts::Error> {
    headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .ok_or(crate::contacts::Error::Invalid(
            "Idempotency-Key is required for mutations",
        ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/organizations",operation_id="listOrganizations",tag="organizations",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),ListContacts),responses((status=200,body=OrganizationPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn list(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<ListContacts>,
) -> Result<Json<OrganizationPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(organizations::list(&state.db, &actor, query).await?))
}

#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/organizations/{organization_id}",operation_id="getOrganization",tag="organizations",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("organization_id"=Uuid,Path)),responses((status=200,body=Organization),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn get(
    State(state): State<AppState>,
    Path((workspace, organization)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Organization>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        organizations::get(&state.db, &actor, organization).await?,
    ))
}

#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/organizations",operation_id="createOrganization",tag="organizations",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=OrganizationInput,responses((status=201,body=Organization),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn create(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<OrganizationInput>,
) -> Result<(StatusCode, Json<Organization>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok((
        StatusCode::CREATED,
        Json(organizations::create(&state.db, &actor, key(&headers)?, input).await?),
    ))
}

#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/organizations/{organization_id}",operation_id="updateOrganization",tag="organizations",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("organization_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=EditOrganization,responses((status=200,body=Organization),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn edit(
    State(state): State<AppState>,
    Path((workspace, organization)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<EditOrganization>,
) -> Result<Json<Organization>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        organizations::edit(&state.db, &actor, organization, key(&headers)?, input).await?,
    ))
}

#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/organizations/{organization_id}/archive",operation_id="archiveOrganization",tag="organizations",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("organization_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=ArchiveContact,responses((status=200,body=Organization),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn archive(
    State(state): State<AppState>,
    Path((workspace, organization)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<ArchiveContact>,
) -> Result<Json<Organization>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        organizations::archive(&state.db, &actor, organization, key(&headers)?, input).await?,
    ))
}

#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/contacts/{contact_id}/organizations",operation_id="listContactOrganizations",tag="organizations",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("contact_id"=Uuid,Path)),responses((status=200,body=Vec<Relationship>),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn relationships(
    State(state): State<AppState>,
    Path((workspace, contact)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Vec<Relationship>>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        organizations::relationships(&state.db, &actor, contact).await?,
    ))
}

#[utoipa::path(put,path="/api/v1/workspaces/{workspace_id}/contacts/{contact_id}/organizations/{organization_id}",operation_id="setContactOrganization",tag="organizations",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("contact_id"=Uuid,Path),("organization_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=RelationshipInput,responses((status=200,body=Relationship),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn relate(
    State(state): State<AppState>,
    Path((workspace, contact, organization)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<RelationshipInput>,
) -> Result<Json<Relationship>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        organizations::relate(
            &state.db,
            &actor,
            contact,
            organization,
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
        .routes(routes!(archive))
        .routes(routes!(relationships))
        .routes(routes!(relate))
}
