// SPDX-License-Identifier: AGPL-3.0-only
use super::{
    ContentField, ContentMedia, ContentRevision, ContentType, Draft, EntrySummary,
    ListContentTypes, ListEntries, ListMedia, PublicEntryLink, SaveContentType, SaveMedia,
    StoredEntry,
};
use crate::application::OperationError as Error;
use serde_json::{Map, Value};
use sqlx::{PgConnection, types::Json};
use uuid::Uuid;

const TYPE: &str = "jsonb_build_object('id',t.id,'name',t.name,'description',t.description,'fields',t.fields,'public_key',t.public_key,'archived',t.archived_at IS NOT NULL,
 'published_entries',(SELECT count(*) FROM cms.entries e WHERE e.workspace_id=t.workspace_id AND e.type_id=t.id AND e.published_revision_id IS NOT NULL),
 'version',t.version,'created_at',t.created_at::text,'updated_at',t.updated_at::text)";
const MEDIA: &str = "jsonb_build_object('id',m.id,'file_id',m.file_id,'title',m.title,'alt_text',m.alt_text,'media_type',m.media_type,'filename',f.filename,'byte_length',f.byte_length,'archived',m.archived_at IS NOT NULL,'file_archived',f.archived_at IS NOT NULL,'version',m.version,'created_at',m.created_at::text,'updated_at',m.updated_at::text) FROM cms.media m JOIN files.files f ON f.workspace_id=m.workspace_id AND f.id=m.file_id";
const REVISION: &str = "jsonb_build_object('id',r.id,'entry_id',r.entry_id,'number',r.number,'title',r.title,'slug',r.slug,'values',r.content,'fields',r.fields,'restored_from_number',r.restored_from_number,'created_at',r.created_at::text,'last_published_at',r.last_published_at::text)";
const ENTRY: &str = "jsonb_build_object('id',e.id,'type_id',e.type_id,'type_name',t.name,'title',e.title,'slug',e.slug,'values',e.draft_values,
 'head_number',h.number,'published',CASE WHEN p.id IS NULL THEN NULL ELSE jsonb_build_object('revision_id',p.id,'revision_number',p.number,'title',p.title,'slug',e.public_slug,'values',p.content,'published_at',e.published_at::text) END,
 'public_key',t.public_key,'type_archived',t.archived_at IS NOT NULL,'archived',e.archived_at IS NOT NULL,'version',e.version,'created_at',e.created_at::text,'updated_at',e.updated_at::text)
 FROM cms.entries e JOIN cms.content_types t ON t.workspace_id=e.workspace_id AND t.id=e.type_id
 JOIN LATERAL (SELECT number FROM cms.revisions x WHERE x.workspace_id=e.workspace_id AND x.entry_id=e.id ORDER BY number DESC LIMIT 1) h ON true
 LEFT JOIN cms.revisions p ON p.workspace_id=e.workspace_id AND p.id=e.published_revision_id";

