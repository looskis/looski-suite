// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, verified_identity};
use crate::contacts::{
    self, ArchiveContact, Contact, ContactInput, ContactPage, EditContact, ListContacts,
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
type Failure = (StatusCode, Json<ApiError>);
impl From<contacts::Error> for Failure {
    fn from(error: contacts::Error) -> Self {
        let (status, code, message) = match error {
            contacts::Error::Invalid(message) => {
                (StatusCode::BAD_REQUEST, "invalid_input", message)
            }
            contacts::Error::Forbidden => (
                StatusCode::FORBIDDEN,
                "forbidden",
                "This operation is not permitted",
            ),
            contacts::Error::NotFound => (StatusCode::NOT_FOUND, "not_found", "Record not found"),
            contacts::Error::Conflict(message) => (StatusCode::CONFLICT, "conflict", message),
            contacts::Error::Database(error) => {
                // Avoid SQL bind values/customer data in diagnostics.
                tracing::error!(database_code=?error.as_database_error().and_then(|e|e.code()),"Contact operation failed");
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "unavailable",
                    "The service is temporarily unavailable",
                )
            }
        };
        (
            status,
            Json(ApiError {
                code: code.into(),
                message: message.into(),
            }),
        )
    }
}
fn key(headers: &HeaderMap) -> Result<&str, contacts::Error> {
    headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .ok_or(contacts::Error::Invalid(
            "Idempotency-Key is required for mutations",
        ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/contacts",operation_id="listContacts",tag="contacts",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),ListContacts),responses((status=200,body=ContactPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn list(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<super::RequestId>,
    Query(query): Query<ListContacts>,
) -> Result<Json<ContactPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = crate::application::ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: super::source_interface(&headers),
    };
    Ok(Json(contacts::list(&state.db, &actor, query).await?))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/contacts/{contact_id}",operation_id="getContact",tag="contacts",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("contact_id"=Uuid,Path)),responses((status=200,body=Contact),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=503,body=ApiError)))]
async fn get(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<super::RequestId>,
) -> Result<Json<Contact>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = crate::application::ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: super::source_interface(&headers),
    };
    Ok(Json(contacts::get(&state.db, &actor, id).await?))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/contacts",operation_id="createContact",tag="contacts",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=ContactInput,responses((status=201,body=Contact),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn create(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<super::RequestId>,
    Json(input): Json<ContactInput>,
) -> Result<(StatusCode, Json<Contact>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = crate::application::ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: super::source_interface(&headers),
    };
    Ok((
        StatusCode::CREATED,
        Json(contacts::create(&state.db, &actor, key(&headers)?, input).await?),
    ))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/contacts/{contact_id}",operation_id="updateContact",tag="contacts",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("contact_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=EditContact,responses((status=200,body=Contact),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn edit(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<super::RequestId>,
    Json(input): Json<EditContact>,
) -> Result<Json<Contact>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = crate::application::ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: super::source_interface(&headers),
    };
    Ok(Json(
        contacts::edit(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/contacts/{contact_id}/archive",operation_id="archiveContact",tag="contacts",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("contact_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=ArchiveContact,responses((status=200,body=Contact),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn archive(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<super::RequestId>,
    Json(input): Json<ArchiveContact>,
) -> Result<Json<Contact>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = crate::application::ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: super::source_interface(&headers),
    };
    Ok(Json(
        contacts::archive(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/contacts/{contact_id}/merge",operation_id="mergeContacts",tag="contacts",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("contact_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=contacts::MergeContacts,responses((status=200,body=Contact),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn merge(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<super::RequestId>,
    Json(input): Json<contacts::MergeContacts>,
) -> Result<Json<Contact>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = crate::application::ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: super::source_interface(&headers),
    };
    Ok(Json(
        contacts::merge(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
pub(super) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(get, edit))
        .routes(routes!(archive))
        .routes(routes!(merge))
}
