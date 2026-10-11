// SPDX-License-Identifier: AGPL-3.0-only
//! The knowledge base owns collections, documents and immutable revisions, found
//! through PostgreSQL full-text search. `knowledge.reaches` in the migration is the
//! one definition of who reaches a collection, and every read names it in the same
//! statement that selects the rows, so search results, counts, snippets, previews,
//! exports and the activity feed cannot disagree. Hidden records answer exactly as
//! records that do not exist. It uses the neutral rich text parser and needs no
//! other optional application.
mod dao;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    files, modules, reliability,
    rich_text::{self, RichBlock},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

const READ: &str = "knowledge.read";
const EDIT: &str = "knowledge.documents.edit";
const MANAGE: &str = "knowledge.collections.manage";
const OVERRIDE: &str = "knowledge.restricted.access";
const MAX_BODY_CHARS: usize = 100_000;
const MAX_MEMBERS: i64 = 500;
const MAX_LINKS: usize = 50;
/// `ts_headline` marks matches with these; document text can never contain them.
const MATCH_START: char = '\u{1}';
const MATCH_END: char = '\u{2}';

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeDocumentKind {
    #[default]
    Article,
    Sop,
}
impl KnowledgeDocumentKind {
    fn key(self) -> &'static str {
        match self {
            Self::Article => "article",
            Self::Sop => "sop",
        }
    }
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct KnowledgeCollection {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    /// Reachable only through its member list or the explicit override.
    pub restricted: bool,
    pub archived: bool,
    /// The calling member is listed on this collection.
    pub is_member: bool,
    /// Active documents; anyone who sees the collection may see all of them.
    pub document_count: i64,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveKnowledgeCollection {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub restricted: bool,
    /// Required for updates; omitted on creation.
    pub expected_version: Option<i64>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct ListKnowledgeCollections {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
    pub search: Option<String>,
    pub archived: Option<bool>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct KnowledgeCollectionPage {
    pub items: Vec<KnowledgeCollection>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ArchiveKnowledgeRecord {
    pub expected_version: i64,
    pub archived: bool,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct KnowledgeMember {
    pub id: Uuid,
    pub display_name: String,
    pub active: bool,
    pub added_at: Option<String>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AddKnowledgeMember {
    pub membership_id: Uuid,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct KnowledgeMemberRemoved {
    pub collection_id: Uuid,
    pub membership_id: Uuid,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct KnowledgePageQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
}
/// What a reader may learn about a linked document. Absent means the same whether
/// the target is hidden, in another workspace or does not exist.
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct KnowledgeLinkPreview {
    pub title: String,
    pub collection_name: String,
    pub kind: KnowledgeDocumentKind,
    pub excerpt: String,
    pub archived: bool,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct LinkedKnowledgeDocument {
    pub id: Uuid,
    pub preview: Option<KnowledgeLinkPreview>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct KnowledgeDocument {
    pub id: Uuid,
    pub collection_id: Uuid,
    pub collection_name: String,
    pub collection_restricted: bool,
    pub collection_archived: bool,
    pub kind: KnowledgeDocumentKind,
    pub title: String,
    /// Plain source in the rich text subset.
    pub body: String,
    /// Typed blocks parsed from `body`; renderers use only these.
    #[serde(default)]
    #[schema(required = true)]
    pub blocks: Vec<RichBlock>,
    /// Documents this one links to, as far as the reader may see them.
    #[serde(default)]
    #[schema(required = true)]
    pub linked: Vec<LinkedKnowledgeDocument>,
    pub revision_number: i32,
    pub updated_by_name: String,
    pub archived: bool,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct KnowledgeDocumentSummary {
    pub id: Uuid,
    pub collection_id: Uuid,
    pub collection_name: String,
    pub kind: KnowledgeDocumentKind,
    pub title: String,
    pub revision_number: i32,
    pub archived: bool,
    pub version: i64,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateKnowledgeDocument {
    pub collection_id: Uuid,
    #[serde(default)]
    pub kind: KnowledgeDocumentKind,
    pub title: String,
    #[serde(default)]
    pub body: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveKnowledgeDocument {
    pub expected_version: i64,
    #[serde(default)]
    pub kind: KnowledgeDocumentKind,
    pub title: String,
    #[serde(default)]
    pub body: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MoveKnowledgeDocument {
    pub expected_version: i64,
    pub collection_id: Uuid,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RestoreKnowledgeRevision {
    pub expected_version: i64,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct ListKnowledgeDocuments {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
    pub collection_id: Option<Uuid>,
    #[param(inline)]
    pub kind: Option<KnowledgeDocumentKind>,
    pub archived: Option<bool>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct KnowledgeDocumentPage {
    pub items: Vec<KnowledgeDocumentSummary>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct KnowledgeSearchQuery {
    /// Words to find; quotes, `or` and `-word` work as in a web search.
    pub q: String,
    pub limit: Option<i64>,
    /// The `next_cursor` of the previous page.
    pub after: Option<String>,
    pub collection_id: Option<Uuid>,
    #[param(inline)]
    pub kind: Option<KnowledgeDocumentKind>,
}
/// One stretch of a snippet; matched words are marked by structure, never markup.
#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct KnowledgeSnippetPart {
    pub text: String,
    pub matched: bool,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct KnowledgeSearchHit {
    pub id: Uuid,
    pub title: String,
    pub collection_id: Uuid,
    pub collection_name: String,
    pub kind: KnowledgeDocumentKind,
    #[serde(default)]
    #[schema(required = true)]
    pub snippet: Vec<KnowledgeSnippetPart>,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct KnowledgeSearchPage {
    pub items: Vec<KnowledgeSearchHit>,
    /// All matches this reader may see, across pages.
    pub total: i64,
    pub next_cursor: Option<String>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct KnowledgeSuggestQuery {
    pub q: String,
    pub limit: Option<i64>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct KnowledgeSuggestion {
    pub id: Uuid,
    pub title: String,
    pub collection_name: String,
    pub kind: KnowledgeDocumentKind,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct KnowledgeSuggestions {
    pub items: Vec<KnowledgeSuggestion>,
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeChange {
    Created,
    Edited,
    Moved,
    Restored,
}
impl KnowledgeChange {
    fn key(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Edited => "edited",
            Self::Moved => "moved",
            Self::Restored => "restored",
        }
    }
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct KnowledgeRevision {
    pub id: Uuid,
    pub document_id: Uuid,
    pub number: i32,
    pub change: KnowledgeChange,
    /// The collection the document was in when this revision was made.
    pub collection_id: Uuid,
    pub collection_name: String,
    pub kind: KnowledgeDocumentKind,
    pub title: String,
    pub body: String,
    #[serde(default)]
    #[schema(required = true)]
    pub blocks: Vec<RichBlock>,
    pub restored_from_number: Option<i32>,
    pub created_by_name: String,
    pub created_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct KnowledgeRevisionPage {
    pub items: Vec<KnowledgeRevision>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct KnowledgeActivity {
    pub revision_id: Uuid,
    pub document_id: Uuid,
    pub number: i32,
    pub change: KnowledgeChange,
    pub title: String,
    pub collection_id: Uuid,
    pub collection_name: String,
    pub actor_name: String,
    pub created_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct KnowledgeActivityPage {
    pub items: Vec<KnowledgeActivity>,
    pub next_cursor: Option<Uuid>,
}
/// A retried mutation remembers only which record it changed, then answers from
/// current state under current authority. Neither the retry table nor a retry can
/// carry document text past a revocation or a move.
#[derive(Serialize, Deserialize)]
struct Applied {
    id: Uuid,
}
/// The collections an action covers for this member, passed to `knowledge.reaches`.
#[derive(Clone, Copy)]
struct Reach {
    all: bool,
    wide: bool,
    membership: Uuid,
}
struct Write {
    access: access::Access,
    read: Reach,
    write: Reach,
}

async fn authorize(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    action: &str,
) -> Result<access::Access, Error> {
    let a = access::require(db, actor.identity, actor.workspace, action).await?;
    modules::require(db, actor.workspace, "knowledge").await?;
    Ok(a)
}
/// Authority is read from the database on every call, so a change applies to the
/// next request on the same token. The override only widens a workspace-wide grant
/// of the action in hand; it grants no action by itself.
async fn reach(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    action: &str,
) -> Result<(access::Access, Reach), Error> {
    let a = authorize(db, actor, action).await?;
    let all = a.workspace_wide
        && match access::require(db, actor.identity, actor.workspace, OVERRIDE).await {
            Ok(o) => o.workspace_wide,
            Err(Error::Forbidden) => false,
            Err(error) => return Err(error),
        };
    let r = Reach {
        all,
        wide: a.workspace_wide,
        membership: a.membership_id,
    };
    Ok((a, r))
}
/// Check write authority, serialize with application selection and every other
/// knowledge mutation in the workspace, then re-check. Writers must also read.
async fn mutation(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    action: &str,
) -> Result<Write, Error> {
    authorize(db, actor, action).await?;
    modules::lock_workspace(db, actor.workspace).await?;
    let (_, read) = reach(db, actor, READ).await?;
    let (access, write) = reach(db, actor, action).await?;
    Ok(Write {
        access,
        read,
        write,
    })
}
fn page_limit(limit: Option<i64>, default: i64, most: i64) -> Result<i64, Error> {
    let limit = limit.unwrap_or(default);
    if !(1..=most).contains(&limit) {
        return Err(Error::Invalid("Limit is outside the allowed range"));
    }
    Ok(limit)
}
fn validate_query(q: &str) -> Result<(), Error> {
    if q.trim().is_empty() || q.len() > 200 || q.chars().any(char::is_control) {
        return Err(Error::Invalid(
            "Search must be 1–200 bytes of printable text",
        ));
    }
    Ok(())
}
fn validate_collection(input: &SaveKnowledgeCollection) -> Result<(), Error> {
    if input.name.trim().is_empty()
        || input.name.chars().count() > 120
        || input.name.chars().any(char::is_control)
        || input.description.chars().count() > 2000
        || input.description.contains('\0')
    {
        return Err(Error::Invalid(
            "Name must be 1–120 printable characters and description at most 2000",
        ));
    }
    Ok(())
}
fn validate_document(title: &str, body: &str) -> Result<(), Error> {
    if title.trim().is_empty() || title.chars().count() > 200 || title.chars().any(char::is_control)
    {
        return Err(Error::Invalid("Title must be 1–200 printable characters"));
    }
    if body.chars().count() > MAX_BODY_CHARS
        || body
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(Error::Invalid(
            "Text must be at most 100000 characters, without control characters",
        ));
    }
    rich_text::validate(body)
}
/// What a retry is compared with: enough to recognize the input, without keeping a
/// second copy of restricted text outside the document's own tables.
fn fingerprint(title: &str, body: &str, rest: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "title_sha256": files::digest(title.as_bytes()),
        "body_sha256": files::digest(body.as_bytes()),
        "rest": rest,
    })
}
/// Same-site links to documents in this workspace, in reading order, without repeats.
fn document_links(blocks: &[RichBlock], workspace: Uuid, this: Uuid) -> Vec<Uuid> {
    let prefix = format!("/workspaces/{workspace}/knowledge/documents/");
    let mut links = Vec::new();
    for href in rich_text::hrefs(blocks) {
        let Some(rest) = href.strip_prefix(&prefix) else {
            continue;
        };
        let id = rest.split(['/', '?', '#']).next().unwrap_or_default();
        if let Ok(id) = Uuid::parse_str(id)
            && id != this
            && !links.contains(&id)
            && links.len() < MAX_LINKS
        {
            links.push(id);
        }
    }
    links
}
fn encode_cursor(rank: f32, id: Uuid) -> String {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    URL_SAFE_NO_PAD.encode(format!("{rank}:{id}"))
}
/// A cursor holds the last visible result's own rank and ID and is only ever compared
/// with rows the reader may see. Nothing is looked up, so a forged one reveals nothing.
fn decode_cursor(cursor: &str) -> Result<(f32, Uuid), Error> {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    URL_SAFE_NO_PAD
        .decode(cursor)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .and_then(|text| {
            let (rank, id) = text.split_once(':')?;
            let rank = rank.parse::<f32>().ok().filter(|r| r.is_finite())?;
            Some((rank, Uuid::parse_str(id).ok()?))
        })
        .ok_or(Error::Invalid("Search position is not valid"))
}
fn snippet_parts(headline: &str) -> Vec<KnowledgeSnippetPart> {
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut matched = false;
    for c in headline.chars() {
        if c == MATCH_START || c == MATCH_END {
            if !text.is_empty() {
                parts.push(KnowledgeSnippetPart {
                    text: std::mem::take(&mut text),
                    matched,
                });
            }
            matched = c == MATCH_START;
        } else {
            text.push(c);
        }
    }
    if !text.is_empty() {
        parts.push(KnowledgeSnippetPart { text, matched });
    }
    parts
}
/// Whether this action covers the document's collection, once the reader can see it.
async fn require_reach(
    db: &mut PgConnection,
    workspace: Uuid,
    collection: Uuid,
    write: Reach,
) -> Result<(), Error> {
    if !dao::reaches(db, workspace, collection, write).await? {
        return Err(Error::Forbidden);
    }
    Ok(())
}

/// Previews of linked documents, under the caller's own current authority. This is
/// the interface other applications use to cite a document: an application that links
/// articles stores only the ID and asks here each time it shows one. With the
/// knowledge base disabled or no read permission, every preview is simply absent.
pub async fn previews(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    ids: &[Uuid],
) -> Result<Vec<LinkedKnowledgeDocument>, Error> {
    let found = match reach(db, actor, READ).await {
        Ok((_, read)) => dao::previews(db, actor.workspace, ids, read).await?,
        Err(Error::Forbidden) => Vec::new(),
        Err(error) => return Err(error),
    };
    let mut found: std::collections::HashMap<Uuid, KnowledgeLinkPreview> =
        found.into_iter().collect();
    Ok(ids
        .iter()
        .map(|id| LinkedKnowledgeDocument {
            id: *id,
            preview: found.remove(id),
        })
        .collect())
}
/// A document as this reader may see it, with its blocks and linked previews.
async fn present(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    read: Reach,
    id: Uuid,
) -> Result<KnowledgeDocument, Error> {
    let mut document = dao::document(db, actor.workspace, id, read).await?;
    document.blocks = rich_text::parse(&document.body);
    let links = document_links(&document.blocks, actor.workspace, id);
    document.linked = previews(db, actor, &links).await?;
    Ok(document)
}
fn present_revision(mut revision: KnowledgeRevision) -> KnowledgeRevision {
    revision.blocks = rich_text::parse(&revision.body);
    revision
}

pub async fn list_collections(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: ListKnowledgeCollections,
) -> Result<KnowledgeCollectionPage, Error> {
    let limit = page_limit(query.limit, 50, 100)?;
    if let Some(search) = &query.search {
        validate_query(search)?;
    }
    let mut tx = pool.begin().await?;
    let (_, read) = reach(&mut tx, actor, READ).await?;
    if let Some(after) = query.after {
        dao::collection(&mut tx, actor.workspace, after, read).await?;
    }
    let mut items = dao::collections(&mut tx, actor.workspace, &query, read, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|c| c.id)
    } else {
        None
    };
    Ok(KnowledgeCollectionPage { items, next_cursor })
}
pub async fn get_collection(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<KnowledgeCollection, Error> {
    let mut tx = pool.begin().await?;
    let (_, read) = reach(&mut tx, actor, READ).await?;
    dao::collection(&mut tx, actor.workspace, id, read).await
}
pub async fn save_collection(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Option<Uuid>,
    key: &str,
    input: SaveKnowledgeCollection,
) -> Result<KnowledgeCollection, Error> {
    validate_collection(&input)?;
    if id.is_some() != input.expected_version.is_some() {
        return Err(Error::Invalid(
            "Updates require expected_version; creation must omit it",
        ));
    }
    let mut tx = pool.begin().await?;
    let w = mutation(&mut tx, actor, MANAGE).await?;
    let operation = id
        .map(|id| format!("knowledge.collections.update:{id}"))
        .unwrap_or_else(|| "knowledge.collections.create".into());
    let request = serde_json::to_value(&input).expect("collection serializes");
    if let Some(saved) = reliability::replay::<Applied>(
        &mut tx,
        actor.workspace,
        &w.access,
        &operation,
        key,
        &request,
    )
    .await?
    {
        return dao::collection(&mut tx, actor.workspace, saved.id, w.read).await;
    }
    // Whoever creates or restricts a collection stays able to reach it.
    let mut list_actor = true;
    let id = if let Some(id) = id {
        let current = dao::collection(&mut tx, actor.workspace, id, w.read).await?;
        list_actor = input.restricted && !current.restricted;
        require_reach(&mut tx, actor.workspace, id, w.write).await?;
        if Some(current.version) != input.expected_version || current.archived {
            return Err(Error::Conflict(
                "Collection changed or is archived; reload before editing",
            ));
        }
        dao::update_collection(&mut tx, actor.workspace, id, &input).await?;
        id
    } else {
        if !w.access.workspace_wide {
            return Err(Error::Forbidden);
        }
        dao::insert_collection(&mut tx, actor.workspace, &input, w.access.membership_id).await?
    };
    if list_actor {
        dao::add_member(&mut tx, actor.workspace, id, w.access.membership_id).await?;
    }
    reliability::audit(
        &mut tx,
        actor,
        &w.access,
        "knowledge.collection_saved",
        id,
        serde_json::json!({"restricted":input.restricted}),
    )
    .await?;
    let result = dao::collection(&mut tx, actor.workspace, id, w.read).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &w.access,
        &operation,
        key,
        &request,
        &Applied { id },
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
/// An archived collection stays readable by those who reach it, leaves search, and
/// accepts no edits until restored.
pub async fn archive_collection(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: ArchiveKnowledgeRecord,
) -> Result<KnowledgeCollection, Error> {
    let mut tx = pool.begin().await?;
    let w = mutation(&mut tx, actor, MANAGE).await?;
    let operation = format!("knowledge.collections.archive:{id}");
    let request = serde_json::to_value(&input).expect("archive serializes");
    if reliability::replay::<Applied>(
        &mut tx,
        actor.workspace,
        &w.access,
        &operation,
        key,
        &request,
    )
    .await?
    .is_some()
    {
        return dao::collection(&mut tx, actor.workspace, id, w.read).await;
    }
    let current = dao::collection(&mut tx, actor.workspace, id, w.read).await?;
    require_reach(&mut tx, actor.workspace, id, w.write).await?;
    if current.version != input.expected_version {
        return Err(Error::Conflict("Collection changed; reload before editing"));
    }
    dao::archive_collection(&mut tx, actor.workspace, id, input.archived).await?;
    reliability::audit(
        &mut tx,
        actor,
        &w.access,
        "knowledge.collection_archived",
        id,
        serde_json::json!({"archived":input.archived}),
    )
    .await?;
    let result = dao::collection(&mut tx, actor.workspace, id, w.read).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &w.access,
        &operation,
        key,
        &request,
        &Applied { id },
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn members(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    collection: Uuid,
) -> Result<Vec<KnowledgeMember>, Error> {
    let mut tx = pool.begin().await?;
    let (_, read) = reach(&mut tx, actor, READ).await?;
    dao::collection(&mut tx, actor.workspace, collection, read).await?;
    dao::members(&mut tx, actor.workspace, collection).await
}
/// Active workspace members a manager may list; nobody else needs the directory.
pub async fn directory(
    pool: &PgPool,
    actor: &ActorContext<'_>,
) -> Result<Vec<KnowledgeMember>, Error> {
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, MANAGE).await?;
    dao::directory(&mut tx, actor.workspace).await
}
pub async fn add_member(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    collection: Uuid,
    key: &str,
    input: AddKnowledgeMember,
) -> Result<KnowledgeMember, Error> {
    let mut tx = pool.begin().await?;
    let w = mutation(&mut tx, actor, MANAGE).await?;
    let operation = format!("knowledge.members.add:{collection}");
    let request = serde_json::to_value(&input).expect("member serializes");
    let retried = reliability::replay::<Applied>(
        &mut tx,
        actor.workspace,
        &w.access,
        &operation,
        key,
        &request,
    )
    .await?
    .is_some();
    dao::collection(&mut tx, actor.workspace, collection, w.read).await?;
    if retried {
        return dao::member(&mut tx, actor.workspace, collection, input.membership_id).await;
    }
    require_reach(&mut tx, actor.workspace, collection, w.write).await?;
    if !dao::is_active_member(&mut tx, actor.workspace, input.membership_id).await? {
        return Err(Error::Invalid("Choose an active member of this workspace"));
    }
    if dao::member_count(&mut tx, actor.workspace, collection).await? >= MAX_MEMBERS {
        return Err(Error::Conflict("A collection lists at most 500 members"));
    }
    dao::add_member(&mut tx, actor.workspace, collection, input.membership_id).await?;
    reliability::audit(
        &mut tx,
        actor,
        &w.access,
        "knowledge.member_added",
        collection,
        serde_json::json!({"membership_id":input.membership_id}),
    )
    .await?;
    let result = dao::member(&mut tx, actor.workspace, collection, input.membership_id).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &w.access,
        &operation,
        key,
        &request,
        &Applied {
            id: input.membership_id,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
/// Removal applies to the next request, on any token. A manager may remove themselves,
/// and an empty restricted collection stays reachable through the override.
pub async fn remove_member(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    collection: Uuid,
    membership: Uuid,
    key: &str,
) -> Result<KnowledgeMemberRemoved, Error> {
    let mut tx = pool.begin().await?;
    let w = mutation(&mut tx, actor, MANAGE).await?;
    let operation = format!("knowledge.members.remove:{collection}:{membership}");
    let request = serde_json::json!({});
    let result = KnowledgeMemberRemoved {
        collection_id: collection,
        membership_id: membership,
    };
    if reliability::replay::<KnowledgeMemberRemoved>(
        &mut tx,
        actor.workspace,
        &w.access,
        &operation,
        key,
        &request,
    )
    .await?
    .is_some()
    {
        return Ok(result);
    }
    dao::collection(&mut tx, actor.workspace, collection, w.read).await?;
    require_reach(&mut tx, actor.workspace, collection, w.write).await?;
    if !dao::remove_member(&mut tx, actor.workspace, collection, membership).await? {
        return Err(Error::NotFound);
    }
    reliability::audit(
        &mut tx,
        actor,
        &w.access,
        "knowledge.member_removed",
        collection,
        serde_json::json!({"membership_id":membership}),
    )
    .await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &w.access,
        &operation,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn list_documents(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: ListKnowledgeDocuments,
) -> Result<KnowledgeDocumentPage, Error> {
    let limit = page_limit(query.limit, 50, 100)?;
    let mut tx = pool.begin().await?;
    let (_, read) = reach(&mut tx, actor, READ).await?;
    if let Some(after) = query.after {
        dao::document(&mut tx, actor.workspace, after, read).await?;
    }
    let mut items = dao::documents(&mut tx, actor.workspace, &query, read, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|d| d.id)
    } else {
        None
    };
    Ok(KnowledgeDocumentPage { items, next_cursor })
}
pub async fn get_document(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<KnowledgeDocument, Error> {
    let mut tx = pool.begin().await?;
    let (_, read) = reach(&mut tx, actor, READ).await?;
    present(&mut tx, actor, read, id).await
}
pub async fn create_document(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    key: &str,
    input: CreateKnowledgeDocument,
) -> Result<KnowledgeDocument, Error> {
    validate_document(&input.title, &input.body)?;
    let mut tx = pool.begin().await?;
    let w = mutation(&mut tx, actor, EDIT).await?;
    let operation = "knowledge.documents.create";
    let request = fingerprint(
        &input.title,
        &input.body,
        serde_json::json!({"collection_id":input.collection_id,"kind":input.kind}),
    );
    if let Some(saved) = reliability::replay::<Applied>(
        &mut tx,
        actor.workspace,
        &w.access,
        operation,
        key,
        &request,
    )
    .await?
    {
        return present(&mut tx, actor, w.read, saved.id).await;
    }
    let collection = dao::collection(&mut tx, actor.workspace, input.collection_id, w.read).await?;
    require_reach(&mut tx, actor.workspace, collection.id, w.write).await?;
    if collection.archived {
        return Err(Error::Conflict(
            "Restore this collection before adding to it",
        ));
    }
    let text = rich_text::plain_text(&rich_text::parse(&input.body));
    let id = dao::insert_document(
        &mut tx,
        actor.workspace,
        collection.id,
        input.kind.key(),
        input.title.trim(),
        &input.body,
        &text,
    )
    .await?;
    dao::insert_revision(
        &mut tx,
        actor.workspace,
        &dao::NewRevision {
            document: id,
            number: 1,
            collection: collection.id,
            change: KnowledgeChange::Created.key(),
            kind: input.kind.key(),
            title: input.title.trim(),
            body: &input.body,
            restored_from: None,
            author: w.access.membership_id,
        },
    )
    .await?;
    reliability::audit(
        &mut tx,
        actor,
        &w.access,
        "knowledge.document_created",
        id,
        serde_json::json!({"collection_id":collection.id}),
    )
    .await?;
    let result = present(&mut tx, actor, w.read, id).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &w.access,
        operation,
        key,
        &request,
        &Applied { id },
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
/// A document this writer may change: visible first, so nothing about a hidden one
/// (its version included) is ever compared, then covered by the action, then current.
async fn editable(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    w: &Write,
    id: Uuid,
    expected_version: i64,
) -> Result<KnowledgeDocument, Error> {
    let current = dao::document(db, actor.workspace, id, w.read).await?;
    require_reach(db, actor.workspace, current.collection_id, w.write).await?;
    if current.version != expected_version {
        return Err(Error::Conflict(
            "Someone else changed this document; reload and reapply your edit",
        ));
    }
    if current.archived || current.collection_archived {
        return Err(Error::Conflict(
            "Restore this document and its collection before changing it",
        ));
    }
    Ok(current)
}
/// Append a revision and make it the document's current text.
#[allow(clippy::too_many_arguments)]
async fn append(
    db: &mut PgConnection,
    workspace: Uuid,
    current: &KnowledgeDocument,
    collection: Uuid,
    change: KnowledgeChange,
    kind: &str,
    title: &str,
    body: &str,
    restored_from: Option<i32>,
    author: Uuid,
) -> Result<(), Error> {
    let number = current.revision_number + 1;
    let text = rich_text::plain_text(&rich_text::parse(body));
    dao::insert_revision(
        db,
        workspace,
        &dao::NewRevision {
            document: current.id,
            number,
            collection,
            change: change.key(),
            kind,
            title,
            body,
            restored_from,
            author,
        },
    )
    .await?;
    dao::update_head(
        db, workspace, current.id, collection, kind, title, body, &text, number,
    )
    .await
}
pub async fn update_document(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: SaveKnowledgeDocument,
) -> Result<KnowledgeDocument, Error> {
    validate_document(&input.title, &input.body)?;
    let mut tx = pool.begin().await?;
    let w = mutation(&mut tx, actor, EDIT).await?;
    let operation = format!("knowledge.documents.update:{id}");
    let request = fingerprint(
        &input.title,
        &input.body,
        serde_json::json!({"expected_version":input.expected_version,"kind":input.kind}),
    );
    if reliability::replay::<Applied>(
        &mut tx,
        actor.workspace,
        &w.access,
        &operation,
        key,
        &request,
    )
    .await?
    .is_some()
    {
        return present(&mut tx, actor, w.read, id).await;
    }
    let current = editable(&mut tx, actor, &w, id, input.expected_version).await?;
    let title = input.title.trim();
    // Saving without a change adds nothing to the history.
    if current.title != title || current.body != input.body || current.kind != input.kind {
        append(
            &mut tx,
            actor.workspace,
            &current,
            current.collection_id,
            KnowledgeChange::Edited,
            input.kind.key(),
            title,
            &input.body,
            None,
            w.access.membership_id,
        )
        .await?;
        reliability::audit(
            &mut tx,
            actor,
            &w.access,
            "knowledge.document_edited",
            id,
            serde_json::json!({"revision":current.revision_number+1}),
        )
        .await?;
    }
    let result = present(&mut tx, actor, w.read, id).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &w.access,
        &operation,
        key,
        &request,
        &Applied { id },
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
/// Moving releases the current text to the target's readers and nothing older: the
/// move is a new revision written in the target, and earlier revisions keep needing
/// the collection they were written in. Access follows on the next request.
pub async fn move_document(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: MoveKnowledgeDocument,
) -> Result<KnowledgeDocument, Error> {
    let mut tx = pool.begin().await?;
    let w = mutation(&mut tx, actor, EDIT).await?;
    let operation = format!("knowledge.documents.move:{id}");
    let request = serde_json::to_value(&input).expect("move serializes");
    if reliability::replay::<Applied>(
        &mut tx,
        actor.workspace,
        &w.access,
        &operation,
        key,
        &request,
    )
    .await?
    .is_some()
    {
        return present(&mut tx, actor, w.read, id).await;
    }
    let current = editable(&mut tx, actor, &w, id, input.expected_version).await?;
    let target = dao::collection(&mut tx, actor.workspace, input.collection_id, w.read).await?;
    require_reach(&mut tx, actor.workspace, target.id, w.write).await?;
    if target.archived {
        return Err(Error::Conflict(
            "Restore that collection before moving into it",
        ));
    }
    if target.id == current.collection_id {
        return Err(Error::Invalid("The document is already in that collection"));
    }
    append(
        &mut tx,
        actor.workspace,
        &current,
        target.id,
        KnowledgeChange::Moved,
        current.kind.key(),
        &current.title,
        &current.body,
        None,
        w.access.membership_id,
    )
    .await?;
    reliability::audit(
        &mut tx,
        actor,
        &w.access,
        "knowledge.document_moved",
        id,
        serde_json::json!({"from":current.collection_id,"to":target.id}),
    )
    .await?;
    let result = present(&mut tx, actor, w.read, id).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &w.access,
        &operation,
        key,
        &request,
        &Applied { id },
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn archive_document(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: ArchiveKnowledgeRecord,
) -> Result<KnowledgeDocument, Error> {
    let mut tx = pool.begin().await?;
    let w = mutation(&mut tx, actor, EDIT).await?;
    let operation = format!("knowledge.documents.archive:{id}");
    let request = serde_json::to_value(&input).expect("archive serializes");
    if reliability::replay::<Applied>(
        &mut tx,
        actor.workspace,
        &w.access,
        &operation,
        key,
        &request,
    )
    .await?
    .is_some()
    {
        return present(&mut tx, actor, w.read, id).await;
    }
    let current = dao::document(&mut tx, actor.workspace, id, w.read).await?;
    require_reach(&mut tx, actor.workspace, current.collection_id, w.write).await?;
    if current.version != input.expected_version {
        return Err(Error::Conflict("Document changed; reload before editing"));
    }
    if current.collection_archived {
        return Err(Error::Conflict("Restore its collection first"));
    }
    dao::archive_document(&mut tx, actor.workspace, id, input.archived).await?;
    reliability::audit(
        &mut tx,
        actor,
        &w.access,
        "knowledge.document_archived",
        id,
        serde_json::json!({"archived":input.archived}),
    )
    .await?;
    let result = present(&mut tx, actor, w.read, id).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &w.access,
        &operation,
        key,
        &request,
        &Applied { id },
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
/// Newest first. A revision written in a collection this reader cannot reach is absent,
/// so its number may leave a gap; that it existed is all a gap can say.
pub async fn revisions(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    document: Uuid,
    query: KnowledgePageQuery,
) -> Result<KnowledgeRevisionPage, Error> {
    let limit = page_limit(query.limit, 50, 100)?;
    let mut tx = pool.begin().await?;
    let (_, read) = reach(&mut tx, actor, READ).await?;
    dao::document(&mut tx, actor.workspace, document, read).await?;
    if let Some(after) = query.after {
        dao::revision(&mut tx, actor.workspace, document, after, read).await?;
    }
    let mut items = dao::revisions(
        &mut tx,
        actor.workspace,
        document,
        query.after,
        read,
        limit + 1,
    )
    .await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    Ok(KnowledgeRevisionPage {
        items: items.into_iter().map(present_revision).collect(),
        next_cursor,
    })
}
pub async fn revision(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    document: Uuid,
    id: Uuid,
) -> Result<KnowledgeRevision, Error> {
    let mut tx = pool.begin().await?;
    let (_, read) = reach(&mut tx, actor, READ).await?;
    dao::document(&mut tx, actor.workspace, document, read).await?;
    Ok(present_revision(
        dao::revision(&mut tx, actor.workspace, document, id, read).await?,
    ))
}
/// Restoring copies a revision the writer may read into a new revision. History is
/// never rewritten, and a revision they may not read answers as if it did not exist.
pub async fn restore_revision(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    document: Uuid,
    id: Uuid,
    key: &str,
    input: RestoreKnowledgeRevision,
) -> Result<KnowledgeDocument, Error> {
    let mut tx = pool.begin().await?;
    let w = mutation(&mut tx, actor, EDIT).await?;
    let operation = format!("knowledge.documents.restore:{document}:{id}");
    let request = serde_json::to_value(&input).expect("restore serializes");
    if reliability::replay::<Applied>(
        &mut tx,
        actor.workspace,
        &w.access,
        &operation,
        key,
        &request,
    )
    .await?
    .is_some()
    {
        return present(&mut tx, actor, w.read, document).await;
    }
    dao::document(&mut tx, actor.workspace, document, w.read).await?;
    let source = dao::revision(&mut tx, actor.workspace, document, id, w.read).await?;
    let current = editable(&mut tx, actor, &w, document, input.expected_version).await?;
    append(
        &mut tx,
        actor.workspace,
        &current,
        current.collection_id,
        KnowledgeChange::Restored,
        source.kind.key(),
        &source.title,
        &source.body,
        Some(source.number),
        w.access.membership_id,
    )
    .await?;
    reliability::audit(
        &mut tx,
        actor,
        &w.access,
        "knowledge.revision_restored",
        document,
        serde_json::json!({"from_number":source.number,"revision":current.revision_number+1}),
    )
    .await?;
    let result = present(&mut tx, actor, w.read, document).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &w.access,
        &operation,
        key,
        &request,
        &Applied { id: document },
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn search(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: KnowledgeSearchQuery,
) -> Result<KnowledgeSearchPage, Error> {
    let limit = page_limit(query.limit, 20, 50)?;
    validate_query(&query.q)?;
    let cursor = query.after.as_deref().map(decode_cursor).transpose()?;
    let mut tx = pool.begin().await?;
    let (_, read) = reach(&mut tx, actor, READ).await?;
    let total = dao::search_total(&mut tx, actor.workspace, &query, read).await?;
    let mut rows = dao::search(&mut tx, actor.workspace, &query, cursor, read, limit + 1).await?;
    let next_cursor = if rows.len() > limit as usize {
        rows.pop();
        rows.last()
            .map(|(hit, rank, _)| encode_cursor(*rank, hit.id))
    } else {
        None
    };
    Ok(KnowledgeSearchPage {
        items: rows
            .into_iter()
            .map(|(mut hit, _, headline)| {
                hit.snippet = snippet_parts(&headline);
                hit
            })
            .collect(),
        total,
        next_cursor,
    })
}
/// Titles matching what has been typed so far, for search boxes and link pickers.
pub async fn suggest(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: KnowledgeSuggestQuery,
) -> Result<KnowledgeSuggestions, Error> {
    let limit = page_limit(query.limit, 8, 20)?;
    validate_query(&query.q)?;
    let mut tx = pool.begin().await?;
    let (_, read) = reach(&mut tx, actor, READ).await?;
    Ok(KnowledgeSuggestions {
        items: dao::suggest(&mut tx, actor.workspace, &query.q, read, limit).await?,
    })
}
/// The workspace's recent changes, newest first, as far as this reader may see them.
pub async fn activity(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: KnowledgePageQuery,
) -> Result<KnowledgeActivityPage, Error> {
    let limit = page_limit(query.limit, 20, 100)?;
    let mut tx = pool.begin().await?;
    let (_, read) = reach(&mut tx, actor, READ).await?;
    if let Some(after) = query.after
        && !dao::activity_visible(&mut tx, actor.workspace, after, read).await?
    {
        return Err(Error::NotFound);
    }
    let mut items = dao::activity(&mut tx, actor.workspace, query.after, read, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|a| a.revision_id)
    } else {
        None
    };
    Ok(KnowledgeActivityPage { items, next_cursor })
}
/// Business export projection of a collection's member list.
pub async fn export_members(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    collection: Uuid,
    query: KnowledgePageQuery,
) -> Result<serde_json::Value, Error> {
    let limit = page_limit(query.limit, 25, 100)? as usize;
    let mut items = members(pool, actor, collection).await?;
    items.sort_by_key(|m| m.id);
    if let Some(after) = query.after {
        if !items.iter().any(|m| m.id == after) {
            return Err(Error::NotFound);
        }
        items.retain(|m| m.id > after);
    }
    let more = items.len() > limit;
    items.truncate(limit);
    let next_cursor = more.then(|| items.last().map(|m| m.id)).flatten();
    Ok(serde_json::json!({"items":items,"next_cursor":next_cursor}))
}
/// Business export projection of documents: complete source, blocks and linked previews
/// as this exporter may see them, in collections they reach.
pub async fn export_documents(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: ListKnowledgeDocuments,
) -> Result<serde_json::Value, Error> {
    let page = list_documents(pool, actor, query).await?;
    let mut tx = pool.begin().await?;
    let (_, read) = reach(&mut tx, actor, READ).await?;
    let mut items = Vec::with_capacity(page.items.len());
    for item in page.items {
        match present(&mut tx, actor, read, item.id).await {
            Ok(document) => items.push(document),
            Err(Error::NotFound) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(serde_json::json!({"items":items,"next_cursor":page.next_cursor}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snippets_are_typed_parts() {
        let parts = snippet_parts("the \u{1}<img onerror=x>\u{2} and \u{1}word\u{2}");
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[1].text, "<img onerror=x>");
        assert!(parts[1].matched && !parts[2].matched && parts[3].matched);
        assert_eq!(snippet_parts(""), vec![]);
    }
    #[test]
    fn cursors_round_trip_and_refuse_forgeries() {
        let id = Uuid::new_v4();
        for rank in [0.0_f32, 0.0607927, 0.6079271, 1.0e-7] {
            assert_eq!(
                decode_cursor(&encode_cursor(rank, id)).ok(),
                Some((rank, id))
            );
        }
        assert!(decode_cursor("not a cursor").is_err());
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
        assert!(decode_cursor(&URL_SAFE_NO_PAD.encode(format!("NaN:{id}"))).is_err());
        assert!(decode_cursor(&URL_SAFE_NO_PAD.encode("0.5:1 OR 1=1")).is_err());
    }
    #[test]
    fn only_same_site_workspace_document_links_count() {
        let ws = Uuid::new_v4();
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let other = Uuid::new_v4();
        let source = format!(
            "[a](/workspaces/{ws}/knowledge/documents/{a}) [again](/workspaces/{ws}/knowledge/documents/{a}#top)\n\n\
             [self](/workspaces/{ws}/knowledge/documents/{b})\n\n\
             [foreign](/workspaces/{other}/knowledge/documents/{a}) [off](https://e.example/workspaces/{ws}/knowledge/documents/{a})\n\n\
             [bad](/workspaces/{ws}/knowledge/documents/nope)"
        );
        assert_eq!(document_links(&rich_text::parse(&source), ws, b), vec![a]);
    }
    #[test]
    fn document_text_cannot_hold_the_match_markers() {
        assert!(validate_document("Title", "a\u{1}b").is_err());
        assert!(validate_document("Ti\u{2}tle", "ab").is_err());
        assert!(validate_document("Title", "line\n\tnext").is_ok());
        assert!(validate_document("Title", "[x](javascript:alert(1))").is_err());
    }
}
