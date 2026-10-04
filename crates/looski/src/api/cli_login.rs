// SPDX-License-Identifier: AGPL-3.0-only
use super::{ApiError, AppState, RequestId, source_interface, verified_identity};
use crate::cli_login::{
    self, Authorization, CliSession, Context, CreateAuthorization, Error, ExchangeCode,
    RefreshSession,
};
use axum::{
    Extension, Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use utoipa_axum::{router::OpenApiRouter, routes};
type Failure = (StatusCode, Json<ApiError>);
impl From<Error> for Failure {
    fn from(error: Error) -> Self {
        let (status, code, message) = match error {
            Error::Grant => (
                StatusCode::BAD_REQUEST,
                "invalid_grant",
                "The sign-in is invalid, expired or already used. Run `looski login` again.",
            ),
            Error::Throttled => (
                StatusCode::TOO_MANY_REQUESTS,
                "throttled",
                "Too many sign-in attempts. Wait a minute and try again.",
            ),
            Error::Forbidden => (
                StatusCode::FORBIDDEN,
                "forbidden",
                "No active Looski account is linked to this sign-in",
            ),
            Error::Invalid(message) => (StatusCode::BAD_REQUEST, "invalid_input", message),
            Error::Operation(error) => return error.into(),
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
fn context(headers: &HeaderMap, request_id: RequestId) -> Context {
    Context {
        request_id: request_id.0,
        source_interface: source_interface(headers),
    }
}
/// Throttle key: the client address the gateway forwards (Cloudflare's
/// CF-Connecting-IP behind the tunnel, otherwise the direct peer).
fn client(headers: &HeaderMap) -> &str {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.rsplit(',').next())
        .map(str::trim)
        .unwrap_or("direct")
}

#[utoipa::path(post,path="/api/v1/cli/authorizations",operation_id="cli_authorization_create",tag="cli",security(("bearerAuth"=[])),request_body=CreateAuthorization,responses((status=201,body=Authorization),(status=400,body=ApiError),(status=401,body=ApiError),(status=403,body=ApiError),(status=429,body=ApiError),(status=503,body=ApiError)))]
async fn authorize(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<CreateAuthorization>,
) -> Result<(StatusCode, Json<Authorization>), Failure> {
    let identity = verified_identity(&state, &headers).await?;
    Ok((
        StatusCode::CREATED,
        Json(
            cli_login::authorize(&state.db, &identity, &context(&headers, request_id), input)
                .await?,
        ),
    ))
}
#[utoipa::path(post,path="/api/v1/cli/sessions",operation_id="cli_session_create",tag="cli",request_body=ExchangeCode,responses((status=200,body=CliSession),(status=400,body=ApiError),(status=429,body=ApiError),(status=503,body=ApiError)))]
async fn exchange(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    Json(input): Json<ExchangeCode>,
) -> Result<Json<CliSession>, Failure> {
    Ok(Json(
        state
            .cli_login
            .exchange(
                &state.db,
                &state.verifier,
                &context(&headers, request_id),
                client(&headers),
                input,
            )
            .await?,
    ))
}
#[utoipa::path(post,path="/api/v1/cli/sessions/refresh",operation_id="cli_session_refresh",tag="cli",request_body=RefreshSession,responses((status=200,body=CliSession),(status=400,body=ApiError),(status=429,body=ApiError),(status=503,body=ApiError)))]
async fn refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<RefreshSession>,
) -> Result<Json<CliSession>, Failure> {
    Ok(Json(
        state
            .cli_login
            .refresh(&state.db, &state.verifier, client(&headers), input)
            .await?,
    ))
}
#[utoipa::path(post,path="/api/v1/cli/sessions/logout",operation_id="cli_session_logout",tag="cli",security(("bearerAuth"=[])),responses((status=204),(status=400,body=ApiError),(status=401,body=ApiError),(status=503,body=ApiError)))]
async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<StatusCode, Failure> {
    let identity = verified_identity(&state, &headers).await?;
    let token = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .unwrap_or_default();
    state
        .cli_login
        .logout(&state.db, &identity, token, &context(&headers, request_id))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(super) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(authorize))
        .routes(routes!(exchange))
        .routes(routes!(refresh))
        .routes(routes!(logout))
}
