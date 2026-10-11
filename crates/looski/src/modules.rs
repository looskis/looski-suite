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
/// Selectable applications, in roadmap order.
pub(crate) const APPLICATIONS: &[(&str, &str)] = &[
    ("crm", "CRM"),
    ("forms", "Forms"),
    ("cms", "CMS"),
    ("projects", "Projects"),
    ("knowledge", "Knowledge base"),
    ("helpdesk", "Helpdesk"),
    ("time", "Time tracking"),
    ("scheduling", "Scheduling"),
];
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
    let mut modules = Vec::new();
    for (key, _) in APPLICATIONS {
        modules.push(dao::get(&mut tx, actor.workspace, key).await?);
    }
    Ok(modules)
}
pub async fn set(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    module: &str,
    key: &str,
    input: SetModule,
) -> Result<Module, Error> {
    if !APPLICATIONS.iter().any(|(key, _)| *key == module) {
        return Err(Error::NotFound);
    }
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "workspace.modules.manage").await?;
    lock_workspace(&mut tx, actor.workspace).await?;
    let access = authorize(&mut tx, actor, "workspace.modules.manage").await?;
    authorize(&mut tx, actor, "workspace.modules.read").await?;
    let request = serde_json::to_value(&input).expect("module serializes");
    let operation = format!("modules.{module}.set");
    let operation = operation.as_str();
    if let Some(saved) =
        reliability::replay(&mut tx, actor.workspace, &access, operation, key, &request).await?
    {
        return Ok(saved);
    }
    let current = dao::get(&mut tx, actor.workspace, module).await?;
    if current.version != input.expected_version {
        return Err(Error::Conflict(
            "Module selection changed; reload before editing",
        ));
    }
    dao::set(&mut tx, actor.workspace, module, input.enabled).await?;
    if !input.enabled {
        crate::jobs::block_module(&mut tx, actor.workspace, module).await?;
    }
    reliability::audit(
        &mut tx,
        actor,
        &access,
        &format!("modules.{module}.changed"),
        actor.workspace,
        serde_json::json!({"enabled":input.enabled}),
    )
    .await?;
    let result = dao::get(&mut tx, actor.workspace, module).await?;
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
    require(db, actor.workspace, "crm").await
}
pub(crate) async fn require(
    db: &mut PgConnection,
    workspace: uuid::Uuid,
    module: &str,
) -> Result<(), Error> {
    if !enabled(db, workspace, module).await? {
        return Err(Error::Forbidden);
    }
    Ok(())
}
pub(crate) async fn enabled(
    db: &mut PgConnection,
    workspace: uuid::Uuid,
    module: &str,
) -> Result<bool, Error> {
    Ok(dao::get(db, workspace, module).await?.enabled)
}
