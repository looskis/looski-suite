// SPDX-License-Identifier: AGPL-3.0-only
use super::{AnchorKind, TimelineQuery};
use crate::{application::OperationError as Error, history::Position};
use serde::Deserialize;
use sqlx::{PgConnection, types::Json};
use std::collections::HashSet;
use uuid::Uuid;
#[derive(Deserialize)]
pub(super) struct Candidate {
    pub position: Position,
    pub source: String,
    pub detail: serde_json::Value,
}
fn tables(kind: AnchorKind) -> Option<(&'static str, &'static str, &'static str, &'static str)> {
    match kind {
        AnchorKind::Contact => None,
        AnchorKind::Opportunity => Some((
            "crm.opportunity_stage_changes",
            "opportunity_id",
            "crm.opportunity_tasks",
            "crm.opportunity_task_changes",
        )),
        AnchorKind::Deal => Some((
            "crm.deal_status_changes",
            "deal_id",
            "crm.deal_tasks",
            "crm.deal_task_changes",
        )),
    }
}
pub(super) async fn candidates(
    db: &mut PgConnection,
    ws: Uuid,
    q: &TimelineQuery,
    family: &[Uuid],
    after: Option<&Position>,
    exact: Option<(&str, Uuid)>,
    visibility: &super::Sources,
) -> Result<Vec<Candidate>, Error> {
    let column = match q.anchor_kind {
        AnchorKind::Contact => "contact_id",
        AnchorKind::Opportunity => "opportunity_id",
        AnchorKind::Deal => "deal_id",
    };
    let mut sources = format!(
        "SELECT c.id,c.occurred_at,'activity'::text AS source,jsonb_build_object('id',c.id,'snapshot',c.snapshot,'audit_event_id',c.audit_event_id,'occurred_at',c.occurred_at::text) AS detail FROM crm.activity_changes c JOIN crm.activities a ON (a.workspace_id,a.id)=(c.workspace_id,c.activity_id) WHERE c.workspace_id=$1 AND a.{column}=ANY($2) AND $8"
    );
    if let Some((changes, parent, links, link_changes)) = tables(q.anchor_kind) {
        let (from, to, from_outcome, to_outcome) = if q.anchor_kind == AnchorKind::Deal {
            ("from_status_id", "to_status_id", "NULL::text", "NULL::text")
        } else {
            ("from_stage_id", "to_stage_id", "from_outcome", "to_outcome")
        };
        sources.push_str(&format!(" UNION ALL SELECT id,occurred_at,'transition'::text,jsonb_build_object('id',id,'from_id',{from},'to_id',{to},'from_label',from_label,'to_label',to_label,'from_outcome',{from_outcome},'to_outcome',{to_outcome},'audit_event_id',audit_event_id,'occurred_at',occurred_at::text) FROM {changes} WHERE workspace_id=$1 AND {parent}=ANY($2) UNION ALL SELECT c.id,c.occurred_at,'task_link'::text,jsonb_build_object('link_id',l.id,'task_id',l.task_id,'task_title','','ended',c.ended,'audit_event_id',c.audit_event_id) FROM {link_changes} c JOIN {links} l ON (l.workspace_id,l.id)=(c.workspace_id,c.link_id) WHERE c.workspace_id=$1 AND l.{parent}=ANY($2) AND $9"));
    }
    let sql = format!(
        "SELECT jsonb_build_object('position',jsonb_build_object('id',id,'source',source,'recorded_at',to_char(occurred_at AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"')),'source',source,'detail',detail) FROM ({sources}) events WHERE (source<>'task_link' OR $9) AND ($3::text IS NULL OR (occurred_at,source,id)<($3::timestamptz,$5::text,$4::uuid)) AND ($6::text IS NULL OR (source=$6 AND id=$7)) ORDER BY occurred_at DESC,source DESC,id DESC LIMIT 100"
    );
    Ok(
        sqlx::query_scalar::<_, Json<Candidate>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(family)
            .bind(after.map(|p| p.recorded_at.as_str()))
            .bind(after.map(|p| p.id))
            .bind(after.map(|p| p.source.as_str()))
            .bind(exact.map(|e| e.0))
            .bind(exact.map(|e| e.1))
            .bind(visibility.activities)
            .bind(visibility.tasks)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|r| r.0)
            .collect(),
    )
}
pub(super) async fn linked_tasks(
    db: &mut PgConnection,
    ws: Uuid,
    kind: AnchorKind,
    parent_id: Uuid,
    ids: &[Uuid],
) -> Result<HashSet<Uuid>, Error> {
    if ids.len() > 100 {
        return Err(Error::Invalid("Task reference batch exceeds 100"));
    }
    let Some((_, parent, table, _)) = tables(kind) else {
        return Ok(HashSet::new());
    };
    let sql = format!(
        "SELECT task_id FROM {table} WHERE workspace_id=$1 AND {parent}=$2 AND task_id=ANY($3)"
    );
    Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(ws)
        .bind(parent_id)
        .bind(ids)
        .fetch_all(db)
        .await?
        .into_iter()
        .collect())
}

pub(super) async fn task_ids(
    db: &mut PgConnection,
    ws: Uuid,
    kind: AnchorKind,
    parent_id: Uuid,
    after: Option<Uuid>,
) -> Result<Vec<Uuid>, Error> {
    let Some((_, parent, table, _)) = tables(kind) else {
        return Ok(Vec::new());
    };
    let sql = format!(
        "SELECT task_id FROM {table} WHERE workspace_id=$1 AND {parent}=$2 AND ($3::uuid IS NULL OR task_id>$3) ORDER BY task_id LIMIT 100"
    );
    Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(ws)
        .bind(parent_id)
        .bind(after)
        .fetch_all(db)
        .await?)
}
