mod crm;
mod crm_activities;
mod crm_coordination;
mod crm_starter;
mod crm_task_links;
mod crm_timeline;
mod jobs;
mod task_configuration;
mod tasks;
mod workspace_access;
// SPDX-License-Identifier: AGPL-3.0-only
mod cli_login;
mod contacts;
mod exports;
mod files;
mod intake;
mod invitations;
mod organizations;
mod tags;
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

#[derive(Clone)]
pub struct AppState {
    pub auth_mail_enabled: bool,
    pub db: PgPool,
    pub http: reqwest::Client,
    pub auth_url: String,
    /// Canonical browser origin; clients send people here to approve sign-ins.
    pub public_url: String,
    pub verifier: crate::identity::Verifier,
    pub invitation_tokens: crate::workspace_access::invitations::TokenIssuer,
    pub cli_login: crate::cli_login::Service,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SystemResponse {
    pub name: String,
    pub version: String,
    pub stage: String,
    /// Browser origin for sign-in approval; may differ from the API address a
    /// local client uses. Absent from older servers.
    #[serde(default)]
    pub public_url: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: &'static str,
}

#[utoipa::path(get, path = "/api/v1/system", operation_id = "getSystem", tag = "system",
    responses((status = 200, body = SystemResponse)))]
async fn system(State(state): State<AppState>) -> Json<SystemResponse> {
    let info = crate::application::system_info();
    Json(SystemResponse {
        name: info.name.into(),
        version: info.version.into(),
        stage: info.stage.into(),
        public_url: state.public_url.clone(),
    })
}

async fn live() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[utoipa::path(get, path = "/health/ready", operation_id = "getReadiness", tag = "system",
    responses((status = 200, body = HealthResponse), (status = 503, body = ApiError)))]
async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    let db = async {
        sqlx::query_scalar::<_, i32>(
            "SELECT version FROM suite.schema_version WHERE singleton = true",
        )
        .fetch_one(&state.db)
        .await
        .is_ok_and(|version| version == 29)
    };
    let auth = async {
        state
            .http
            .get(format!("{}/health", state.auth_url))
            .send()
            .await
            .is_ok_and(|response| response.status().is_success())
    };
    let (db_ok, auth_ok) = tokio::join!(db, auth);
    if db_ok && auth_ok {
        (StatusCode::OK, Json(serde_json::json!({"status": "ok"})))
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!(ApiError {
                code: "not_ready".into(),
                message: "Required services or schema are unavailable".into(),
            })),
        )
    }
}

#[utoipa::path(get, path = "/api/v1/workspaces", operation_id = "listWorkspaces", tag = "workspaces",
    security(("bearerAuth" = [])),
    responses((status = 200, body = Vec<crate::workspaces::Workspace>),
        (status = 401, body = ApiError), (status = 503, body = ApiError)))]
async fn workspaces(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<crate::workspaces::Workspace>>, (StatusCode, Json<ApiError>)> {
    let identity = verified_identity(&state, &headers).await?;
    crate::workspaces::list(&state.db, &identity)
        .await
        .map(Json)
        .map_err(|error| {
            tracing::error!(database_code=?error.as_database_error().and_then(|e|e.code()), "Workspace query failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiError {
                    code: "unavailable".into(),
                    message: "The service is temporarily unavailable".into(),
                }),
            )
        })
}

pub(super) async fn verified_identity(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<crate::identity::Identity, (StatusCode, Json<ApiError>)> {
    let denied = || {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                code: "unauthenticated".into(),
                message: "A valid first-party session is required".into(),
            }),
        )
    };
    let unavailable = || {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiError {
                code: "unavailable".into(),
                message: "The service is temporarily unavailable".into(),
            }),
        )
    };
    let token = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(denied)?;
    let identity = state.verifier.verify(token).ok_or_else(denied)?;
    // Verify the account still exists upstream; application suspension and
    // membership revocation are independently checked in the business operation.
    let response = state
        .http
        .get(format!("{}/user", state.auth_url))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|_| unavailable())?;
    if response.status().is_client_error() {
        return Err(denied());
    }
    if !response.status().is_success() {
        return Err(unavailable());
    }
    #[derive(Deserialize)]
    struct User {
        id: String,
        email: Option<String>,
        email_confirmed_at: Option<String>,
    }
    let user: User = response.json().await.map_err(|_| unavailable())?;
    if user.id != identity.subject() {
        return Err(denied());
    }
    Ok(identity.bind_confirmed_email(user.email.filter(|_| user.email_confirmed_at.is_some())))
}

