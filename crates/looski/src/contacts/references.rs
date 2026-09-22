// SPDX-License-Identifier: AGPL-3.0-only
//! Authorized, minimal projections for modules that link to Contacts records.
mod dao;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
};
use serde::Deserialize;
use sqlx::PgConnection;
use uuid::Uuid;
#[derive(Clone, Copy)]
pub(crate) enum Kind {
    Contact,
    Organization,
}
impl Kind {
    pub(crate) fn permission(self) -> &'static str {
        match self {
            Self::Contact => "contacts.read",
            Self::Organization => "contacts.organizations.read",
        }
    }
}
#[derive(Deserialize)]
pub(crate) struct Reference {
    pub reference_id: Uuid,
    pub canonical_id: Uuid,
    pub name: String,
    pub archived: bool,
}
/// Returns only independently visible references, including both alias and survivor scope.
/// Callers page their own links after this projection, never expose omitted IDs/counts.
pub(crate) async fn visible(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    kind: Kind,
    ids: &[Uuid],
) -> Result<Vec<Reference>, Error> {
    if ids.len() > 100 {
        return Err(Error::Invalid("Reference projection is limited to 100 IDs"));
    }
    let a = match access::require(db, actor.identity, actor.workspace, kind.permission()).await {
        Ok(a) => a,
        Err(Error::Forbidden) => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    dao::visible(db, actor.workspace, kind, &a, ids).await
}
pub(crate) async fn require(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    kind: Kind,
    id: Uuid,
) -> Result<Reference, Error> {
    let a = access::require(db, actor.identity, actor.workspace, kind.permission()).await?;
    dao::visible(db, actor.workspace, kind, &a, &[id])
        .await?
        .pop()
        .ok_or(Error::NotFound)
}
/// Stabilize a link target for the caller's transaction. Contact merges use the
/// same workspace advisory lock before row locks. Lock original and survivor in
/// UUID order, then re-read current scope, archive state and canonical identity.
pub(crate) async fn for_mutation(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    kind: Kind,
    id: Uuid,
) -> Result<Reference, Error> {
    if matches!(kind, Kind::Contact) {
        super::dao::lock_merge(db, actor.workspace).await?;
    }
    let reference = require(db, actor, kind, id).await?;
    let mut ids = vec![id, reference.canonical_id];
    ids.sort_unstable();
    ids.dedup();
    dao::lock(db, actor.workspace, kind, &ids).await?;
    require(db, actor, kind, id).await
}
