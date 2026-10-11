// SPDX-License-Identifier: AGPL-3.0-only
//! First-party CLI/MCP sign-in. A signed-in browser approves a short-lived,
//! single-use code bound to a PKCE S256 challenge; exchange mints a new,
//! independent Auth session for that user through the private admin API.
//! Codes and tokens are never persisted or logged; only code hashes are stored.
mod dao;

use crate::{
    application::OperationError,
    identity::{Identity, Verifier},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

/// Approval codes are typed by a person; keep the window short.
pub const CODE_LIFETIME_SECONDS: i64 = 300;
/// Outstanding approvals per user; bounds stored rows and phishing retries.
const PENDING_LIMIT: i64 = 5;
const EXCHANGES_PER_MINUTE: u32 = 20;
const REFRESHES_PER_MINUTE: u32 = 60;

pub enum Error {
    /// Uniform answer for unknown, expired, reused or mismatched codes and
    /// revoked refresh tokens; callers learn nothing about which applied.
    Grant,
    Throttled,
    Forbidden,
    Invalid(&'static str),
    Operation(OperationError),
}
impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        Self::Operation(OperationError::Database(error))
    }
}
impl From<OperationError> for Error {
    fn from(error: OperationError) -> Self {
        Self::Operation(error)
    }
}

