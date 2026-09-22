// SPDX-License-Identifier: AGPL-3.0-only
use super::{Target, TaskLinkChange};
use crate::{application::OperationError as Error, crm::HistoryQuery};
use serde::Deserialize;
use sqlx::{PgConnection, types::Json};
use uuid::Uuid;
#[derive(Deserialize)]
pub(super) struct Row {
    pub id: Uuid,
    pub task_id: Uuid,
    pub ended: bool,
    pub created_at: String,
    pub updated_at: String,
}
const FIELDS: &str = "jsonb_build_object('id',id,'task_id',task_id,'ended',ended_at IS NOT NULL,'created_at',created_at::text,'updated_at',updated_at::text)";
pub(super) async fn get(
    db: &mut PgConnection,
    workspace: Uuid,
    target: Target,
    parent: Uuid,
    id: Uuid,
) -> Result<Row, Error> {
    let (table, column, _) = target.table();
    let sql =
        format!("SELECT {FIELDS} FROM {table} WHERE workspace_id=$1 AND {column}=$2 AND id=$3");
    Ok(
        sqlx::query_scalar::<_, Json<Row>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(parent)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
pub(super) async fn list(
    db: &mut PgConnection,
    workspace: Uuid,
    target: Target,
    parent: Uuid,
    after: Option<Uuid>,
    ended: bool,
) -> Result<Vec<Row>, Error> {
    let (table, column, _) = target.table();
    let sql = format!(
        "SELECT {FIELDS} FROM {table} WHERE workspace_id=$1 AND {column}=$2 AND (ended_at IS NOT NULL)=$3 AND ($4::uuid IS NULL OR (created_at,id)>(SELECT created_at,id FROM {table} WHERE workspace_id=$1 AND {column}=$2 AND id=$4)) ORDER BY created_at,id LIMIT 100"
    );
    Ok(
        sqlx::query_scalar::<_, Json<Row>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(parent)
            .bind(ended)
            .bind(after)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|r| r.0)
            .collect(),
    )
}
pub(super) async fn insert(
    db: &mut PgConnection,
    workspace: Uuid,
    target: Target,
    parent: Uuid,
    task: Uuid,
    member: Uuid,
) -> Result<Row, Error> {
    let (table, column, _) = target.table();
    let sql = format!(
        "SELECT EXISTS(SELECT 1 FROM {table} WHERE workspace_id=$1 AND {column}=$2 AND task_id=$3)"
    );
    if sqlx::query_scalar::<_, bool>(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(workspace)
        .bind(parent)
        .bind(task)
        .fetch_one(&mut *db)
        .await?
    {
        return Err(Error::Conflict(
            "Task is already linked; restore its ended link instead",
        ));
    }
    let sql = format!(
        "INSERT INTO {table}(workspace_id,{column},task_id,created_by_membership_id) VALUES($1,$2,$3,$4) RETURNING id"
    );
    let id = sqlx::query_scalar(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(workspace)
        .bind(parent)
        .bind(task)
        .bind(member)
        .fetch_one(&mut *db)
        .await?;
    get(db, workspace, target, parent, id).await
}
pub(super) async fn end(
    db: &mut PgConnection,
    workspace: Uuid,
    target: Target,
    id: Uuid,
    ended: bool,
) -> Result<(), Error> {
    let (table, _, _) = target.table();
    let sql = format!(
        "UPDATE {table} SET ended_at=CASE WHEN $3 THEN COALESCE(ended_at,now()) ELSE NULL END,updated_at=now() WHERE workspace_id=$1 AND id=$2"
    );
    sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(workspace)
        .bind(id)
        .bind(ended)
        .execute(db)
        .await?;
    Ok(())
}
pub(super) async fn change(
    db: &mut PgConnection,
    workspace: Uuid,
    target: Target,
    id: Uuid,
    ended: bool,
    audit: Uuid,
) -> Result<(), Error> {
    let (_, _, table) = target.table();
    let sql = format!(
        "INSERT INTO {table}(workspace_id,link_id,ended,audit_event_id) VALUES($1,$2,$3,$4)"
    );
    sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(workspace)
        .bind(id)
        .bind(ended)
        .bind(audit)
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
pub(super) async fn history(
    db: &mut PgConnection,
    workspace: Uuid,
    target: Target,
    id: Uuid,
    query: &HistoryQuery,
    limit: i64,
) -> Result<Vec<TaskLinkChange>, Error> {
    let (_, _, table) = target.table();
    if let Some(cursor) = query.after {
        let sql = format!(
            "SELECT EXISTS(SELECT 1 FROM {table} WHERE workspace_id=$1 AND link_id=$2 AND id=$3)"
        );
        if !sqlx::query_scalar::<_, bool>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(id)
            .bind(cursor)
            .fetch_one(&mut *db)
            .await?
        {
            return Err(Error::NotFound);
        }
    }
    let sql = format!(
        "SELECT jsonb_build_object('id',id,'ended',ended,'audit_event_id',audit_event_id,'occurred_at',occurred_at::text) FROM {table} WHERE workspace_id=$1 AND link_id=$2 AND ($3::uuid IS NULL OR (occurred_at,id)>(SELECT occurred_at,id FROM {table} WHERE workspace_id=$1 AND link_id=$2 AND id=$3)) ORDER BY occurred_at,id LIMIT $4"
    );
    Ok(
        sqlx::query_scalar::<_, Json<TaskLinkChange>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(id)
            .bind(query.after)
            .bind(limit)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|r| r.0)
            .collect(),
    )
}
