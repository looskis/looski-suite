// SPDX-License-Identifier: AGPL-3.0-only
//! Every read names `knowledge.reaches` in the statement that selects its rows.
use super::{
    KnowledgeActivity, KnowledgeCollection, KnowledgeDocument, KnowledgeDocumentSummary,
    KnowledgeLinkPreview, KnowledgeMember, KnowledgeRevision, KnowledgeSearchHit,
    KnowledgeSearchQuery, KnowledgeSuggestion, ListKnowledgeCollections, ListKnowledgeDocuments,
    Reach, SaveKnowledgeCollection,
};
use crate::application::OperationError as Error;
use sqlx::{PgConnection, types::Json};
use uuid::Uuid;

const COLLECTION: &str = "jsonb_build_object('id',c.id,'name',c.name,'description',c.description,'restricted',c.restricted,'archived',c.archived_at IS NOT NULL,
 'is_member',EXISTS(SELECT 1 FROM knowledge.collection_members m WHERE m.workspace_id=c.workspace_id AND m.collection_id=c.id AND m.membership_id=$5),
 'document_count',(SELECT count(*) FROM knowledge.documents d WHERE d.workspace_id=c.workspace_id AND d.collection_id=c.id AND d.archived_at IS NULL),
 'version',c.version,'created_at',c.created_at::text,'updated_at',c.updated_at::text)";
const DOCUMENT: &str = "jsonb_build_object('id',d.id,'collection_id',d.collection_id,'collection_name',c.name,'collection_restricted',c.restricted,'collection_archived',c.archived_at IS NOT NULL,
 'kind',d.kind,'title',d.title,'body',d.body,'revision_number',d.head_number,'archived',d.archived_at IS NOT NULL,
 'updated_by_name',(SELECT u.display_name FROM knowledge.revisions r JOIN suite.memberships m ON (m.workspace_id,m.id)=(r.workspace_id,r.created_by_membership_id) JOIN suite.users u ON u.id=m.user_id WHERE r.workspace_id=d.workspace_id AND r.document_id=d.id AND r.number=d.head_number),
 'version',d.version,'created_at',d.created_at::text,'updated_at',d.updated_at::text)
 FROM knowledge.documents d JOIN knowledge.collections c ON c.workspace_id=d.workspace_id AND c.id=d.collection_id";
const SUMMARY: &str = "jsonb_build_object('id',d.id,'collection_id',d.collection_id,'collection_name',c.name,'kind',d.kind,'title',d.title,'revision_number',d.head_number,'archived',d.archived_at IS NOT NULL,'version',d.version,'updated_at',d.updated_at::text)
 FROM knowledge.documents d JOIN knowledge.collections c ON c.workspace_id=d.workspace_id AND c.id=d.collection_id";
const REVISION: &str = "jsonb_build_object('id',r.id,'document_id',r.document_id,'number',r.number,'change',r.change,'collection_id',r.collection_id,'collection_name',o.name,'kind',r.kind,'title',r.title,'body',r.body,'restored_from_number',r.restored_from_number,'created_by_name',u.display_name,'created_at',r.created_at::text)
 FROM knowledge.revisions r JOIN knowledge.collections o ON o.workspace_id=r.workspace_id AND o.id=r.collection_id
 JOIN knowledge.documents d ON d.workspace_id=r.workspace_id AND d.id=r.document_id
 JOIN suite.memberships m ON (m.workspace_id,m.id)=(r.workspace_id,r.created_by_membership_id) JOIN suite.users u ON u.id=m.user_id";
