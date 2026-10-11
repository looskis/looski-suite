// SPDX-License-Identifier: AGPL-3.0-only
//! Forms owns definitions, immutable published versions and original answers.
//! Public submission rights come only from a form's public key and never grant
//! any other workspace access.
mod dao;
pub mod definition;
pub mod recipes;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    modules, reliability,
};
pub use definition::Definition;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct VersionRef {
    pub id: Uuid,
    pub number: i32,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Form {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub confirmation_message: String,
    pub draft: Definition,
    pub public_key: String,
    pub published_version: Option<VersionRef>,
    /// The draft differs from the published version.
    pub unpublished_changes: bool,
    pub accepting: bool,
    pub archived: bool,
    pub notification_membership_ids: Vec<Uuid>,
    pub submission_count: i64,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct FormSummary {
    pub id: Uuid,
    pub name: String,
    pub public_key: String,
    pub published_version_number: Option<i32>,
    pub accepting: bool,
    pub archived: bool,
    pub submission_count: i64,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveForm {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub confirmation_message: String,
    #[serde(default)]
    pub draft: Definition,
    /// Required for updates; omitted on creation.
    pub expected_version: Option<i64>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct ListForms {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
    pub search: Option<String>,
    pub archived: Option<bool>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct FormPage {
    pub items: Vec<FormSummary>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PublishForm {
    pub expected_version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SetAccepting {
    pub expected_version: i64,
    pub accepting: bool,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ArchiveForm {
    pub expected_version: i64,
    pub archived: bool,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SetNotifications {
    pub expected_version: i64,
    pub membership_ids: Vec<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct FormVersion {
    pub id: Uuid,
    pub form_id: Uuid,
    pub number: i32,
    pub name: String,
    pub definition: Definition,
    pub published_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Submission {
    pub id: Uuid,
    pub form_id: Uuid,
    pub version_id: Uuid,
    pub version_number: i32,
    /// Original visible answers keyed by stable field keys.
    pub answers: serde_json::Map<String, serde_json::Value>,
    pub submitted_at: String,
    pub recipe_run: Option<recipes::RunSummary>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct SubmissionQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct SubmissionPage {
    pub items: Vec<Submission>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct OptionCount {
    pub key: String,
    pub label: String,
    pub count: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct FieldSummary {
    pub key: String,
    pub label: String,
    pub answered: i64,
    /// Choice and checkbox counts; empty for free-text questions.
    pub options: Vec<OptionCount>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct FormSummaryReport {
    pub submissions: i64,
    pub version_number: i32,
    pub fields: Vec<FieldSummary>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct PublicForm {
    pub name: String,
    pub description: String,
    pub definition: Definition,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PublicSubmission {
    /// Client-generated retry key: 16–128 visible ASCII characters.
    pub submission_key: String,
    pub answers: serde_json::Map<String, serde_json::Value>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Receipt {
    pub confirmation_message: String,
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
    modules::require(db, actor.workspace, "forms").await?;
    Ok(a)
}
/// Check write authority, serialize with application selection, then re-check.
async fn mutation(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    action: &str,
) -> Result<access::Access, Error> {
    authorize(db, actor, action).await?;
    modules::lock_workspace(db, actor.workspace).await?;
    authorize(db, actor, "forms.read").await?;
    authorize(db, actor, action).await
}
fn page_limit(limit: Option<i64>) -> Result<i64, Error> {
    let limit = limit.unwrap_or(50);
    if !(1..=100).contains(&limit) {
        return Err(Error::Invalid("Limit must be 1–100"));
    }
    Ok(limit)
}
fn validate_form(input: &SaveForm) -> Result<(), Error> {
    if input.name.trim().is_empty()
        || input.name.chars().count() > 120
        || input.name.chars().any(char::is_control)
        || input.description.chars().count() > 2000
        || input.confirmation_message.chars().count() > 2000
    {
        return Err(Error::Invalid(
            "Name must be 1–120 printable characters; description and confirmation at most 2000",
        ));
    }
    definition::validate(&input.draft, false)
}

pub async fn list(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: ListForms,
) -> Result<FormPage, Error> {
    let limit = page_limit(query.limit)?;
    if query.search.as_ref().is_some_and(|s| s.len() > 200) {
        return Err(Error::Invalid("Search must be at most 200 bytes"));
    }
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "forms.read").await?;
    if let Some(after) = query.after {
        dao::get(&mut tx, actor.workspace, after, false).await?;
    }
    let mut forms = dao::list(&mut tx, actor.workspace, &query, limit + 1).await?;
    let next_cursor = if forms.len() > limit as usize {
        forms.pop();
        forms.last().map(|f| f.id)
    } else {
        None
    };
    let items = forms
        .into_iter()
        .map(|form| FormSummary {
            id: form.id,
            name: form.name,
            public_key: form.public_key,
            published_version_number: form.published_version.map(|v| v.number),
            accepting: form.accepting,
            archived: form.archived,
            submission_count: form.submission_count,
            version: form.version,
            created_at: form.created_at,
            updated_at: form.updated_at,
        })
        .collect();
    Ok(FormPage { items, next_cursor })
}
/// Business export projection: complete forms, including drafts.
pub async fn export_forms(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: ListForms,
) -> Result<serde_json::Value, Error> {
    let limit = page_limit(query.limit)?;
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "forms.read").await?;
    if let Some(after) = query.after {
        dao::get(&mut tx, actor.workspace, after, false).await?;
    }
    let mut items = dao::list(&mut tx, actor.workspace, &query, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|f| f.id)
    } else {
        None
    };
    Ok(serde_json::json!({"items":items,"next_cursor":next_cursor}))
}
/// Business export projection: one bounded page of a form's published versions.
pub async fn export_versions(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    query: SubmissionQuery,
) -> Result<serde_json::Value, Error> {
    let limit = page_limit(query.limit)?;
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "forms.read").await?;
    dao::get(&mut tx, actor.workspace, id, false).await?;
    if let Some(after) = query.after {
        dao::version(&mut tx, actor.workspace, id, after).await?;
    }
    let mut items = dao::version_page(&mut tx, actor.workspace, id, query.after, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|v| v.id)
    } else {
        None
    };
    Ok(serde_json::json!({"items":items,"next_cursor":next_cursor}))
}
pub async fn get(pool: &PgPool, actor: &ActorContext<'_>, id: Uuid) -> Result<Form, Error> {
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "forms.read").await?;
    dao::get(&mut tx, actor.workspace, id, false).await
}
pub async fn save(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Option<Uuid>,
    key: &str,
    input: SaveForm,
) -> Result<Form, Error> {
    validate_form(&input)?;
    if id.is_some() != input.expected_version.is_some() {
        return Err(Error::Invalid(
            "Updates require expected_version; creation must omit it",
        ));
    }
    let mut tx = pool.begin().await?;
    let write = mutation(&mut tx, actor, "forms.manage").await?;
    let operation = id
        .map(|id| format!("forms.update:{id}"))
        .unwrap_or_else(|| "forms.create".into());
    let request = serde_json::to_value(&input).expect("form serializes");
    if let Some(saved) =
        reliability::replay::<Form>(&mut tx, actor.workspace, &write, &operation, key, &request)
            .await?
    {
        dao::get(&mut tx, actor.workspace, saved.id, false).await?;
        return Ok(saved);
    }
    let id = if let Some(id) = id {
        let current = dao::get(&mut tx, actor.workspace, id, true).await?;
        if Some(current.version) != input.expected_version || current.archived {
            return Err(Error::Conflict(
                "Form changed or is archived; reload before editing",
            ));
        }
        dao::update(&mut tx, actor.workspace, id, &input).await?;
        id
    } else {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|_| Error::Unavailable)?;
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
        dao::insert(
            &mut tx,
            actor.workspace,
            &input,
            &URL_SAFE_NO_PAD.encode(bytes),
        )
        .await?
    };
    reliability::audit(
        &mut tx,
        actor,
        &write,
        "forms.saved",
        id,
        serde_json::json!({}),
    )
    .await?;
    let result = dao::get(&mut tx, actor.workspace, id, false).await?;
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
/// Shared lifecycle path: replay, version check, change, audit, result.
#[allow(clippy::too_many_arguments)]
async fn change<T: Serialize>(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    operation: &str,
    input: &T,
    expected: i64,
    apply: impl AsyncFnOnce(&mut PgConnection, &access::Access, &Form) -> Result<(), Error>,
) -> Result<Form, Error> {
    let mut tx = pool.begin().await?;
    let write = mutation(&mut tx, actor, "forms.manage").await?;
    let operation = format!("{operation}:{id}");
    let request = serde_json::to_value(input).expect("form change serializes");
    if let Some(saved) =
        reliability::replay::<Form>(&mut tx, actor.workspace, &write, &operation, key, &request)
            .await?
    {
        dao::get(&mut tx, actor.workspace, id, false).await?;
        return Ok(saved);
    }
    let current = dao::get(&mut tx, actor.workspace, id, true).await?;
    if current.version != expected {
        return Err(Error::Conflict("Form changed; reload before editing"));
    }
    apply(&mut tx, &write, &current).await?;
    reliability::audit(
        &mut tx,
        actor,
        &write,
        "forms.changed",
        id,
        serde_json::json!({"operation":operation.split(':').next()}),
    )
    .await?;
    let result = dao::get(&mut tx, actor.workspace, id, false).await?;
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
/// Snapshot the draft as a new immutable version and start accepting answers.
pub async fn publish(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: PublishForm,
) -> Result<Form, Error> {
    let workspace = actor.workspace;
    change(
        pool,
        actor,
        id,
        key,
        "forms.publish",
        &input,
        input.expected_version,
        async |db, write, current| {
            if current.archived {
                return Err(Error::Conflict("Restore the form before publishing"));
            }
            if current.published_version.is_some() && !current.unpublished_changes {
                return Err(Error::Conflict("The draft has no unpublished changes"));
            }
            definition::validate(&current.draft, true)?;
            dao::publish(db, workspace, current, write.membership_id).await
        },
    )
    .await
}
pub async fn set_accepting(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: SetAccepting,
) -> Result<Form, Error> {
    let workspace = actor.workspace;
    let accepting = input.accepting;
    change(
        pool,
        actor,
        id,
        key,
        "forms.accepting",
        &input,
        input.expected_version,
        async |db, _, current| {
            if accepting && (current.archived || current.published_version.is_none()) {
                return Err(Error::Conflict(
                    "Only a published, unarchived form can accept answers",
                ));
            }
            dao::set_accepting(db, workspace, id, accepting).await
        },
    )
    .await
}
pub async fn archive(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: ArchiveForm,
) -> Result<Form, Error> {
    let workspace = actor.workspace;
    let archived = input.archived;
    change(
        pool,
        actor,
        id,
        key,
        "forms.archive",
        &input,
        input.expected_version,
        async |db, _, _| dao::archive(db, workspace, id, archived).await,
    )
    .await
}
/// Recipients must currently hold `forms.submissions.read`; delivery re-checks.
pub async fn set_notifications(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: SetNotifications,
) -> Result<Form, Error> {
    let mut members = input.membership_ids.clone();
    members.sort_unstable();
    members.dedup();
    if members.len() != input.membership_ids.len() || members.len() > 20 {
        return Err(Error::Invalid(
            "List at most 20 distinct notification recipients",
        ));
    }
    let workspace = actor.workspace;
    change(
        pool,
        actor,
        id,
        key,
        "forms.notifications",
        &input,
        input.expected_version,
        async |db, _, _| {
            for member in &members {
                if !dao::can_read_submissions(db, workspace, *member).await? {
                    return Err(Error::Invalid(
                        "Recipients must be active members who can read submissions",
                    ));
                }
            }
            dao::set_notifications(db, workspace, id, &members).await
        },
    )
    .await
}
pub async fn versions(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<Vec<FormVersion>, Error> {
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "forms.read").await?;
    dao::get(&mut tx, actor.workspace, id, false).await?;
    dao::versions(&mut tx, actor.workspace, id).await
}
pub async fn submissions(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    query: SubmissionQuery,
) -> Result<SubmissionPage, Error> {
    let limit = page_limit(query.limit)?;
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "forms.submissions.read").await?;
    dao::get(&mut tx, actor.workspace, id, false).await?;
    if let Some(after) = query.after {
        dao::submission(&mut tx, actor.workspace, id, after).await?;
    }
    let mut items = dao::submissions(&mut tx, actor.workspace, id, query.after, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|s| s.id)
    } else {
        None
    };
    recipes::attach_runs(&mut tx, actor, &mut items).await?;
    Ok(SubmissionPage { items, next_cursor })
}
pub async fn submission(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    submission: Uuid,
) -> Result<Submission, Error> {
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "forms.submissions.read").await?;
    let mut item = vec![dao::submission(&mut tx, actor.workspace, id, submission).await?];
    recipes::attach_runs(&mut tx, actor, &mut item).await?;
    Ok(item.pop().expect("one submission"))
}
/// Response counts for the published version's choice and checkbox questions.
pub async fn summary(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<FormSummaryReport, Error> {
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "forms.submissions.read").await?;
    let form = dao::get(&mut tx, actor.workspace, id, false).await?;
    let Some(published) = form.published_version else {
        return Err(Error::Conflict("Publish the form to collect responses"));
    };
    let version = dao::version(&mut tx, actor.workspace, id, published.id).await?;
    let mut fields = Vec::new();
    for field in &version.definition.fields {
        if field.kind == definition::FieldKind::Statement {
            continue;
        }
        let answered = dao::answered(&mut tx, actor.workspace, id, &field.key).await?;
        let options = match field.kind {
            definition::FieldKind::Choice | definition::FieldKind::MultiChoice => {
                let mut counts = Vec::new();
                for option in &field.options {
                    counts.push(OptionCount {
                        key: option.key.clone(),
                        label: option.label.clone(),
                        count: dao::chosen(
                            &mut tx,
                            actor.workspace,
                            id,
                            &field.key,
                            serde_json::Value::String(option.key.clone()),
                        )
                        .await?,
                    });
                }
                counts
            }
            definition::FieldKind::Checkbox => vec![OptionCount {
                key: "true".into(),
                label: "Checked".into(),
                count: dao::chosen(
                    &mut tx,
                    actor.workspace,
                    id,
                    &field.key,
                    serde_json::Value::Bool(true),
                )
                .await?,
            }],
            _ => Vec::new(),
        };
        fields.push(FieldSummary {
            key: field.key.clone(),
            label: field.label.clone(),
            answered,
            options,
        });
    }
    Ok(FormSummaryReport {
        submissions: form.submission_count,
        version_number: published.number,
        fields,
    })
}

/// May this actor point other content at this form? Unknown forms, foreign forms and
/// missing authority answer alike, so a reference cannot probe for existence.
pub(crate) async fn can_reference(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<bool, Error> {
    match authorize(db, actor, "forms.read").await {
        Ok(_) => {}
        Err(Error::Forbidden) => return Ok(false),
        Err(error) => return Err(error),
    }
    match dao::get(db, actor.workspace, id, false).await {
        Ok(form) => Ok(!form.archived),
        Err(Error::NotFound) => Ok(false),
        Err(error) => Err(error),
    }
}
/// The public key to embed, only while Forms is enabled and the form accepts answers.
pub(crate) async fn embeddable(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
) -> Result<Option<String>, Error> {
    if !modules::enabled(db, workspace, "forms").await? {
        return Ok(None);
    }
    match dao::get(db, workspace, id, false).await {
        Ok(form) if form.accepting && !form.archived => Ok(Some(form.public_key)),
        Ok(_) | Err(Error::NotFound) => Ok(None),
        Err(error) => Err(error),
    }
}
/// The published definition, only while the form accepts answers and Forms is enabled.
pub async fn public_form(pool: &PgPool, key: &str) -> Result<PublicForm, Error> {
    let mut tx = pool.begin().await?;
    let (workspace, form, _) = public_identity(&mut tx, key, false).await?;
    let version = dao::accepting_version(&mut tx, workspace, form).await?;
    Ok(PublicForm {
        name: version.name,
        description: dao::public_description(&mut tx, version.form_id).await?,
        definition: version.definition,
    })
}
/// Resolve a public key to its form while Forms is enabled for the workspace.
async fn public_identity(
    db: &mut PgConnection,
    key: &str,
    lock: bool,
) -> Result<(Uuid, Uuid, String), Error> {
    if key.len() != 22
        || !key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(Error::NotFound);
    }
    let identity = dao::public(db, key, lock).await?;
    if !modules::enabled(db, identity.0, "forms").await? {
        return Err(Error::NotFound);
    }
    Ok(identity)
}
pub async fn public_submit(
    pool: &PgPool,
    key: &str,
    input: PublicSubmission,
) -> Result<Receipt, Error> {
    if !input.submission_key.bytes().all(|b| b.is_ascii_graphic())
        || !(16..=128).contains(&input.submission_key.len())
    {
        return Err(Error::Invalid(
            "A submission key of 16–128 visible ASCII characters is required",
        ));
    }
    let mut tx = pool.begin().await?;
    // Shares the workspace row with other submissions; conflicts only with
    // application disablement, so pending work cannot start after disable.
    let (workspace, form, confirmation) = public_identity(&mut tx, key, true).await?;
    // A retry of a committed submission is compared with the version it answered,
    // so closing the form or publishing again cannot turn success into failure.
    if let Some((version, existing)) =
        dao::submission_by_key(&mut tx, workspace, form, &input.submission_key).await?
    {
        let version = dao::version(&mut tx, workspace, form, version).await?;
        let answers = definition::answers(&version.definition, &input.answers)?;
        if existing != serde_json::Value::Object(answers) {
            return Err(Error::Conflict(
                "This submission key was already used with different answers",
            ));
        }
        return Ok(Receipt {
            confirmation_message: confirmation,
        });
    }
    let version = dao::accepting_version(&mut tx, workspace, form).await?;
    let answers = definition::answers(&version.definition, &input.answers)?;
    // Reserve room for JSONB whitespace expansion, like the column constraint measures.
    if serde_json::to_vec_pretty(&answers)
        .expect("answers serialize")
        .len()
        > 131072
    {
        return Err(Error::Invalid("These answers are too long to submit"));
    }
    let id = dao::insert_submission(
        &mut tx,
        workspace,
        &version,
        &answers,
        &input.submission_key,
    )
    .await?;
    recipes::start(&mut tx, workspace, version.form_id, id).await?;
    for recipient in dao::recipients(&mut tx, workspace, version.form_id).await? {
        crate::mail::notify_submission(&mut tx, workspace, &recipient, &version, id).await?;
    }
    tx.commit().await?;
    Ok(Receipt {
        confirmation_message: confirmation,
    })
}
/// A member who may currently receive submission content.
pub(crate) struct Recipient {
    pub user: Uuid,
    pub membership: Uuid,
    pub issuer: String,
    pub subject: String,
}

/// Scheduling owns the surrounding transaction and the sole booking-triggered recipe.
/// This interface deliberately does not start the standalone Forms recipe or notifications.
pub(crate) async fn booking_form(
    db: &mut PgConnection,
    ws: Uuid,
    form: Uuid,
) -> Result<FormVersion, Error> {
    modules::require(db, ws, "forms").await?;
    dao::accepting_version(db, ws, form).await
}
pub(crate) async fn booking_submit(
    db: &mut PgConnection,
    ws: Uuid,
    form: Uuid,
    version_id: Uuid,
    answers: &serde_json::Map<String, serde_json::Value>,
    booking: Uuid,
) -> Result<Uuid, Error> {
    let version = booking_form(db, ws, form).await?;
    if version.id != version_id {
        return Err(Error::Conflict("Intake changed; reload the booking page"));
    }
    let answers = definition::answers(&version.definition, answers)?;
    if serde_json::to_vec_pretty(&answers)
        .expect("serializes")
        .len()
        > 131072
    {
        return Err(Error::Invalid("These answers are too long to submit"));
    }
    dao::insert_submission(db, ws, &version, &answers, &format!("booking:{booking}")).await
}
pub(crate) async fn booking_submission(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    form: Uuid,
    id: Uuid,
) -> Result<Submission, Error> {
    authorize(db, a, "forms.submissions.read").await?;
    dao::submission(db, a.workspace, form, id).await
}
