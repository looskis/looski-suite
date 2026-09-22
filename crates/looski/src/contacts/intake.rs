// SPDX-License-Identifier: AGPL-3.0-only
//! Retained input, automatic identity resolution and explicit review decisions.
mod dao;
use super::{Contact, ContactInput, Error};
use crate::{access, application::ActorContext, reliability};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
#[derive(Clone, Deserialize, Serialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SubmitIntake {
    pub source_ref: String,
    pub proposed: ContactInput,
    pub contact_id: Option<Uuid>,
}
#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
pub struct Intake {
    pub id: Uuid,
    pub owner_membership_id: Uuid,
    pub source_ref: String,
    pub input: SubmitIntake,
    pub state: String,
    pub contact_id: Option<Uuid>,
    pub resolution_reason: Option<String>,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Default, Deserialize, Serialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct ListIntakes {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
    pub state: Option<String>,
}
#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
pub struct IntakePage {
    pub items: Vec<Intake>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Link,
    Create,
    Dismiss,
}
#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolveIntake {
    pub expected_version: i64,
    pub decision: Decision,
    pub reason: String,
    pub contact_id: Option<Uuid>,
    pub contact: Option<ContactInput>,
}
fn identifiers(input: &ContactInput) -> Result<Vec<super::Identifier>, Error> {
    let mut input = input.clone();
    input.display_name = "Intake identity".into();
    input.notes.clear();
    super::validate(&input)
}
async fn visible_contact(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Uuid,
    lock: bool,
) -> Result<Contact, Error> {
    let read = access::require(db, actor.identity, actor.workspace, "contacts.read").await?;
    let contact = super::dao::get(db, actor.workspace, id, &read, lock).await?;
    if let Some(survivor) = contact.merged_into_id {
        super::dao::get(db, actor.workspace, survivor, &read, lock).await
    } else {
        Ok(contact)
    }
}
async fn present(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    mut item: Intake,
) -> Result<Intake, Error> {
    if let Some(id) = item.contact_id {
        match visible_contact(db, actor, id, false).await {
            Ok(c) => item.contact_id = Some(c.id),
            Err(Error::Forbidden | Error::NotFound) => item.contact_id = None,
            Err(e) => return Err(e),
        }
    }
    Ok(item)
}
pub async fn list(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: ListIntakes,
) -> Result<IntakePage, Error> {
    let limit = query.limit.unwrap_or(50);
    let state = query.state.as_deref().unwrap_or("pending");
    if !(1..=100).contains(&limit) || !matches!(state, "pending" | "resolved" | "dismissed") {
        return Err(Error::Invalid(
            "Limit must be 1–100; state must be pending, resolved or dismissed",
        ));
    }
    let mut tx = pool.begin().await?;
    let read = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        "contacts.intake.read",
    )
    .await?;
    let mut rows = dao::list(&mut tx, actor.workspace, &read, &query, state, limit + 1).await?;
    let next_cursor = if rows.len() > limit as usize {
        rows.pop();
        rows.last().map(|i| i.id)
    } else {
        None
    };
    let mut items = Vec::new();
    for row in rows {
        items.push(present(&mut tx, actor, row).await?)
    }
    tx.commit().await?;
    Ok(IntakePage { items, next_cursor })
}
pub async fn get(pool: &PgPool, actor: &ActorContext<'_>, id: Uuid) -> Result<Intake, Error> {
    let mut tx = pool.begin().await?;
    let read = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        "contacts.intake.read",
    )
    .await?;
    let row = dao::get(&mut tx, actor.workspace, id, &read, false).await?;
    let item = present(&mut tx, actor, row).await?;
    tx.commit().await?;
    Ok(item)
}
async fn replay_access(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Uuid,
    write: &access::Access,
    read: &access::Access,
    saved: &Intake,
) -> Result<(), Error> {
    dao::get(db, actor.workspace, id, write, false).await?;
    dao::get(db, actor.workspace, id, read, false).await?;
    if let Some(id) = saved.contact_id {
        visible_contact(db, actor, id, false).await?;
    }
    Ok(())
}
pub async fn submit(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    key: &str,
    input: SubmitIntake,
) -> Result<Intake, Error> {
    let request = serde_json::to_value(&input).expect("intake serializes");
    // Reserve room for JSONB whitespace expansion as well as field-size bounds.
    if input.source_ref.trim().is_empty()
        || input.source_ref.chars().count() > 200
        || serde_json::to_vec_pretty(&request)
            .expect("input serializes")
            .len()
            > 30000
        || input.proposed.identifiers.len() > 20
    {
        return Err(Error::Invalid(
            "Intake requires a source reference up to 200 characters, at most 20 identifiers and a bounded 30KB input",
        ));
    }
    if input.proposed.allow_shared {
        return Err(Error::Invalid(
            "Shared-identifier creation requires an explicit review decision",
        ));
    }
    let mut tx = pool.begin().await?;
    let write = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        "contacts.intake.create",
    )
    .await?;
    let read = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        "contacts.intake.read",
    )
    .await?;
    if !read.permits_owner(write.membership_id) {
        return Err(Error::Forbidden);
    }
    if let Some(saved) = reliability::replay::<Intake>(
        &mut tx,
        actor.workspace,
        &write,
        "contacts.intake.submit",
        key,
        &request,
    )
    .await?
    {
        replay_access(&mut tx, actor, saved.id, &write, &read, &saved).await?;
        return Ok(saved);
    }
    let can_auto =
        match access::require(&mut tx, actor.identity, actor.workspace, "contacts.read").await {
            Ok(authority) => authority.workspace_wide,
            Err(Error::Forbidden) => false,
            Err(error) => return Err(error),
        };
    let mut contact = None;
    if let Some(id) = input.contact_id {
        let c = visible_contact(&mut tx, actor, id, true).await?;
        if !c.archived && identifiers(&input.proposed).is_ok() {
            contact = Some(c.id)
        }
    } else if let Ok(identifiers) = identifiers(&input.proposed)
        && can_auto
        && !identifiers.is_empty()
    {
        super::dao::lock_identifiers(&mut tx, actor.workspace, &identifiers).await?;
        let matches = super::dao::matches(&mut tx, actor.workspace, &identifiers).await?;
        let case_review = dao::has_case_suggestions(&mut tx, actor.workspace, &identifiers).await?;
        if matches.len() == 1 && !case_review {
            match visible_contact(&mut tx, actor, matches[0], true).await {
                Ok(c) if !c.archived => {
                    if super::dao::matches(&mut tx, actor.workspace, &identifiers).await? == matches
                    {
                        contact = Some(c.id)
                    }
                }
                Ok(_) | Err(Error::Forbidden | Error::NotFound) => {}
                Err(e) => return Err(e),
            }
        } else if matches.is_empty() && !case_review {
            match super::create_in_transaction(&mut tx, actor, &input.proposed).await {
                Ok(c) => contact = Some(c.id),
                Err(Error::Forbidden | Error::Invalid(_)) => {}
                Err(e) => return Err(e),
            }
        }
    }
    let id = dao::insert(
        &mut tx,
        actor.workspace,
        write.membership_id,
        &input,
        contact,
    )
    .await?;
    reliability::audit(
        &mut tx,
        actor,
        &write,
        "contacts.intake_submitted",
        id,
        serde_json::json!({"resolved":contact.is_some()}),
    )
    .await?;
    let result = dao::get(&mut tx, actor.workspace, id, &read, false).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &write,
        "contacts.intake.submit",
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn resolve(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: ResolveIntake,
) -> Result<Intake, Error> {
    if input.reason.trim().is_empty() || input.reason.chars().count() > 1000 {
        return Err(Error::Invalid(
            "A review reason of 1–1000 characters is required",
        ));
    }
    match input.decision {
        Decision::Link if input.contact_id.is_some() && input.contact.is_none() => {}
        Decision::Create if input.contact_id.is_none() && input.contact.is_some() => {}
        Decision::Dismiss if input.contact_id.is_none() && input.contact.is_none() => {}
        _ => {
            return Err(Error::Invalid(
                "Link requires contact_id, create requires contact, dismiss accepts neither",
            ));
        }
    }
    let mut tx = pool.begin().await?;
    let write = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        "contacts.intake.resolve",
    )
    .await?;
    let read = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        "contacts.intake.read",
    )
    .await?;
    let request = serde_json::to_value(&input).expect("review serializes");
    let operation = format!("contacts.intake.resolve:{id}");
    if let Some(saved) =
        reliability::replay::<Intake>(&mut tx, actor.workspace, &write, &operation, key, &request)
            .await?
    {
        replay_access(&mut tx, actor, id, &write, &read, &saved).await?;
        return Ok(saved);
    }
    let current = dao::get(&mut tx, actor.workspace, id, &write, true).await?;
    dao::get(&mut tx, actor.workspace, id, &read, false).await?;
    if current.version != input.expected_version || current.state != "pending" {
        return Err(Error::Conflict(
            "Intake changed or was resolved; reload before reviewing",
        ));
    }
    let contact = match input.decision {
        Decision::Link => {
            let c =
                visible_contact(&mut tx, actor, input.contact_id.expect("validated"), true).await?;
            if c.archived {
                return Err(Error::Conflict(
                    "Restore the contact before linking this intake",
                ));
            }
            Some(c.id)
        }
        Decision::Create => Some(
            super::create_in_transaction(
                &mut tx,
                actor,
                input.contact.as_ref().expect("validated"),
            )
            .await?
            .id,
        ),
        Decision::Dismiss => None,
    };
    dao::resolve(&mut tx, actor.workspace, id, contact, &input.reason).await?;
    reliability::audit(
        &mut tx,
        actor,
        &write,
        "contacts.intake_resolved",
        id,
        serde_json::json!({"decision":input.decision}),
    )
    .await?;
    let result = dao::get(&mut tx, actor.workspace, id, &read, false).await?;
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
/// Only permitted current candidates are returned; no hidden-match counts/reasons.
pub async fn candidates(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<Vec<Contact>, Error> {
    let mut tx = pool.begin().await?;
    let read = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        "contacts.intake.read",
    )
    .await?;
    let item = dao::get(&mut tx, actor.workspace, id, &read, false).await?;
    let contact_access =
        match access::require(&mut tx, actor.identity, actor.workspace, "contacts.read").await {
            Ok(a) => a,
            Err(Error::Forbidden) => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
    let input = item.input.proposed;
    let mut result = Vec::new();
    if let Ok(ids) = identifiers(&input) {
        for candidate in dao::candidate_ids(&mut tx, actor.workspace, &ids, &contact_access).await?
        {
            match visible_contact(&mut tx, actor, candidate, false).await {
                Ok(c) => result.push(c),
                Err(Error::Forbidden | Error::NotFound) => {}
                Err(e) => return Err(e),
            }
        }
    }
    tx.commit().await?;
    Ok(result)
}
