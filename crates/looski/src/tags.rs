// SPDX-License-Identifier: AGPL-3.0-only
//! Shared label definitions. Assignments belong to the module owning the record.
mod dao;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    reliability,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Tag {
    pub id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub archived: bool,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveTag {
    pub name: String,
    pub color: Option<String>,
    #[serde(default)]
    pub archived: Option<bool>,
    pub expected_version: Option<i64>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct ListTags {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
    pub search: Option<String>,
    pub archived: Option<bool>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TagPage {
    pub items: Vec<Tag>,
    pub next_cursor: Option<Uuid>,
}
async fn authorize(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    action: &str,
) -> Result<access::Access, Error> {
    let access = access::require(db, actor.identity, actor.workspace, action).await?;
    if !access.workspace_wide {
        return Err(Error::Forbidden);
    }
    Ok(access)
}
pub async fn list(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: ListTags,
) -> Result<TagPage, Error> {
    let limit = query.limit.unwrap_or(50);
    if !(1..=100).contains(&limit) || query.search.as_ref().is_some_and(|s| s.len() > 200) {
        return Err(Error::Invalid(
            "Limit must be 1–100 and search at most 200 bytes",
        ));
    }
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "workspace.tags.read").await?;
    let mut items = dao::list(&mut tx, actor.workspace, &query, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|t| t.id)
    } else {
        None
    };
    tx.commit().await?;
    Ok(TagPage { items, next_cursor })
}
pub async fn get(pool: &PgPool, actor: &ActorContext<'_>, id: Uuid) -> Result<Tag, Error> {
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "workspace.tags.read").await?;
    let tag = dao::get(&mut tx, actor.workspace, id, false).await?;
    tx.commit().await?;
    Ok(tag)
}
pub async fn save(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Option<Uuid>,
    key: &str,
    input: SaveTag,
) -> Result<Tag, Error> {
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
        return Err(Error::Invalid("Tag name must be 1–80 printable characters"));
    }
    if input.color.as_ref().is_some_and(|c| {
        c.len() != 7 || !c.starts_with('#') || !c[1..].bytes().all(|b| b.is_ascii_hexdigit())
    }) {
        return Err(Error::Invalid(
            "Color must be a six-digit hex value such as #285b4b",
        ));
    }
    if id.is_some() != input.expected_version.is_some() {
        return Err(Error::Invalid(
            "Updates require expected_version; creation must omit it",
        ));
    }
    let mut tx = pool.begin().await?;
    let access = authorize(&mut tx, actor, "workspace.tags.manage").await?;
    authorize(&mut tx, actor, "workspace.tags.read").await?;
    let operation = id
        .map(|id| format!("tags.update:{id}"))
        .unwrap_or_else(|| "tags.create".into());
    let request = serde_json::to_value(&input).expect("tag serializes");
    if let Some(saved) =
        reliability::replay::<Tag>(&mut tx, actor.workspace, &access, &operation, key, &request)
            .await?
    {
        dao::get(&mut tx, actor.workspace, saved.id, false).await?;
        return Ok(saved);
    }
    if let Some(id) = id {
        let current = dao::get(&mut tx, actor.workspace, id, true).await?;
        if Some(current.version) != input.expected_version {
            return Err(Error::Conflict("Tag changed; reload before editing"));
        }
    }
    let name_key = name.to_lowercase();
    dao::reserve_name(&mut tx, actor.workspace, id, &name_key).await?;
    let id = dao::save(&mut tx, actor.workspace, id, &input, &name_key).await?;
    reliability::audit(
        &mut tx,
        actor,
        &access,
        "tags.saved",
        id,
        serde_json::json!({}),
    )
    .await?;
    let result = dao::get(&mut tx, actor.workspace, id, false).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &access,
        &operation,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
/// Called by module-owned assignment operations within their transaction.
/// Lock in ID order so archive cannot race a newly attached label.
pub(crate) async fn definitions(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    ids: &[Uuid],
    lock: bool,
) -> Result<Vec<Tag>, Error> {
    authorize(db, actor, "workspace.tags.read").await?;
    let mut ids = ids.to_vec();
    ids.sort_unstable();
    ids.dedup();
    let mut result = Vec::new();
    for id in ids {
        result.push(dao::get(db, actor.workspace, id, lock).await?)
    }
    Ok(result)
}
