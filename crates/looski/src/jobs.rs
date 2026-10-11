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
    pub invitation_context:
        Option<crate::workspace_access::invitations::delivery::InvitationJobContext>,
    #[serde(default)]
    pub forms_context: Option<crate::forms::recipes::RunContext>,
    #[serde(default)]
    pub notification_context: Option<crate::mail::NotificationContext>,
    #[serde(default)]
    pub projects_context: Option<crate::projects::recipes::DealConversionContext>,
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
    match job.kind.as_str() {
        "crm.coordination" => {
            job.context = crate::crm::status_jobs::context(&mut tx, actor, id).await?
        }
        "workspace.invitation_email" => {
            job.invitation_context =
                crate::workspace_access::invitations::delivery::context(&mut tx, actor, id).await?
        }
        "forms.crm_inquiry" => {
            job.forms_context = crate::forms::recipes::context(&mut tx, actor, id).await?
        }
        "mail.notification" => {
            job.notification_context = crate::mail::context(&mut tx, actor, id).await?
        }
        "projects.deal_conversion" => {
            job.projects_context = crate::projects::recipes::context(&mut tx, actor, id).await?
        }
        _ => return Err(Error::NotFound),
    }
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
    let current = dao::lock(&mut tx, actor.workspace, id).await?;
    if matches!(&input.action, JobAction::Resume)
        && !modules_ready(&mut tx, actor.workspace, &current.kind, id).await?
    {
        return Err(Error::Forbidden);
    }
    let operation = format!("jobs.change:{id}");
    let request = serde_json::to_value(&input).expect("job serializes");
    if let Some(saved) =
        reliability::replay(&mut tx, actor.workspace, &a, &operation, key, &request).await?
    {
        return Ok(saved);
    }
    if current.version != input.expected_version {
        return Err(Error::Conflict("Job changed; reload before editing"));
    }
    match input.action {
        JobAction::Resume => {
            if !["blocked", "exhausted"].contains(&current.state.as_str()) {
                return Err(Error::Conflict("Only blocked or exhausted jobs can resume"));
            }
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
    dao::enqueue(db, actor, a, dedup, "crm.coordination").await
}
/// Block queued work depending on a newly disabled application; resuming it is
/// an explicit administrative action after re-enabling.
pub(crate) async fn block_module(
    db: &mut PgConnection,
    workspace: Uuid,
    module: &str,
) -> Result<(), Error> {
    let kinds: &[&str] = match module {
        "crm" => &[
            "crm.coordination",
            "forms.crm_inquiry",
            "scheduling.crm_inquiry",
            "projects.deal_conversion",
        ],
        "forms" => &["forms.crm_inquiry"],
        "scheduling" => &["scheduling.crm_inquiry", "scheduling.reminder"],
        "projects" => &["projects.deal_conversion"],
        _ => &[],
    };
    dao::block_module(db, workspace, kinds, module).await
}
/// Applications a job kind needs; a notification needs its purpose's owner.
async fn modules_ready(
    db: &mut PgConnection,
    workspace: Uuid,
    kind: &str,
    id: Uuid,
) -> Result<bool, Error> {
    let purpose;
    let required: Vec<&str> = match kind {
        "crm.coordination" => vec!["crm"],
        "forms.crm_inquiry" => vec!["forms", "crm"],
        "scheduling.crm_inquiry" => {
            if crate::scheduling::needs_forms(db, workspace, id).await? {
                vec!["scheduling", "crm", "forms"]
            } else {
                vec!["scheduling", "crm"]
            }
        }
        "scheduling.reminder" => vec!["scheduling"],
        "projects.deal_conversion" => vec!["projects", "crm"],
        "mail.notification" => {
            purpose = dao::purpose(db, workspace, id).await?;
            vec![purpose.split('.').next().unwrap_or_default()]
        }
        _ => vec![],
    };
    for module in required {
        if !modules::enabled(db, workspace, module).await? {
            return Ok(false);
        }
    }
    Ok(true)
}
#[allow(clippy::too_many_arguments)]
pub(crate) async fn enqueue_recorded(
    db: &mut PgConnection,
    workspace: Uuid,
    user: Uuid,
    member: Uuid,
    issuer: &str,
    subject: &str,
    dedup: &str,
    kind: &str,
) -> Result<Uuid, Error> {
    dao::enqueue_recorded(db, workspace, user, member, issuer, subject, dedup, kind).await
}
/// Process at most one notification. The database lease/fence owns execution,
/// so a stale worker cannot commit after another worker reclaims its job.
pub async fn run_once(
    pool: &PgPool,
    mailer: &crate::workspace_access::invitations::delivery::Mailer,
    outbox: &crate::mail::Outbox,
) -> Result<bool, Error> {
    let Some(message) = dao::notification(pool).await? else {
        return Ok(false);
    };
    let Some(claim) = dao::claim(pool, message).await? else {
        return Ok(true);
    };
    if let Err(stop) = execute(pool, &claim, mailer, outbox).await {
        let (code, retry) = match stop {
            Stop::ModuleDisabled => ("module_disabled", false),
            Stop::Failed(Error::Forbidden) => ("authority", false),
            Stop::Failed(Error::NotFound | Error::Conflict(_)) => ("unavailable", false),
            Stop::Failed(Error::Invalid(_)) => ("invalid_input", false),
            Stop::Failed(Error::Database(_)) => ("database", true),
            Stop::Failed(Error::Unavailable) => ("provider", true),
        };
        dao::failure(pool, &claim, code, retry).await?;
    }
    Ok(true)
}
enum Stop {
    ModuleDisabled,
    Failed(Error),
}
impl From<Error> for Stop {
    fn from(error: Error) -> Self {
        Self::Failed(error)
    }
}
impl From<sqlx::Error> for Stop {
    fn from(error: sqlx::Error) -> Self {
        Self::Failed(error.into())
    }
}
async fn execute(
    pool: &PgPool,
    claim: &dao::Claim,
    mailer: &crate::workspace_access::invitations::delivery::Mailer,
    outbox: &crate::mail::Outbox,
) -> Result<(), Stop> {
    let mut tx = pool.begin().await?;
    modules::lock_workspace(&mut tx, claim.workspace).await?;
    if !dao::fenced(&mut tx, claim).await? {
        return Ok(());
    }
    // The workspace lock serializes this check with application disablement.
    if !modules_ready(&mut tx, claim.workspace, &claim.kind, claim.id).await? {
        return Err(Stop::ModuleDisabled);
    }
    let identity = Identity::resume(&claim.identity);
    let actor = ActorContext {
        identity: &identity,
        workspace: claim.workspace,
        request_id: claim.id,
        source_interface: "worker",
    };
    let permission = match claim.kind.as_str() {
        "crm.coordination" => "crm.deals.coordination.apply",
        "workspace.invitation_email" => "workspace.access.admin",
        "forms.crm_inquiry" => "forms.recipes.manage",
        "scheduling.crm_inquiry" => "scheduling.recipes.manage",
        "scheduling.reminder" => "scheduling.manage",
        "projects.deal_conversion" => "projects.recipes.manage",
        "mail.notification" => {
            crate::mail::authority(&dao::purpose(&mut tx, claim.workspace, claim.id).await?)
        }
        _ => return Err(Error::NotFound.into()),
    };
    let authority = access::require(&mut tx, &identity, claim.workspace, permission).await?;
    if authority.user_id != claim.user || authority.membership_id != claim.member {
        return Err(Error::Forbidden.into());
    }
    match claim.kind.as_str() {
        "crm.coordination" => crate::crm::status_jobs::execute(&mut tx, &actor, claim.id).await?,
        "workspace.invitation_email" => {
            crate::workspace_access::invitations::delivery::execute(
                &mut tx, &actor, claim.id, mailer,
            )
            .await?
        }
        "forms.crm_inquiry" => crate::forms::recipes::execute(&mut tx, &actor, claim.id).await?,
        "scheduling.crm_inquiry" => crate::scheduling::execute(&mut tx, &actor, claim.id).await?,
        "scheduling.reminder" => {
            crate::scheduling::reminder(&mut tx, &actor, claim.id, outbox).await?
        }
        "projects.deal_conversion" => {
            crate::projects::recipes::execute(&mut tx, &actor, claim.id).await?
        }
        "mail.notification" => crate::mail::execute(&mut tx, &actor, claim.id, outbox).await?,
        _ => return Err(Error::NotFound.into()),
    }
    if !dao::finish(&mut tx, claim).await? {
        return Err(Error::Conflict("Job lease expired").into());
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

pub(crate) async fn enqueue_invitation(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    a: &access::Access,
    dedup: &str,
) -> Result<Uuid, Error> {
    dao::enqueue(db, actor, a, dedup, "workspace.invitation_email").await
}
/// Resume a blocked or exhausted job from the owning module's retry operation.
pub(crate) async fn resume_in(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
) -> Result<(), Error> {
    let current = dao::lock(db, workspace, id).await?;
    if !["blocked", "exhausted"].contains(&current.state.as_str()) {
        return Err(Error::Conflict("Only blocked or exhausted jobs can resume"));
    }
    if !modules_ready(db, workspace, &current.kind, id).await? {
        return Err(Error::Forbidden);
    }
    dao::resume(db, workspace, id).await
}
pub(crate) async fn record(db: &mut PgConnection, workspace: Uuid, id: Uuid) -> Result<Job, Error> {
    dao::get(db, workspace, id).await
}

pub(crate) async fn pending(
    db: &mut PgConnection,
    workspace: Uuid,
    ids: &[Uuid],
) -> Result<bool, Error> {
    dao::pending(db, workspace, ids).await
}
