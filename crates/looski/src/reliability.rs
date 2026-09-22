// SPDX-License-Identifier: AGPL-3.0-only
//! Foundation-owned audit and retry persistence, using the caller's transaction.
use crate::{
    access::Access,
    application::{ActorContext, OperationError as Error},
};
use sqlx::{PgConnection, Row, types::Json};
use uuid::Uuid;

pub async fn audit(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    access: &Access,
    action: &str,
    record: Uuid,
    details: serde_json::Value,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO suite.audit_events(workspace_id,operation_id,actor_user_id,action,record_id,request_id,source_interface,details) VALUES($1,$2,$3,$4,$5,$6,$7,$8)")
        .bind(actor.workspace).bind(Uuid::new_v4()).bind(access.user_id).bind(action).bind(record).bind(actor.request_id).bind(actor.source_interface).bind(Json(details)).execute(db).await?;
    Ok(())
}
pub async fn replay<T: serde::de::DeserializeOwned>(
    db: &mut PgConnection,
    workspace: Uuid,
    access: &Access,
    operation: &str,
    key: &str,
    request: &serde_json::Value,
) -> Result<Option<T>, Error> {
    if key.is_empty() || key.len() > 128 || !key.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(Error::Invalid(
            "An Idempotency-Key of 1–128 visible ASCII characters is required",
        ));
    }
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(format!(
            "retry:{workspace}:{}:{operation}:{key}",
            access.user_id
        ))
        .execute(&mut *db)
        .await?;
    let row=sqlx::query("SELECT request,result,expires_at>now() AS valid FROM suite.idempotency_records WHERE workspace_id=$1 AND actor_user_id=$2 AND operation_key=$3 AND idempotency_key=$4")
    .bind(workspace).bind(access.user_id).bind(operation).bind(key).fetch_optional(db).await?;
    match row {
        None => Ok(None),
        Some(row) => {
            if row.try_get::<Json<serde_json::Value>, _>("request")?.0 != *request {
                return Err(Error::Conflict(
                    "Idempotency key was already used with different input",
                ));
            }
            if !row.try_get::<bool, _>("valid")? {
                return Err(Error::Conflict(
                    "Idempotency key expired; reconcile the earlier result before starting a new operation",
                ));
            }
            Ok(Some(row.try_get::<Json<T>, _>("result")?.0))
        }
    }
}
pub async fn save_replay<T: serde::Serialize>(
    db: &mut PgConnection,
    workspace: Uuid,
    access: &Access,
    operation: &str,
    key: &str,
    request: &serde_json::Value,
    result: &T,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO suite.idempotency_records(workspace_id,actor_user_id,operation_key,idempotency_key,request,result) VALUES($1,$2,$3,$4,$5,$6)")
    .bind(workspace).bind(access.user_id).bind(operation).bind(key).bind(Json(request)).bind(Json(result)).execute(db).await?;
    Ok(())
}
