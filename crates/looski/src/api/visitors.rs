// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::{
    application::{ActorContext, OperationError},
    identity::Identity,
    visitors::{
        self, Collect, Detail, EventsPage, EventsQuery, Link, Page, PublicSite,
        Query as VisitorQuery, Receipt, Resolve, Review, SaveSite, Site, Visitor, Withdraw,
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
fn key(headers: &HeaderMap) -> Result<&str, OperationError> {
    headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .ok_or(OperationError::Invalid("Idempotency-Key is required"))
}
fn actor<'a>(
    identity: &'a Identity,
    workspace: Uuid,
    request: RequestId,
    headers: &HeaderMap,
) -> ActorContext<'a> {
    ActorContext {
        identity,
        workspace,
        request_id: request.0,
        source_interface: source_interface(headers),
    }
}
fn origin<'a>(state: &AppState, headers: &'a HeaderMap) -> Result<&'a str, Failure> {
    let client = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.rsplit(',').next())
        .unwrap_or("direct")
        .trim();
    if !state.public.admit("visitors.collect", client, 120) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ApiError {
                code: "throttled".into(),
                message: "Too many visitor requests".into(),
            }),
        ));
    }
    headers
        .get("origin")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| OperationError::NotFound.into())
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/visitors/sites",operation_id="listVisitorSites",tag="visitors",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path)),responses((status=200,body=Vec<Site>),(status=403,body=ApiError)))]
async fn sites(
    State(state): State<AppState>,
    Path(ws): Path<Uuid>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
) -> Result<Json<Vec<Site>>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    Ok(Json(
        visitors::sites(&state.db, &actor(&identity, ws, request, &headers)).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/visitors/sites",operation_id="saveVisitorSite",tag="visitors",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveSite,responses((status=200,body=Site),(status=400,body=ApiError),(status=403,body=ApiError),(status=409,body=ApiError)))]
async fn save_site(
    State(state): State<AppState>,
    Path(ws): Path<Uuid>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Json(input): Json<SaveSite>,
) -> Result<Json<Site>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    Ok(Json(
        visitors::save_site(
            &state.db,
            &actor(&identity, ws, request, &headers),
            key(&headers)?,
            input,
        )
        .await?,
    ))
}
#[utoipa::path(get,path="/api/v1/public/visitors/{site_key}",operation_id="getPublicVisitorSite",tag="visitors",params(("site_key"=String,Path),("Origin"=String,Header)),responses((status=200,body=PublicSite),(status=404,body=ApiError),(status=429,body=ApiError)))]
async fn configuration(
    State(state): State<AppState>,
    Path(site): Path<String>,
    headers: HeaderMap,
) -> Result<Json<PublicSite>, Failure> {
    Ok(Json(
        visitors::configuration(&state.db, &site, origin(&state, &headers)?).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/public/visitors/{site_key}/events",operation_id="collectVisitorEvent",tag="visitors",params(("site_key"=String,Path),("Origin"=String,Header)),request_body=Collect,responses((status=200,body=Receipt),(status=400,body=ApiError),(status=404,body=ApiError),(status=429,body=ApiError)))]
async fn collect(
    State(state): State<AppState>,
    Path(site): Path<String>,
    headers: HeaderMap,
    Json(input): Json<Collect>,
) -> Result<Json<Receipt>, Failure> {
    Ok(Json(
        visitors::collect(&state.db, &site, origin(&state, &headers)?, input).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/public/visitors/{site_key}/withdraw",operation_id="withdrawVisitorConsent",tag="visitors",params(("site_key"=String,Path),("Origin"=String,Header)),request_body=Withdraw,responses((status=200,body=Receipt),(status=400,body=ApiError),(status=404,body=ApiError),(status=429,body=ApiError)))]
async fn withdraw(
    State(state): State<AppState>,
    Path(site): Path<String>,
    headers: HeaderMap,
    Json(input): Json<Withdraw>,
) -> Result<Json<Receipt>, Failure> {
    Ok(Json(
        visitors::withdraw(&state.db, &site, origin(&state, &headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/visitors",operation_id="listVisitors",tag="visitors",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),VisitorQuery),responses((status=200,body=Page),(status=403,body=ApiError)))]
async fn list(
    State(state): State<AppState>,
    Path(ws): Path<Uuid>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Query(query): Query<VisitorQuery>,
) -> Result<Json<Page>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    Ok(Json(
        visitors::list(&state.db, &actor(&identity, ws, request, &headers), query).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/visitors/{visitor_id}",operation_id="getVisitor",tag="visitors",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("visitor_id"=Uuid,Path)),responses((status=200,body=Detail),(status=403,body=ApiError),(status=404,body=ApiError)))]
async fn detail(
    State(state): State<AppState>,
    Path((ws, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
) -> Result<Json<Detail>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    Ok(Json(
        visitors::detail(&state.db, &actor(&identity, ws, request, &headers), id).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/visitors/resolve",operation_id="resolveVisitor",tag="visitors",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=Resolve,responses((status=200,body=Visitor),(status=400,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError)))]
async fn resolve(
    State(state): State<AppState>,
    Path(ws): Path<Uuid>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Json(input): Json<Resolve>,
) -> Result<Json<Visitor>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    Ok(Json(
        visitors::resolve(
            &state.db,
            &actor(&identity, ws, request, &headers),
            key(&headers)?,
            input,
        )
        .await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/visitors/{visitor_id}/matches",operation_id="reviewVisitorMatch",tag="visitors",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("visitor_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=Review,responses((status=200,body=Receipt),(status=400,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError)))]
async fn review(
    State(state): State<AppState>,
    Path((ws, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Json(input): Json<Review>,
) -> Result<Json<Receipt>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    Ok(Json(
        visitors::review(
            &state.db,
            &actor(&identity, ws, request, &headers),
            id,
            key(&headers)?,
            input,
        )
        .await?,
    ))
}
#[utoipa::path(delete,path="/api/v1/workspaces/{workspace_id}/visitors/{visitor_id}",operation_id="deleteVisitor",tag="visitors",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("visitor_id"=Uuid,Path),("Idempotency-Key"=String,Header)),responses((status=200,body=Receipt),(status=403,body=ApiError)))]
async fn delete(
    State(state): State<AppState>,
    Path((ws, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
) -> Result<Json<Receipt>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    Ok(Json(
        visitors::delete(
            &state.db,
            &actor(&identity, ws, request, &headers),
            id,
            key(&headers)?,
        )
        .await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/visitors/{visitor_id}/events",operation_id="listVisitorEvents",tag="visitors",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("visitor_id"=Uuid,Path),EventsQuery),responses((status=200,body=EventsPage),(status=403,body=ApiError),(status=404,body=ApiError)))]
async fn events(
    State(state): State<AppState>,
    Path((ws, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Query(q): Query<EventsQuery>,
) -> Result<Json<EventsPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    Ok(Json(
        visitors::events(&state.db, &actor(&identity, ws, request, &headers), id, q).await?,
    ))
}
#[utoipa::path(put,path="/api/v1/workspaces/{workspace_id}/visitors/{visitor_id}/contact",operation_id="linkVisitorContact",tag="visitors",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("visitor_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=Link,responses((status=200,body=Visitor),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError)))]
async fn link(
    State(state): State<AppState>,
    Path((ws, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Json(input): Json<Link>,
) -> Result<Json<Visitor>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    Ok(Json(
        visitors::link(
            &state.db,
            &actor(&identity, ws, request, &headers),
            id,
            key(&headers)?,
            input,
        )
        .await?,
    ))
}
pub(super) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(sites, save_site))
        .routes(routes!(configuration))
        .routes(routes!(collect))
        .routes(routes!(withdraw))
        .routes(routes!(list))
        .routes(routes!(detail, delete))
        .routes(routes!(resolve))
        .routes(routes!(review))
        .routes(routes!(events))
        .routes(routes!(link))
}
