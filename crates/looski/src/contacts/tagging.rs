// SPDX-License-Identifier: AGPL-3.0-only
//! Contacts owns typed person and organization label assignments.
mod dao;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    reliability,
    tags::{self, Tag},
};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;
#[derive(Clone, Copy)]
pub enum Target {
    Contact,
    Organization,
}
impl Target {
    fn permission(self) -> &'static str {
        match self {
            Self::Contact => "contacts.tags.manage",
            Self::Organization => "contacts.organizations.tags.manage",
        }
    }
    fn read_permission(self) -> &'static str {
        match self {
            Self::Contact => "contacts.read",
            Self::Organization => "contacts.organizations.read",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Contact => "contacts",
            Self::Organization => "organizations",
        }
    }
}
pub use crate::tags::SetTags;
async fn parent(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    target: Target,
    id: Uuid,
    access: &access::Access,
    lock: bool,
) -> Result<(Uuid, i64, bool), Error> {
    match target {
        Target::Contact => {
            let c = super::dao::get(db, actor.workspace, id, access, lock).await?;
            Ok((c.merged_into_id.unwrap_or(c.id), c.version, c.archived))
        }
        Target::Organization => {
            super::organizations::tag_target(db, actor.workspace, id, access, lock).await
        }
    }
}
pub async fn list(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    target: Target,
    id: Uuid,
) -> Result<Vec<Tag>, Error> {
    let mut tx = pool.begin().await?;
    let read = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        target.read_permission(),
    )
    .await?;
    let (resolved, _, _) = parent(&mut tx, actor, target, id, &read, false).await?;
    if resolved != id {
        parent(&mut tx, actor, target, resolved, &read, false).await?;
    }
    let ids = dao::ids(&mut tx, actor.workspace, target, resolved).await?;
    let result = tags::definitions(&mut tx, actor, &ids, false).await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn set(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    target: Target,
    id: Uuid,
    key: &str,
    input: SetTags,
) -> Result<Vec<Tag>, Error> {
    if input.tag_ids.len() > 50 {
        return Err(Error::Invalid("A record supports at most 50 tags"));
    }
    let mut ids = input.tag_ids.clone();
    ids.sort_unstable();
    ids.dedup();
    if ids.len() != input.tag_ids.len() {
        return Err(Error::Invalid("Tag IDs must be distinct"));
    }
    let mut tx = pool.begin().await?;
    let access = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        target.permission(),
    )
    .await?;
    let read = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        target.read_permission(),
    )
    .await?;
    let operation = format!("{}.tags:{id}", target.label());
    let request = serde_json::to_value(&input).expect("tag input serializes");
    let saved = reliability::replay::<Vec<Tag>>(
        &mut tx,
        actor.workspace,
        &access,
        &operation,
        key,
        &request,
    )
    .await?;
    let (resolved, version, archived) = parent(&mut tx, actor, target, id, &access, true).await?;
    parent(&mut tx, actor, target, id, &read, false).await?;
    let definitions = tags::definitions(&mut tx, actor, &ids, true).await?;
    if let Some(saved) = saved {
        if resolved != id {
            parent(&mut tx, actor, target, resolved, &access, false).await?;
            parent(&mut tx, actor, target, resolved, &read, false).await?;
        }
        return Ok(saved);
    }
    if resolved != id || version != input.expected_version {
        return Err(Error::Conflict(
            "Record changed; reload before changing tags",
        ));
    }
    let existing = dao::ids(&mut tx, actor.workspace, target, id).await?;
    for tag in &definitions {
        if !existing.contains(&tag.id) && (archived || tag.archived) {
            return Err(Error::Conflict(
                "Restore archived records and tags before creating assignments",
            ));
        }
    }
    dao::set(
        &mut tx,
        actor.workspace,
        target,
        id,
        &ids,
        access.membership_id,
    )
    .await?;
    match target {
        Target::Contact => super::dao::touch(&mut tx, actor.workspace, id).await?,
        Target::Organization => {
            super::organizations::touch_for_tags(&mut tx, actor.workspace, id).await?
        }
    }
    reliability::audit(
        &mut tx,
        actor,
        &access,
        &format!("{}.tags_set", target.label()),
        id,
        serde_json::json!({"tag_ids":ids}),
    )
    .await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &access,
        &operation,
        key,
        &request,
        &definitions,
    )
    .await?;
    tx.commit().await?;
    Ok(definitions)
}
pub(super) async fn merge(
    db: &mut PgConnection,
    workspace: Uuid,
    source: Uuid,
    survivor: Uuid,
) -> Result<(), Error> {
    dao::merge(db, workspace, source, survivor).await
}
