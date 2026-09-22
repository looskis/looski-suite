// SPDX-License-Identifier: AGPL-3.0-only
use super::{ParticipantChange, Target};
use crate::{application::OperationError as Error, crm::HistoryQuery};
use serde::Deserialize;
use sqlx::{PgConnection, types::Json};
use uuid::Uuid;
#[derive(Deserialize)]
pub(super) struct Row {
    pub id: Uuid,
    pub reference_id: Uuid,
    pub role_label: String,
    pub is_primary: bool,
    pub ended: bool,
    pub created_at: String,
    pub updated_at: String,
}
fn columns(target: Target) -> (&'static str, &'static str) {
    match target {
        Target::OpportunityContact => ("opportunity_id", "contact_id"),
        Target::OpportunityOrganization => ("opportunity_id", "organization_id"),
        Target::DealContact => ("deal_id", "contact_id"),
        Target::DealOrganization => ("deal_id", "organization_id"),
    }
}
fn projection(target: Target) -> String {
    let (_, reference) = columns(target);
    format!(
        "jsonb_build_object('id',id,'reference_id',{reference},'role_label',role_label,'is_primary',is_primary,'ended',ended_at IS NOT NULL,'created_at',created_at::text,'updated_at',updated_at::text)"
    )
}
fn changes_table(target: Target) -> &'static str {
    match target {
        Target::OpportunityContact => "crm.opportunity_contact_changes",
        Target::OpportunityOrganization => "crm.opportunity_organization_changes",
        Target::DealContact => "crm.deal_contact_changes",
        Target::DealOrganization => "crm.deal_organization_changes",
    }
}
pub(super) async fn get(
    db: &mut PgConnection,
    workspace: Uuid,
    target: Target,
    parent: Uuid,
    id: Uuid,
) -> Result<Row, Error> {
    let (parent_col, _) = columns(target);
    let sql = format!(
        "SELECT {} FROM {} WHERE workspace_id=$1 AND {parent_col}=$2 AND id=$3",
        projection(target),
        target.label()
    );
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
pub(super) async fn by_reference(
    db: &mut PgConnection,
    workspace: Uuid,
    target: Target,
    parent: Uuid,
    reference: Uuid,
) -> Result<Option<Uuid>, Error> {
    let (parent_col, reference_col) = columns(target);
    let sql = format!(
        "SELECT id FROM {} WHERE workspace_id=$1 AND {parent_col}=$2 AND {reference_col}=$3",
        target.label()
    );
    Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(workspace)
        .bind(parent)
        .bind(reference)
        .fetch_optional(db)
        .await?)
}
pub(super) async fn list(
    db: &mut PgConnection,
    workspace: Uuid,
    target: Target,
    parent: Uuid,
    after: Option<Uuid>,
    ended: bool,
) -> Result<Vec<Row>, Error> {
    let (parent_col, _) = columns(target);
    let table = target.label();
    let sql = format!(
        "SELECT {} FROM {table} WHERE workspace_id=$1 AND {parent_col}=$2 AND (ended_at IS NOT NULL)=$3 AND ($4::uuid IS NULL OR (created_at,id)>(SELECT created_at,id FROM {table} WHERE workspace_id=$1 AND {parent_col}=$2 AND id=$4)) ORDER BY created_at,id LIMIT 100",
        projection(target)
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
#[allow(clippy::too_many_arguments)]
pub(super) async fn save(
    db: &mut PgConnection,
    workspace: Uuid,
    target: Target,
    parent: Uuid,
    id: Uuid,
    reference: Uuid,
    label: &str,
    primary: bool,
    ended: bool,
    member: Uuid,
    existing: bool,
) -> Result<(), Error> {
    let (parent_col, reference_col) = columns(target);
    let table = target.label();
    if existing {
        let sql = format!(
            "UPDATE {table} SET role_label=$4,is_primary=$5,ended_at=CASE WHEN $6 THEN COALESCE(ended_at,now()) ELSE NULL END,updated_at=now() WHERE workspace_id=$1 AND {parent_col}=$2 AND id=$3"
        );
        sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(parent)
            .bind(id)
            .bind(label)
            .bind(primary)
            .bind(ended)
            .execute(db)
            .await?;
    } else {
        let sql = format!(
            "INSERT INTO {table}(workspace_id,{parent_col},id,{reference_col},role_label,is_primary,created_by_membership_id) VALUES($1,$2,$3,$4,$5,$6,$7)"
        );
        sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(parent)
            .bind(id)
            .bind(reference)
            .bind(label)
            .bind(primary)
            .bind(member)
            .execute(db)
            .await?;
    }
    Ok(())
}
pub(super) async fn demote(
    db: &mut PgConnection,
    workspace: Uuid,
    target: Target,
    parent: Uuid,
    except: Uuid,
) -> Result<Vec<Row>, Error> {
    let (parent_col, _) = columns(target);
    let sql = format!(
        "UPDATE {} SET is_primary=false,updated_at=now() WHERE workspace_id=$1 AND {parent_col}=$2 AND id<>$3 AND is_primary RETURNING {}",
        target.label(),
        projection(target)
    );
    Ok(
        sqlx::query_scalar::<_, Json<Row>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(parent)
            .bind(except)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|r| r.0)
            .collect(),
    )
}
pub(super) async fn touch(
    db: &mut PgConnection,
    workspace: Uuid,
    target: Target,
    id: Uuid,
) -> Result<(), Error> {
    let table = if target.is_deal() {
        "crm.deals"
    } else {
        "crm.opportunities"
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
    row: &Row,
    audit: Uuid,
) -> Result<(), Error> {
    let sql = format!(
        "INSERT INTO {}(workspace_id,association_id,role_label,is_primary,ended,audit_event_id) VALUES($1,$2,$3,$4,$5,$6)",
        changes_table(target)
    );
    sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(workspace)
        .bind(row.id)
        .bind(&row.role_label)
        .bind(row.is_primary)
        .bind(row.ended)
        .bind(audit)
        .execute(db)
        .await?;
    Ok(())
}
pub(super) async fn changes(
    db: &mut PgConnection,
    workspace: Uuid,
    target: Target,
    id: Uuid,
    query: &HistoryQuery,
    limit: i64,
) -> Result<Vec<ParticipantChange>, Error> {
    let table = changes_table(target);
    if let Some(after) = query.after {
        let sql = format!(
            "SELECT EXISTS(SELECT 1 FROM {table} WHERE workspace_id=$1 AND association_id=$2 AND id=$3)"
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
        "SELECT jsonb_build_object('id',id,'role_label',role_label,'is_primary',is_primary,'ended',ended,'occurred_at',occurred_at::text,'audit_event_id',audit_event_id) FROM {table} WHERE workspace_id=$1 AND association_id=$2 AND ($3::uuid IS NULL OR (occurred_at,id)>(SELECT occurred_at,id FROM {table} WHERE workspace_id=$1 AND association_id=$2 AND id=$3)) ORDER BY occurred_at,id LIMIT $4"
    );
    Ok(
        sqlx::query_scalar::<_, Json<ParticipantChange>>(sqlx::AssertSqlSafe(sql.as_str()))
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
