mod transfer;
// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::{
    application::{ActorContext, OperationError},
    files,
};
use axum::{
    Extension, Json,
    extract::{Path, Query, Request, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
type Failure = (StatusCode, Json<ApiError>);
fn key(headers: &HeaderMap) -> Result<&str, OperationError> {
    headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .ok_or(OperationError::Invalid(
            "Idempotency-Key is required for mutations",
        ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/files",operation_id="files_list",tag="files",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), files::FileQuery),responses((status=200,body=files::FilePage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=408,body=ApiError),(status=413,body=ApiError),(status=503,body=ApiError)))]
async fn list(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<files::FileQuery>,
) -> Result<Json<files::FilePage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = files::list(&state.db, &actor, query).await?;
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/files/{id}",operation_id="files_get",tag="files",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("id"=Uuid,Path)),responses((status=200,body=files::FileRecord),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=408,body=ApiError),(status=413,body=ApiError),(status=503,body=ApiError)))]
async fn get(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<files::FileRecord>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = files::get(&state.db, &actor, id).await?;
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/files/{id}/content",operation_id="files_content",tag="files",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("id"=Uuid,Path)),responses((status=200,body=files::FileContent),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=408,body=ApiError),(status=413,body=ApiError),(status=503,body=ApiError)))]
async fn content(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Response, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let _permit = transfer::acquire()?;
    let result = files::content(&state.db, &actor, id).await?;
    transfer::download(&result, _permit)
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/files",operation_id="files_upload",tag="files",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("Idempotency-Key"=String,Header)),request_body=files::UploadFile,responses((status=201,body=files::FileRecord),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=408,body=ApiError),(status=413,body=ApiError),(status=503,body=ApiError)))]
async fn upload(
    State(state): State<AppState>,
    Path(workspace): Path<Uuid>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    request: Request,
) -> Result<Response, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let _permit = transfer::acquire()?;
    let key = key(&headers)?;
    // Current membership/grants must pass before accepting a potentially large body.
    let mut tx = state.db.begin().await.map_err(OperationError::from)?;
    crate::access::require(&mut tx, &identity, workspace, "files.create").await?;
    crate::access::require(&mut tx, &identity, workspace, "files.read").await?;
    drop(tx);
    let bytes = transfer::upload(request.into_body()).await?;
    let input: files::UploadFile = serde_json::from_slice(&bytes)
        .map_err(|_| OperationError::Invalid("Expected a file upload JSON object"))?;
    let result = files::upload(&state.db, &actor, key, input).await?;
    Ok((StatusCode::CREATED, Json(result)).into_response())
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/files/{id}",operation_id="files_edit",tag="files",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("id"=Uuid,Path), ("Idempotency-Key"=String,Header)),request_body=files::EditFile,responses((status=200,body=files::FileRecord),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=408,body=ApiError),(status=413,body=ApiError),(status=503,body=ApiError)))]
async fn edit(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<files::EditFile>,
) -> Result<Json<files::FileRecord>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = files::edit(&state.db, &actor, id, key(&headers)?, input).await?;
    Ok(Json(result))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/files/{id}/archive",operation_id="files_archive",tag="files",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("id"=Uuid,Path), ("Idempotency-Key"=String,Header)),request_body=files::ArchiveFile,responses((status=200,body=files::FileRecord),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=408,body=ApiError),(status=413,body=ApiError),(status=503,body=ApiError)))]
async fn archive(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<files::ArchiveFile>,
) -> Result<Json<files::FileRecord>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    let result = files::archive(&state.db, &actor, id, key(&headers)?, input).await?;
    Ok(Json(result))
}
pub(super) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list))
        .routes(routes!(get))
        .routes(routes!(content))
        .routes(routes!(upload))
        .routes(routes!(edit))
        .routes(routes!(archive))
}
