// SPDX-License-Identifier: AGPL-3.0-only
use super::Target;
use crate::application::OperationError as Error;
use sqlx::PgConnection;
use uuid::Uuid;
// Closed constants only; never interpolate caller-supplied identifiers.
fn table(target: Target) -> (&'static str, &'static str) {
    match target {
        Target::Contact => ("contacts.contact_tags", "contact_id"),
        Target::Organization => ("contacts.organization_tags", "organization_id"),
    }
}
pub(super) async fn ids(
    db: &mut PgConnection,
    workspace: Uuid,
    target: Target,
    id: Uuid,
) -> Result<Vec<Uuid>, Error> {
    let (table, column) = table(target);
    let sql =
        format!("SELECT tag_id FROM {table} WHERE workspace_id=$1 AND {column}=$2 ORDER BY tag_id");
    Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(workspace)
        .bind(id)
        .fetch_all(db)
        .await?)
}
pub(super) async fn set(
    db: &mut PgConnection,
    workspace: Uuid,
    target: Target,
    id: Uuid,
    ids: &[Uuid],
    member: Uuid,
) -> Result<(), Error> {
    let (table, column) = table(target);
    let sql = format!(
        "DELETE FROM {table} WHERE workspace_id=$1 AND {column}=$2 AND NOT(tag_id=ANY($3))"
    );
    sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(workspace)
        .bind(id)
        .bind(ids)
        .execute(&mut *db)
        .await?;
    let sql = format!(
        "INSERT INTO {table}(workspace_id,{column},tag_id,created_by_membership_id) SELECT $1,$2,unnest($3::uuid[]),$4 ON CONFLICT DO NOTHING"
    );
    sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(workspace)
        .bind(id)
        .bind(ids)
        .bind(member)
        .execute(db)
        .await?;
    Ok(())
}
pub(super) async fn merge(
    db: &mut PgConnection,
    workspace: Uuid,
    source: Uuid,
    survivor: Uuid,
) -> Result<(), Error> {
    let count=sqlx::query_scalar::<_,i64>("SELECT count(DISTINCT tag_id) FROM contacts.contact_tags WHERE workspace_id=$1 AND contact_id IN ($2,$3)").bind(workspace).bind(source).bind(survivor).fetch_one(&mut *db).await?;
    if count > 50 {
        return Err(Error::Conflict("Merged contact would exceed 50 tags"));
    }
    sqlx::query("INSERT INTO contacts.contact_tags(workspace_id,contact_id,tag_id,created_by_membership_id,created_at) SELECT workspace_id,$3,tag_id,created_by_membership_id,created_at FROM contacts.contact_tags WHERE workspace_id=$1 AND contact_id=$2 ON CONFLICT DO NOTHING").bind(workspace).bind(source).bind(survivor).execute(db).await?;
    Ok(())
}
