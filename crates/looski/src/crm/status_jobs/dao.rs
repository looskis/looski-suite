// SPDX-License-Identifier: AGPL-3.0-only
use super::{CoordinationRule, RuleQuery, SaveCoordinationRule};
use crate::application::OperationError as Error;
use sqlx::{PgConnection, Row, types::Json};
use uuid::Uuid;
const RULE: &str = "SELECT jsonb_build_object('id',id,'name',name,'status_id',status_id,'revision_id',revision_id,'archived',archived_at IS NOT NULL,'version',version) FROM crm.coordination_rules";
pub(super) async fn get(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
) -> Result<CoordinationRule, Error> {
    let sql = format!("{RULE} WHERE workspace_id=$1 AND id=$2");
    Ok(
        sqlx::query_scalar::<_, Json<CoordinationRule>>(sqlx::AssertSqlSafe(sql.as_str()))
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
    q: &RuleQuery,
    n: i64,
) -> Result<Vec<CoordinationRule>, Error> {
    if let Some(id) = q.after {
        get(db, ws, id).await?;
    }
    let sql = format!(
        "{RULE} WHERE workspace_id=$1 AND (archived_at IS NOT NULL)=$2 AND ($3::uuid IS NULL OR (created_at,id)>(SELECT created_at,id FROM crm.coordination_rules WHERE workspace_id=$1 AND id=$3)) ORDER BY created_at,id LIMIT $4"
    );
    Ok(
        sqlx::query_scalar::<_, Json<CoordinationRule>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(q.archived.unwrap_or(false))
            .bind(q.after)
            .bind(n)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|r| r.0)
            .collect(),
    )
}
pub(super) async fn active_status(db: &mut PgConnection, ws: Uuid, id: Uuid) -> Result<(), Error> {
    let active=sqlx::query_scalar::<_,bool>("SELECT s.archived_at IS NULL AND c.archived_at IS NULL FROM crm.deal_statuses s JOIN crm.deal_categories c ON (c.workspace_id,c.id)=(s.workspace_id,s.category_id) WHERE s.workspace_id=$1 AND s.id=$2").bind(ws).bind(id).fetch_optional(db).await?.ok_or(Error::NotFound)?;
    if !active {
        return Err(Error::Conflict("Status and category must be active"));
    }
    Ok(())
}
pub(super) async fn conflicting(
    db: &mut PgConnection,
    ws: Uuid,
    id: Option<Uuid>,
    status: Uuid,
) -> Result<bool, Error> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM crm.coordination_rules WHERE workspace_id=$1 AND status_id=$2 AND archived_at IS NULL AND ($3::uuid IS NULL OR id<>$3))").bind(ws).bind(status).bind(id).fetch_one(db).await?)
}
pub(super) async fn save(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    input: &SaveCoordinationRule,
    archived: bool,
    new: bool,
) -> Result<(), Error> {
    if new {
        sqlx::query("INSERT INTO crm.coordination_rules(workspace_id,id,name,status_id,revision_id,archived_at) VALUES($1,$2,$3,$4,$5,CASE WHEN $6 THEN now() END)").bind(ws).bind(id).bind(&input.name).bind(input.status_id).bind(input.revision_id).bind(archived).execute(db).await?;
    } else {
        sqlx::query("UPDATE crm.coordination_rules SET name=$3,status_id=$4,revision_id=$5,archived_at=CASE WHEN $6 THEN COALESCE(archived_at,now()) END,version=version+1 WHERE workspace_id=$1 AND id=$2").bind(ws).bind(id).bind(&input.name).bind(input.status_id).bind(input.revision_id).bind(archived).execute(db).await?;
    }
    Ok(())
}
pub(super) async fn active_rule(
    db: &mut PgConnection,
    ws: Uuid,
    status: Uuid,
) -> Result<Option<CoordinationRule>, Error> {
    let sql = format!("{RULE} WHERE workspace_id=$1 AND status_id=$2 AND archived_at IS NULL");
    Ok(
        sqlx::query_scalar::<_, Json<CoordinationRule>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(status)
            .fetch_optional(db)
            .await?
            .map(|r| r.0),
    )
}
pub(super) async fn insert_run(
    db: &mut PgConnection,
    ws: Uuid,
    job: Uuid,
    deal: Uuid,
    rule: &CoordinationRule,
    audit: Uuid,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO crm.coordination_runs(workspace_id,job_id,rule_id,deal_id,status_id,revision_id,transition_audit_id,base_on) VALUES($1,$2,$3,$4,$5,$6,$7,(now() AT TIME ZONE 'UTC')::date)").bind(ws).bind(job).bind(rule.id).bind(deal).bind(rule.status_id).bind(rule.revision_id).bind(audit).execute(db).await?;
    Ok(())
}
pub(super) struct Run {
    pub deal: Uuid,
    pub status: Uuid,
    pub revision: Uuid,
    pub audit: Uuid,
    pub base_on: String,
}
pub(super) async fn run(db: &mut PgConnection, ws: Uuid, job: Uuid) -> Result<Run, Error> {
    let r=sqlx::query("SELECT deal_id,status_id,revision_id,transition_audit_id,base_on::text FROM crm.coordination_runs WHERE workspace_id=$1 AND job_id=$2 AND instance_id IS NULL").bind(ws).bind(job).fetch_optional(db).await?.ok_or(Error::NotFound)?;
    Ok(Run {
        deal: r.try_get("deal_id")?,
        status: r.try_get("status_id")?,
        revision: r.try_get("revision_id")?,
        audit: r.try_get("transition_audit_id")?,
        base_on: r.try_get("base_on")?,
    })
}
pub(super) async fn is_current_transition(
    db: &mut PgConnection,
    ws: Uuid,
    deal: Uuid,
    audit: Uuid,
) -> Result<bool, Error> {
    Ok(sqlx::query_scalar::<_,bool>("SELECT audit_event_id=$3 FROM crm.deal_status_changes WHERE workspace_id=$1 AND deal_id=$2 ORDER BY transition_sequence DESC LIMIT 1").bind(ws).bind(deal).bind(audit).fetch_optional(db).await?.unwrap_or(false))
}
pub(super) async fn complete_run(
    db: &mut PgConnection,
    ws: Uuid,
    job: Uuid,
    instance: Uuid,
) -> Result<(), Error> {
    sqlx::query(
        "UPDATE crm.coordination_runs SET instance_id=$3 WHERE workspace_id=$1 AND job_id=$2",
    )
    .bind(ws)
    .bind(job)
    .bind(instance)
    .execute(db)
    .await?;
    Ok(())
}

pub(super) async fn context(
    db: &mut PgConnection,
    ws: Uuid,
    job: Uuid,
) -> Result<Option<(Uuid, Option<Uuid>)>, Error> {
    Ok(sqlx::query_as(
        "SELECT deal_id,instance_id FROM crm.coordination_runs WHERE workspace_id=$1 AND job_id=$2",
    )
    .bind(ws)
    .bind(job)
    .fetch_optional(db)
    .await?)
}