pub(super) async fn content_type(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    lock: bool,
) -> Result<ContentType, Error> {
    let sql = format!(
        "SELECT {TYPE} FROM cms.content_types t WHERE t.workspace_id=$1 AND t.id=$2 {}",
        if lock { "FOR UPDATE OF t" } else { "" }
    );
    Ok(
        sqlx::query_scalar::<_, Json<ContentType>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
pub(super) async fn content_types(
    db: &mut PgConnection,
    ws: Uuid,
    q: &ListContentTypes,
    limit: i64,
) -> Result<Vec<ContentType>, Error> {
    let sql = format!(
        "SELECT {TYPE} FROM cms.content_types t WHERE t.workspace_id=$1 AND (t.archived_at IS NOT NULL)=$2 AND ($3::uuid IS NULL OR (t.created_at,t.id)>(SELECT created_at,id FROM cms.content_types WHERE workspace_id=$1 AND id=$3)) AND ($4::text IS NULL OR strpos(lower(t.name),lower($4))>0) ORDER BY t.created_at,t.id LIMIT $5"
    );
    Ok(
        sqlx::query_scalar::<_, Json<ContentType>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
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
pub(super) async fn insert_type(
    db: &mut PgConnection,
    ws: Uuid,
    input: &SaveContentType,
    public_key: &str,
) -> Result<Uuid, Error> {
    Ok(sqlx::query_scalar("INSERT INTO cms.content_types(workspace_id,name,description,fields,public_key) VALUES($1,$2,$3,$4,$5) RETURNING id")
        .bind(ws).bind(input.name.trim()).bind(&input.description).bind(Json(&input.fields)).bind(public_key)
        .fetch_one(db).await?)
}
pub(super) async fn update_type(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    input: &SaveContentType,
) -> Result<(), Error> {
    sqlx::query("UPDATE cms.content_types SET name=$3,description=$4,fields=$5,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(id).bind(input.name.trim()).bind(&input.description).bind(Json(&input.fields))
        .execute(db).await?;
    Ok(())
}
pub(super) async fn archive_type(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    archived: bool,
) -> Result<(), Error> {
    sqlx::query("UPDATE cms.content_types SET archived_at=CASE WHEN $3 THEN COALESCE(archived_at,now()) END,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(id).bind(archived).execute(db).await?;
    Ok(())
}
pub(super) async fn entry(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    lock: bool,
) -> Result<StoredEntry, Error> {
    let sql = format!(
        "SELECT {ENTRY} WHERE e.workspace_id=$1 AND e.id=$2 {}",
        if lock { "FOR UPDATE OF e" } else { "" }
    );
    Ok(
        sqlx::query_scalar::<_, Json<StoredEntry>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
pub(super) async fn entries(
    db: &mut PgConnection,
    ws: Uuid,
    q: &ListEntries,
    limit: i64,
) -> Result<Vec<StoredEntry>, Error> {
    let sql = format!(
        "SELECT {ENTRY} WHERE e.workspace_id=$1 AND (e.archived_at IS NOT NULL)=$2 AND ($3::uuid IS NULL OR (e.created_at,e.id)>(SELECT created_at,id FROM cms.entries WHERE workspace_id=$1 AND id=$3)) AND ($4::uuid IS NULL OR e.type_id=$4) ORDER BY e.created_at,e.id LIMIT $5"
    );
    Ok(
        sqlx::query_scalar::<_, Json<StoredEntry>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(q.archived.unwrap_or(false))
            .bind(q.after)
            .bind(q.type_id)
            .bind(limit)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|r| r.0)
            .collect(),
    )
}
/// Summaries reveal only published words to members who cannot read drafts.
pub(super) async fn summaries(
    db: &mut PgConnection,
    ws: Uuid,
    q: &ListEntries,
    drafts: bool,
    limit: i64,
) -> Result<Vec<EntrySummary>, Error> {
    let sql = "SELECT jsonb_build_object('id',e.id,'type_id',e.type_id,'type_name',t.name,
      'title',CASE WHEN $6 THEN e.title ELSE p.title END,'slug',CASE WHEN $6 THEN e.slug ELSE e.public_slug END,
      'published_revision_number',p.number,'published_slug',e.public_slug,
      'unpublished_changes',CASE WHEN $6 THEN p.id IS NULL OR p.number<>(SELECT max(number) FROM cms.revisions x WHERE x.workspace_id=e.workspace_id AND x.entry_id=e.id) ELSE false END,
      'public_key',t.public_key,'archived',e.archived_at IS NOT NULL,'version',e.version,'updated_at',e.updated_at::text)
     FROM cms.entries e JOIN cms.content_types t ON t.workspace_id=e.workspace_id AND t.id=e.type_id
     LEFT JOIN cms.revisions p ON p.workspace_id=e.workspace_id AND p.id=e.published_revision_id
     WHERE e.workspace_id=$1 AND (e.archived_at IS NOT NULL)=$2 AND ($6 OR p.id IS NOT NULL)
      AND ($3::uuid IS NULL OR (e.created_at,e.id)>(SELECT created_at,id FROM cms.entries WHERE workspace_id=$1 AND id=$3))
      AND ($4::uuid IS NULL OR e.type_id=$4)
      AND ($7::text IS NULL OR strpos(lower(CASE WHEN $6 THEN e.title ELSE p.title END),lower($7))>0)
     ORDER BY e.created_at,e.id LIMIT $5";
    Ok(sqlx::query_scalar::<_, Json<EntrySummary>>(sql)
        .bind(ws)
        .bind(q.archived.unwrap_or(false))
        .bind(q.after)
        .bind(q.type_id)
        .bind(limit)
        .bind(drafts)
        .bind(&q.search)
        .fetch_all(db)
        .await?
        .into_iter()
        .map(|r| r.0)
        .collect())
}
pub(super) async fn insert_entry(
    db: &mut PgConnection,
    ws: Uuid,
    type_id: Uuid,
    draft: &Draft,
) -> Result<Uuid, Error> {
    Ok(sqlx::query_scalar("INSERT INTO cms.entries(workspace_id,type_id,title,slug,draft_values) VALUES($1,$2,$3,$4,$5) RETURNING id")
        .bind(ws).bind(type_id).bind(draft.title.trim()).bind(&draft.slug).bind(Json(&draft.values))
        .fetch_one(db).await?)
}
pub(super) async fn update_draft(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    draft: &Draft,
) -> Result<(), Error> {
    sqlx::query("UPDATE cms.entries SET title=$3,slug=$4,draft_values=$5,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(id).bind(draft.title.trim()).bind(&draft.slug).bind(Json(&draft.values))
        .execute(db).await?;
    Ok(())
}
/// Append the next immutable revision and record which media it names.
#[allow(clippy::too_many_arguments)]
pub(super) async fn insert_revision(
    db: &mut PgConnection,
    ws: Uuid,
    entry: Uuid,
    draft: &Draft,
    fields: &[ContentField],
    media: &[Uuid],
    restored_from: Option<i32>,
    author: Uuid,
) -> Result<(Uuid, i32), Error> {
    let (id, number): (Uuid, i32) = sqlx::query_as("INSERT INTO cms.revisions(workspace_id,entry_id,number,title,slug,content,fields,restored_from_number,created_by_membership_id)
      SELECT $1,$2,COALESCE(max(number),0)+1,$3,$4,$5,$6,$7,$8 FROM cms.revisions WHERE workspace_id=$1 AND entry_id=$2 RETURNING id,number")
        .bind(ws).bind(entry).bind(draft.title.trim()).bind(&draft.slug).bind(Json(&draft.values)).bind(Json(fields)).bind(restored_from).bind(author)
        .fetch_one(&mut *db).await?;
    for media_id in media {
        sqlx::query(
            "INSERT INTO cms.revision_media(workspace_id,revision_id,media_id) VALUES($1,$2,$3)",
        )
        .bind(ws)
        .bind(id)
        .bind(media_id)
        .execute(&mut *db)
        .await?;
    }
    Ok((id, number))
}
pub(super) async fn revision(
    db: &mut PgConnection,
    ws: Uuid,
    entry: Uuid,
    id: Uuid,
) -> Result<ContentRevision, Error> {
    let sql = format!(
        "SELECT {REVISION} FROM cms.revisions r WHERE r.workspace_id=$1 AND r.entry_id=$2 AND r.id=$3"
    );
    Ok(
        sqlx::query_scalar::<_, Json<ContentRevision>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(entry)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
pub(super) async fn head_revision(
    db: &mut PgConnection,
    ws: Uuid,
    entry: Uuid,
) -> Result<ContentRevision, Error> {
    let sql = format!(
        "SELECT {REVISION} FROM cms.revisions r WHERE r.workspace_id=$1 AND r.entry_id=$2 ORDER BY r.number DESC LIMIT 1"
    );
    Ok(
        sqlx::query_scalar::<_, Json<ContentRevision>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(entry)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
/// Newest first, the order people read history in.
pub(super) async fn revisions(
    db: &mut PgConnection,
    ws: Uuid,
    entry: Uuid,
    after: Option<Uuid>,
    limit: i64,
) -> Result<Vec<ContentRevision>, Error> {
    let sql = format!(
        "SELECT {REVISION} FROM cms.revisions r WHERE r.workspace_id=$1 AND r.entry_id=$2 AND ($3::uuid IS NULL OR r.number<(SELECT number FROM cms.revisions WHERE workspace_id=$1 AND entry_id=$2 AND id=$3)) ORDER BY r.number DESC LIMIT $4"
    );
    Ok(
        sqlx::query_scalar::<_, Json<ContentRevision>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(entry)
            .bind(after)
            .bind(limit)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|r| r.0)
            .collect(),
    )
}
pub(super) async fn address_taken(
    db: &mut PgConnection,
    ws: Uuid,
    type_id: Uuid,
    slug: &str,
    entry: Uuid,
) -> Result<bool, Error> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM cms.entries WHERE workspace_id=$1 AND type_id=$2 AND public_slug=$3 AND id<>$4)")
        .bind(ws).bind(type_id).bind(slug).bind(entry).fetch_one(db).await?)
}
pub(super) async fn publish(
    db: &mut PgConnection,
    ws: Uuid,
    entry: Uuid,
    revision: Uuid,
    slug: &str,
) -> Result<(), Error> {
    sqlx::query("UPDATE cms.entries SET published_revision_id=$3,public_slug=$4,published_at=now(),version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(entry).bind(revision).bind(slug).execute(&mut *db).await?;
    sqlx::query("UPDATE cms.revisions SET last_published_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws)
        .bind(revision)
        .execute(db)
        .await?;
    Ok(())
}
pub(super) async fn unpublish(db: &mut PgConnection, ws: Uuid, entry: Uuid) -> Result<(), Error> {
    sqlx::query("UPDATE cms.entries SET published_revision_id=NULL,public_slug=NULL,published_at=NULL,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(entry).execute(db).await?;
    Ok(())
}
/// Archiving always withdraws: the row constraint forbids an archived live entry.
pub(super) async fn archive_entry(
    db: &mut PgConnection,
    ws: Uuid,
    entry: Uuid,
    archived: bool,
) -> Result<(), Error> {
    sqlx::query("UPDATE cms.entries SET archived_at=CASE WHEN $3 THEN COALESCE(archived_at,now()) END,published_revision_id=CASE WHEN $3 THEN NULL ELSE published_revision_id END,public_slug=CASE WHEN $3 THEN NULL ELSE public_slug END,published_at=CASE WHEN $3 THEN NULL ELSE published_at END,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(entry).bind(archived).execute(db).await?;
    Ok(())
}
pub(super) async fn media(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    lock: bool,
) -> Result<ContentMedia, Error> {
    let sql = format!(
        "SELECT {MEDIA} WHERE m.workspace_id=$1 AND m.id=$2 {}",
        if lock { "FOR UPDATE OF m" } else { "" }
    );
    Ok(
        sqlx::query_scalar::<_, Json<ContentMedia>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
pub(super) async fn media_page(
    db: &mut PgConnection,
    ws: Uuid,
    q: &ListMedia,
    limit: i64,
) -> Result<Vec<ContentMedia>, Error> {
    let sql = format!(
        "SELECT {MEDIA} WHERE m.workspace_id=$1 AND (m.archived_at IS NOT NULL)=$2 AND ($3::uuid IS NULL OR (m.created_at,m.id)>(SELECT created_at,id FROM cms.media WHERE workspace_id=$1 AND id=$3)) AND ($4::text IS NULL OR strpos(lower(m.title),lower($4))>0) ORDER BY m.created_at,m.id LIMIT $5"
    );
    Ok(
        sqlx::query_scalar::<_, Json<ContentMedia>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
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
pub(super) async fn media_for_file(
    db: &mut PgConnection,
    ws: Uuid,
    file: Uuid,
) -> Result<bool, Error> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM cms.media WHERE workspace_id=$1 AND file_id=$2)",
    )
    .bind(ws)
    .bind(file)
    .fetch_one(db)
    .await?)
}
pub(super) async fn insert_media(
    db: &mut PgConnection,
    ws: Uuid,
    file: Uuid,
    title: &str,
    alt_text: &str,
    media_type: &str,
) -> Result<Uuid, Error> {
    Ok(sqlx::query_scalar("INSERT INTO cms.media(workspace_id,file_id,title,alt_text,media_type) VALUES($1,$2,$3,$4,$5) RETURNING id")
        .bind(ws).bind(file).bind(title.trim()).bind(alt_text).bind(media_type).fetch_one(db).await?)
}
pub(super) async fn update_media(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    input: &SaveMedia,
) -> Result<(), Error> {
    sqlx::query("UPDATE cms.media SET title=$3,alt_text=$4,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(id).bind(input.title.trim()).bind(&input.alt_text).execute(db).await?;
    Ok(())
}
pub(super) async fn archive_media(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    archived: bool,
) -> Result<(), Error> {
    sqlx::query("UPDATE cms.media SET archived_at=CASE WHEN $3 THEN COALESCE(archived_at,now()) END,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(id).bind(archived).execute(db).await?;
    Ok(())
}
/// Which of these ids are media in this workspace; used to reject foreign references.
pub(super) async fn known_media(
    db: &mut PgConnection,
    ws: Uuid,
    ids: &[Uuid],
    active_only: bool,
) -> Result<i64, Error> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM cms.media m JOIN files.files f ON f.workspace_id=m.workspace_id AND f.id=m.file_id WHERE m.workspace_id=$1 AND m.id=ANY($2) AND (NOT $3 OR (m.archived_at IS NULL AND f.archived_at IS NULL))")
        .bind(ws).bind(ids).bind(active_only).fetch_one(db).await?)
}
pub(super) struct PublicType {
    pub workspace: Uuid,
    pub id: Uuid,
}
pub(super) async fn public_type(db: &mut PgConnection, key: &str) -> Result<PublicType, Error> {
    let (workspace, id) = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT workspace_id,id FROM cms.content_types WHERE public_key=$1 AND archived_at IS NULL",
    )
    .bind(key)
    .fetch_optional(db)
    .await?
    .ok_or(Error::NotFound)?;
    Ok(PublicType { workspace, id })
}
pub(super) async fn public_name(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
) -> Result<(String, String), Error> {
    Ok(sqlx::query_as(
        "SELECT name,description FROM cms.content_types WHERE workspace_id=$1 AND id=$2",
    )
    .bind(ws)
    .bind(id)
    .fetch_one(db)
    .await?)
}
/// Published entries in address order; the address is the cursor.
pub(super) async fn public_entries(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    after: Option<&str>,
    limit: i64,
) -> Result<Vec<PublicEntryLink>, Error> {
    Ok(sqlx::query_scalar::<_, Json<PublicEntryLink>>("SELECT jsonb_build_object('title',r.title,'slug',e.public_slug,'published_at',e.published_at::text) FROM cms.entries e JOIN cms.revisions r ON r.workspace_id=e.workspace_id AND r.id=e.published_revision_id WHERE e.workspace_id=$1 AND e.type_id=$2 AND ($3::text IS NULL OR e.public_slug>$3) ORDER BY e.public_slug LIMIT $4")
        .bind(ws).bind(id).bind(after).bind(limit).fetch_all(db).await?.into_iter().map(|r| r.0).collect())
}
pub(super) struct PublishedRow {
    pub title: String,
    pub slug: String,
    pub published_at: String,
    pub fields: Vec<ContentField>,
    pub values: Map<String, Value>,
}
pub(super) async fn public_entry(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    slug: &str,
) -> Result<PublishedRow, Error> {
    let row = sqlx::query_as::<_, (String, String, String, Json<Vec<ContentField>>, Json<Map<String, Value>>)>(
        "SELECT r.title,e.public_slug,e.published_at::text,r.fields,r.content FROM cms.entries e JOIN cms.revisions r ON r.workspace_id=e.workspace_id AND r.id=e.published_revision_id WHERE e.workspace_id=$1 AND e.type_id=$2 AND e.public_slug=$3",
    )
    .bind(ws).bind(id).bind(slug).fetch_optional(db).await?.ok_or(Error::NotFound)?;
    Ok(PublishedRow {
        title: row.0,
        slug: row.1,
        published_at: row.2,
        fields: row.3.0,
        values: row.4.0,
    })
}
/// What a public reader may see of a media item, while it and its file are active.
pub(super) async fn visible_media(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
) -> Result<Option<(String, String, String)>, Error> {
    Ok(sqlx::query_as("SELECT m.title,m.alt_text,m.media_type FROM cms.media m JOIN files.files f ON f.workspace_id=m.workspace_id AND f.id=m.file_id WHERE m.workspace_id=$1 AND m.id=$2 AND m.archived_at IS NULL AND f.archived_at IS NULL")
        .bind(ws).bind(id).fetch_optional(db).await?)
}
/// The only way from a public key to bytes: active media named by a revision that
/// is live right now in this content type. Drafts and old revisions grant nothing.
pub(super) async fn published_media(
    db: &mut PgConnection,
    ws: Uuid,
    type_id: Uuid,
    media: Uuid,
) -> Result<(Uuid, String), Error> {
    sqlx::query_as("SELECT m.file_id,m.media_type FROM cms.media m
      WHERE m.workspace_id=$1 AND m.id=$3 AND m.archived_at IS NULL AND EXISTS(
       SELECT 1 FROM cms.entries e JOIN cms.revision_media rm ON rm.workspace_id=e.workspace_id AND rm.revision_id=e.published_revision_id
       WHERE e.workspace_id=$1 AND e.type_id=$2 AND rm.media_id=m.id)")
        .bind(ws).bind(type_id).bind(media).fetch_optional(db).await?.ok_or(Error::NotFound)
}
