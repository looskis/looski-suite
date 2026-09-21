// SPDX-License-Identifier: AGPL-3.0-only
use super::Workspace;
use crate::identity::Identity;
use sqlx::PgPool;

pub(super) async fn visible_to(
    pool: &PgPool,
    identity: &Identity,
) -> Result<Vec<Workspace>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT w.id::text, w.name FROM suite.workspaces w
         JOIN suite.memberships m ON m.workspace_id = w.id
         JOIN suite.users u ON u.id = m.user_id
         JOIN suite.auth_identities i ON i.user_id = u.id
         WHERE i.issuer = $1 AND i.subject = $2 AND i.disabled_at IS NULL
           AND u.state = 'active' AND m.state = 'active' AND w.state = 'active'
           AND EXISTS (
             SELECT 1 FROM suite.membership_roles mr
             JOIN suite.roles r ON (r.workspace_id, r.id) = (mr.workspace_id, mr.role_id)
             JOIN suite.role_permissions rp ON (rp.workspace_id, rp.role_id) = (r.workspace_id, r.id)
             WHERE mr.workspace_id = w.id AND mr.membership_id = m.id
               AND r.archived_at IS NULL AND rp.permission_key = 'workspace.read'
           ) ORDER BY w.created_at, w.id"
    ).bind(identity.issuer()).bind(identity.subject()).fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|(id, name)| Workspace { id, name })
        .collect())
}