async fn not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(ApiError {
            code: "not_found".into(),
            message: "Endpoint not found".into(),
        }),
    )
}

fn documented_routes() -> OpenApiRouter<AppState> {
    let mut spec = utoipa::openapi::OpenApi::default();
    spec.info = utoipa::openapi::Info::new("Looski Suite API", env!("CARGO_PKG_VERSION"));
    spec.info.license = Some(utoipa::openapi::License::new("AGPL-3.0-only"));
    let mut components = utoipa::openapi::Components::new();
    components.add_security_scheme(
        "bearerAuth",
        utoipa::openapi::security::SecurityScheme::Http(utoipa::openapi::security::Http::new(
            utoipa::openapi::security::HttpAuthScheme::Bearer,
        )),
    );
    spec.components = Some(components);
    OpenApiRouter::with_openapi(spec)
        .routes(routes!(system))
        .routes(routes!(ready))
        .routes(routes!(workspaces))
        .merge(cli_login::routes())
        .merge(contacts::routes())
        .merge(organizations::routes())
        .merge(tags::routes())
        .merge(workspace_access::routes())
        .merge(invitations::routes())
        .merge(exports::routes())
        .merge(files::routes())
        .merge(crm::routes())
        .merge(crm_timeline::routes())
        .merge(crm_starter::routes())
        .merge(jobs::routes())
        .merge(crm_coordination::routes())
        .merge(crm_activities::routes())
        .merge(crm_task_links::routes())
        .merge(tasks::routes())
        .merge(task_configuration::routes())
        .merge(intake::routes())
}

pub fn openapi() -> utoipa::openapi::OpenApi {
    documented_routes().split_for_parts().1
}

pub fn router(state: AppState) -> Router {
    router_with_diagnostics(state, crate::diagnostics::Diagnostics::default())
}

