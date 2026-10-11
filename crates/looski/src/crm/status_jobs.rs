// SPDX-License-Identifier: AGPL-3.0-only
//! One bounded recipe: a material deal-status transition schedules a checklist.
mod dao;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    jobs, modules, reliability,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct CoordinationRule {
    pub id: Uuid,
    pub name: String,
    pub status_id: Uuid,
    pub revision_id: Uuid,
    pub archived: bool,
    pub version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveCoordinationRule {
    pub name: String,
    pub status_id: Uuid,
    pub revision_id: Uuid,
    pub archived: Option<bool>,
    pub expected_version: Option<i64>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct RuleQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
    pub archived: Option<bool>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct CoordinationRulePage {
    pub items: Vec<CoordinationRule>,
    pub next_cursor: Option<Uuid>,
}
async fn require(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    action: &str,
) -> Result<access::Access, Error> {
    let a = super::authorize(db, actor, action).await?;
    crate::tasks::configuration::require_read(db, actor).await?;
    if !a.workspace_wide {
        return Err(Error::Forbidden);
    }
    Ok(a)
}
pub async fn list(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    q: RuleQuery,
) -> Result<CoordinationRulePage, Error> {
    let n = q.limit.unwrap_or(25);
    if !(1..=100).contains(&n) {
        return Err(Error::Invalid("Rule page limit must be 1–100"));
    }
    let mut tx = pool.begin().await?;
    require(&mut tx, actor, "crm.configuration.read").await?;
    let mut items = dao::list(&mut tx, actor.workspace, &q, n + 1).await?;
    let next_cursor = if items.len() > n as usize {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    Ok(CoordinationRulePage { items, next_cursor })
}
pub async fn get(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<CoordinationRule, Error> {
    let mut tx = pool.begin().await?;
    let result = get_in_transaction(&mut tx, actor, id).await?;
    Ok(result)
}
pub(crate) async fn get_in_transaction(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<CoordinationRule, Error> {
    require(db, actor, "crm.configuration.read").await?;
    dao::get(db, actor.workspace, id).await
}

pub async fn save(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Option<Uuid>,
    key: &str,
    input: SaveCoordinationRule,
) -> Result<CoordinationRule, Error> {
    let mut tx = pool.begin().await?;
    let result = save_in_transaction(&mut tx, actor, id, key, input).await?;
    tx.commit().await?;
    Ok(result)
}
pub(crate) async fn save_in_transaction(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Option<Uuid>,
    key: &str,
    mut input: SaveCoordinationRule,
) -> Result<CoordinationRule, Error> {
    let request = serde_json::to_value(&input).expect("rule serializes");
    input.name = input.name.trim().into();
    if input.name.is_empty()
        || input.name.chars().count() > 120
        || input.name.chars().any(char::is_control)
    {
        return Err(Error::Invalid("Name must be 1–120 printable characters"));
    }
    if id.is_some() != input.expected_version.is_some() {
        return Err(Error::Invalid(
            "Updates require expected_version; creation must omit it",
        ));
    }
    require(db, actor, "crm.configuration.manage").await?;
    modules::lock_workspace(db, actor.workspace).await?;
    let a = require(db, actor, "crm.configuration.manage").await?;
    require(db, actor, "crm.configuration.read").await?;
    let operation = format!(
        "crm.coordination_rule.save:{}",
        id.map(|i| i.to_string()).unwrap_or_else(|| "new".into())
    );
    if let Some(saved) =
        reliability::replay(db, actor.workspace, &a, &operation, key, &request).await?
    {
        return Ok(saved);
    }
    let current = if let Some(id) = id {
        let r = dao::get(db, actor.workspace, id).await?;
        if Some(r.version) != input.expected_version {
            return Err(Error::Conflict("Rule changed; reload before editing"));
        }
        Some(r)
    } else {
        None
    };
    let archived = input
        .archived
        .unwrap_or_else(|| current.as_ref().is_some_and(|r| r.archived));
    // Retirement remains possible even when referenced definitions are retired.
    if !archived
        || current
            .as_ref()
            .is_none_or(|r| r.revision_id != input.revision_id || r.status_id != input.status_id)
    {
        super::coordination::require_active_revision(db, actor, input.revision_id).await?;
        dao::active_status(db, actor.workspace, input.status_id).await?;
    }
    if !archived && dao::conflicting(db, actor.workspace, id, input.status_id).await? {
        return Err(Error::Conflict(
            "This status already has an active coordination rule",
        ));
    }
    let id = id.unwrap_or_else(Uuid::new_v4);
    dao::save(db, actor.workspace, id, &input, archived, current.is_none()).await?;
    reliability::audit(db,actor,&a,"crm.coordination_rule.saved",id,serde_json::json!({"status_id":input.status_id,"revision_id":input.revision_id,"archived":archived})).await?;
    let result = dao::get(db, actor.workspace, id).await?;
    reliability::save_replay(db, actor.workspace, &a, &operation, key, &request, &result).await?;
    Ok(result)
}

pub(super) async fn active_status(
    db: &mut PgConnection,
    workspace: Uuid,
    status: Uuid,
) -> Result<(), Error> {
    dao::active_status(db, workspace, status).await
}
pub(super) async fn schedule(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    a: &access::Access,
    deal: Uuid,
    status: Uuid,
    audit: Uuid,
) -> Result<(), Error> {
    if let Some(rule) = dao::active_rule(db, actor.workspace, status).await? {
        let job = jobs::enqueue(db, actor, a, &format!("{}:{audit}", rule.id)).await?;
        dao::insert_run(db, actor.workspace, job, deal, &rule, audit).await?;
    }
    Ok(())
}
pub(crate) async fn execute(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    job: Uuid,
) -> Result<(), Error> {
    let run = dao::run(db, actor.workspace, job).await?;
    let deal = super::read_deal(db, actor, run.deal).await?;
    if deal.archived
        || deal.status_id != run.status
        || !dao::is_current_transition(db, actor.workspace, run.deal, run.audit).await?
    {
        return Err(Error::Conflict(
            "The triggering deal status is no longer current",
        ));
    }
    dao::active_status(db, actor.workspace, run.status).await?;
    let instance = super::coordination::apply_in_transaction(
        db,
        actor,
        run.deal,
        &format!("job:{job}"),
        super::coordination::ApplyCoordination {
            expected_version: deal.version,
            revision_id: run.revision,
            base_on: run.base_on,
        },
    )
    .await?;
    dao::complete_run(db, actor.workspace, job, instance.id).await
}

#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct JobContext {
    pub deal_id: Uuid,
    pub deal_title: String,
    pub instance_id: Option<Uuid>,
}
pub(crate) async fn context(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    job: Uuid,
) -> Result<Option<JobContext>, Error> {
    let Some((id, instance_id)) = dao::context(db, actor.workspace, job).await? else {
        return Ok(None);
    };
    match super::read_deal(db, actor, id).await {
        Ok(deal) => Ok(Some(JobContext {
            deal_id: id,
            deal_title: deal.title,
            instance_id,
        })),
        Err(Error::Forbidden | Error::NotFound) => Ok(None),
        Err(error) => Err(error),
    }
}
