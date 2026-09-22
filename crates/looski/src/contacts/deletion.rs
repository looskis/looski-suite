// SPDX-License-Identifier: AGPL-3.0-only
//! Physical removal is distinct from archive and never cascades into business history.
mod dao;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    reliability,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteContact {
    pub expected_version: i64,
    pub confirm_id: Uuid,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeletedContact {
    pub id: Uuid,
    pub deleted_at: String,
}
pub async fn delete(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: DeleteContact,
) -> Result<DeletedContact, Error> {
    if input.confirm_id != id || input.expected_version < 1 {
        return Err(Error::Invalid(
            "Confirm this contact ID and its current version",
        ));
    }
    let mut tx = pool.begin().await?;
    crate::modules::lock_workspace(&mut tx, actor.workspace).await?;
    let write = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        "contacts.delete_permanently",
    )
    .await?;
    let read = access::require(&mut tx, actor.identity, actor.workspace, "contacts.read").await?;
    let operation = format!("contacts.delete_permanently:{id}");
    let request = serde_json::to_value(&input).expect("delete serializes");
    if let Some(owner) = dao::receipt_owner(&mut tx, actor.workspace, id).await? {
        if !read.permits_owner(owner) || !write.permits_owner(owner) {
            return Err(Error::NotFound);
        }
        return reliability::replay(&mut tx, actor.workspace, &write, &operation, key, &request)
            .await?
            .ok_or(Error::NotFound);
    }
    // Read ownership before acquiring any record lock, then use the same
    // identifier-before-record order as Contact edits and merges.
    let observed = super::dao::get(&mut tx, actor.workspace, id, &write, false).await?;
    if !read.permits_owner(observed.owner_membership_id) {
        return Err(Error::NotFound);
    }
    if let Some(saved) =
        reliability::replay(&mut tx, actor.workspace, &write, &operation, key, &request).await?
    {
        return Ok(saved);
    }
    let identifiers = dao::identifiers(&mut tx, actor.workspace, id).await?;
    super::dao::lock_identifiers(&mut tx, actor.workspace, &identifiers).await?;
    let current = super::dao::get(&mut tx, actor.workspace, id, &write, true).await?;
    if !read.permits_owner(current.owner_membership_id) {
        return Err(Error::NotFound);
    }
    // Every change increments the version, including identifier replacement.
    // A changed snapshot must be retried rather than expanding locks out of order.
    if current.version != input.expected_version || current.version != observed.version {
        return Err(Error::Conflict("Contact changed; reload before deleting"));
    }
    if !current.archived || current.merged_into_id.is_some() {
        return Err(Error::Conflict(
            "Only archived, unmerged contacts can be permanently deleted",
        ));
    }
    let result = dao::delete(
        &mut tx,
        actor.workspace,
        id,
        current.owner_membership_id,
        current.version,
    )
    .await?;
    reliability::redact_contact_replays(&mut tx, actor.workspace, id).await?;
    reliability::audit(
        &mut tx,
        actor,
        &write,
        "contacts.deleted_permanently",
        id,
        serde_json::json!({}),
    )
    .await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
