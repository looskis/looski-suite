// SPDX-License-Identifier: AGPL-3.0-only
use super::{ActivityType, ActivityTypeQuery, SaveActivityType};
use crate::application::OperationError as Error;
use sqlx::{PgConnection, types::Json};
use uuid::Uuid;
const RECORD: &str = "SELECT jsonb_build_object('id',id,'name',name,'behavior',behavior,'position',position,'archived',archived_at IS NOT NULL,'version',version) FROM crm.activity_types";
pub(super) async fn get(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
) -> Result<ActivityType, Error> {
    let sql = format!("{RECORD} WHERE workspace_id=$1 AND id=$2");
    Ok(
        sqlx::query_scalar::<_, Json<ActivityType>>(sqlx::AssertSqlSafe(sql.as_str()))
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
    q: &ActivityTypeQuery,
    limit: i64,
) -> Result<Vec<ActivityType>, Error> {
    if let Some(id) = q.after {
        get(db, workspace, id).await?;
    }
    let sql = format!(
        "{RECORD} WHERE workspace_id=$1 AND (archived_at IS NOT NULL)=$2 AND ($3::uuid IS NULL OR (position,created_at,id)>(SELECT position,created_at,id FROM crm.activity_types WHERE workspace_id=$1 AND id=$3)) ORDER BY position,created_at,id LIMIT $4"
    );
    Ok(
        sqlx::query_scalar::<_, Json<ActivityType>>(sqlx::AssertSqlSafe(sql.as_str()))
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
pub(super) async fn save(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Option<Uuid>,
    i: &SaveActivityType,
) -> Result<Uuid, Error> {
    if let Some(id) = id {
        sqlx::query("UPDATE crm.activity_types SET name=$3,position=COALESCE($4,position),archived_at=CASE WHEN $5 IS NULL THEN archived_at WHEN $5 THEN COALESCE(archived_at,now()) ELSE NULL END,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(workspace).bind(id).bind(i.name.trim()).bind(i.position).bind(i.archived).execute(db).await?;
        Ok(id)
    } else {
        Ok(sqlx::query_scalar("INSERT INTO crm.activity_types(workspace_id,name,behavior,position,archived_at) VALUES($1,$2,$3,$4,CASE WHEN $5 THEN now() ELSE NULL END) RETURNING id").bind(workspace).bind(i.name.trim()).bind(i.behavior.key()).bind(i.position.unwrap_or(0)).bind(i.archived.unwrap_or(false)).fetch_one(db).await?)
    }
}
