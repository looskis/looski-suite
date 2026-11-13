// SPDX-License-Identifier: AGPL-3.0-only
//! Collection grants and revision-bound editorial review. cms.contribute alone
//! grants no content access. The same checks protect browser, API, CLI and MCP.
use super::*;
use sqlx::types::Json;

#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Author {
    pub name: String,
    pub bio: String,
    pub media_id: Option<Uuid>,
    /// Only the bundled brand asset or a published CMS media path; never user HTML.
    pub image_url: Option<String>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Contributor {
    pub membership_id: Uuid,
    pub type_id: Uuid,
    pub name: String,
    pub bio: String,
    pub photo_media_id: Option<Uuid>,
    pub active: bool,
    pub version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveContributor {
    pub membership_id: Uuid,
    pub type_id: Uuid,
    pub name: String,
    #[serde(default)]
    pub bio: String,
    pub photo_media_id: Option<Uuid>,
    pub active: bool,
    /// Zero creates a grant; updates require the current version.
    pub expected_version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CoauthorSettings {
    pub expected_version: i64,
    /// Empty removes the default co-author from future contributor drafts.
    pub name: String,
    #[serde(default)]
    pub use_looski_logo: bool,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewChange {
    pub expected_version: i64,
    pub action: ReviewAction,
    #[serde(default)]
    pub note: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReviewAction {
    Submit,
    RequestChanges,
    Withdraw,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct WritingAccess {
    pub contributor_only: bool,
    pub can_edit: bool,
    pub can_publish: bool,
    pub can_manage_contributors: bool,
    pub can_manage_types: bool,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ContributionEvent {
    pub id: Uuid,
    pub entry_id: Uuid,
    pub title: String,
    pub kind: String,
    pub note: String,
    pub created_at: String,
}
async fn staff(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    action: &str,
) -> Result<access::Access, Error> {
    let a = access::require(db, actor.identity, actor.workspace, action).await?;
    if !a.workspace_wide {
        return Err(Error::Forbidden);
    }
    Ok(a)
}
pub async fn writing_access(
    pool: &PgPool,
    actor: &ActorContext<'_>,
) -> Result<WritingAccess, Error> {
    let mut db = pool.begin().await?;
    staff(&mut db, actor, "workspace.read").await?;
    // UI capabilities never replace operation-level checks.
    let read = staff(&mut db, actor, "cms.read").await.is_ok();
    let contributor = access::require(&mut db, actor.identity, actor.workspace, "cms.contribute")
        .await
        .is_ok();
    Ok(WritingAccess {
        contributor_only: contributor && !read,
        can_edit: authorize(&mut db, actor, "cms.entries.edit").await.is_ok(),
        can_publish: staff(&mut db, actor, "cms.entries.publish").await.is_ok(),
        can_manage_contributors: staff(&mut db, actor, "workspace.access.admin")
            .await
            .is_ok(),
        can_manage_types: staff(&mut db, actor, "cms.types.manage").await.is_ok(),
    })
}
pub(super) async fn has_grant(
    db: &mut PgConnection,
    ws: Uuid,
    member: Uuid,
) -> Result<bool, Error> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM cms.contributors c JOIN cms.content_types t ON (t.workspace_id,t.id)=(c.workspace_id,c.type_id) WHERE c.workspace_id=$1 AND c.membership_id=$2 AND c.active AND t.archived_at IS NULL)")
        .bind(ws).bind(member).fetch_one(db).await?)
}
pub(super) async fn type_access(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Uuid,
    action: &str,
) -> Result<(), Error> {
    let mut a = authorize(db, actor, action).await?;
    if matches!(
        action,
        "cms.entries.edit" | "cms.entries.publish" | "cms.drafts.read"
    ) {
        a.workspace_wide &= authorize(db, actor, "cms.read").await?.workspace_wide;
        a.workspace_wide &= authorize(db, actor, "cms.drafts.read")
            .await?
            .workspace_wide;
    }
    if !a.workspace_wide && !sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM cms.contributors WHERE workspace_id=$1 AND type_id=$2 AND membership_id=$3 AND active)")
        .bind(actor.workspace).bind(id).bind(a.membership_id).fetch_one(db).await? { return Err(Error::NotFound); }
    Ok(())
}
pub(super) async fn entry_access(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Uuid,
    action: &str,
) -> Result<(), Error> {
    let mut a = authorize(db, actor, action).await?;
    if matches!(
        action,
        "cms.entries.edit" | "cms.entries.publish" | "cms.drafts.read"
    ) {
        a.workspace_wide &= authorize(db, actor, "cms.read").await?.workspace_wide;
        a.workspace_wide &= authorize(db, actor, "cms.drafts.read")
            .await?
            .workspace_wide;
    }
    if !a.workspace_wide && !sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM cms.entries e JOIN cms.contributors c ON (c.workspace_id,c.type_id,c.membership_id)=(e.workspace_id,e.type_id,e.contributor_id) WHERE e.workspace_id=$1 AND e.id=$2 AND e.contributor_id=$3 AND c.active)")
        .bind(actor.workspace).bind(id).bind(a.membership_id).fetch_one(db).await? { return Err(Error::NotFound); }
    Ok(())
}
pub(super) async fn media_access(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Uuid,
    action: &str,
) -> Result<(), Error> {
    let mut a = authorize(db, actor, action).await?;
    if matches!(
        action,
        "cms.entries.edit" | "cms.entries.publish" | "cms.drafts.read"
    ) {
        a.workspace_wide &= authorize(db, actor, "cms.read").await?.workspace_wide;
        a.workspace_wide &= authorize(db, actor, "cms.drafts.read")
            .await?
            .workspace_wide;
    }
    a.workspace_wide &= authorize(db, actor, "cms.read").await?.workspace_wide;
    // An administrator may supply a writer's profile image. Sharing that image
    // permits reading and retaining it, but never editing somebody else's media.
    if !a.workspace_wide && !sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM cms.media WHERE workspace_id=$1 AND id=$2 AND contributor_id=$3) OR ($4 AND EXISTS(SELECT 1 FROM cms.contributors c JOIN cms.content_types t ON (t.workspace_id,t.id)=(c.workspace_id,c.type_id) WHERE c.workspace_id=$1 AND c.photo_media_id=$2 AND c.membership_id=$3 AND c.active AND t.archived_at IS NULL))")
        .bind(actor.workspace).bind(id).bind(a.membership_id).bind(action == "cms.read").fetch_one(db).await? { return Err(Error::NotFound); }
    Ok(())
}
pub(super) async fn assign_entry(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Uuid,
    type_id: Uuid,
    member: Uuid,
) -> Result<(), Error> {
    type_access(db, actor, type_id, "cms.entries.edit").await?;
    sqlx::query("UPDATE cms.entries SET contributor_id=$3 WHERE workspace_id=$1 AND id=$2")
        .bind(actor.workspace)
        .bind(id)
        .bind(member)
        .execute(&mut *db)
        .await?;
    refresh_authors(db, actor, id).await
}
pub(super) async fn refresh_authors(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<(), Error> {
    // Attribution comes from the approved grant and collection, never client bylines.
    sqlx::query("UPDATE cms.entries e SET authors=jsonb_build_array(jsonb_build_object('name',c.name,'bio',c.bio,'media_id',c.photo_media_id,'image_url',NULL)) || CASE WHEN t.contribution_coauthor IS NULL THEN '[]'::jsonb ELSE jsonb_build_array(t.contribution_coauthor) END FROM cms.contributors c,cms.content_types t WHERE e.workspace_id=$1 AND e.id=$2 AND (c.workspace_id,c.type_id,c.membership_id)=(e.workspace_id,e.type_id,e.contributor_id) AND (t.workspace_id,t.id)=(e.workspace_id,e.type_id)")
        .bind(actor.workspace).bind(id).execute(db).await?;
    Ok(())
}
pub(super) fn editable(entry: &StoredEntry) -> Result<(), Error> {
    if entry.contributor_id.is_some() && entry.review_state == "submitted" {
        return Err(Error::Conflict(
            "This revision is in review; return it or withdraw the submission before editing",
        ));
    }
    Ok(())
}
pub(super) async fn publishable(
    db: &mut PgConnection,
    ws: Uuid,
    entry: &StoredEntry,
    head: Uuid,
    changed: bool,
) -> Result<(), Error> {
    if entry.contributor_id.is_some() {
        let submitted: Option<Uuid> = sqlx::query_scalar(
            "SELECT submitted_revision_id FROM cms.entries WHERE workspace_id=$1 AND id=$2",
        )
        .bind(ws)
        .bind(entry.id)
        .fetch_one(&mut *db)
        .await?;
        if entry.review_state != "submitted" || submitted != Some(head) || changed {
            return Err(Error::Conflict(
                "Submit the current revision for review before publishing; return it if the collection fields changed",
            ));
        }
        let media: Vec<Uuid> = entry.authors.iter().filter_map(|a| a.media_id).collect();
        if dao::known_media(db, ws, &media, true).await? != media.len() as i64 {
            return Err(Error::Conflict(
                "Replace the unavailable author image before publishing",
            ));
        }
    }
    Ok(())
}
pub(super) async fn published(db: &mut PgConnection, ws: Uuid, id: Uuid) -> Result<(), Error> {
    sqlx::query("UPDATE cms.entries SET review_state='published',review_note='' WHERE workspace_id=$1 AND id=$2").bind(ws).bind(id).execute(&mut *db).await?;
    event(db, ws, id, "published", "").await
}
async fn event(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    kind: &str,
    note: &str,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO cms.contribution_events(workspace_id,entry_id,kind,note) VALUES($1,$2,$3,$4)",
    )
    .bind(ws)
    .bind(id)
    .bind(kind)
    .bind(note)
    .execute(db)
    .await?;
    Ok(())
}
pub fn public_authors(mut authors: Vec<Author>, key: &str) -> Vec<Author> {
    for a in &mut authors {
        if let Some(id) = a.media_id {
            a.image_url = Some(format!("/api/v1/public/cms/{key}/media/{id}"));
        }
    }
    authors
}
pub async fn review(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: ReviewChange,
) -> Result<ContentEntry, Error> {
    if input.note.len() > 3000 {
        return Err(Error::Invalid("Feedback must be at most 3000 bytes"));
    }
    let mut tx = pool.begin().await?;
    let action = if matches!(input.action, ReviewAction::RequestChanges) {
        "cms.entries.publish"
    } else {
        "cms.entries.edit"
    };
    let write = entry_mutation(&mut tx, actor, action).await?;
    entry_access(&mut tx, actor, id, action).await?;
    let operation = format!("cms.entries.review:{id}");
    let request = serde_json::to_value(&input).expect("review serializes");
    if let Some(saved) =
        reliability::replay(&mut tx, actor.workspace, &write, &operation, key, &request).await?
    {
        return Ok(saved);
    }
    let current = dao::entry(&mut tx, actor.workspace, id, true).await?;
    if current.version != input.expected_version
        || current.archived
        || current.type_archived
        || current.contributor_id.is_none()
    {
        return Err(Error::Conflict(
            "Submission changed or is unavailable; reload before reviewing",
        ));
    }
    let head = dao::head_revision(&mut tx, actor.workspace, id).await?;
    let (state, revision, note) = match input.action {
        ReviewAction::Submit => {
            if current.review_state == "submitted"
                || !view(dao::entry(&mut tx, actor.workspace, id, false).await?, true)
                    .unpublished_changes
            {
                return Err(Error::Conflict("There are no new changes to submit"));
            }
            let t = dao::content_type(&mut tx, actor.workspace, current.type_id, false).await?;
            if head.fields != t.fields {
                return Err(Error::Conflict(
                    "Collection fields changed; save a new revision before submitting",
                ));
            }
            content::require_complete(&t.fields, &current.values)?;
            ("submitted", Some(head.id), "")
        }
        ReviewAction::RequestChanges => {
            if current.review_state != "submitted" || input.note.trim().is_empty() {
                return Err(Error::Conflict(
                    "Select a submitted revision and provide feedback",
                ));
            }
            ("changes_requested", None, input.note.trim())
        }
        ReviewAction::Withdraw => {
            if current.review_state != "submitted" {
                return Err(Error::Conflict("Only submitted revisions can be withdrawn"));
            }
            ("draft", None, "")
        }
    };
    sqlx::query("UPDATE cms.entries SET review_state=$3,submitted_revision_id=$4,review_note=$5,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
        .bind(actor.workspace).bind(id).bind(state).bind(revision).bind(note).execute(&mut *tx).await?;
    if state != "draft" {
        event(&mut tx, actor.workspace, id, state, note).await?;
    }
    finish_entry(
        &mut tx,
        actor,
        &write,
        id,
        "cms.submission_reviewed",
        &operation,
        key,
        &request,
    )
    .await?;
    let result = view(dao::entry(&mut tx, actor.workspace, id, false).await?, true);
    tx.commit().await?;
    Ok(result)
}
const GRANT: &str = "SELECT membership_id,type_id,name,bio,photo_media_id,active,version FROM cms.contributors WHERE workspace_id=$1";
#[derive(sqlx::FromRow)]
struct GrantRow {
    membership_id: Uuid,
    type_id: Uuid,
    name: String,
    bio: String,
    photo_media_id: Option<Uuid>,
    active: bool,
    version: i64,
}
impl From<GrantRow> for Contributor {
    fn from(r: GrantRow) -> Self {
        Self {
            membership_id: r.membership_id,
            type_id: r.type_id,
            name: r.name,
            bio: r.bio,
            photo_media_id: r.photo_media_id,
            active: r.active,
            version: r.version,
        }
    }
}
pub async fn list(pool: &PgPool, actor: &ActorContext<'_>) -> Result<Vec<Contributor>, Error> {
    let mut tx = pool.begin().await?;
    modules::require(&mut tx, actor.workspace, "cms").await?;
    let member = if staff(&mut tx, actor, "workspace.access.admin")
        .await
        .is_ok()
    {
        None
    } else {
        Some(authorize(&mut tx, actor, "cms.read").await?.membership_id)
    };
    let query = format!(
        "{GRANT} AND ($2::uuid IS NULL OR membership_id=$2) ORDER BY type_id,membership_id LIMIT 100"
    );
    Ok(
        sqlx::query_as::<_, GrantRow>(sqlx::AssertSqlSafe(query.as_str()))
            .bind(actor.workspace)
            .bind(member)
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
    )
}
pub async fn save(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    key: &str,
    input: SaveContributor,
) -> Result<Contributor, Error> {
    if input.name.trim().is_empty()
        || input.name.chars().any(char::is_control)
        || input.name.chars().count() > 120
        || input.bio.chars().count() > 2000
    {
        return Err(Error::Invalid(
            "Enter a name up to 120 characters and bio up to 2000 characters",
        ));
    }
    let mut tx = pool.begin().await?;
    modules::lock_workspace(&mut tx, actor.workspace).await?;
    modules::require(&mut tx, actor.workspace, "cms").await?;
    let admin = staff(&mut tx, actor, "workspace.access.admin").await;
    let write = match admin {
        Ok(a) => a,
        Err(Error::Forbidden) => authorize(&mut tx, actor, "cms.read").await?,
        Err(e) => return Err(e),
    };
    let is_admin = staff(&mut tx, actor, "workspace.access.admin")
        .await
        .is_ok();
    let query = format!("{GRANT} AND membership_id=$2 AND type_id=$3 FOR UPDATE");
    let current = sqlx::query_as::<_, GrantRow>(sqlx::AssertSqlSafe(query.as_str()))
        .bind(actor.workspace)
        .bind(input.membership_id)
        .bind(input.type_id)
        .fetch_optional(&mut *tx)
        .await?;
    if !is_admin
        && (write.membership_id != input.membership_id
            || current
                .as_ref()
                .is_none_or(|c| !c.active || c.active != input.active))
    {
        return Err(Error::Forbidden);
    }
    let operation = "cms.contributors.save";
    let request = serde_json::to_value(&input).expect("grant serializes");
    if let Some(saved) =
        reliability::replay(&mut tx, actor.workspace, &write, operation, key, &request).await?
    {
        return Ok(saved);
    }
    if current.as_ref().map_or(0, |c| c.version) != input.expected_version {
        return Err(Error::Conflict("Contributor changed; reload before saving"));
    }
    // Suspension and withdrawn media must not prevent removing a grant.
    if input.active {
        access::require_active_member(&mut tx, actor.workspace, input.membership_id).await?;
    }
    dao::content_type(&mut tx, actor.workspace, input.type_id, false).await?;
    if let Some(id) = input.photo_media_id.filter(|_| {
        input.active || current.as_ref().and_then(|c| c.photo_media_id) != input.photo_media_id
    }) {
        media_access(&mut tx, actor, id, "cms.read").await?;
        if dao::known_media(&mut tx, actor.workspace, &[id], true).await? != 1 {
            return Err(Error::Invalid("Choose an active profile image"));
        }
        let media = dao::media(&mut tx, actor.workspace, id, false).await?;
        if !media.media_type.starts_with("image/") {
            return Err(Error::Invalid("Profile images must be images"));
        }
    }
    sqlx::query("INSERT INTO cms.contributors(workspace_id,membership_id,type_id,name,bio,photo_media_id,active) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(workspace_id,membership_id,type_id) DO UPDATE SET name=excluded.name,bio=excluded.bio,photo_media_id=excluded.photo_media_id,active=excluded.active,version=contributors.version+1")
        .bind(actor.workspace).bind(input.membership_id).bind(input.type_id).bind(input.name.trim()).bind(&input.bio).bind(input.photo_media_id).bind(input.active).execute(&mut *tx).await?;
    let result = Contributor {
        membership_id: input.membership_id,
        type_id: input.type_id,
        name: input.name.trim().into(),
        bio: input.bio,
        photo_media_id: input.photo_media_id,
        active: input.active,
        version: input.expected_version + 1,
    };
    reliability::audit(
        &mut tx,
        actor,
        &write,
        "cms.contributor_saved",
        input.membership_id,
        serde_json::json!({"type_id":input.type_id,"active":input.active}),
    )
    .await?;
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
pub async fn coauthor(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: CoauthorSettings,
) -> Result<ContentType, Error> {
    if input.name.chars().count() > 120 || input.name.chars().any(char::is_control) {
        return Err(Error::Invalid(
            "Co-author names must be at most 120 characters",
        ));
    }
    let mut tx = pool.begin().await?;
    let write = mutation(&mut tx, actor, "cms.types.manage").await?;
    let operation = format!("cms.coauthor:{id}");
    let request = serde_json::to_value(&input).expect("settings serialize");
    if let Some(saved) =
        reliability::replay(&mut tx, actor.workspace, &write, &operation, key, &request).await?
    {
        return Ok(saved);
    }
    let t = dao::content_type(&mut tx, actor.workspace, id, true).await?;
    if t.version != input.expected_version {
        return Err(Error::Conflict("Collection changed; reload before saving"));
    }
    let author = (!input.name.trim().is_empty()).then(|| Author {
        name: input.name.trim().into(),
        bio: String::new(),
        media_id: None,
        image_url: input
            .use_looski_logo
            .then(|| "/brand/looski-mark.svg".into()),
    });
    sqlx::query("UPDATE cms.content_types SET contribution_coauthor=$3,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(actor.workspace).bind(id).bind(author.map(Json)).execute(&mut *tx).await?;
    let result = dao::content_type(&mut tx, actor.workspace, id, false).await?;
    reliability::audit(
        &mut tx,
        actor,
        &write,
        "cms.coauthor_saved",
        id,
        serde_json::json!({}),
    )
    .await?;
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
pub async fn events(
    pool: &PgPool,
    actor: &ActorContext<'_>,
) -> Result<Vec<ContributionEvent>, Error> {
    let mut tx = pool.begin().await?;
    let read = authorize(&mut tx, actor, "cms.drafts.read").await?;
    let reader = authorize(&mut tx, actor, "cms.read").await?;
    let member = (!(read.workspace_wide && reader.workspace_wide)).then_some(read.membership_id);
    Ok(sqlx::query_scalar::<_,Json<ContributionEvent>>("SELECT jsonb_build_object('id',n.id,'entry_id',n.entry_id,'title',e.title,'kind',n.kind,'note',n.note,'created_at',n.created_at::text) FROM cms.contribution_events n JOIN cms.entries e ON (e.workspace_id,e.id)=(n.workspace_id,n.entry_id) WHERE n.workspace_id=$1 AND ($2::uuid IS NULL OR (e.contributor_id=$2 AND EXISTS(SELECT 1 FROM cms.contributors c WHERE (c.workspace_id,c.type_id,c.membership_id)=(e.workspace_id,e.type_id,$2) AND c.active))) ORDER BY n.created_at DESC,n.id DESC LIMIT 50")
        .bind(actor.workspace).bind(member).fetch_all(&mut *tx).await?.into_iter().map(|r|r.0).collect())
}

/// Compatibility fields for headless consumers that still read a text byline.
/// Structured revision authors remain the canonical attribution.
pub(super) async fn byline_values(
    db: &mut PgConnection,
    ws: Uuid,
    type_id: Uuid,
    member: Option<Uuid>,
    fields: &[ContentField],
    values: &mut Map<String, Value>,
) -> Result<(), Error> {
    let Some(member) = member else {
        return Ok(());
    };
    let (name,coauthor):(String,Option<String>)=sqlx::query_as("SELECT c.name,t.contribution_coauthor->>'name' FROM cms.contributors c JOIN cms.content_types t ON (t.workspace_id,t.id)=(c.workspace_id,c.type_id) WHERE c.workspace_id=$1 AND c.type_id=$2 AND c.membership_id=$3").bind(ws).bind(type_id).bind(member).fetch_one(db).await?;
    for field in fields {
        if field.kind == ContentFieldKind::Text {
            if field.key == "author" {
                values.insert(
                    field.key.clone(),
                    Value::String(
                        coauthor
                            .as_ref()
                            .map_or_else(|| name.clone(), |co| format!("{name} and {co}")),
                    ),
                );
            }
            if field.key == "author_id" {
                values.insert(field.key.clone(), Value::String(String::new()));
            }
        }
    }
    Ok(())
}
