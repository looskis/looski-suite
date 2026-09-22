// SPDX-License-Identifier: AGPL-3.0-only
use super::{RemovalDecision, StarterQuery, StarterSummary};
use crate::application::OperationError as Error;
use serde::Deserialize;
use sqlx::{PgConnection, types::Json};
use uuid::Uuid;
#[derive(Deserialize)]
pub(super) struct References {
    pub pipeline: Uuid,
    pub category: Uuid,
    pub note: Uuid,
    pub interaction: Uuid,
    pub task_type: Uuid,
    pub task_template: Uuid,
    pub coordination: Uuid,
    pub rule: Uuid,
    pub stages: Vec<Uuid>,
    pub statuses: Vec<Uuid>,
}
#[derive(Deserialize)]
pub(super) struct Stored {
    #[serde(flatten)]
    pub summary: StarterSummary,
    #[serde(flatten)]
    pub refs: References,
    pub removal: Option<Vec<RemovalDecision>>,
}

const SUMMARY: &str = "'id',i.id,'package_version',i.package_version,'version',i.version,'installed_at',i.installed_at::text,'removed',i.removed_at IS NOT NULL";
pub(super) async fn get(db: &mut PgConnection, ws: Uuid, id: Uuid) -> Result<Stored, Error> {
    let sql = format!(
        "SELECT jsonb_build_object({SUMMARY},'pipeline',i.pipeline_id,'category',i.category_id,'note',i.note_type_id,'interaction',i.interaction_type_id,'task_type',i.task_type_id,'task_template',i.task_template_id,'coordination',i.coordination_template_id,'rule',i.rule_id,'stages',(SELECT jsonb_agg(stage_id ORDER BY position) FROM crm.starter_stages WHERE workspace_id=$1 AND installation_id=i.id),'statuses',(SELECT jsonb_agg(status_id ORDER BY position) FROM crm.starter_statuses WHERE workspace_id=$1 AND installation_id=i.id),'removal',removal_summary) FROM crm.starter_installations i WHERE i.workspace_id=$1 AND i.id=$2"
    );
    Ok(
        sqlx::query_scalar::<_, Json<Stored>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
pub(super) async fn list(
    db: &mut PgConnection,
    ws: Uuid,
    q: &StarterQuery,
    n: i64,
) -> Result<Vec<StarterSummary>, Error> {
    if let Some(id) = q.after {
        get(db, ws, id).await?;
    }
    let sql = format!(
        "SELECT jsonb_build_object({SUMMARY}) FROM crm.starter_installations i WHERE i.workspace_id=$1 AND ($2::uuid IS NULL OR (installed_at,id)>(SELECT installed_at,id FROM crm.starter_installations WHERE workspace_id=$1 AND id=$2)) ORDER BY installed_at,id LIMIT $3"
    );
    Ok(
        sqlx::query_scalar::<_, Json<StarterSummary>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(q.after)
            .bind(n)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|r| r.0)
            .collect(),
    )
}
pub(super) async fn active(db: &mut PgConnection, ws: Uuid) -> Result<bool, Error> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM crm.starter_installations WHERE workspace_id=$1 AND removed_at IS NULL)").bind(ws).fetch_one(db).await?)
}
pub(super) async fn insert(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    r: &References,
    audit: Uuid,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO crm.starter_installations(workspace_id,id,package_version,installed_audit_id,pipeline_id,category_id,note_type_id,interaction_type_id,task_type_id,task_template_id,coordination_template_id,rule_id) VALUES($1,$2,1,$3,$4,$5,$6,$7,$8,$9,$10,$11)").bind(ws).bind(id).bind(audit).bind(r.pipeline).bind(r.category).bind(r.note).bind(r.interaction).bind(r.task_type).bind(r.task_template).bind(r.coordination).bind(r.rule).execute(&mut *db).await?;
    for (position, stage) in r.stages.iter().enumerate() {
        sqlx::query("INSERT INTO crm.starter_stages(workspace_id,installation_id,stage_id,position) VALUES($1,$2,$3,$4)").bind(ws).bind(id).bind(stage).bind(position as i32).execute(&mut *db).await?;
    }
    for (position, status) in r.statuses.iter().enumerate() {
        sqlx::query("INSERT INTO crm.starter_statuses(workspace_id,installation_id,status_id,position) VALUES($1,$2,$3,$4)").bind(ws).bind(id).bind(status).bind(position as i32).execute(&mut *db).await?;
    }
    Ok(())
}
pub(super) async fn finish_removal(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    audit: Uuid,
    summary: &[RemovalDecision],
) -> Result<(), Error> {
    sqlx::query("UPDATE crm.starter_installations SET removed_at=now(),removed_audit_id=$3,removal_summary=$4,version=version+1 WHERE workspace_id=$1 AND id=$2").bind(ws).bind(id).bind(audit).bind(Json(summary)).execute(db).await?;
    Ok(())
}
pub(super) async fn active_rule_for_template(
    db: &mut PgConnection,
    ws: Uuid,
    template: Uuid,
) -> Result<bool, Error> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM crm.coordination_rules q JOIN crm.coordination_revisions r ON (r.workspace_id,r.id)=(q.workspace_id,q.revision_id) WHERE q.workspace_id=$1 AND r.template_id=$2 AND q.archived_at IS NULL)").bind(ws).bind(template).fetch_one(db).await?)
}
pub(super) async fn active_checklist_for_task_revision(
    db: &mut PgConnection,
    ws: Uuid,
    revision: Uuid,
) -> Result<bool, Error> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM crm.coordination_templates t JOIN crm.coordination_revisions r ON (r.workspace_id,r.template_id)=(t.workspace_id,t.id) JOIN crm.coordination_steps s ON (s.workspace_id,s.revision_id)=(r.workspace_id,r.id) WHERE t.workspace_id=$1 AND t.archived_at IS NULL AND s.task_template_revision_id=$2)").bind(ws).bind(revision).fetch_one(db).await?)
}
pub(super) async fn active_rule_for_status(
    db: &mut PgConnection,
    ws: Uuid,
    status: Uuid,
) -> Result<bool, Error> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM crm.coordination_rules WHERE workspace_id=$1 AND status_id=$2 AND archived_at IS NULL)").bind(ws).bind(status).fetch_one(db).await?)
}
pub(super) async fn active_children(
    db: &mut PgConnection,
    ws: Uuid,
    kind: super::cfg::Kind,
    parent: Uuid,
) -> Result<bool, Error> {
    let query = match kind {
        super::cfg::Kind::Pipeline => {
            "SELECT EXISTS(SELECT 1 FROM crm.opportunity_stages WHERE workspace_id=$1 AND pipeline_id=$2 AND archived_at IS NULL)"
        }
        super::cfg::Kind::Category => {
            "SELECT EXISTS(SELECT 1 FROM crm.deal_statuses WHERE workspace_id=$1 AND category_id=$2 AND archived_at IS NULL)"
        }
        _ => return Ok(false),
    };
    Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(query))
        .bind(ws)
        .bind(parent)
        .fetch_one(db)
        .await?)
}
