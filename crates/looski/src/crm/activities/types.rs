// SPDX-License-Identifier: AGPL-3.0-only
mod dao;
use crate::{
    application::{ActorContext, OperationError as Error},
    modules, reliability,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActivityBehavior {
    Note,
    Interaction,
}
impl ActivityBehavior {
    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Interaction => "interaction",
        }
    }
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ActivityType {
    pub id: Uuid,
    pub name: String,
    pub behavior: ActivityBehavior,
    pub position: i32,
    pub archived: bool,
    pub version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveActivityType {
    pub name: String,
    pub behavior: ActivityBehavior,
    pub position: Option<i32>,
    pub archived: Option<bool>,
    pub expected_version: Option<i64>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct ActivityTypeQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
    pub archived: Option<bool>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ActivityTypePage {
    pub items: Vec<ActivityType>,
    pub next_cursor: Option<Uuid>,
}
async fn access(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    action: &str,
) -> Result<crate::access::Access, Error> {
    let a = super::super::authorize(db, actor, action).await?;
    if !a.workspace_wide {
        return Err(Error::Forbidden);
    }
    Ok(a)
}
pub async fn list(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: ActivityTypeQuery,
) -> Result<ActivityTypePage, Error> {
    let n = super::limit(query.limit)?;
    let mut tx = pool.begin().await?;
    access(&mut tx, actor, "crm.configuration.read").await?;
    let mut items = dao::list(&mut tx, actor.workspace, &query, n + 1).await?;
    let next_cursor = if items.len() > n as usize {
        items.pop();
        items.last().map(|i| i.id)
    } else {
        None
    };
    Ok(ActivityTypePage { items, next_cursor })
}
pub async fn get(pool: &PgPool, actor: &ActorContext<'_>, id: Uuid) -> Result<ActivityType, Error> {
    let mut tx = pool.begin().await?;
    access(&mut tx, actor, "crm.configuration.read").await?;
    dao::get(&mut tx, actor.workspace, id).await
}
pub async fn save(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Option<Uuid>,
    key: &str,
    input: SaveActivityType,
) -> Result<ActivityType, Error> {
    if input.name.trim().is_empty()
        || input.name.chars().count() > 120
        || input.name.chars().any(char::is_control)
    {
        return Err(Error::Invalid(
            "Type name must be 1–120 printable characters",
        ));
    }
    if id.is_some() != input.expected_version.is_some() {
        return Err(Error::Invalid(
            "Updates require expected_version; creation must omit it",
        ));
    }
    let mut tx = pool.begin().await?;
    access(&mut tx, actor, "crm.configuration.manage").await?;
    modules::lock_workspace(&mut tx, actor.workspace).await?;
    let a = access(&mut tx, actor, "crm.configuration.manage").await?;
    access(&mut tx, actor, "crm.configuration.read").await?;
    let op = format!(
        "crm.activity_types.save:{}",
        id.map(|i| i.to_string()).unwrap_or_else(|| "new".into())
    );
    let request = serde_json::to_value(&input).expect("type serializes");
    if let Some(saved) =
        reliability::replay(&mut tx, actor.workspace, &a, &op, key, &request).await?
    {
        return Ok(saved);
    }
    if let Some(id) = id {
        let old = dao::get(&mut tx, actor.workspace, id).await?;
        if Some(old.version) != input.expected_version {
            return Err(Error::Conflict(
                "Activity type changed; reload before editing",
            ));
        }
        if old.behavior != input.behavior {
            return Err(Error::Conflict(
                "Activity behavior is immutable; create a new type",
            ));
        }
    }
    let id = dao::save(&mut tx, actor.workspace, id, &input).await?;
    reliability::audit(
        &mut tx,
        actor,
        &a,
        "crm.activity_type.saved",
        id,
        serde_json::json!({}),
    )
    .await?;
    let result = dao::get(&mut tx, actor.workspace, id).await?;
    reliability::save_replay(&mut tx, actor.workspace, &a, &op, key, &request, &result).await?;
    tx.commit().await?;
    Ok(result)
}
pub(super) async fn state(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    allow_archived: bool,
) -> Result<ActivityType, Error> {
    let t = dao::get(db, workspace, id).await?;
    if t.archived && !allow_archived {
        return Err(Error::Conflict(
            "Archived activity types cannot receive new assignments",
        ));
    }
    Ok(t)
}
