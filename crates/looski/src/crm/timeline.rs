// SPDX-License-Identifier: AGPL-3.0-only
//! Read-only composition of independently authorized business history.
mod dao;
use super::{
    Transition,
    activities::{self, ActivityChange, AnchorKind},
};
use crate::{
    application::{ActorContext, OperationError as Error},
    contacts::references,
    history::Position,
    tasks,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
#[derive(Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct TimelineQuery {
    pub anchor_kind: AnchorKind,
    pub anchor_id: Uuid,
    pub limit: Option<i64>,
    pub after: Option<String>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum EventDetail {
    Activity(ActivityChange),
    Transition(Transition),
    Task(tasks::TaskChange),
    TaskLink(LinkEvent),
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct LinkEvent {
    pub link_id: Uuid,
    pub task_id: Uuid,
    pub task_title: String,
    pub ended: bool,
    pub audit_event_id: Uuid,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TimelineEvent {
    pub cursor: String,
    pub recorded_at: String,
    pub event: EventDetail,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TimelinePage {
    pub anchor_kind: AnchorKind,
    pub anchor_id: Uuid,
    pub anchor_name: String,
    pub items: Vec<TimelineEvent>,
    pub next_cursor: Option<String>,
}
struct VisibleEvent {
    position: Position,
    event: EventDetail,
}
struct Anchor {
    id: Uuid,
    name: String,
    family: Vec<Uuid>,
}
async fn anchor(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    q: &TimelineQuery,
) -> Result<Anchor, Error> {
    let (id, name, family) = match q.anchor_kind {
        AnchorKind::Contact => {
            let r = references::require(db, actor, references::Kind::Contact, q.anchor_id).await?;
            let family = references::contact_family(db, actor, r.canonical_id).await?;
            (r.canonical_id, r.name, family)
        }
        AnchorKind::Opportunity => {
            let r = super::read_opportunity(db, actor, q.anchor_id).await?;
            (r.id, r.title, vec![r.id])
        }
        AnchorKind::Deal => {
            let r = super::read_deal(db, actor, q.anchor_id).await?;
            (r.id, r.title, vec![r.id])
        }
    };
    crate::modules::require_crm(db, actor).await?;
    Ok(Anchor { id, name, family })
}

struct Sources {
    activities: bool,
    tasks: bool,
}
fn allowed(result: Result<crate::access::Access, Error>) -> Result<bool, Error> {
    match result {
        Ok(_) => Ok(true),
        Err(Error::Forbidden) => Ok(false),
        Err(e) => Err(e),
    }
}
async fn crm_events(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    q: &TimelineQuery,
    rows: Vec<dao::Candidate>,
) -> Result<Vec<VisibleEvent>, Error> {
    let mut parsed = Vec::new();
    let mut activity_ids = Vec::new();
    let mut task_ids = Vec::new();
    for r in rows {
        let event = match r.source.as_str() {
            "activity" => {
                let c: ActivityChange = serde_json::from_value(r.detail)
                    .map_err(|_| Error::Invalid("Invalid activity history"))?;
                activity_ids.push(c.snapshot.id);
                EventDetail::Activity(c)
            }
            "task_link" => {
                let l: LinkEvent = serde_json::from_value(r.detail)
                    .map_err(|_| Error::Invalid("Invalid link history"))?;
                task_ids.push(l.task_id);
                EventDetail::TaskLink(l)
            }
            "transition" => EventDetail::Transition(
                serde_json::from_value(r.detail)
                    .map_err(|_| Error::Invalid("Invalid transition history"))?,
            ),
            _ => return Err(Error::Invalid("Invalid history source")),
        };
        parsed.push(VisibleEvent {
            position: r.position,
            event,
        });
    }
    let activities = if activity_ids.is_empty() {
        Default::default()
    } else {
        activities::project_many(db, actor, q.anchor_kind, q.anchor_id, &activity_ids).await?
    };
    let tasks = if task_ids.is_empty() {
        Default::default()
    } else {
        tasks::project_many(db, actor, &task_ids).await?
    };
    Ok(parsed
        .into_iter()
        .filter_map(|mut r| {
            match &mut r.event {
                EventDetail::Activity(c) => {
                    let current = activities.get(&c.snapshot.id)?;
                    c.snapshot.canonical_anchor_id = current.canonical_anchor_id;
                    c.snapshot.anchor_name = current.anchor_name.clone()
                }
                EventDetail::TaskLink(l) => l.task_title = tasks.get(&l.task_id)?.title.clone(),
                _ => {}
            };
            Some(r)
        })
        .collect())
}
fn anchor_key(kind: AnchorKind) -> &'static str {
    match kind {
        AnchorKind::Contact => "contact",
        AnchorKind::Opportunity => "opportunity",
        AnchorKind::Deal => "deal",
    }
}
fn parse_cursor(
    value: &str,
    workspace: Uuid,
    kind: AnchorKind,
    parent: Uuid,
) -> Result<(&str, Uuid), Error> {
    let parts = value.split(':').collect::<Vec<_>>();
    if parts.len() != 5 || !["activity", "transition", "task_link", "task"].contains(&parts[3]) {
        return Err(Error::Invalid("Invalid timeline cursor"));
    }
    let uuid = |v: &str| Uuid::parse_str(v).map_err(|_| Error::Invalid("Invalid timeline cursor"));
    if uuid(parts[0])? != workspace || parts[1] != anchor_key(kind) || uuid(parts[2])? != parent {
        return Err(Error::NotFound);
    }
    Ok((parts[3], uuid(parts[4])?))
}
fn trim(events: &mut Vec<VisibleEvent>, n: usize) {
    events.sort_by(|a, b| b.position.key().cmp(&a.position.key()));
    events.truncate(n)
}
pub async fn list(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    q: TimelineQuery,
) -> Result<TimelinePage, Error> {
    let n = q.limit.unwrap_or(25);
    if !(1..=100).contains(&n) {
        return Err(Error::Invalid("Timeline limit must be 1–100"));
    }
    let n = n as usize;
    let mut tx = pool.begin().await?;
    let a = anchor(&mut tx, actor, &q).await?;
    let sources = Sources {
        activities: allowed(super::authorize(&mut tx, actor, "crm.activities.read").await)?,
        tasks: allowed(tasks::require_read(&mut tx, actor).await)?,
    };
    let mut after = None;
    if let Some(cursor) = q.after.as_deref() {
        let (kind, id) = parse_cursor(cursor, actor.workspace, q.anchor_kind, a.id)?;
        let event = if kind == "task" {
            if !sources.tasks {
                return Err(Error::NotFound);
            }
            match tasks::history_stream(&mut tx, actor, &[], None, None, Some(id))
                .await?
                .pop()
            {
                Some(r) => {
                    let linked = dao::linked_tasks(
                        &mut tx,
                        actor.workspace,
                        q.anchor_kind,
                        q.anchor_id,
                        &[r.change.snapshot.id],
                    )
                    .await?;
                    if linked.is_empty() {
                        None
                    } else {
                        Some(VisibleEvent {
                            position: r.position,
                            event: EventDetail::Task(r.change),
                        })
                    }
                }
                None => None,
            }
        } else {
            let rows = dao::candidates(
                &mut tx,
                actor.workspace,
                &q,
                &a.family,
                None,
                Some((kind, id)),
                &sources,
            )
            .await?;
            crm_events(&mut tx, actor, &q, rows).await?.pop()
        };
        after = Some(event.ok_or(Error::NotFound)?.position);
    }
    let mut events = Vec::new();
    let mut scan = after.clone();
    loop {
        let rows = dao::candidates(
            &mut tx,
            actor.workspace,
            &q,
            &a.family,
            scan.as_ref(),
            None,
            &sources,
        )
        .await?;
        let done = rows.len() < 100;
        scan = rows.last().map(|r| r.position.clone());
        events.extend(crm_events(&mut tx, actor, &q, rows).await?);
        trim(&mut events, n + 1);
        if events.len() > n || done {
            break;
        }
    }
    if q.anchor_kind != AnchorKind::Contact && sources.tasks {
        let mut last = None;
        loop {
            let ids =
                dao::task_ids(&mut tx, actor.workspace, q.anchor_kind, q.anchor_id, last).await?;
            let done = ids.len() < 100;
            last = ids.last().copied();
            if ids.is_empty() {
                break;
            }
            let floor = if events.len() > n {
                events.last().map(|e| e.position.clone())
            } else {
                None
            };
            let rows =
                tasks::history_stream(&mut tx, actor, &ids, after.as_ref(), floor.as_ref(), None)
                    .await?;
            events.extend(rows.into_iter().map(|r| VisibleEvent {
                position: r.position,
                event: EventDetail::Task(r.change),
            }));
            trim(&mut events, n + 1);
            if done {
                break;
            }
        }
    }
    let more = events.len() > n;
    events.truncate(n);
    let items = events
        .into_iter()
        .map(|r| TimelineEvent {
            cursor: format!(
                "{}:{}:{}:{}:{}",
                actor.workspace,
                anchor_key(q.anchor_kind),
                a.id,
                r.position.source,
                r.position.id
            ),
            recorded_at: r.position.recorded_at,
            event: r.event,
        })
        .collect::<Vec<_>>();
    let next_cursor = if more {
        items.last().map(|r| r.cursor.clone())
    } else {
        None
    };
    Ok(TimelinePage {
        anchor_kind: q.anchor_kind,
        anchor_id: a.id,
        anchor_name: a.name,
        items,
        next_cursor,
    })
}
