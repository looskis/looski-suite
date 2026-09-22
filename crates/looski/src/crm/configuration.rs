// SPDX-License-Identifier: AGPL-3.0-only
mod dao;
use super::authorize;
use crate::{
    application::{ActorContext, OperationError as Error},
    modules, reliability,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
#[derive(Clone, Copy)]
pub enum Kind {
    Pipeline,
    Stage,
    Category,
    Status,
}
impl Kind {
    pub fn key(self) -> &'static str {
        match self {
            Self::Pipeline => "pipelines",
            Self::Stage => "stages",
            Self::Category => "categories",
            Self::Status => "statuses",
        }
    }
    fn parent(self) -> Option<Self> {
        match self {
            Self::Stage => Some(Self::Pipeline),
            Self::Status => Some(Self::Category),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Open,
    Won,
    Lost,
}
impl Outcome {
    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Won => "won",
            Self::Lost => "lost",
        }
    }
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Definition {
    pub id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub outcome: Option<Outcome>,
    pub position: i32,
    pub archived: bool,
    pub version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveDefinition {
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub outcome: Option<Outcome>,
    pub position: Option<i32>,
    pub archived: Option<bool>,
    pub expected_version: Option<i64>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct DefinitionQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
    pub parent_id: Option<Uuid>,
    pub archived: Option<bool>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DefinitionPage {
    pub items: Vec<Definition>,
    pub next_cursor: Option<Uuid>,
}
async fn access(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    action: &str,
) -> Result<crate::access::Access, Error> {
    let a = authorize(db, actor, action).await?;
    if !a.workspace_wide {
        return Err(Error::Forbidden);
    }
    Ok(a)
}
pub async fn list(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    kind: Kind,
    query: DefinitionQuery,
) -> Result<DefinitionPage, Error> {
    let limit = query.limit.unwrap_or(50);
    if !(1..=100).contains(&limit) || query.parent_id.is_some() && kind.parent().is_none() {
        return Err(Error::Invalid(
            "Limit must be 1–100; parent filter applies only to stages/statuses",
        ));
    }
    let mut tx = pool.begin().await?;
    access(&mut tx, actor, "crm.configuration.read").await?;
    let mut items = dao::list(&mut tx, actor.workspace, kind, &query, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    Ok(DefinitionPage { items, next_cursor })
}
pub async fn get(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    kind: Kind,
    id: Uuid,
) -> Result<Definition, Error> {
    let mut tx = pool.begin().await?;
    access(&mut tx, actor, "crm.configuration.read").await?;
    dao::get(&mut tx, actor.workspace, kind, id).await
}
pub async fn save(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    kind: Kind,
    id: Option<Uuid>,
    key: &str,
    mut input: SaveDefinition,
) -> Result<Definition, Error> {
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 120 || name.chars().any(char::is_control) {
        return Err(Error::Invalid(
            "Definition name must be 1–120 printable characters",
        ));
    }
    if id.is_some() != input.expected_version.is_some() {
        return Err(Error::Invalid(
            "Updates require expected_version; creation must omit it",
        ));
    }
    if !matches!(kind, Kind::Stage) && input.outcome.is_some()
        || kind.parent().is_none() && input.parent_id.is_some()
    {
        return Err(Error::Invalid(
            "This definition does not support that parent or outcome",
        ));
    }
    let mut tx = pool.begin().await?;
    access(&mut tx, actor, "crm.configuration.manage").await?;
    modules::lock_workspace(&mut tx, actor.workspace).await?;
    let a = access(&mut tx, actor, "crm.configuration.manage").await?;
    access(&mut tx, actor, "crm.configuration.read").await?;
    let operation = format!(
        "crm.{}.save:{}",
        kind.key(),
        id.map(|i| i.to_string()).unwrap_or_else(|| "new".into())
    );
    let request = serde_json::to_value(&input).expect("definition serializes");
    if let Some(saved) =
        reliability::replay(&mut tx, actor.workspace, &a, &operation, key, &request).await?
    {
        return Ok(saved);
    }
    let current = if let Some(id) = id {
        let c = dao::get(&mut tx, actor.workspace, kind, id).await?;
        if Some(c.version) != input.expected_version {
            return Err(Error::Conflict("Definition changed; reload before editing"));
        }
        Some(c)
    } else {
        None
    };
    if let Some(ref current) = current {
        if input.parent_id.is_some() && input.parent_id != current.parent_id
            || input.outcome.is_some() && input.outcome != current.outcome
        {
            return Err(Error::Conflict(
                "Parent and semantic outcome are immutable; create a new definition",
            ));
        }
        input.parent_id = current.parent_id;
        input.outcome = current.outcome;
        input.position = Some(input.position.unwrap_or(current.position));
    }
    if let Some(parent_kind) = kind.parent() {
        let parent = input
            .parent_id
            .ok_or(Error::Invalid("Parent definition is required"))?;
        let definition = dao::get(&mut tx, actor.workspace, parent_kind, parent).await?;
        if definition.archived && (current.is_none() || input.archived == Some(false)) {
            return Err(Error::Conflict(
                "An archived parent cannot receive active definitions",
            ));
        }
    }
    if matches!(kind, Kind::Stage) && input.outcome.is_none() {
        return Err(Error::Invalid("Opportunity stage outcome is required"));
    }
    let id = dao::save(&mut tx, actor.workspace, kind, id, &input).await?;
    reliability::audit(
        &mut tx,
        actor,
        &a,
        &format!("crm.{}.saved", kind.key()),
        id,
        serde_json::json!({}),
    )
    .await?;
    let result = dao::get(&mut tx, actor.workspace, kind, id).await?;
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
/// CRM record operations already enforce their read/write authority. Definitions
/// are part of their permitted record's state, not a route to other sales records.
pub(super) async fn state(
    db: &mut PgConnection,
    workspace: Uuid,
    kind: Kind,
    parent: Uuid,
    id: Uuid,
    allow_archived: bool,
) -> Result<Definition, Error> {
    let definition = dao::get(db, workspace, kind, id).await?;
    if definition.parent_id != Some(parent) {
        return Err(Error::Invalid(
            "Stage/status must belong to the selected pipeline/category",
        ));
    }
    let p = dao::get(
        db,
        workspace,
        kind.parent().expect("state kind has parent"),
        parent,
    )
    .await?;
    if !allow_archived && (definition.archived || p.archived) {
        return Err(Error::Conflict(
            "Archived definitions cannot receive new assignments",
        ));
    }
    Ok(definition)
}
