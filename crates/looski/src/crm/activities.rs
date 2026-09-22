// SPDX-License-Identifier: AGPL-3.0-only
//! Logged events, independently authorized from their business-record anchors.
mod dao;
pub mod types;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    contacts::references,
    reliability,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use types::ActivityBehavior;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnchorKind {
    Contact,
    Opportunity,
    Deal,
}
impl AnchorKind {
    fn column(self) -> &'static str {
        match self {
            Self::Contact => "contact_id",
            Self::Opportunity => "opportunity_id",
            Self::Deal => "deal_id",
        }
    }
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Activity {
    pub id: Uuid,
    pub anchor_kind: AnchorKind,
    pub anchor_id: Uuid,
    pub canonical_anchor_id: Uuid,
    pub anchor_name: String,
    pub type_id: Uuid,
    pub type_name: String,
    pub behavior: ActivityBehavior,
    pub subject: String,
    pub body: String,
    pub occurred_at: String,
    pub duration_minutes: Option<i32>,
    pub responsible_membership_id: Uuid,
    pub archived: bool,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ActivityInput {
    pub type_id: Uuid,
    pub subject: String,
    #[serde(default)]
    pub body: String,
    pub occurred_at: String,
    pub duration_minutes: Option<i32>,
    pub responsible_membership_id: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct CreateActivity {
    pub anchor_kind: AnchorKind,
    pub anchor_id: Uuid,
    #[serde(flatten)]
    pub fields: ActivityInput,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct EditActivity {
    pub expected_version: i64,
    #[serde(flatten)]
    pub fields: ActivityInput,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ArchiveActivity {
    pub expected_version: i64,
    pub archived: bool,
}
#[derive(Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct ActivityQuery {
    pub anchor_kind: AnchorKind,
    pub anchor_id: Uuid,
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
    pub archived: Option<bool>,
    pub type_id: Option<Uuid>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct ActivityHistoryQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ActivityPage {
    pub items: Vec<Activity>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ActivityChange {
    pub id: Uuid,
    pub snapshot: Activity,
    pub audit_event_id: Uuid,
    pub occurred_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ActivityHistoryPage {
    pub items: Vec<ActivityChange>,
    pub next_cursor: Option<Uuid>,
}
fn limit(n: Option<i64>) -> Result<i64, Error> {
    let n = n.unwrap_or(50);
    if !(1..=100).contains(&n) {
        return Err(Error::Invalid("Limit must be 1–100"));
    }
    Ok(n)
}
fn validate(input: &mut ActivityInput) -> Result<(), Error> {
    if input.subject.trim().is_empty()
        || input.subject.chars().count() > 200
        || input.subject.chars().any(char::is_control)
        || input.body.chars().count() > 10000
    {
        return Err(Error::Invalid(
            "Subject must be 1–200 printable characters; body at most 10000 characters",
        ));
    }
    if input
        .duration_minutes
        .is_some_and(|n| !(0..=10080).contains(&n))
    {
        return Err(Error::Invalid("Duration must be 0–10080 minutes"));
    }
    let timestamp = chrono::DateTime::parse_from_rfc3339(&input.occurred_at)
        .map_err(|_| Error::Invalid("Occurrence must be an offset-qualified RFC 3339 timestamp"))?;
    use chrono::{Datelike, Timelike};
    if !(1..=9999).contains(&timestamp.year())
        || !(1..=9999).contains(&timestamp.with_timezone(&chrono::Utc).year())
        || timestamp.nanosecond() >= 1_000_000_000
    {
        return Err(Error::Invalid(
            "Occurrence must be a finite year 0001–9999 timestamp without leap seconds",
        ));
    }
    input.occurred_at = timestamp
        .with_nanosecond(timestamp.nanosecond() / 1000 * 1000)
        .expect("valid microseconds")
        .with_timezone(&chrono::Utc)
        .to_rfc3339();
    Ok(())
}
struct Anchor {
    canonical: Uuid,
    name: String,
    archived: bool,
}
async fn anchor(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    kind: AnchorKind,
    id: Uuid,
    lock: bool,
) -> Result<Anchor, Error> {
    match kind {
        AnchorKind::Contact => {
            let r = if lock {
                references::for_mutation(db, actor, references::Kind::Contact, id).await?
            } else {
                references::require(db, actor, references::Kind::Contact, id).await?
            };
            Ok(Anchor {
                canonical: r.canonical_id,
                name: r.name,
                archived: r.archived,
            })
        }
        AnchorKind::Opportunity => {
            let r = super::read_opportunity(db, actor, id).await?;
            Ok(Anchor {
                canonical: id,
                name: r.title,
                archived: r.archived,
            })
        }
        AnchorKind::Deal => {
            let r = super::read_deal(db, actor, id).await?;
            Ok(Anchor {
                canonical: id,
                name: r.title,
                archived: r.archived,
            })
        }
    }
}
fn scope(a: &access::Access, r: &Activity) -> Result<(), Error> {
    super::record_scope(a, r.responsible_membership_id)
}
fn project(r: &mut Activity, a: &Anchor) {
    r.canonical_anchor_id = a.canonical;
    r.anchor_name = a.name.clone();
}
async fn read(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<Activity, Error> {
    let a = super::authorize(db, actor, "crm.activities.read").await?;
    let mut r = dao::get(db, actor.workspace, id).await?;
    scope(&a, &r)?;
    let p = anchor(db, actor, r.anchor_kind, r.anchor_id, false).await?;
    project(&mut r, &p);
    Ok(r)
}
pub async fn get(pool: &PgPool, actor: &ActorContext<'_>, id: Uuid) -> Result<Activity, Error> {
    let mut tx = pool.begin().await?;
    read(&mut tx, actor, id).await
}
pub async fn list(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: ActivityQuery,
) -> Result<ActivityPage, Error> {
    let n = limit(query.limit)? as usize;
    let mut tx = pool.begin().await?;
    let a = super::authorize(&mut tx, actor, "crm.activities.read").await?;
    let p = anchor(&mut tx, actor, query.anchor_kind, query.anchor_id, false).await?;
    if let Some(id) = query.after {
        let r = read(&mut tx, actor, id).await?;
        if r.anchor_kind != query.anchor_kind || r.canonical_anchor_id != p.canonical {
            return Err(Error::NotFound);
        }
    }
    let anchor_ids = if query.anchor_kind == AnchorKind::Contact {
        references::contact_family(&mut tx, actor, p.canonical).await?
    } else {
        vec![query.anchor_id]
    };
    let mut after = query.after;
    let mut items = Vec::new();
    loop {
        let rows = dao::candidates(
            &mut tx,
            actor.workspace,
            &a,
            &query,
            &anchor_ids,
            after,
            100,
        )
        .await?;
        let done = rows.len() < 100;
        if rows.is_empty() {
            break;
        }
        after = rows.last().map(|r| r.id);
        let projections = if query.anchor_kind == AnchorKind::Contact {
            references::visible(
                &mut tx,
                actor,
                references::Kind::Contact,
                &rows.iter().map(|r| r.anchor_id).collect::<Vec<_>>(),
            )
            .await?
            .into_iter()
            .map(|r| (r.reference_id, r))
            .collect::<std::collections::HashMap<_, _>>()
        } else {
            std::collections::HashMap::new()
        };
        for mut row in rows {
            if query.anchor_kind == AnchorKind::Contact {
                let Some(r) = projections.get(&row.anchor_id) else {
                    continue;
                };
                if r.canonical_id != p.canonical {
                    continue;
                }
            }
            project(&mut row, &p);
            items.push(row);
            if items.len() > n {
                break;
            }
        }
        if items.len() > n || done {
            break;
        }
    }
    let next_cursor = if items.len() > n {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    Ok(ActivityPage { items, next_cursor })
}
async fn record(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    a: &access::Access,
    action: &str,
    r: &Activity,
) -> Result<(), Error> {
    let audit = reliability::audit(
        db,
        actor,
        a,
        action,
        r.id,
        serde_json::json!({"version":r.version,"type_id":r.type_id,"archived":r.archived}),
    )
    .await?;
    dao::history_insert(db, actor.workspace, r, audit).await
}
pub async fn create(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    key: &str,
    input: CreateActivity,
) -> Result<Activity, Error> {
    save(
        pool,
        actor,
        None,
        None,
        Some((input.anchor_kind, input.anchor_id)),
        key,
        input.fields,
    )
    .await
}
pub async fn edit(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: EditActivity,
) -> Result<Activity, Error> {
    save(
        pool,
        actor,
        Some(id),
        Some(input.expected_version),
        None,
        key,
        input.fields,
    )
    .await
}
async fn save(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Option<Uuid>,
    expected: Option<i64>,
    new_anchor: Option<(AnchorKind, Uuid)>,
    key: &str,
    mut input: ActivityInput,
) -> Result<Activity, Error> {
    // Bind retry to caller input before timestamp normalization.
    let request =
        serde_json::json!({"input":input,"expected_version":expected,"anchor":new_anchor});
    validate(&mut input)?;
    let mut tx = pool.begin().await?;
    let (write, read_access) = super::mutation(
        &mut tx,
        actor,
        if id.is_some() {
            "crm.activities.update"
        } else {
            "crm.activities.create"
        },
        "crm.activities.read",
    )
    .await?;
    let operation = format!(
        "crm.activities.save:{}",
        id.map(|i| i.to_string()).unwrap_or_else(|| "new".into())
    );
    if let Some(mut saved) =
        reliability::replay::<Activity>(&mut tx, actor.workspace, &write, &operation, key, &request)
            .await?
    {
        let r = read(&mut tx, actor, saved.id).await?;
        scope(&write, &r)?;
        saved.canonical_anchor_id = r.canonical_anchor_id;
        saved.anchor_name = r.anchor_name;
        return Ok(saved);
    }
    let current = if let Some(id) = id {
        let r = read(&mut tx, actor, id).await?;
        scope(&write, &r)?;
        if Some(r.version) != expected || r.archived {
            return Err(Error::Conflict(
                "Activity changed or is archived; reload before editing",
            ));
        }
        Some(r)
    } else {
        None
    };
    let (kind, parent) = new_anchor.unwrap_or_else(|| {
        let r = current.as_ref().expect("edit has current");
        (r.anchor_kind, r.anchor_id)
    });
    let p = anchor(&mut tx, actor, kind, parent, true).await?;
    if current.is_none() && p.archived {
        return Err(Error::Conflict(
            "Archived anchors cannot receive new activities",
        ));
    }
    let responsible = input.responsible_membership_id.unwrap_or_else(|| {
        current
            .as_ref()
            .map(|r| r.responsible_membership_id)
            .unwrap_or(write.membership_id)
    });
    if !write.permits_owner(responsible) || !read_access.permits_owner(responsible) {
        return Err(Error::Forbidden);
    }
    access::require_active_member(&mut tx, actor.workspace, responsible).await?;
    let t = types::state(
        &mut tx,
        actor.workspace,
        input.type_id,
        current.as_ref().is_some_and(|r| r.type_id == input.type_id),
    )
    .await?;
    if t.behavior == ActivityBehavior::Note && input.duration_minutes.is_some() {
        return Err(Error::Invalid(
            "Note types cannot contain interaction duration",
        ));
    }
    let id = dao::save(
        &mut tx,
        actor.workspace,
        id,
        kind,
        parent,
        responsible,
        &t,
        &input,
    )
    .await?;
    let mut result = dao::get(&mut tx, actor.workspace, id).await?;
    project(&mut result, &p);
    record(&mut tx, actor, &write, "crm.activity.saved", &result).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn archive(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: ArchiveActivity,
) -> Result<Activity, Error> {
    let mut tx = pool.begin().await?;
    let (write, _) = super::mutation(
        &mut tx,
        actor,
        "crm.activities.archive",
        "crm.activities.read",
    )
    .await?;
    let current = read(&mut tx, actor, id).await?;
    scope(&write, &current)?;
    let operation = format!("crm.activities.archive:{id}");
    let request = serde_json::to_value(&input).expect("archive serializes");
    if let Some(mut saved) =
        reliability::replay::<Activity>(&mut tx, actor.workspace, &write, &operation, key, &request)
            .await?
    {
        saved.canonical_anchor_id = current.canonical_anchor_id;
        saved.anchor_name = current.anchor_name;
        return Ok(saved);
    }
    if input.expected_version != current.version {
        return Err(Error::Conflict("Activity changed; reload before archiving"));
    }
    let p = anchor(&mut tx, actor, current.anchor_kind, current.anchor_id, true).await?;
    if !input.archived && p.archived {
        return Err(Error::Conflict("Restore the anchor before its activity"));
    }
    dao::archive(&mut tx, actor.workspace, id, input.archived).await?;
    let mut result = dao::get(&mut tx, actor.workspace, id).await?;
    project(&mut result, &p);
    record(&mut tx, actor, &write, "crm.activity.archived", &result).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn history(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    query: ActivityHistoryQuery,
) -> Result<ActivityHistoryPage, Error> {
    let n = limit(query.limit)?;
    let mut tx = pool.begin().await?;
    let current = read(&mut tx, actor, id).await?;
    let mut items = dao::history(&mut tx, actor.workspace, id, &query, n + 1).await?;
    let next_cursor = if items.len() > n as usize {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    for change in &mut items {
        change.snapshot.canonical_anchor_id = current.canonical_anchor_id;
        change.snapshot.anchor_name = current.anchor_name.clone();
    }
    Ok(ActivityHistoryPage { items, next_cursor })
}

/// Current independently authorized activity projections for a single anchor.
pub(super) async fn project_many(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    kind: AnchorKind,
    parent: Uuid,
    ids: &[Uuid],
) -> Result<std::collections::HashMap<Uuid, Activity>, Error> {
    if ids.len() > 100 {
        return Err(Error::Invalid("Activity projection batch exceeds 100"));
    }
    let access = match super::authorize(db, actor, "crm.activities.read").await {
        Ok(a) => a,
        Err(Error::Forbidden) => return Ok(Default::default()),
        Err(e) => return Err(e),
    };
    let anchor = anchor(db, actor, kind, parent, false).await?;
    let mut rows = dao::project_many(db, actor.workspace, &access, ids).await?;
    let contacts = if kind == AnchorKind::Contact {
        references::visible(
            db,
            actor,
            references::Kind::Contact,
            &rows.iter().map(|r| r.anchor_id).collect::<Vec<_>>(),
        )
        .await?
        .into_iter()
        .filter(|r| r.canonical_id == anchor.canonical)
        .map(|r| r.reference_id)
        .collect::<std::collections::HashSet<_>>()
    } else {
        Default::default()
    };
    rows.retain(|r| {
        r.anchor_kind == kind
            && if kind == AnchorKind::Contact {
                contacts.contains(&r.anchor_id)
            } else {
                r.anchor_id == parent
            }
    });
    Ok(rows
        .into_iter()
        .map(|mut r| {
            project(&mut r, &anchor);
            (r.id, r)
        })
        .collect())
}
