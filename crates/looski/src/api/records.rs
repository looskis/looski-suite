// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::{
    application::ActorContext,
    records::{self, RecordField, RecordView, SaveField, SaveView},
};
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
type Failure = (StatusCode, Json<ApiError>);
fn key(headers: &HeaderMap) -> Result<&str, crate::contacts::Error> {
    headers
        .get("idempotency-key")
        .and_then(|s| s.to_str().ok())
        .ok_or(crate::contacts::Error::Invalid(
            "Idempotency-Key is required",
        ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/records/{kind}/fields",operation_id="record_fields",tag="records",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("kind"=String,Path)),responses((status=200,body=Vec<RecordField>),(status=400,body=ApiError),(status=403,body=ApiError),(status=409,body=ApiError)))]
async fn fields(
    State(state): State<AppState>,
    Path((workspace, kind)): Path<(Uuid, String)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Vec<RecordField>>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(records::fields(&state.db, &actor, &kind).await?))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/records/{kind}/fields",operation_id="record_save_field",tag="records",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("kind"=String,Path),("Idempotency-Key"=String,Header)),request_body=SaveField,responses((status=200,body=RecordField),(status=400,body=ApiError),(status=403,body=ApiError),(status=409,body=ApiError)))]
async fn save_field(
    State(state): State<AppState>,
    Path((workspace, kind)): Path<(Uuid, String)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SaveField>,
) -> Result<Json<RecordField>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        records::save_field(&state.db, &actor, &kind, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/records/{kind}/views",operation_id="record_views",tag="records",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("kind"=String,Path)),responses((status=200,body=Vec<RecordView>),(status=400,body=ApiError),(status=403,body=ApiError),(status=409,body=ApiError)))]
async fn views(
    State(state): State<AppState>,
    Path((workspace, kind)): Path<(Uuid, String)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Vec<RecordView>>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(records::views(&state.db, &actor, &kind).await?))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/records/{kind}/views",operation_id="record_create_view",tag="records",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("kind"=String,Path),("Idempotency-Key"=String,Header)),request_body=SaveView,responses((status=200,body=RecordView),(status=400,body=ApiError),(status=403,body=ApiError),(status=409,body=ApiError)))]
async fn create_view(
    State(state): State<AppState>,
    Path((workspace, kind)): Path<(Uuid, String)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SaveView>,
) -> Result<Json<RecordView>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        records::save_view(&state.db, &actor, &kind, None, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(put,path="/api/v1/workspaces/{workspace_id}/records/{kind}/views/{view_id}",operation_id="record_update_view",tag="records",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("kind"=String,Path),("view_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveView,responses((status=200,body=RecordView),(status=400,body=ApiError),(status=403,body=ApiError),(status=409,body=ApiError)))]
async fn update_view(
    State(state): State<AppState>,
    Path((workspace, kind, id)): Path<(Uuid, String, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<SaveView>,
) -> Result<Json<RecordView>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        records::save_view(&state.db, &actor, &kind, Some(id), key(&headers)?, input).await?,
    ))
}
pub(super) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(fields, save_field))
        .routes(routes!(views, create_view))
        .routes(routes!(update_view, delete_view))
}
#[utoipa::path(delete,path="/api/v1/workspaces/{workspace_id}/records/{kind}/views/{view_id}",operation_id="record_delete_view",tag="records",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("kind"=String,Path),("view_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=crate::records::DeleteView,responses((status=200,body=RecordView),(status=400,body=ApiError),(status=403,body=ApiError),(status=409,body=ApiError)))]
async fn delete_view(
    State(state): State<AppState>,
    Path((workspace, kind, id)): Path<(Uuid, String, Uuid)>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<crate::records::DeleteView>,
) -> Result<Json<RecordView>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace,
        request_id: request_id.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        records::delete_view(&state.db, &actor, &kind, id, key(&headers)?, input).await?,
    ))
}
