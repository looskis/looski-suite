// SPDX-License-Identifier: AGPL-3.0-only
//! CRM owns labels on sales records; the shared catalog owns their definitions.
mod dao;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    reliability,
    tags::{self, SetTags, Tag},
};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;
#[derive(Clone, Copy)]
pub enum Target {
    Opportunity,
    Deal,
}
impl Target {
    fn permission(self) -> &'static str {
        match self {
            Self::Opportunity => "crm.opportunities.tags.manage",
            Self::Deal => "crm.deals.tags.manage",
        }
    }
    fn read(self) -> &'static str {
        match self {
            Self::Opportunity => "crm.opportunities.read",
            Self::Deal => "crm.deals.read",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Opportunity => "crm.opportunities",
            Self::Deal => "crm.deals",
        }
    }
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
pub async fn list(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    target: Target,
    id: Uuid,
) -> Result<Vec<Tag>, Error> {
    let mut tx = pool.begin().await?;
    let read = super::authorize(&mut tx, actor, target.read()).await?;
    parent(&mut tx, actor, target, id, &read).await?;
    let ids = dao::ids(&mut tx, actor.workspace, target, id).await?;
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
    let (write, read) = super::mutation(&mut tx, actor, target.permission(), target.read()).await?;
    let operation = format!("{}.tags:{id}", target.label());
    let request = serde_json::to_value(&input).expect("tag input serializes");
    let saved = reliability::replay::<Vec<Tag>>(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
    )
    .await?;
    let (version, archived) = parent(&mut tx, actor, target, id, &write).await?;
    parent(&mut tx, actor, target, id, &read).await?;
    let definitions = tags::definitions(&mut tx, actor, &ids, true).await?;
    if let Some(saved) = saved {
        return Ok(saved);
    }
    if version != input.expected_version {
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
        write.membership_id,
    )
    .await?;
    dao::touch(&mut tx, actor.workspace, target, id).await?;
    reliability::audit(
        &mut tx,
        actor,
        &write,
        &format!("{}.tags_set", target.label()),
        id,
        serde_json::json!({"tag_ids":ids}),
    )
    .await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
        &definitions,
    )
    .await?;
    tx.commit().await?;
    Ok(definitions)
}
