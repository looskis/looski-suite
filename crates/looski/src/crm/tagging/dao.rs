// SPDX-License-Identifier: AGPL-3.0-only
use super::Target;
use crate::application::OperationError as Error;
use sqlx::PgConnection;
use uuid::Uuid;
// Closed constants only; never interpolate caller-supplied identifiers.
fn table(target: Target) -> (&'static str, &'static str) {
    match target {
        Target::Opportunity => ("crm.opportunity_tags", "opportunity_id"),
        Target::Deal => ("crm.deal_tags", "deal_id"),
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

pub(super) async fn touch(
    db: &mut PgConnection,
    workspace: Uuid,
    target: Target,
    id: Uuid,
) -> Result<(), Error> {
    let table = match target {
        Target::Opportunity => "crm.opportunities",
        Target::Deal => "crm.deals",
    };
    let sql = format!(
        "UPDATE {table} SET version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2"
    );
    sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(workspace)
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}
