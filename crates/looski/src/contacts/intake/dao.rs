// SPDX-License-Identifier: AGPL-3.0-only
use super::{Intake, ListIntakes, SubmitIntake};
use crate::{access::Access, application::OperationError as Error, contacts::Identifier};
use sqlx::{PgConnection, Row, postgres::PgRow, types::Json};
use uuid::Uuid;
const COLUMNS: &str = "id,owner_membership_id,source_ref,input,state,contact_id,resolution_reason,version,created_at::text,updated_at::text";
fn decode(row: PgRow) -> Result<Intake, Error> {
    Ok(Intake {
        id: row.try_get("id")?,
        owner_membership_id: row.try_get("owner_membership_id")?,
        source_ref: row.try_get("source_ref")?,
        input: row.try_get::<Json<SubmitIntake>, _>("input")?.0,
        state: row.try_get("state")?,
        contact_id: row.try_get("contact_id")?,
        resolution_reason: row.try_get("resolution_reason")?,
        version: row.try_get("version")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}
pub(super) async fn get(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    access: &Access,
    lock: bool,
) -> Result<Intake, Error> {
    let sql = format!(
        "SELECT {COLUMNS} FROM contacts.intakes WHERE workspace_id=$1 AND id=$2 AND ($3 OR owner_membership_id=$4) {}",
        if lock { "FOR UPDATE" } else { "" }
    );
    decode(
        sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(id)
            .bind(access.workspace_wide)
            .bind(access.membership_id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?,
    )
}
pub(super) async fn list(
    db: &mut PgConnection,
    workspace: Uuid,
    access: &Access,
    query: &ListIntakes,
    state: &str,
    limit: i64,
) -> Result<Vec<Intake>, Error> {
    if let Some(id) = query.after {
        get(db, workspace, id, access, false).await?;
    }
    let sql = format!(
        "SELECT {COLUMNS} FROM contacts.intakes WHERE workspace_id=$1 AND ($2 OR owner_membership_id=$3) AND state=$4 AND ($5::uuid IS NULL OR (created_at,id)>(SELECT created_at,id FROM contacts.intakes WHERE workspace_id=$1 AND id=$5)) ORDER BY created_at,id LIMIT $6"
    );
    sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(workspace)
        .bind(access.workspace_wide)
        .bind(access.membership_id)
        .bind(state)
        .bind(query.after)
        .bind(limit)
        .fetch_all(db)
        .await?
        .into_iter()
        .map(decode)
        .collect()
}
pub(super) async fn insert(
    db: &mut PgConnection,
    workspace: Uuid,
    owner: Uuid,
    input: &SubmitIntake,
    contact: Option<Uuid>,
) -> Result<Uuid, Error> {
    Ok(sqlx::query_scalar("INSERT INTO contacts.intakes(workspace_id,owner_membership_id,source_ref,input,state,contact_id) VALUES($1,$2,$3,$4,CASE WHEN $5::uuid IS NULL THEN 'pending' ELSE 'resolved' END,$5) RETURNING id").bind(workspace).bind(owner).bind(input.source_ref.trim()).bind(Json(input)).bind(contact).fetch_one(db).await?)
}
pub(super) async fn resolve(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    contact: Option<Uuid>,
    reason: &str,
) -> Result<(), Error> {
    sqlx::query("UPDATE contacts.intakes SET state=CASE WHEN $3::uuid IS NULL THEN 'dismissed' ELSE 'resolved' END,contact_id=$3,resolution_reason=$4,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(workspace).bind(id).bind(contact).bind(reason.trim()).execute(db).await?;
    Ok(())
}
pub(super) async fn has_case_suggestions(
    db: &mut PgConnection,
    workspace: Uuid,
    ids: &[Identifier],
) -> Result<bool, Error> {
    for identifier in ids.iter().filter(|i| i.kind == "email") {
        if sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM contacts.identifiers i JOIN contacts.contact_identifiers ci ON (ci.workspace_id,ci.identifier_id)=(i.workspace_id,i.id) WHERE i.workspace_id=$1 AND i.kind='email' AND lower(i.normalized_value)=lower($2) AND i.normalized_value<>$2 AND ci.ended_at IS NULL AND NOT EXISTS(SELECT 1 FROM contacts.identifiers e JOIN contacts.contact_identifiers ec ON (ec.workspace_id,ec.identifier_id)=(e.workspace_id,e.id) WHERE e.workspace_id=$1 AND e.kind='email' AND e.normalized_value=$2 AND ec.ended_at IS NULL))").bind(workspace).bind(&identifier.normalized).fetch_one(&mut *db).await?{return Ok(true)}
    }
    Ok(false)
}
pub(super) async fn candidate_ids(
    db: &mut PgConnection,
    workspace: Uuid,
    ids: &[Identifier],
    access: &Access,
) -> Result<Vec<Uuid>, Error> {
    let mut result = Vec::new();
    for identifier in ids {
        let candidates=sqlx::query_scalar::<_,Uuid>("SELECT DISTINCT c.id FROM contacts.identifiers i JOIN contacts.contact_identifiers ci ON (ci.workspace_id,ci.identifier_id)=(i.workspace_id,i.id) JOIN contacts.contacts c ON (c.workspace_id,c.id)=(ci.workspace_id,ci.contact_id) WHERE i.workspace_id=$1 AND i.kind=$2 AND (i.normalized_value=$3 OR (i.kind='email' AND lower(i.normalized_value)=lower($3))) AND ci.ended_at IS NULL AND ($4 OR c.owner_membership_id=$5) ORDER BY c.id LIMIT 50").bind(workspace).bind(&identifier.kind).bind(&identifier.normalized).bind(access.workspace_wide).bind(access.membership_id).fetch_all(&mut *db).await?;
        result.extend(candidates);
    }
    result.sort_unstable();
    result.dedup();
    result.truncate(50);
    Ok(result)
}
