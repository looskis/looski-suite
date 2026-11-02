// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::{
    application::{ActorContext, OperationError},
    communications::{self, *},
};
use axum::{
    Extension, Json,
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
type Failure = (StatusCode, Json<ApiError>);
fn key(h: &HeaderMap) -> Result<&str, OperationError> {
    h.get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .ok_or(OperationError::Invalid("Idempotency-Key is required"))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/communications/accounts",operation_id="communications_accounts",tag="communications",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path)),responses((status=200,body=Vec<Account>),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn accounts(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
) -> Result<Json<Vec<Account>>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(communications::accounts(&state.db, &actor).await?))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/communications/accounts",operation_id="communications_connect",tag="communications",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("Idempotency-Key"=String,Header)),request_body=ConnectAccount,responses((status=200,body=Account),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn connect(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Json(input): Json<ConnectAccount>,
) -> Result<Json<Account>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        communications::connect(&state.db, &actor, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/communications/accounts/{id}",operation_id="communications_update_account",tag="communications",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("id"=Uuid,Path), ("Idempotency-Key"=String,Header)),request_body=UpdateAccount,responses((status=200,body=Account),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn update_account(
    State(state): State<AppState>,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Json(input): Json<UpdateAccount>,
) -> Result<Json<Account>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        communications::update_account(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/communications/accounts/{id}/sync",operation_id="communications_sync",tag="communications",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("id"=Uuid,Path)),responses((status=200,body=SyncResult),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn sync(
    State(state): State<AppState>,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
) -> Result<Json<SyncResult>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(communications::sync(&state.db, &actor, id).await?))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/communications/conversations",operation_id="communications_list",tag="communications",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ConversationQuery),responses((status=200,body=Conversations),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn list(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Query(query): Query<ConversationQuery>,
) -> Result<Json<Conversations>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(communications::list(&state.db, &actor, query).await?))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/communications/conversations/{id}",operation_id="communications_get",tag="communications",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("id"=Uuid,Path)),responses((status=200,body=Conversation),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn get(
    State(state): State<AppState>,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
) -> Result<Json<Conversation>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(communications::get(&state.db, &actor, id).await?))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/communications/conversations/{id}",operation_id="communications_update",tag="communications",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("id"=Uuid,Path), ("Idempotency-Key"=String,Header)),request_body=UpdateConversation,responses((status=200,body=Conversation),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn update(
    State(state): State<AppState>,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Json(input): Json<UpdateConversation>,
) -> Result<Json<Conversation>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        communications::update(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/communications/conversations/{id}/messages",operation_id="communications_messages",tag="communications",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("id"=Uuid,Path), MessageQuery),responses((status=200,body=Messages),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn messages(
    State(state): State<AppState>,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Query(query): Query<MessageQuery>,
) -> Result<Json<Messages>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        communications::messages(&state.db, &actor, id, query).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/communications/conversations/{id}/messages",operation_id="communications_send",tag="communications",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path), ("id"=Uuid,Path), ("Idempotency-Key"=String,Header)),request_body=SendMessage,responses((status=200,body=Message),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn send(
    State(state): State<AppState>,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Json(input): Json<SendMessage>,
) -> Result<Json<Message>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        communications::send(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}

#[utoipa::path(post,path="/api/v1/connectors/communications/{id}/events",operation_id="communications_event",tag="communications",params(("id"=Uuid,Path),("X-Connector-Signature"=Option<String>,Header),("X-Blueski-Signature"=Option<String>,Header)),request_body=serde_json::Value,responses((status=204),(status=429,body=ApiError),(status=400,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn event(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, Failure> {
    let client = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.rsplit(',').next())
        .map(str::trim)
        .unwrap_or("direct");
    if !state.public.admit("communications.events", client, 600) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ApiError {
                code: "throttled".into(),
                message: "Retry shortly; journal replay recovers missed events".into(),
            }),
        ));
    }
    let signature = headers
        .get("x-blueski-signature")
        .or_else(|| headers.get("x-connector-signature"))
        .and_then(|v| v.to_str().ok())
        .ok_or(OperationError::Forbidden)?;
    communications::webhook(&state.db, id, signature, &body).await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(super) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(accounts, connect))
        .routes(routes!(update_account))
        .routes(routes!(sync))
        .routes(routes!(list))
        .routes(routes!(get, update))
        .routes(routes!(messages, send))
        .routes(routes!(event))
}
