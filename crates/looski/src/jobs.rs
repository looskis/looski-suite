// SPDX-License-Identifier: AGPL-3.0-only
//! Durable orchestration state. Payloads and business effects remain module-owned.
mod dao;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    identity::Identity,
    modules, reliability,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
/// Sealed record loaded only by the Jobs DAO, never deserialized from requests.
pub(crate) struct RecordedIdentity {
    issuer: String,
    subject: String,
}
impl RecordedIdentity {
    pub(crate) fn issuer(&self) -> &str {
        &self.issuer
    }
    pub(crate) fn subject(&self) -> &str {
        &self.subject
    }
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Job {
    pub context: Option<crate::crm::status_jobs::JobContext>,
    pub id: Uuid,
    pub kind: String,
    pub state: String,
    pub attempts: i32,
    pub error_code: Option<String>,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct JobEvent {
    pub id: Uuid,
    pub state: String,
    pub attempt: i32,
    pub error_code: Option<String>,
    pub created_at: String,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct JobQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct JobPage {
    pub items: Vec<Job>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct JobEventPage {
    pub items: Vec<JobEvent>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobAction {
    Resume,
    Cancel,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ChangeJob {
    pub expected_version: i64,
    pub action: JobAction,
}
fn limit(n: Option<i64>) -> Result<i64, Error> {
    let n = n.unwrap_or(25);
    if !(1..=100).contains(&n) {
        return Err(Error::Invalid("Job page limit must be 1–100"));
    }
    Ok(n)
}
async fn require(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    action: &str,
) -> Result<access::Access, Error> {
    let a = access::require(db, actor.identity, actor.workspace, action).await?;
    if !a.workspace_wide {
        return Err(Error::Forbidden);
    }
    Ok(a)
}
pub async fn list(pool: &PgPool, actor: &ActorContext<'_>, q: JobQuery) -> Result<JobPage, Error> {
    let n = limit(q.limit)?;
    let mut tx = pool.begin().await?;
    require(&mut tx, actor, "workspace.jobs.read").await?;
    let mut items = dao::list(&mut tx, actor.workspace, &q, n + 1).await?;
    let next_cursor = if items.len() > n as usize {
        items.pop();
        items.last().map(|j| j.id)
    } else {
        None
    };
    Ok(JobPage { items, next_cursor })
}
pub async fn get(pool: &PgPool, actor: &ActorContext<'_>, id: Uuid) -> Result<Job, Error> {
    let mut tx = pool.begin().await?;
    require(&mut tx, actor, "workspace.jobs.read").await?;
    let mut job = dao::get(&mut tx, actor.workspace, id).await?;
    job.context = crate::crm::status_jobs::context(&mut tx, actor, id).await?;
    Ok(job)
}
pub async fn history(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    q: JobQuery,
) -> Result<JobEventPage, Error> {
    let n = limit(q.limit)?;
    let mut tx = pool.begin().await?;
    require(&mut tx, actor, "workspace.jobs.read").await?;
    dao::get(&mut tx, actor.workspace, id).await?;
    let mut items = dao::events(&mut tx, actor.workspace, id, &q, n + 1).await?;
    let next_cursor = if items.len() > n as usize {
        items.pop();
        items.last().map(|j| j.id)
    } else {
        None
    };
    Ok(JobEventPage { items, next_cursor })
}
pub async fn change(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: ChangeJob,
) -> Result<Job, Error> {
    let mut tx = pool.begin().await?;
    require(&mut tx, actor, "workspace.jobs.manage").await?;
    modules::lock_workspace(&mut tx, actor.workspace).await?;
    let a = require(&mut tx, actor, "workspace.jobs.manage").await?;
    require(&mut tx, actor, "workspace.jobs.read").await?;
    if matches!(&input.action, JobAction::Resume) {
        modules::require_crm(&mut tx, actor).await?;
    }
    let operation = format!("jobs.change:{id}");
    let request = serde_json::to_value(&input).expect("job serializes");
    if let Some(saved) =
        reliability::replay(&mut tx, actor.workspace, &a, &operation, key, &request).await?
    {
        return Ok(saved);
    }
    let current = dao::lock(&mut tx, actor.workspace, id).await?;
    if current.version != input.expected_version {
        return Err(Error::Conflict("Job changed; reload before editing"));
    }
    match input.action {
        JobAction::Resume => {
            if !["blocked", "exhausted"].contains(&current.state.as_str()) {
                return Err(Error::Conflict("Only blocked or exhausted jobs can resume"));
            }
            modules::require_crm(&mut tx, actor).await?;
            dao::resume(&mut tx, actor.workspace, id).await?;
        }
        JobAction::Cancel => {
            if !["queued", "blocked", "exhausted"].contains(&current.state.as_str()) {
                return Err(Error::Conflict(
                    "Only queued, blocked or exhausted jobs can be cancelled",
                ));
            }
            dao::terminal(&mut tx, actor.workspace, id, "cancelled", None).await?;
        }
    }
    reliability::audit(
        &mut tx,
        actor,
        &a,
        "jobs.changed",
        id,
        serde_json::json!({"action":input.action}),
    )
    .await?;
    let result = dao::get(&mut tx, actor.workspace, id).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &a,
        &operation,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
pub(crate) async fn enqueue(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    a: &access::Access,
    dedup: &str,
) -> Result<Uuid, Error> {
    dao::enqueue(db, actor, a, dedup).await
}
pub(crate) async fn block_crm(db: &mut PgConnection, workspace: Uuid) -> Result<(), Error> {
    dao::block_crm(db, workspace).await
}
/// Process at most one notification. The database lease/fence owns execution,
/// so a stale worker cannot commit after another worker reclaims its job.
pub async fn run_once(pool: &PgPool) -> Result<bool, Error> {
    let Some(message) = dao::notification(pool).await? else {
        return Ok(false);
    };
    let Some(claim) = dao::claim(pool, message).await? else {
        return Ok(true);
    };
    if let Err(error) = execute(pool, &claim).await {
        let (code, retry) = match error {
            Error::Forbidden => ("authority", false),
            Error::NotFound | Error::Conflict(_) => ("unavailable", false),
            Error::Invalid(_) => ("invalid_input", false),
            Error::Database(_) => ("database", true),
        };
        dao::failure(pool, &claim, code, retry).await?;
    }
    Ok(true)
}
async fn execute(pool: &PgPool, claim: &dao::Claim) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    modules::lock_workspace(&mut tx, claim.workspace).await?;
    if !dao::fenced(&mut tx, claim).await? {
        return Ok(());
    }
    let identity = Identity::resume(&claim.identity);
    let actor = ActorContext {
        identity: &identity,
        workspace: claim.workspace,
        request_id: claim.id,
        source_interface: "worker",
    };
    let authority = access::require(
        &mut tx,
        &identity,
        claim.workspace,
        "crm.deals.coordination.apply",
    )
    .await?;
    if authority.user_id != claim.user || authority.membership_id != claim.member {
        return Err(Error::Forbidden);
    }
    crate::crm::status_jobs::execute(&mut tx, &actor, claim.id).await?;
    if !dao::finish(&mut tx, claim).await? {
        return Err(Error::Conflict("Job lease expired"));
    }
    reliability::audit(
        &mut tx,
        &actor,
        &authority,
        "jobs.completed",
        claim.id,
        serde_json::json!({"attempt":claim.attempt}),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
