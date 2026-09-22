// SPDX-License-Identifier: AGPL-3.0-only
use super::{AccessQuery, EditMember, Member, Role, SaveRole};
use crate::application::OperationError as Error;
use sqlx::{PgConnection, types::Json};
use uuid::Uuid;
const MEMBER_SELECT: &str = "SELECT jsonb_build_object('id',m.id,'user_id',m.user_id,'display_name',u.display_name,'state',m.state,'version',m.version,'roles',COALESCE((SELECT jsonb_agg(jsonb_build_object('id',r.id,'name',r.name,'archived',r.archived_at IS NOT NULL) ORDER BY r.id) FROM suite.membership_roles mr JOIN suite.roles r ON (r.workspace_id,r.id)=(mr.workspace_id,mr.role_id) WHERE mr.workspace_id=m.workspace_id AND mr.membership_id=m.id),'[]'::jsonb)) FROM suite.memberships m JOIN suite.users u ON u.id=m.user_id";
const ROLE_SELECT: &str = "SELECT jsonb_build_object('id',r.id,'name',r.name,'archived',r.archived_at IS NOT NULL,'version',r.version,'grants',COALESCE((SELECT jsonb_agg(jsonb_build_object('action',permission_key,'scope',record_scope) ORDER BY permission_key,record_scope) FROM suite.role_permissions WHERE workspace_id=r.workspace_id AND role_id=r.id),'[]'::jsonb)) FROM suite.roles r";
pub(super) async fn lock(db: &mut PgConnection, workspace: Uuid) -> Result<(), Error> {
    sqlx::query("SELECT id FROM suite.workspaces WHERE id=$1 FOR NO KEY UPDATE")
        .bind(workspace)
        .fetch_optional(db)
        .await?
        .ok_or(Error::NotFound)?;
    Ok(())
}
pub(super) async fn role(db: &mut PgConnection, workspace: Uuid, id: Uuid) -> Result<Role, Error> {
    let sql = format!("{ROLE_SELECT} WHERE r.workspace_id=$1 AND r.id=$2");
    let row = sqlx::query_scalar::<_, Json<Role>>(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(workspace)
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or(Error::NotFound)?;
    Ok(row.0)
}
pub(super) async fn roles(
    db: &mut PgConnection,
    workspace: Uuid,
    query: &AccessQuery,
    limit: i64,
) -> Result<Vec<Role>, Error> {
    if let Some(after) = query.after {
        role(db, workspace, after).await?;
    }
    let sql = format!(
        "{ROLE_SELECT} WHERE r.workspace_id=$1 AND ($2::uuid IS NULL OR (r.created_at,r.id)>(SELECT created_at,id FROM suite.roles WHERE workspace_id=$1 AND id=$2)) ORDER BY r.created_at,r.id LIMIT $3"
    );
    Ok(
        sqlx::query_scalar::<_, Json<Role>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(query.after)
            .bind(limit)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|row| row.0)
            .collect(),
    )
}
pub(super) async fn member(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
) -> Result<Member, Error> {
    let sql = format!("{MEMBER_SELECT} WHERE m.workspace_id=$1 AND m.id=$2");
    let row = sqlx::query_scalar::<_, Json<Member>>(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(workspace)
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or(Error::NotFound)?;
    Ok(row.0)
}
pub(super) async fn members(
    db: &mut PgConnection,
    workspace: Uuid,
    query: &AccessQuery,
    limit: i64,
) -> Result<Vec<Member>, Error> {
    if let Some(after) = query.after {
        member(db, workspace, after).await?;
    }
    let sql = format!(
        "{MEMBER_SELECT} WHERE m.workspace_id=$1 AND ($2::uuid IS NULL OR (m.joined_at,m.id)>(SELECT joined_at,id FROM suite.memberships WHERE workspace_id=$1 AND id=$2)) ORDER BY m.joined_at,m.id LIMIT $3"
    );
    Ok(
        sqlx::query_scalar::<_, Json<Member>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(query.after)
            .bind(limit)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|row| row.0)
            .collect(),
    )
}
pub(super) async fn save_role(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Option<Uuid>,
    input: &SaveRole,
) -> Result<Uuid, Error> {
    let id = if let Some(id) = id {
        sqlx::query("UPDATE suite.roles SET name=$3,archived_at=CASE WHEN COALESCE($4,archived_at IS NOT NULL) THEN COALESCE(archived_at,now()) ELSE NULL END,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(workspace).bind(id).bind(input.name.trim()).bind(input.archived).execute(&mut *db).await?;
        id
    } else {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO suite.roles(id,workspace_id,key,name,archived_at) VALUES($1,$2,$1::text,$3,CASE WHEN COALESCE($4,false) THEN now() ELSE NULL END)").bind(id).bind(workspace).bind(input.name.trim()).bind(input.archived).execute(&mut *db).await?;
        id
    };
    sqlx::query("DELETE FROM suite.role_permissions WHERE workspace_id=$1 AND role_id=$2")
        .bind(workspace)
        .bind(id)
        .execute(&mut *db)
        .await?;
    for grant in &input.grants {
        sqlx::query("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key,record_scope) VALUES($1,$2,$3,$4)").bind(workspace).bind(id).bind(&grant.action).bind(grant.scope.key()).execute(&mut *db).await?;
    }
    Ok(id)
}
pub(super) async fn edit_member(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    input: &EditMember,
) -> Result<(), Error> {
    sqlx::query("UPDATE suite.memberships SET state=$3,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(workspace).bind(id).bind(input.state.key()).execute(&mut *db).await?;
    sqlx::query("DELETE FROM suite.membership_roles WHERE workspace_id=$1 AND membership_id=$2")
        .bind(workspace)
        .bind(id)
        .execute(&mut *db)
        .await?;
    for role in &input.role_ids {
        sqlx::query("INSERT INTO suite.membership_roles(workspace_id,membership_id,role_id) VALUES($1,$2,$3)").bind(workspace).bind(id).bind(role).execute(&mut *db).await?;
    }
    Ok(())
}
pub(super) async fn require_administrator(
    db: &mut PgConnection,
    workspace: Uuid,
) -> Result<(), Error> {
    let exists=sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT m.id FROM suite.memberships m JOIN suite.users u ON u.id=m.user_id JOIN suite.membership_roles mr ON (mr.workspace_id,mr.membership_id)=(m.workspace_id,m.id) JOIN suite.roles r ON (r.workspace_id,r.id)=(mr.workspace_id,mr.role_id) JOIN suite.role_permissions rp ON (rp.workspace_id,rp.role_id)=(r.workspace_id,r.id) WHERE m.workspace_id=$1 AND m.state='active' AND u.state='active' AND r.archived_at IS NULL AND rp.record_scope='workspace' AND rp.permission_key IN ('workspace.access.admin','workspace.access.read','workspace.read') AND EXISTS(SELECT 1 FROM suite.auth_identities i WHERE i.user_id=u.id AND i.disabled_at IS NULL) GROUP BY m.id HAVING count(DISTINCT rp.permission_key)=3)").bind(workspace).fetch_one(db).await?;
    if !exists {
        return Err(Error::Conflict(
            "At least one active administrator with workspace and access read permissions must remain",
        ));
    }
    Ok(())
}
