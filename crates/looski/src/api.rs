// SPDX-License-Identifier: AGPL-3.0-only
use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub http: reqwest::Client,
    pub auth_url: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SystemResponse {
    pub name: String,
    pub version: String,
    pub stage: String,
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
async fn system() -> Json<SystemResponse> {
    let info = crate::application::system_info();
    Json(SystemResponse {
        name: info.name.into(),
        version: info.version.into(),
        stage: info.stage.into(),
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
        .is_ok_and(|version| version == 1)
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
    OpenApiRouter::with_openapi(spec)
        .routes(routes!(system))
        .routes(routes!(ready))
}

pub fn openapi() -> utoipa::openapi::OpenApi {
    documented_routes().split_for_parts().1
}

pub fn router(state: AppState) -> Router {
    let (router, spec) = documented_routes().split_for_parts();
    router
        .route("/health/live", get(live))
        .route("/api/v1/openapi.json", get(move || async { Json(spec) }))
        .fallback(not_found)
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
            db: PgPoolOptions::new()
                .acquire_timeout(std::time::Duration::from_millis(50))
                .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
                .unwrap(),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(50))
                .build()
                .unwrap(),
            auth_url: "http://127.0.0.1:1".into(),
        })
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
}
