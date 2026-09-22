// SPDX-License-Identifier: AGPL-3.0-only
use super::{Task, TaskChange, TaskHistoryQuery, TaskInput, TaskQuery};
use crate::{access::Access, application::OperationError as Error};
use sqlx::{PgConnection, types::Json};
use uuid::Uuid;
const RECORD: &str = "SELECT jsonb_build_object('id',id,'title',title,'notes',notes,'assigned_membership_id',assigned_membership_id,'due_on',due_on::text,'status',status,'completed_at',completed_at::text,'archived',archived_at IS NOT NULL,'version',version,'created_at',created_at::text,'updated_at',updated_at::text) FROM tasks.tasks";
pub(super) async fn get(db: &mut PgConnection, workspace: Uuid, id: Uuid) -> Result<Task, Error> {
    let sql = format!("{RECORD} WHERE workspace_id=$1 AND id=$2");
    Ok(
        sqlx::query_scalar::<_, Json<Task>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
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
    a: &Access,
    q: &TaskQuery,
    limit: i64,
) -> Result<Vec<Task>, Error> {
    let sql = format!(
        "{RECORD} WHERE workspace_id=$1 AND ($2 OR assigned_membership_id=$3) AND ($4::uuid IS NULL OR (created_at,id)>(SELECT created_at,id FROM tasks.tasks WHERE workspace_id=$1 AND id=$4)) AND ($5::text IS NULL OR strpos(lower(title),lower($5))>0) AND (archived_at IS NOT NULL)=$6 AND ($7::text IS NULL OR status=$7) AND ($8::uuid IS NULL OR assigned_membership_id=$8) AND ($9::text IS NULL OR due_on<=$9::text::date) ORDER BY created_at,id LIMIT $10"
    );
    Ok(
        sqlx::query_scalar::<_, Json<Task>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(a.workspace_wide)
            .bind(a.membership_id)
            .bind(q.after)
            .bind(&q.search)
            .bind(q.archived.unwrap_or(false))
            .bind(q.status.map(|s| s.key()))
            .bind(q.assigned_membership_id)
            .bind(&q.due_before)
            .bind(limit)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|r| r.0)
            .collect(),
    )
}
pub(super) async fn save(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Option<Uuid>,
    assignee: Uuid,
    input: &TaskInput,
) -> Result<Uuid, Error> {
    if let Some(id) = id {
        sqlx::query("UPDATE tasks.tasks SET title=$3,notes=$4,assigned_membership_id=$5,due_on=$6::text::date,status=$7,completed_at=CASE WHEN $7='completed' THEN COALESCE(completed_at,now()) ELSE NULL END,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(workspace).bind(id).bind(input.title.trim()).bind(&input.notes).bind(assignee).bind(&input.due_on).bind(input.status.key()).execute(db).await?;
        Ok(id)
    } else {
        Ok(sqlx::query_scalar("INSERT INTO tasks.tasks(workspace_id,title,notes,assigned_membership_id,due_on,status,completed_at) VALUES($1,$2,$3,$4,$5::text::date,$6,CASE WHEN $6='completed' THEN now() ELSE NULL END) RETURNING id").bind(workspace).bind(input.title.trim()).bind(&input.notes).bind(assignee).bind(&input.due_on).bind(input.status.key()).fetch_one(db).await?)
    }
}
pub(super) async fn archive(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    archived: bool,
) -> Result<(), Error> {
    sqlx::query("UPDATE tasks.tasks SET archived_at=CASE WHEN $3 THEN COALESCE(archived_at,now()) ELSE NULL END,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(workspace).bind(id).bind(archived).execute(db).await?;
    Ok(())
}
pub(super) async fn history_insert(
    db: &mut PgConnection,
    workspace: Uuid,
    task: &Task,
    audit: Uuid,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO tasks.task_changes(workspace_id,task_id,snapshot,audit_event_id) VALUES($1,$2,$3,$4)").bind(workspace).bind(task.id).bind(Json(task)).bind(audit).execute(db).await?;
    Ok(())
}
pub(super) async fn history(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    q: &TaskHistoryQuery,
    limit: i64,
) -> Result<Vec<TaskChange>, Error> {
    if let Some(after)=q.after && !sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM tasks.task_changes WHERE workspace_id=$1 AND task_id=$2 AND id=$3)").bind(workspace).bind(id).bind(after).fetch_one(&mut *db).await?{return Err(Error::NotFound);}
    Ok(sqlx::query_scalar::<_,Json<TaskChange>>("SELECT jsonb_build_object('id',id,'snapshot',snapshot,'audit_event_id',audit_event_id,'occurred_at',occurred_at::text) FROM tasks.task_changes WHERE workspace_id=$1 AND task_id=$2 AND ($3::uuid IS NULL OR (occurred_at,id)>(SELECT occurred_at,id FROM tasks.task_changes WHERE workspace_id=$1 AND task_id=$2 AND id=$3)) ORDER BY occurred_at,id LIMIT $4").bind(workspace).bind(id).bind(q.after).bind(limit).fetch_all(db).await?.into_iter().map(|r|r.0).collect())
}
