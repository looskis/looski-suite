// SPDX-License-Identifier: AGPL-3.0-only
//! CMS owns content types, entry drafts, immutable revisions and media. A content
//! type's public key reaches only entries published under that type and the media
//! those published revisions name; drafts, old revisions and private Files stay out
//! of reach. Published text is rendered from typed structures, never stored HTML.
pub mod content;
mod dao;
use crate::rich_text::RichBlock;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    files, forms, modules, reliability, rich_text,
};
pub use content::{ContentField, ContentFieldKind};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::{PgConnection, PgPool};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ContentType {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub fields: Vec<ContentField>,
    /// Public read rights for this type's published entries only.
    pub public_key: String,
    pub archived: bool,
    pub published_entries: i64,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveContentType {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub fields: Vec<ContentField>,
    /// Required for updates; omitted on creation.
    pub expected_version: Option<i64>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct ListContentTypes {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
    pub search: Option<String>,
    pub archived: Option<bool>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ContentTypePage {
    pub items: Vec<ContentType>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ArchiveContent {
    pub expected_version: i64,
    pub archived: bool,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExpectedVersion {
    pub expected_version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct EntryDraft {
    pub title: String,
    pub slug: String,
    pub values: Map<String, Value>,
    pub revision_number: i32,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct PublishedEntry {
    pub revision_id: Uuid,
    pub revision_number: i32,
    pub title: String,
    pub slug: String,
    pub values: Map<String, Value>,
    pub published_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ContentEntry {
    pub id: Uuid,
    pub type_id: Uuid,
    pub type_name: String,
    /// Absent for members who cannot read drafts.
    pub draft: Option<EntryDraft>,
    pub published: Option<PublishedEntry>,
    /// The draft differs from what is published.
    pub unpublished_changes: bool,
    pub public_key: String,
    pub type_archived: bool,
    pub archived: bool,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct EntrySummary {
    pub id: Uuid,
    pub type_id: Uuid,
    pub type_name: String,
    pub title: String,
    pub slug: String,
    pub published_revision_number: Option<i32>,
    pub published_slug: Option<String>,
    pub unpublished_changes: bool,
    pub public_key: String,
    pub archived: bool,
    pub version: i64,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateEntry {
    pub type_id: Uuid,
    pub title: String,
    pub slug: String,
    #[serde(default)]
    pub values: Map<String, Value>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveEntry {
    pub expected_version: i64,
    pub title: String,
    pub slug: String,
    #[serde(default)]
    pub values: Map<String, Value>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct ListEntries {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
    pub search: Option<String>,
    pub archived: Option<bool>,
    pub type_id: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct EntryPage {
    pub items: Vec<EntrySummary>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ContentRevision {
    pub id: Uuid,
    pub entry_id: Uuid,
    pub number: i32,
    pub title: String,
    pub slug: String,
    pub values: Map<String, Value>,
    /// The content type's fields when this revision was made.
    pub fields: Vec<ContentField>,
    pub restored_from_number: Option<i32>,
    pub created_at: String,
    pub last_published_at: Option<String>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct RevisionQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RevisionPage {
    pub items: Vec<ContentRevision>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RestoredEntry {
    pub entry: ContentEntry,
    /// Values left out because their field no longer exists with the same kind.
    /// They remain readable in the revision.
    pub dropped_fields: Vec<String>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ContentMedia {
    pub id: Uuid,
    pub file_id: Uuid,
    pub title: String,
    pub alt_text: String,
    pub media_type: String,
    pub filename: String,
    pub byte_length: i64,
    pub archived: bool,
    /// Archiving the file in Files also withdraws it from public pages.
    pub file_archived: bool,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RegisterMedia {
    pub file_id: Uuid,
    pub title: String,
    #[serde(default)]
    pub alt_text: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveMedia {
    pub expected_version: i64,
    pub title: String,
    #[serde(default)]
    pub alt_text: String,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct ListMedia {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
    pub search: Option<String>,
    pub archived: Option<bool>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct MediaPage {
    pub items: Vec<ContentMedia>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct MediaContent {
    pub media: ContentMedia,
    pub content_base64: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct PublicEntryLink {
    pub title: String,
    pub slug: String,
    pub published_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct PublicIndex {
    pub name: String,
    pub description: String,
    pub entries: Vec<PublicEntryLink>,
    pub next_cursor: Option<String>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct PublicIndexQuery {
    pub after: Option<String>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PublicValue {
    Text {
        text: String,
    },
    RichText {
        blocks: Vec<RichBlock>,
    },
    Number {
        number: f64,
    },
    Boolean {
        checked: bool,
    },
    Date {
        date: String,
    },
    Url {
        href: String,
    },
    Media {
        id: Uuid,
        title: String,
        alt_text: String,
        media_type: String,
    },
    /// `public_key` is absent while the form cannot be answered, including while
    /// the Forms application is disabled; the page explains rather than breaks.
    Form {
        public_key: Option<String>,
    },
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct PublicField {
    pub key: String,
    pub label: String,
    pub value: PublicValue,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct PublicPage {
    pub title: String,
    pub slug: String,
    pub published_at: Option<String>,
    pub fields: Vec<PublicField>,
}
pub struct PublicMediaBytes {
    pub media_type: String,
    pub bytes: Vec<u8>,
}
/// Everything stored about an entry; `view` decides what a reader receives.
#[derive(Serialize, Deserialize)]
struct StoredEntry {
    id: Uuid,
    type_id: Uuid,
    type_name: String,
    title: String,
    slug: String,
    values: Map<String, Value>,
    head_number: i32,
    published: Option<PublishedEntry>,
    public_key: String,
    type_archived: bool,
    archived: bool,
    version: i64,
    created_at: String,
    updated_at: String,
}
struct Draft {
    title: String,
    slug: String,
    values: Map<String, Value>,
}
fn view(entry: StoredEntry, drafts: bool) -> ContentEntry {
    let unpublished_changes = entry
        .published
        .as_ref()
        .is_none_or(|p| p.revision_number != entry.head_number);
    ContentEntry {
        id: entry.id,
        type_id: entry.type_id,
        type_name: entry.type_name,
        draft: drafts.then_some(EntryDraft {
            title: entry.title,
            slug: entry.slug,
            values: entry.values,
            revision_number: entry.head_number,
        }),
        published: entry.published,
        unpublished_changes: drafts && unpublished_changes,
        public_key: entry.public_key,
        type_archived: entry.type_archived,
        archived: entry.archived,
        version: entry.version,
        created_at: entry.created_at,
        updated_at: entry.updated_at,
    }
}

async fn authorize(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    action: &str,
) -> Result<access::Access, Error> {
    let a = access::require(db, actor.identity, actor.workspace, action).await?;
    if !a.workspace_wide {
        return Err(Error::Forbidden);
    }
    modules::require(db, actor.workspace, "cms").await?;
    Ok(a)
}
async fn may_read_drafts(db: &mut PgConnection, actor: &ActorContext<'_>) -> Result<bool, Error> {
    match authorize(db, actor, "cms.drafts.read").await {
        Ok(_) => Ok(true),
        Err(Error::Forbidden) => Ok(false),
        Err(error) => Err(error),
    }
}
/// Check write authority, serialize with application selection, then re-check.
async fn mutation(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    action: &str,
) -> Result<access::Access, Error> {
    authorize(db, actor, action).await?;
    modules::lock_workspace(db, actor.workspace).await?;
    authorize(db, actor, "cms.read").await?;
    authorize(db, actor, action).await
}
/// Editing and publishing both act on words, so both require reading them.
async fn entry_mutation(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    action: &str,
) -> Result<access::Access, Error> {
    let write = mutation(db, actor, action).await?;
    authorize(db, actor, "cms.drafts.read").await?;
    Ok(write)
}
fn page_limit(limit: Option<i64>) -> Result<i64, Error> {
    let limit = limit.unwrap_or(50);
    if !(1..=100).contains(&limit) {
        return Err(Error::Invalid("Limit must be 1–100"));
    }
    Ok(limit)
}
fn search_limit(search: &Option<String>) -> Result<(), Error> {
    if search
        .as_ref()
        .is_some_and(|s| s.len() > 200 || s.contains('\0'))
    {
        return Err(Error::Invalid(
            "Search must be at most 200 bytes of printable text",
        ));
    }
    Ok(())
}
fn validate_type(input: &SaveContentType) -> Result<(), Error> {
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
    content::validate_fields(&input.fields)
}
fn validate_media_text(title: &str, alt_text: &str) -> Result<(), Error> {
    if title.trim().is_empty()
        || title.chars().count() > 200
        || title.chars().any(char::is_control)
        || alt_text.chars().count() > 1000
        || alt_text.chars().any(char::is_control)
    {
        return Err(Error::Invalid(
            "Titles must be 1–200 and descriptions at most 1000 printable characters",
        ));
    }
    Ok(())
}
fn new_public_key() -> Result<String, Error> {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| Error::Unavailable)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

pub async fn list_types(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: ListContentTypes,
) -> Result<ContentTypePage, Error> {
    let limit = page_limit(query.limit)?;
    search_limit(&query.search)?;
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "cms.read").await?;
    if let Some(after) = query.after {
        dao::content_type(&mut tx, actor.workspace, after, false).await?;
    }
    let mut items = dao::content_types(&mut tx, actor.workspace, &query, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|t| t.id)
    } else {
        None
    };
    Ok(ContentTypePage { items, next_cursor })
}
pub async fn get_type(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<ContentType, Error> {
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "cms.read").await?;
    dao::content_type(&mut tx, actor.workspace, id, false).await
}
pub async fn save_type(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Option<Uuid>,
    key: &str,
    input: SaveContentType,
) -> Result<ContentType, Error> {
    validate_type(&input)?;
    if id.is_some() != input.expected_version.is_some() {
        return Err(Error::Invalid(
            "Updates require expected_version; creation must omit it",
        ));
    }
    let mut tx = pool.begin().await?;
    let write = mutation(&mut tx, actor, "cms.types.manage").await?;
    let operation = id
        .map(|id| format!("cms.types.update:{id}"))
        .unwrap_or_else(|| "cms.types.create".into());
    let request = serde_json::to_value(&input).expect("content type serializes");
    if let Some(saved) = reliability::replay::<ContentType>(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
    )
    .await?
    {
        dao::content_type(&mut tx, actor.workspace, saved.id, false).await?;
        return Ok(saved);
    }
    let id = if let Some(id) = id {
        let current = dao::content_type(&mut tx, actor.workspace, id, true).await?;
        if Some(current.version) != input.expected_version || current.archived {
            return Err(Error::Conflict(
                "Content type changed or is archived; reload before editing",
            ));
        }
        if !content::kinds_unchanged(&current.fields, &input.fields) {
            return Err(Error::Invalid(
                "A field keeps its kind; add a new field instead of changing one",
            ));
        }
        dao::update_type(&mut tx, actor.workspace, id, &input).await?;
        id
    } else {
        dao::insert_type(&mut tx, actor.workspace, &input, &new_public_key()?).await?
    };
    reliability::audit(
        &mut tx,
        actor,
        &write,
        "cms.type_saved",
        id,
        serde_json::json!({}),
    )
    .await?;
    let result = dao::content_type(&mut tx, actor.workspace, id, false).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
/// Archiving a type takes its public pages offline, so with published entries it
/// also needs publishing authority. Restoring brings those entries back.
pub async fn archive_type(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: ArchiveContent,
) -> Result<ContentType, Error> {
    let mut tx = pool.begin().await?;
    let write = mutation(&mut tx, actor, "cms.types.manage").await?;
    let operation = format!("cms.types.archive:{id}");
    let request = serde_json::to_value(&input).expect("archive serializes");
    if let Some(saved) = reliability::replay::<ContentType>(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
    )
    .await?
    {
        dao::content_type(&mut tx, actor.workspace, id, false).await?;
        return Ok(saved);
    }
    let current = dao::content_type(&mut tx, actor.workspace, id, true).await?;
    if current.version != input.expected_version {
        return Err(Error::Conflict(
            "Content type changed; reload before editing",
        ));
    }
    if current.published_entries > 0 {
        authorize(&mut tx, actor, "cms.entries.publish").await?;
    }
    dao::archive_type(&mut tx, actor.workspace, id, input.archived).await?;
    reliability::audit(
        &mut tx,
        actor,
        &write,
        "cms.type_archived",
        id,
        serde_json::json!({"archived":input.archived}),
    )
    .await?;
    let result = dao::content_type(&mut tx, actor.workspace, id, false).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn list_entries(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: ListEntries,
) -> Result<EntryPage, Error> {
    let limit = page_limit(query.limit)?;
    search_limit(&query.search)?;
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "cms.read").await?;
    let drafts = may_read_drafts(&mut tx, actor).await?;
    if let Some(after) = query.after {
        visible(&mut tx, actor, after, drafts).await?;
    }
    let mut items = dao::summaries(&mut tx, actor.workspace, &query, drafts, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|e| e.id)
    } else {
        None
    };
    Ok(EntryPage { items, next_cursor })
}
/// An entry as this reader may see it; an unpublished entry does not exist to
/// members who cannot read drafts.
async fn visible(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Uuid,
    drafts: bool,
) -> Result<ContentEntry, Error> {
    let entry = dao::entry(db, actor.workspace, id, false).await?;
    if !drafts && entry.published.is_none() {
        return Err(Error::NotFound);
    }
    Ok(view(entry, drafts))
}
pub async fn get_entry(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<ContentEntry, Error> {
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "cms.read").await?;
    let drafts = may_read_drafts(&mut tx, actor).await?;
    visible(&mut tx, actor, id, drafts).await
}
/// Newly chosen references need current authority; unchanged ones are kept, so an
/// entry stays editable while Forms is disabled or a form is closed.
async fn check_references(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    fields: &[ContentField],
    values: &Map<String, Value>,
    previous: Option<&Map<String, Value>>,
) -> Result<(), Error> {
    for field in fields {
        let Some(value) = values.get(&field.key) else {
            continue;
        };
        if previous.and_then(|p| p.get(&field.key)) == Some(value) {
            continue;
        }
        let Some(id) = content::reference(value) else {
            continue;
        };
        match field.kind {
            ContentFieldKind::Form if !forms::can_reference(db, actor, id).await? => {
                return Err(Error::Invalid(
                    "Choose a form you can read in this workspace, with Forms enabled",
                ));
            }
            ContentFieldKind::Media
                if dao::known_media(db, actor.workspace, &[id], true).await? != 1 =>
            {
                return Err(Error::Invalid(
                    "Choose active media from this workspace's library",
                ));
            }
            _ => {}
        }
    }
    Ok(())
}
pub async fn create_entry(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    key: &str,
    input: CreateEntry,
) -> Result<ContentEntry, Error> {
    content::validate_title(&input.title)?;
    content::validate_slug(&input.slug)?;
    let mut tx = pool.begin().await?;
    let write = entry_mutation(&mut tx, actor, "cms.entries.edit").await?;
    let operation = "cms.entries.create";
    let request = serde_json::to_value(&input).expect("entry serializes");
    if let Some(saved) = reliability::replay::<ContentEntry>(
        &mut tx,
        actor.workspace,
        &write,
        operation,
        key,
        &request,
    )
    .await?
    {
        dao::entry(&mut tx, actor.workspace, saved.id, false).await?;
        return Ok(saved);
    }
    let content_type = dao::content_type(&mut tx, actor.workspace, input.type_id, false).await?;
    if content_type.archived {
        return Err(Error::Conflict("Restore the content type first"));
    }
    let values = content::validate_values(&content_type.fields, &input.values)?;
    check_references(&mut tx, actor, &content_type.fields, &values, None).await?;
    let draft = Draft {
        title: input.title.trim().to_string(),
        slug: input.slug.clone(),
        values,
    };
    let id = dao::insert_entry(&mut tx, actor.workspace, input.type_id, &draft).await?;
    dao::insert_revision(
        &mut tx,
        actor.workspace,
        id,
        &draft,
        &content_type.fields,
        &content::media_ids(&content_type.fields, &draft.values),
        None,
        write.membership_id,
    )
    .await?;
    finish_entry(
        &mut tx,
        actor,
        &write,
        id,
        "cms.entry_saved",
        operation,
        key,
        &request,
    )
    .await?;
    let result = view(dao::entry(&mut tx, actor.workspace, id, false).await?, true);
    tx.commit().await?;
    Ok(result)
}
/// Audit the change and record the retry result in one place.
#[allow(clippy::too_many_arguments)]
async fn finish_entry(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    write: &access::Access,
    id: Uuid,
    action: &str,
    operation: &str,
    key: &str,
    request: &Value,
) -> Result<(), Error> {
    reliability::audit(db, actor, write, action, id, serde_json::json!({})).await?;
    let result = view(dao::entry(db, actor.workspace, id, false).await?, true);
    reliability::save_replay(db, actor.workspace, write, operation, key, request, &result).await
}
pub async fn update_entry(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: SaveEntry,
) -> Result<ContentEntry, Error> {
    content::validate_title(&input.title)?;
    content::validate_slug(&input.slug)?;
    let mut tx = pool.begin().await?;
    let write = entry_mutation(&mut tx, actor, "cms.entries.edit").await?;
    let operation = format!("cms.entries.update:{id}");
    let request = serde_json::to_value(&input).expect("entry serializes");
    if let Some(saved) = reliability::replay::<ContentEntry>(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
    )
    .await?
    {
        dao::entry(&mut tx, actor.workspace, id, false).await?;
        return Ok(saved);
    }
    let current = dao::entry(&mut tx, actor.workspace, id, true).await?;
    if current.version != input.expected_version || current.archived || current.type_archived {
        return Err(Error::Conflict(
            "Entry changed or is archived; reload before editing",
        ));
    }
    let content_type = dao::content_type(&mut tx, actor.workspace, current.type_id, false).await?;
    // A client echoing a value of a since-removed field is not asking to set it.
    let mut supplied = input.values.clone();
    supplied.retain(|field, value| {
        content_type.fields.iter().any(|f| &f.key == field)
            || current.values.get(field) != Some(value)
    });
    let values = content::validate_values(&content_type.fields, &supplied)?;
    check_references(
        &mut tx,
        actor,
        &content_type.fields,
        &values,
        Some(&current.values),
    )
    .await?;
    let draft = Draft {
        title: input.title.trim().to_string(),
        slug: input.slug.clone(),
        values,
    };
    // Saving nothing new adds no revision for people to wade through.
    if draft.title != current.title || draft.slug != current.slug || draft.values != current.values
    {
        dao::update_draft(&mut tx, actor.workspace, id, &draft).await?;
        dao::insert_revision(
            &mut tx,
            actor.workspace,
            id,
            &draft,
            &content_type.fields,
            &content::media_ids(&content_type.fields, &draft.values),
            None,
            write.membership_id,
        )
        .await?;
    }
    finish_entry(
        &mut tx,
        actor,
        &write,
        id,
        "cms.entry_saved",
        &operation,
        key,
        &request,
    )
    .await?;
    let result = view(dao::entry(&mut tx, actor.workspace, id, false).await?, true);
    tx.commit().await?;
    Ok(result)
}
/// Make the draft live. Validated against the current type; if the type has changed
/// since the last save a new revision records exactly what goes public.
pub async fn publish(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: ExpectedVersion,
) -> Result<ContentEntry, Error> {
    let mut tx = pool.begin().await?;
    let write = entry_mutation(&mut tx, actor, "cms.entries.publish").await?;
    let operation = format!("cms.entries.publish:{id}");
    let request = serde_json::to_value(&input).expect("publish serializes");
    if let Some(saved) = reliability::replay::<ContentEntry>(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
    )
    .await?
    {
        dao::entry(&mut tx, actor.workspace, id, false).await?;
        return Ok(saved);
    }
    let current = dao::entry(&mut tx, actor.workspace, id, true).await?;
    if current.version != input.expected_version {
        return Err(Error::Conflict("Entry changed; reload before publishing"));
    }
    if current.archived || current.type_archived {
        return Err(Error::Conflict(
            "Restore the entry and its content type before publishing",
        ));
    }
    let content_type = dao::content_type(&mut tx, actor.workspace, current.type_id, false).await?;
    let head = dao::head_revision(&mut tx, actor.workspace, id).await?;
    let (values, _) = content::retained(&head.fields, &content_type.fields, &current.values);
    let new_revision = head.fields != content_type.fields || values != current.values;
    if !new_revision
        && current
            .published
            .as_ref()
            .is_some_and(|p| p.revision_number == head.number)
    {
        return Err(Error::Conflict("The draft has no unpublished changes"));
    }
    content::require_complete(&content_type.fields, &values)?;
    let media = content::media_ids(&content_type.fields, &values);
    if dao::known_media(&mut tx, actor.workspace, &media, true).await? != media.len() as i64 {
        return Err(Error::Conflict("Replace archived media before publishing"));
    }
    if dao::address_taken(&mut tx, actor.workspace, current.type_id, &current.slug, id).await? {
        return Err(Error::Conflict(
            "Another published entry of this type uses this address",
        ));
    }
    let revision = if new_revision {
        let draft = Draft {
            title: current.title.clone(),
            slug: current.slug.clone(),
            values,
        };
        dao::update_draft(&mut tx, actor.workspace, id, &draft).await?;
        dao::insert_revision(
            &mut tx,
            actor.workspace,
            id,
            &draft,
            &content_type.fields,
            &media,
            None,
            write.membership_id,
        )
        .await?
        .0
    } else {
        head.id
    };
    dao::publish(&mut tx, actor.workspace, id, revision, &current.slug).await?;
    finish_entry(
        &mut tx,
        actor,
        &write,
        id,
        "cms.entry_published",
        &operation,
        key,
        &request,
    )
    .await?;
    let result = view(dao::entry(&mut tx, actor.workspace, id, false).await?, true);
    tx.commit().await?;
    Ok(result)
}
pub async fn unpublish(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: ExpectedVersion,
) -> Result<ContentEntry, Error> {
    let mut tx = pool.begin().await?;
    let write = entry_mutation(&mut tx, actor, "cms.entries.publish").await?;
    let operation = format!("cms.entries.unpublish:{id}");
    let request = serde_json::to_value(&input).expect("unpublish serializes");
    if let Some(saved) = reliability::replay::<ContentEntry>(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
    )
    .await?
    {
        dao::entry(&mut tx, actor.workspace, id, false).await?;
        return Ok(saved);
    }
    let current = dao::entry(&mut tx, actor.workspace, id, true).await?;
    if current.version != input.expected_version {
        return Err(Error::Conflict("Entry changed; reload before unpublishing"));
    }
    if current.published.is_none() {
        return Err(Error::Conflict("This entry is not published"));
    }
    dao::unpublish(&mut tx, actor.workspace, id).await?;
    finish_entry(
        &mut tx,
        actor,
        &write,
        id,
        "cms.entry_unpublished",
        &operation,
        key,
        &request,
    )
    .await?;
    let result = view(dao::entry(&mut tx, actor.workspace, id, false).await?, true);
    tx.commit().await?;
    Ok(result)
}
/// Archiving a published entry withdraws it, so that also needs publishing authority.
/// Restoring leaves the entry unpublished until someone publishes it again.
pub async fn archive_entry(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: ArchiveContent,
) -> Result<ContentEntry, Error> {
    let mut tx = pool.begin().await?;
    let write = entry_mutation(&mut tx, actor, "cms.entries.edit").await?;
    let operation = format!("cms.entries.archive:{id}");
    let request = serde_json::to_value(&input).expect("archive serializes");
    if let Some(saved) = reliability::replay::<ContentEntry>(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
    )
    .await?
    {
        dao::entry(&mut tx, actor.workspace, id, false).await?;
        return Ok(saved);
    }
    let current = dao::entry(&mut tx, actor.workspace, id, true).await?;
    if current.version != input.expected_version {
        return Err(Error::Conflict("Entry changed; reload before editing"));
    }
    if current.published.is_some() {
        authorize(&mut tx, actor, "cms.entries.publish").await?;
    }
    dao::archive_entry(&mut tx, actor.workspace, id, input.archived).await?;
    finish_entry(
        &mut tx,
        actor,
        &write,
        id,
        "cms.entry_archived",
        &operation,
        key,
        &request,
    )
    .await?;
    let result = view(dao::entry(&mut tx, actor.workspace, id, false).await?, true);
    tx.commit().await?;
    Ok(result)
}
pub async fn revisions(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    query: RevisionQuery,
) -> Result<RevisionPage, Error> {
    let limit = page_limit(query.limit)?;
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "cms.read").await?;
    authorize(&mut tx, actor, "cms.drafts.read").await?;
    dao::entry(&mut tx, actor.workspace, id, false).await?;
    if let Some(after) = query.after {
        dao::revision(&mut tx, actor.workspace, id, after).await?;
    }
    let mut items = dao::revisions(&mut tx, actor.workspace, id, query.after, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    Ok(RevisionPage { items, next_cursor })
}
pub async fn revision(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    revision: Uuid,
) -> Result<ContentRevision, Error> {
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "cms.read").await?;
    authorize(&mut tx, actor, "cms.drafts.read").await?;
    dao::revision(&mut tx, actor.workspace, id, revision).await
}
/// Copy an earlier revision into the draft as a new revision. History is never
/// rewritten and what is live changes only when someone publishes the result.
pub async fn restore_revision(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    revision: Uuid,
    key: &str,
    input: ExpectedVersion,
) -> Result<RestoredEntry, Error> {
    let mut tx = pool.begin().await?;
    let write = entry_mutation(&mut tx, actor, "cms.entries.edit").await?;
    let operation = format!("cms.entries.restore:{id}:{revision}");
    let request = serde_json::to_value(&input).expect("restore serializes");
    if let Some(saved) = reliability::replay::<RestoredEntry>(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
    )
    .await?
    {
        dao::entry(&mut tx, actor.workspace, id, false).await?;
        return Ok(saved);
    }
    let current = dao::entry(&mut tx, actor.workspace, id, true).await?;
    if current.version != input.expected_version || current.archived || current.type_archived {
        return Err(Error::Conflict(
            "Entry changed or is archived; reload before restoring",
        ));
    }
    let source = dao::revision(&mut tx, actor.workspace, id, revision).await?;
    let content_type = dao::content_type(&mut tx, actor.workspace, current.type_id, false).await?;
    let (values, dropped_fields) =
        content::retained(&source.fields, &content_type.fields, &source.values);
    let draft = Draft {
        title: source.title.clone(),
        slug: source.slug.clone(),
        values,
    };
    if draft.title == current.title && draft.slug == current.slug && draft.values == current.values
    {
        return Err(Error::Conflict("The draft already matches this revision"));
    }
    dao::update_draft(&mut tx, actor.workspace, id, &draft).await?;
    dao::insert_revision(
        &mut tx,
        actor.workspace,
        id,
        &draft,
        &content_type.fields,
        &content::media_ids(&content_type.fields, &draft.values),
        Some(source.number),
        write.membership_id,
    )
    .await?;
    reliability::audit(
        &mut tx,
        actor,
        &write,
        "cms.revision_restored",
        id,
        serde_json::json!({"revision":source.number,"dropped_fields":dropped_fields}),
    )
    .await?;
    let result = RestoredEntry {
        entry: view(dao::entry(&mut tx, actor.workspace, id, false).await?, true),
        dropped_fields,
    };
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn list_media(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: ListMedia,
) -> Result<MediaPage, Error> {
    let limit = page_limit(query.limit)?;
    search_limit(&query.search)?;
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "cms.read").await?;
    if let Some(after) = query.after {
        dao::media(&mut tx, actor.workspace, after, false).await?;
    }
    let mut items = dao::media_page(&mut tx, actor.workspace, &query, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|m| m.id)
    } else {
        None
    };
    Ok(MediaPage { items, next_cursor })
}
pub async fn get_media(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<ContentMedia, Error> {
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "cms.read").await?;
    dao::media(&mut tx, actor.workspace, id, false).await
}
/// Staff read of media bytes, including media no published page uses yet.
pub async fn media_content(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<MediaContent, Error> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "cms.read").await?;
    let media = dao::media(&mut tx, actor.workspace, id, false).await?;
    let bytes = files::active_bytes(&mut tx, actor.workspace, media.file_id).await?;
    Ok(MediaContent {
        media,
        content_base64: STANDARD.encode(bytes),
    })
}
/// Adopt a file the actor can already read. Registering lets members with `cms.read`
/// see it and lets published entries serve it; the file itself stays private in Files.
pub async fn register_media(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    key: &str,
    input: RegisterMedia,
) -> Result<ContentMedia, Error> {
    validate_media_text(&input.title, &input.alt_text)?;
    let mut tx = pool.begin().await?;
    let write = mutation(&mut tx, actor, "cms.media.manage").await?;
    let file = files::readable(&mut tx, actor, input.file_id).await?;
    let operation = "cms.media.register";
    let request = serde_json::to_value(&input).expect("media serializes");
    if let Some(saved) = reliability::replay::<ContentMedia>(
        &mut tx,
        actor.workspace,
        &write,
        operation,
        key,
        &request,
    )
    .await?
    {
        dao::media(&mut tx, actor.workspace, saved.id, false).await?;
        return Ok(saved);
    }
    if file.archived {
        return Err(Error::Conflict("Restore the file before using it as media"));
    }
    if dao::media_for_file(&mut tx, actor.workspace, input.file_id).await? {
        return Err(Error::Conflict("This file is already in the media library"));
    }
    let bytes = files::active_bytes(&mut tx, actor.workspace, input.file_id).await?;
    if !content::media_type_matches(&file.media_type, &bytes) {
        return Err(Error::Invalid(
            "Media must be a PNG, JPEG, GIF, WebP or PDF file whose content matches its type",
        ));
    }
    let id = dao::insert_media(
        &mut tx,
        actor.workspace,
        input.file_id,
        &input.title,
        &input.alt_text,
        &file.media_type,
    )
    .await?;
    reliability::audit(
        &mut tx,
        actor,
        &write,
        "cms.media_registered",
        id,
        serde_json::json!({"file_id":input.file_id}),
    )
    .await?;
    let result = dao::media(&mut tx, actor.workspace, id, false).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &write,
        operation,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn update_media(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: SaveMedia,
) -> Result<ContentMedia, Error> {
    validate_media_text(&input.title, &input.alt_text)?;
    let mut tx = pool.begin().await?;
    let write = mutation(&mut tx, actor, "cms.media.manage").await?;
    let operation = format!("cms.media.update:{id}");
    let request = serde_json::to_value(&input).expect("media serializes");
    if let Some(saved) = reliability::replay::<ContentMedia>(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
    )
    .await?
    {
        dao::media(&mut tx, actor.workspace, id, false).await?;
        return Ok(saved);
    }
    let current = dao::media(&mut tx, actor.workspace, id, true).await?;
    if current.version != input.expected_version {
        return Err(Error::Conflict("Media changed; reload before editing"));
    }
    dao::update_media(&mut tx, actor.workspace, id, &input).await?;
    reliability::audit(
        &mut tx,
        actor,
        &write,
        "cms.media_updated",
        id,
        serde_json::json!({}),
    )
    .await?;
    let result = dao::media(&mut tx, actor.workspace, id, false).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
/// Archiving withdraws the bytes from public pages at once; pages that use the
/// media render without it and regain it when the media is restored.
pub async fn archive_media(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: ArchiveContent,
) -> Result<ContentMedia, Error> {
    let mut tx = pool.begin().await?;
    let write = mutation(&mut tx, actor, "cms.media.manage").await?;
    let operation = format!("cms.media.archive:{id}");
    let request = serde_json::to_value(&input).expect("media serializes");
    if let Some(saved) = reliability::replay::<ContentMedia>(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
    )
    .await?
    {
        dao::media(&mut tx, actor.workspace, id, false).await?;
        return Ok(saved);
    }
    let current = dao::media(&mut tx, actor.workspace, id, true).await?;
    if current.version != input.expected_version {
        return Err(Error::Conflict("Media changed; reload before editing"));
    }
    dao::archive_media(&mut tx, actor.workspace, id, input.archived).await?;
    reliability::audit(
        &mut tx,
        actor,
        &write,
        "cms.media_archived",
        id,
        serde_json::json!({"archived":input.archived}),
    )
    .await?;
    let result = dao::media(&mut tx, actor.workspace, id, false).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
/// Turn stored values into typed public structures, in field order. Empty values,
/// withdrawn media and references to other workspaces produce nothing readable.
async fn render(
    db: &mut PgConnection,
    workspace: Uuid,
    fields: &[ContentField],
    values: &Map<String, Value>,
) -> Result<Vec<PublicField>, Error> {
    let mut out = Vec::new();
    for field in fields {
        let Some(value) = values.get(&field.key) else {
            continue;
        };
        let text = || value.as_str().map(str::to_string);
        let rendered = match field.kind {
            ContentFieldKind::Text => text().map(|text| PublicValue::Text { text }),
            ContentFieldKind::RichText => value.as_str().map(|source| PublicValue::RichText {
                blocks: rich_text::parse(source),
            }),
            ContentFieldKind::Number => value.as_f64().map(|number| PublicValue::Number { number }),
            ContentFieldKind::Boolean => value
                .as_bool()
                .map(|checked| PublicValue::Boolean { checked }),
            ContentFieldKind::Date => text().map(|date| PublicValue::Date { date }),
            ContentFieldKind::Url => text()
                .filter(|href| rich_text::safe_href(href))
                .map(|href| PublicValue::Url { href }),
            ContentFieldKind::Media => match content::reference(value) {
                Some(id) => dao::visible_media(db, workspace, id).await?.map(
                    |(title, alt_text, media_type)| PublicValue::Media {
                        id,
                        title,
                        alt_text,
                        media_type,
                    },
                ),
                None => None,
            },
            ContentFieldKind::Form => match content::reference(value) {
                Some(id) => Some(PublicValue::Form {
                    public_key: forms::embeddable(db, workspace, id).await?,
                }),
                None => None,
            },
        };
        if let Some(value) = rendered {
            out.push(PublicField {
                key: field.key.clone(),
                label: field.label.clone(),
                value,
            });
        }
    }
    Ok(out)
}
/// The draft through the same renderer as the public page, for staff only.
pub async fn preview(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<PublicPage, Error> {
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "cms.read").await?;
    authorize(&mut tx, actor, "cms.drafts.read").await?;
    let entry = dao::entry(&mut tx, actor.workspace, id, false).await?;
    let content_type = dao::content_type(&mut tx, actor.workspace, entry.type_id, false).await?;
    let fields = render(
        &mut tx,
        actor.workspace,
        &content_type.fields,
        &entry.values,
    )
    .await?;
    Ok(PublicPage {
        title: entry.title,
        slug: entry.slug,
        published_at: None,
        fields,
    })
}
/// Business export projection: complete entries, including drafts.
pub async fn export_entries(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: ListEntries,
) -> Result<Value, Error> {
    let limit = page_limit(query.limit)?;
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "cms.read").await?;
    authorize(&mut tx, actor, "cms.drafts.read").await?;
    if let Some(after) = query.after {
        dao::entry(&mut tx, actor.workspace, after, false).await?;
    }
    let mut entries = dao::entries(&mut tx, actor.workspace, &query, limit + 1).await?;
    let next_cursor = if entries.len() > limit as usize {
        entries.pop();
        entries.last().map(|e| e.id)
    } else {
        None
    };
    let items: Vec<ContentEntry> = entries.into_iter().map(|e| view(e, true)).collect();
    Ok(serde_json::json!({"items":items,"next_cursor":next_cursor}))
}
/// Resolve a public key to its content type while CMS is enabled for the workspace.
async fn public_identity(db: &mut PgConnection, key: &str) -> Result<dao::PublicType, Error> {
    if key.len() != 22
        || !key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(Error::NotFound);
    }
    let identity = dao::public_type(db, key).await?;
    if !modules::enabled(db, identity.workspace, "cms").await? {
        return Err(Error::NotFound);
    }
    Ok(identity)
}
pub async fn public_index(
    pool: &PgPool,
    key: &str,
    query: PublicIndexQuery,
) -> Result<PublicIndex, Error> {
    const LIMIT: i64 = 50;
    if let Some(after) = &query.after {
        content::validate_slug(after)?;
    }
    let mut tx = pool.begin().await?;
    let identity = public_identity(&mut tx, key).await?;
    let (name, description) = dao::public_name(&mut tx, identity.workspace, identity.id).await?;
    let mut entries = dao::public_entries(
        &mut tx,
        identity.workspace,
        identity.id,
        query.after.as_deref(),
        LIMIT + 1,
    )
    .await?;
    let next_cursor = if entries.len() > LIMIT as usize {
        entries.pop();
        entries.last().map(|e| e.slug.clone())
    } else {
        None
    };
    Ok(PublicIndex {
        name,
        description,
        entries,
        next_cursor,
    })
}
pub async fn public_entry(pool: &PgPool, key: &str, slug: &str) -> Result<PublicPage, Error> {
    content::validate_slug(slug).map_err(|_| Error::NotFound)?;
    let mut tx = pool.begin().await?;
    let identity = public_identity(&mut tx, key).await?;
    let row = dao::public_entry(&mut tx, identity.workspace, identity.id, slug).await?;
    let fields = render(&mut tx, identity.workspace, &row.fields, &row.values).await?;
    Ok(PublicPage {
        title: row.title,
        slug: row.slug,
        published_at: Some(row.published_at),
        fields,
    })
}
pub async fn public_media(
    pool: &PgPool,
    key: &str,
    media: Uuid,
) -> Result<PublicMediaBytes, Error> {
    let mut tx = pool.begin().await?;
    let identity = public_identity(&mut tx, key).await?;
    let (file, media_type) =
        dao::published_media(&mut tx, identity.workspace, identity.id, media).await?;
    Ok(PublicMediaBytes {
        media_type,
        bytes: files::active_bytes(&mut tx, identity.workspace, file).await?,
    })
}
