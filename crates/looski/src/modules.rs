// SPDX-License-Identifier: AGPL-3.0-only
//! Optional application state. Foundation capabilities do not depend on CRM.
mod dao;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    reliability,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use utoipa::ToSchema;
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Module {
    pub key: String,
    pub name: String,
    pub enabled: bool,
    pub version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SetModule {
    pub enabled: bool,
    pub expected_version: i64,
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
pub async fn list(pool: &PgPool, actor: &ActorContext<'_>) -> Result<Vec<Module>, Error> {
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "workspace.modules.read").await?;
    Ok(vec![dao::get(&mut tx, actor.workspace).await?])
}
pub async fn set(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    module: &str,
    key: &str,
    input: SetModule,
) -> Result<Module, Error> {
    if module != "crm" {
        return Err(Error::NotFound);
    }
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "workspace.modules.manage").await?;
    lock_workspace(&mut tx, actor.workspace).await?;
    let access = authorize(&mut tx, actor, "workspace.modules.manage").await?;
    authorize(&mut tx, actor, "workspace.modules.read").await?;
    let request = serde_json::to_value(&input).expect("module serializes");
    let operation = "modules.crm.set";
    if let Some(saved) =
        reliability::replay(&mut tx, actor.workspace, &access, operation, key, &request).await?
    {
        return Ok(saved);
    }
    let current = dao::get(&mut tx, actor.workspace).await?;
    if current.version != input.expected_version {
        return Err(Error::Conflict(
            "Module selection changed; reload before editing",
        ));
    }
    dao::set(&mut tx, actor.workspace, input.enabled).await?;
    reliability::audit(
        &mut tx,
        actor,
        &access,
        "modules.crm.changed",
        actor.workspace,
        serde_json::json!({"enabled":input.enabled}),
    )
    .await?;
    let result = dao::get(&mut tx, actor.workspace).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &access,
        operation,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
pub(crate) async fn lock_workspace(
    db: &mut PgConnection,
    workspace: uuid::Uuid,
) -> Result<(), Error> {
    dao::lock(db, workspace).await
}
pub(crate) async fn require_crm(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
) -> Result<(), Error> {
    if !dao::get(db, actor.workspace).await?.enabled {
        return Err(Error::Forbidden);
    }
    Ok(())
}
