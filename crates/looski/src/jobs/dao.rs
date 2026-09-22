// SPDX-License-Identifier: AGPL-3.0-only
use super::{Job, JobEvent, JobQuery, RecordedIdentity};
use crate::{
    access::Access,
    application::{ActorContext, OperationError as Error},
};
use sqlx::{PgConnection, PgPool, Row, types::Json};
use uuid::Uuid;
const JOB: &str = "SELECT jsonb_build_object('id',id,'kind',kind,'state',state,'attempts',attempts,'error_code',error_code,'version',version,'created_at',created_at::text,'updated_at',updated_at::text) FROM suite.jobs";
pub(super) async fn get(db: &mut PgConnection, ws: Uuid, id: Uuid) -> Result<Job, Error> {
    let sql = format!("{JOB} WHERE workspace_id=$1 AND id=$2");
    Ok(
        sqlx::query_scalar::<_, Json<Job>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
pub(super) async fn lock(db: &mut PgConnection, ws: Uuid, id: Uuid) -> Result<Job, Error> {
    sqlx::query("SELECT id FROM suite.jobs WHERE workspace_id=$1 AND id=$2 FOR UPDATE")
        .bind(ws)
        .bind(id)
        .fetch_optional(&mut *db)
        .await?
        .ok_or(Error::NotFound)?;
    get(db, ws, id).await
}
pub(super) async fn list(
    db: &mut PgConnection,
    ws: Uuid,
    q: &JobQuery,
    n: i64,
) -> Result<Vec<Job>, Error> {
    if let Some(id) = q.after {
        get(db, ws, id).await?;
    }
    let sql = format!(
        "{JOB} WHERE workspace_id=$1 AND ($2::uuid IS NULL OR (created_at,id)>(SELECT created_at,id FROM suite.jobs WHERE workspace_id=$1 AND id=$2)) ORDER BY created_at,id LIMIT $3"
    );
    Ok(
        sqlx::query_scalar::<_, Json<Job>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(q.after)
            .bind(n)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|r| r.0)
            .collect(),
    )
}
pub(super) async fn events(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    q: &JobQuery,
    n: i64,
) -> Result<Vec<JobEvent>, Error> {
    if let Some(after)=q.after&&!sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM suite.job_events WHERE workspace_id=$1 AND job_id=$2 AND id=$3)").bind(ws).bind(id).bind(after).fetch_one(&mut *db).await?{return Err(Error::NotFound)}
    Ok(sqlx::query_scalar::<_,Json<JobEvent>>("SELECT jsonb_build_object('id',id,'state',state,'attempt',attempt,'error_code',error_code,'created_at',created_at::text) FROM suite.job_events WHERE workspace_id=$1 AND job_id=$2 AND ($3::uuid IS NULL OR event_sequence>(SELECT event_sequence FROM suite.job_events WHERE workspace_id=$1 AND job_id=$2 AND id=$3)) ORDER BY event_sequence LIMIT $4").bind(ws).bind(id).bind(q.after).bind(n).fetch_all(db).await?.into_iter().map(|r|r.0).collect())
}
async fn event(db: &mut PgConnection, ws: Uuid, id: Uuid) -> Result<(), Error> {
    sqlx::query("INSERT INTO suite.job_events(workspace_id,job_id,state,attempt,error_code) SELECT workspace_id,id,state,attempts,error_code FROM suite.jobs WHERE workspace_id=$1 AND id=$2").bind(ws).bind(id).execute(db).await?;
    Ok(())
}
async fn send(db: &mut PgConnection, id: Uuid) -> Result<i64, Error> {
    Ok(
        sqlx::query_scalar("SELECT pgmq.send('looski_jobs',jsonb_build_object('job_id',$1::text))")
            .bind(id.to_string())
            .fetch_one(db)
            .await?,
    )
}
pub(super) async fn enqueue(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    a: &Access,
    dedup: &str,
) -> Result<Uuid, Error> {
    let id = Uuid::new_v4();
    let message = send(db, id).await?;
    sqlx::query("INSERT INTO suite.jobs(id,workspace_id,kind,deduplication_key,actor_user_id,actor_membership_id,actor_issuer,actor_subject,message_id) VALUES($1,$2,'crm.coordination',$3,$4,$5,$6,$7,$8)").bind(id).bind(actor.workspace).bind(dedup).bind(a.user_id).bind(a.membership_id).bind(actor.identity.issuer()).bind(actor.identity.subject()).bind(message).execute(&mut *db).await?;
    event(db, actor.workspace, id).await?;
    Ok(id)
}
async fn remove_message(db: &mut PgConnection, ws: Uuid, id: Uuid) -> Result<(), Error> {
    sqlx::query("SELECT pgmq.delete('looski_jobs',message_id) FROM suite.jobs WHERE workspace_id=$1 AND id=$2 AND message_id IS NOT NULL").bind(ws).bind(id).execute(db).await?;
    Ok(())
}
pub(super) async fn terminal(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    state: &str,
    code: Option<&str>,
) -> Result<(), Error> {
    remove_message(db, ws, id).await?;
    sqlx::query("UPDATE suite.jobs SET state=$3,error_code=$4,message_id=NULL,lease_token=NULL,lease_until=NULL,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(ws).bind(id).bind(state).bind(code).execute(&mut *db).await?;
    event(db, ws, id).await
}
pub(super) async fn resume(db: &mut PgConnection, ws: Uuid, id: Uuid) -> Result<(), Error> {
    let message = send(db, id).await?;
    sqlx::query("UPDATE suite.jobs SET state='queued',attempts=0,error_code=NULL,message_id=$3,lease_token=NULL,lease_until=NULL,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(ws).bind(id).bind(message).execute(&mut *db).await?;
    event(db, ws, id).await
}
pub(super) async fn block_crm(db: &mut PgConnection, ws: Uuid) -> Result<(), Error> {
    sqlx::query("WITH locked AS MATERIALIZED (SELECT id,message_id FROM suite.jobs WHERE workspace_id=$1 AND kind='crm.coordination' AND state IN ('queued','running') ORDER BY id FOR UPDATE), removed AS MATERIALIZED (SELECT pgmq.delete('looski_jobs',message_id) FROM locked WHERE message_id IS NOT NULL), changed AS (UPDATE suite.jobs SET state='blocked',error_code='module_disabled',message_id=NULL,lease_token=NULL,lease_until=NULL,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id IN (SELECT id FROM locked) AND (SELECT count(*) FROM removed)>=0 RETURNING workspace_id,id,state,attempts,error_code) INSERT INTO suite.job_events(workspace_id,job_id,state,attempt,error_code) SELECT workspace_id,id,state,attempts,error_code FROM changed").bind(ws).execute(db).await?;
    Ok(())
}
pub(super) struct Notification {
    id: Option<Uuid>,
    message: i64,
}
/// Commit queue visibility before locking application state, avoiding inverse
/// queue-row/job-row lock order with cancellation and module disablement.
pub(super) async fn notification(pool: &PgPool) -> Result<Option<Notification>, Error> {
    let row = sqlx::query_as::<_, (i64, Json<serde_json::Value>)>(
        "SELECT msg_id,message FROM pgmq.read('looski_jobs',120,1)",
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(message, body)| Notification {
        id: body
            .0
            .get("job_id")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok()),
        message,
    }))
}
pub(super) struct Claim {
    pub id: Uuid,
    pub workspace: Uuid,
    pub token: Uuid,
    pub identity: RecordedIdentity,
    pub user: Uuid,
    pub member: Uuid,
    pub attempt: i32,
}
pub(super) async fn claim(
    pool: &PgPool,
    notification: Notification,
) -> Result<Option<Claim>, Error> {
    let mut tx = pool.begin().await?;
    let row=sqlx::query("SELECT workspace_id,state,attempts,message_id,lease_until>clock_timestamp() AS leased,actor_issuer,actor_subject,actor_user_id,actor_membership_id FROM suite.jobs WHERE id=$1 FOR UPDATE").bind(notification.id).fetch_optional(&mut *tx).await?;
    let Some(row) = row else {
        sqlx::query("SELECT pgmq.delete('looski_jobs',$1)")
            .bind(notification.message)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Ok(None);
    };
    let id = notification.id.expect("queried nonnull id");
    let ws: Uuid = row.try_get("workspace_id")?;
    let state: String = row.try_get("state")?;
    if row.try_get::<Option<i64>, _>("message_id")? != Some(notification.message)
        || !["queued", "running"].contains(&state.as_str())
    {
        sqlx::query("SELECT pgmq.delete('looski_jobs',$1)")
            .bind(notification.message)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Ok(None);
    }
    if row.try_get::<Option<bool>, _>("leased")?.unwrap_or(false) {
        return Ok(None);
    }
    let attempts: i32 = row.try_get("attempts")?;
    if attempts >= 5 {
        terminal(&mut tx, ws, id, "exhausted", Some("lease_expired")).await?;
        tx.commit().await?;
        return Ok(None);
    }
    let token = Uuid::new_v4();
    sqlx::query("UPDATE suite.jobs SET state='running',attempts=attempts+1,lease_token=$2,lease_until=clock_timestamp()+interval '120 seconds',error_code=NULL,version=version+1,updated_at=now() WHERE id=$1").bind(id).bind(token).execute(&mut *tx).await?;
    event(&mut tx, ws, id).await?;
    tx.commit().await?;
    Ok(Some(Claim {
        id,
        workspace: ws,
        token,
        identity: RecordedIdentity {
            issuer: row.try_get("actor_issuer")?,
            subject: row.try_get("actor_subject")?,
        },
        user: row.try_get("actor_user_id")?,
        member: row.try_get("actor_membership_id")?,
        attempt: attempts + 1,
    }))
}
pub(super) async fn fenced(db: &mut PgConnection, c: &Claim) -> Result<bool, Error> {
    Ok(sqlx::query_scalar::<_,bool>("SELECT state='running' AND lease_token=$3 AND lease_until>clock_timestamp() FROM suite.jobs WHERE workspace_id=$1 AND id=$2 FOR UPDATE").bind(c.workspace).bind(c.id).bind(c.token).fetch_optional(db).await?.unwrap_or(false))
}
pub(super) async fn finish(db: &mut PgConnection, c: &Claim) -> Result<bool, Error> {
    if !fenced(db, c).await? {
        return Ok(false);
    }
    terminal(db, c.workspace, c.id, "completed", None).await?;
    Ok(true)
}
pub(super) async fn failure(
    pool: &PgPool,
    c: &Claim,
    code: &str,
    retry: bool,
) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    if !fenced(&mut tx, c).await? {
        return Ok(());
    }
    if retry && c.attempt < 5 {
        sqlx::query("SELECT pgmq.set_vt('looski_jobs',message_id,$3) FROM suite.jobs WHERE workspace_id=$1 AND id=$2").bind(c.workspace).bind(c.id).bind(1_i32<<c.attempt).execute(&mut *tx).await?;
        sqlx::query("UPDATE suite.jobs SET state='queued',lease_token=NULL,lease_until=NULL,error_code=$3,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(c.workspace).bind(c.id).bind(code).execute(&mut *tx).await?;
        event(&mut tx, c.workspace, c.id).await?;
    } else {
        terminal(
            &mut tx,
            c.workspace,
            c.id,
            if retry { "exhausted" } else { "blocked" },
            Some(code),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}
