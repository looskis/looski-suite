// SPDX-License-Identifier: AGPL-3.0-only
use super::{
    Deal, DealInput, DealQuery, HistoryQuery, Opportunity, OpportunityInput, OpportunityQuery,
    Transition,
    configuration::{Definition, Outcome},
};
use crate::{access::Access, application::OperationError as Error};
use sqlx::{PgConnection, types::Json};
use uuid::Uuid;
const OPPORTUNITY: &str = "SELECT jsonb_build_object('id',o.id,'title',o.title,'notes',o.notes,'owner_membership_id',o.owner_membership_id,'pipeline_id',o.pipeline_id,'pipeline_name',p.name,'stage_id',o.stage_id,'stage_name',s.name,'outcome',s.outcome,'expected_value',o.expected_value::text,'currency_code',o.currency_code,'expected_close_on',o.expected_close_on::text,'closed_at',o.closed_at::text,'archived',o.archived_at IS NOT NULL,'version',o.version,'created_at',o.created_at::text,'updated_at',o.updated_at::text) FROM crm.opportunities o JOIN crm.opportunity_pipelines p ON (p.workspace_id,p.id)=(o.workspace_id,o.pipeline_id) JOIN crm.opportunity_stages s ON (s.workspace_id,s.id)=(o.workspace_id,o.stage_id)";
const DEAL: &str = "SELECT jsonb_build_object('id',d.id,'title',d.title,'notes',d.notes,'owner_membership_id',d.owner_membership_id,'category_id',d.category_id,'category_name',c.name,'status_id',d.status_id,'status_name',s.name,'originating_opportunity_id',d.originating_opportunity_id,'value',d.value::text,'currency_code',d.currency_code,'starts_on',d.starts_on::text,'ends_on',d.ends_on::text,'archived',d.archived_at IS NOT NULL,'version',d.version,'created_at',d.created_at::text,'updated_at',d.updated_at::text) FROM crm.deals d JOIN crm.deal_categories c ON (c.workspace_id,c.id)=(d.workspace_id,d.category_id) JOIN crm.deal_statuses s ON (s.workspace_id,s.id)=(d.workspace_id,d.status_id)";
pub(super) async fn opportunity(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
) -> Result<Opportunity, Error> {
    let sql = format!("{OPPORTUNITY} WHERE o.workspace_id=$1 AND o.id=$2");
    Ok(
        sqlx::query_scalar::<_, Json<Opportunity>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
pub(super) async fn deal(db: &mut PgConnection, workspace: Uuid, id: Uuid) -> Result<Deal, Error> {
    let sql = format!("{DEAL} WHERE d.workspace_id=$1 AND d.id=$2");
    Ok(
        sqlx::query_scalar::<_, Json<Deal>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
pub(super) async fn opportunities(
    db: &mut PgConnection,
    workspace: Uuid,
    a: &Access,
    q: &OpportunityQuery,
    limit: i64,
) -> Result<Vec<Opportunity>, Error> {
    let sql = format!(
        "{OPPORTUNITY} WHERE o.workspace_id=$1 AND ($2 OR o.owner_membership_id=$3) AND ($4::uuid IS NULL OR (o.created_at,o.id)>(SELECT created_at,id FROM crm.opportunities WHERE workspace_id=$1 AND id=$4)) AND ($5::text IS NULL OR strpos(lower(o.title),lower($5))>0) AND (o.archived_at IS NOT NULL)=$6 AND ($7::uuid IS NULL OR o.pipeline_id=$7) AND ($8::uuid IS NULL OR o.stage_id=$8) AND ($10::uuid IS NULL OR EXISTS(SELECT 1 FROM crm.opportunity_tags t WHERE t.workspace_id=o.workspace_id AND t.opportunity_id=o.id AND t.tag_id=$10)) ORDER BY o.created_at,o.id LIMIT $9"
    );
    Ok(
        sqlx::query_scalar::<_, Json<Opportunity>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(a.workspace_wide)
            .bind(a.membership_id)
            .bind(q.after)
            .bind(&q.search)
            .bind(q.archived.unwrap_or(false))
            .bind(q.pipeline_id)
            .bind(q.stage_id)
            .bind(limit)
            .bind(q.tag_id)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|r| r.0)
            .collect(),
    )
}
pub(super) async fn deals(
    db: &mut PgConnection,
    workspace: Uuid,
    a: &Access,
    q: &DealQuery,
    limit: i64,
) -> Result<Vec<Deal>, Error> {
    let sql = format!(
        "{DEAL} WHERE d.workspace_id=$1 AND ($2 OR d.owner_membership_id=$3) AND ($4::uuid IS NULL OR (d.created_at,d.id)>(SELECT created_at,id FROM crm.deals WHERE workspace_id=$1 AND id=$4)) AND ($5::text IS NULL OR strpos(lower(d.title),lower($5))>0) AND (d.archived_at IS NOT NULL)=$6 AND ($7::uuid IS NULL OR d.category_id=$7) AND ($8::uuid IS NULL OR d.status_id=$8) AND ($9::uuid IS NULL OR d.originating_opportunity_id=$9) AND ($11::uuid IS NULL OR EXISTS(SELECT 1 FROM crm.deal_tags t WHERE t.workspace_id=d.workspace_id AND t.deal_id=d.id AND t.tag_id=$11)) ORDER BY d.created_at,d.id LIMIT $10"
    );
    Ok(
        sqlx::query_scalar::<_, Json<Deal>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(a.workspace_wide)
            .bind(a.membership_id)
            .bind(q.after)
            .bind(&q.search)
            .bind(q.archived.unwrap_or(false))
            .bind(q.category_id)
            .bind(q.status_id)
            .bind(q.originating_opportunity_id)
            .bind(limit)
            .bind(q.tag_id)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|r| r.0)
            .collect(),
    )
}
pub(super) async fn visible_opportunities(
    db: &mut PgConnection,
    workspace: Uuid,
    a: &Access,
    ids: &[Uuid],
) -> Result<Vec<Uuid>, Error> {
    Ok(sqlx::query_scalar("SELECT id FROM crm.opportunities WHERE workspace_id=$1 AND ($2 OR owner_membership_id=$3) AND id=ANY($4)").bind(workspace).bind(a.workspace_wide).bind(a.membership_id).bind(ids).fetch_all(db).await?)
}
pub(super) async fn save_opportunity(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Option<Uuid>,
    owner: Uuid,
    input: &OpportunityInput,
    outcome: Outcome,
    unchanged: bool,
) -> Result<Uuid, Error> {
    if let Some(id) = id {
        sqlx::query("UPDATE crm.opportunities SET title=$3,notes=$4,owner_membership_id=$5,pipeline_id=$6,stage_id=$7,expected_value=$8::text::numeric,currency_code=$9,expected_close_on=$10::text::date,closed_at=CASE WHEN $11 THEN closed_at WHEN $12='open' THEN NULL ELSE now() END,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(workspace).bind(id).bind(input.title.trim()).bind(&input.notes).bind(owner).bind(input.pipeline_id).bind(input.stage_id).bind(&input.expected_value).bind(&input.currency_code).bind(&input.expected_close_on).bind(unchanged).bind(outcome.key()).execute(db).await?;
        Ok(id)
    } else {
        Ok(sqlx::query_scalar("INSERT INTO crm.opportunities(workspace_id,title,notes,owner_membership_id,pipeline_id,stage_id,expected_value,currency_code,expected_close_on,closed_at) VALUES($1,$2,$3,$4,$5,$6,$7::text::numeric,$8,$9::text::date,CASE WHEN $10='open' THEN NULL ELSE now() END) RETURNING id").bind(workspace).bind(input.title.trim()).bind(&input.notes).bind(owner).bind(input.pipeline_id).bind(input.stage_id).bind(&input.expected_value).bind(&input.currency_code).bind(&input.expected_close_on).bind(outcome.key()).fetch_one(db).await?)
    }
}
pub(super) async fn save_deal(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Option<Uuid>,
    owner: Uuid,
    input: &DealInput,
    origin: Option<Uuid>,
) -> Result<Uuid, Error> {
    if let Some(id) = id {
        sqlx::query("UPDATE crm.deals SET title=$3,notes=$4,owner_membership_id=$5,category_id=$6,status_id=$7,value=$8::text::numeric,currency_code=$9,starts_on=$10::text::date,ends_on=$11::text::date,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(workspace).bind(id).bind(input.title.trim()).bind(&input.notes).bind(owner).bind(input.category_id).bind(input.status_id).bind(&input.value).bind(&input.currency_code).bind(&input.starts_on).bind(&input.ends_on).execute(db).await?;
        Ok(id)
    } else {
        Ok(sqlx::query_scalar("INSERT INTO crm.deals(workspace_id,title,notes,owner_membership_id,category_id,status_id,value,currency_code,starts_on,ends_on,originating_opportunity_id) VALUES($1,$2,$3,$4,$5,$6,$7::text::numeric,$8,$9::text::date,$10::text::date,$11) RETURNING id").bind(workspace).bind(input.title.trim()).bind(&input.notes).bind(owner).bind(input.category_id).bind(input.status_id).bind(&input.value).bind(&input.currency_code).bind(&input.starts_on).bind(&input.ends_on).bind(origin).fetch_one(db).await?)
    }
}
pub(super) async fn opportunity_transition(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    previous: Option<&Opportunity>,
    state: &Definition,
    audit: Uuid,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO crm.opportunity_stage_changes(workspace_id,opportunity_id,from_stage_id,to_stage_id,from_label,to_label,from_outcome,to_outcome,audit_event_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)").bind(workspace).bind(id).bind(previous.map(|p|p.stage_id)).bind(state.id).bind(previous.map(|p|&p.stage_name)).bind(&state.name).bind(previous.map(|p|p.outcome.key())).bind(state.outcome.map(|o|o.key())).bind(audit).execute(db).await?;
    Ok(())
}
pub(super) async fn deal_transition(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    previous: Option<&Deal>,
    state: &Definition,
    audit: Uuid,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO crm.deal_status_changes(workspace_id,deal_id,from_status_id,to_status_id,from_label,to_label,audit_event_id) VALUES($1,$2,$3,$4,$5,$6,$7)").bind(workspace).bind(id).bind(previous.map(|p|p.status_id)).bind(state.id).bind(previous.map(|p|&p.status_name)).bind(&state.name).bind(audit).execute(db).await?;
    Ok(())
}
pub(super) async fn history(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    is_deal: bool,
    q: &HistoryQuery,
    limit: i64,
) -> Result<Vec<Transition>, Error> {
    let (table, parent, from, to, from_outcome, to_outcome) = if is_deal {
        (
            "crm.deal_status_changes",
            "deal_id",
            "from_status_id",
            "to_status_id",
            "NULL::text",
            "NULL::text",
        )
    } else {
        (
            "crm.opportunity_stage_changes",
            "opportunity_id",
            "from_stage_id",
            "to_stage_id",
            "from_outcome",
            "to_outcome",
        )
    };
    if let Some(after) = q.after {
        let sql = format!(
            "SELECT EXISTS(SELECT 1 FROM {table} WHERE workspace_id=$1 AND {parent}=$2 AND id=$3)"
        );
        if !sqlx::query_scalar::<_, bool>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(id)
            .bind(after)
            .fetch_one(&mut *db)
            .await?
        {
            return Err(Error::NotFound);
        }
    }
    let sql = format!(
        "SELECT jsonb_build_object('id',id,'from_id',{from},'to_id',{to},'from_label',from_label,'to_label',to_label,'from_outcome',{from_outcome},'to_outcome',{to_outcome},'audit_event_id',audit_event_id,'occurred_at',occurred_at::text) FROM {table} WHERE workspace_id=$1 AND {parent}=$2 AND ($3::uuid IS NULL OR (occurred_at,id)>(SELECT occurred_at,id FROM {table} WHERE workspace_id=$1 AND {parent}=$2 AND id=$3)) ORDER BY occurred_at,id LIMIT $4"
    );
    Ok(
        sqlx::query_scalar::<_, Json<Transition>>(sqlx::AssertSqlSafe(sql.as_str()))
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
pub(super) async fn archive(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    is_deal: bool,
    archived: bool,
) -> Result<(), Error> {
    let table = if is_deal {
        "crm.deals"
    } else {
        "crm.opportunities"
    };
    let sql = format!(
        "UPDATE {table} SET archived_at=CASE WHEN $3 THEN COALESCE(archived_at,now()) ELSE NULL END,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2"
    );
    sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(workspace)
        .bind(id)
        .bind(archived)
        .execute(db)
        .await?;
    Ok(())
}
