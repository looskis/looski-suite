// SPDX-License-Identifier: AGPL-3.0-only
//! Millisecond accounting: a transition banks elapsed time exactly once, under the workspace lock.
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    modules, projects, reliability,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{AssertSqlSafe, FromRow, PgConnection, PgPool};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
#[derive(Serialize, Deserialize, JsonSchema, ToSchema, FromRow)]
pub struct TimeEntry {
    pub id: Uuid,
    pub membership_id: Uuid,
    pub project_id: Option<Uuid>,
    pub description: String,
    pub work_on: String,
    pub billable: bool,
    pub archived: bool,
    pub state: String,
    pub duration_ms: i64,
    pub running_since: Option<String>,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveTimeEntry {
    pub description: String,
    pub work_on: String,
    pub project_id: Option<Uuid>,
    pub billable: bool,
    pub duration_ms: i64,
    #[serde(default)]
    pub start_timer: bool,
    pub expected_version: Option<i64>,
    #[serde(default)]
    pub archived: bool,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TimerTransition {
    pub expected_version: i64,
    pub action: String,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct TimeQuery {
    pub after: Option<Uuid>,
    pub limit: Option<i64>,
    pub archived: Option<bool>,
    pub project_id: Option<Uuid>,
    pub from_on: Option<String>,
    pub through_on: Option<String>,
    pub billable: Option<bool>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TimeEntryPage {
    pub items: Vec<TimeEntry>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema, FromRow)]
pub struct TimeReport {
    pub count: i64,
    pub duration_ms: i64,
    pub billable_ms: i64,
    pub as_of: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema, FromRow)]
pub struct TimeBudget {
    pub id: Uuid,
    pub name: String,
    pub project_id: Option<Uuid>,
    pub from_on: String,
    pub through_on: String,
    pub budget_ms: i64,
    pub billable_only: bool,
    pub archived: bool,
    pub version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveTimeBudget {
    pub name: String,
    pub project_id: Option<Uuid>,
    pub from_on: String,
    pub through_on: String,
    pub budget_ms: i64,
    pub billable_only: bool,
    pub archived: bool,
    pub expected_version: Option<i64>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TimeBudgetPage {
    pub items: Vec<TimeBudget>,
    pub next_cursor: Option<Uuid>,
}
const COLUMNS: &str = "id,membership_id,project_id,description,work_on::text,billable,archived,state,duration_ms+COALESCE(GREATEST(0,floor(extract(epoch FROM (transaction_timestamp()-running_since))*1000))::bigint,0) AS duration_ms,running_since::text,version,created_at::text,updated_at::text";
const FILTER: &str = "workspace_id=$1 AND ($2 OR membership_id=$3) AND archived=$4 AND ($5::uuid IS NULL OR project_id=$5) AND ($6::date IS NULL OR work_on >= $6::date) AND ($7::date IS NULL OR work_on <= $7::date) AND ($8::boolean IS NULL OR billable=$8)";
async fn authority(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    permission: &str,
) -> Result<access::Access, Error> {
    modules::require(db, a.workspace, "time").await?;
    access::require(db, a.identity, a.workspace, permission).await
}
async fn project(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    id: Option<Uuid>,
    editing: bool,
) -> Result<(), Error> {
    if let Some(id) = id {
        let p = projects::reference(db, a, id).await?;
        if editing && p.archived {
            return Err(Error::Conflict("Project is archived"));
        }
    }
    Ok(())
}
async fn present(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    mut e: TimeEntry,
) -> Result<TimeEntry, Error> {
    if let Some(id) = e.project_id {
        match projects::reference(db, a, id).await {
            Ok(_) => {}
            Err(Error::Forbidden | Error::NotFound) => e.project_id = None,
            Err(e) => return Err(e),
        }
    }
    Ok(e)
}
fn validate_query(q: &TimeQuery) -> Result<i64, Error> {
    crate::tasks::date(q.from_on.as_deref())?;
    crate::tasks::date(q.through_on.as_deref())?;
    if q.from_on
        .as_ref()
        .zip(q.through_on.as_ref())
        .is_some_and(|(f, t)| f > t)
    {
        return Err(Error::Invalid("Date range is reversed"));
    }
    let n = q.limit.unwrap_or(50);
    if !(1..=100).contains(&n) {
        return Err(Error::Invalid("Limit must be 1–100"));
    }
    Ok(n)
}
async fn raw(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    id: Uuid,
    permission: &str,
) -> Result<(access::Access, TimeEntry), Error> {
    let r = authority(db, a, "time.read").await?;
    let p = authority(db, a, permission).await?;
    let sql = format!(
        "SELECT {COLUMNS} FROM time_tracking.entries WHERE workspace_id=$1 AND id=$2 AND ($3 OR membership_id=$4)"
    );
    let e = sqlx::query_as::<_, TimeEntry>(AssertSqlSafe(sql))
        .bind(a.workspace)
        .bind(id)
        .bind(r.workspace_wide)
        .bind(r.membership_id)
        .fetch_optional(db)
        .await?
        .ok_or(Error::NotFound)?;
    if !p.workspace_wide && e.membership_id != p.membership_id {
        return Err(Error::Forbidden);
    }
    Ok((p, e))
}
pub async fn get(pool: &PgPool, a: &ActorContext<'_>, id: Uuid) -> Result<TimeEntry, Error> {
    let mut tx = pool.begin().await?;
    let (_, e) = raw(&mut tx, a, id, "time.read").await?;
    present(&mut tx, a, e).await
}
pub async fn list(
    pool: &PgPool,
    a: &ActorContext<'_>,
    q: TimeQuery,
) -> Result<TimeEntryPage, Error> {
    let n = validate_query(&q)?;
    let mut tx = pool.begin().await?;
    let p = authority(&mut tx, a, "time.read").await?;
    project(&mut tx, a, q.project_id, false).await?;
    let sql = format!(
        "SELECT {COLUMNS} FROM time_tracking.entries WHERE {FILTER} AND ($9::uuid IS NULL OR id>$9) ORDER BY id LIMIT $10"
    );
    let mut rows = sqlx::query_as::<_, TimeEntry>(AssertSqlSafe(sql))
        .bind(a.workspace)
        .bind(p.workspace_wide)
        .bind(p.membership_id)
        .bind(q.archived.unwrap_or(false))
        .bind(q.project_id)
        .bind(q.from_on)
        .bind(q.through_on)
        .bind(q.billable)
        .bind(q.after)
        .bind(n + 1)
        .fetch_all(&mut *tx)
        .await?;
    let more = rows.len() > n as usize;
    rows.truncate(n as usize);
    let next_cursor = if more {
        rows.last().map(|r| r.id)
    } else {
        None
    };
    let mut items = Vec::new();
    for e in rows {
        items.push(present(&mut tx, a, e).await?);
    }
    Ok(TimeEntryPage { items, next_cursor })
}
pub async fn report(
    pool: &PgPool,
    a: &ActorContext<'_>,
    q: TimeQuery,
) -> Result<TimeReport, Error> {
    validate_query(&q)?;
    let mut tx = pool.begin().await?;
    let p = authority(&mut tx, a, "time.read").await?;
    project(&mut tx, a, q.project_id, false).await?;
    let sql = format!(
        "SELECT count(*)::bigint AS count,COALESCE(sum(duration_ms),0)::bigint AS duration_ms,COALESCE(sum(duration_ms) FILTER(WHERE billable),0)::bigint AS billable_ms,transaction_timestamp()::text AS as_of FROM (SELECT {COLUMNS} FROM time_tracking.entries WHERE {FILTER}) entries"
    );
    Ok(sqlx::query_as::<_, TimeReport>(AssertSqlSafe(sql))
        .bind(a.workspace)
        .bind(p.workspace_wide)
        .bind(p.membership_id)
        .bind(q.archived.unwrap_or(false))
        .bind(q.project_id)
        .bind(q.from_on)
        .bind(q.through_on)
        .bind(q.billable)
        .fetch_one(&mut *tx)
        .await?)
}
fn request<T: Serialize>(input: &T) -> serde_json::Value {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(serde_json::to_vec(input).expect("serializes"));
    serde_json::json!({"sha256":hash.iter().map(|b|format!("{b:02x}")).collect::<String>()})
}
async fn available(db: &mut PgConnection, ws: Uuid, member: Uuid) -> Result<(), Error> {
    if sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM time_tracking.entries WHERE workspace_id=$1 AND membership_id=$2 AND state='running')").bind(ws).bind(member).fetch_one(db).await?{return Err(Error::Conflict("Pause or stop your running timer first"));}
    Ok(())
}
pub async fn save(
    pool: &PgPool,
    a: &ActorContext<'_>,
    id: Option<Uuid>,
    key: &str,
    input: SaveTimeEntry,
) -> Result<TimeEntry, Error> {
    let mut tx = pool.begin().await?;
    modules::lock_workspace(&mut tx, a.workspace).await?;
    let p = authority(&mut tx, a, "time.manage").await?;
    authority(&mut tx, a, "time.read").await?;
    let old = if let Some(id) = id {
        Some(raw(&mut tx, a, id, "time.manage").await?.1)
    } else {
        None
    };
    let op = format!("time.save:{id:?}");
    let req = request(&input);
    let result = if let Some(id) =
        reliability::replay::<Uuid>(&mut tx, a.workspace, &p, &op, key, &req).await?
    {
        id
    } else {
        crate::tasks::date(Some(&input.work_on))?;
        if input.description.trim().is_empty()
            || input.description.chars().count() > 2000
            || input.duration_ms < 0
            || input.duration_ms > 31_536_000_000
        {
            return Err(Error::Invalid(
                "Description and duration (up to one year) are required",
            ));
        }
        project(&mut tx, a, input.project_id, true).await?;
        let id = if let Some(old) = old {
            if old.version != input.expected_version.unwrap_or(0) {
                return Err(Error::Conflict("TimeEntry changed; reload"));
            }
            if old.state != "stopped" || input.start_timer {
                return Err(Error::Conflict("Stop the timer before editing"));
            }
            if old.project_id != input.project_id {
                project(&mut tx, a, old.project_id, false).await?;
            }
            sqlx::query("UPDATE time_tracking.entries SET description=$3,work_on=$4::date,project_id=$5,billable=$6,duration_ms=$7,archived=$8,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(a.workspace).bind(old.id).bind(&input.description).bind(&input.work_on).bind(input.project_id).bind(input.billable).bind(input.duration_ms).bind(input.archived).execute(&mut *tx).await?;
            old.id
        } else {
            if input.expected_version.is_some()
                || input.archived
                || input.start_timer && input.duration_ms != 0
            {
                return Err(Error::Invalid(
                    "New timer starts at zero; new entry cannot be archived or versioned",
                ));
            }
            if input.start_timer {
                available(&mut tx, a.workspace, p.membership_id).await?;
            }
            sqlx::query_scalar::<_,Uuid>("INSERT INTO time_tracking.entries(workspace_id,membership_id,description,work_on,project_id,billable,duration_ms,state,running_since) VALUES($1,$2,$3,$4::date,$5,$6,$7,CASE WHEN $8 THEN 'running' ELSE 'stopped' END,CASE WHEN $8 THEN transaction_timestamp() ELSE NULL END) RETURNING id").bind(a.workspace).bind(p.membership_id).bind(&input.description).bind(&input.work_on).bind(input.project_id).bind(input.billable).bind(input.duration_ms).bind(input.start_timer).fetch_one(&mut *tx).await?
        };
        reliability::audit(
            &mut tx,
            a,
            &p,
            "time.entry.saved",
            id,
            serde_json::json!({}),
        )
        .await?;
        reliability::save_replay(&mut tx, a.workspace, &p, &op, key, &req, &id).await?;
        id
    };
    let (_, e) = raw(&mut tx, a, result, "time.read").await?;
    let e = present(&mut tx, a, e).await?;
    tx.commit().await?;
    Ok(e)
}
pub async fn transition(
    pool: &PgPool,
    a: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: TimerTransition,
) -> Result<TimeEntry, Error> {
    let mut tx = pool.begin().await?;
    modules::lock_workspace(&mut tx, a.workspace).await?;
    let (p, e) = raw(&mut tx, a, id, "time.manage").await?;
    if e.membership_id != p.membership_id {
        return Err(Error::Forbidden);
    }
    let op = format!("time.transition:{id}");
    let req = request(&input);
    if reliability::replay::<bool>(&mut tx, a.workspace, &p, &op, key, &req)
        .await?
        .is_none()
    {
        if e.version != input.expected_version {
            return Err(Error::Conflict("Timer changed; reload"));
        }
        let next = match (e.state.as_str(), input.action.as_str()) {
            ("running", "pause") => "paused",
            ("running" | "paused", "stop") => "stopped",
            ("paused", "resume") => "running",
            _ => return Err(Error::Conflict("Invalid timer transition")),
        };
        if next == "running" {
            available(&mut tx, a.workspace, p.membership_id).await?;
        }
        sqlx::query("UPDATE time_tracking.entries SET duration_ms=$3,state=$4,running_since=CASE WHEN $4='running' THEN transaction_timestamp() ELSE NULL END,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(a.workspace).bind(id).bind(e.duration_ms).bind(next).execute(&mut *tx).await?;
        reliability::audit(
            &mut tx,
            a,
            &p,
            "time.timer.changed",
            id,
            serde_json::json!({"state":next}),
        )
        .await?;
        reliability::save_replay(&mut tx, a.workspace, &p, &op, key, &req, &true).await?;
    }
    let (_, e) = raw(&mut tx, a, id, "time.read").await?;
    let e = present(&mut tx, a, e).await?;
    tx.commit().await?;
    Ok(e)
}
pub async fn budgets(
    pool: &PgPool,
    a: &ActorContext<'_>,
    q: TimeQuery,
) -> Result<TimeBudgetPage, Error> {
    let n = validate_query(&q)?;
    let mut tx = pool.begin().await?;
    let p = authority(&mut tx, a, "time.read").await?;
    if !p.workspace_wide {
        return Err(Error::Forbidden);
    }
    let rows=sqlx::query_as::<_,TimeBudget>("SELECT id,name,project_id,from_on::text,through_on::text,budget_ms,billable_only,archived,version FROM time_tracking.budgets WHERE workspace_id=$1 AND archived=$2 AND ($3::uuid IS NULL OR id>$3) ORDER BY id LIMIT $4").bind(a.workspace).bind(q.archived.unwrap_or(false)).bind(q.after).bind(n+1).fetch_all(&mut *tx).await?;
    let more = rows.len() > n as usize;
    let next_cursor = if more {
        rows.get(n as usize - 1).map(|r| r.id)
    } else {
        None
    };
    let mut items = Vec::new();
    for b in rows.into_iter().take(n as usize) {
        match project(&mut tx, a, b.project_id, false).await {
            Ok(()) => items.push(b),
            Err(Error::Forbidden | Error::NotFound) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(TimeBudgetPage { items, next_cursor })
}
pub async fn save_budget(
    pool: &PgPool,
    a: &ActorContext<'_>,
    id: Option<Uuid>,
    key: &str,
    input: SaveTimeBudget,
) -> Result<TimeBudget, Error> {
    let mut tx = pool.begin().await?;
    modules::lock_workspace(&mut tx, a.workspace).await?;
    let p = authority(&mut tx, a, "time.budgets.manage").await?;
    let read = authority(&mut tx, a, "time.read").await?;
    if !p.workspace_wide || !read.workspace_wide {
        return Err(Error::Forbidden);
    }
    project(&mut tx, a, input.project_id, false).await?;
    let op = format!("time.budget:{id:?}");
    let req = request(&input);
    let result = if let Some(id) =
        reliability::replay::<Uuid>(&mut tx, a.workspace, &p, &op, key, &req).await?
    {
        id
    } else {
        crate::tasks::date(Some(&input.from_on))?;
        crate::tasks::date(Some(&input.through_on))?;
        if input.from_on > input.through_on
            || input.budget_ms <= 0
            || input.name.trim().is_empty()
            || input.name.chars().count() > 120
        {
            return Err(Error::Invalid(
                "Name, ordered dates and positive budget are required",
            ));
        }
        let id = if let Some(id) = id {
            let old=sqlx::query_as::<_,(i64,Option<Uuid>)>("SELECT version,project_id FROM time_tracking.budgets WHERE workspace_id=$1 AND id=$2").bind(a.workspace).bind(id).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
            project(&mut tx, a, old.1, false).await?;
            if input.expected_version != Some(old.0) {
                return Err(Error::Conflict("TimeBudget changed; reload"));
            }
            sqlx::query("UPDATE time_tracking.budgets SET name=$3,project_id=$4,from_on=$5::date,through_on=$6::date,budget_ms=$7,billable_only=$8,archived=$9,version=version+1 WHERE workspace_id=$1 AND id=$2").bind(a.workspace).bind(id).bind(&input.name).bind(input.project_id).bind(&input.from_on).bind(&input.through_on).bind(input.budget_ms).bind(input.billable_only).bind(input.archived).execute(&mut *tx).await?;
            id
        } else {
            if input.expected_version.is_some() {
                return Err(Error::Invalid("New budget has no version"));
            }
            sqlx::query_scalar::<_,Uuid>("INSERT INTO time_tracking.budgets(workspace_id,name,project_id,from_on,through_on,budget_ms,billable_only,archived) VALUES($1,$2,$3,$4::date,$5::date,$6,$7,$8) RETURNING id").bind(a.workspace).bind(&input.name).bind(input.project_id).bind(&input.from_on).bind(&input.through_on).bind(input.budget_ms).bind(input.billable_only).bind(input.archived).fetch_one(&mut *tx).await?
        };
        reliability::audit(
            &mut tx,
            a,
            &p,
            "time.budget.saved",
            id,
            serde_json::json!({}),
        )
        .await?;
        reliability::save_replay(&mut tx, a.workspace, &p, &op, key, &req, &id).await?;
        id
    };
    let b=sqlx::query_as::<_,TimeBudget>("SELECT id,name,project_id,from_on::text,through_on::text,budget_ms,billable_only,archived,version FROM time_tracking.budgets WHERE workspace_id=$1 AND id=$2").bind(a.workspace).bind(result).fetch_one(&mut *tx).await?;
    project(&mut tx, a, b.project_id, false).await?;
    tx.commit().await?;
    Ok(b)
}
