// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::{
    application::{ActorContext, OperationError},
    helpdesk::{self, *},
};
use axum::{
    Extension, Json,
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
fn code(h: &HeaderMap) -> Result<&str, OperationError> {
    h.get("x-ticket-key")
        .and_then(|v| v.to_str().ok())
        .ok_or(OperationError::NotFound)
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/helpdesk/settings",operation_id="helpdesk_settings",tag="helpdesk",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path)),responses((status=200,body=HelpdeskSettings),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn settings(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
) -> Result<Json<HelpdeskSettings>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(helpdesk::settings(&state.db, &actor).await?))
}
#[utoipa::path(put,path="/api/v1/workspaces/{workspace_id}/helpdesk/settings",operation_id="helpdesk_configure",tag="helpdesk",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=ConfigureHelpdesk,responses((status=200,body=HelpdeskSettings),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn configure(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Json(input): Json<ConfigureHelpdesk>,
) -> Result<Json<HelpdeskSettings>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        helpdesk::configure(&state.db, &actor, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/helpdesk/directory",operation_id="helpdesk_directory",tag="helpdesk",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path)),responses((status=200,body=Vec<HelpdeskMember>),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn directory(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
) -> Result<Json<Vec<HelpdeskMember>>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(helpdesk::directory(&state.db, &actor).await?))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/helpdesk/tickets",operation_id="helpdesk_list",tag="helpdesk",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),TicketQuery),responses((status=200,body=TicketPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn list(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Query(query): Query<TicketQuery>,
) -> Result<Json<TicketPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(helpdesk::list(&state.db, &actor, query).await?))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/helpdesk/tickets/{ticket_id}",operation_id="helpdesk_get",tag="helpdesk",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("ticket_id"=Uuid,Path)),responses((status=200,body=Ticket),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn get(
    State(state): State<AppState>,
    Path((workspace_id, ticket_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
) -> Result<Json<Ticket>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(helpdesk::get(&state.db, &actor, ticket_id).await?))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/helpdesk/tickets/{ticket_id}",operation_id="helpdesk_update",tag="helpdesk",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("ticket_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=UpdateTicket,responses((status=200,body=Ticket),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn update(
    State(state): State<AppState>,
    Path((workspace_id, ticket_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Json(input): Json<UpdateTicket>,
) -> Result<Json<Ticket>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        helpdesk::update(&state.db, &actor, ticket_id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(put,path="/api/v1/workspaces/{workspace_id}/helpdesk/tickets/{ticket_id}/links",operation_id="helpdesk_link",tag="helpdesk",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("ticket_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=LinkTicket,responses((status=200,body=Ticket),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn link(
    State(state): State<AppState>,
    Path((workspace_id, ticket_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Json(input): Json<LinkTicket>,
) -> Result<Json<Ticket>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        helpdesk::link(&state.db, &actor, ticket_id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/helpdesk/tickets/{ticket_id}/messages",operation_id="helpdesk_messages",tag="helpdesk",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("ticket_id"=Uuid,Path),MessageQuery),responses((status=200,body=TicketMessages),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn messages(
    State(state): State<AppState>,
    Path((workspace_id, ticket_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Query(query): Query<MessageQuery>,
) -> Result<Json<TicketMessages>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        helpdesk::messages(&state.db, &actor, ticket_id, query).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/helpdesk/tickets/{ticket_id}/messages",operation_id="helpdesk_reply",tag="helpdesk",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("ticket_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=ReplyTicket,responses((status=200,body=TicketMessage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn reply(
    State(state): State<AppState>,
    Path((workspace_id, ticket_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Json(input): Json<ReplyTicket>,
) -> Result<Json<TicketMessage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        helpdesk::reply(&state.db, &actor, ticket_id, key(&headers)?, input).await?,
    ))
}
fn admit(state: &AppState, h: &HeaderMap, bucket: &'static str, limit: u32) -> Result<(), Failure> {
    let client = h
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.rsplit(',').next())
        .map(str::trim)
        .unwrap_or("direct");
    if state.public.admit(bucket, client, limit) {
        Ok(())
    } else {
        Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ApiError {
                code: "throttled".into(),
                message: "Too many requests; retry in a minute".into(),
            }),
        ))
    }
}
#[utoipa::path(get,path="/api/v1/public/helpdesk/intake/{public_key}",operation_id="helpdesk_intake",tag="helpdesk",params(("public_key"=Uuid,Path)),responses((status=200,body=HelpdeskIntake),(status=400,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=429,body=ApiError),(status=503,body=ApiError)))]
async fn intake(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<HelpdeskIntake>, Failure> {
    admit(&state, &headers, "helpdesk.read", 180)?;
    Ok(Json(helpdesk::intake(&state.db, id).await?))
}
#[utoipa::path(post,path="/api/v1/public/helpdesk/intake/{public_key}",operation_id="helpdesk_open",tag="helpdesk",params(("public_key"=Uuid,Path),("X-Ticket-Key"=String,Header)),request_body=OpenTicket,responses((status=200,body=TicketReceipt),(status=400,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=429,body=ApiError),(status=503,body=ApiError)))]
async fn open(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<OpenTicket>,
) -> Result<Json<TicketReceipt>, Failure> {
    admit(&state, &headers, "helpdesk.write", 30)?;
    Ok(Json(
        helpdesk::open(&state.db, id, code(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/public/helpdesk/tickets/{ticket_id}",operation_id="helpdesk_customer_get",tag="helpdesk",params(("ticket_id"=Uuid,Path),("X-Ticket-Key"=String,Header)),responses((status=200,body=CustomerTicket),(status=400,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=429,body=ApiError),(status=503,body=ApiError)))]
async fn customer_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<CustomerTicket>, Failure> {
    admit(&state, &headers, "helpdesk.read", 180)?;
    Ok(Json(
        helpdesk::customer_get(&state.db, id, code(&headers)?).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/public/helpdesk/tickets/{ticket_id}/messages",operation_id="helpdesk_customer_messages",tag="helpdesk",params(("ticket_id"=Uuid,Path),("X-Ticket-Key"=String,Header),MessageQuery),responses((status=200,body=TicketMessages),(status=400,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=429,body=ApiError),(status=503,body=ApiError)))]
async fn customer_messages(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Query(query): Query<MessageQuery>,
) -> Result<Json<TicketMessages>, Failure> {
    admit(&state, &headers, "helpdesk.read", 180)?;
    Ok(Json(
        helpdesk::customer_messages(&state.db, id, code(&headers)?, query).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/public/helpdesk/tickets/{ticket_id}/messages",operation_id="helpdesk_customer_reply",tag="helpdesk",params(("ticket_id"=Uuid,Path),("X-Ticket-Key"=String,Header),("Idempotency-Key"=String,Header)),request_body=CustomerReply,responses((status=200,body=TicketMessage),(status=400,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=429,body=ApiError),(status=503,body=ApiError)))]
async fn customer_reply(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<CustomerReply>,
) -> Result<Json<TicketMessage>, Failure> {
    admit(&state, &headers, "helpdesk.write", 30)?;
    Ok(Json(
        helpdesk::customer_reply(&state.db, id, code(&headers)?, key(&headers)?, input).await?,
    ))
}
pub(super) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(settings, configure))
        .routes(routes!(directory))
        .routes(routes!(list))
        .routes(routes!(get, update))
        .routes(routes!(link))
        .routes(routes!(messages, reply))
        .routes(routes!(intake, open))
        .routes(routes!(customer_get))
        .routes(routes!(customer_messages, customer_reply))
}
