// SPDX-License-Identifier: AGPL-3.0-only
//! Provider-neutral, private HTTP calendar adapter. Provider credentials stay outside Suite.
use super::*;
use chrono::{DateTime, Duration, FixedOffset};
use serde_json::{Value, json};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Connection {
    workspace_id: Uuid,
    host_id: Uuid,
    endpoint: String,
    api_token: String,
}
fn connection(workspace: Uuid, host: Uuid) -> Result<Option<Connection>, Error> {
    let path = match std::env::var("LOOSKI_CALENDARS_CONFIG_FILE") {
        Ok(path) => path,
        Err(_) => return Ok(None),
    };
    let bytes = std::fs::read(path).map_err(|_| Error::Unavailable)?;
    let connections: Vec<Connection> =
        serde_json::from_slice(&bytes).map_err(|_| Error::Unavailable)?;
    let mut matches = connections
        .into_iter()
        .filter(|c| c.workspace_id == workspace && c.host_id == host);
    let result = matches.next();
    if matches.next().is_some() {
        return Err(Error::Conflict("Duplicate calendar connection"));
    }
    if let Some(c) = &result {
        let url = reqwest::Url::parse(&c.endpoint).map_err(|_| Error::Unavailable)?;
        let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if !(url.scheme() == "https" || (local && url.scheme() == "http"))
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || c.endpoint.ends_with('/')
            || c.api_token.len() < 32
        {
            return Err(Error::Conflict("Invalid calendar connector configuration"));
        }
    }
    Ok(result)
}
async fn call(c: &Connection, path: &str, value: Value) -> Result<Value, Error> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| Error::Unavailable)?;
    let mut response = client
        .post(format!("{}{path}", c.endpoint))
        .bearer_auth(&c.api_token)
        .json(&value)
        .send()
        .await
        .map_err(|_| Error::Unavailable)?;
    if response.status().as_u16() == 409 {
        return Err(Error::Conflict(
            "Calendar changed externally; reconcile the event before retrying",
        ));
    }
    if !response.status().is_success() {
        return Err(Error::Unavailable);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| Error::Unavailable)? {
        if bytes.len() + chunk.len() > 2_000_000 {
            return Err(Error::Unavailable);
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| Error::Unavailable)
}
fn instant(value: &str) -> Result<DateTime<FixedOffset>, Error> {
    DateTime::parse_from_rfc3339(value).map_err(|_| Error::Invalid("Invalid calendar time"))
}
#[derive(Deserialize)]
struct BusyResponse {
    busy: Vec<Busy>,
}
#[derive(Deserialize)]
pub(super) struct Busy {
    start: String,
    end: String,
}
pub(super) async fn busy(
    t: &TypeRow,
    start: &str,
    end: &str,
    exclude: Option<Uuid>,
) -> Result<Vec<Busy>, Error> {
    let Some(c) = connection(t.workspace, t.view.host_id)? else {
        return Ok(vec![]);
    };
    let response = call(
        &c,
        "/v1/calendar/busy",
        json!({"start":start,"end":end,"exclude":exclude}),
    )
    .await?;
    let parsed: BusyResponse = serde_json::from_value(response).map_err(|_| Error::Unavailable)?;
    for b in &parsed.busy {
        if instant(&b.start)? >= instant(&b.end)? {
            return Err(Error::Unavailable);
        }
    }
    Ok(parsed.busy)
}
pub(super) fn available(t: &TypeRow, start: &str, busy: &[Busy]) -> Result<bool, Error> {
    let s = &t.view.settings;
    let start = instant(start)?;
    let end = start + Duration::minutes(i64::from(s.duration_minutes + s.buffer_after));
    let start = start - Duration::minutes(i64::from(s.buffer_before));
    for b in busy {
        if start < instant(&b.end)? && end > instant(&b.start)? {
            return Ok(false);
        }
    }
    Ok(true)
}
pub(super) async fn check(t: &TypeRow, start: &str, exclude: Option<Uuid>) -> Result<bool, Error> {
    let s = &t.view.settings;
    let value = instant(start)?;
    let begin = (value - Duration::minutes(i64::from(s.buffer_before))).to_rfc3339();
    let end =
        (value + Duration::minutes(i64::from(s.duration_minutes + s.buffer_after))).to_rfc3339();
    available(t, start, &busy(t, &begin, &end, exclude).await?)
}
pub(super) async fn enqueue(
    db: &mut PgConnection,
    t: &TypeRow,
    id: Uuid,
    version: i64,
) -> Result<(), Error> {
    if connection(t.workspace, t.view.host_id)?.is_none() {
        return Ok(());
    }
    let job = crate::jobs::enqueue_recorded(
        db,
        t.workspace,
        t.user,
        t.member,
        &t.issuer,
        &t.subject,
        &format!("booking:{id}:calendar:{version}"),
        "scheduling.calendar",
    )
    .await?;
    sqlx::query(
        "INSERT INTO scheduling.calendar_jobs(workspace_id,job_id,appointment_id) VALUES($1,$2,$3)",
    )
    .bind(t.workspace)
    .bind(job)
    .bind(id)
    .execute(db)
    .await?;
    Ok(())
}
pub(crate) async fn execute(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    job: Uuid,
) -> Result<(), Error> {
    let id = sqlx::query_scalar::<_, Uuid>(
        "SELECT appointment_id FROM scheduling.calendar_jobs WHERE workspace_id=$1 AND job_id=$2",
    )
    .bind(a.workspace)
    .bind(job)
    .fetch_one(&mut *db)
    .await?;
    let (_, b) = raw(db, a, id, "scheduling.manage").await?;
    let c = connection(a.workspace, b.host_id)?
        .ok_or(Error::Conflict("Calendar connection was removed"))?;
    // Always reconcile the current version; a delayed job cannot overwrite a newer booking.
    let result = call(&c,"/v1/calendar/appointments",json!({"id":b.id,"version":b.version,"state":b.state,
        "name":b.name,"start":b.starts_at,"end":b.ends_at,"attendee_name":b.attendee_name,"attendee_email":b.attendee_email})).await?;
    if !matches!(result["state"].as_str(), Some("synced" | "superseded")) {
        return Err(Error::Unavailable);
    }
    Ok(())
}
