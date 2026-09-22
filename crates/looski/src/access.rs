// SPDX-License-Identifier: AGPL-3.0-only
//! Current application authority, independent of transport and token claims.
use crate::{application::OperationError as Error, identity::Identity};
use sqlx::PgConnection;
use uuid::Uuid;

pub struct Access {
    pub user_id: Uuid,
    pub membership_id: Uuid,
    pub workspace_wide: bool,
}
impl Access {
    pub fn permits_owner(&self, owner: Uuid) -> bool {
        self.workspace_wide || self.membership_id == owner
    }
}

pub async fn require(
    db: &mut PgConnection,
    identity: &Identity,
    workspace: Uuid,
    action: &str,
) -> Result<Access, Error> {
    let row = sqlx::query_as::<_, (Uuid, Uuid, bool)>(
        "SELECT u.id,m.id,bool_or(rp.record_scope='workspace')
         FROM suite.users u JOIN suite.auth_identities i ON i.user_id=u.id
         JOIN suite.memberships m ON m.user_id=u.id JOIN suite.workspaces w ON w.id=m.workspace_id
         JOIN suite.membership_roles mr ON (mr.workspace_id,mr.membership_id)=(w.id,m.id)
         JOIN suite.roles r ON (r.workspace_id,r.id)=(w.id,mr.role_id)
         JOIN suite.role_permissions rp ON (rp.workspace_id,rp.role_id)=(w.id,r.id)
         WHERE i.issuer=$1 AND i.subject=$2 AND i.disabled_at IS NULL
         AND u.state='active' AND m.state='active' AND w.state='active'
         AND r.archived_at IS NULL AND w.id=$3 AND rp.permission_key=$4
         AND rp.record_scope IN ('workspace','assigned') GROUP BY u.id,m.id",
    )
    .bind(identity.issuer())
    .bind(identity.subject())
    .bind(workspace)
    .bind(action)
    .fetch_optional(db)
    .await?
    .ok_or(Error::Forbidden)?;
    Ok(Access {
        user_id: row.0,
        membership_id: row.1,
        workspace_wide: row.2,
    })
}

pub async fn require_active_member(
    db: &mut PgConnection,
    workspace: Uuid,
    owner: Uuid,
) -> Result<(), Error> {
    if !sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM suite.memberships WHERE workspace_id=$1 AND id=$2 AND state='active')").bind(workspace).bind(owner).fetch_one(db).await? { return Err(Error::Invalid("Owner must be an active workspace member")); }
    Ok(())
}
