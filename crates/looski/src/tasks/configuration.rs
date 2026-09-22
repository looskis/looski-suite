// SPDX-License-Identifier: AGPL-3.0-only
//! Stable task types and immutable reusable work specifications.
mod dao;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    modules, reliability,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TaskType {
    pub id: Uuid,
    pub name: String,
    pub position: i32,
    pub archived: bool,
    pub version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveTaskType {
    pub name: String,
    pub position: Option<i32>,
    pub archived: Option<bool>,
    pub expected_version: Option<i64>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct TaskConfigurationQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
    pub archived: Option<bool>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct TaskRevisionQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TaskTypePage {
    pub items: Vec<TaskType>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentRule {
    Initiator,
    ContextOwner,
    Fixed,
}
impl AssignmentRule {
    fn key(self) -> &'static str {
        match self {
            Self::Initiator => "initiator",
            Self::ContextOwner => "context_owner",
            Self::Fixed => "fixed",
        }
    }
}
#[derive(PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TemplateFields {
    pub title: String,
    #[serde(default)]
    pub notes: String,
    pub type_id: Option<Uuid>,
    pub assignment: AssignmentRule,
    pub fixed_membership_id: Option<Uuid>,
    pub due_offset_days: i32,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TaskTemplateRevision {
    pub id: Uuid,
    pub template_id: Uuid,
    pub number: i64,
    #[serde(flatten)]
    pub fields: TemplateFields,
    pub type_name: Option<String>,
    pub created_at: String,
    pub audit_event_id: Uuid,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TaskTemplate {
    pub id: Uuid,
    pub name: String,
    pub archived: bool,
    pub version: i64,
    pub revision: TaskTemplateRevision,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct SaveTaskTemplate {
    pub name: String,
    pub archived: Option<bool>,
    pub expected_version: Option<i64>,
    #[serde(flatten)]
    pub fields: TemplateFields,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TaskTemplatePage {
    pub items: Vec<TaskTemplate>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TaskRevisionPage {
    pub items: Vec<TaskTemplateRevision>,
    pub next_cursor: Option<Uuid>,
}
fn name(value: &str) -> Result<(), Error> {
    if value.trim().is_empty() || value.chars().count() > 120 || value.chars().any(char::is_control)
    {
        return Err(Error::Invalid("Name must be 1–120 printable characters"));
    }
    Ok(())
}
async fn require(
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
async fn mutation(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
) -> Result<access::Access, Error> {
    require(db, actor, "tasks.configuration.manage").await?;
    modules::lock_workspace(db, actor.workspace).await?;
    let a = require(db, actor, "tasks.configuration.manage").await?;
    require(db, actor, "tasks.configuration.read").await?;
    Ok(a)
}
pub async fn types(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    q: TaskConfigurationQuery,
) -> Result<TaskTypePage, Error> {
    let n = super::limit(q.limit)?;
    let mut tx = pool.begin().await?;
    require(&mut tx, actor, "tasks.configuration.read").await?;
    let mut items = dao::types(&mut tx, actor.workspace, &q, n + 1).await?;
    let next_cursor = if items.len() > n as usize {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    Ok(TaskTypePage { items, next_cursor })
}
pub async fn task_type(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<TaskType, Error> {
    let mut tx = pool.begin().await?;
    let result = task_type_in_transaction(&mut tx, actor, id).await?;
    Ok(result)
}
pub(crate) async fn task_type_in_transaction(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<TaskType, Error> {
    require(db, actor, "tasks.configuration.read").await?;
    dao::task_type(db, actor.workspace, id).await
}

pub async fn save_type(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Option<Uuid>,
    key: &str,
    input: SaveTaskType,
) -> Result<TaskType, Error> {
    let mut tx = pool.begin().await?;
    let result = save_type_in_transaction(&mut tx, actor, id, key, input).await?;
    tx.commit().await?;
    Ok(result)
}
pub(crate) async fn save_type_in_transaction(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Option<Uuid>,
    key: &str,
    input: SaveTaskType,
) -> Result<TaskType, Error> {
    name(&input.name)?;
    if id.is_some() != input.expected_version.is_some() {
        return Err(Error::Invalid(
            "Updates require expected_version; creation must omit it",
        ));
    }
    let a = mutation(db, actor).await?;
    let op = format!(
        "tasks.type.save:{}",
        id.map(|i| i.to_string()).unwrap_or_else(|| "new".into())
    );
    let request = serde_json::to_value(&input).expect("type serializes");
    if let Some(saved) = reliability::replay(db, actor.workspace, &a, &op, key, &request).await? {
        return Ok(saved);
    }
    if let Some(id) = id {
        let old = dao::task_type(db, actor.workspace, id).await?;
        if Some(old.version) != input.expected_version {
            return Err(Error::Conflict("Task type changed; reload before editing"));
        }
    }
    let id = dao::save_type(db, actor.workspace, id, &input).await?;
    reliability::audit(db, actor, &a, "tasks.type.saved", id, serde_json::json!({})).await?;
    let result = dao::task_type(db, actor.workspace, id).await?;
    reliability::save_replay(db, actor.workspace, &a, &op, key, &request, &result).await?;
    Ok(result)
}

pub(super) async fn type_state(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    allow_archived: bool,
) -> Result<TaskType, Error> {
    let t = dao::task_type(db, workspace, id).await?;
    if t.archived && !allow_archived {
        return Err(Error::Conflict(
            "Archived task types cannot receive new assignments",
        ));
    }
    Ok(t)
}
pub async fn templates(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    q: TaskConfigurationQuery,
) -> Result<TaskTemplatePage, Error> {
    let n = super::limit(q.limit)?;
    let mut tx = pool.begin().await?;
    require(&mut tx, actor, "tasks.configuration.read").await?;
    let mut items = dao::templates(&mut tx, actor.workspace, &q, n + 1).await?;
    let next_cursor = if items.len() > n as usize {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    Ok(TaskTemplatePage { items, next_cursor })
}
pub async fn template(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<TaskTemplate, Error> {
    let mut tx = pool.begin().await?;
    let result = template_in_transaction(&mut tx, actor, id).await?;
    Ok(result)
}
pub(crate) async fn template_in_transaction(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<TaskTemplate, Error> {
    require(db, actor, "tasks.configuration.read").await?;
    dao::template(db, actor.workspace, id).await
}

pub async fn revisions(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    q: TaskRevisionQuery,
) -> Result<TaskRevisionPage, Error> {
    let n = super::limit(q.limit)?;
    let mut tx = pool.begin().await?;
    require(&mut tx, actor, "tasks.configuration.read").await?;
    dao::template(&mut tx, actor.workspace, id).await?;
    let mut items = dao::revisions(&mut tx, actor.workspace, id, &q, n + 1).await?;
    let next_cursor = if items.len() > n as usize {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    Ok(TaskRevisionPage { items, next_cursor })
}
pub async fn save_template(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Option<Uuid>,
    key: &str,
    input: SaveTaskTemplate,
) -> Result<TaskTemplate, Error> {
    let mut tx = pool.begin().await?;
    let result = save_template_in_transaction(&mut tx, actor, id, key, input).await?;
    tx.commit().await?;
    Ok(result)
}
pub(crate) async fn save_template_in_transaction(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Option<Uuid>,
    key: &str,
    input: SaveTaskTemplate,
) -> Result<TaskTemplate, Error> {
    name(&input.name)?;
    if id.is_some() != input.expected_version.is_some() {
        return Err(Error::Invalid(
            "Updates require expected_version; creation must omit it",
        ));
    }
    let fields = &input.fields;
    if fields.title.trim().is_empty()
        || fields.title.chars().count() > 200
        || fields.title.chars().any(char::is_control)
        || fields.notes.chars().count() > 10000
    {
        return Err(Error::Invalid(
            "Title must be 1–200 printable characters; notes at most 10000 characters",
        ));
    }
    if !(-3650..=3650).contains(&fields.due_offset_days)
        || (fields.assignment == AssignmentRule::Fixed) != fields.fixed_membership_id.is_some()
    {
        return Err(Error::Invalid(
            "Due offset must be within 3650 days; only fixed assignment requires a membership",
        ));
    }
    let a = mutation(db, actor).await?;
    let op = format!(
        "tasks.template.save:{}",
        id.map(|i| i.to_string()).unwrap_or_else(|| "new".into())
    );
    let request = serde_json::to_value(&input).expect("template serializes");
    if let Some(saved) = reliability::replay(db, actor.workspace, &a, &op, key, &request).await? {
        return Ok(saved);
    }
    let current = if let Some(id) = id {
        let t = dao::template(db, actor.workspace, id).await?;
        if Some(t.version) != input.expected_version {
            return Err(Error::Conflict(
                "Task template changed; reload before editing",
            ));
        }
        Some(t)
    } else {
        None
    };
    let changed = current
        .as_ref()
        .is_none_or(|t| t.revision.fields != *fields);
    let type_name = if changed {
        if let Some(id) = fields.type_id {
            Some(type_state(db, actor.workspace, id, false).await?.name)
        } else {
            None
        }
    } else {
        current.as_ref().and_then(|t| t.revision.type_name.clone())
    };
    if changed && let Some(member) = fields.fixed_membership_id {
        access::require_active_member(db, actor.workspace, member).await?;
    }
    let id = id.unwrap_or_else(Uuid::new_v4);
    let revision = current
        .as_ref()
        .map(|t| t.revision.number + i64::from(changed))
        .unwrap_or(1);
    let audit = reliability::audit(
        db,
        actor,
        &a,
        "tasks.template.saved",
        id,
        serde_json::json!({"revision":revision,"new_revision":changed}),
    )
    .await?;
    dao::save_template(db, actor.workspace, id, &input, revision, current.is_none()).await?;
    if changed {
        dao::insert_revision(
            db,
            actor.workspace,
            id,
            revision,
            fields,
            type_name.as_deref(),
            audit,
        )
        .await?;
    }
    let result = dao::template(db, actor.workspace, id).await?;
    reliability::save_replay(db, actor.workspace, &a, &op, key, &request, &result).await?;
    Ok(result)
}

#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InstantiateTaskTemplate {
    pub revision_id: Uuid,
    pub base_on: String,
    pub context_owner_membership_id: Option<Uuid>,
}
pub async fn instantiate(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: InstantiateTaskTemplate,
) -> Result<super::Task, Error> {
    super::date(Some(&input.base_on))?;
    let mut tx = pool.begin().await?;
    let (write, _) = super::mutation(&mut tx, actor, "tasks.create").await?;
    require(&mut tx, actor, "tasks.configuration.read").await?;
    let op = format!("tasks.template.instantiate:{id}");
    let request = serde_json::to_value(&input).expect("instantiation serializes");
    if let Some(saved) =
        reliability::replay::<super::Task>(&mut tx, actor.workspace, &write, &op, key, &request)
            .await?
    {
        super::require_creation_replay(&mut tx, actor, saved.id).await?;
        return Ok(saved);
    }
    let revision = dao::revision(&mut tx, actor.workspace, input.revision_id).await?;
    if revision.template_id != id {
        return Err(Error::NotFound);
    }
    let result = instantiate_revision(
        &mut tx,
        actor,
        input.revision_id,
        &input.base_on,
        input.context_owner_membership_id,
    )
    .await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &write,
        &op,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
/// The composing application owns its transaction and outer replay identity.
/// Configuration read and Tasks create/read/assignment remain independently checked.
pub(crate) async fn instantiate_revision(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Uuid,
    base_on: &str,
    context_owner: Option<Uuid>,
) -> Result<super::Task, Error> {
    super::date(Some(base_on))?;
    let (write, _) = super::mutation(db, actor, "tasks.create").await?;
    require(db, actor, "tasks.configuration.read").await?;
    let revision = dao::revision(db, actor.workspace, id).await?;
    let template = dao::template(db, actor.workspace, revision.template_id).await?;
    if template.archived {
        return Err(Error::Conflict("Archived templates cannot create new work"));
    }
    let type_name = revision.type_name;
    let fields = revision.fields;
    let assignee = match fields.assignment {
        AssignmentRule::Initiator => write.membership_id,
        AssignmentRule::ContextOwner => {
            context_owner.ok_or(Error::Invalid("This template requires a contextual owner"))?
        }
        AssignmentRule::Fixed => fields
            .fixed_membership_id
            .expect("fixed assignment membership"),
    };
    let base = chrono::NaiveDate::parse_from_str(base_on, "%Y-%m-%d")
        .map_err(|_| Error::Invalid("Invalid base calendar date"))?;
    let due = base
        .checked_add_signed(chrono::Duration::days(i64::from(fields.due_offset_days)))
        .ok_or(Error::Invalid(
            "Due date is outside supported calendar range",
        ))?;
    use chrono::Datelike;
    if !(1..=9999).contains(&due.year()) {
        return Err(Error::Invalid("Due date must be in years 0001–9999"));
    }
    super::create_from_template(
        db,
        actor,
        super::TaskInput {
            type_id: Some(fields.type_id),
            title: fields.title,
            notes: fields.notes,
            assigned_membership_id: Some(assignee),
            due_on: Some(due.format("%Y-%m-%d").to_string()),
            status: super::Status::Open,
        },
        id,
        type_name,
    )
    .await
}

pub(crate) async fn require_read(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
) -> Result<(), Error> {
    require(db, actor, "tasks.configuration.read").await?;
    Ok(())
}
/// Validate a bounded set of pinned task specifications for a composing template.
pub(crate) async fn validate_revisions(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    ids: &[Uuid],
) -> Result<(), Error> {
    require_read(db, actor).await?;
    if ids.len() > 25 {
        return Err(Error::Invalid(
            "At most 25 task specifications are supported",
        ));
    }
    for id in ids {
        let revision = dao::revision(db, actor.workspace, *id).await?;
        let template = dao::template(db, actor.workspace, revision.template_id).await?;
        if template.archived {
            return Err(Error::Conflict(
                "Archived task templates cannot receive new coordination references",
            ));
        }
    }
    Ok(())
}

/// Configuration dependency inspection stays within Tasks ownership.
pub(crate) async fn active_templates_for_type(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<bool, Error> {
    require_read(db, actor).await?;
    dao::active_templates_for_type(db, actor.workspace, id).await
}
