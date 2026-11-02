// SPDX-License-Identifier: AGPL-3.0-only
//! Optional, short-lived owner claim. Runs separately from the business API.
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{net::SocketAddr, sync::Arc, time::Duration};

#[derive(Clone, Deserialize)]
pub struct Claim {
    pub token_sha256: String,
    pub expires_at: u64,
}

impl Claim {
    fn accepts(&self, token: &str, now: u64) -> bool {
        if now >= self.expires_at || token.len() != 64 {
            return false;
        }
        // Only compare the digest, never a prefix of the bearer credential.
        let digest = Sha256::digest(token.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        digest == self.token_sha256
    }
}

#[derive(Clone)]
struct Setup {
    db: PgPool,
    http: reqwest::Client,
    origin: String,
    auth_url: String,
    issuer: String,
    secret: Arc<String>,
    claim: Option<Claim>,
    throttle: crate::cli_login::Throttle,
}

impl Setup {
    async fn open(&self) -> bool {
        if !self
            .claim
            .as_ref()
            .is_some_and(|claim| jsonwebtoken::get_current_timestamp() < claim.expires_at)
        {
            return false;
        }
        sqlx::query_scalar::<_, bool>(
            "SELECT bootstrapped_at IS NULL FROM suite.installation WHERE singleton",
        )
        .fetch_one(&self.db)
        .await
        .unwrap_or(false)
    }
}

async fn page(State(state): State<Setup>) -> Response {
    if !state.open().await {
        return StatusCode::NOT_FOUND.into_response();
    }
    Html(include_str!("setup/index.html")).into_response()
}

async fn claim(
    State(state): State<Setup>,
    headers: HeaderMap,
    Json(input): Json<crate::bootstrap::OwnerInput>,
) -> StatusCode {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    if headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) != Some(&state.origin)
        || !state
            .claim
            .as_ref()
            .is_some_and(|claim| claim.accepts(token, jsonwebtoken::get_current_timestamp()))
    {
        return StatusCode::NOT_FOUND;
    }
    // Invalid credentials never consume this bucket. Bound attempts using a valid claim.
    if !state.throttle.admit("owner-claim", "installation", 10) {
        return StatusCode::TOO_MANY_REQUESTS;
    }
    if !state.open().await {
        return StatusCode::NOT_FOUND;
    }
    // Provision holds the permanent installation row lock through Auth and commit.
    // Concurrent claims and retries therefore cannot create a second workspace.
    match crate::bootstrap::provision(
        &state.db,
        &state.http,
        &state.auth_url,
        &state.issuer,
        state.secret.trim().as_bytes(),
        input,
    )
    .await
    {
        Ok(()) => StatusCode::CREATED,
        Err(_) => StatusCode::BAD_REQUEST, // Never expose Auth, SQL, or credentials.
    }
}

fn router(state: Setup) -> Router {
    Router::new()
        .route("/health/live", get(|| async { StatusCode::OK }))
        .route("/setup", get(page))
        .route("/setup/owner", post(claim))
        .route("/setup.js", get(|| async {
            ([(header::CONTENT_TYPE, "text/javascript; charset=utf-8")], include_str!("setup/setup.js"))
        }))
        .route("/setup.css", get(|| async {
            ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], include_str!("setup/setup.css"))
        }))
        .layer(DefaultBodyLimit::max(8192))
        .layer(axum::middleware::map_response(|mut response: Response| async {
            let headers = response.headers_mut();
            headers.insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
            headers.insert(header::REFERRER_POLICY, "no-referrer".parse().unwrap());
            headers.insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
            headers.insert(header::CONTENT_SECURITY_POLICY,
                "default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; form-action 'self'; frame-ancestors 'none'; base-uri 'none'".parse().unwrap());
            response
        }))
        .with_state(state)
}

pub async fn serve(bind: SocketAddr) -> anyhow::Result<()> {
    let read =
        |name| -> anyhow::Result<String> { Ok(std::fs::read_to_string(std::env::var(name)?)?) };
    let claim: Option<Claim> = serde_json::from_str(&read("LOOSKI_OWNER_CLAIM_FILE")?).ok();
    let state = Setup {
        db: PgPoolOptions::new()
            .max_connections(2)
            .acquire_timeout(Duration::from_secs(3))
            .connect_lazy(read("LOOSKI_DATABASE_URL_FILE")?.trim())?,
        http: reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?,
        origin: std::env::var("LOOSKI_PUBLIC_URL")?,
        auth_url: std::env::var("LOOSKI_AUTH_INTERNAL_URL")?,
        issuer: std::env::var("LOOSKI_AUTH_ISSUER")?,
        secret: Arc::new(read("LOOSKI_JWT_SECRET_FILE")?),
        claim,
        throttle: crate::cli_login::Throttle::default(),
    };
    axum::serve(tokio::net::TcpListener::bind(bind).await?, router(state)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[test]
    fn claims_expire_and_reject_wrong_credentials() {
        let token = "a".repeat(64);
        let claim = Claim {
            token_sha256: Sha256::digest(token.as_bytes())
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            expires_at: 100,
        };
        assert!(claim.accepts(&token, 99));
        assert!(!claim.accepts(&token, 100));
        assert!(!claim.accepts(&"b".repeat(64), 99));
        assert!(!claim.accepts("a", 99));
    }

    #[tokio::test]
    async fn missing_claim_and_cross_origin_fail_without_database_access() {
        let state = Setup {
            db: PgPoolOptions::new()
                .connect_lazy("postgres://localhost/nonexistent")
                .unwrap(),
            http: reqwest::Client::new(),
            origin: "https://suite.example.com".into(),
            auth_url: "http://localhost:1".into(),
            issuer: "unused".into(),
            secret: Arc::new("unused".into()),
            claim: None,
            throttle: crate::cli_login::Throttle::default(),
        };
        let app = router(state);
        let page = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/setup")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(page.status(), StatusCode::NOT_FOUND);
        assert_eq!(page.headers()[header::CACHE_CONTROL], "no-store");
        let response = app.oneshot(Request::builder().method("POST").uri("/setup/owner")
            .header("content-type", "application/json").header("origin", "https://attacker.example")
            .body(Body::from(r#"{"email":"owner@example.com","password":"some-password","display_name":"Owner","workspace_name":"Example"}"#)).unwrap()).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
