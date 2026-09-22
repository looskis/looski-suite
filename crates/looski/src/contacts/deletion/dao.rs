// SPDX-License-Identifier: AGPL-3.0-only
use super::DeletedContact;
use crate::{application::OperationError as Error, contacts::Identifier};
use sqlx::PgConnection;
use uuid::Uuid;
pub(super) async fn receipt_owner(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
) -> Result<Option<Uuid>, Error> {
    Ok(sqlx::query_scalar("SELECT owner_membership_id FROM contacts.deletion_receipts WHERE workspace_id=$1 AND id=$2").bind(workspace).bind(id).fetch_optional(db).await?)
}
pub(super) async fn identifiers(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
) -> Result<Vec<Identifier>, Error> {
    let rows=sqlx::query_as::<_,(String,String)>("SELECT DISTINCT i.kind,i.normalized_value FROM contacts.contact_identifiers ci JOIN contacts.identifiers i ON (i.workspace_id,i.id)=(ci.workspace_id,ci.identifier_id) WHERE ci.workspace_id=$1 AND ci.contact_id=$2 ORDER BY i.kind,i.normalized_value").bind(workspace).bind(id).fetch_all(db).await?;
    Ok(rows
        .into_iter()
        .map(|(kind, normalized)| Identifier {
            kind,
            original: normalized.clone(),
            normalized,
            country: None,
            extension: None,
        })
        .collect())
}
pub(super) async fn delete(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    owner: Uuid,
    version: i64,
) -> Result<DeletedContact, Error> {
    let identifiers=sqlx::query_scalar::<_,Uuid>("DELETE FROM contacts.contact_identifiers WHERE workspace_id=$1 AND contact_id=$2 RETURNING identifier_id").bind(workspace).bind(id).fetch_all(&mut *db).await?;
    // Restrictive FKs protect current, historical and future module references.
    if let Err(error) = sqlx::query("DELETE FROM contacts.contacts WHERE workspace_id=$1 AND id=$2")
        .bind(workspace)
        .bind(id)
        .execute(&mut *db)
        .await
    {
        if error.as_database_error().and_then(|e| e.code()).as_deref() == Some("23503") {
            return Err(Error::Conflict(
                "Contact has retained business references; keep it archived",
            ));
        }
        return Err(error.into());
    }
    sqlx::query("DELETE FROM contacts.identifiers i WHERE i.workspace_id=$1 AND i.id=ANY($2) AND NOT EXISTS(SELECT 1 FROM contacts.contact_identifiers ci WHERE ci.workspace_id=i.workspace_id AND ci.identifier_id=i.id)").bind(workspace).bind(identifiers).execute(&mut *db).await?;
    let deleted_at=sqlx::query_scalar("INSERT INTO contacts.deletion_receipts(workspace_id,id,owner_membership_id,deleted_version) VALUES($1,$2,$3,$4) RETURNING deleted_at::text").bind(workspace).bind(id).bind(owner).bind(version).fetch_one(db).await?;
    Ok(DeletedContact { id, deleted_at })
}
