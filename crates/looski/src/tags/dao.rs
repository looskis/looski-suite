// SPDX-License-Identifier: AGPL-3.0-only
use super::{ListTags, SaveTag, Tag};
use crate::application::OperationError as Error;
use sqlx::{PgConnection, Row, postgres::PgRow};
use uuid::Uuid;
const COLUMNS: &str =
    "id,name,color,archived_at IS NOT NULL AS archived,version,created_at::text,updated_at::text";
fn decode(row: PgRow) -> Result<Tag, Error> {
    Ok(Tag {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        color: row.try_get("color")?,
        archived: row.try_get("archived")?,
        version: row.try_get("version")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}
pub(super) async fn get(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    lock: bool,
) -> Result<Tag, Error> {
    let sql = format!(
        "SELECT {COLUMNS} FROM suite.tags WHERE workspace_id=$1 AND id=$2 {}",
        if lock { "FOR UPDATE" } else { "" }
    );
    decode(
        sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?,
    )
}
pub(super) async fn list(
    db: &mut PgConnection,
    workspace: Uuid,
    query: &ListTags,
    limit: i64,
) -> Result<Vec<Tag>, Error> {
    if let Some(id) = query.after {
        get(db, workspace, id, false).await?;
    }
    let sql = format!(
        "SELECT {COLUMNS} FROM suite.tags WHERE workspace_id=$1 AND (archived_at IS NOT NULL)=$2 AND ($3::uuid IS NULL OR (created_at,id)>(SELECT created_at,id FROM suite.tags WHERE workspace_id=$1 AND id=$3)) AND ($4::text IS NULL OR strpos(lower(name),lower($4))>0) ORDER BY created_at,id LIMIT $5"
    );
    sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(workspace)
        .bind(query.archived.unwrap_or(false))
        .bind(query.after)
        .bind(&query.search)
        .bind(limit)
        .fetch_all(db)
        .await?
        .into_iter()
        .map(decode)
        .collect()
}
pub(super) async fn reserve_name(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Option<Uuid>,
    key: &str,
) -> Result<(), Error> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(format!("tag:{workspace}:{key}"))
        .execute(&mut *db)
        .await?;
    if sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM suite.tags WHERE workspace_id=$1 AND name_key=$2 AND ($3::uuid IS NULL OR id<>$3))").bind(workspace).bind(key).bind(id).fetch_one(db).await?{return Err(Error::Conflict("Tag name is already in use; restore or rename the existing tag"))}
    Ok(())
}
pub(super) async fn save(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Option<Uuid>,
    input: &SaveTag,
    key: &str,
) -> Result<Uuid, Error> {
    Ok(if let Some(id) = id {
        sqlx::query("UPDATE suite.tags SET name=$3,name_key=$4,color=$5,archived_at=CASE WHEN COALESCE($6,archived_at IS NOT NULL) THEN COALESCE(archived_at,now()) ELSE NULL END,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(workspace).bind(id).bind(input.name.trim()).bind(key).bind(&input.color).bind(input.archived).execute(db).await?;
        id
    } else {
        sqlx::query_scalar("INSERT INTO suite.tags(workspace_id,name,name_key,color,archived_at) VALUES($1,$2,$3,$4,CASE WHEN COALESCE($5,false) THEN now() ELSE NULL END) RETURNING id").bind(workspace).bind(input.name.trim()).bind(key).bind(&input.color).bind(input.archived).fetch_one(db).await?
    })
}
