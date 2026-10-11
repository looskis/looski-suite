// SPDX-License-Identifier: AGPL-3.0-only
use super::{FileInput, FileQuery, FileRecord};
use crate::{access::Access, application::OperationError as Error};
use sqlx::{PgConnection, types::Json};
use uuid::Uuid;
const RECORD: &str = "jsonb_build_object('id',f.id,'filename',f.filename,'media_type',f.media_type,'notes',f.notes,'owner_membership_id',f.owner_membership_id,'byte_length',f.byte_length,'sha256',f.sha256,'archived',f.archived_at IS NOT NULL,'version',f.version,'created_at',f.created_at::text,'updated_at',f.updated_at::text)";
pub(super) async fn get(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    access: &Access,
    lock: bool,
) -> Result<FileRecord, Error> {
    let sql = format!(
        "SELECT {RECORD} FROM files.files f WHERE f.workspace_id=$1 AND f.id=$2 AND ($3 OR f.owner_membership_id=$4) {}",
        if lock { "FOR UPDATE OF f" } else { "" }
    );
    Ok(
        sqlx::query_scalar::<_, Json<FileRecord>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(id)
            .bind(access.workspace_wide)
            .bind(access.membership_id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
pub(super) async fn list(
    db: &mut PgConnection,
    workspace: Uuid,
    access: &Access,
    q: &FileQuery,
    limit: i64,
) -> Result<Vec<FileRecord>, Error> {
    if let Some(after) = q.after {
        get(db, workspace, after, access, false).await?;
    }
    let sql = format!(
        "SELECT {RECORD} FROM files.files f WHERE f.workspace_id=$1 AND ($2 OR f.owner_membership_id=$3) AND (f.archived_at IS NOT NULL)=$4 AND ($5::uuid IS NULL OR (f.created_at,f.id)>(SELECT created_at,id FROM files.files WHERE workspace_id=$1 AND id=$5)) AND ($6::text IS NULL OR strpos(lower(f.filename),lower($6))>0) ORDER BY f.created_at,f.id LIMIT $7"
    );
    Ok(
        sqlx::query_scalar::<_, Json<FileRecord>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(access.workspace_wide)
            .bind(access.membership_id)
            .bind(q.archived.unwrap_or(false))
            .bind(q.after)
            .bind(&q.search)
            .bind(limit)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|r| r.0)
            .collect(),
    )
}
pub(super) async fn active_digest(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
) -> Result<(i64, String), Error> {
    sqlx::query_as(
        "SELECT byte_length,sha256 FROM files.files WHERE workspace_id=$1 AND id=$2 AND archived_at IS NULL",
    )
    .bind(workspace)
    .bind(id)
    .fetch_optional(db)
    .await?
    .ok_or(Error::NotFound)
}
pub(super) async fn content(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
) -> Result<Vec<u8>, Error> {
    sqlx::query_scalar("SELECT content FROM files.contents WHERE workspace_id=$1 AND file_id=$2")
        .bind(workspace)
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or(Error::Unavailable)
}
pub(super) async fn insert(
    db: &mut PgConnection,
    workspace: Uuid,
    owner: Uuid,
    input: &FileInput,
    digest: &str,
    bytes: &[u8],
) -> Result<Uuid, Error> {
    let id=sqlx::query_scalar("INSERT INTO files.files(workspace_id,owner_membership_id,filename,media_type,notes,sha256,byte_length) VALUES($1,$2,$3,$4,$5,$6,$7) RETURNING id").bind(workspace).bind(owner).bind(&input.filename).bind(&input.media_type).bind(&input.notes).bind(digest).bind(bytes.len() as i64).fetch_one(&mut *db).await?;
    sqlx::query("INSERT INTO files.contents(workspace_id,file_id,sha256,byte_length,content) VALUES($1,$2,$3,$4,$5)").bind(workspace).bind(id).bind(digest).bind(bytes.len() as i64).bind(bytes).execute(db).await?;
    Ok(id)
}
pub(super) async fn update(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    owner: Uuid,
    input: &FileInput,
) -> Result<(), Error> {
    sqlx::query("UPDATE files.files SET filename=$3,media_type=$4,notes=$5,owner_membership_id=$6,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(workspace).bind(id).bind(&input.filename).bind(&input.media_type).bind(&input.notes).bind(owner).execute(db).await?;
    Ok(())
}
pub(super) async fn archive(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    archived: bool,
) -> Result<(), Error> {
    sqlx::query("UPDATE files.files SET archived_at=CASE WHEN $3 THEN coalesce(archived_at,now()) ELSE NULL END,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(workspace).bind(id).bind(archived).execute(db).await?;
    Ok(())
}