/// $1 workspace, $3–$5 reach: a revision needs the document's current collection and
/// the collection it was written in.
const REVISION_VISIBLE: &str = "knowledge.reaches(r.workspace_id,d.collection_id,$3,$4,$5) AND knowledge.reaches(r.workspace_id,r.collection_id,$3,$4,$5)";
const ACTIVITY: &str = "FROM knowledge.revisions r JOIN knowledge.documents d ON d.workspace_id=r.workspace_id AND d.id=r.document_id
 JOIN knowledge.collections k ON k.workspace_id=d.workspace_id AND k.id=d.collection_id
 JOIN knowledge.collections o ON o.workspace_id=r.workspace_id AND o.id=r.collection_id
 JOIN suite.memberships m ON (m.workspace_id,m.id)=(r.workspace_id,r.created_by_membership_id) JOIN suite.users u ON u.id=m.user_id
 WHERE r.workspace_id=$1 AND d.archived_at IS NULL AND k.archived_at IS NULL";
/// Search covers current text of active documents in active collections the reader reaches.
const MATCHES: &str = "FROM knowledge.documents d JOIN knowledge.collections c ON c.workspace_id=d.workspace_id AND c.id=d.collection_id
 CROSS JOIN websearch_to_tsquery('english',$2) q
 WHERE d.workspace_id=$1 AND d.search_vector @@ q AND d.archived_at IS NULL AND c.archived_at IS NULL
 AND knowledge.reaches(d.workspace_id,d.collection_id,$3,$4,$5)
 AND ($6::uuid IS NULL OR d.collection_id=$6) AND ($7::text IS NULL OR d.kind=$7)";

pub(super) struct NewRevision<'a> {
    pub document: Uuid,
    pub number: i32,
    pub collection: Uuid,
    pub change: &'a str,
    pub kind: &'a str,
    pub title: &'a str,
    pub body: &'a str,
    pub restored_from: Option<i32>,
    pub author: Uuid,
}

