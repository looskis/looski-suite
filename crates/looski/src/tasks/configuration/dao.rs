// SPDX-License-Identifier: AGPL-3.0-only
use super::{
    SaveTaskTemplate, SaveTaskType, TaskConfigurationQuery, TaskRevisionQuery, TaskTemplate,
    TaskTemplateRevision, TaskType, TemplateFields,
};
use crate::application::OperationError as Error;
use sqlx::{PgConnection, types::Json};
use uuid::Uuid;
const TYPE: &str = "SELECT jsonb_build_object('id',id,'name',name,'position',position,'archived',archived_at IS NOT NULL,'version',version) FROM tasks.task_types";
const REVISION: &str = "jsonb_build_object('id',r.id,'template_id',r.template_id,'number',r.number,'title',r.title,'notes',r.notes,'type_id',r.type_id,'type_name',r.type_name,'assignment',r.assignment,'fixed_membership_id',r.fixed_membership_id,'due_offset_days',r.due_offset_days,'audit_event_id',r.audit_event_id,'created_at',r.created_at::text)";
fn template_sql() -> String {
    format!(
        "SELECT jsonb_build_object('id',t.id,'name',t.name,'archived',t.archived_at IS NOT NULL,'version',t.version,'revision',{REVISION}) FROM tasks.templates t JOIN tasks.template_revisions r ON (r.workspace_id,r.template_id,r.number)=(t.workspace_id,t.id,t.current_revision)"
    )
}
pub(super) async fn task_type(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
) -> Result<TaskType, Error> {
    let sql = format!("{TYPE} WHERE workspace_id=$1 AND id=$2");
    Ok(
        sqlx::query_scalar::<_, Json<TaskType>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
pub(super) async fn types(
    db: &mut PgConnection,
    workspace: Uuid,
    q: &TaskConfigurationQuery,
    limit: i64,
) -> Result<Vec<TaskType>, Error> {
    if let Some(id) = q.after {
        task_type(db, workspace, id).await?;
    }
    let sql = format!(
        "{TYPE} WHERE workspace_id=$1 AND (archived_at IS NOT NULL)=$2 AND ($3::uuid IS NULL OR (position,created_at,id)>(SELECT position,created_at,id FROM tasks.task_types WHERE workspace_id=$1 AND id=$3)) ORDER BY position,created_at,id LIMIT $4"
    );
    Ok(
        sqlx::query_scalar::<_, Json<TaskType>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(q.archived.unwrap_or(false))
            .bind(q.after)
            .bind(limit)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|r| r.0)
            .collect(),
    )
}
pub(super) async fn save_type(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Option<Uuid>,
    i: &SaveTaskType,
) -> Result<Uuid, Error> {
    if let Some(id) = id {
        sqlx::query("UPDATE tasks.task_types SET name=$3,position=COALESCE($4,position),archived_at=CASE WHEN $5 IS NULL THEN archived_at WHEN $5 THEN COALESCE(archived_at,now()) ELSE NULL END,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(workspace).bind(id).bind(i.name.trim()).bind(i.position).bind(i.archived).execute(db).await?;
        Ok(id)
    } else {
        Ok(sqlx::query_scalar("INSERT INTO tasks.task_types(workspace_id,name,position,archived_at) VALUES($1,$2,$3,CASE WHEN $4 THEN now() ELSE NULL END) RETURNING id").bind(workspace).bind(i.name.trim()).bind(i.position.unwrap_or(0)).bind(i.archived.unwrap_or(false)).fetch_one(db).await?)
    }
}
pub(super) async fn template(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
) -> Result<TaskTemplate, Error> {
    let sql = format!("{} WHERE t.workspace_id=$1 AND t.id=$2", template_sql());
    Ok(
        sqlx::query_scalar::<_, Json<TaskTemplate>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
pub(super) async fn templates(
    db: &mut PgConnection,
    workspace: Uuid,
    q: &TaskConfigurationQuery,
    limit: i64,
) -> Result<Vec<TaskTemplate>, Error> {
    if let Some(id) = q.after {
        template(db, workspace, id).await?;
    }
    let sql = format!(
        "{} WHERE t.workspace_id=$1 AND (t.archived_at IS NOT NULL)=$2 AND ($3::uuid IS NULL OR (t.created_at,t.id)>(SELECT created_at,id FROM tasks.templates WHERE workspace_id=$1 AND id=$3)) ORDER BY t.created_at,t.id LIMIT $4",
        template_sql()
    );
    Ok(
        sqlx::query_scalar::<_, Json<TaskTemplate>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(q.archived.unwrap_or(false))
            .bind(q.after)
            .bind(limit)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|r| r.0)
            .collect(),
    )
}
pub(super) async fn revisions(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    q: &TaskRevisionQuery,
    limit: i64,
) -> Result<Vec<TaskTemplateRevision>, Error> {
    if let Some(after)=q.after&&!sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM tasks.template_revisions WHERE workspace_id=$1 AND template_id=$2 AND id=$3)").bind(workspace).bind(id).bind(after).fetch_one(&mut *db).await?{return Err(Error::NotFound)}
    let sql = format!(
        "SELECT {REVISION} FROM tasks.template_revisions r WHERE r.workspace_id=$1 AND r.template_id=$2 AND ($3::uuid IS NULL OR r.number>(SELECT number FROM tasks.template_revisions WHERE workspace_id=$1 AND template_id=$2 AND id=$3)) ORDER BY r.number LIMIT $4"
    );
    Ok(
        sqlx::query_scalar::<_, Json<TaskTemplateRevision>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(id)
            .bind(q.after)
            .bind(limit)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|r| r.0)
            .collect(),
    )
}
pub(super) async fn save_template(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    i: &SaveTaskTemplate,
    revision: i64,
    new: bool,
) -> Result<(), Error> {
    if new {
        sqlx::query("INSERT INTO tasks.templates(workspace_id,id,name,current_revision,archived_at) VALUES($1,$2,$3,$4,CASE WHEN $5 THEN now() ELSE NULL END)").bind(workspace).bind(id).bind(i.name.trim()).bind(revision).bind(i.archived.unwrap_or(false)).execute(db).await?;
    } else {
        sqlx::query("UPDATE tasks.templates SET name=$3,current_revision=$4,archived_at=CASE WHEN $5 IS NULL THEN archived_at WHEN $5 THEN COALESCE(archived_at,now()) ELSE NULL END,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(workspace).bind(id).bind(i.name.trim()).bind(revision).bind(i.archived).execute(db).await?;
    }
    Ok(())
}
#[allow(clippy::too_many_arguments)]
pub(super) async fn insert_revision(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    number: i64,
    f: &TemplateFields,
    type_name: Option<&str>,
    audit: Uuid,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO tasks.template_revisions(workspace_id,template_id,number,title,notes,type_id,type_name,assignment,fixed_membership_id,due_offset_days,audit_event_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)").bind(workspace).bind(id).bind(number).bind(f.title.trim()).bind(&f.notes).bind(f.type_id).bind(type_name).bind(f.assignment.key()).bind(f.fixed_membership_id).bind(f.due_offset_days).bind(audit).execute(db).await?;
    Ok(())
}

pub(super) async fn revision(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
) -> Result<TaskTemplateRevision, Error> {
    let sql = format!(
        "SELECT {REVISION} FROM tasks.template_revisions r WHERE workspace_id=$1 AND id=$2"
    );
    Ok(
        sqlx::query_scalar::<_, Json<TaskTemplateRevision>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
