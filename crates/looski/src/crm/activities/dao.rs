// SPDX-License-Identifier: AGPL-3.0-only
use super::{
    Activity, ActivityChange, ActivityHistoryQuery, ActivityInput, ActivityQuery, AnchorKind,
    types::ActivityType,
};
use crate::{access::Access, application::OperationError as Error};
use sqlx::{PgConnection, types::Json};
use uuid::Uuid;
const RECORD: &str = "SELECT jsonb_build_object('id',id,'anchor_kind',CASE WHEN contact_id IS NOT NULL THEN 'contact' WHEN opportunity_id IS NOT NULL THEN 'opportunity' ELSE 'deal' END,'anchor_id',COALESCE(contact_id,opportunity_id,deal_id),'canonical_anchor_id',COALESCE(contact_id,opportunity_id,deal_id),'anchor_name','','type_id',type_id,'type_name',type_name,'behavior',behavior,'subject',subject,'body',body,'occurred_at',occurred_at,'duration_minutes',duration_minutes,'responsible_membership_id',responsible_membership_id,'archived',archived_at IS NOT NULL,'version',version,'created_at',created_at::text,'updated_at',updated_at::text) FROM crm.activities";
pub(super) async fn get(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
) -> Result<Activity, Error> {
    let sql = format!("{RECORD} WHERE workspace_id=$1 AND id=$2");
    Ok(
        sqlx::query_scalar::<_, Json<Activity>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
pub(super) async fn candidates(
    db: &mut PgConnection,
    workspace: Uuid,
    a: &Access,
    q: &ActivityQuery,
    anchor_ids: &[Uuid],
    after: Option<Uuid>,
    limit: i64,
) -> Result<Vec<Activity>, Error> {
    // Anchor identities come from the owning module; values remain bound.
    let predicate = format!("{}=ANY($7)", q.anchor_kind.column());
    let sql = format!(
        "{RECORD} WHERE workspace_id=$1 AND ($2 OR responsible_membership_id=$3) AND (archived_at IS NOT NULL)=$4 AND ($5::uuid IS NULL OR type_id=$5) AND ($6::uuid IS NULL OR (created_at,id)>(SELECT created_at,id FROM crm.activities WHERE workspace_id=$1 AND id=$6)) AND {predicate} ORDER BY created_at,id LIMIT $8"
    );
    Ok(
        sqlx::query_scalar::<_, Json<Activity>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(a.workspace_wide)
            .bind(a.membership_id)
            .bind(q.archived.unwrap_or(false))
            .bind(q.type_id)
            .bind(after)
            .bind(anchor_ids)
            .bind(limit)
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
    id: Option<Uuid>,
    kind: AnchorKind,
    parent: Uuid,
    responsible: Uuid,
    t: &ActivityType,
    i: &ActivityInput,
) -> Result<Uuid, Error> {
    if let Some(id) = id {
        sqlx::query("UPDATE crm.activities SET type_id=$3,type_name=$4,behavior=$5,subject=$6,body=$7,occurred_at=$8::text::timestamptz,duration_minutes=$9,responsible_membership_id=$10,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(workspace).bind(id).bind(t.id).bind(&t.name).bind(t.behavior.key()).bind(i.subject.trim()).bind(&i.body).bind(&i.occurred_at).bind(i.duration_minutes).bind(responsible).execute(db).await?;
        Ok(id)
    } else {
        let sql = format!(
            "INSERT INTO crm.activities(workspace_id,{},type_id,type_name,behavior,subject,body,occurred_at,duration_minutes,responsible_membership_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8::text::timestamptz,$9,$10) RETURNING id",
            kind.column()
        );
        Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(parent)
            .bind(t.id)
            .bind(&t.name)
            .bind(t.behavior.key())
            .bind(i.subject.trim())
            .bind(&i.body)
            .bind(&i.occurred_at)
            .bind(i.duration_minutes)
            .bind(responsible)
            .fetch_one(db)
            .await?)
    }
}
pub(super) async fn archive(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    archived: bool,
) -> Result<(), Error> {
    sqlx::query("UPDATE crm.activities SET archived_at=CASE WHEN $3 THEN COALESCE(archived_at,now()) ELSE NULL END,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(workspace).bind(id).bind(archived).execute(db).await?;
    Ok(())
}
pub(super) async fn history_insert(
    db: &mut PgConnection,
    workspace: Uuid,
    r: &Activity,
    audit: Uuid,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO crm.activity_changes(workspace_id,activity_id,snapshot,audit_event_id) VALUES($1,$2,$3,$4)").bind(workspace).bind(r.id).bind(Json(r)).bind(audit).execute(db).await?;
    Ok(())
}
pub(super) async fn history(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    q: &ActivityHistoryQuery,
    limit: i64,
) -> Result<Vec<ActivityChange>, Error> {
    if let Some(after)=q.after&&!sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM crm.activity_changes WHERE workspace_id=$1 AND activity_id=$2 AND id=$3)").bind(workspace).bind(id).bind(after).fetch_one(&mut *db).await?{return Err(Error::NotFound)}
    Ok(sqlx::query_scalar::<_,Json<ActivityChange>>("SELECT jsonb_build_object('id',id,'snapshot',snapshot,'audit_event_id',audit_event_id,'occurred_at',occurred_at::text) FROM crm.activity_changes WHERE workspace_id=$1 AND activity_id=$2 AND ($3::uuid IS NULL OR (occurred_at,id)>(SELECT occurred_at,id FROM crm.activity_changes WHERE workspace_id=$1 AND activity_id=$2 AND id=$3)) ORDER BY occurred_at,id LIMIT $4").bind(workspace).bind(id).bind(q.after).bind(limit).fetch_all(db).await?.into_iter().map(|r|r.0).collect())
}

pub(super) async fn project_many(
    db: &mut PgConnection,
    ws: Uuid,
    a: &Access,
    ids: &[Uuid],
) -> Result<Vec<Activity>, Error> {
    let sql = format!(
        "{RECORD} WHERE workspace_id=$1 AND id=ANY($2) AND ($3 OR responsible_membership_id=$4)"
    );
    Ok(
        sqlx::query_scalar::<_, Json<Activity>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(ids)
            .bind(a.workspace_wide)
            .bind(a.membership_id)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|r| r.0)
            .collect(),
    )
}
