// SPDX-License-Identifier: AGPL-3.0-only
//! Projects owns grouping, membership, boards and comments. Task state stays in
//! shared Tasks and is reached only through its interface, so every view composes
//! two independent authorities: the project's and the task's.
mod dao;
pub mod recipes;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    crm, modules, reliability,
    rich_text::{self, RichBlock},
    tasks::{self, Status, Task, TaskInput, TaskPlan, TaskQuery},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sqlx::{PgConnection, PgPool};
use std::collections::HashMap;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// A board stays readable in one bounded response.
const MAX_CARDS: i64 = 500;
const MAX_COLUMNS: usize = 20;
const MAX_MEMBERS: usize = 200;
const DEFAULT_COLUMNS: [(&str, Status); 3] = [
    ("To do", Status::Open),
    ("In progress", Status::InProgress),
    ("Done", Status::Completed),
];

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    #[default]
    Active,
    OnHold,
    Completed,
    Cancelled,
}
impl ProjectStatus {
    fn key(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::OnHold => "on_hold",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub status: ProjectStatus,
    pub due_on: Option<String>,
    pub archived: bool,
    /// The calling member belongs to this project.
    pub is_member: bool,
    /// The deal this project was converted from, only for members who can read that deal.
    pub source_deal_id: Option<Uuid>,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveProject {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub status: ProjectStatus,
    pub due_on: Option<String>,
    /// Required for updates; omitted on creation.
    pub expected_version: Option<i64>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ArchiveProject {
    pub expected_version: i64,
    pub archived: bool,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct ListProjects {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
    pub search: Option<String>,
    pub archived: Option<bool>,
    pub status: Option<ProjectStatus>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ProjectPage {
    pub items: Vec<Project>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct PageQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ProjectMember {
    pub id: Uuid,
    pub display_name: String,
    pub active: bool,
    pub added_at: Option<String>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AddMember {
    pub membership_id: Uuid,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Column {
    pub id: Uuid,
    pub name: String,
    pub position: i32,
    /// Moving a card here asks Tasks for this status; null changes nothing.
    pub task_status: Option<Status>,
    pub version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveColumn {
    pub name: String,
    pub task_status: Option<Status>,
    /// Zero-based place on the board; omitted keeps it, or adds at the end.
    pub position: Option<i32>,
    /// Required for updates; omitted on creation.
    pub expected_version: Option<i64>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RemoveColumn {
    pub expected_version: i64,
    /// Every card in the column moves to the end of this column, unchanged.
    pub move_cards_to: Uuid,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Card {
    pub id: Uuid,
    pub project_id: Uuid,
    /// Null once the task was removed from the board.
    pub column_id: Option<Uuid>,
    /// Place among the cards this member can see, so it reveals nothing hidden.
    pub position: Option<i32>,
    pub task: Task,
    pub assignee_name: Option<String>,
    /// False when Tasks assigned this to someone outside the project.
    pub assignee_in_project: bool,
    pub comment_count: i64,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AddCard {
    /// Omit to use the first column matching the task's status, or the first column.
    pub column_id: Option<Uuid>,
    /// Link an existing shared task; its status is left as it is.
    pub task_id: Option<Uuid>,
    /// Or create a new shared task; a column's status applies to it.
    pub task: Option<TaskInput>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MoveCard {
    pub expected_version: i64,
    pub column_id: Uuid,
    /// Place the card before this one; null places it last.
    pub before_card_id: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RemoveCard {
    pub expected_version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ScheduleCard {
    /// The shared task's version, as read from the card.
    pub expected_task_version: i64,
    /// A new assignee must belong to the project.
    pub assigned_membership_id: Uuid,
    /// Null clears the deadline.
    pub due_on: Option<String>,
    /// Omit to keep the task's status.
    pub status: Option<Status>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct CardQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
    /// True lists tasks that were removed from the board.
    pub ended: Option<bool>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct CardPage {
    pub items: Vec<Card>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct BoardColumn {
    pub id: Uuid,
    pub name: String,
    pub position: i32,
    pub task_status: Option<Status>,
    pub version: i64,
    pub cards: Vec<Card>,
}
/// Columns and the cards this member can see; hidden tasks are not counted anywhere.
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Board {
    pub project: Project,
    pub columns: Vec<BoardColumn>,
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DueFilter {
    Overdue,
    Soon,
    Unscheduled,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct DeliveryQuery {
    pub assigned_membership_id: Option<Uuid>,
    pub status: Option<Status>,
    #[param(inline)]
    pub due: Option<DueFilter>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliverySummary {
    pub total: i64,
    pub open: i64,
    pub in_progress: i64,
    pub completed: i64,
    pub cancelled: i64,
    pub overdue: i64,
    /// Unfinished work due within the next seven days.
    pub due_soon: i64,
    /// Unfinished work with no deadline.
    pub unscheduled: i64,
    /// Unfinished work due after the project's own deadline.
    pub past_project_deadline: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct AssigneeLoad {
    pub membership_id: Uuid,
    pub display_name: Option<String>,
    pub unfinished: i64,
    pub overdue: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryItem {
    pub card_id: Uuid,
    pub column_name: String,
    pub task: Task,
    pub assignee_name: Option<String>,
    pub overdue: bool,
    pub past_project_deadline: bool,
}
/// Delivery view over the project's visible tasks, soonest deadline first. The
/// summary and workload cover every visible task; filters narrow only `items`.
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Delivery {
    pub project: Project,
    /// The installation's current date, which deadlines are compared with.
    pub today: String,
    pub summary: DeliverySummary,
    pub workload: Vec<AssigneeLoad>,
    pub items: Vec<DeliveryItem>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct MyWorkQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
    /// Include completed and cancelled tasks.
    pub include_finished: Option<bool>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct MyWorkItem {
    pub project_id: Uuid,
    pub project_name: String,
    pub card_id: Uuid,
    pub task: Task,
    pub overdue: bool,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct MyWork {
    pub today: String,
    pub items: Vec<MyWorkItem>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Comment {
    pub id: Uuid,
    pub project_id: Uuid,
    pub card_id: Uuid,
    pub author_membership_id: Uuid,
    pub author_name: String,
    /// Plain source in the rich text subset; empty once removed.
    pub body: String,
    /// Typed blocks parsed from the source; the only form to render.
    #[serde(default)]
    pub blocks: Vec<RichBlock>,
    pub edited: bool,
    pub removed: bool,
    /// The calling member wrote this comment.
    #[serde(default)]
    pub mine: bool,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NewComment {
    pub body: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EditComment {
    pub expected_version: i64,
    pub body: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RemoveComment {
    pub expected_version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct CommentPage {
    pub items: Vec<Comment>,
    pub next_cursor: Option<Uuid>,
}

async fn authorize(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    action: &str,
) -> Result<access::Access, Error> {
    let a = access::require(db, actor.identity, actor.workspace, action).await?;
    modules::require(db, actor.workspace, "projects").await?;
    Ok(a)
}
/// Assigned scope is project membership. Workspace members outside it see no project.
async fn in_scope(
    db: &mut PgConnection,
    workspace: Uuid,
    a: &access::Access,
    project: Uuid,
) -> Result<bool, Error> {
    Ok(a.workspace_wide || dao::is_member(db, workspace, project, a.membership_id).await?)
}
pub(crate) async fn reference(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    project: Uuid,
) -> Result<Project, Error> {
    let a = authorize(db, actor, "projects.read").await?;
    let mut current = dao::get(db, actor.workspace, project).await?;
    if !in_scope(db, actor.workspace, &a, project).await? {
        return Err(Error::NotFound);
    }
    present(db, actor, &mut current).await?;
    Ok(current)
}
/// Fill caller-relative fields. A source deal needs its own, independent CRM authority.
async fn present(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    project: &mut Project,
) -> Result<(), Error> {
    if let Some(deal) = project.source_deal_id {
        match crm::read_deal(db, actor, deal).await {
            Ok(_) => {}
            Err(Error::Forbidden | Error::NotFound) => project.source_deal_id = None,
            Err(error) => return Err(error),
        }
    }
    project.is_member =
        match access::require(db, actor.identity, actor.workspace, "projects.read").await {
            Ok(a) => dao::is_member(db, actor.workspace, project.id, a.membership_id).await?,
            Err(Error::Forbidden) => false,
            Err(error) => return Err(error),
        };
    Ok(())
}
/// Check write authority, serialize with application selection and with every other
/// board change in the workspace, then re-check it inside that order.
async fn mutation(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    action: &str,
    project: Option<Uuid>,
) -> Result<access::Access, Error> {
    authorize(db, actor, action).await?;
    modules::lock_workspace(db, actor.workspace).await?;
    let read = authorize(db, actor, "projects.read").await?;
    let write = authorize(db, actor, action).await?;
    if let Some(project) = project {
        dao::get(db, actor.workspace, project).await?;
        if !in_scope(db, actor.workspace, &read, project).await? {
            return Err(Error::NotFound);
        }
        if !in_scope(db, actor.workspace, &write, project).await? {
            return Err(Error::Forbidden);
        }
    }
    Ok(write)
}
async fn unarchived(
    db: &mut PgConnection,
    workspace: Uuid,
    project: Uuid,
) -> Result<Project, Error> {
    let current = dao::get(db, workspace, project).await?;
    if current.archived {
        return Err(Error::Conflict("Restore the project before changing it"));
    }
    Ok(current)
}
/// A saved result may be replayed only while its subject is still visible.
trait Replayable: Serialize + DeserializeOwned {
    async fn recheck(
        &mut self,
        db: &mut PgConnection,
        actor: &ActorContext<'_>,
    ) -> Result<(), Error>;
}
impl Replayable for Project {
    async fn recheck(
        &mut self,
        db: &mut PgConnection,
        actor: &ActorContext<'_>,
    ) -> Result<(), Error> {
        present(db, actor, self).await
    }
}
impl Replayable for Column {
    async fn recheck(&mut self, _: &mut PgConnection, _: &ActorContext<'_>) -> Result<(), Error> {
        Ok(())
    }
}
impl Replayable for ProjectMember {
    async fn recheck(&mut self, _: &mut PgConnection, _: &ActorContext<'_>) -> Result<(), Error> {
        Ok(())
    }
}
impl Replayable for Card {
    async fn recheck(
        &mut self,
        db: &mut PgConnection,
        actor: &ActorContext<'_>,
    ) -> Result<(), Error> {
        visible_task(db, actor, self.task.id).await.map(|_| ())
    }
}
impl Replayable for Comment {
    async fn recheck(
        &mut self,
        db: &mut PgConnection,
        actor: &ActorContext<'_>,
    ) -> Result<(), Error> {
        let row = dao::card(db, actor.workspace, self.project_id, self.card_id).await?;
        visible_task(db, actor, row.task_id).await.map(|_| ())
    }
}
/// Shared mutation path: authority, replay, change, retry result, commit.
#[allow(clippy::too_many_arguments)]
async fn change<I: Serialize, R: Replayable>(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    project: Option<Uuid>,
    action: &str,
    key: &str,
    operation: String,
    input: &I,
    apply: impl AsyncFnOnce(&mut PgConnection, &access::Access) -> Result<R, Error>,
) -> Result<R, Error> {
    let mut tx = pool.begin().await?;
    let write = mutation(&mut tx, actor, action, project).await?;
    let request = serde_json::to_value(input).expect("project change serializes");
    if let Some(mut saved) =
        reliability::replay::<R>(&mut tx, actor.workspace, &write, &operation, key, &request)
            .await?
    {
        saved.recheck(&mut tx, actor).await?;
        return Ok(saved);
    }
    let result = apply(&mut tx, &write).await?;
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
fn page_limit(limit: Option<i64>) -> Result<i64, Error> {
    let limit = limit.unwrap_or(50);
    if !(1..=100).contains(&limit) {
        return Err(Error::Invalid("Limit must be 1–100"));
    }
    Ok(limit)
}
fn validate_project(input: &SaveProject) -> Result<(), Error> {
    if input.name.trim().is_empty()
        || input.name.chars().count() > 120
        || input.name.chars().any(char::is_control)
        || input.description.chars().count() > 4000
    {
        return Err(Error::Invalid(
            "Name must be 1–120 printable characters; description at most 4000",
        ));
    }
    tasks::date(input.due_on.as_deref())
}
fn validate_column(input: &SaveColumn) -> Result<(), Error> {
    if input.name.trim().is_empty()
        || input.name.chars().count() > 60
        || input.name.chars().any(char::is_control)
    {
        return Err(Error::Invalid(
            "Column name must be 1–60 printable characters",
        ));
    }
    if input.position.is_some_and(|p| p < 0) {
        return Err(Error::Invalid("Column position cannot be negative"));
    }
    Ok(())
}
fn validate_comment(body: &str) -> Result<(), Error> {
    if body.trim().is_empty() || body.chars().count() > 10000 {
        return Err(Error::Invalid("Comments must be 1–10000 characters"));
    }
    rich_text::validate(body)
}
/// Tasks decides visibility, in its bounded batches, for every composition here.
async fn visible_tasks(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    ids: &[Uuid],
) -> Result<HashMap<Uuid, Task>, Error> {
    let mut visible = HashMap::new();
    for batch in ids.chunks(100) {
        visible.extend(tasks::project_many(db, actor, batch).await?);
    }
    Ok(visible)
}
async fn visible_task(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<Task, Error> {
    visible_tasks(db, actor, &[id])
        .await?
        .remove(&id)
        .ok_or(Error::NotFound)
}
/// Every card of a project this member can see. Positions count only what they see.
async fn load_cards(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    project: Uuid,
    ended: bool,
) -> Result<Vec<Card>, Error> {
    let rows = dao::cards(db, actor.workspace, project, ended).await?;
    let ids: Vec<Uuid> = rows.iter().map(|r| r.task_id).collect();
    let mut visible = visible_tasks(db, actor, &ids).await?;
    let members: HashMap<Uuid, String> = dao::members(db, actor.workspace, project)
        .await?
        .into_iter()
        .map(|m| (m.id, m.display_name))
        .collect();
    let mut places: HashMap<Uuid, i32> = HashMap::new();
    let mut cards = Vec::with_capacity(rows.len());
    for row in rows {
        let Some(task) = visible.remove(&row.task_id) else {
            continue;
        };
        let position = match row.column_id {
            Some(column) if !task.archived => {
                let place = places.entry(column).or_insert(0);
                *place += 1;
                Some(*place - 1)
            }
            _ => None,
        };
        let assignee_name = members.get(&task.assigned_membership_id).cloned();
        cards.push(Card {
            id: row.id,
            project_id: project,
            column_id: row.column_id,
            position,
            assignee_in_project: assignee_name.is_some(),
            assignee_name,
            task,
            comment_count: row.comment_count,
            version: row.version,
            created_at: row.created_at,
            updated_at: row.updated_at,
        });
    }
    Ok(cards)
}
async fn one_card(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    project: Uuid,
    card: Uuid,
    ended: bool,
) -> Result<Card, Error> {
    load_cards(db, actor, project, ended)
        .await?
        .into_iter()
        .find(|c| c.id == card)
        .ok_or(Error::NotFound)
}

pub async fn list(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: ListProjects,
) -> Result<ProjectPage, Error> {
    let limit = page_limit(query.limit)?;
    if query.search.as_ref().is_some_and(|s| s.len() > 200) {
        return Err(Error::Invalid("Search must be at most 200 bytes"));
    }
    let mut tx = pool.begin().await?;
    let a = authorize(&mut tx, actor, "projects.read").await?;
    if let Some(after) = query.after {
        reference(&mut tx, actor, after).await?;
    }
    let mut items = dao::list(&mut tx, actor.workspace, &a, &query, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|p| p.id)
    } else {
        None
    };
    for item in &mut items {
        present(&mut tx, actor, item).await?;
    }
    Ok(ProjectPage { items, next_cursor })
}
pub async fn get(pool: &PgPool, actor: &ActorContext<'_>, id: Uuid) -> Result<Project, Error> {
    let mut tx = pool.begin().await?;
    reference(&mut tx, actor, id).await
}
/// The creator joins, so an assigned-scope creator keeps access to what they made.
pub(super) async fn insert(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    write: &access::Access,
    input: &SaveProject,
    members: &[Uuid],
) -> Result<Uuid, Error> {
    let id = dao::insert(db, actor.workspace, input, write.membership_id).await?;
    for (position, (name, status)) in DEFAULT_COLUMNS.iter().enumerate() {
        dao::insert_column(
            db,
            actor.workspace,
            id,
            name,
            position as i32,
            Some(*status),
        )
        .await?;
    }
    for member in members {
        dao::add_member(db, actor.workspace, id, *member).await?;
    }
    reliability::audit(
        db,
        actor,
        write,
        "projects.created",
        id,
        serde_json::json!({}),
    )
    .await?;
    Ok(id)
}
pub async fn save(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Option<Uuid>,
    key: &str,
    input: SaveProject,
) -> Result<Project, Error> {
    validate_project(&input)?;
    if id.is_some() != input.expected_version.is_some() {
        return Err(Error::Invalid(
            "Updates require expected_version; creation must omit it",
        ));
    }
    let workspace = actor.workspace;
    let (action, operation) = match id {
        Some(id) => ("projects.manage", format!("projects.update:{id}")),
        None => ("projects.create", "projects.create".to_string()),
    };
    change(
        pool,
        actor,
        id,
        action,
        key,
        operation,
        &input,
        async |db, write| {
            let saved = match id {
                Some(id) => {
                    let current = unarchived(db, workspace, id).await?;
                    if Some(current.version) != input.expected_version {
                        return Err(Error::Conflict("Project changed; reload before editing"));
                    }
                    dao::update(db, workspace, id, &input).await?;
                    reliability::audit(
                        db,
                        actor,
                        write,
                        "projects.saved",
                        id,
                        serde_json::json!({"status":input.status}),
                    )
                    .await?;
                    id
                }
                None => insert(db, actor, write, &input, &[write.membership_id]).await?,
            };
            let mut result = dao::get(db, workspace, saved).await?;
            present(db, actor, &mut result).await?;
            Ok(result)
        },
    )
    .await
}
pub async fn archive(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: ArchiveProject,
) -> Result<Project, Error> {
    let workspace = actor.workspace;
    change(
        pool,
        actor,
        Some(id),
        "projects.manage",
        key,
        format!("projects.archive:{id}"),
        &input,
        async |db, write| {
            let current = dao::get(db, workspace, id).await?;
            if current.version != input.expected_version {
                return Err(Error::Conflict("Project changed; reload before archiving"));
            }
            dao::archive(db, workspace, id, input.archived).await?;
            reliability::audit(
                db,
                actor,
                write,
                "projects.archived",
                id,
                serde_json::json!({"archived":input.archived}),
            )
            .await?;
            let mut result = dao::get(db, workspace, id).await?;
            present(db, actor, &mut result).await?;
            Ok(result)
        },
    )
    .await
}
pub async fn members(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    project: Uuid,
) -> Result<Vec<ProjectMember>, Error> {
    let mut tx = pool.begin().await?;
    reference(&mut tx, actor, project).await?;
    dao::members(&mut tx, actor.workspace, project).await
}
/// Active workspace members a manager may add or configure; nobody else needs the list.
pub async fn directory(
    pool: &PgPool,
    actor: &ActorContext<'_>,
) -> Result<Vec<ProjectMember>, Error> {
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "projects.members.manage").await?;
    dao::directory(&mut tx, actor.workspace).await
}
pub async fn add_member(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    project: Uuid,
    key: &str,
    input: AddMember,
) -> Result<ProjectMember, Error> {
    let workspace = actor.workspace;
    let member = input.membership_id;
    change(
        pool,
        actor,
        Some(project),
        "projects.members.manage",
        key,
        format!("projects.member.add:{project}"),
        &input,
        async |db, write| {
            unarchived(db, workspace, project).await?;
            access::require_active_member(db, workspace, member).await?;
            let current = dao::members(db, workspace, project).await?;
            if current.iter().any(|m| m.id == member) {
                return Err(Error::Conflict("Already a member of this project"));
            }
            if current.len() >= MAX_MEMBERS {
                return Err(Error::Conflict("A project holds at most 200 members"));
            }
            dao::add_member(db, workspace, project, member).await?;
            reliability::audit(
                db,
                actor,
                write,
                "projects.member.added",
                project,
                serde_json::json!({"membership_id":member}),
            )
            .await?;
            dao::members(db, workspace, project)
                .await?
                .into_iter()
                .find(|m| m.id == member)
                .ok_or(Error::NotFound)
        },
    )
    .await
}
/// Removal ends project scope at once; assigned tasks stay with Tasks and are flagged.
pub async fn remove_member(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    project: Uuid,
    member: Uuid,
    key: &str,
) -> Result<ProjectMember, Error> {
    let workspace = actor.workspace;
    let input = serde_json::json!({});
    change(
        pool,
        actor,
        Some(project),
        "projects.members.manage",
        key,
        format!("projects.member.remove:{project}:{member}"),
        &input,
        async |db, write| {
            let removed = dao::members(db, workspace, project)
                .await?
                .into_iter()
                .find(|m| m.id == member)
                .ok_or(Error::NotFound)?;
            dao::remove_member(db, workspace, project, member).await?;
            reliability::audit(
                db,
                actor,
                write,
                "projects.member.removed",
                project,
                serde_json::json!({"membership_id":member}),
            )
            .await?;
            Ok(removed)
        },
    )
    .await
}
pub async fn columns(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    project: Uuid,
) -> Result<Vec<Column>, Error> {
    let mut tx = pool.begin().await?;
    reference(&mut tx, actor, project).await?;
    dao::columns(&mut tx, actor.workspace, project).await
}
pub async fn save_column(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    project: Uuid,
    id: Option<Uuid>,
    key: &str,
    input: SaveColumn,
) -> Result<Column, Error> {
    validate_column(&input)?;
    if id.is_some() != input.expected_version.is_some() {
        return Err(Error::Invalid(
            "Updates require expected_version; creation must omit it",
        ));
    }
    let workspace = actor.workspace;
    let operation = match id {
        Some(id) => format!("projects.column.update:{id}"),
        None => format!("projects.column.create:{project}"),
    };
    change(
        pool,
        actor,
        Some(project),
        "projects.manage",
        key,
        operation,
        &input,
        async |db, write| {
            unarchived(db, workspace, project).await?;
            let current = dao::columns(db, workspace, project).await?;
            let mut order: Vec<Uuid> = current.iter().map(|c| c.id).collect();
            let saved = match id {
                Some(id) => {
                    let column = current.iter().find(|c| c.id == id).ok_or(Error::NotFound)?;
                    if Some(column.version) != input.expected_version {
                        return Err(Error::Conflict("Column changed; reload before editing"));
                    }
                    dao::update_column(db, workspace, id, &input).await?;
                    id
                }
                None => {
                    if current.len() >= MAX_COLUMNS {
                        return Err(Error::Conflict("A board holds at most 20 columns"));
                    }
                    let id = dao::insert_column(
                        db,
                        workspace,
                        project,
                        input.name.trim(),
                        current.len() as i32,
                        input.task_status,
                    )
                    .await?;
                    order.push(id);
                    id
                }
            };
            if let Some(position) = input.position {
                order.retain(|c| *c != saved);
                order.insert((position as usize).min(order.len()), saved);
                dao::order_columns(db, workspace, project, &order).await?;
            }
            reliability::audit(
                db,
                actor,
                write,
                "projects.column.saved",
                saved,
                serde_json::json!({"project_id":project}),
            )
            .await?;
            dao::columns(db, workspace, project)
                .await?
                .into_iter()
                .find(|c| c.id == saved)
                .ok_or(Error::NotFound)
        },
    )
    .await
}
/// The destination is always required, so the request reveals no hidden card. Cards
/// keep their task status: a bulk move is not a decision about each task.
pub async fn remove_column(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    project: Uuid,
    id: Uuid,
    key: &str,
    input: RemoveColumn,
) -> Result<Column, Error> {
    if input.move_cards_to == id {
        return Err(Error::Invalid(
            "Choose a different column to receive the cards",
        ));
    }
    let workspace = actor.workspace;
    change(
        pool,
        actor,
        Some(project),
        "projects.manage",
        key,
        format!("projects.column.remove:{id}"),
        &input,
        async |db, write| {
            unarchived(db, workspace, project).await?;
            let current = dao::columns(db, workspace, project).await?;
            if !current.iter().any(|c| c.id == input.move_cards_to) {
                return Err(Error::NotFound);
            }
            let column = current
                .into_iter()
                .find(|c| c.id == id)
                .ok_or(Error::NotFound)?;
            if column.version != input.expected_version {
                return Err(Error::Conflict("Column changed; reload before removing"));
            }
            let mut order = dao::column_order(db, workspace, input.move_cards_to).await?;
            order.extend(dao::column_order(db, workspace, id).await?);
            dao::place_cards(db, workspace, project, input.move_cards_to, &order).await?;
            dao::delete_column(db, workspace, id).await?;
            let remaining: Vec<Uuid> = dao::columns(db, workspace, project)
                .await?
                .into_iter()
                .map(|c| c.id)
                .collect();
            dao::order_columns(db, workspace, project, &remaining).await?;
            reliability::audit(
                db,
                actor,
                write,
                "projects.column.removed",
                id,
                serde_json::json!({"project_id":project,"moved_to":input.move_cards_to}),
            )
            .await?;
            Ok(column)
        },
    )
    .await
}
pub async fn board(pool: &PgPool, actor: &ActorContext<'_>, id: Uuid) -> Result<Board, Error> {
    let mut tx = pool.begin().await?;
    let project = reference(&mut tx, actor, id).await?;
    let mut cards = load_cards(&mut tx, actor, id, false).await?;
    cards.retain(|c| !c.task.archived);
    let columns = dao::columns(&mut tx, actor.workspace, id)
        .await?
        .into_iter()
        .map(|column| {
            let (inside, rest): (Vec<Card>, Vec<Card>) = std::mem::take(&mut cards)
                .into_iter()
                .partition(|c| c.column_id == Some(column.id));
            cards = rest;
            BoardColumn {
                id: column.id,
                name: column.name,
                position: column.position,
                task_status: column.task_status,
                version: column.version,
                cards: inside,
            }
        })
        .collect();
    Ok(Board { project, columns })
}
pub async fn delivery(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    query: DeliveryQuery,
) -> Result<Delivery, Error> {
    let mut tx = pool.begin().await?;
    let project = reference(&mut tx, actor, id).await?;
    let (today, soon) = dao::today(&mut tx).await?;
    let columns: HashMap<Uuid, String> = dao::columns(&mut tx, actor.workspace, id)
        .await?
        .into_iter()
        .map(|c| (c.id, c.name))
        .collect();
    let cards = load_cards(&mut tx, actor, id, false).await?;
    let mut summary = DeliverySummary::default();
    let mut workload: HashMap<Uuid, AssigneeLoad> = HashMap::new();
    let mut items = Vec::new();
    for card in cards {
        if card.task.archived {
            continue;
        }
        let task = &card.task;
        let unfinished = matches!(task.status, Status::Open | Status::InProgress);
        let due = task.due_on.as_deref();
        let overdue = unfinished && due.is_some_and(|d| d < today.as_str());
        let is_soon = unfinished && due.is_some_and(|d| d >= today.as_str() && d <= soon.as_str());
        let past_deadline =
            unfinished && matches!((due, project.due_on.as_deref()), (Some(d), Some(p)) if d > p);
        summary.total += 1;
        match task.status {
            Status::Open => summary.open += 1,
            Status::InProgress => summary.in_progress += 1,
            Status::Completed => summary.completed += 1,
            Status::Cancelled => summary.cancelled += 1,
        }
        summary.overdue += overdue as i64;
        summary.due_soon += is_soon as i64;
        summary.unscheduled += (unfinished && due.is_none()) as i64;
        summary.past_project_deadline += past_deadline as i64;
        if unfinished {
            let load = workload
                .entry(task.assigned_membership_id)
                .or_insert_with(|| AssigneeLoad {
                    membership_id: task.assigned_membership_id,
                    display_name: card.assignee_name.clone(),
                    unfinished: 0,
                    overdue: 0,
                });
            load.unfinished += 1;
            load.overdue += overdue as i64;
        }
        let selected = query
            .assigned_membership_id
            .is_none_or(|a| a == task.assigned_membership_id)
            && query.status.is_none_or(|s| s == task.status)
            && match query.due {
                None => true,
                Some(DueFilter::Overdue) => overdue,
                Some(DueFilter::Soon) => is_soon,
                Some(DueFilter::Unscheduled) => unfinished && due.is_none(),
            };
        if selected {
            items.push(DeliveryItem {
                card_id: card.id,
                column_name: card
                    .column_id
                    .and_then(|c| columns.get(&c).cloned())
                    .unwrap_or_default(),
                assignee_name: card.assignee_name,
                overdue,
                past_project_deadline: past_deadline,
                task: card.task,
            });
        }
    }
    items.sort_by(|a, b| {
        let key = |i: &DeliveryItem| (i.task.due_on.is_none(), i.task.due_on.clone(), i.task.id);
        key(a).cmp(&key(b))
    });
    let mut workload: Vec<AssigneeLoad> = workload.into_values().collect();
    workload.sort_by(|a, b| {
        (&a.display_name, a.membership_id).cmp(&(&b.display_name, b.membership_id))
    });
    Ok(Delivery {
        project,
        today,
        summary,
        workload,
        items,
    })
}
/// Tasks assigned to the caller, in projects they may read. Walking the caller's own
/// tasks keeps this bounded however many tasks the workspace's projects hold.
pub async fn my_work(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: MyWorkQuery,
) -> Result<MyWork, Error> {
    let limit = page_limit(query.limit)? as usize;
    let mut tx = pool.begin().await?;
    let a = authorize(&mut tx, actor, "projects.read").await?;
    let (today, _) = dao::today(&mut tx).await?;
    if tasks::require_read(&mut tx, actor).await.is_err() {
        return Ok(MyWork {
            today,
            items: Vec::new(),
            next_cursor: None,
        });
    }
    let finished = query.include_finished.unwrap_or(false);
    let mut items = Vec::new();
    let mut after = query.after;
    let mut next_cursor = None;
    'pages: loop {
        let page = tasks::list_in(
            &mut tx,
            actor,
            TaskQuery {
                limit: Some(100),
                after,
                assigned_membership_id: Some(a.membership_id),
                archived: Some(false),
                ..TaskQuery::default()
            },
            100,
        )
        .await?;
        let ids: Vec<Uuid> = page.items.iter().map(|t| t.id).collect();
        let cards = dao::cards_for_tasks(&mut tx, actor.workspace, &a, &ids).await?;
        for task in page.items {
            let id = task.id;
            let open = matches!(task.status, Status::Open | Status::InProgress);
            if let Some(card) = cards.get(&id)
                && (open || finished)
            {
                if items.len() == limit {
                    next_cursor = after;
                    break 'pages;
                }
                items.push(MyWorkItem {
                    project_id: card.project_id,
                    project_name: card.project_name.clone(),
                    card_id: card.id,
                    overdue: open && task.due_on.as_deref().is_some_and(|d| d < today.as_str()),
                    task,
                });
            }
            after = Some(id);
        }
        if page.next_cursor.is_none() {
            break;
        }
    }
    Ok(MyWork {
        today,
        items,
        next_cursor,
    })
}
pub async fn cards(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    project: Uuid,
    query: CardQuery,
) -> Result<CardPage, Error> {
    let limit = page_limit(query.limit)? as usize;
    let mut tx = pool.begin().await?;
    reference(&mut tx, actor, project).await?;
    let mut items = load_cards(&mut tx, actor, project, query.ended.unwrap_or(false)).await?;
    items.sort_by_key(|c| c.id);
    if let Some(after) = query.after {
        if !items.iter().any(|c| c.id == after) {
            return Err(Error::NotFound);
        }
        items.retain(|c| c.id > after);
    }
    let next_cursor = if items.len() > limit {
        items.truncate(limit);
        items.last().map(|c| c.id)
    } else {
        None
    };
    Ok(CardPage { items, next_cursor })
}
pub async fn card(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    project: Uuid,
    id: Uuid,
) -> Result<Card, Error> {
    let mut tx = pool.begin().await?;
    reference(&mut tx, actor, project).await?;
    one_card(&mut tx, actor, project, id, false).await
}
pub async fn add_card(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    project: Uuid,
    key: &str,
    mut input: AddCard,
) -> Result<Card, Error> {
    if input.task_id.is_some() == input.task.is_some() {
        return Err(Error::Invalid(
            "Provide either an existing task or a new one",
        ));
    }
    let workspace = actor.workspace;
    let request = serde_json::to_value(&input).expect("card serializes");
    change(
        pool,
        actor,
        Some(project),
        "projects.board.edit",
        key,
        format!("projects.card.add:{project}"),
        &request,
        async |db, write| {
            unarchived(db, workspace, project).await?;
            if dao::active_cards(db, workspace, project).await? >= MAX_CARDS {
                return Err(Error::Conflict("A board holds at most 500 tasks"));
            }
            let columns = dao::columns(db, workspace, project).await?;
            let chosen = match input.column_id {
                Some(id) => Some(columns.iter().find(|c| c.id == id).ok_or(Error::NotFound)?),
                None => None,
            };
            let task = match (input.task_id, input.task.take()) {
                (Some(id), _) => {
                    let task = visible_task(db, actor, id).await?;
                    if task.archived {
                        return Err(Error::Conflict("Restore the task before adding it"));
                    }
                    task
                }
                (_, Some(mut new)) => {
                    let assignee = new.assigned_membership_id.unwrap_or(write.membership_id);
                    if !dao::is_member(db, workspace, project, assignee).await? {
                        return Err(Error::Invalid("Assign the task to a project member"));
                    }
                    if let Some(status) = chosen.and_then(|c| c.task_status) {
                        new.status = status;
                    }
                    tasks::create_in_transaction(db, actor, new).await?
                }
                _ => {
                    return Err(Error::Invalid(
                        "Provide either an existing task or a new one",
                    ));
                }
            };
            let column = chosen
                .or_else(|| columns.iter().find(|c| c.task_status == Some(task.status)))
                .or(columns.first())
                .ok_or(Error::Conflict("Add a board column first"))?;
            if dao::on_a_board(db, workspace, task.id).await? {
                return Err(Error::Conflict("This task is already on a project board"));
            }
            let id = dao::add_card(db, workspace, project, task.id, column.id).await?;
            reliability::audit(
                db,
                actor,
                write,
                "projects.card.added",
                id,
                serde_json::json!({"project_id":project,"task_id":task.id}),
            )
            .await?;
            one_card(db, actor, project, id, false).await
        },
    )
    .await
}
/// Concurrent moves serialize on the workspace lock taken by `mutation`, and each
/// renumbers whole columns, so places stay dense and unique. A move of a card someone
/// else just moved is refused by its version rather than silently undoing theirs.
pub async fn move_card(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    project: Uuid,
    id: Uuid,
    key: &str,
    input: MoveCard,
) -> Result<Card, Error> {
    if input.before_card_id == Some(id) {
        return Err(Error::Invalid("A card cannot be placed before itself"));
    }
    let workspace = actor.workspace;
    change(
        pool,
        actor,
        Some(project),
        "projects.board.edit",
        key,
        format!("projects.card.move:{id}"),
        &input,
        async |db, write| {
            unarchived(db, workspace, project).await?;
            let row = dao::card(db, workspace, project, id).await?;
            let task = visible_task(db, actor, row.task_id).await?;
            let Some(source) = row.column_id else {
                return Err(Error::NotFound);
            };
            if row.version != input.expected_version {
                return Err(Error::Conflict("Card moved or changed; reload the board"));
            }
            if task.archived {
                return Err(Error::Conflict("Restore the task before moving it"));
            }
            let target = dao::columns(db, workspace, project)
                .await?
                .into_iter()
                .find(|c| c.id == input.column_id)
                .ok_or(Error::NotFound)?;
            let mut order = dao::column_order(db, workspace, target.id).await?;
            order.retain(|c| *c != id);
            let place = match input.before_card_id {
                Some(before) => {
                    let other = dao::card(db, workspace, project, before).await?;
                    // A hidden neighbour answers like one that does not exist.
                    visible_task(db, actor, other.task_id).await?;
                    order
                        .iter()
                        .position(|c| *c == before)
                        .ok_or(Error::Conflict("That card moved; reload the board"))?
                }
                None => order.len(),
            };
            order.insert(place, id);
            dao::place_cards(db, workspace, project, target.id, &order).await?;
            if source != target.id {
                dao::compact_column(db, workspace, source).await?;
            }
            dao::touch_card(db, workspace, id).await?;
            if source != target.id
                && let Some(status) = target.task_status
                && status != task.status
            {
                // Tasks decides whether this member may change this task's status.
                tasks::plan_in_transaction(
                    db,
                    actor,
                    task.id,
                    None,
                    TaskPlan {
                        status: Some(status),
                        ..TaskPlan::default()
                    },
                )
                .await?;
            }
            reliability::audit(
                db,
                actor,
                write,
                "projects.card.moved",
                id,
                serde_json::json!({"project_id":project,"column_id":target.id}),
            )
            .await?;
            one_card(db, actor, project, id, false).await
        },
    )
    .await
}
/// Assignment and deadline through Tasks, under a project's own assignment rule.
pub async fn schedule_card(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    project: Uuid,
    id: Uuid,
    key: &str,
    input: ScheduleCard,
) -> Result<Card, Error> {
    let workspace = actor.workspace;
    change(
        pool,
        actor,
        Some(project),
        "projects.board.edit",
        key,
        format!("projects.card.schedule:{id}"),
        &input,
        async |db, write| {
            unarchived(db, workspace, project).await?;
            let row = dao::card(db, workspace, project, id).await?;
            if row.column_id.is_none() {
                return Err(Error::NotFound);
            }
            let task = visible_task(db, actor, row.task_id).await?;
            if input.assigned_membership_id != task.assigned_membership_id
                && !dao::is_member(db, workspace, project, input.assigned_membership_id).await?
            {
                return Err(Error::Invalid("Assign the task to a project member"));
            }
            tasks::plan_in_transaction(
                db,
                actor,
                task.id,
                Some(input.expected_task_version),
                TaskPlan {
                    assignee: Some(input.assigned_membership_id),
                    due_on: Some(input.due_on.clone()),
                    status: input.status,
                },
            )
            .await?;
            reliability::audit(
                db,
                actor,
                write,
                "projects.card.scheduled",
                id,
                serde_json::json!({"project_id":project,"task_id":task.id}),
            )
            .await?;
            one_card(db, actor, project, id, false).await
        },
    )
    .await
}
/// The task itself and its comments are kept; adding it back restores the discussion.
pub async fn remove_card(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    project: Uuid,
    id: Uuid,
    key: &str,
    input: RemoveCard,
) -> Result<Card, Error> {
    let workspace = actor.workspace;
    change(
        pool,
        actor,
        Some(project),
        "projects.board.edit",
        key,
        format!("projects.card.remove:{id}"),
        &input,
        async |db, write| {
            unarchived(db, workspace, project).await?;
            let row = dao::card(db, workspace, project, id).await?;
            visible_task(db, actor, row.task_id).await?;
            let Some(source) = row.column_id else {
                return Err(Error::Conflict("This task already left the board"));
            };
            if row.version != input.expected_version {
                return Err(Error::Conflict("Card moved or changed; reload the board"));
            }
            dao::remove_card(db, workspace, id).await?;
            dao::compact_column(db, workspace, source).await?;
            reliability::audit(
                db,
                actor,
                write,
                "projects.card.removed",
                id,
                serde_json::json!({"project_id":project}),
            )
            .await?;
            one_card(db, actor, project, id, true).await
        },
    )
    .await
}
/// Removed comments keep their place in a thread but no text.
fn with_blocks(mut comment: Comment, reader: Uuid) -> Comment {
    comment.blocks = rich_text::parse(&comment.body);
    comment.mine = comment.author_membership_id == reader;
    comment
}
/// A page of comments on visible tasks only. Candidates are read in batches and
/// Tasks filters them, so the cursor and page size reveal nothing hidden.
async fn comment_page(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    project: Uuid,
    card: Option<Uuid>,
    query: PageQuery,
) -> Result<CommentPage, Error> {
    let limit = page_limit(query.limit)? as usize;
    let reader = authorize(db, actor, "projects.read").await?.membership_id;
    let mut items = Vec::new();
    let mut after = query.after;
    if let Some(after) = after {
        let (row, task) = dao::comment(db, actor.workspace, project, after).await?;
        visible_task(db, actor, task).await?;
        if card.is_some_and(|c| c != row.card_id) {
            return Err(Error::NotFound);
        }
    }
    let mut next_cursor = None;
    'batches: loop {
        let rows = dao::comments(db, actor.workspace, project, card, after, 100).await?;
        let exhausted = rows.len() < 100;
        let ids: Vec<Uuid> = rows.iter().map(|(_, task)| *task).collect();
        let visible = visible_tasks(db, actor, &ids).await?;
        for (comment, task) in rows {
            let id = comment.id;
            if visible.contains_key(&task) {
                if items.len() == limit {
                    next_cursor = after;
                    break 'batches;
                }
                items.push(with_blocks(comment, reader));
            }
            after = Some(id);
        }
        if exhausted {
            break;
        }
    }
    Ok(CommentPage { items, next_cursor })
}
pub async fn comments(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    project: Uuid,
    card: Uuid,
    query: PageQuery,
) -> Result<CommentPage, Error> {
    let mut tx = pool.begin().await?;
    reference(&mut tx, actor, project).await?;
    let row = dao::card(&mut tx, actor.workspace, project, card).await?;
    visible_task(&mut tx, actor, row.task_id).await?;
    comment_page(&mut tx, actor, project, Some(card), query).await
}
/// Business export projection: every visible comment in a project.
pub async fn project_comments(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    project: Uuid,
    query: PageQuery,
) -> Result<CommentPage, Error> {
    let mut tx = pool.begin().await?;
    reference(&mut tx, actor, project).await?;
    comment_page(&mut tx, actor, project, None, query).await
}
pub async fn add_comment(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    project: Uuid,
    card: Uuid,
    key: &str,
    input: NewComment,
) -> Result<Comment, Error> {
    validate_comment(&input.body)?;
    let workspace = actor.workspace;
    change(
        pool,
        actor,
        Some(project),
        "projects.comments.write",
        key,
        format!("projects.comment.create:{card}"),
        &input,
        async |db, write| {
            unarchived(db, workspace, project).await?;
            let row = dao::card(db, workspace, project, card).await?;
            if row.column_id.is_none() {
                return Err(Error::NotFound);
            }
            visible_task(db, actor, row.task_id).await?;
            let id = dao::insert_comment(
                db,
                workspace,
                project,
                card,
                write.membership_id,
                &input.body,
            )
            .await?;
            reliability::audit(
                db,
                actor,
                write,
                "projects.comment.created",
                id,
                serde_json::json!({"project_id":project,"card_id":card}),
            )
            .await?;
            Ok(with_blocks(
                dao::comment(db, workspace, project, id).await?.0,
                write.membership_id,
            ))
        },
    )
    .await
}
pub async fn edit_comment(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    project: Uuid,
    id: Uuid,
    key: &str,
    input: EditComment,
) -> Result<Comment, Error> {
    validate_comment(&input.body)?;
    let workspace = actor.workspace;
    change(
        pool,
        actor,
        Some(project),
        "projects.comments.write",
        key,
        format!("projects.comment.edit:{id}"),
        &input,
        async |db, write| {
            unarchived(db, workspace, project).await?;
            let (current, task) = dao::comment(db, workspace, project, id).await?;
            visible_task(db, actor, task).await?;
            if current.author_membership_id != write.membership_id {
                return Err(Error::Forbidden);
            }
            if current.removed || current.version != input.expected_version {
                return Err(Error::Conflict(
                    "Comment changed or was removed; reload before editing",
                ));
            }
            dao::edit_comment(db, workspace, id, &input.body).await?;
            reliability::audit(
                db,
                actor,
                write,
                "projects.comment.edited",
                id,
                serde_json::json!({"project_id":project}),
            )
            .await?;
            Ok(with_blocks(
                dao::comment(db, workspace, project, id).await?.0,
                write.membership_id,
            ))
        },
    )
    .await
}
/// Authors may remove their own; `projects.comments.moderate` removes anyone's. Removal
/// erases the text and its retry payloads, leaving a marker in the thread.
pub async fn remove_comment(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    project: Uuid,
    id: Uuid,
    key: &str,
    input: RemoveComment,
) -> Result<Comment, Error> {
    let author = {
        let mut tx = pool.begin().await?;
        let read = authorize(&mut tx, actor, "projects.read").await?;
        if !in_scope(&mut tx, actor.workspace, &read, project).await? {
            return Err(Error::NotFound);
        }
        let (current, task) = dao::comment(&mut tx, actor.workspace, project, id).await?;
        visible_task(&mut tx, actor, task).await?;
        current.author_membership_id == read.membership_id
    };
    let workspace = actor.workspace;
    change(
        pool,
        actor,
        Some(project),
        if author {
            "projects.comments.write"
        } else {
            "projects.comments.moderate"
        },
        key,
        format!("projects.comment.remove:{id}"),
        &input,
        async |db, write| {
            unarchived(db, workspace, project).await?;
            let (current, task) = dao::comment(db, workspace, project, id).await?;
            visible_task(db, actor, task).await?;
            if current.removed || current.version != input.expected_version {
                return Err(Error::Conflict(
                    "Comment changed or was removed; reload before removing",
                ));
            }
            dao::remove_comment(db, workspace, id).await?;
            reliability::redact_comment_replays(db, workspace, id, current.card_id).await?;
            reliability::audit(
                db,
                actor,
                write,
                "projects.comment.removed",
                id,
                serde_json::json!({"project_id":project,"by_author":author}),
            )
            .await?;
            Ok(with_blocks(
                dao::comment(db, workspace, project, id).await?.0,
                write.membership_id,
            ))
        },
    )
    .await
}
fn id_page<T: Serialize>(
    mut items: Vec<T>,
    id: impl Fn(&T) -> Uuid,
    query: PageQuery,
) -> Result<serde_json::Value, Error> {
    let limit = page_limit(query.limit)? as usize;
    items.sort_by_key(&id);
    if let Some(after) = query.after {
        if !items.iter().any(|item| id(item) == after) {
            return Err(Error::NotFound);
        }
        items.retain(|item| id(item) > after);
    }
    let more = items.len() > limit;
    items.truncate(limit);
    let next_cursor = if more { items.last().map(&id) } else { None };
    Ok(serde_json::json!({"items":items,"next_cursor":next_cursor}))
}
/// Business export projection of a project's members.
pub async fn export_members(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    project: Uuid,
    query: PageQuery,
) -> Result<serde_json::Value, Error> {
    id_page(members(pool, actor, project).await?, |m| m.id, query)
}
/// Business export projection of a board's columns.
pub async fn export_columns(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    project: Uuid,
    query: PageQuery,
) -> Result<serde_json::Value, Error> {
    id_page(columns(pool, actor, project).await?, |c| c.id, query)
}
