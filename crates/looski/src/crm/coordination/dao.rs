// SPDX-License-Identifier: AGPL-3.0-only
use super::{
    ApplyCoordination, CoordinationInstanceSummary, CoordinationPageQuery, CoordinationQuery,
    CoordinationRevision, CoordinationTemplate, SaveCoordinationTemplate,
};
use crate::application::OperationError as Error;
use serde::Deserialize;
use sqlx::{PgConnection, types::Json};
use uuid::Uuid;
const REVISION: &str = "jsonb_build_object('id',r.id,'template_id',r.template_id,'number',r.number,'name',r.name,'created_at',r.created_at::text,'audit_event_id',r.audit_event_id,'items',COALESCE((SELECT jsonb_agg(jsonb_build_object('id',s.id,'position',s.position,'label',s.label,'task_template_revision_id',s.task_template_revision_id) ORDER BY s.position) FROM crm.coordination_steps s WHERE s.workspace_id=r.workspace_id AND s.revision_id=r.id),'[]'::jsonb))";
fn template_sql() -> String {
    format!(
        "SELECT jsonb_build_object('id',t.id,'name',t.name,'archived',t.archived_at IS NOT NULL,'version',t.version,'revision',{REVISION}) FROM crm.coordination_templates t JOIN crm.coordination_revisions r ON (r.workspace_id,r.template_id,r.number)=(t.workspace_id,t.id,t.current_revision)"
    )
}
pub(super) async fn template(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
) -> Result<CoordinationTemplate, Error> {
    let sql = format!("{} WHERE t.workspace_id=$1 AND t.id=$2", template_sql());
    Ok(
        sqlx::query_scalar::<_, Json<CoordinationTemplate>>(sqlx::AssertSqlSafe(sql.as_str()))
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
    q: &CoordinationQuery,
    limit: i64,
) -> Result<Vec<CoordinationTemplate>, Error> {
    if let Some(id) = q.after {
        template(db, workspace, id).await?;
    }
    let sql = format!(
        "{} WHERE t.workspace_id=$1 AND (t.archived_at IS NOT NULL)=$2 AND ($3::uuid IS NULL OR (t.created_at,t.id)>(SELECT created_at,id FROM crm.coordination_templates WHERE workspace_id=$1 AND id=$3)) ORDER BY t.created_at,t.id LIMIT $4",
        template_sql()
    );
    Ok(
        sqlx::query_scalar::<_, Json<CoordinationTemplate>>(sqlx::AssertSqlSafe(sql.as_str()))
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
pub(super) async fn revision(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
) -> Result<CoordinationRevision, Error> {
    let sql = format!(
        "SELECT {REVISION} FROM crm.coordination_revisions r WHERE r.workspace_id=$1 AND r.id=$2"
    );
    Ok(
        sqlx::query_scalar::<_, Json<CoordinationRevision>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
pub(super) async fn revisions(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    q: &CoordinationPageQuery,
    limit: i64,
) -> Result<Vec<CoordinationRevision>, Error> {
    if let Some(after) = q.after {
        let r = revision(db, workspace, after).await?;
        if r.template_id != id {
            return Err(Error::NotFound);
        }
    }
    let sql = format!(
        "SELECT {REVISION} FROM crm.coordination_revisions r WHERE r.workspace_id=$1 AND r.template_id=$2 AND ($3::uuid IS NULL OR r.number>(SELECT number FROM crm.coordination_revisions WHERE workspace_id=$1 AND template_id=$2 AND id=$3)) ORDER BY r.number LIMIT $4"
    );
    Ok(
        sqlx::query_scalar::<_, Json<CoordinationRevision>>(sqlx::AssertSqlSafe(sql.as_str()))
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
pub(super) async fn save(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    i: &SaveCoordinationTemplate,
    revision: i64,
    new: bool,
) -> Result<(), Error> {
    if new {
        sqlx::query("INSERT INTO crm.coordination_templates(workspace_id,id,name,current_revision,archived_at) VALUES($1,$2,$3,$4,CASE WHEN $5 THEN now() ELSE NULL END)").bind(workspace).bind(id).bind(&i.name).bind(revision).bind(i.archived.unwrap_or(false)).execute(db).await?;
    } else {
        sqlx::query("UPDATE crm.coordination_templates SET name=$3,current_revision=$4,archived_at=CASE WHEN $5 IS NULL THEN archived_at WHEN $5 THEN COALESCE(archived_at,now()) ELSE NULL END,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(workspace).bind(id).bind(&i.name).bind(revision).bind(i.archived).execute(db).await?;
    }
    Ok(())
}
pub(super) async fn insert_revision(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    number: i64,
    input: &SaveCoordinationTemplate,
    audit: Uuid,
) -> Result<(), Error> {
    let revision:Uuid=sqlx::query_scalar("INSERT INTO crm.coordination_revisions(workspace_id,template_id,number,name,audit_event_id) VALUES($1,$2,$3,$4,$5) RETURNING id").bind(workspace).bind(id).bind(number).bind(&input.name).bind(audit).fetch_one(&mut *db).await?;
    for (position, item) in input.items.iter().enumerate() {
        sqlx::query("INSERT INTO crm.coordination_steps(workspace_id,revision_id,position,label,task_template_revision_id) VALUES($1,$2,$3,$4,$5)").bind(workspace).bind(revision).bind(position as i32).bind(&item.label).bind(item.task_template_revision_id).execute(&mut *db).await?;
    }
    Ok(())
}
#[derive(Deserialize)]
pub(super) struct ItemRow {
    pub id: Uuid,
    pub label: String,
    pub task_id: Uuid,
    pub link_id: Uuid,
    pub link_ended: bool,
}
#[derive(Deserialize)]
pub(super) struct InstanceRow {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub revision_id: Uuid,
    pub template_name: String,
    pub base_on: String,
    pub created_at: String,
    pub items: Vec<ItemRow>,
}
const SUMMARY: &str = "'id',i.id,'deal_id',i.deal_id,'revision_id',i.revision_id,'template_name',r.name,'base_on',i.base_on::text,'created_at',i.created_at::text";
const INSTANCES: &str = "FROM crm.deal_coordination_instances i JOIN crm.coordination_revisions r ON (r.workspace_id,r.id)=(i.workspace_id,i.revision_id)";
pub(super) async fn instance(
    db: &mut PgConnection,
    workspace: Uuid,
    deal: Uuid,
    id: Uuid,
) -> Result<InstanceRow, Error> {
    let sql = format!(
        "SELECT jsonb_build_object({SUMMARY},'items',COALESCE((SELECT jsonb_agg(jsonb_build_object('id',ci.id,'label',s.label,'task_id',ci.task_id,'link_id',l.id,'link_ended',l.ended_at IS NOT NULL) ORDER BY s.position) FROM crm.deal_coordination_items ci JOIN crm.coordination_steps s ON (s.workspace_id,s.revision_id,s.id)=(ci.workspace_id,ci.revision_id,ci.step_id) JOIN crm.deal_tasks l ON (l.workspace_id,l.deal_id,l.task_id)=(ci.workspace_id,ci.deal_id,ci.task_id) WHERE ci.workspace_id=i.workspace_id AND ci.instance_id=i.id),'[]'::jsonb)) {INSTANCES} WHERE i.workspace_id=$1 AND i.deal_id=$2 AND i.id=$3"
    );
    Ok(
        sqlx::query_scalar::<_, Json<InstanceRow>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(deal)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
pub(super) async fn instances(
    db: &mut PgConnection,
    workspace: Uuid,
    deal: Uuid,
    q: &CoordinationPageQuery,
    limit: i64,
) -> Result<Vec<CoordinationInstanceSummary>, Error> {
    if let Some(after)=q.after&&!sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM crm.deal_coordination_instances WHERE workspace_id=$1 AND deal_id=$2 AND id=$3)").bind(workspace).bind(deal).bind(after).fetch_one(&mut *db).await?{return Err(Error::NotFound)}
    let sql = format!(
        "SELECT jsonb_build_object({SUMMARY}) {INSTANCES} WHERE i.workspace_id=$1 AND i.deal_id=$2 AND ($3::uuid IS NULL OR (i.created_at,i.id)>(SELECT created_at,id FROM crm.deal_coordination_instances WHERE workspace_id=$1 AND deal_id=$2 AND id=$3)) ORDER BY i.created_at,i.id LIMIT $4"
    );
    Ok(
        sqlx::query_scalar::<_, Json<CoordinationInstanceSummary>>(sqlx::AssertSqlSafe(
            sql.as_str(),
        ))
        .bind(workspace)
        .bind(deal)
        .bind(q.after)
        .bind(limit)
        .fetch_all(db)
        .await?
        .into_iter()
        .map(|r| r.0)
        .collect(),
    )
}
pub(super) async fn insert_instance(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    deal: Uuid,
    input: &ApplyCoordination,
    member: Uuid,
    audit: Uuid,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO crm.deal_coordination_instances(workspace_id,id,deal_id,revision_id,base_on,created_by_membership_id,audit_event_id) VALUES($1,$2,$3,$4,$5::text::date,$6,$7)").bind(workspace).bind(id).bind(deal).bind(input.revision_id).bind(&input.base_on).bind(member).bind(audit).execute(db).await?;
    Ok(())
}
pub(super) async fn insert_item(
    db: &mut PgConnection,
    workspace: Uuid,
    instance: Uuid,
    deal: Uuid,
    revision: Uuid,
    step: Uuid,
    task: Uuid,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO crm.deal_coordination_items(workspace_id,instance_id,deal_id,revision_id,step_id,task_id) VALUES($1,$2,$3,$4,$5,$6)").bind(workspace).bind(instance).bind(deal).bind(revision).bind(step).bind(task).execute(db).await?;
    Ok(())
}
