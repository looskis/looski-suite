// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::{
    application::{ActorContext, OperationError},
    scheduling::{self, *},
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
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/scheduling/types",operation_id="scheduling_types",tag="scheduling",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),SchedulingQuery),responses((status=200,body=AppointmentTypePage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn types(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Query(query): Query<SchedulingQuery>,
) -> Result<Json<AppointmentTypePage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(scheduling::types(&state.db, &actor, query).await?))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/scheduling/types",operation_id="scheduling_create_type",tag="scheduling",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveAppointmentType,responses((status=200,body=AppointmentType),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn create_type(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Json(input): Json<SaveAppointmentType>,
) -> Result<Json<AppointmentType>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        scheduling::save_type(&state.db, &actor, None, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/scheduling/types/{id}",operation_id="scheduling_get_type",tag="scheduling",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path)),responses((status=200,body=AppointmentType),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn get_type(
    State(state): State<AppState>,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
) -> Result<Json<AppointmentType>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(scheduling::get_type(&state.db, &actor, id).await?))
}
#[utoipa::path(put,path="/api/v1/workspaces/{workspace_id}/scheduling/types/{id}",operation_id="scheduling_update_type",tag="scheduling",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=SaveAppointmentType,responses((status=200,body=AppointmentType),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn update_type(
    State(state): State<AppState>,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Json(input): Json<SaveAppointmentType>,
) -> Result<Json<AppointmentType>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        scheduling::save_type(&state.db, &actor, Some(id), key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/scheduling/appointments",operation_id="scheduling_list",tag="scheduling",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),SchedulingQuery),responses((status=200,body=BookingPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn list(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Query(query): Query<SchedulingQuery>,
) -> Result<Json<BookingPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(scheduling::list(&state.db, &actor, query).await?))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/scheduling/appointments/{id}",operation_id="scheduling_get",tag="scheduling",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path)),responses((status=200,body=Booking),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn get(
    State(state): State<AppState>,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
) -> Result<Json<Booking>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(scheduling::get(&state.db, &actor, id).await?))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/scheduling/appointments/{id}/change",operation_id="scheduling_change",tag="scheduling",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=ChangeBooking,responses((status=200,body=Booking),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn change(
    State(state): State<AppState>,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Json(input): Json<ChangeBooking>,
) -> Result<Json<Booking>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        scheduling::change(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/scheduling/appointments/{id}/retry",operation_id="scheduling_retry",tag="scheduling",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),("Idempotency-Key"=String,Header)),request_body=RetryBooking,responses((status=200,body=Booking),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn retry(
    State(state): State<AppState>,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Json(input): Json<RetryBooking>,
) -> Result<Json<Booking>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        scheduling::retry(&state.db, &actor, id, key(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/scheduling/appointments/{id}/events",operation_id="scheduling_events",tag="scheduling",security(("bearerAuth"=[])),params(("workspace_id"=Uuid,Path),("id"=Uuid,Path),SchedulingQuery),responses((status=200,body=BookingEventPage),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=503,body=ApiError)))]
async fn events(
    State(state): State<AppState>,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Extension(request): Extension<RequestId>,
    Query(query): Query<SchedulingQuery>,
) -> Result<Json<BookingEventPage>, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let actor = ActorContext {
        identity: &identity,
        workspace: workspace_id,
        request_id: request.0,
        source_interface: source_interface(&headers),
    };
    Ok(Json(
        scheduling::events(&state.db, &actor, id, query).await?,
    ))
}
fn staff_routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(types, create_type))
        .routes(routes!(get_type, update_type))
        .routes(routes!(list))
        .routes(routes!(get))
        .routes(routes!(change))
        .routes(routes!(retry))
        .routes(routes!(events))
}

fn code(h: &HeaderMap) -> Result<&str, OperationError> {
    h.get("x-booking-key")
        .and_then(|v| v.to_str().ok())
        .ok_or(OperationError::NotFound)
}
fn admit(state: &AppState, h: &HeaderMap, write: bool) -> Result<(), Failure> {
    let client = h
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.rsplit(',').next())
        .map(str::trim)
        .unwrap_or("direct");
    if state.public.admit(
        if write {
            "scheduling.write"
        } else {
            "scheduling.read"
        },
        client,
        if write { 30 } else { 180 },
    ) {
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
#[utoipa::path(get,path="/api/v1/public/scheduling/types/{id}",operation_id="scheduling_public_info",tag="scheduling",params(("id"=Uuid,Path)),responses((status=200,body=PublicAppointmentType),(status=400,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=429,body=ApiError),(status=503,body=ApiError)))]
async fn public_info(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<PublicAppointmentType>, Failure> {
    admit(&state, &headers, false)?;
    Ok(Json(scheduling::public_info(&state.db, id).await?))
}
#[utoipa::path(get,path="/api/v1/public/scheduling/types/{id}/slots",operation_id="scheduling_slots",tag="scheduling",params(("id"=Uuid,Path),SlotQuery),responses((status=200,body=Vec<BookingSlot>),(status=400,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=429,body=ApiError),(status=503,body=ApiError)))]
async fn slots(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Query(query): Query<SlotQuery>,
) -> Result<Json<Vec<BookingSlot>>, Failure> {
    admit(&state, &headers, false)?;
    Ok(Json(scheduling::slots(&state.db, id, query).await?))
}
#[utoipa::path(post,path="/api/v1/public/scheduling/types/{id}/book",operation_id="scheduling_reserve",tag="scheduling",params(("id"=Uuid,Path),("X-Booking-Key"=String,Header)),request_body=ReserveBooking,responses((status=200,body=BookingReceipt),(status=400,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=429,body=ApiError),(status=503,body=ApiError)))]
async fn reserve(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<ReserveBooking>,
) -> Result<Json<BookingReceipt>, Failure> {
    admit(&state, &headers, true)?;
    Ok(Json(
        scheduling::reserve(&state.db, id, code(&headers)?, input).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/public/scheduling/appointments/{id}",operation_id="scheduling_customer_get",tag="scheduling",params(("id"=Uuid,Path),("X-Booking-Key"=String,Header)),responses((status=200,body=PublicBooking),(status=400,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=429,body=ApiError),(status=503,body=ApiError)))]
async fn customer_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<PublicBooking>, Failure> {
    admit(&state, &headers, false)?;
    Ok(Json(
        scheduling::customer_get(&state.db, id, code(&headers)?).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/public/scheduling/appointments/{id}",operation_id="scheduling_customer_change",tag="scheduling",params(("id"=Uuid,Path),("X-Booking-Key"=String,Header),("Idempotency-Key"=String,Header)),request_body=ChangeBooking,responses((status=200,body=PublicBooking),(status=400,body=ApiError),(status=404,body=ApiError),(status=409,body=ApiError),(status=429,body=ApiError),(status=503,body=ApiError)))]
async fn customer_change(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<ChangeBooking>,
) -> Result<Json<PublicBooking>, Failure> {
    admit(&state, &headers, true)?;
    Ok(Json(
        scheduling::customer_change(&state.db, id, code(&headers)?, key(&headers)?, input).await?,
    ))
}
pub(super) fn routes() -> OpenApiRouter<AppState> {
    staff_routes()
        .routes(routes!(public_info))
        .routes(routes!(slots))
        .routes(routes!(reserve))
        .routes(routes!(customer_get, customer_change))
}
