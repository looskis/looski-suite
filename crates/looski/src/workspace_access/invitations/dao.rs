// SPDX-License-Identifier: AGPL-3.0-only
use super::{Invitation, Record};
use crate::{
    access::Access,
    application::{ActorContext, OperationError as Error},
    workspace_access::AccessQuery,
};
use sqlx::{PgConnection, types::Json};
use uuid::Uuid;
const RECORD: &str = "SELECT jsonb_build_object('invitation',jsonb_build_object('id',i.id,'workspace_id',i.workspace_id,'email',i.email,'state',CASE WHEN i.state='pending' AND i.expires_at<=clock_timestamp() THEN 'expired' ELSE i.state END,'expires_at',to_char(i.expires_at AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),'created_at',to_char(i.created_at AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"'),'version',i.version,'inviter_membership_id',i.inviter_membership_id,'accepted_membership_id',i.accepted_membership_id,'roles',(SELECT COALESCE(jsonb_agg(jsonb_build_object('id',role_id,'name',role_name,'version',role_version) ORDER BY role_id),'[]'::jsonb) FROM suite.staff_invitation_roles WHERE workspace_id=i.workspace_id AND invitation_id=i.id)),'nonce',i.nonce,'expires_epoch',floor(extract(epoch FROM i.expires_at))::bigint,'inviter_identity_id',i.inviter_identity_id) FROM suite.staff_invitations i";
pub(super) async fn record(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
) -> Result<Record, Error> {
    let sql = format!("{RECORD} WHERE i.workspace_id=$1 AND i.id=$2");
    Ok(
        sqlx::query_scalar::<_, Json<Record>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
pub(super) async fn list(
    db: &mut PgConnection,
    workspace: Uuid,
    query: &AccessQuery,
    limit: i64,
) -> Result<Vec<Invitation>, Error> {
    if let Some(id) = query.after {
        record(db, workspace, id).await?;
    }
    let sql = format!(
        "{RECORD} WHERE i.workspace_id=$1 AND ($2::uuid IS NULL OR (i.created_at,i.id)>(SELECT created_at,id FROM suite.staff_invitations WHERE workspace_id=$1 AND id=$2)) ORDER BY i.created_at,i.id LIMIT $3"
    );
    Ok(
        sqlx::query_scalar::<_, Json<Record>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(query.after)
            .bind(limit)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|v| v.0.invitation)
            .collect(),
    )
}
pub(super) async fn create(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    access: &Access,
    email: &str,
    hours: i64,
    roles: &[Uuid],
) -> Result<Uuid, Error> {
    let id=sqlx::query_scalar::<_,Uuid>("INSERT INTO suite.staff_invitations(workspace_id,email,inviter_membership_id,inviter_identity_id,expires_at) SELECT $1,$2,$3,id,date_trunc('second',now())+make_interval(hours=>$6::int) FROM suite.auth_identities WHERE issuer=$4 AND subject=$5 AND disabled_at IS NULL RETURNING id")
 .bind(actor.workspace).bind(email).bind(access.membership_id).bind(actor.identity.issuer()).bind(actor.identity.subject()).bind(hours).fetch_one(&mut *db).await?;
    sqlx::query("INSERT INTO suite.staff_invitation_roles(workspace_id,invitation_id,role_id,role_version,role_name) SELECT workspace_id,$2,id,version,name FROM suite.roles WHERE workspace_id=$1 AND id=ANY($3)").bind(actor.workspace).bind(id).bind(roles).execute(db).await?;
    Ok(id)
}
pub(super) async fn revoke(db: &mut PgConnection, workspace: Uuid, id: Uuid) -> Result<(), Error> {
    sqlx::query("UPDATE suite.staff_invitations SET state='revoked',revoked_at=now(),updated_at=now(),version=version+1 WHERE workspace_id=$1 AND id=$2").bind(workspace).bind(id).execute(db).await?;
    Ok(())
}
pub(super) async fn require_inviter(
    db: &mut PgConnection,
    workspace: Uuid,
    record: &Record,
) -> Result<(), Error> {
    let allowed=sqlx::query_scalar::<_,bool>("SELECT COALESCE(bool_or(p.permission_key='workspace.access.admin') AND bool_or(p.permission_key='workspace.access.read'),false) FROM suite.memberships m JOIN suite.users u ON u.id=m.user_id JOIN suite.auth_identities i ON i.user_id=u.id JOIN suite.workspaces w ON w.id=m.workspace_id JOIN suite.membership_roles mr ON (mr.workspace_id,mr.membership_id)=(m.workspace_id,m.id) JOIN suite.roles r ON (r.workspace_id,r.id)=(mr.workspace_id,mr.role_id) JOIN suite.role_permissions p ON (p.workspace_id,p.role_id)=(r.workspace_id,r.id) WHERE m.workspace_id=$1 AND m.id=$2 AND i.id=$3 AND i.disabled_at IS NULL AND u.state='active' AND m.state='active' AND w.state='active' AND r.archived_at IS NULL AND p.record_scope='workspace'")
 .bind(workspace).bind(record.invitation.inviter_membership_id).bind(record.inviter_identity_id).fetch_one(db).await?;
    if !allowed {
        return Err(Error::Forbidden);
    }
    Ok(())
}
pub(super) async fn recipient(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    name: &str,
    accepted: Option<Uuid>,
) -> Result<Access, Error> {
    // Serialize first mapping across different workspace invitations for this identity.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(format!(
            "identity:{}:{}",
            actor.identity.issuer(),
            actor.identity.subject()
        ))
        .execute(&mut *db)
        .await?;
    let mapped=sqlx::query_as::<_,(Uuid,bool)>("SELECT u.id,u.state='active' AND i.disabled_at IS NULL FROM suite.auth_identities i JOIN suite.users u ON u.id=i.user_id WHERE i.issuer=$1 AND i.subject=$2").bind(actor.identity.issuer()).bind(actor.identity.subject()).fetch_optional(&mut *db).await?;
    let user = match mapped {
        Some((id, true)) => id,
        Some((_, false)) => return Err(Error::Forbidden),
        None => {
            if accepted.is_some() {
                return Err(Error::Forbidden);
            }
            let id = sqlx::query_scalar::<_, Uuid>(
                "INSERT INTO suite.users(display_name) VALUES($1) RETURNING id",
            )
            .bind(name)
            .fetch_one(&mut *db)
            .await?;
            sqlx::query(
                "INSERT INTO suite.auth_identities(user_id,issuer,subject) VALUES($1,$2,$3)",
            )
            .bind(id)
            .bind(actor.identity.issuer())
            .bind(actor.identity.subject())
            .execute(&mut *db)
            .await?;
            id
        }
    };
    let existing = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id,state FROM suite.memberships WHERE workspace_id=$1 AND user_id=$2 FOR UPDATE",
    )
    .bind(actor.workspace)
    .bind(user)
    .fetch_optional(&mut *db)
    .await?;
    let membership = match existing {
        Some((id, state))
            if state == "active" && accepted.is_none_or(|previous| previous == id) =>
        {
            id
        }
        Some(_) => return Err(Error::Forbidden),
        None => {
            if accepted.is_some() {
                return Err(Error::Forbidden);
            }
            sqlx::query_scalar::<_, Uuid>(
                "INSERT INTO suite.memberships(workspace_id,user_id) VALUES($1,$2) RETURNING id",
            )
            .bind(actor.workspace)
            .bind(user)
            .fetch_one(&mut *db)
            .await?
        }
    };
    Ok(Access {
        user_id: user,
        membership_id: membership,
        workspace_wide: false,
    })
}
pub(super) async fn accept(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    membership: Uuid,
) -> Result<(), Error> {
    let count = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM (SELECT role_id FROM suite.membership_roles WHERE workspace_id=$1 AND membership_id=$3 UNION SELECT role_id FROM suite.staff_invitation_roles WHERE workspace_id=$1 AND invitation_id=$2) roles")
        .bind(workspace).bind(id).bind(membership).fetch_one(&mut *db).await?;
    if count > 50 {
        return Err(Error::Conflict(
            "Membership would exceed 50 roles; review existing roles first",
        ));
    }
    // This is the authoritative grant point, after workspace and identity locks.
    // Transaction-start now() must not extend an invitation while a lock waits.
    let accepted = sqlx::query("UPDATE suite.staff_invitations SET state='accepted',accepted_membership_id=$3,accepted_at=clock_timestamp(),updated_at=clock_timestamp(),version=version+1 WHERE workspace_id=$1 AND id=$2 AND state='pending' AND expires_at>clock_timestamp()")
        .bind(workspace).bind(id).bind(membership).execute(&mut *db).await?;
    if accepted.rows_affected() != 1 {
        return Err(Error::Conflict("Invitation is no longer pending"));
    }
    sqlx::query("INSERT INTO suite.membership_roles(workspace_id,membership_id,role_id) SELECT workspace_id,$3,role_id FROM suite.staff_invitation_roles WHERE workspace_id=$1 AND invitation_id=$2 ON CONFLICT DO NOTHING").bind(workspace).bind(id).bind(membership).execute(&mut *db).await?;
    sqlx::query("UPDATE suite.memberships SET version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(workspace).bind(membership).execute(&mut *db).await?;
    Ok(())
}
pub(super) async fn workspace_name(
    db: &mut PgConnection,
    workspace: Uuid,
) -> Result<String, Error> {
    sqlx::query_scalar("SELECT name FROM suite.workspaces WHERE id=$1 AND state='active'")
        .bind(workspace)
        .fetch_optional(db)
        .await?
        .ok_or(Error::Forbidden)
}

pub(super) async fn require_accepted_recipient(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    member: Uuid,
) -> Result<(), Error> {
    let allowed = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM suite.memberships m JOIN suite.users u ON u.id=m.user_id JOIN suite.auth_identities i ON i.user_id=u.id WHERE m.workspace_id=$1 AND m.id=$2 AND m.state='active' AND u.state='active' AND i.disabled_at IS NULL AND i.issuer=$3 AND i.subject=$4)")
        .bind(actor.workspace).bind(member).bind(actor.identity.issuer()).bind(actor.identity.subject()).fetch_one(db).await?;
    if !allowed {
        return Err(Error::Forbidden);
    }
    Ok(())
}

pub(super) async fn delivery_jobs(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
) -> Result<Vec<Uuid>, Error> {
    Ok(sqlx::query_scalar(
        "SELECT job_id FROM suite.invitation_deliveries WHERE workspace_id=$1 AND invitation_id=$2",
    )
    .bind(workspace)
    .bind(id)
    .fetch_all(db)
    .await?)
}
pub(super) async fn add_delivery(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    job: Uuid,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO suite.invitation_deliveries(workspace_id,invitation_id,job_id) VALUES($1,$2,$3)").bind(workspace).bind(id).bind(job).execute(db).await?;
    Ok(())
}
pub(super) async fn delivery_invitation(
    db: &mut PgConnection,
    workspace: Uuid,
    job: Uuid,
) -> Result<Uuid, Error> {
    sqlx::query_scalar(
        "SELECT invitation_id FROM suite.invitation_deliveries WHERE workspace_id=$1 AND job_id=$2",
    )
    .bind(workspace)
    .bind(job)
    .fetch_optional(db)
    .await?
    .ok_or(Error::NotFound)
}