pub(super) async fn reaches(
    db: &mut PgConnection,
    ws: Uuid,
    collection: Uuid,
    r: Reach,
) -> Result<bool, Error> {
    Ok(
        sqlx::query_scalar("SELECT knowledge.reaches($1,$2,$3,$4,$5)")
            .bind(ws)
            .bind(collection)
            .bind(r.all)
            .bind(r.wide)
            .bind(r.membership)
            .fetch_one(db)
            .await?,
    )
}
pub(super) async fn collection(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    r: Reach,
) -> Result<KnowledgeCollection, Error> {
    let sql = format!(
        "SELECT {COLLECTION} FROM knowledge.collections c WHERE c.workspace_id=$1 AND c.id=$2 AND knowledge.reaches(c.workspace_id,c.id,$3,$4,$5)"
    );
    Ok(
        sqlx::query_scalar::<_, Json<KnowledgeCollection>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(id)
            .bind(r.all)
            .bind(r.wide)
            .bind(r.membership)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
pub(super) async fn collections(
    db: &mut PgConnection,
    ws: Uuid,
    q: &ListKnowledgeCollections,
    r: Reach,
    limit: i64,
) -> Result<Vec<KnowledgeCollection>, Error> {
    let sql = format!(
        "SELECT {COLLECTION} FROM knowledge.collections c WHERE c.workspace_id=$1 AND knowledge.reaches(c.workspace_id,c.id,$3,$4,$5) AND (c.archived_at IS NOT NULL)=$2
         AND ($6::uuid IS NULL OR (c.created_at,c.id)>(SELECT created_at,id FROM knowledge.collections WHERE workspace_id=$1 AND id=$6))
         AND ($7::text IS NULL OR strpos(lower(c.name),lower($7))>0) ORDER BY c.created_at,c.id LIMIT $8"
    );
    Ok(
        sqlx::query_scalar::<_, Json<KnowledgeCollection>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(q.archived.unwrap_or(false))
            .bind(r.all)
            .bind(r.wide)
            .bind(r.membership)
            .bind(q.after)
            .bind(&q.search)
            .bind(limit)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|c| c.0)
            .collect(),
    )
}
pub(super) async fn insert_collection(
    db: &mut PgConnection,
    ws: Uuid,
    input: &SaveKnowledgeCollection,
    creator: Uuid,
) -> Result<Uuid, Error> {
    Ok(sqlx::query_scalar("INSERT INTO knowledge.collections(workspace_id,name,description,restricted,created_by_membership_id) VALUES($1,$2,$3,$4,$5) RETURNING id")
        .bind(ws).bind(input.name.trim()).bind(&input.description).bind(input.restricted).bind(creator)
        .fetch_one(db).await?)
}
pub(super) async fn update_collection(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    input: &SaveKnowledgeCollection,
) -> Result<(), Error> {
    sqlx::query("UPDATE knowledge.collections SET name=$3,description=$4,restricted=$5,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(id).bind(input.name.trim()).bind(&input.description).bind(input.restricted)
        .execute(db).await?;
    Ok(())
}
pub(super) async fn archive_collection(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    archived: bool,
) -> Result<(), Error> {
    sqlx::query("UPDATE knowledge.collections SET archived_at=CASE WHEN $3 THEN COALESCE(archived_at,now()) END,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(id).bind(archived).execute(db).await?;
    Ok(())
}
const MEMBER: &str = "SELECT jsonb_build_object('id',m.id,'display_name',u.display_name,'active',m.state='active','added_at',l.added_at::text) FROM knowledge.collection_members l JOIN suite.memberships m ON (m.workspace_id,m.id)=(l.workspace_id,l.membership_id) JOIN suite.users u ON u.id=m.user_id WHERE l.workspace_id=$1 AND l.collection_id=$2";
pub(super) async fn members(
    db: &mut PgConnection,
    ws: Uuid,
    collection: Uuid,
) -> Result<Vec<KnowledgeMember>, Error> {
    let sql = format!("{MEMBER} ORDER BY u.display_name,m.id");
    Ok(
        sqlx::query_scalar::<_, Json<KnowledgeMember>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(collection)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|m| m.0)
            .collect(),
    )
}
pub(super) async fn member(
    db: &mut PgConnection,
    ws: Uuid,
    collection: Uuid,
    membership: Uuid,
) -> Result<KnowledgeMember, Error> {
    let sql = format!("{MEMBER} AND l.membership_id=$3");
    Ok(
        sqlx::query_scalar::<_, Json<KnowledgeMember>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(collection)
            .bind(membership)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
pub(super) async fn directory(
    db: &mut PgConnection,
    ws: Uuid,
) -> Result<Vec<KnowledgeMember>, Error> {
    Ok(sqlx::query_scalar::<_, Json<KnowledgeMember>>("SELECT jsonb_build_object('id',m.id,'display_name',u.display_name,'active',true,'added_at',NULL) FROM suite.memberships m JOIN suite.users u ON u.id=m.user_id WHERE m.workspace_id=$1 AND m.state='active' ORDER BY u.display_name,m.id LIMIT 500")
        .bind(ws).fetch_all(db).await?.into_iter().map(|m| m.0).collect())
}
pub(super) async fn is_active_member(
    db: &mut PgConnection,
    ws: Uuid,
    membership: Uuid,
) -> Result<bool, Error> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM suite.memberships WHERE workspace_id=$1 AND id=$2 AND state='active')")
        .bind(ws).bind(membership).fetch_one(db).await?)
}
pub(super) async fn member_count(
    db: &mut PgConnection,
    ws: Uuid,
    collection: Uuid,
) -> Result<i64, Error> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM knowledge.collection_members WHERE workspace_id=$1 AND collection_id=$2",
    )
    .bind(ws)
    .bind(collection)
    .fetch_one(db)
    .await?)
}
pub(super) async fn add_member(
    db: &mut PgConnection,
    ws: Uuid,
    collection: Uuid,
    membership: Uuid,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO knowledge.collection_members(workspace_id,collection_id,membership_id) VALUES($1,$2,$3) ON CONFLICT DO NOTHING")
        .bind(ws).bind(collection).bind(membership).execute(db).await?;
    Ok(())
}
pub(super) async fn remove_member(
    db: &mut PgConnection,
    ws: Uuid,
    collection: Uuid,
    membership: Uuid,
) -> Result<bool, Error> {
    Ok(sqlx::query(
        "DELETE FROM knowledge.collection_members WHERE workspace_id=$1 AND collection_id=$2 AND membership_id=$3",
    )
    .bind(ws)
    .bind(collection)
    .bind(membership)
    .execute(db)
    .await?
    .rows_affected()
        > 0)
}
pub(super) async fn document(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    r: Reach,
) -> Result<KnowledgeDocument, Error> {
    let sql = format!(
        "SELECT {DOCUMENT} WHERE d.workspace_id=$1 AND d.id=$2 AND knowledge.reaches(d.workspace_id,d.collection_id,$3,$4,$5)"
    );
    Ok(
        sqlx::query_scalar::<_, Json<KnowledgeDocument>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(id)
            .bind(r.all)
            .bind(r.wide)
            .bind(r.membership)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
pub(super) async fn documents(
    db: &mut PgConnection,
    ws: Uuid,
    q: &ListKnowledgeDocuments,
    r: Reach,
    limit: i64,
) -> Result<Vec<KnowledgeDocumentSummary>, Error> {
    let sql = format!(
        "SELECT {SUMMARY} WHERE d.workspace_id=$1 AND (d.archived_at IS NOT NULL)=$2 AND knowledge.reaches(d.workspace_id,d.collection_id,$3,$4,$5)
         AND ($6::uuid IS NULL OR (d.created_at,d.id)>(SELECT created_at,id FROM knowledge.documents WHERE workspace_id=$1 AND id=$6))
         AND ($7::uuid IS NULL OR d.collection_id=$7) AND ($8::text IS NULL OR d.kind=$8) ORDER BY d.created_at,d.id LIMIT $9"
    );
    Ok(
        sqlx::query_scalar::<_, Json<KnowledgeDocumentSummary>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(q.archived.unwrap_or(false))
            .bind(r.all)
            .bind(r.wide)
            .bind(r.membership)
            .bind(q.after)
            .bind(q.collection_id)
            .bind(q.kind.map(|k| k.key()))
            .bind(limit)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|d| d.0)
            .collect(),
    )
}
pub(super) async fn insert_document(
    db: &mut PgConnection,
    ws: Uuid,
    collection: Uuid,
    kind: &str,
    title: &str,
    body: &str,
    text: &str,
) -> Result<Uuid, Error> {
    Ok(sqlx::query_scalar("INSERT INTO knowledge.documents(workspace_id,collection_id,kind,title,body,search_text) VALUES($1,$2,$3,$4,$5,$6) RETURNING id")
        .bind(ws).bind(collection).bind(kind).bind(title).bind(body).bind(text)
        .fetch_one(db).await?)
}
#[allow(clippy::too_many_arguments)]
pub(super) async fn update_head(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    collection: Uuid,
    kind: &str,
    title: &str,
    body: &str,
    text: &str,
    number: i32,
) -> Result<(), Error> {
    sqlx::query("UPDATE knowledge.documents SET collection_id=$3,kind=$4,title=$5,body=$6,search_text=$7,head_number=$8,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(id).bind(collection).bind(kind).bind(title).bind(body).bind(text).bind(number)
        .execute(db).await?;
    Ok(())
}
pub(super) async fn archive_document(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    archived: bool,
) -> Result<(), Error> {
    sqlx::query("UPDATE knowledge.documents SET archived_at=CASE WHEN $3 THEN COALESCE(archived_at,now()) END,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(ws).bind(id).bind(archived).execute(db).await?;
    Ok(())
}
pub(super) async fn insert_revision(
    db: &mut PgConnection,
    ws: Uuid,
    new: &NewRevision<'_>,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO knowledge.revisions(workspace_id,document_id,number,collection_id,change,kind,title,body,restored_from_number,created_by_membership_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)")
        .bind(ws).bind(new.document).bind(new.number).bind(new.collection).bind(new.change).bind(new.kind)
        .bind(new.title).bind(new.body).bind(new.restored_from).bind(new.author)
        .execute(db).await?;
    Ok(())
}
pub(super) async fn revision(
    db: &mut PgConnection,
    ws: Uuid,
    document: Uuid,
    id: Uuid,
    r: Reach,
) -> Result<KnowledgeRevision, Error> {
    let sql = format!(
        "SELECT {REVISION} WHERE r.workspace_id=$1 AND r.document_id=$2 AND {REVISION_VISIBLE} AND r.id=$6"
    );
    Ok(
        sqlx::query_scalar::<_, Json<KnowledgeRevision>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(document)
            .bind(r.all)
            .bind(r.wide)
            .bind(r.membership)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
pub(super) async fn revisions(
    db: &mut PgConnection,
    ws: Uuid,
    document: Uuid,
    after: Option<Uuid>,
    r: Reach,
    limit: i64,
) -> Result<Vec<KnowledgeRevision>, Error> {
    let sql = format!(
        "SELECT {REVISION} WHERE r.workspace_id=$1 AND r.document_id=$2 AND {REVISION_VISIBLE}
         AND ($6::uuid IS NULL OR r.number<(SELECT number FROM knowledge.revisions WHERE workspace_id=$1 AND id=$6)) ORDER BY r.number DESC LIMIT $7"
    );
    Ok(
        sqlx::query_scalar::<_, Json<KnowledgeRevision>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(document)
            .bind(r.all)
            .bind(r.wide)
            .bind(r.membership)
            .bind(after)
            .bind(limit)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|x| x.0)
            .collect(),
    )
}
pub(super) async fn previews(
    db: &mut PgConnection,
    ws: Uuid,
    ids: &[Uuid],
    r: Reach,
) -> Result<Vec<(Uuid, KnowledgeLinkPreview)>, Error> {
    let rows = sqlx::query_as::<_, (Uuid, Json<KnowledgeLinkPreview>)>(
        "SELECT d.id,jsonb_build_object('title',d.title,'collection_name',c.name,'kind',d.kind,'excerpt',left(d.search_text,240),'archived',d.archived_at IS NOT NULL OR c.archived_at IS NOT NULL)
         FROM knowledge.documents d JOIN knowledge.collections c ON c.workspace_id=d.workspace_id AND c.id=d.collection_id
         WHERE d.workspace_id=$1 AND d.id=ANY($2) AND knowledge.reaches(d.workspace_id,d.collection_id,$3,$4,$5)",
    )
    .bind(ws)
    .bind(ids)
    .bind(r.all)
    .bind(r.wide)
    .bind(r.membership)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|(id, p)| (id, p.0)).collect())
}
pub(super) async fn search_total(
    db: &mut PgConnection,
    ws: Uuid,
    q: &KnowledgeSearchQuery,
    r: Reach,
) -> Result<i64, Error> {
    let sql = format!("SELECT count(*) {MATCHES}");
    Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(ws)
        .bind(&q.q)
        .bind(r.all)
        .bind(r.wide)
        .bind(r.membership)
        .bind(q.collection_id)
        .bind(q.kind.map(|k| k.key()))
        .fetch_one(db)
        .await?)
}
/// Ranks compare with the reader's previous page by value. Headlines are made only for
/// the rows returned, after the visibility filter, from visible rows' own text.
pub(super) async fn search(
    db: &mut PgConnection,
    ws: Uuid,
    q: &KnowledgeSearchQuery,
    cursor: Option<(f32, Uuid)>,
    r: Reach,
    limit: i64,
) -> Result<Vec<(KnowledgeSearchHit, f32, String)>, Error> {
    let sql = format!(
        "SELECT jsonb_build_object('id',x.id,'title',x.title,'collection_id',x.collection_id,'collection_name',x.collection_name,'kind',x.kind,'updated_at',x.updated_at),
          x.rank,ts_headline('english',x.search_text,x.q,$11)
         FROM (SELECT d.id,d.title,d.collection_id,c.name AS collection_name,d.kind,d.updated_at::text AS updated_at,d.search_text,q,ts_rank(d.search_vector,q) AS rank {MATCHES}) x
         WHERE ($8::real IS NULL OR x.rank<$8 OR (x.rank=$8 AND x.id>$9)) ORDER BY x.rank DESC,x.id LIMIT $10"
    );
    let options = format!(
        "StartSel=\"{}\",StopSel=\"{}\",MaxFragments=2,MaxWords=30,MinWords=10,FragmentDelimiter=\" … \"",
        super::MATCH_START,
        super::MATCH_END
    );
    let rows = sqlx::query_as::<_, (Json<KnowledgeSearchHit>, f32, String)>(sqlx::AssertSqlSafe(
        sql.as_str(),
    ))
    .bind(ws)
    .bind(&q.q)
    .bind(r.all)
    .bind(r.wide)
    .bind(r.membership)
    .bind(q.collection_id)
    .bind(q.kind.map(|k| k.key()))
    .bind(cursor.map(|c| c.0))
    .bind(cursor.map(|c| c.1))
    .bind(limit)
    .bind(options)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(h, rank, s)| (h.0, rank, s))
        .collect())
}
pub(super) async fn suggest(
    db: &mut PgConnection,
    ws: Uuid,
    q: &str,
    r: Reach,
    limit: i64,
) -> Result<Vec<KnowledgeSuggestion>, Error> {
    Ok(sqlx::query_scalar::<_, Json<KnowledgeSuggestion>>(
        "SELECT jsonb_build_object('id',d.id,'title',d.title,'collection_name',c.name,'kind',d.kind)
         FROM knowledge.documents d JOIN knowledge.collections c ON c.workspace_id=d.workspace_id AND c.id=d.collection_id
         WHERE d.workspace_id=$1 AND strpos(lower(d.title),lower($2))>0 AND d.archived_at IS NULL AND c.archived_at IS NULL
         AND knowledge.reaches(d.workspace_id,d.collection_id,$3,$4,$5)
         ORDER BY strpos(lower(d.title),lower($2)),d.title,d.id LIMIT $6",
    )
    .bind(ws)
    .bind(q.trim())
    .bind(r.all)
    .bind(r.wide)
    .bind(r.membership)
    .bind(limit)
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|s| s.0)
    .collect())
}
pub(super) async fn activity_visible(
    db: &mut PgConnection,
    ws: Uuid,
    revision: Uuid,
    r: Reach,
) -> Result<bool, Error> {
    let sql = format!("SELECT EXISTS(SELECT 1 {ACTIVITY} AND r.id=$2 AND {REVISION_VISIBLE})");
    Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(ws)
        .bind(revision)
        .bind(r.all)
        .bind(r.wide)
        .bind(r.membership)
        .fetch_one(db)
        .await?)
}
pub(super) async fn activity(
    db: &mut PgConnection,
    ws: Uuid,
    after: Option<Uuid>,
    r: Reach,
    limit: i64,
) -> Result<Vec<KnowledgeActivity>, Error> {
    let sql = format!(
        "SELECT jsonb_build_object('revision_id',r.id,'document_id',r.document_id,'number',r.number,'change',r.change,'title',r.title,'collection_id',r.collection_id,'collection_name',o.name,'actor_name',u.display_name,'created_at',r.created_at::text)
         {ACTIVITY} AND {REVISION_VISIBLE}
         AND ($2::uuid IS NULL OR (r.created_at,r.id)<(SELECT created_at,id FROM knowledge.revisions WHERE workspace_id=$1 AND id=$2)) ORDER BY r.created_at DESC,r.id DESC LIMIT $6"
    );
    Ok(
        sqlx::query_scalar::<_, Json<KnowledgeActivity>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(after)
            .bind(r.all)
            .bind(r.wide)
            .bind(r.membership)
            .bind(limit)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|a| a.0)
            .collect(),
    )
}
