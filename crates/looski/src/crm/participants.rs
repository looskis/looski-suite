// SPDX-License-Identifier: AGPL-3.0-only
//! Typed sales participants with independently authorized Contacts references.
mod dao;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    contacts::references::{self, Kind, Reference},
    reliability,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
#[derive(Clone, Copy)]
pub enum Target {
    OpportunityContact,
    OpportunityOrganization,
    DealContact,
    DealOrganization,
}
impl Target {
    fn is_deal(self) -> bool {
        matches!(self, Self::DealContact | Self::DealOrganization)
    }
    fn kind(self) -> Kind {
        match self {
            Self::OpportunityContact | Self::DealContact => Kind::Contact,
            _ => Kind::Organization,
        }
    }
    fn read(self) -> &'static str {
        if self.is_deal() {
            "crm.deals.read"
        } else {
            "crm.opportunities.read"
        }
    }
    fn manage(self) -> &'static str {
        if self.is_deal() {
            "crm.deals.participants.manage"
        } else {
            "crm.opportunities.participants.manage"
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::OpportunityContact => "crm.opportunity_contacts",
            Self::OpportunityOrganization => "crm.opportunity_organizations",
            Self::DealContact => "crm.deal_contacts",
            Self::DealOrganization => "crm.deal_organizations",
        }
    }
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NewParticipant {
    pub expected_version: i64,
    pub reference_id: Uuid,
    #[serde(default)]
    pub role_label: String,
    #[serde(default)]
    pub is_primary: bool,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EditParticipant {
    pub expected_version: i64,
    pub role_label: Option<String>,
    pub is_primary: Option<bool>,
    pub ended: Option<bool>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Participant {
    pub id: Uuid,
    pub reference_id: Uuid,
    pub canonical_id: Uuid,
    pub name: String,
    pub reference_archived: bool,
    pub role_label: String,
    pub is_primary: bool,
    pub ended: bool,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct ParticipantQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
    pub ended: Option<bool>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ParticipantPage {
    pub items: Vec<Participant>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ParticipantChange {
    pub id: Uuid,
    pub role_label: String,
    pub is_primary: bool,
    pub ended: bool,
    pub occurred_at: String,
    pub audit_event_id: Uuid,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ParticipantHistory {
    pub participant: Participant,
    pub items: Vec<ParticipantChange>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize)]
struct SavedParticipant {
    result: Participant,
    promoted: bool,
}
#[derive(Serialize)]
struct SavedParticipantRef<'a> {
    result: &'a Participant,
    promoted: bool,
}
fn project(row: dao::Row, reference: Reference) -> Participant {
    Participant {
        id: row.id,
        reference_id: row.reference_id,
        canonical_id: reference.canonical_id,
        name: reference.name,
        reference_archived: reference.archived,
        role_label: row.role_label,
        is_primary: row.is_primary,
        ended: row.ended,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}
async fn parent(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    target: Target,
    id: Uuid,
    a: &access::Access,
) -> Result<(i64, bool), Error> {
    let (owner, version, archived) = if target.is_deal() {
        let r = super::dao::deal(db, actor.workspace, id).await?;
        (r.owner_membership_id, r.version, r.archived)
    } else {
        let r = super::dao::opportunity(db, actor.workspace, id).await?;
        (r.owner_membership_id, r.version, r.archived)
    };
    super::record_scope(a, owner)?;
    Ok((version, archived))
}
async fn visible_rows(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    target: Target,
    rows: Vec<dao::Row>,
) -> Result<Vec<Participant>, Error> {
    let ids: Vec<_> = rows.iter().map(|r| r.reference_id).collect();
    let mut refs = references::visible(db, actor, target.kind(), &ids).await?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            refs.iter()
                .position(|r| r.reference_id == row.reference_id)
                .map(|i| project(row, refs.swap_remove(i)))
        })
        .collect())
}
pub async fn list(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    target: Target,
    id: Uuid,
    query: ParticipantQuery,
) -> Result<ParticipantPage, Error> {
    let limit = super::page_limit(query.limit, None)? as usize;
    let mut tx = pool.begin().await?;
    let read = super::authorize(&mut tx, actor, target.read()).await?;
    parent(&mut tx, actor, target, id, &read).await?;
    if let Some(after) = query.after {
        let row = dao::get(&mut tx, actor.workspace, target, id, after).await?;
        references::require(&mut tx, actor, target.kind(), row.reference_id).await?;
    }
    let mut after = query.after;
    let mut items = Vec::new();
    loop {
        let rows = dao::list(
            &mut tx,
            actor.workspace,
            target,
            id,
            after,
            query.ended.unwrap_or(false),
        )
        .await?;
        let exhausted = rows.len() < 100;
        after = rows.last().map(|r| r.id);
        items.extend(visible_rows(&mut tx, actor, target, rows).await?);
        if items.len() > limit || exhausted {
            break;
        }
    }
    let next_cursor = if items.len() > limit {
        items.truncate(limit);
        items.last().map(|r| r.id)
    } else {
        None
    };
    Ok(ParticipantPage { items, next_cursor })
}
async fn duplicate(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    target: Target,
    id: Uuid,
    canonical: Uuid,
    except: Option<Uuid>,
) -> Result<(), Error> {
    let mut after = None;
    loop {
        let rows = dao::list(db, actor.workspace, target, id, after, false).await?;
        let exhausted = rows.len() < 100;
        after = rows.last().map(|r| r.id);
        for p in visible_rows(db, actor, target, rows).await? {
            if Some(p.id) != except && p.canonical_id == canonical {
                return Err(Error::Conflict(
                    "A visible participant already represents this record; edit that association",
                ));
            }
        }
        if exhausted {
            return Ok(());
        }
    }
}
#[allow(clippy::too_many_arguments)]
async fn save(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    target: Target,
    parent_id: Uuid,
    id: Option<Uuid>,
    key: &str,
    new: Option<NewParticipant>,
    edit: Option<EditParticipant>,
) -> Result<Participant, Error> {
    let mut tx = pool.begin().await?;
    let result = save_in(&mut tx, actor, target, parent_id, id, key, new, edit).await?;
    tx.commit().await?;
    Ok(result)
}
/// Add a participant within the caller's transaction, such as a workflow step.
pub(crate) async fn create_in(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    target: Target,
    parent_id: Uuid,
    key: &str,
    input: NewParticipant,
) -> Result<Participant, Error> {
    save_in(db, actor, target, parent_id, None, key, Some(input), None).await
}
#[allow(clippy::too_many_arguments)]
async fn save_in(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    target: Target,
    parent_id: Uuid,
    id: Option<Uuid>,
    key: &str,
    new: Option<NewParticipant>,
    edit: Option<EditParticipant>,
) -> Result<Participant, Error> {
    let request = if let Some(n) = &new {
        serde_json::to_value(n)
    } else {
        serde_json::to_value(&edit)
    }
    .expect("participant input serializes");
    let (write, read) = super::mutation(&mut *db, actor, target.manage(), target.read()).await?;
    let operation = format!(
        "{}.{}:{parent_id}:{}",
        target.label(),
        if id.is_some() { "edit" } else { "create" },
        id.map(|v| v.to_string()).unwrap_or_default()
    );
    let saved = reliability::replay::<SavedParticipant>(
        &mut *db,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
    )
    .await?;
    let (version, archived) = parent(&mut *db, actor, target, parent_id, &write).await?;
    parent(&mut *db, actor, target, parent_id, &read).await?;
    if let Some(saved) = saved {
        if saved.promoted
            && !access::require(
                &mut *db,
                actor.identity,
                actor.workspace,
                target.kind().permission(),
            )
            .await?
            .workspace_wide
        {
            return Err(Error::Forbidden);
        }
        let saved = saved.result;
        references::require(&mut *db, actor, target.kind(), saved.reference_id).await?;
        references::require(&mut *db, actor, target.kind(), saved.canonical_id).await?;
        return Ok(saved);
    }
    let previous = if let Some(id) = id {
        Some(dao::get(&mut *db, actor.workspace, target, parent_id, id).await?)
    } else {
        None
    };
    let (expected, reference_id, label, primary, ended, promote) = if let Some(n) = new {
        (
            n.expected_version,
            n.reference_id,
            n.role_label,
            n.is_primary,
            false,
            n.is_primary,
        )
    } else {
        let e = edit.expect("edit input");
        let p = previous.as_ref().expect("existing association");
        let ended = e.ended.unwrap_or(p.ended);
        if ended && e.is_primary == Some(true) {
            return Err(Error::Invalid("An ended participant cannot be primary"));
        }
        (
            e.expected_version,
            p.reference_id,
            e.role_label.unwrap_or_else(|| p.role_label.clone()),
            !ended && e.is_primary.unwrap_or(p.is_primary),
            ended,
            e.is_primary == Some(true) && (!p.is_primary || p.ended),
        )
    };
    let reference = references::for_mutation(&mut *db, actor, target.kind(), reference_id).await?;
    if expected != version {
        return Err(Error::Conflict(
            "Record changed; reload before changing participants",
        ));
    }
    if label.chars().count() > 80 || label.chars().any(char::is_control) {
        return Err(Error::Invalid(
            "Participant role must be at most 80 printable characters",
        ));
    }
    if promote
        && !access::require(
            &mut *db,
            actor.identity,
            actor.workspace,
            target.kind().permission(),
        )
        .await?
        .workspace_wide
    {
        return Err(Error::Forbidden);
    }
    let activating = previous.as_ref().is_none_or(|p| p.ended && !ended);
    if activating {
        if archived || reference.archived {
            return Err(Error::Conflict(
                "Restore archived records before adding or restoring participants",
            ));
        }
        duplicate(
            &mut *db,
            actor,
            target,
            parent_id,
            reference.canonical_id,
            id,
        )
        .await?;
    } else if archived && !ended {
        return Err(Error::Conflict(
            "Restore the sales record before editing active participants",
        ));
    }
    let reference_id = if previous.is_none() {
        reference.canonical_id
    } else {
        reference_id
    };
    if previous.is_none()
        && dao::by_reference(&mut *db, actor.workspace, target, parent_id, reference_id)
            .await?
            .is_some()
    {
        return Err(Error::Conflict(
            "An association already exists; edit or restore it",
        ));
    }
    let id = id.unwrap_or_else(Uuid::new_v4);
    let audit=reliability::audit(&mut *db,actor,&write,&format!("{}.saved",target.label()),parent_id,serde_json::json!({"association_id":id,"reference_id":reference_id,"role_label":label.trim(),"is_primary":primary,"ended":ended})).await?;
    if primary {
        for demoted in dao::demote(&mut *db, actor.workspace, target, parent_id, id).await? {
            dao::history(&mut *db, actor.workspace, target, &demoted, audit).await?;
        }
    }
    dao::save(
        &mut *db,
        actor.workspace,
        target,
        parent_id,
        id,
        reference_id,
        label.trim(),
        primary,
        ended,
        write.membership_id,
        previous.is_some(),
    )
    .await?;
    dao::touch(&mut *db, actor.workspace, target, parent_id).await?;
    let row = dao::get(&mut *db, actor.workspace, target, parent_id, id).await?;
    dao::history(&mut *db, actor.workspace, target, &row, audit).await?;
    let reference = references::require(&mut *db, actor, target.kind(), reference_id).await?;
    let result = project(row, reference);
    reliability::save_replay(
        &mut *db,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
        &SavedParticipantRef {
            result: &result,
            promoted: promote,
        },
    )
    .await?;
    Ok(result)
}
pub async fn create(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    target: Target,
    id: Uuid,
    key: &str,
    input: NewParticipant,
) -> Result<Participant, Error> {
    save(pool, actor, target, id, None, key, Some(input), None).await
}
pub async fn edit(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    target: Target,
    parent: Uuid,
    id: Uuid,
    key: &str,
    input: EditParticipant,
) -> Result<Participant, Error> {
    save(
        pool,
        actor,
        target,
        parent,
        Some(id),
        key,
        None,
        Some(input),
    )
    .await
}
pub async fn history(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    target: Target,
    parent_id: Uuid,
    id: Uuid,
    query: super::HistoryQuery,
) -> Result<ParticipantHistory, Error> {
    let limit = super::page_limit(query.limit, None)? as usize;
    let mut tx = pool.begin().await?;
    let read = super::authorize(&mut tx, actor, target.read()).await?;
    parent(&mut tx, actor, target, parent_id, &read).await?;
    let row = dao::get(&mut tx, actor.workspace, target, parent_id, id).await?;
    let reference = references::require(&mut tx, actor, target.kind(), row.reference_id).await?;
    let participant = project(row, reference);
    let mut items = dao::changes(
        &mut tx,
        actor.workspace,
        target,
        id,
        &query,
        (limit + 1) as i64,
    )
    .await?;
    let next_cursor = if items.len() > limit {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    Ok(ParticipantHistory {
        participant,
        items,
        next_cursor,
    })
}
