// SPDX-License-Identifier: AGPL-3.0-only
use super::Context;
use crate::identity::Identity;
use sqlx::{PgConnection, Row, types::Json};
use uuid::Uuid;

pub(super) struct Account {
    pub user_id: Uuid,
    pub identity_id: Uuid,
}
/// The active application account mapped to a verified provider identity.
pub(super) async fn account(
    db: &mut PgConnection,
    identity: &Identity,
) -> Result<Option<Account>, sqlx::Error> {
    Ok(sqlx::query(
        "SELECT u.id AS user_id,i.id AS identity_id FROM suite.auth_identities i
         JOIN suite.users u ON u.id=i.user_id
         WHERE i.issuer=$1 AND i.subject=$2 AND i.disabled_at IS NULL AND u.state='active'",
    )
    .bind(identity.issuer())
    .bind(identity.subject())
    .fetch_optional(db)
    .await?
    .map(|row| Account {
        user_id: row.get("user_id"),
        identity_id: row.get("identity_id"),
    }))
}

/// Serializes approvals per user so the pending-code bound cannot be raced.
pub(super) async fn pending(db: &mut PgConnection, user: Uuid) -> Result<i64, sqlx::Error> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(format!("cli-login:{user}"))
        .execute(&mut *db)
        .await?;
    sqlx::query_scalar("SELECT count(*) FROM suite.cli_authorizations WHERE user_id=$1 AND consumed_at IS NULL AND expires_at>clock_timestamp()")
        .bind(user)
        .fetch_one(db)
        .await
}

pub(super) async fn create(
    db: &mut PgConnection,
    account: &Account,
    hash: &[u8],
    challenge: &str,
    device: Option<&str>,
    lifetime: i64,
) -> Result<(Uuid, String), sqlx::Error> {
    let row = sqlx::query("INSERT INTO suite.cli_authorizations(user_id,identity_id,code_hash,code_challenge,device_label,created_at,expires_at) VALUES($1,$2,$3,$4,$5,clock_timestamp(),clock_timestamp()+make_interval(secs=>$6)) RETURNING id,to_char(expires_at AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS expires_at")
        .bind(account.user_id).bind(account.identity_id).bind(hash).bind(challenge).bind(device).bind(lifetime as f64)
        .fetch_one(db).await?;
    Ok((row.get("id"), row.get("expires_at")))
}

pub(super) struct Pending {
    pub id: Uuid,
    pub user_id: Uuid,
    pub subject: String,
    pub issuer: String,
    pub challenge: String,
    pub device: Option<String>,
    pub consumed: bool,
    pub live: bool,
    pub active: bool,
}
/// Locks the authorization row; concurrent exchanges serialize here and later
/// callers observe the first caller's outcome.
pub(super) async fn lock(
    db: &mut PgConnection,
    hash: &[u8],
) -> Result<Option<Pending>, sqlx::Error> {
    Ok(sqlx::query(
        "SELECT a.id,a.user_id,i.subject,i.issuer,a.code_challenge,a.device_label,
                a.consumed_at IS NOT NULL AS consumed,a.expires_at>clock_timestamp() AS live,
                (u.state='active' AND i.disabled_at IS NULL) AS active
         FROM suite.cli_authorizations a
         JOIN suite.users u ON u.id=a.user_id JOIN suite.auth_identities i ON i.id=a.identity_id
         WHERE a.code_hash=$1 FOR UPDATE OF a",
    )
    .bind(hash)
    .fetch_optional(db)
    .await?
    .map(|row| Pending {
        id: row.get("id"),
        user_id: row.get("user_id"),
        subject: row.get("subject"),
        issuer: row.get("issuer"),
        challenge: row.get("code_challenge"),
        device: row.get("device_label"),
        consumed: row.get("consumed"),
        live: row.get("live"),
        active: row.get("active"),
    }))
}

pub(super) async fn consume(
    db: &mut PgConnection,
    id: Uuid,
    session: Option<Uuid>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE suite.cli_authorizations SET consumed_at=clock_timestamp(),outcome=CASE WHEN $2::uuid IS NULL THEN 'rejected' ELSE 'issued' END,session_id=$2 WHERE id=$1 AND consumed_at IS NULL")
        .bind(id)
        .bind(session)
        .execute(db)
        .await?;
    Ok(())
}

/// Refresh is offered only for sessions minted by a CLI exchange.
pub(super) async fn cli_session(db: &mut PgConnection, session: Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM suite.cli_authorizations WHERE session_id=$1 AND outcome='issued')")
        .bind(session)
        .fetch_one(db)
        .await
}

pub(super) async fn security_event(
    db: &mut PgConnection,
    user: Uuid,
    action: &str,
    context: &Context,
    details: serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO suite.security_events(user_id,action,request_id,source_interface,details) VALUES($1,$2,$3,$4,$5)")
        .bind(user)
        .bind(action)
        .bind(context.request_id)
        .bind(context.source_interface)
        .bind(Json(details))
        .execute(db)
        .await?;
    Ok(())
}
