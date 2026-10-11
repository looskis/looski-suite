// SPDX-License-Identifier: AGPL-3.0-only
//! Host calendars, public capabilities, atomic intake and durable booking follow-up.
mod workflow;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    forms, modules, reliability,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{AssertSqlSafe, FromRow, PgConnection, PgPool, types::Json};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MinuteWindow {
    pub start: i32,
    pub end: i32,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct WeeklyWindow {
    pub weekday: i32,
    pub start: i32,
    pub end: i32,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AvailabilityException {
    pub date: String,
    pub windows: Vec<MinuteWindow>,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AppointmentSettings {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub timezone: String,
    pub duration_minutes: i32,
    #[serde(default)]
    pub buffer_before: i32,
    #[serde(default)]
    pub buffer_after: i32,
    pub weekly: Vec<WeeklyWindow>,
    #[serde(default)]
    pub exceptions: Vec<AvailabilityException>,
    pub accepting: bool,
    #[serde(default)]
    pub archived: bool,
    pub form_id: Option<Uuid>,
    pub crm: Option<forms::recipes::Mapping>,
    /// Zero disables attendee reminder mail. Otherwise 1–10080 minutes before start.
    #[serde(default)]
    pub reminder_minutes: i32,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveAppointmentType {
    pub settings: AppointmentSettings,
    pub expected_version: Option<i64>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct AppointmentType {
    pub id: Uuid,
    pub host_id: Uuid,
    pub public_key: Uuid,
    pub settings: AppointmentSettings,
    pub version: i64,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct SchedulingQuery {
    pub after: Option<Uuid>,
    pub limit: Option<i64>,
    pub archived: Option<bool>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct AppointmentTypePage {
    pub items: Vec<AppointmentType>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct BookingPage {
    pub items: Vec<Booking>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema, FromRow)]
pub struct Booking {
    pub id: Uuid,
    pub type_id: Uuid,
    pub host_id: Uuid,
    pub name: String,
    pub timezone: String,
    pub starts_at: String,
    pub ends_at: String,
    pub attendee_name: String,
    pub attendee_email: String,
    pub state: String,
    pub version: i64,
    pub form_id: Option<Uuid>,
    pub submission_id: Option<Uuid>,
    pub contact_id: Option<Uuid>,
    pub opportunity_id: Option<Uuid>,
    pub intake_id: Option<Uuid>,
    pub recipe_state: String,
    pub recipe_job_id: Option<Uuid>,
    pub job_state: Option<String>,
    pub error_code: Option<String>,
    pub created_at: String,
    #[sqlx(skip)]
    #[serde(default)]
    pub intake: Option<forms::Submission>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct PublicAppointmentType {
    pub name: String,
    pub description: String,
    pub timezone: String,
    pub duration_minutes: i32,
    pub form: Option<forms::FormVersion>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema, FromRow)]
pub struct BookingSlot {
    pub starts_at: String,
    pub ends_at: String,
    pub label: String,
}
#[derive(Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct SlotQuery {
    pub date: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReserveBooking {
    pub starts_at: String,
    pub attendee_name: String,
    pub attendee_email: String,
    pub form_version_id: Option<Uuid>,
    #[serde(default)]
    pub answers: serde_json::Map<String, serde_json::Value>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ChangeBooking {
    pub expected_version: i64,
    pub action: String,
    pub starts_at: Option<String>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema, FromRow)]
pub struct PublicBooking {
    pub id: Uuid,
    pub public_key: Uuid,
    pub name: String,
    pub timezone: String,
    pub starts_at: String,
    pub ends_at: String,
    pub state: String,
    pub version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct BookingReceipt {
    pub id: Uuid,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema, FromRow)]
pub struct BookingEvent {
    pub id: Uuid,
    pub appointment_id: Uuid,
    pub action: String,
    pub version: i64,
    pub starts_at: String,
    pub ends_at: String,
    pub created_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct BookingEventPage {
    pub items: Vec<BookingEvent>,
    pub next_cursor: Option<Uuid>,
}
struct TypeRow {
    workspace: Uuid,
    view: AppointmentType,
    user: Uuid,
    member: Uuid,
    issuer: String,
    subject: String,
}
const BOOKING: &str = "SELECT a.id,a.type_id,a.host_id,a.name,a.timezone,to_char(a.starts_at AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS starts_at,to_char(a.ends_at AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS ends_at,a.attendee_name,a.attendee_email,a.state,a.version,a.form_id,a.submission_id,a.contact_id,a.opportunity_id,a.intake_id,a.recipe_state,a.recipe_job_id,j.state AS job_state,j.error_code,a.created_at::text FROM scheduling.appointments a LEFT JOIN suite.jobs j ON (j.workspace_id,j.id)=(a.workspace_id,a.recipe_job_id)";
fn digest(text: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn request<T: Serialize>(input: &T) -> serde_json::Value {
    serde_json::json!({"sha256":digest(&serde_json::to_string(input).expect("serializes"))})
}
fn code_hash(code: &str) -> Result<String, Error> {
    if code.len() != 64
        || !code
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(Error::NotFound);
    }
    Ok(digest(code))
}
fn limit(q: &SchedulingQuery) -> Result<i64, Error> {
    let n = q.limit.unwrap_or(50);
    if !(1..=100).contains(&n) {
        return Err(Error::Invalid("Limit must be 1–100"));
    }
    Ok(n)
}
async fn authority(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    permission: &str,
) -> Result<access::Access, Error> {
    modules::require(db, a.workspace, "scheduling").await?;
    access::require(db, a.identity, a.workspace, permission).await
}
async fn type_row(db: &mut PgConnection, id: Uuid, public: bool) -> Result<TypeRow, Error> {
    let (workspace,id,host_id,public_key,settings,version,user,member,issuer,subject)=sqlx::query_as::<_,(Uuid,Uuid,Uuid,Uuid,Json<AppointmentSettings>,i64,Uuid,Uuid,String,String)>("SELECT workspace_id,id,host_id,public_key,settings,version,actor_user_id,actor_membership_id,actor_issuer,actor_subject FROM scheduling.types WHERE (CASE WHEN $2 THEN public_key ELSE id END)=$1").bind(id).bind(public).fetch_optional(db).await?.ok_or(Error::NotFound)?;
    Ok(TypeRow {
        workspace,
        view: AppointmentType {
            id,
            host_id,
            public_key,
            settings: settings.0,
            version,
        },
        user,
        member,
        issuer,
        subject,
    })
}
async fn readable_type(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    id: Uuid,
    permission: &str,
) -> Result<(access::Access, TypeRow), Error> {
    let r = authority(db, a, "scheduling.read").await?;
    let p = authority(db, a, permission).await?;
    let t = type_row(db, id, false).await?;
    if t.workspace != a.workspace || !r.workspace_wide && t.view.host_id != r.membership_id {
        return Err(Error::NotFound);
    }
    if !p.workspace_wide && t.view.host_id != p.membership_id {
        return Err(Error::Forbidden);
    }
    Ok((p, t))
}
async fn public_type(db: &mut PgConnection, key: Uuid, accepting: bool) -> Result<TypeRow, Error> {
    let t = type_row(db, key, true).await?;
    if !modules::enabled(db, t.workspace, "scheduling").await?
        || t.view.settings.archived
        || accepting && !t.view.settings.accepting
    {
        return Err(Error::NotFound);
    }
    access::require_active_member(db, t.workspace, t.view.host_id)
        .await
        .map_err(|_| Error::NotFound)?;
    Ok(t)
}
pub async fn types(
    pool: &PgPool,
    a: &ActorContext<'_>,
    q: SchedulingQuery,
) -> Result<AppointmentTypePage, Error> {
    let n = limit(&q)?;
    let mut tx = pool.begin().await?;
    let p = authority(&mut tx, a, "scheduling.read").await?;
    let mut ids=sqlx::query_scalar::<_,Uuid>("SELECT id FROM scheduling.types WHERE workspace_id=$1 AND ($2 OR host_id=$3) AND (settings->>'archived')::boolean=$4 AND ($5::uuid IS NULL OR id>$5) ORDER BY id LIMIT $6").bind(a.workspace).bind(p.workspace_wide).bind(p.membership_id).bind(q.archived.unwrap_or(false)).bind(q.after).bind(n+1).fetch_all(&mut *tx).await?;
    let more = ids.len() > n as usize;
    ids.truncate(n as usize);
    let next_cursor = if more { ids.last().copied() } else { None };
    let mut items = Vec::new();
    for id in ids {
        items.push(type_row(&mut tx, id, false).await?.view);
    }
    Ok(AppointmentTypePage { items, next_cursor })
}
pub async fn get_type(
    pool: &PgPool,
    a: &ActorContext<'_>,
    id: Uuid,
) -> Result<AppointmentType, Error> {
    let mut tx = pool.begin().await?;
    Ok(readable_type(&mut tx, a, id, "scheduling.read")
        .await?
        .1
        .view)
}
fn window(start: i32, end: i32) -> bool {
    start >= 0 && end <= 1440 && start < end
}
async fn validate(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    s: &AppointmentSettings,
) -> Result<(), Error> {
    if s.name.trim().is_empty()
        || s.name.chars().count() > 120
        || s.name.chars().any(char::is_control)
        || s.description.chars().count() > 4000
        || !(5..=480).contains(&s.duration_minutes)
        || !(0..=120).contains(&s.buffer_before)
        || !(0..=120).contains(&s.buffer_after)
        || !(0..=10080).contains(&s.reminder_minutes)
        || s.weekly.len() > 50
        || s.exceptions.len() > 100
    {
        return Err(Error::Invalid(
            "Invalid appointment name, duration, buffers or availability limits",
        ));
    }
    if !sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM pg_timezone_names WHERE name=$1)",
    )
    .bind(&s.timezone)
    .fetch_one(&mut *db)
    .await?
    {
        return Err(Error::Invalid("Choose an IANA timezone"));
    }
    if s.weekly
        .iter()
        .any(|w| !(1..=7).contains(&w.weekday) || !window(w.start, w.end))
    {
        return Err(Error::Invalid(
            "Weekly windows use weekdays 1–7 and minutes 0–1440",
        ));
    }
    let mut dates = std::collections::HashSet::new();
    for x in &s.exceptions {
        crate::tasks::date(Some(&x.date))?;
        if !dates.insert(&x.date)
            || x.windows.len() > 10
            || x.windows.iter().any(|w| !window(w.start, w.end))
        {
            return Err(Error::Invalid(
                "Date exceptions must be unique with valid windows",
            ));
        }
    }
    let form = if let Some(form) = s.form_id {
        if !forms::can_reference(db, a, form).await? {
            return Err(Error::NotFound);
        }
        Some(forms::booking_form(db, a.workspace, form).await?)
    } else {
        None
    };
    if let Some(mapping) = &s.crm {
        workflow::validate_mapping(db, a, mapping, form.as_ref()).await?;
    }
    Ok(())
}
pub async fn save_type(
    pool: &PgPool,
    a: &ActorContext<'_>,
    id: Option<Uuid>,
    key: &str,
    input: SaveAppointmentType,
) -> Result<AppointmentType, Error> {
    let mut tx = pool.begin().await?;
    modules::lock_workspace(&mut tx, a.workspace).await?;
    let p = authority(&mut tx, a, "scheduling.manage").await?;
    authority(&mut tx, a, "scheduling.read").await?;
    let old = if let Some(id) = id {
        Some(readable_type(&mut tx, a, id, "scheduling.manage").await?.1)
    } else {
        None
    };
    let op = format!("scheduling.type:{id:?}");
    let req = request(&input);
    let id = if let Some(id) =
        reliability::replay::<Uuid>(&mut tx, a.workspace, &p, &op, key, &req).await?
    {
        id
    } else {
        validate(&mut tx, a, &input.settings).await?;
        if old.as_ref().is_some_and(|t| t.view.settings.crm.is_some()) {
            authority(&mut tx, a, "scheduling.recipes.manage").await?;
        }
        let id = if let Some(old) = old {
            if input.expected_version != Some(old.view.version) {
                return Err(Error::Conflict("Appointment type changed; reload"));
            }
            sqlx::query("UPDATE scheduling.types SET settings=$3,version=version+1,actor_user_id=$4,actor_membership_id=$5,actor_issuer=$6,actor_subject=$7 WHERE workspace_id=$1 AND id=$2").bind(a.workspace).bind(old.view.id).bind(Json(&input.settings)).bind(p.user_id).bind(p.membership_id).bind(a.identity.issuer()).bind(a.identity.subject()).execute(&mut *tx).await?;
            old.view.id
        } else {
            if input.expected_version.is_some() {
                return Err(Error::Invalid("New type has no version"));
            }
            sqlx::query_scalar::<_,Uuid>("INSERT INTO scheduling.types(workspace_id,host_id,settings,actor_user_id,actor_membership_id,actor_issuer,actor_subject) VALUES($1,$2,$3,$4,$2,$5,$6) RETURNING id").bind(a.workspace).bind(p.membership_id).bind(Json(&input.settings)).bind(p.user_id).bind(a.identity.issuer()).bind(a.identity.subject()).fetch_one(&mut *tx).await?
        };
        reliability::audit(
            &mut tx,
            a,
            &p,
            "scheduling.type.saved",
            id,
            serde_json::json!({}),
        )
        .await?;
        reliability::save_replay(&mut tx, a.workspace, &p, &op, key, &req, &id).await?;
        id
    };
    let t = readable_type(&mut tx, a, id, "scheduling.read")
        .await?
        .1
        .view;
    tx.commit().await?;
    Ok(t)
}
pub async fn public_info(pool: &PgPool, key: Uuid) -> Result<PublicAppointmentType, Error> {
    let mut tx = pool.begin().await?;
    let t = public_type(&mut tx, key, true).await?;
    let s = t.view.settings;
    let form = if let Some(id) = s.form_id {
        Some(forms::booking_form(&mut tx, t.workspace, id).await?)
    } else {
        None
    };
    Ok(PublicAppointmentType {
        name: s.name,
        description: s.description,
        timezone: s.timezone,
        duration_minutes: s.duration_minutes,
        form,
    })
}
fn timestamp(s: &str) -> Result<(), Error> {
    let t = chrono::DateTime::parse_from_rfc3339(s)
        .map_err(|_| Error::Invalid("Start must include a UTC offset"))?;
    if t.timestamp() % 900 != 0 || t.timestamp_subsec_nanos() != 0 {
        return Err(Error::Invalid("Starts must be on a 15-minute boundary"));
    }
    Ok(())
}
/// Enumerating UTC minutes handles both occurrences of a folded hour and never invents a gap.
/// Every buffered minute must be inside the host's local availability, including DST transitions.
async fn slot_available(
    db: &mut PgConnection,
    t: &TypeRow,
    start: &str,
    exclude: Option<Uuid>,
) -> Result<bool, Error> {
    timestamp(start)?;
    let s = &t.view.settings;
    let free=sqlx::query_scalar::<_,bool>("SELECT $1::timestamptz>now() AND $1::timestamptz<now()+interval '366 days' AND NOT EXISTS(SELECT 1 FROM scheduling.appointments WHERE workspace_id=$2 AND host_id=$3 AND state='confirmed' AND ($7::uuid IS NULL OR id<>$7) AND busy && tstzrange($1::timestamptz-$4*interval '1 minute',$1::timestamptz+($5+$6)*interval '1 minute','[)'))").bind(start).bind(t.workspace).bind(t.view.host_id).bind(s.buffer_before).bind(s.duration_minutes).bind(s.buffer_after).bind(exclude).fetch_one(&mut *db).await?;
    if !free {
        return Ok(false);
    }
    let minutes=sqlx::query_as::<_,(String,i32,i32)>("SELECT (g AT TIME ZONE $2)::date::text,extract(isodow FROM g AT TIME ZONE $2)::integer,(extract(hour FROM g AT TIME ZONE $2)*60+extract(minute FROM g AT TIME ZONE $2))::integer FROM generate_series($1::timestamptz-$3*interval '1 minute',$1::timestamptz+($4+$5-1)*interval '1 minute',interval '1 minute') g").bind(start).bind(&s.timezone).bind(s.buffer_before).bind(s.duration_minutes).bind(s.buffer_after).fetch_all(db).await?;
    Ok(minutes.iter().all(|(date, dow, minute)| {
        if let Some(x) = s.exceptions.iter().find(|x| &x.date == date) {
            x.windows
                .iter()
                .any(|w| *minute >= w.start && *minute < w.end)
        } else {
            s.weekly
                .iter()
                .any(|w| w.weekday == *dow && *minute >= w.start && *minute < w.end)
        }
    }))
}
pub async fn slots(pool: &PgPool, key: Uuid, q: SlotQuery) -> Result<Vec<BookingSlot>, Error> {
    crate::tasks::date(Some(&q.date))?;
    let mut tx = pool.begin().await?;
    let t = public_type(&mut tx, key, true).await?;
    if let Some(id) = t.view.settings.form_id {
        forms::booking_form(&mut tx, t.workspace, id).await?;
    }
    let valid = sqlx::query_scalar::<_, bool>(
        "SELECT $1::date BETWEEN current_date-1 AND current_date+366",
    )
    .bind(&q.date)
    .fetch_one(&mut *tx)
    .await?;
    if !valid {
        return Err(Error::Invalid("Choose a day within the next year"));
    }
    let candidates=sqlx::query_as::<_,BookingSlot>("SELECT to_char(g AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS starts_at,to_char((g+$3*interval '1 minute') AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS ends_at,to_char(g AT TIME ZONE $2,'YYYY-MM-DD HH24:MI') || ' ' || $2 || ' (UTC' || CASE WHEN (g AT TIME ZONE $2)>=(g AT TIME ZONE 'UTC') THEN '+' ELSE '-' END || lpad((abs(extract(epoch FROM ((g AT TIME ZONE $2)-(g AT TIME ZONE 'UTC'))))::integer/3600)::text,2,'0') || ':' || lpad((abs(extract(epoch FROM ((g AT TIME ZONE $2)-(g AT TIME ZONE 'UTC'))))::integer%3600/60)::text,2,'0') || ')' AS label FROM generate_series($1::date::timestamp AT TIME ZONE 'UTC'-interval '14 hours',$1::date::timestamp AT TIME ZONE 'UTC'+interval '38 hours',interval '15 minutes') g WHERE (g AT TIME ZONE $2)::date=$1::date AND g>now() ORDER BY g").bind(&q.date).bind(&t.view.settings.timezone).bind(t.view.settings.duration_minutes).fetch_all(&mut *tx).await?;
    let mut items = Vec::new();
    for slot in candidates {
        if slot_available(&mut tx, &t, &slot.starts_at, None).await? {
            items.push(slot);
        }
    }
    Ok(items)
}
async fn event(db: &mut PgConnection, ws: Uuid, id: Uuid, action: &str) -> Result<(), Error> {
    sqlx::query("INSERT INTO scheduling.events(workspace_id,appointment_id,action,version,starts_at,ends_at) SELECT workspace_id,id,$3,version,starts_at,ends_at FROM scheduling.appointments WHERE workspace_id=$1 AND id=$2").bind(ws).bind(id).bind(action).execute(db).await?;
    Ok(())
}
pub async fn reserve(
    pool: &PgPool,
    key: Uuid,
    code: &str,
    input: ReserveBooking,
) -> Result<BookingReceipt, Error> {
    let hash = code_hash(code)?;
    let mut tx = pool.begin().await?;
    let t = public_type(&mut tx, key, true).await?;
    modules::lock_workspace(&mut tx, t.workspace).await?;
    let t = public_type(&mut tx, key, true).await?;
    let req = digest(&serde_json::to_string(&input).expect("serializes"));
    if let Some((id, old)) = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id,request_hash FROM scheduling.appointments WHERE workspace_id=$1 AND key_hash=$2",
    )
    .bind(t.workspace)
    .bind(&hash)
    .fetch_optional(&mut *tx)
    .await?
    {
        if old != req {
            return Err(Error::Conflict(
                "Access code already used for different booking",
            ));
        }
        return Ok(BookingReceipt { id });
    }
    if input.attendee_name.trim().is_empty()
        || input.attendee_name.chars().count() > 200
        || input.attendee_name.chars().any(char::is_control)
        || input.attendee_email.len() > 254
        || !input.attendee_email.contains('@')
        || input.attendee_email.parse::<lettre::Address>().is_err()
    {
        return Err(Error::Invalid(
            "A name and valid email address are required",
        ));
    }
    if !slot_available(&mut tx, &t, &input.starts_at, None).await? {
        return Err(Error::Conflict("This time is no longer available"));
    }
    let id = Uuid::new_v4();
    let s = &t.view.settings;
    let submission = if let Some(form) = s.form_id {
        Some(
            forms::booking_submit(
                &mut tx,
                t.workspace,
                form,
                input
                    .form_version_id
                    .ok_or(Error::Invalid("Intake version is required"))?,
                &input.answers,
                id,
            )
            .await?,
        )
    } else {
        if input.form_version_id.is_some() || !input.answers.is_empty() {
            return Err(Error::Invalid("This appointment has no custom intake"));
        }
        None
    };
    sqlx::query("INSERT INTO scheduling.appointments(id,workspace_id,type_id,host_id,name,timezone,starts_at,ends_at,busy,attendee_name,attendee_email,key_hash,request_hash,form_id,submission_id,mapping,recipe_state) VALUES($1,$2,$3,$4,$5,$6,$7::timestamptz,$7::timestamptz+$8*interval '1 minute',tstzrange($7::timestamptz-$9*interval '1 minute',$7::timestamptz+($8+$10)*interval '1 minute','[)'),$11,$12,$13,$14,$15,$16,$17,CASE WHEN $17::jsonb IS NULL THEN 'none' ELSE 'queued' END)").bind(id).bind(t.workspace).bind(t.view.id).bind(t.view.host_id).bind(&s.name).bind(&s.timezone).bind(&input.starts_at).bind(s.duration_minutes).bind(s.buffer_before).bind(s.buffer_after).bind(input.attendee_name.trim()).bind(input.attendee_email.trim()).bind(hash).bind(req).bind(s.form_id).bind(submission).bind(s.crm.as_ref().map(Json)).execute(&mut *tx).await?;
    event(&mut tx, t.workspace, id, "confirmed").await?;
    workflow::enqueue(&mut tx, &t, id, 1, true).await?;
    tx.commit().await?;
    Ok(BookingReceipt { id })
}
async fn public_booking(
    db: &mut PgConnection,
    id: Uuid,
    code: &str,
) -> Result<(Uuid, PublicBooking), Error> {
    let hash = code_hash(code)?;
    let row = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT workspace_id,type_id FROM scheduling.appointments WHERE id=$1 AND key_hash=$2",
    )
    .bind(id)
    .bind(hash)
    .fetch_optional(&mut *db)
    .await?
    .ok_or(Error::NotFound)?;
    if !modules::enabled(db, row.0, "scheduling").await? {
        return Err(Error::NotFound);
    }
    let b=sqlx::query_as::<_,PublicBooking>("SELECT a.id,t.public_key,a.name,a.timezone,to_char(a.starts_at AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS starts_at,to_char(a.ends_at AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS ends_at,a.state,a.version FROM scheduling.appointments a JOIN scheduling.types t ON t.id=a.type_id WHERE a.id=$1").bind(id).fetch_one(db).await?;
    Ok((row.0, b))
}
pub async fn customer_get(pool: &PgPool, id: Uuid, code: &str) -> Result<PublicBooking, Error> {
    let mut tx = pool.begin().await?;
    Ok(public_booking(&mut tx, id, code).await?.1)
}
async fn change_in(
    db: &mut PgConnection,
    t: &TypeRow,
    id: Uuid,
    input: &ChangeBooking,
) -> Result<(), Error> {
    let (version, state) = sqlx::query_as::<_, (i64, String)>(
        "SELECT version,state FROM scheduling.appointments WHERE workspace_id=$1 AND id=$2",
    )
    .bind(t.workspace)
    .bind(id)
    .fetch_optional(&mut *db)
    .await?
    .ok_or(Error::NotFound)?;
    if input.expected_version != version {
        return Err(Error::Conflict("Appointment changed; reload"));
    }
    if state != "confirmed" {
        return Err(Error::Conflict("Appointment is cancelled"));
    }
    match input.action.as_str() {
        "cancel" => {
            if input.starts_at.is_some() {
                return Err(Error::Invalid("Cancellation has no new start"));
            }
            sqlx::query("UPDATE scheduling.appointments SET state='cancelled',version=version+1 WHERE workspace_id=$1 AND id=$2").bind(t.workspace).bind(id).execute(&mut *db).await?;
        }
        "reschedule" => {
            access::require_active_member(db, t.workspace, t.view.host_id).await?;
            if !t.view.settings.accepting || t.view.settings.archived {
                return Err(Error::Conflict(
                    "This appointment type is not accepting reschedules",
                ));
            }
            let start = input
                .starts_at
                .as_deref()
                .ok_or(Error::Invalid("New start is required"))?;
            if !slot_available(db, t, start, Some(id)).await? {
                return Err(Error::Conflict("This time is no longer available"));
            }
            let s = &t.view.settings;
            sqlx::query("UPDATE scheduling.appointments SET starts_at=$3::timestamptz,ends_at=$3::timestamptz+$4*interval '1 minute',busy=tstzrange($3::timestamptz-$5*interval '1 minute',$3::timestamptz+($4+$6)*interval '1 minute','[)'),timezone=$7,version=version+1 WHERE workspace_id=$1 AND id=$2").bind(t.workspace).bind(id).bind(start).bind(s.duration_minutes).bind(s.buffer_before).bind(s.buffer_after).bind(&s.timezone).execute(&mut *db).await?;
            workflow::enqueue(db, t, id, version + 1, false).await?;
        }
        _ => return Err(Error::Invalid("Action must be cancel or reschedule")),
    }
    event(db, t.workspace, id, &input.action).await
}
pub async fn customer_change(
    pool: &PgPool,
    id: Uuid,
    code: &str,
    key: &str,
    input: ChangeBooking,
) -> Result<PublicBooking, Error> {
    if key.is_empty() || key.len() > 128 || !key.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(Error::Invalid(
            "Idempotency-Key must be 1–128 visible ASCII characters",
        ));
    }
    let mut tx = pool.begin().await?;
    let (ws, _) = public_booking(&mut tx, id, code).await?;
    modules::lock_workspace(&mut tx, ws).await?;
    let (_, b) = public_booking(&mut tx, id, code).await?;
    let req = digest(&serde_json::to_string(&input).expect("serializes"));
    let old=sqlx::query_scalar::<_,String>("SELECT request_hash FROM scheduling.retries WHERE workspace_id=$1 AND appointment_id=$2 AND key=$3").bind(ws).bind(id).bind(key).fetch_optional(&mut *tx).await?;
    if let Some(old) = old {
        if old != req {
            return Err(Error::Conflict(
                "Retry key already used with different input",
            ));
        }
    } else {
        let t = type_row(&mut tx, b.public_key, true).await?;
        change_in(&mut tx, &t, id, &input).await?;
        sqlx::query("INSERT INTO scheduling.retries(workspace_id,appointment_id,key,request_hash) VALUES($1,$2,$3,$4)").bind(ws).bind(id).bind(key).bind(req).execute(&mut *tx).await?;
    }
    let b = public_booking(&mut tx, id, code).await?.1;
    tx.commit().await?;
    Ok(b)
}
async fn raw(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    id: Uuid,
    permission: &str,
) -> Result<(access::Access, Booking), Error> {
    let read = authority(db, a, "scheduling.read").await?;
    let p = authority(db, a, permission).await?;
    let b = sqlx::query_as::<_, Booking>(AssertSqlSafe(format!(
        "{BOOKING} WHERE a.workspace_id=$1 AND a.id=$2 AND ($3 OR a.host_id=$4)"
    )))
    .bind(a.workspace)
    .bind(id)
    .bind(read.workspace_wide)
    .bind(read.membership_id)
    .fetch_optional(db)
    .await?
    .ok_or(Error::NotFound)?;
    if !p.workspace_wide && b.host_id != p.membership_id {
        return Err(Error::Forbidden);
    }
    Ok((p, b))
}
async fn present(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    mut b: Booking,
) -> Result<Booking, Error> {
    if let (Some(form), Some(id)) = (b.form_id, b.submission_id) {
        match forms::booking_submission(db, a, form, id).await {
            Ok(s) => b.intake = Some(s),
            Err(Error::Forbidden | Error::NotFound) => {
                b.form_id = None;
                b.submission_id = None;
            }
            Err(e) => return Err(e),
        }
    }
    if let Some(id) = b.contact_id {
        b.contact_id = crate::contacts::references::visible(
            db,
            a,
            crate::contacts::references::Kind::Contact,
            &[id],
        )
        .await?
        .first()
        .map(|r| r.canonical_id);
    }
    // CRM references are omitted without current module/read authority; the appointment remains accessible.
    if let Some(id) = b.opportunity_id {
        match crate::crm::read_opportunity(db, a, id).await {
            Ok(_) => {}
            Err(Error::Forbidden | Error::NotFound) => b.opportunity_id = None,
            Err(e) => return Err(e),
        }
    }
    if let Some(id) = b.intake_id {
        match crate::contacts::intake::get_in(db, a, id).await {
            Ok(_) => {}
            Err(Error::Forbidden | Error::NotFound) => b.intake_id = None,
            Err(e) => return Err(e),
        }
    }
    Ok(b)
}
pub async fn get(pool: &PgPool, a: &ActorContext<'_>, id: Uuid) -> Result<Booking, Error> {
    let mut tx = pool.begin().await?;
    let (_, b) = raw(&mut tx, a, id, "scheduling.read").await?;
    present(&mut tx, a, b).await
}
pub async fn list(
    pool: &PgPool,
    a: &ActorContext<'_>,
    q: SchedulingQuery,
) -> Result<BookingPage, Error> {
    let n = limit(&q)?;
    let mut tx = pool.begin().await?;
    let p = authority(&mut tx, a, "scheduling.read").await?;
    let mut rows=sqlx::query_as::<_,Booking>(AssertSqlSafe(format!("{BOOKING} WHERE a.workspace_id=$1 AND ($2 OR a.host_id=$3) AND (a.state='cancelled')=$4 AND ($5::uuid IS NULL OR a.id>$5) ORDER BY a.id LIMIT $6"))).bind(a.workspace).bind(p.workspace_wide).bind(p.membership_id).bind(q.archived.unwrap_or(false)).bind(q.after).bind(n+1).fetch_all(&mut *tx).await?;
    let more = rows.len() > n as usize;
    rows.truncate(n as usize);
    let next_cursor = if more {
        rows.last().map(|b| b.id)
    } else {
        None
    };
    let mut items = Vec::new();
    for b in rows {
        items.push(present(&mut tx, a, b).await?);
    }
    Ok(BookingPage { items, next_cursor })
}
pub async fn change(
    pool: &PgPool,
    a: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: ChangeBooking,
) -> Result<Booking, Error> {
    let mut tx = pool.begin().await?;
    modules::lock_workspace(&mut tx, a.workspace).await?;
    let (p, b) = raw(&mut tx, a, id, "scheduling.manage").await?;
    let req = request(&input);
    let op = format!("scheduling.change:{id}");
    if reliability::replay::<bool>(&mut tx, a.workspace, &p, &op, key, &req)
        .await?
        .is_none()
    {
        let t = type_row(&mut tx, b.type_id, false).await?;
        change_in(&mut tx, &t, id, &input).await?;
        reliability::audit(
            &mut tx,
            a,
            &p,
            "scheduling.booking.changed",
            id,
            serde_json::json!({"action":input.action}),
        )
        .await?;
        reliability::save_replay(&mut tx, a.workspace, &p, &op, key, &req, &true).await?;
    }
    let b = raw(&mut tx, a, id, "scheduling.read").await?.1;
    let b = present(&mut tx, a, b).await?;
    tx.commit().await?;
    Ok(b)
}
pub async fn events(
    pool: &PgPool,
    a: &ActorContext<'_>,
    id: Uuid,
    q: SchedulingQuery,
) -> Result<BookingEventPage, Error> {
    let n = limit(&q)?;
    let mut tx = pool.begin().await?;
    raw(&mut tx, a, id, "scheduling.read").await?;
    let mut items=sqlx::query_as::<_,BookingEvent>("SELECT id,appointment_id,action,version,starts_at::text,ends_at::text,created_at::text FROM scheduling.events WHERE workspace_id=$1 AND appointment_id=$2 AND ($3::uuid IS NULL OR id>$3) ORDER BY id LIMIT $4").bind(a.workspace).bind(id).bind(q.after).bind(n+1).fetch_all(&mut *tx).await?;
    let more = items.len() > n as usize;
    items.truncate(n as usize);
    let next_cursor = if more {
        items.last().map(|e| e.id)
    } else {
        None
    };
    Ok(BookingEventPage { items, next_cursor })
}

#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RetryBooking {
    pub expected_version: i64,
}
pub async fn retry(
    pool: &PgPool,
    a: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: RetryBooking,
) -> Result<Booking, Error> {
    let mut tx = pool.begin().await?;
    modules::lock_workspace(&mut tx, a.workspace).await?;
    let (p, b) = raw(&mut tx, a, id, "scheduling.recipes.manage").await?;
    let op = format!("scheduling.retry:{id}");
    let req = request(&input);
    if reliability::replay::<bool>(&mut tx, a.workspace, &p, &op, key, &req)
        .await?
        .is_none()
    {
        if b.version != input.expected_version {
            return Err(Error::Conflict("Appointment changed; reload"));
        }
        modules::require_crm(&mut tx, a).await?;
        if b.form_id.is_some() {
            modules::require(&mut tx, a.workspace, "forms").await?;
        }
        match (b.recipe_state.as_str(), b.job_state.as_deref()) {
            ("awaiting_review", Some("completed")) => {
                let job = crate::jobs::enqueue_recorded(
                    &mut tx,
                    a.workspace,
                    p.user_id,
                    p.membership_id,
                    a.identity.issuer(),
                    a.identity.subject(),
                    &format!("booking:{id}:review:{}", Uuid::new_v4()),
                    "scheduling.crm_inquiry",
                )
                .await?;
                sqlx::query("UPDATE scheduling.appointments SET recipe_job_id=$3,recipe_state='queued' WHERE workspace_id=$1 AND id=$2").bind(a.workspace).bind(id).bind(job).execute(&mut *tx).await?;
            }
            (_, Some("blocked" | "exhausted")) => {
                crate::jobs::resume_in(
                    &mut tx,
                    a.workspace,
                    b.recipe_job_id.ok_or(Error::NotFound)?,
                )
                .await?
            }
            _ => {
                return Err(Error::Conflict(
                    "Only failed or reviewed follow-up can be retried",
                ));
            }
        }
        // Recipe state has its own job version; appointment version only changes when its reservation changes.
        reliability::audit(
            &mut tx,
            a,
            &p,
            "scheduling.recipe.retried",
            id,
            serde_json::json!({}),
        )
        .await?;
        reliability::save_replay(&mut tx, a.workspace, &p, &op, key, &req, &true).await?;
    }
    let b = raw(&mut tx, a, id, "scheduling.read").await?.1;
    let b = present(&mut tx, a, b).await?;
    tx.commit().await?;
    Ok(b)
}
pub(crate) use workflow::{execute, needs_forms, reminder};

#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct AppointmentTimeline {
    pub appointment: Booking,
    pub action: String,
    pub starts_at: String,
    pub ends_at: String,
}
pub(crate) async fn history(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    family: &[Uuid],
    after: Option<&crate::history::Position>,
    only: Option<Uuid>,
    n: i64,
) -> Result<Vec<(crate::history::Position, AppointmentTimeline)>, Error> {
    let p = match authority(db, a, "scheduling.read").await {
        Ok(p) => p,
        Err(Error::Forbidden) => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let rows=sqlx::query_as::<_,(Json<crate::history::Position>,Uuid,String,String,String)>("SELECT jsonb_build_object('id',e.id,'recorded_at',to_char(e.created_at AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"'),'source','appointment'),a.id,e.action,e.starts_at::text,e.ends_at::text FROM scheduling.events e JOIN scheduling.appointments a ON (a.workspace_id,a.id)=(e.workspace_id,e.appointment_id) WHERE a.workspace_id=$1 AND a.contact_id=ANY($2) AND ($3 OR a.host_id=$4) AND ($5::uuid IS NULL OR e.id=$5) AND ($6::text IS NULL OR (e.created_at,'appointment'::text,e.id)<($6::timestamptz,$7::text,$8::uuid)) ORDER BY e.created_at DESC,e.id DESC LIMIT $9").bind(a.workspace).bind(family).bind(p.workspace_wide).bind(p.membership_id).bind(only).bind(after.map(|p|p.recorded_at.as_str())).bind(after.map(|p|p.source.as_str())).bind(after.map(|p|p.id)).bind(n).fetch_all(&mut *db).await?;
    let mut result = Vec::new();
    for (position, id, action, starts_at, ends_at) in rows {
        let b = raw(db, a, id, "scheduling.read").await?.1;
        result.push((
            position.0,
            AppointmentTimeline {
                appointment: present(db, a, b).await?,
                action,
                starts_at,
                ends_at,
            },
        ));
    }
    Ok(result)
}