pub fn router_with_diagnostics(
    state: AppState,
    diagnostics: crate::diagnostics::Diagnostics,
) -> Router {
    let (router, spec) = documented_routes().split_for_parts();
    router
        .route("/health/live", get(live))
        .route("/api/v1/openapi.json", get(move || async { Json(spec) }))
        .fallback(not_found)
        .layer(axum::middleware::from_fn_with_state(
            diagnostics,
            request_context,
        ))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;

    fn app() -> Router {
        router(AppState {
            auth_mail_enabled: false,
            invitation_tokens: crate::workspace_access::invitations::TokenIssuer::new(
                b"test-invitation-key-with-32-bytes!",
            ),
            db: PgPoolOptions::new()
                .acquire_timeout(std::time::Duration::from_millis(50))
                .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
                .unwrap(),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(50))
                .build()
                .unwrap(),
            auth_url: "http://127.0.0.1:1".into(),
            public_url: "http://localhost:58888".into(),
            verifier: crate::identity::Verifier::new(
                b"test-key",
                "http://localhost/auth/v1".into(),
            ),
            cli_login: crate::cli_login::Service::new(
                "http://127.0.0.1:1".into(),
                "http://localhost/auth/v1".into(),
                b"test-key",
            )
            .unwrap(),
        })
    }

    #[tokio::test]
    async fn file_upload_rejects_missing_identity_without_reading_body() {
        struct UnreadBody;
        impl axum::body::HttpBody for UnreadBody {
            type Data = axum::body::Bytes;
            type Error = std::io::Error;
            fn poll_frame(
                self: std::pin::Pin<&mut Self>,
                _: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>>
            {
                panic!("Unauthenticated upload body must not be polled");
            }
        }
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/workspaces/00000000-0000-0000-0000-000000000001/files")
                    .header("content-type", "application/json")
                    .body(Body::new(UnreadBody))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn liveness_survives_dependency_outage_but_readiness_fails() {
        for (path, expected) in [
            ("/health/live", StatusCode::OK),
            ("/health/ready", StatusCode::SERVICE_UNAVAILABLE),
        ] {
            let response = app()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), expected);
        }
    }

    #[tokio::test]
    async fn contact_operations_require_bearer_identity_before_database_access() {
        for (method, path, body) in [
            (
                "POST",
                "/api/v1/workspaces/00000000-0000-0000-0000-000000000001/exports/page",
                r#"{"collection":"contacts"}"#,
            ),
            (
                "GET",
                "/api/v1/workspaces/00000000-0000-0000-0000-000000000001/exports/collections",
                "",
            ),
            (
                "GET",
                "/api/v1/workspaces/00000000-0000-0000-0000-000000000001/contacts",
                "",
            ),
            (
                "POST",
                "/api/v1/workspaces/00000000-0000-0000-0000-000000000001/contacts",
                r#"{"display_name":"Alex"}"#,
            ),
            (
                "POST",
                "/api/v1/workspaces/00000000-0000-0000-0000-000000000001/contacts/00000000-0000-0000-0000-000000000002/delete-permanently",
                r#"{"expected_version":2,"confirm_id":"00000000-0000-0000-0000-000000000002"}"#,
            ),
        ] {
            let response = app()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(path)
                        .header("content-type", "application/json")
                        .body(Body::from(body))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }
    }

    #[tokio::test]
    async fn organization_directory_requires_identity() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/workspaces/00000000-0000-0000-0000-000000000001/organizations")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn tag_catalog_requires_identity() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/workspaces/00000000-0000-0000-0000-000000000001/tags")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn unknown_business_routes_are_closed() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/contacts")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn workspace_api_rejects_wrong_or_delegated_credentials_before_dependencies() {
        use jsonwebtoken::{EncodingKey, Header, encode};
        let base = serde_json::json!({
            "iss": "http://localhost/auth/v1", "aud": "authenticated",
            "sub": "a48d6928-2106-4d8d-99ec-b2b1a94d8f05",
            "session_id": "d5a10b87-3134-4584-af3d-882fe36c31c7",
            "exp": jsonwebtoken::get_current_timestamp() + 60, "role": "authenticated"
        });
        for (field, value) in [
            ("iss", serde_json::json!("https://other.example/auth/v1")),
            ("aud", serde_json::json!("service_role")),
            ("exp", serde_json::json!(1)),
            ("role", serde_json::json!("service_role")),
            ("is_anonymous", serde_json::json!(true)),
            ("client_id", serde_json::json!("external-app")),
            ("sub", serde_json::json!("")),
            ("session_id", serde_json::json!(null)),
        ] {
            let mut claims = base.clone();
            claims[field] = value;
            let token = encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(b"test-key"),
            )
            .unwrap();
            let response = app()
                .oneshot(
                    Request::builder()
                        .uri("/api/v1/workspaces")
                        .header("authorization", format!("Bearer {token}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{field}");
        }
        for key in [b"test-key".as_slice(), b"wrong-key".as_slice()] {
            let token = encode(&Header::default(), &base, &EncodingKey::from_secret(key)).unwrap();
            let response = app()
                .oneshot(
                    Request::builder()
                        .uri("/api/v1/workspaces")
                        .header("authorization", format!("Bearer {token}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                if key == b"test-key" {
                    StatusCode::SERVICE_UNAVAILABLE
                } else {
                    StatusCode::UNAUTHORIZED
                }
            );
        }
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/workspaces")
                    .header("cookie", "looski_access=anything")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}

#[derive(Clone, Copy)]
pub(super) struct RequestId(pub uuid::Uuid);
async fn request_context(
    State(diagnostics): State<crate::diagnostics::Diagnostics>,
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use tracing::Instrument;
    let started = std::time::Instant::now();
    let method = request.method().clone();
    let route = request
        .extensions()
        .get::<axum::extract::MatchedPath>()
        .map(|p| p.as_str().to_owned());
    let id = uuid::Uuid::new_v4();
    request.extensions_mut().insert(RequestId(id));
    let span = tracing::info_span!("request",request_id=%id,method=%request.method());
    let mut response = next.run(request).instrument(span).await;
    // Unify framework extractor errors without echoing payloads or parse details.
    if matches!(response.status().as_u16(), 400 | 413 | 415 | 422)
        && response
            .headers()
            .get("content-type")
            .is_none_or(|v| v != "application/json")
    {
        response = (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: "invalid_input".into(),
                message: "Invalid request path, query, or JSON body".into(),
            }),
        )
            .into_response();
    }
    response
        .headers_mut()
        .insert("x-request-id", id.to_string().parse().expect("UUID header"));
    response.headers_mut().insert(
        "cache-control",
        axum::http::HeaderValue::from_static("no-store"),
    );
    diagnostics.request(
        id,
        method.as_str(),
        route.as_deref(),
        response.status().as_u16(),
        started.elapsed(),
    );
    response
}

fn source_interface(headers: &HeaderMap) -> &'static str {
    match headers
        .get("x-looski-interface")
        .and_then(|v| v.to_str().ok())
    {
        Some("web") => "web",
        Some("cli") => "cli",
        Some("mcp") => "mcp",
        _ => "api",
    }
}
