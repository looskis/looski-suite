// SPDX-License-Identifier: AGPL-3.0-only
//! Pinned, bounded task checklists over ordinary independently authorized Tasks.
mod dao;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    modules, reliability, tasks,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
#[derive(PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CoordinationStepInput {
    pub label: String,
    pub task_template_revision_id: Uuid,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct CoordinationStep {
    pub id: Uuid,
    pub position: i32,
    #[serde(flatten)]
    pub fields: CoordinationStepInput,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct CoordinationRevision {
    pub id: Uuid,
    pub template_id: Uuid,
    pub number: i64,
    pub name: String,
    pub items: Vec<CoordinationStep>,
    pub created_at: String,
    pub audit_event_id: Uuid,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct CoordinationTemplate {
    pub id: Uuid,
    pub name: String,
    pub version: i64,
    pub archived: bool,
    pub revision: CoordinationRevision,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveCoordinationTemplate {
    pub name: String,
    pub items: Vec<CoordinationStepInput>,
    pub archived: Option<bool>,
    pub expected_version: Option<i64>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct CoordinationQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
    pub archived: Option<bool>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct CoordinationPageQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct CoordinationTemplatePage {
    pub items: Vec<CoordinationTemplate>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct CoordinationRevisionPage {
    pub items: Vec<CoordinationRevision>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplyCoordination {
    pub expected_version: i64,
    pub revision_id: Uuid,
    pub base_on: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct CoordinationItem {
    pub id: Uuid,
    pub label: String,
    pub link_id: Uuid,
    pub link_ended: bool,
    pub task: tasks::Task,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct CoordinationInstance {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub revision_id: Uuid,
    pub template_name: String,
    pub base_on: String,
    pub created_at: String,
    pub items: Vec<CoordinationItem>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct CoordinationInstanceSummary {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub revision_id: Uuid,
    pub template_name: String,
    pub base_on: String,
    pub created_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct CoordinationInstancePage {
    pub items: Vec<CoordinationInstanceSummary>,
    pub next_cursor: Option<Uuid>,
}
fn limit(n: Option<i64>) -> Result<i64, Error> {
    let n = n.unwrap_or(20);
    if !(1..=25).contains(&n) {
        return Err(Error::Invalid("Coordination page limit must be 1–25"));
    }
    Ok(n)
}
async fn config(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    action: &str,
) -> Result<access::Access, Error> {
    let a = super::authorize(db, actor, action).await?;
    if !a.workspace_wide {
        return Err(Error::Forbidden);
    }
    tasks::configuration::require_read(db, actor).await?;
    Ok(a)
}
pub async fn templates(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    q: CoordinationQuery,
) -> Result<CoordinationTemplatePage, Error> {
    let n = limit(q.limit)?;
    let mut tx = pool.begin().await?;
    config(&mut tx, actor, "crm.configuration.read").await?;
    let mut items = dao::templates(&mut tx, actor.workspace, &q, n + 1).await?;
    let next_cursor = if items.len() > n as usize {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    Ok(CoordinationTemplatePage { items, next_cursor })
}
pub async fn template(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<CoordinationTemplate, Error> {
    let mut tx = pool.begin().await?;
    config(&mut tx, actor, "crm.configuration.read").await?;
    dao::template(&mut tx, actor.workspace, id).await
}
pub async fn revisions(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    q: CoordinationPageQuery,
) -> Result<CoordinationRevisionPage, Error> {
    let n = limit(q.limit)?;
    let mut tx = pool.begin().await?;
    config(&mut tx, actor, "crm.configuration.read").await?;
    dao::template(&mut tx, actor.workspace, id).await?;
    let mut items = dao::revisions(&mut tx, actor.workspace, id, &q, n + 1).await?;
    let next_cursor = if items.len() > n as usize {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    Ok(CoordinationRevisionPage { items, next_cursor })
}
pub async fn save(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Option<Uuid>,
    key: &str,
    mut input: SaveCoordinationTemplate,
) -> Result<CoordinationTemplate, Error> {
    let request = serde_json::to_value(&input).expect("coordination serializes");
    input.name = input.name.trim().to_string();
    if input.name.is_empty()
        || input.name.chars().count() > 120
        || input.name.chars().any(char::is_control)
        || input.items.is_empty()
        || input.items.len() > 25
    {
        return Err(Error::Invalid(
            "Name must be 1–120 printable characters; provide 1–25 checklist items",
        ));
    }
    for step in &mut input.items {
        step.label = step.label.trim().to_string();
        if step.label.is_empty()
            || step.label.chars().count() > 200
            || step.label.chars().any(char::is_control)
        {
            return Err(Error::Invalid(
                "Checklist labels must be 1–200 printable characters",
            ));
        }
    }
    if id.is_some() != input.expected_version.is_some() {
        return Err(Error::Invalid(
            "Updates require expected_version; creation must omit it",
        ));
    }
    let mut tx = pool.begin().await?;
    config(&mut tx, actor, "crm.configuration.manage").await?;
    modules::lock_workspace(&mut tx, actor.workspace).await?;
    let a = config(&mut tx, actor, "crm.configuration.manage").await?;
    config(&mut tx, actor, "crm.configuration.read").await?;
    let operation = format!(
        "crm.coordination.save:{}",
        id.map(|i| i.to_string()).unwrap_or_else(|| "new".into())
    );
    if let Some(saved) =
        reliability::replay(&mut tx, actor.workspace, &a, &operation, key, &request).await?
    {
        return Ok(saved);
    }
    let current = if let Some(id) = id {
        let t = dao::template(&mut tx, actor.workspace, id).await?;
        if Some(t.version) != input.expected_version {
            return Err(Error::Conflict(
                "Coordination template changed; reload before editing",
            ));
        }
        Some(t)
    } else {
        None
    };
    let changed = current.as_ref().is_none_or(|t| {
        t.revision.items.len() != input.items.len()
            || t.revision
                .items
                .iter()
                .zip(&input.items)
                .any(|(old, new)| old.fields != *new)
    });
    if changed {
        tasks::configuration::validate_revisions(
            &mut tx,
            actor,
            &input
                .items
                .iter()
                .map(|i| i.task_template_revision_id)
                .collect::<Vec<_>>(),
        )
        .await?;
    }
    let id = id.unwrap_or_else(Uuid::new_v4);
    let number = current
        .as_ref()
        .map(|t| t.revision.number + i64::from(changed))
        .unwrap_or(1);
    let audit = reliability::audit(
        &mut tx,
        actor,
        &a,
        "crm.coordination_template.saved",
        id,
        serde_json::json!({"revision":number,"new_revision":changed}),
    )
    .await?;
    dao::save(
        &mut tx,
        actor.workspace,
        id,
        &input,
        number,
        current.is_none(),
    )
    .await?;
    if changed {
        dao::insert_revision(&mut tx, actor.workspace, id, number, &input, audit).await?;
    }
    let result = dao::template(&mut tx, actor.workspace, id).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &a,
        &operation,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
async fn project_rows(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    rows: Vec<dao::InstanceRow>,
) -> Result<Vec<CoordinationInstance>, Error> {
    let ids = rows
        .iter()
        .flat_map(|r| r.items.iter().map(|i| i.task_id))
        .collect::<Vec<_>>();
    let mut tasks = std::collections::HashMap::new();
    for batch in ids.chunks(100) {
        tasks.extend(tasks::project_many(db, actor, batch).await?);
    }
    Ok(rows
        .into_iter()
        .map(|row| CoordinationInstance {
            id: row.id,
            deal_id: row.deal_id,
            revision_id: row.revision_id,
            template_name: row.template_name,
            base_on: row.base_on,
            created_at: row.created_at,
            items: row
                .items
                .into_iter()
                .filter_map(|i| {
                    tasks.remove(&i.task_id).map(|task| CoordinationItem {
                        id: i.id,
                        label: i.label,
                        link_id: i.link_id,
                        link_ended: i.link_ended,
                        task,
                    })
                })
                .collect(),
        })
        .collect())
}
pub async fn instance(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    deal: Uuid,
    id: Uuid,
) -> Result<CoordinationInstance, Error> {
    let mut tx = pool.begin().await?;
    super::read_deal(&mut tx, actor, deal).await?;
    let row = dao::instance(&mut tx, actor.workspace, deal, id).await?;
    Ok(project_rows(&mut tx, actor, vec![row])
        .await?
        .pop()
        .expect("one instance"))
}
pub async fn instances(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    deal: Uuid,
    q: CoordinationPageQuery,
) -> Result<CoordinationInstancePage, Error> {
    let n = limit(q.limit)?;
    let mut tx = pool.begin().await?;
    super::read_deal(&mut tx, actor, deal).await?;
    let mut rows = dao::instances(&mut tx, actor.workspace, deal, &q, n + 1).await?;
    let next_cursor = if rows.len() > n as usize {
        rows.pop();
        rows.last().map(|r| r.id)
    } else {
        None
    };
    let items = rows;
    Ok(CoordinationInstancePage { items, next_cursor })
}
pub async fn apply(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    deal: Uuid,
    key: &str,
    input: ApplyCoordination,
) -> Result<CoordinationInstance, Error> {
    let mut tx = pool.begin().await?;
    let result = apply_in_transaction(&mut tx, actor, deal, key, input).await?;
    tx.commit().await?;
    Ok(result)
}
pub(crate) async fn apply_in_transaction(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    deal: Uuid,
    key: &str,
    input: ApplyCoordination,
) -> Result<CoordinationInstance, Error> {
    let (write, read) =
        super::mutation(db, actor, "crm.deals.coordination.apply", "crm.deals.read").await?;
    config(db, actor, "crm.configuration.read").await?;
    let parent = super::read_deal(db, actor, deal).await?;
    super::record_scope(&write, parent.owner_membership_id)?;
    super::record_scope(&read, parent.owner_membership_id)?;
    let manage = super::authorize(db, actor, "crm.deals.tasks.manage").await?;
    super::record_scope(&manage, parent.owner_membership_id)?;
    let operation = format!("crm.coordination.apply:{deal}");
    let request = serde_json::to_value(&input).expect("application serializes");
    if let Some(saved) = reliability::replay::<CoordinationInstance>(
        db,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
    )
    .await?
    {
        for item in &saved.items {
            tasks::require_creation_replay(db, actor, item.task.id).await?;
        }
        return Ok(saved);
    }
    if parent.version != input.expected_version || parent.archived {
        return Err(Error::Conflict(
            "Deal changed or is archived; reload before applying coordination",
        ));
    }
    let revision = dao::revision(db, actor.workspace, input.revision_id).await?;
    let template = dao::template(db, actor.workspace, revision.template_id).await?;
    if template.archived {
        return Err(Error::Conflict(
            "Archived coordination templates cannot create new work",
        ));
    }
    let id = Uuid::new_v4();
    let audit = reliability::audit(
        db,
        actor,
        &write,
        "crm.coordination.applied",
        deal,
        serde_json::json!({"instance_id":id,"revision_id":revision.id}),
    )
    .await?;
    // Task creation validates the base date before instance persistence.
    for step in &revision.items {
        let task = tasks::configuration::instantiate_revision(
            db,
            actor,
            step.fields.task_template_revision_id,
            &input.base_on,
            Some(parent.owner_membership_id),
        )
        .await?;
        super::task_links::attach_created(db, actor, deal, task.id).await?;
        if step.position == 0 {
            dao::insert_instance(
                db,
                actor.workspace,
                id,
                deal,
                &input,
                write.membership_id,
                audit,
            )
            .await?;
        }
        dao::insert_item(db, actor.workspace, id, deal, revision.id, step.id, task.id).await?;
    }
    let row = dao::instance(db, actor.workspace, deal, id).await?;
    let result = project_rows(db, actor, vec![row])
        .await?
        .pop()
        .expect("one instance");
    reliability::save_replay(
        db,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
        &result,
    )
    .await?;
    Ok(result)
}

pub(super) async fn require_active_revision(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<(), Error> {
    config(db, actor, "crm.configuration.read").await?;
    let revision = dao::revision(db, actor.workspace, id).await?;
    if dao::template(db, actor.workspace, revision.template_id)
        .await?
        .archived
    {
        return Err(Error::Conflict("Coordination template is archived"));
    }
    Ok(())
}
