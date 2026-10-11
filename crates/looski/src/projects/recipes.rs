// SPDX-License-Identifier: AGPL-3.0-only
//! Built-in deal-to-project recipe. A deal reaching a configured status commits a
//! conversion and durable job with the status change; the worker reads the deal
//! through CRM and creates the project here, under the authority recorded when an
//! administrator activated the recipe. One conversion per recipe and deal, always.
mod dao;
use super::{SaveProject, authorize, mutation};
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    crm, jobs, modules, reliability,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DealRecipeConfiguration {
    /// The deal status that marks the milestone.
    pub status_id: Uuid,
    /// Project names read `prefix: deal title`; empty uses the deal title alone.
    #[serde(default)]
    pub name_prefix: String,
    /// Add the deal's owner to the project.
    #[serde(default)]
    pub include_deal_owner: bool,
    /// Up to 20 project members.
    #[serde(default)]
    pub member_ids: Vec<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DealRecipeRevision {
    pub id: Uuid,
    pub number: i32,
    pub configuration: DealRecipeConfiguration,
    pub created_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DealRecipe {
    pub id: Uuid,
    pub name: String,
    pub active: bool,
    pub revision: Option<DealRecipeRevision>,
    /// Member whose current permissions each run uses.
    pub activated_by_membership_id: Option<Uuid>,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveDealRecipe {
    pub name: String,
    pub configuration: DealRecipeConfiguration,
    /// Required for updates; omitted on creation.
    pub expected_version: Option<i64>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SetDealRecipeActive {
    pub expected_version: i64,
    pub active: bool,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DealRecipePage {
    pub items: Vec<DealRecipe>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DealConversion {
    pub id: Uuid,
    pub recipe_id: Uuid,
    /// queued or completed; failures are job states.
    pub state: String,
    pub revision_number: i32,
    /// Only for members who can read that deal.
    pub deal_id: Option<Uuid>,
    /// Only for members who can read that project.
    pub project_id: Option<Uuid>,
    pub job_id: Uuid,
    pub job_state: String,
    pub error_code: Option<String>,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct DealConversionQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DealConversionPage {
    pub items: Vec<DealConversion>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RetryDealConversion {
    pub expected_version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DealConversionContext {
    pub run_id: Uuid,
    pub recipe_id: Uuid,
}

/// Permissions a run uses; activation requires them workspace-wide, and each run
/// re-checks them, so a revoked grant blocks the work visibly.
const RUN_PERMISSIONS: &[&str] = &[
    "projects.recipes.manage",
    "projects.read",
    "projects.create",
    "projects.members.manage",
    "crm.deals.read",
];

async fn wide(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    action: &str,
) -> Result<access::Access, Error> {
    let a = authorize(db, actor, action).await?;
    if !a.workspace_wide {
        return Err(Error::Forbidden);
    }
    Ok(a)
}
async fn validate(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    configuration: &DealRecipeConfiguration,
) -> Result<(), Error> {
    if configuration.name_prefix.chars().count() > 80
        || configuration.name_prefix.chars().any(char::is_control)
    {
        return Err(Error::Invalid(
            "Name prefix must be at most 80 printable characters",
        ));
    }
    let mut members = configuration.member_ids.clone();
    members.sort_unstable();
    members.dedup();
    if members.len() != configuration.member_ids.len() || members.len() > 20 {
        return Err(Error::Invalid("List at most 20 distinct members"));
    }
    for member in &members {
        access::require_active_member(db, actor.workspace, *member).await?;
    }
    crm::milestone_status(db, actor, configuration.status_id).await
}
fn validate_name(name: &str) -> Result<(), Error> {
    if name.trim().is_empty() || name.chars().count() > 120 || name.chars().any(char::is_control) {
        return Err(Error::Invalid("Name must be 1–120 printable characters"));
    }
    Ok(())
}
pub async fn list(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: DealConversionQuery,
) -> Result<DealRecipePage, Error> {
    let limit = super::page_limit(query.limit)?;
    let mut tx = pool.begin().await?;
    wide(&mut tx, actor, "projects.recipes.manage").await?;
    if let Some(after) = query.after {
        dao::get(&mut tx, actor.workspace, after, false).await?;
    }
    let mut items = dao::list(&mut tx, actor.workspace, query.after, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    Ok(DealRecipePage { items, next_cursor })
}
pub async fn get(pool: &PgPool, actor: &ActorContext<'_>, id: Uuid) -> Result<DealRecipe, Error> {
    let mut tx = pool.begin().await?;
    wide(&mut tx, actor, "projects.recipes.manage").await?;
    dao::get(&mut tx, actor.workspace, id, false).await
}
pub async fn save(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Option<Uuid>,
    key: &str,
    input: SaveDealRecipe,
) -> Result<DealRecipe, Error> {
    validate_name(&input.name)?;
    if id.is_some() != input.expected_version.is_some() {
        return Err(Error::Invalid(
            "Updates require expected_version; creation must omit it",
        ));
    }
    let mut tx = pool.begin().await?;
    mutation(&mut tx, actor, "projects.recipes.manage", None).await?;
    let write = wide(&mut tx, actor, "projects.recipes.manage").await?;
    let operation = match id {
        Some(id) => format!("projects.recipe.update:{id}"),
        None => "projects.recipe.create".to_string(),
    };
    let request = serde_json::to_value(&input).expect("recipe serializes");
    if let Some(saved) = reliability::replay::<DealRecipe>(
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
    let existing = match id {
        Some(id) => {
            let current = dao::get(&mut tx, actor.workspace, id, true).await?;
            if Some(current.version) != input.expected_version {
                return Err(Error::Conflict("DealRecipe changed; reload before editing"));
            }
            Some(current)
        }
        None => None,
    };
    validate(&mut tx, actor, &input.configuration).await?;
    let recipe = match existing {
        Some(current) => {
            if current.active
                && dao::conflicting(
                    &mut tx,
                    actor.workspace,
                    current.id,
                    input.configuration.status_id,
                )
                .await?
            {
                return Err(Error::Conflict(
                    "Another active recipe already uses this status",
                ));
            }
            current.id
        }
        None => dao::insert(&mut tx, actor.workspace, input.name.trim()).await?,
    };
    dao::revise(
        &mut tx,
        actor.workspace,
        recipe,
        input.name.trim(),
        &input.configuration,
        write.membership_id,
    )
    .await?;
    reliability::audit(
        &mut tx,
        actor,
        &write,
        "projects.recipe.saved",
        recipe,
        serde_json::json!({"status_id":input.configuration.status_id}),
    )
    .await?;
    let result = dao::get(&mut tx, actor.workspace, recipe, false).await?;
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
/// Activation records the activator's authority for future runs; it must already
/// include every action a run performs. Deactivation stops new conversions only.
pub async fn set_active(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: SetDealRecipeActive,
) -> Result<DealRecipe, Error> {
    let mut tx = pool.begin().await?;
    mutation(&mut tx, actor, "projects.recipes.manage", None).await?;
    let write = wide(&mut tx, actor, "projects.recipes.manage").await?;
    let operation = format!("projects.recipe.active:{id}");
    let request = serde_json::to_value(&input).expect("recipe serializes");
    if let Some(saved) = reliability::replay::<DealRecipe>(
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
    let current = dao::get(&mut tx, actor.workspace, id, true).await?;
    if current.version != input.expected_version {
        return Err(Error::Conflict("DealRecipe changed; reload before editing"));
    }
    if input.active {
        let Some(revision) = &current.revision else {
            return Err(Error::Conflict("Configure the recipe before activating it"));
        };
        for action in RUN_PERMISSIONS {
            wide_any(&mut tx, actor, action).await?;
        }
        validate(&mut tx, actor, &revision.configuration).await?;
        if dao::conflicting(
            &mut tx,
            actor.workspace,
            id,
            revision.configuration.status_id,
        )
        .await?
        {
            return Err(Error::Conflict(
                "Another active recipe already uses this status",
            ));
        }
        dao::activate(&mut tx, actor, id, &write).await?;
    } else {
        dao::deactivate(&mut tx, actor.workspace, id).await?;
    }
    reliability::audit(
        &mut tx,
        actor,
        &write,
        "projects.recipe.activation_changed",
        id,
        serde_json::json!({"active":input.active}),
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
/// A run permission from any application, held workspace-wide.
async fn wide_any(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    action: &str,
) -> Result<(), Error> {
    if !access::require(db, actor.identity, actor.workspace, action)
        .await?
        .workspace_wide
    {
        return Err(Error::Forbidden);
    }
    Ok(())
}
async fn redact(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    runs: &mut [DealConversion],
) -> Result<(), Error> {
    for run in runs {
        if let Some(deal) = run.deal_id {
            match crm::read_deal(db, actor, deal).await {
                Ok(_) => {}
                Err(Error::Forbidden | Error::NotFound) => run.deal_id = None,
                Err(error) => return Err(error),
            }
        }
        if let Some(project) = run.project_id {
            match super::reference(db, actor, project).await {
                Ok(_) => {}
                Err(Error::Forbidden | Error::NotFound) => run.project_id = None,
                Err(error) => return Err(error),
            }
        }
    }
    Ok(())
}
pub async fn runs(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    recipe: Uuid,
    query: DealConversionQuery,
) -> Result<DealConversionPage, Error> {
    let limit = super::page_limit(query.limit)?;
    let mut tx = pool.begin().await?;
    wide(&mut tx, actor, "projects.recipes.manage").await?;
    dao::get(&mut tx, actor.workspace, recipe, false).await?;
    if let Some(after) = query.after {
        dao::run(&mut tx, actor.workspace, recipe, after).await?;
    }
    let mut items = dao::runs(&mut tx, actor.workspace, recipe, query.after, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    redact(&mut tx, actor, &mut items).await?;
    Ok(DealConversionPage { items, next_cursor })
}
/// Resume a blocked or exhausted conversion; it stays the same conversion.
pub async fn retry(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    recipe: Uuid,
    run: Uuid,
    key: &str,
    input: RetryDealConversion,
) -> Result<DealConversion, Error> {
    let mut tx = pool.begin().await?;
    mutation(&mut tx, actor, "projects.recipes.manage", None).await?;
    let write = wide(&mut tx, actor, "projects.recipes.manage").await?;
    let operation = format!("projects.recipe.retry:{run}");
    let request = serde_json::to_value(&input).expect("retry serializes");
    if let Some(saved) = reliability::replay::<DealConversion>(
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
    let current = dao::run(&mut tx, actor.workspace, recipe, run).await?;
    if current.version != input.expected_version {
        return Err(Error::Conflict(
            "DealConversion changed; reload before retrying",
        ));
    }
    if !["blocked", "exhausted"].contains(&current.job_state.as_str()) {
        return Err(Error::Conflict(
            "Only a run with a failed job can be retried",
        ));
    }
    if !modules::enabled(&mut tx, actor.workspace, "crm").await? {
        return Err(Error::Conflict("Turn on CRM before retrying this run"));
    }
    jobs::resume_in(&mut tx, actor.workspace, current.job_id).await?;
    dao::touch(&mut tx, actor.workspace, run).await?;
    reliability::audit(
        &mut tx,
        actor,
        &write,
        "projects.recipe_run.retried",
        run,
        serde_json::json!({}),
    )
    .await?;
    let mut result = vec![dao::run(&mut tx, actor.workspace, recipe, run).await?];
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
/// Called by CRM with a deal's status change. Runs whether or not Projects is enabled:
/// with Projects off the conversion commits and its job shows as blocked, so the
/// milestone is not lost. Reaching the status again finds the existing conversion.
pub(crate) async fn deal_reached_status(
    db: &mut PgConnection,
    workspace: Uuid,
    deal: Uuid,
    status: Uuid,
) -> Result<(), Error> {
    let Some((recipe, revision, identity)) = dao::active_for_status(db, workspace, status).await?
    else {
        return Ok(());
    };
    if dao::converted(db, workspace, recipe, deal).await? {
        return Ok(());
    }
    let job = jobs::enqueue_recorded(
        db,
        workspace,
        identity.user,
        identity.membership,
        &identity.issuer,
        &identity.subject,
        &format!("recipe:{recipe}:deal:{deal}"),
        "projects.deal_conversion",
    )
    .await?;
    dao::insert_run(db, workspace, recipe, revision, deal, job).await
}
pub(crate) async fn context(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    job: Uuid,
) -> Result<Option<DealConversionContext>, Error> {
    match access::require(
        db,
        actor.identity,
        actor.workspace,
        "projects.recipes.manage",
    )
    .await
    {
        Ok(_) => dao::context(db, actor.workspace, job).await,
        Err(Error::Forbidden) => Ok(None),
        Err(error) => Err(error),
    }
}
/// Worker step. The project commits with job completion, so a retry after any failure
/// starts from the last committed state and cannot leave or add a second project.
pub(crate) async fn execute(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    job: Uuid,
) -> Result<(), Error> {
    let run = dao::run_for_job(db, actor.workspace, job).await?;
    if run.state == "completed" {
        return Ok(());
    }
    for action in RUN_PERMISSIONS {
        wide_any(db, actor, action).await?;
    }
    let write = wide(db, actor, "projects.create").await?;
    let deal = crm::read_deal(db, actor, run.deal_id).await?;
    if deal.archived {
        return Err(Error::Conflict("The deal was archived before conversion"));
    }
    let configuration = run.configuration;
    let mut members = configuration.member_ids;
    for member in &members {
        access::require_active_member(db, actor.workspace, *member).await?;
    }
    if configuration.include_deal_owner
        && !members.contains(&deal.owner_membership_id)
        && access::require_active_member(db, actor.workspace, deal.owner_membership_id)
            .await
            .is_ok()
    {
        members.push(deal.owner_membership_id);
    }
    let prefix = configuration.name_prefix.trim();
    let name = if prefix.is_empty() {
        deal.title.clone()
    } else {
        format!("{prefix}: {}", deal.title)
    };
    let input = SaveProject {
        name: name.chars().take(120).collect(),
        description: String::new(),
        status: Default::default(),
        due_on: None,
        expected_version: None,
    };
    let project = super::insert(db, actor, &write, &input, &members).await?;
    dao::complete(db, actor.workspace, run.id, project).await
}
