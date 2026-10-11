// SPDX-License-Identifier: AGPL-3.0-only
//! Built-in Forms-to-CRM inquiry recipe. A submission commits with its run and
//! durable job; the worker resolves the contact through Contacts intake and
//! records the inquiry as an opportunity through CRM, under the authority
//! recorded when an administrator activated the recipe.
mod dao;
use super::{Submission, authorize, definition::FieldKind, mutation};
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    contacts::{self, ContactInput, IdentifierInput},
    crm, jobs, modules, reliability,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Explicit field mapping using stable field keys of the published version.
#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Mapping {
    /// Up to three short-text answers joined to form the contact name.
    #[serde(default)]
    pub name_fields: Vec<String>,
    pub email_field: Option<String>,
    pub phone_field: Option<String>,
    /// Default region for phone answers without a country code, such as `US`.
    pub phone_country: Option<String>,
    pub pipeline_id: Uuid,
    pub stage_id: Uuid,
    pub owner_membership_id: Option<Uuid>,
    pub title_prefix: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RecipeRevision {
    pub id: Uuid,
    pub number: i32,
    pub mapping: Mapping,
    pub created_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Recipe {
    pub id: Uuid,
    pub form_id: Uuid,
    pub active: bool,
    pub revision: Option<RecipeRevision>,
    /// Member whose current permissions each run uses.
    pub activated_by_membership_id: Option<Uuid>,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfigureRecipe {
    /// Omitted for the first configuration of a form's recipe.
    pub expected_version: Option<i64>,
    pub mapping: Mapping,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SetRecipeActive {
    pub expected_version: i64,
    pub active: bool,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RunSummary {
    pub id: Uuid,
    pub submission_id: Uuid,
    /// queued, awaiting_review, completed or dismissed; failures are job states.
    pub state: String,
    pub revision_number: i32,
    pub job_id: Uuid,
    pub job_state: String,
    pub error_code: Option<String>,
    pub intake_id: Option<Uuid>,
    pub contact_id: Option<Uuid>,
    pub opportunity_id: Option<Uuid>,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RunContext {
    pub run_id: Uuid,
    pub recipe_id: Uuid,
    pub form_id: Uuid,
    pub submission_id: Uuid,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct RunQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RunPage {
    pub items: Vec<RunSummary>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RetryRun {
    pub expected_version: i64,
}

/// Permissions a run uses; activation requires the activator to hold them.
const RUN_PERMISSIONS: &[&str] = &[
    "forms.recipes.manage",
    "contacts.read",
    "contacts.create",
    "contacts.intake.create",
    "contacts.intake.read",
    "crm.opportunities.read",
    "crm.opportunities.create",
    "crm.opportunities.participants.manage",
];

async fn validate_mapping(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    form: Uuid,
    mapping: &Mapping,
) -> Result<(), Error> {
    if mapping.title_prefix.trim().is_empty()
        || mapping.title_prefix.chars().count() > 120
        || mapping.title_prefix.chars().any(char::is_control)
        || mapping.name_fields.len() > 3
        || mapping
            .phone_country
            .as_ref()
            .is_some_and(|c| c.len() != 2 || !c.bytes().all(|b| b.is_ascii_uppercase()))
    {
        return Err(Error::Invalid(
            "Title prefix must be 1–120 characters, at most three name fields and a two-letter country",
        ));
    }
    if mapping.email_field.is_none() && mapping.phone_field.is_none() {
        return Err(Error::Invalid(
            "Map an email or phone question so respondents can be matched",
        ));
    }
    let current = super::dao::get(db, actor.workspace, form, false).await?;
    let Some(published) = current.published_version else {
        return Err(Error::Conflict(
            "Publish the form before configuring its recipe",
        ));
    };
    let version = super::dao::version(db, actor.workspace, form, published.id).await?;
    let kind = |key: &String| {
        version
            .definition
            .fields
            .iter()
            .find(|f| &f.key == key)
            .map(|f| f.kind)
    };
    if mapping
        .name_fields
        .iter()
        .any(|k| kind(k) != Some(FieldKind::ShortText))
        || mapping
            .email_field
            .as_ref()
            .is_some_and(|k| kind(k) != Some(FieldKind::Email))
        || mapping
            .phone_field
            .as_ref()
            .is_some_and(|k| kind(k) != Some(FieldKind::Phone))
    {
        return Err(Error::Invalid(
            "Mapped questions must exist in the published version with name, email or phone types",
        ));
    }
    modules::require_crm(db, actor).await?;
    crm::assignable_stage(db, actor, mapping.pipeline_id, mapping.stage_id).await?;
    if let Some(owner) = mapping.owner_membership_id {
        access::require_active_member(db, actor.workspace, owner).await?;
    }
    Ok(())
}
pub async fn get(pool: &PgPool, actor: &ActorContext<'_>, form: Uuid) -> Result<Recipe, Error> {
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "forms.read").await?;
    super::dao::get(&mut tx, actor.workspace, form, false).await?;
    dao::get(&mut tx, actor.workspace, form, false).await
}
pub async fn configure(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    form: Uuid,
    key: &str,
    input: ConfigureRecipe,
) -> Result<Recipe, Error> {
    let mut tx = pool.begin().await?;
    let write = mutation(&mut tx, actor, "forms.recipes.manage").await?;
    let operation = format!("forms.recipe.configure:{form}");
    let request = serde_json::to_value(&input).expect("recipe serializes");
    if let Some(saved) =
        reliability::replay::<Recipe>(&mut tx, actor.workspace, &write, &operation, key, &request)
            .await?
    {
        dao::get(&mut tx, actor.workspace, form, false).await?;
        return Ok(saved);
    }
    super::dao::get(&mut tx, actor.workspace, form, true).await?;
    let existing = match dao::get(&mut tx, actor.workspace, form, true).await {
        Ok(recipe) => Some(recipe),
        Err(Error::NotFound) => None,
        Err(error) => return Err(error),
    };
    if existing.as_ref().map(|r| r.version) != input.expected_version {
        return Err(Error::Conflict("Recipe changed; reload before editing"));
    }
    validate_mapping(&mut tx, actor, form, &input.mapping).await?;
    let recipe = match existing {
        Some(recipe) => recipe.id,
        None => dao::insert(&mut tx, actor.workspace, form).await?,
    };
    dao::revise(
        &mut tx,
        actor.workspace,
        recipe,
        &input.mapping,
        write.membership_id,
    )
    .await?;
    reliability::audit(
        &mut tx,
        actor,
        &write,
        "forms.recipe.configured",
        recipe,
        serde_json::json!({"form_id":form}),
    )
    .await?;
    let result = dao::get(&mut tx, actor.workspace, form, false).await?;
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
/// Activation records the activator's authority for future runs; it must
/// already include every action a run performs. Deactivation stops new runs.
pub async fn set_active(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    form: Uuid,
    key: &str,
    input: SetRecipeActive,
) -> Result<Recipe, Error> {
    let mut tx = pool.begin().await?;
    let write = mutation(&mut tx, actor, "forms.recipes.manage").await?;
    let operation = format!("forms.recipe.active:{form}");
    let request = serde_json::to_value(&input).expect("recipe serializes");
    if let Some(saved) =
        reliability::replay::<Recipe>(&mut tx, actor.workspace, &write, &operation, key, &request)
            .await?
    {
        dao::get(&mut tx, actor.workspace, form, false).await?;
        return Ok(saved);
    }
    let current = dao::get(&mut tx, actor.workspace, form, true).await?;
    if current.version != input.expected_version {
        return Err(Error::Conflict("Recipe changed; reload before editing"));
    }
    if input.active {
        let Some(revision) = &current.revision else {
            return Err(Error::Conflict("Configure the recipe before activating it"));
        };
        for action in RUN_PERMISSIONS {
            if !access::require(&mut tx, actor.identity, actor.workspace, action)
                .await?
                .workspace_wide
            {
                return Err(Error::Forbidden);
            }
        }
        // Configuration may have become stale since it was saved.
        validate_mapping(&mut tx, actor, form, &revision.mapping).await?;
        dao::activate(&mut tx, actor, current.id, &write).await?;
    } else {
        dao::deactivate(&mut tx, actor.workspace, current.id).await?;
    }
    reliability::audit(
        &mut tx,
        actor,
        &write,
        "forms.recipe.activation_changed",
        current.id,
        serde_json::json!({"active":input.active}),
    )
    .await?;
    let result = dao::get(&mut tx, actor.workspace, form, false).await?;
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
pub async fn runs(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    form: Uuid,
    query: RunQuery,
) -> Result<RunPage, Error> {
    let limit = super::page_limit(query.limit)?;
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "forms.submissions.read").await?;
    let recipe = dao::get(&mut tx, actor.workspace, form, false).await?;
    if let Some(after) = query.after {
        dao::run(&mut tx, actor.workspace, recipe.id, after).await?;
    }
    let mut items = dao::runs(&mut tx, actor.workspace, recipe.id, query.after, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    redact(&mut tx, actor, &mut items).await?;
    Ok(RunPage { items, next_cursor })
}
/// Run references are visible only with the matching workspace-wide reads.
async fn redact(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    runs: &mut [RunSummary],
) -> Result<(), Error> {
    async fn wide(
        db: &mut PgConnection,
        actor: &ActorContext<'_>,
        action: &str,
    ) -> Result<bool, Error> {
        match access::require(db, actor.identity, actor.workspace, action).await {
            Ok(a) => Ok(a.workspace_wide),
            Err(Error::Forbidden) => Ok(false),
            Err(error) => Err(error),
        }
    }
    let contacts = wide(db, actor, "contacts.read").await?;
    let intakes = wide(db, actor, "contacts.intake.read").await?;
    let opportunities = wide(db, actor, "crm.opportunities.read").await?
        && modules::enabled(db, actor.workspace, "crm").await?;
    for run in runs {
        if !contacts {
            run.contact_id = None;
        }
        if !intakes {
            run.intake_id = None;
        }
        if !opportunities {
            run.opportunity_id = None;
        }
    }
    Ok(())
}
pub(super) async fn attach_runs(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    submissions: &mut [Submission],
) -> Result<(), Error> {
    let ids: Vec<Uuid> = submissions.iter().map(|s| s.id).collect();
    let mut runs = dao::for_submissions(db, actor.workspace, &ids).await?;
    redact(db, actor, &mut runs).await?;
    for submission in submissions {
        submission.recipe_run = runs
            .iter()
            .find(|r| r.submission_id == submission.id)
            .cloned();
    }
    Ok(())
}
/// Continue a run after its intake was reviewed, or resume its failed job.
pub async fn retry(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    form: Uuid,
    run: Uuid,
    key: &str,
    input: RetryRun,
) -> Result<RunSummary, Error> {
    let mut tx = pool.begin().await?;
    let write = mutation(&mut tx, actor, "forms.recipes.manage").await?;
    authorize(&mut tx, actor, "forms.submissions.read").await?;
    let operation = format!("forms.recipe.retry:{run}");
    let request = serde_json::to_value(&input).expect("retry serializes");
    let recipe = dao::get(&mut tx, actor.workspace, form, true).await?;
    if let Some(saved) = reliability::replay::<RunSummary>(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
    )
    .await?
    {
        return Ok(saved);
    }
    let current = dao::lock_run(&mut tx, actor.workspace, recipe.id, run).await?;
    if current.version != input.expected_version {
        return Err(Error::Conflict("Run changed; reload before retrying"));
    }
    match (current.state.as_str(), current.job_state.as_str()) {
        ("awaiting_review", "completed") => {
            let identity = dao::authority(&mut tx, actor.workspace, recipe.id).await?;
            let job = jobs::enqueue_recorded(
                &mut tx,
                actor.workspace,
                identity.user,
                identity.membership,
                &identity.issuer,
                &identity.subject,
                &format!("run:{run}:{}", current.version + 1),
                "forms.crm_inquiry",
            )
            .await?;
            dao::requeue(&mut tx, actor.workspace, run, job).await?;
        }
        (_, "blocked" | "exhausted") => {
            if !modules::enabled(&mut tx, actor.workspace, "crm").await? {
                return Err(Error::Conflict("Enable CRM before retrying this run"));
            }
            jobs::resume_in(&mut tx, actor.workspace, current.job_id).await?;
            dao::touch(&mut tx, actor.workspace, run).await?;
        }
        _ => {
            return Err(Error::Conflict(
                "Only runs awaiting review or with a failed job can be retried",
            ));
        }
    }
    reliability::audit(
        &mut tx,
        actor,
        &write,
        "forms.recipe_run.retried",
        run,
        serde_json::json!({}),
    )
    .await?;
    let mut result = vec![dao::run(&mut tx, actor.workspace, recipe.id, run).await?];
    redact(&mut tx, actor, &mut result).await?;
    let result = result.pop().expect("one run");
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
/// Called with the submission transaction: an active recipe commits its run
/// and durable job together with the answers.
pub(super) async fn start(
    db: &mut PgConnection,
    workspace: Uuid,
    form: Uuid,
    submission: Uuid,
) -> Result<(), Error> {
    let Some((recipe, revision, identity)) = dao::active(db, workspace, form).await? else {
        return Ok(());
    };
    let job = jobs::enqueue_recorded(
        db,
        workspace,
        identity.user,
        identity.membership,
        &identity.issuer,
        &identity.subject,
        &format!("submission:{submission}"),
        "forms.crm_inquiry",
    )
    .await?;
    dao::insert_run(db, workspace, recipe, revision, submission, job).await
}
pub(crate) async fn context(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    job: Uuid,
) -> Result<Option<RunContext>, Error> {
    access::require(
        db,
        actor.identity,
        actor.workspace,
        "forms.submissions.read",
    )
    .await?;
    dao::context(db, actor.workspace, job).await
}
fn text(answers: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<String> {
    answers
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(Into::into)
}
/// Worker step. Everything commits with job completion, so a retry after any
/// failure starts from the last committed run state without duplicating work.
pub(crate) async fn execute(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    job: Uuid,
) -> Result<(), Error> {
    let run = dao::run_for_job(db, actor.workspace, job).await?;
    if matches!(run.state.as_str(), "completed" | "dismissed") {
        return Ok(());
    }
    let submission =
        super::dao::submission(db, actor.workspace, run.form_id, run.submission_id).await?;
    let version =
        super::dao::version(db, actor.workspace, run.form_id, submission.version_id).await?;
    let mapping = run.mapping;
    // A mapping must still describe the version this respondent answered.
    for key in mapping
        .name_fields
        .iter()
        .chain(mapping.email_field.iter())
        .chain(mapping.phone_field.iter())
    {
        if !version.definition.fields.iter().any(|f| &f.key == key) {
            return Err(Error::Invalid(
                "The recipe maps a question missing from the submitted version",
            ));
        }
    }
    let answers = &submission.answers;
    let name: Vec<String> = mapping
        .name_fields
        .iter()
        .filter_map(|k| text(answers, k))
        .collect();
    let email = mapping.email_field.as_ref().and_then(|k| text(answers, k));
    let phone = mapping.phone_field.as_ref().and_then(|k| text(answers, k));
    let display_name = if name.is_empty() {
        email
            .clone()
            .or_else(|| phone.clone())
            .unwrap_or_else(|| "Form respondent".into())
    } else {
        name.join(" ")
    };
    let display_name: String = display_name.chars().take(200).collect();
    let intake = match run.intake_id {
        Some(id) => contacts::intake::get_in(db, actor, id).await?,
        None => {
            let mut identifiers = Vec::new();
            if let Some(email) = email {
                identifiers.push(IdentifierInput {
                    kind: "email".into(),
                    value: email,
                    country: None,
                    extension: None,
                });
            }
            if let Some(phone) = phone {
                identifiers.push(IdentifierInput {
                    kind: "phone".into(),
                    value: phone,
                    country: mapping.phone_country.clone(),
                    extension: None,
                });
            }
            // Intake links or creates; public answers never edit an existing contact.
            let intake = contacts::intake::submit_in(
                db,
                actor,
                &format!("forms-run:{}:intake", run.id),
                contacts::intake::SubmitIntake {
                    source_ref: format!("forms:submission:{}", submission.id),
                    proposed: ContactInput {
                        display_name: display_name.clone(),
                        notes: String::new(),
                        identifiers,
                        owner_membership_id: mapping.owner_membership_id,
                        allow_shared: false,
                    },
                    contact_id: None,
                },
            )
            .await?;
            dao::set_intake(db, actor.workspace, run.id, intake.id).await?;
            intake
        }
    };
    let contact = match (intake.state.as_str(), intake.contact_id) {
        ("resolved", Some(contact)) => contact,
        ("dismissed", _) => {
            return dao::finish(db, actor.workspace, run.id, "dismissed", None, None).await;
        }
        _ => {
            return dao::finish(db, actor.workspace, run.id, "awaiting_review", None, None).await;
        }
    };
    let opportunity = crm::save_opportunity_in(
        db,
        actor,
        None,
        None,
        &format!("forms-run:{}:opportunity", run.id),
        crm::OpportunityInput {
            title: format!("{}: {display_name}", mapping.title_prefix.trim())
                .chars()
                .take(200)
                .collect(),
            notes: format!(
                "Recorded from a response to the form \"{}\" (submission {}).",
                version.name, submission.id
            ),
            owner_membership_id: mapping.owner_membership_id,
            pipeline_id: mapping.pipeline_id,
            stage_id: mapping.stage_id,
            expected_value: None,
            currency_code: None,
            expected_close_on: None,
        },
    )
    .await?;
    crm::participants::create_in(
        db,
        actor,
        crm::participants::Target::OpportunityContact,
        opportunity.id,
        &format!("forms-run:{}:participant", run.id),
        crm::participants::NewParticipant {
            expected_version: opportunity.version,
            reference_id: contact,
            role_label: "Respondent".into(),
            is_primary: true,
        },
    )
    .await?;
    dao::finish(
        db,
        actor.workspace,
        run.id,
        "completed",
        Some(contact),
        Some(opportunity.id),
    )
    .await
}
