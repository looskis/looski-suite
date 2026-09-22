// SPDX-License-Identifier: AGPL-3.0-only
//! CRM-owned links invoke Tasks operations without reading its tables.
mod dao;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    reliability, tasks,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
#[derive(Clone, Copy)]
pub enum Target {
    Opportunity,
    Deal,
}
impl Target {
    fn read(self) -> &'static str {
        match self {
            Self::Opportunity => "crm.opportunities.read",
            Self::Deal => "crm.deals.read",
        }
    }
    fn manage(self) -> &'static str {
        match self {
            Self::Opportunity => "crm.opportunities.tasks.manage",
            Self::Deal => "crm.deals.tasks.manage",
        }
    }
    fn table(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Opportunity => (
                "crm.opportunity_tasks",
                "opportunity_id",
                "crm.opportunity_task_changes",
            ),
            Self::Deal => ("crm.deal_tasks", "deal_id", "crm.deal_task_changes"),
        }
    }
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LinkTask {
    pub expected_version: i64,
    pub task_id: Uuid,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateFollowup {
    pub expected_version: i64,
    pub task: tasks::TaskInput,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EndTaskLink {
    pub expected_version: i64,
    pub ended: bool,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct TaskLinkQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
    pub ended: Option<bool>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TaskLink {
    pub id: Uuid,
    pub task: tasks::Task,
    pub ended: bool,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TaskLinkPage {
    pub items: Vec<TaskLink>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TaskLinkChange {
    pub id: Uuid,
    pub ended: bool,
    pub occurred_at: String,
    pub audit_event_id: Uuid,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TaskLinkHistory {
    pub link: TaskLink,
    pub items: Vec<TaskLinkChange>,
    pub next_cursor: Option<Uuid>,
}
fn limit(n: Option<i64>) -> Result<usize, Error> {
    let n = n.unwrap_or(50);
    if !(1..=100).contains(&n) {
        return Err(Error::Invalid("Limit must be 1–100"));
    }
    Ok(n as usize)
}
async fn parent(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    target: Target,
    id: Uuid,
    a: &access::Access,
) -> Result<(i64, bool), Error> {
    let (owner, version, archived) = match target {
        Target::Opportunity => {
            let r = super::dao::opportunity(db, actor.workspace, id).await?;
            (r.owner_membership_id, r.version, r.archived)
        }
        Target::Deal => {
            let r = super::dao::deal(db, actor.workspace, id).await?;
            (r.owner_membership_id, r.version, r.archived)
        }
    };
    super::record_scope(a, owner)?;
    Ok((version, archived))
}
async fn project(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    row: dao::Row,
) -> Result<TaskLink, Error> {
    Ok(TaskLink {
        id: row.id,
        task: tasks::read(db, actor, row.task_id).await?,
        ended: row.ended,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}
async fn visible(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    row: dao::Row,
) -> Result<Option<TaskLink>, Error> {
    match project(db, actor, row).await {
        Ok(link) => Ok(Some(link)),
        Err(Error::Forbidden | Error::NotFound) => Ok(None),
        Err(error) => Err(error),
    }
}
pub async fn list(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    target: Target,
    id: Uuid,
    query: TaskLinkQuery,
) -> Result<TaskLinkPage, Error> {
    let n = limit(query.limit)?;
    let mut tx = pool.begin().await?;
    let read = super::authorize(&mut tx, actor, target.read()).await?;
    parent(&mut tx, actor, target, id, &read).await?;
    match tasks::require_read(&mut tx, actor).await {
        Ok(_) => {}
        Err(Error::Forbidden) if query.after.is_some() => return Err(Error::NotFound),
        Err(Error::Forbidden) => {
            return Ok(TaskLinkPage {
                items: Vec::new(),
                next_cursor: None,
            });
        }
        Err(error) => return Err(error),
    }
    if let Some(cursor) = query.after {
        let row = dao::get(&mut tx, actor.workspace, target, id, cursor).await?;
        if visible(&mut tx, actor, row).await?.is_none() {
            return Err(Error::NotFound);
        }
    }
    let mut cursor = query.after;
    let mut items = Vec::new();
    loop {
        let rows = dao::list(
            &mut tx,
            actor.workspace,
            target,
            id,
            cursor,
            query.ended.unwrap_or(false),
        )
        .await?;
        let count = rows.len();
        let ids = rows.iter().map(|row| row.task_id).collect::<Vec<_>>();
        let mut projected = tasks::project_many(&mut tx, actor, &ids).await?;
        for row in rows {
            cursor = Some(row.id);
            if let Some(task) = projected.remove(&row.task_id) {
                items.push(TaskLink {
                    id: row.id,
                    task,
                    ended: row.ended,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                });
                if items.len() > n {
                    break;
                }
            }
        }
        if items.len() > n || count < 100 {
            break;
        }
    }
    let next_cursor = if items.len() > n {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    Ok(TaskLinkPage { items, next_cursor })
}
async fn audit(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    write: &access::Access,
    target: Target,
    parent_id: Uuid,
    row: &dao::Row,
) -> Result<(), Error> {
    let audit = reliability::audit(
        db,
        actor,
        write,
        &format!("{}.changed", target.table().0),
        parent_id,
        serde_json::json!({"link_id":row.id,"task_id":row.task_id,"ended":row.ended}),
    )
    .await?;
    dao::change(db, actor.workspace, target, row.id, row.ended, audit).await?;
    dao::touch(db, actor.workspace, target, parent_id).await
}
async fn guards(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    target: Target,
    id: Uuid,
) -> Result<(access::Access, i64, bool), Error> {
    let (write, read) = super::mutation(db, actor, target.manage(), target.read()).await?;
    let (version, archived) = parent(db, actor, target, id, &write).await?;
    parent(db, actor, target, id, &read).await?;
    Ok((write, version, archived))
}
fn current(version: i64, expected: i64, archived: bool) -> Result<(), Error> {
    if version != expected {
        return Err(Error::Conflict(
            "Record changed; reload before changing task links",
        ));
    }
    if archived {
        return Err(Error::Conflict(
            "Restore archived records before adding task links",
        ));
    }
    Ok(())
}
pub async fn link(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    target: Target,
    id: Uuid,
    key: &str,
    input: LinkTask,
) -> Result<TaskLink, Error> {
    let mut tx = pool.begin().await?;
    let (write, version, archived) = guards(&mut tx, actor, target, id).await?;
    let operation = format!("{}.link:{id}", target.table().0);
    let request = serde_json::to_value(&input).expect("link serializes");
    if let Some(saved) =
        reliability::replay::<TaskLink>(&mut tx, actor.workspace, &write, &operation, key, &request)
            .await?
    {
        tasks::read(&mut tx, actor, saved.task.id).await?;
        return Ok(saved);
    }
    current(version, input.expected_version, archived)?;
    let task = tasks::read(&mut tx, actor, input.task_id).await?;
    if task.archived {
        return Err(Error::Conflict("Restore archived tasks before linking"));
    }
    let row = dao::insert(
        &mut tx,
        actor.workspace,
        target,
        id,
        input.task_id,
        write.membership_id,
    )
    .await?;
    audit(&mut tx, actor, &write, target, id, &row).await?;
    let result = project(&mut tx, actor, row).await?;
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
pub async fn create(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    target: Target,
    id: Uuid,
    key: &str,
    input: CreateFollowup,
) -> Result<TaskLink, Error> {
    let mut tx = pool.begin().await?;
    let (write, version, archived) = guards(&mut tx, actor, target, id).await?;
    let operation = format!("{}.create:{id}", target.table().0);
    let request = serde_json::to_value(&input).expect("followup serializes");
    if let Some(saved) =
        reliability::replay::<TaskLink>(&mut tx, actor.workspace, &write, &operation, key, &request)
            .await?
    {
        tasks::require_creation_replay(&mut tx, actor, saved.task.id).await?;
        return Ok(saved);
    }
    current(version, input.expected_version, archived)?;
    let task = tasks::create_in_transaction(&mut tx, actor, input.task).await?;
    let row = dao::insert(
        &mut tx,
        actor.workspace,
        target,
        id,
        task.id,
        write.membership_id,
    )
    .await?;
    audit(&mut tx, actor, &write, target, id, &row).await?;
    let result = project(&mut tx, actor, row).await?;
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
pub async fn end(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    target: Target,
    id: Uuid,
    link_id: Uuid,
    key: &str,
    input: EndTaskLink,
) -> Result<TaskLink, Error> {
    let mut tx = pool.begin().await?;
    let (write, version, archived) = guards(&mut tx, actor, target, id).await?;
    let operation = format!("{}.end:{id}:{link_id}", target.table().0);
    let request = serde_json::to_value(&input).expect("link state serializes");
    tasks::require_read(&mut tx, actor).await?;
    let row = dao::get(&mut tx, actor.workspace, target, id, link_id).await?;
    let task = tasks::read(&mut tx, actor, row.task_id).await?;
    if let Some(saved) =
        reliability::replay(&mut tx, actor.workspace, &write, &operation, key, &request).await?
    {
        return Ok(saved);
    }
    if version != input.expected_version {
        return Err(Error::Conflict(
            "Record changed; reload before changing task links",
        ));
    }
    if !input.ended && (archived || task.archived) {
        return Err(Error::Conflict(
            "Restore archived records and tasks before restoring links",
        ));
    }
    dao::end(&mut tx, actor.workspace, target, link_id, input.ended).await?;
    let row = dao::get(&mut tx, actor.workspace, target, id, link_id).await?;
    audit(&mut tx, actor, &write, target, id, &row).await?;
    let result = project(&mut tx, actor, row).await?;
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
    target: Target,
    id: Uuid,
    link_id: Uuid,
    query: super::HistoryQuery,
) -> Result<TaskLinkHistory, Error> {
    let n = limit(query.limit)?;
    let mut tx = pool.begin().await?;
    let read = super::authorize(&mut tx, actor, target.read()).await?;
    parent(&mut tx, actor, target, id, &read).await?;
    let row = dao::get(&mut tx, actor.workspace, target, id, link_id).await?;
    let link = visible(&mut tx, actor, row).await?.ok_or(Error::NotFound)?;
    let mut items = dao::history(
        &mut tx,
        actor.workspace,
        target,
        link_id,
        &query,
        n as i64 + 1,
    )
    .await?;
    let next_cursor = if items.len() > n {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    Ok(TaskLinkHistory {
        link,
        items,
        next_cursor,
    })
}

/// Composition retains CRM link authority, task visibility and link audit/history.
/// The caller owns the surrounding transaction and retry boundary.
pub(super) async fn attach_created(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    deal: Uuid,
    task_id: Uuid,
) -> Result<TaskLink, Error> {
    let (write, _, archived) = guards(db, actor, Target::Deal, deal).await?;
    let task = tasks::read(db, actor, task_id).await?;
    if archived || task.archived {
        return Err(Error::Conflict(
            "Archived records cannot receive task links",
        ));
    }
    let row = dao::insert(
        db,
        actor.workspace,
        Target::Deal,
        deal,
        task_id,
        write.membership_id,
    )
    .await?;
    audit(db, actor, &write, Target::Deal, deal, &row).await?;
    project(db, actor, row).await
}
