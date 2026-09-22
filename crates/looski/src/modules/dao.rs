// SPDX-License-Identifier: AGPL-3.0-only
use super::Module;
use crate::application::OperationError as Error;
use sqlx::PgConnection;
use uuid::Uuid;
pub(super) async fn get(db: &mut PgConnection, workspace: Uuid) -> Result<Module, Error> {
    let row=sqlx::query_as::<_,(bool,i64)>("SELECT enabled,version FROM suite.workspace_modules WHERE workspace_id=$1 AND module_key='crm'").bind(workspace).fetch_optional(db).await?;
    let (enabled, version) = row.unwrap_or((false, 0));
    Ok(Module {
        key: "crm".into(),
        name: "CRM".into(),
        enabled,
        version,
    })
}
pub(super) async fn set(
    db: &mut PgConnection,
    workspace: Uuid,
    enabled: bool,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO suite.workspace_modules(workspace_id,module_key,enabled) VALUES($1,'crm',$2) ON CONFLICT(workspace_id,module_key) DO UPDATE SET enabled=$2,version=suite.workspace_modules.version+1,updated_at=now()").bind(workspace).bind(enabled).execute(db).await?;
    Ok(())
}
pub(super) async fn lock(db: &mut PgConnection, workspace: Uuid) -> Result<(), Error> {
    sqlx::query("SELECT id FROM suite.workspaces WHERE id=$1 FOR NO KEY UPDATE")
        .bind(workspace)
        .fetch_optional(db)
        .await?
        .ok_or(Error::NotFound)?;
    Ok(())
}
