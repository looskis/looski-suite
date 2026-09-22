// SPDX-License-Identifier: AGPL-3.0-only
use super::OwnerInput;
use sqlx::PgConnection;
use uuid::Uuid;

pub(super) async fn lock_installation(connection: &mut PgConnection) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT bootstrapped_at IS NOT NULL FROM suite.installation WHERE singleton FOR UPDATE",
    )
    .fetch_one(connection)
    .await
}

pub(super) async fn create_owner(
    connection: &mut PgConnection,
    issuer: &str,
    subject: &str,
    input: &OwnerInput,
) -> Result<(), sqlx::Error> {
    let user_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO suite.users(display_name) VALUES ($1) RETURNING id",
    )
    .bind(input.display_name.trim())
    .fetch_one(&mut *connection)
    .await?;
    sqlx::query("INSERT INTO suite.auth_identities(user_id, issuer, subject) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(issuer)
        .bind(subject)
        .execute(&mut *connection)
        .await?;
    let workspace_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO suite.workspaces(name) VALUES ($1) RETURNING id",
    )
    .bind(input.workspace_name.trim())
    .fetch_one(&mut *connection)
    .await?;
    let membership_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO suite.memberships(workspace_id, user_id) VALUES ($1, $2) RETURNING id",
    )
    .bind(workspace_id)
    .bind(user_id)
    .fetch_one(&mut *connection)
    .await?;
    let role_id = sqlx::query_scalar::<_, Uuid>("INSERT INTO suite.roles(workspace_id, key, name) VALUES ($1, 'owner', 'Owner') RETURNING id")
        .bind(workspace_id).fetch_one(&mut *connection).await?;
    sqlx::query("INSERT INTO suite.role_permissions(workspace_id, role_id, permission_key) SELECT $1, $2, key FROM suite.permissions WHERE key IN ('workspace.read','workspace.modules.read','workspace.modules.manage','crm.configuration.read','crm.configuration.manage','crm.opportunities.read','crm.opportunities.create','crm.opportunities.update','crm.opportunities.archive','crm.deals.read','crm.deals.create','crm.deals.update','crm.deals.archive','workspace.access.read','workspace.access.admin','contacts.read','contacts.create','contacts.update','contacts.archive','contacts.merge','contacts.identifiers.share','contacts.organizations.read','contacts.organizations.create','contacts.organizations.update','contacts.organizations.archive','contacts.relationships.manage','workspace.tags.read','workspace.tags.manage','contacts.tags.manage','contacts.organizations.tags.manage','contacts.intake.create','contacts.intake.read','contacts.intake.resolve')")
        .bind(workspace_id).bind(role_id).execute(&mut *connection).await?;
    sqlx::query("INSERT INTO suite.membership_roles(workspace_id, membership_id, role_id) VALUES ($1, $2, $3)")
        .bind(workspace_id).bind(membership_id).bind(role_id).execute(&mut *connection).await?;
    sqlx::query("INSERT INTO suite.audit_events(workspace_id, operation_id, actor_user_id, action,source_interface) VALUES ($1, $2, $3, 'installation.bootstrapped','operator')")
        .bind(workspace_id).bind(Uuid::new_v4()).bind(user_id).execute(&mut *connection).await?;
    sqlx::query("UPDATE suite.installation SET bootstrapped_at = now() WHERE singleton")
        .execute(&mut *connection)
        .await?;
    Ok(())
}