/// Request attribution only; it cannot grant authority.
pub struct Context {
    pub request_id: Uuid,
    pub source_interface: &'static str,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateAuthorization {
    /// BASE64URL(SHA-256(code_verifier)) without padding.
    pub code_challenge: String,
    /// Only `S256` is accepted.
    pub code_challenge_method: String,
    /// Caller-reported device name, shown for recognition only.
    pub device_label: Option<String>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct Authorization {
    /// One-time code. Display it to its owner; never put it in URLs or logs.
    pub code: String,
    pub expires_at: String,
    pub expires_in: i64,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExchangeCode {
    pub code: String,
    pub code_verifier: String,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RefreshSession {
    pub refresh_token: String,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SessionUser {
    pub id: Uuid,
    pub email: Option<String>,
}
/// A first-party Auth session for the CLI/MCP. The refresh token rotates.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CliSession {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user: SessionUser,
}

/// BASE64URL(SHA-256(verifier)), as defined by RFC 7636.
pub fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}
fn valid_challenge(value: &str) -> bool {
    value.len() == 43
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
fn valid_verifier(value: &str) -> bool {
    (43..=128).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._~".contains(&b))
}
fn hash(code: &str) -> [u8; 32] {
    Sha256::digest(code.as_bytes()).into()
}
fn equal(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len() && left.iter().zip(right).fold(0, |a, (x, y)| a | (x ^ y)) == 0
}

/// Approve a sign-in for the verified browser user.
pub async fn authorize(
    pool: &PgPool,
    identity: &Identity,
    context: &Context,
    input: CreateAuthorization,
) -> Result<Authorization, Error> {
    if input.code_challenge_method != "S256" || !valid_challenge(&input.code_challenge) {
        return Err(Error::Invalid("A PKCE S256 code challenge is required"));
    }
    let device = input.device_label.as_deref().map(str::trim);
    if device
        .is_some_and(|d| d.is_empty() || d.chars().count() > 64 || d.chars().any(char::is_control))
    {
        return Err(Error::Invalid(
            "Device label must be 1–64 printable characters",
        ));
    }
    let mut code = [0u8; 32];
    getrandom::fill(&mut code).map_err(|_| OperationError::Unavailable)?;
    let code = URL_SAFE_NO_PAD.encode(code);
    let mut tx = pool.begin().await?;
    let account = dao::account(&mut tx, identity)
        .await?
        .ok_or(Error::Forbidden)?;
    if dao::pending(&mut tx, account.user_id).await? >= PENDING_LIMIT {
        return Err(Error::Throttled);
    }
    let (id, expires_at) = dao::create(
        &mut tx,
        &account,
        &hash(&code),
        &input.code_challenge,
        device,
        CODE_LIFETIME_SECONDS,
    )
    .await?;
    dao::security_event(
        &mut tx,
        account.user_id,
        "cli_login.authorized",
        context,
        serde_json::json!({"authorization_id": id, "device_label": device}),
    )
    .await?;
    tx.commit().await?;
    Ok(Authorization {
        code,
        expires_at,
        expires_in: CODE_LIFETIME_SECONDS,
    })
}

#[derive(Clone)]
pub struct Service {
    http: reqwest::Client,
    auth_url: String,
    issuer: String,
    admin_key: EncodingKey,
    throttle: Throttle,
}
#[derive(Deserialize)]
struct AuthSession {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
    user: SessionUser,
}
impl Service {
    pub fn new(auth_url: String, issuer: String, secret: &[u8]) -> Result<Self, reqwest::Error> {
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            auth_url,
            issuer,
            admin_key: EncodingKey::from_secret(secret),
            throttle: Throttle::default(),
        })
    }

    fn admin(&self) -> Result<String, Error> {
        encode(
            &Header::default(),
            &serde_json::json!({
                "role": "service_role", "iss": self.issuer,
                "exp": jsonwebtoken::get_current_timestamp() + 60
            }),
            &self.admin_key,
        )
        .map_err(|_| Error::Operation(OperationError::Unavailable))
    }

    /// Mint a new session through a one-time magic-link token generated and
    /// verified server side. Nothing is emailed. Generating it replaces any
    /// outstanding recovery/magic-link token for the account.
    async fn mint(&self, subject: &str) -> Result<AuthSession, Error> {
        let unavailable = || Error::Operation(OperationError::Unavailable);
        let admin = self.admin()?;
        let response = self
            .http
            .get(format!("{}/admin/users/{subject}", self.auth_url))
            .bearer_auth(&admin)
            .send()
            .await
            .map_err(|_| unavailable())?;
        if response.status().as_u16() == 404 {
            return Err(Error::Grant);
        }
        if !response.status().is_success() {
            return Err(unavailable());
        }
        #[derive(Deserialize)]
        struct User {
            id: String,
            email: Option<String>,
        }
        let user: User = response.json().await.map_err(|_| unavailable())?;
        // Look up by stable subject first: a magic link for an unknown email
        // would create an account, and an email alone must never select one.
        let email = user
            .email
            .filter(|e| !e.is_empty() && user.id == subject)
            .ok_or(Error::Grant)?;
        let response = self
            .http
            .post(format!("{}/admin/generate_link", self.auth_url))
            .bearer_auth(&admin)
            .json(&serde_json::json!({"type": "magiclink", "email": email}))
            .send()
            .await
            .map_err(|_| unavailable())?;
        if !response.status().is_success() {
            return Err(unavailable());
        }
        #[derive(Deserialize)]
        struct Link {
            id: String,
            hashed_token: String,
        }
        let link: Link = response.json().await.map_err(|_| unavailable())?;
        if link.id != subject {
            return Err(unavailable());
        }
        let response = self
            .http
            .post(format!("{}/verify", self.auth_url))
            .json(&serde_json::json!({"type": "magiclink", "token_hash": link.hashed_token}))
            .send()
            .await
            .map_err(|_| unavailable())?;
        // A banned or concurrently removed account cannot complete sign-in.
        if response.status().is_client_error() {
            return Err(Error::Grant);
        }
        if !response.status().is_success() {
            return Err(unavailable());
        }
        let session: AuthSession = response.json().await.map_err(|_| unavailable())?;
        if session.user.id.to_string() != subject {
            return Err(unavailable());
        }
        Ok(session)
    }

    async fn revoke(&self, token: &str) {
        let _ = self
            .http
            .post(format!("{}/logout?scope=local", self.auth_url))
            .bearer_auth(token)
            .send()
            .await;
    }

    /// Exchange a code and its PKCE verifier exactly once for a new session.
    pub async fn exchange(
        &self,
        pool: &PgPool,
        verifier: &Verifier,
        context: &Context,
        client: &str,
        input: ExchangeCode,
    ) -> Result<CliSession, Error> {
        if !self
            .throttle
            .admit("exchange", client, EXCHANGES_PER_MINUTE)
        {
            return Err(Error::Throttled);
        }
        if input.code.is_empty() || input.code.len() > 128 || !valid_verifier(&input.code_verifier)
        {
            return Err(Error::Grant);
        }
        let mut tx = pool.begin().await?;
        let Some(pending) = dao::lock(&mut tx, &hash(&input.code)).await? else {
            return Err(Error::Grant);
        };
        if pending.consumed || !pending.live {
            return Err(Error::Grant);
        }
        let details =
            serde_json::json!({"authorization_id": pending.id, "device_label": pending.device});
        // The first presentation consumes the code even when it fails, so a
        // leaked code cannot be retried against other verifiers.
        if !equal(
            challenge(&input.code_verifier).as_bytes(),
            pending.challenge.as_bytes(),
        ) || !pending.active
            || pending.issuer != self.issuer
        {
            dao::consume(&mut tx, pending.id, None).await?;
            dao::security_event(
                &mut tx,
                pending.user_id,
                "cli_login.rejected",
                context,
                details,
            )
            .await?;
            tx.commit().await?;
            return Err(Error::Grant);
        }
        // Mint while holding the row lock. An Auth outage rolls back and leaves
        // the code usable until it expires; nothing is consumed without a session.
        let session = self.mint(&pending.subject).await?;
        let Some(session_id) = verifier.session_id(&session.access_token) else {
            self.revoke(&session.access_token).await;
            return Err(Error::Operation(OperationError::Unavailable));
        };
        let recorded = async {
            dao::consume(&mut tx, pending.id, Some(session_id)).await?;
            let mut details = details;
            details["session_id"] = serde_json::json!(session_id);
            dao::security_event(
                &mut tx,
                pending.user_id,
                "cli_login.exchanged",
                context,
                details,
            )
            .await?;
            tx.commit().await
        }
        .await;
        if let Err(error) = recorded {
            // Never hand out a session whose issuance is not recorded.
            self.revoke(&session.access_token).await;
            return Err(error.into());
        }
        Ok(CliSession {
            access_token: session.access_token,
            refresh_token: session.refresh_token,
            token_type: "bearer".into(),
            expires_in: session.expires_in,
            user: session.user,
        })
    }

    /// Rotate a CLI session's refresh token through private Auth.
    pub async fn refresh(
        &self,
        pool: &PgPool,
        verifier: &Verifier,
        client: &str,
        input: RefreshSession,
    ) -> Result<CliSession, Error> {
        if !self.throttle.admit("refresh", client, REFRESHES_PER_MINUTE) {
            return Err(Error::Throttled);
        }
        if input.refresh_token.is_empty() || input.refresh_token.len() > 512 {
            return Err(Error::Grant);
        }
        let response = self
            .http
            .post(format!("{}/token?grant_type=refresh_token", self.auth_url))
            .json(&serde_json::json!({"refresh_token": input.refresh_token}))
            .send()
            .await
            .map_err(|_| OperationError::Unavailable)?;
        match response.status().as_u16() {
            200 => {}
            429 => return Err(Error::Throttled),
            400..=499 => return Err(Error::Grant),
            _ => return Err(OperationError::Unavailable.into()),
        }
        let session: AuthSession = response
            .json()
            .await
            .map_err(|_| OperationError::Unavailable)?;
        // Browser sessions refresh through SvelteKit only. Rotation has already
        // happened upstream, so a browser token presented here stops working.
        let session_id = verifier
            .session_id(&session.access_token)
            .ok_or(OperationError::Unavailable)?;
        let mut db = pool.acquire().await?;
        if !dao::cli_session(&mut db, session_id).await? {
            return Err(Error::Grant);
        }
        Ok(CliSession {
            access_token: session.access_token,
            refresh_token: session.refresh_token,
            token_type: "bearer".into(),
            expires_in: session.expires_in,
            user: session.user,
        })
    }

    /// Revoke only the presented session; other CLI and browser sessions remain.
    pub async fn logout(
        &self,
        pool: &PgPool,
        identity: &Identity,
        token: &str,
        context: &Context,
    ) -> Result<(), Error> {
        let response = self
            .http
            .post(format!("{}/logout?scope=local", self.auth_url))
            .bearer_auth(token)
            .send()
            .await
            .map_err(|_| OperationError::Unavailable)?;
        if response.status().is_client_error() {
            return Err(Error::Grant);
        }
        if !response.status().is_success() {
            return Err(OperationError::Unavailable.into());
        }
        let mut tx = pool.begin().await?;
        if let Some(account) = dao::account(&mut tx, identity).await? {
            dao::security_event(
                &mut tx,
                account.user_id,
                "cli_session.logout",
                context,
                serde_json::json!({}),
            )
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

type Windows = std::collections::HashMap<(&'static str, String), (std::time::Instant, u32)>;
/// Per-client fixed-window admission for the public endpoints. The table is
/// bounded; when full, unknown clients share one bucket instead of growing it.
#[derive(Clone, Default)]
pub struct Throttle(std::sync::Arc<std::sync::Mutex<Windows>>);
impl Throttle {
    const WINDOW: std::time::Duration = std::time::Duration::from_secs(60);
    const CLIENTS: usize = 4096;
    pub(crate) fn admit(&self, bucket: &'static str, client: &str, limit: u32) -> bool {
        let now = std::time::Instant::now();
        let mut table = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if table.len() >= Self::CLIENTS {
            table.retain(|_, (start, _)| now.duration_since(*start) < Self::WINDOW);
        }
        let client = if table.len() >= Self::CLIENTS {
            String::new()
        } else {
            client.chars().take(64).collect()
        };
        let entry = table.entry((bucket, client)).or_insert((now, 0));
        if now.duration_since(entry.0) >= Self::WINDOW {
            *entry = (now, 0);
        }
        entry.1 = entry.1.saturating_add(1);
        entry.1 <= limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_matches_rfc_7636_example() {
        assert_eq!(
            challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
        assert!(valid_challenge(&challenge("x".repeat(43).as_str())));
        assert!(!valid_challenge("short"));
        assert!(!valid_verifier(
            "contains spaces and is long enough to be a verifier ok"
        ));
    }

    #[test]
    fn throttle_limits_each_client_within_a_window() {
        let throttle = Throttle::default();
        assert!((0..3).all(|_| throttle.admit("exchange", "a", 3)));
        assert!(!throttle.admit("exchange", "a", 3));
        assert!(throttle.admit("exchange", "b", 3));
        assert!(throttle.admit("refresh", "a", 3));
    }
}
