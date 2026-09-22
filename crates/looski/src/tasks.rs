// SPDX-License-Identifier: AGPL-3.0-only
//! Shared actionable work; no dependency on CRM or Projects lifecycle.
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
#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    #[default]
    Open,
    InProgress,
    Completed,
    Cancelled,
}
impl Status {
    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
}
#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub notes: String,
    pub assigned_membership_id: Uuid,
    pub due_on: Option<String>,
    pub status: Status,
    pub completed_at: Option<String>,
    pub archived: bool,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskInput {
    pub title: String,
    #[serde(default)]
    pub notes: String,
    pub assigned_membership_id: Option<Uuid>,
    pub due_on: Option<String>,
    #[serde(default)]
    pub status: Status,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct EditTask {
    pub expected_version: i64,
    #[serde(flatten)]
    pub fields: TaskInput,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ArchiveTask {
    pub expected_version: i64,
    pub archived: bool,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct TaskQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
    pub search: Option<String>,
    pub archived: Option<bool>,
    pub status: Option<Status>,
    pub assigned_membership_id: Option<Uuid>,
    pub due_before: Option<String>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct TaskHistoryQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TaskPage {
    pub items: Vec<Task>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TaskChange {
    pub id: Uuid,
    pub snapshot: Task,
    pub audit_event_id: Uuid,
    pub occurred_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TaskHistoryPage {
    pub items: Vec<TaskChange>,
    pub next_cursor: Option<Uuid>,
}
fn limit(value: Option<i64>) -> Result<i64, Error> {
    let n = value.unwrap_or(50);
    if !(1..=100).contains(&n) {
        return Err(Error::Invalid("Limit must be 1–100"));
    }
    Ok(n)
}
fn date(value: Option<&str>) -> Result<(), Error> {
    if let Some(v) = value {
        let parts: Vec<_> = v.split('-').collect();
        if parts.len() != 3
            || parts[0].len() != 4
            || parts[1].len() != 2
            || parts[2].len() != 2
            || !parts.iter().all(|p| p.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(Error::Invalid(
                "Dates must be valid YYYY-MM-DD calendar dates",
            ));
        }
        let y = parts[0].parse::<u32>().unwrap_or(0);
        let m = parts[1].parse::<u32>().unwrap_or(0);
        let d = parts[2].parse::<u32>().unwrap_or(0);
        let days = match m {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) => 29,
            2 => 28,
            _ => 0,
        };
        if y == 0 || d == 0 || d > days {
            return Err(Error::Invalid(
                "Dates must be valid YYYY-MM-DD calendar dates",
            ));
        }
    }
    Ok(())
}
fn validate(input: &TaskInput) -> Result<(), Error> {
    if input.title.trim().is_empty()
        || input.title.chars().count() > 200
        || input.title.chars().any(char::is_control)
        || input.notes.chars().count() > 10000
    {
        return Err(Error::Invalid(
            "Title must be 1–200 printable characters; notes at most 10000 characters",
        ));
    }
    date(input.due_on.as_deref())
}
fn scope(a: &access::Access, t: &Task) -> Result<(), Error> {
    if !a.permits_owner(t.assigned_membership_id) {
        return Err(Error::NotFound);
    }
    Ok(())
}
async fn require(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    action: &str,
) -> Result<access::Access, Error> {
    access::require(db, actor.identity, actor.workspace, action).await
}
pub(crate) async fn read(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<Task, Error> {
    let a = require(db, actor, "tasks.read").await?;
    let t = dao::get(db, actor.workspace, id).await?;
    scope(&a, &t)?;
    Ok(t)
}
pub async fn get(pool: &PgPool, actor: &ActorContext<'_>, id: Uuid) -> Result<Task, Error> {
    let mut tx = pool.begin().await?;
    read(&mut tx, actor, id).await
}
pub async fn list(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: TaskQuery,
) -> Result<TaskPage, Error> {
    let n = limit(query.limit)?;
    date(query.due_before.as_deref())?;
    if query.search.as_ref().is_some_and(|s| s.len() > 200) {
        return Err(Error::Invalid("Search must be at most 200 bytes"));
    }
    let mut tx = pool.begin().await?;
    let a = require(&mut tx, actor, "tasks.read").await?;
    if let Some(id) = query.after {
        read(&mut tx, actor, id).await?;
    }
    let mut items = dao::list(&mut tx, actor.workspace, &a, &query, n + 1).await?;
    let next_cursor = if items.len() > n as usize {
        items.pop();
        items.last().map(|t| t.id)
    } else {
        None
    };
    Ok(TaskPage { items, next_cursor })
}
async fn mutation(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    action: &str,
) -> Result<(access::Access, access::Access), Error> {
    require(db, actor, action).await?;
    modules::lock_workspace(db, actor.workspace).await?;
    Ok((
        require(db, actor, action).await?,
        require(db, actor, "tasks.read").await?,
    ))
}
pub async fn save(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Option<Uuid>,
    expected: Option<i64>,
    key: &str,
    input: TaskInput,
) -> Result<Task, Error> {
    validate(&input)?;
    let mut tx = pool.begin().await?;
    let (write, read_access) = mutation(
        &mut tx,
        actor,
        if id.is_some() {
            "tasks.update"
        } else {
            "tasks.create"
        },
    )
    .await?;
    let operation = format!(
        "tasks.save:{}",
        id.map(|i| i.to_string()).unwrap_or_else(|| "new".into())
    );
    let request = serde_json::json!({"input":input,"expected_version":expected});
    if let Some(saved) =
        reliability::replay::<Task>(&mut tx, actor.workspace, &write, &operation, key, &request)
            .await?
    {
        let current = read(&mut tx, actor, saved.id).await?;
        scope(&write, &current)?;
        return Ok(saved);
    }
    let current = if let Some(id) = id {
        let current = read(&mut tx, actor, id).await?;
        scope(&write, &current)?;
        if Some(current.version) != expected || current.archived {
            return Err(Error::Conflict(
                "Task changed or is archived; reload before editing",
            ));
        }
        Some(current)
    } else {
        None
    };
    let assignee = input.assigned_membership_id.unwrap_or_else(|| {
        current
            .as_ref()
            .map(|t| t.assigned_membership_id)
            .unwrap_or(write.membership_id)
    });
    if !write.permits_owner(assignee) || !read_access.permits_owner(assignee) {
        return Err(Error::Forbidden);
    }
    access::require_active_member(&mut tx, actor.workspace, assignee).await?;
    let id = dao::save(&mut tx, actor.workspace, id, assignee, &input).await?;
    let result = dao::get(&mut tx, actor.workspace, id).await?;
    record(&mut tx, actor, &write, "tasks.saved", &result).await?;
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
async fn record(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    a: &access::Access,
    action: &str,
    result: &Task,
) -> Result<(), Error> {
    let audit = reliability::audit(
        db,
        actor,
        a,
        action,
        result.id,
        serde_json::json!({"version":result.version,"status":result.status,"archived":result.archived}),
    )
    .await?;
    dao::history_insert(db, actor.workspace, result, audit).await
}
pub async fn archive(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: ArchiveTask,
) -> Result<Task, Error> {
    let mut tx = pool.begin().await?;
    let (write, _) = mutation(&mut tx, actor, "tasks.archive").await?;
    let current = read(&mut tx, actor, id).await?;
    scope(&write, &current)?;
    let operation = format!("tasks.archive:{id}");
    let request = serde_json::to_value(&input).expect("archive serializes");
    if let Some(saved) =
        reliability::replay(&mut tx, actor.workspace, &write, &operation, key, &request).await?
    {
        return Ok(saved);
    }
    if current.version != input.expected_version {
        return Err(Error::Conflict("Task changed; reload before archiving"));
    }
    dao::archive(&mut tx, actor.workspace, id, input.archived).await?;
    let result = dao::get(&mut tx, actor.workspace, id).await?;
    record(&mut tx, actor, &write, "tasks.archived", &result).await?;
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
pub async fn history(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    query: TaskHistoryQuery,
) -> Result<TaskHistoryPage, Error> {
    let n = limit(query.limit)?;
    let mut tx = pool.begin().await?;
    read(&mut tx, actor, id).await?;
    let mut items = dao::history(&mut tx, actor.workspace, id, &query, n + 1).await?;
    let next_cursor = if items.len() > n as usize {
        items.pop();
        items.last().map(|t| t.id)
    } else {
        None
    };
    Ok(TaskHistoryPage { items, next_cursor })
}
