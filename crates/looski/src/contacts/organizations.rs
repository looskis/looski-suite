// SPDX-License-Identifier: AGPL-3.0-only
//! Organization and relationship operations within the shared Contacts boundary.
mod dao;
use super::{Error, ListContacts};
use crate::{access, application::ActorContext, reliability};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct OrganizationInput {
    pub name: String,
    pub website_url: Option<String>,
    #[serde(default)]
    pub notes: String,
    pub owner_membership_id: Option<Uuid>,
}
#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EditOrganization {
    pub expected_version: i64,
    #[serde(flatten)]
    pub organization: OrganizationInput,
}
#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub website_url: Option<String>,
    pub notes: String,
    pub owner_membership_id: Uuid,
    pub archived: bool,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
pub struct OrganizationPage {
    pub items: Vec<Organization>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RelationshipInput {
    pub expected_contact_version: i64,
    #[serde(default)]
    pub relationship_label: String,
    #[serde(default)]
    pub job_title: String,
    #[serde(default)]
    pub is_primary: bool,
    #[serde(default)]
    pub ended: bool,
}
#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
pub struct Relationship {
    pub id: Uuid,
    pub contact_id: Uuid,
    pub organization_id: Uuid,
    pub organization_name: String,
    pub relationship_label: String,
    pub job_title: String,
    pub is_primary: bool,
    pub ended: bool,
    pub created_at: String,
}
fn validate(input: &OrganizationInput) -> Result<(), Error> {
    if input.name.trim().is_empty()
        || input.name.chars().count() > 200
        || input.notes.chars().count() > 10000
    {
        return Err(Error::Invalid(
            "Provide an organization name of 1–200 characters and notes up to 10000 characters",
        ));
    }
    if let Some(website) = &input.website_url {
        let parsed = reqwest::Url::parse(website)
            .map_err(|_| Error::Invalid("Website must be an absolute HTTP or HTTPS URL"))?;
        if website.len() > 2048
            || !matches!(parsed.scheme(), "http" | "https")
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
        {
            return Err(Error::Invalid(
                "Website must be an HTTP or HTTPS URL without credentials",
            ));
        }
    }
    Ok(())
}
pub async fn list(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: ListContacts,
) -> Result<OrganizationPage, Error> {
    let limit = query.limit.unwrap_or(50);
    if !(1..=100).contains(&limit) || query.search.as_ref().is_some_and(|s| s.len() > 200) {
        return Err(Error::Invalid(
            "Limit must be 1–100 and search at most 200 bytes",
        ));
    }
    let mut tx = pool.begin().await?;
    let access = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        "contacts.organizations.read",
    )
    .await?;
    let mut items = dao::list(&mut tx, actor.workspace, &access, &query, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    tx.commit().await?;
    Ok(OrganizationPage { items, next_cursor })
}
pub async fn get(pool: &PgPool, actor: &ActorContext<'_>, id: Uuid) -> Result<Organization, Error> {
    let mut tx = pool.begin().await?;
    let access = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        "contacts.organizations.read",
    )
    .await?;
    let result = dao::get(&mut tx, actor.workspace, id, &access, false).await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn create(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    key: &str,
    input: OrganizationInput,
) -> Result<Organization, Error> {
    validate(&input)?;
    let mut tx = pool.begin().await?;
    let access = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        "contacts.organizations.create",
    )
    .await?;
    let read = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        "contacts.organizations.read",
    )
    .await?;
    let owner = input.owner_membership_id.unwrap_or(access.membership_id);
    if !access.permits_owner(owner) || !read.permits_owner(owner) {
        return Err(Error::Forbidden);
    }
    let request = serde_json::to_value(&input).expect("organization serializes");
    if let Some(saved) = reliability::replay::<Organization>(
        &mut tx,
        actor.workspace,
        &access,
        "organizations.create",
        key,
        &request,
    )
    .await?
    {
        dao::get(&mut tx, actor.workspace, saved.id, &access, false).await?;
        dao::get(&mut tx, actor.workspace, saved.id, &read, false).await?;
        return Ok(saved);
    }
    access::require_active_member(&mut tx, actor.workspace, owner).await?;
    let id = dao::insert(&mut tx, actor.workspace, owner, &input).await?;
    reliability::audit(
        &mut tx,
        actor,
        &access,
        "organizations.created",
        id,
        serde_json::json!({}),
    )
    .await?;
    let result = dao::get(&mut tx, actor.workspace, id, &read, false).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &access,
        "organizations.create",
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn edit(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: EditOrganization,
) -> Result<Organization, Error> {
    validate(&input.organization)?;
    let mut tx = pool.begin().await?;
    let access = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        "contacts.organizations.update",
    )
    .await?;
    let read = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        "contacts.organizations.read",
    )
    .await?;
    let operation = format!("organizations.update:{id}");
    let request = serde_json::to_value(&input).expect("organization edit serializes");
    if let Some(saved) = reliability::replay::<Organization>(
        &mut tx,
        actor.workspace,
        &access,
        &operation,
        key,
        &request,
    )
    .await?
    {
        dao::get(&mut tx, actor.workspace, id, &access, false).await?;
        dao::get(&mut tx, actor.workspace, id, &read, false).await?;
        return Ok(saved);
    }
    let current = dao::get(&mut tx, actor.workspace, id, &access, true).await?;
    if current.version != input.expected_version {
        return Err(Error::Conflict(
            "Organization changed; reload before editing",
        ));
    }
    let owner = input
        .organization
        .owner_membership_id
        .unwrap_or(current.owner_membership_id);
    if !access.permits_owner(owner) || !read.permits_owner(owner) {
        return Err(Error::Forbidden);
    }
    access::require_active_member(&mut tx, actor.workspace, owner).await?;
    dao::update(&mut tx, actor.workspace, id, owner, &input.organization).await?;
    reliability::audit(
        &mut tx,
        actor,
        &access,
        "organizations.updated",
        id,
        serde_json::json!({}),
    )
    .await?;
    let result = dao::get(&mut tx, actor.workspace, id, &read, false).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &access,
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
    input: super::ArchiveContact,
) -> Result<Organization, Error> {
    let mut tx = pool.begin().await?;
    let access = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        "contacts.organizations.archive",
    )
    .await?;
    let read = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        "contacts.organizations.read",
    )
    .await?;
    let operation = format!("organizations.archive:{id}");
    let request = serde_json::to_value(&input).expect("archive serializes");
    if let Some(saved) = reliability::replay::<Organization>(
        &mut tx,
        actor.workspace,
        &access,
        &operation,
        key,
        &request,
    )
    .await?
    {
        dao::get(&mut tx, actor.workspace, id, &access, false).await?;
        dao::get(&mut tx, actor.workspace, id, &read, false).await?;
        return Ok(saved);
    }
    let current = dao::get(&mut tx, actor.workspace, id, &access, true).await?;
    if current.version != input.expected_version {
        return Err(Error::Conflict(
            "Organization changed; reload before archiving or restoring",
        ));
    }
    dao::archive(&mut tx, actor.workspace, id, input.archived).await?;
    reliability::audit(
        &mut tx,
        actor,
        &access,
        if input.archived {
            "organizations.archived"
        } else {
            "organizations.restored"
        },
        id,
        serde_json::json!({}),
    )
    .await?;
    let result = dao::get(&mut tx, actor.workspace, id, &read, false).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &access,
        &operation,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn relationships(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    contact: Uuid,
) -> Result<Vec<Relationship>, Error> {
    let mut tx = pool.begin().await?;
    let contact_access =
        access::require(&mut tx, actor.identity, actor.workspace, "contacts.read").await?;
    let contact =
        super::dao::get(&mut tx, actor.workspace, contact, &contact_access, false).await?;
    let contact = if let Some(id) = contact.merged_into_id {
        super::dao::get(&mut tx, actor.workspace, id, &contact_access, false).await?
    } else {
        contact
    };
    let organization_access = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        "contacts.organizations.read",
    )
    .await?;
    let result =
        dao::relationships(&mut tx, actor.workspace, contact.id, &organization_access).await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn relate(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    contact: Uuid,
    organization: Uuid,
    key: &str,
    input: RelationshipInput,
) -> Result<Relationship, Error> {
    if input.relationship_label.chars().count() > 120 || input.job_title.chars().count() > 120 {
        return Err(Error::Invalid(
            "Relationship labels and job titles must be at most 120 characters",
        ));
    }
    let mut tx = pool.begin().await?;
    let access = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        "contacts.relationships.manage",
    )
    .await?;
    let read = access::require(&mut tx, actor.identity, actor.workspace, "contacts.read").await?;
    let organization_access = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        "contacts.organizations.read",
    )
    .await?;
    let operation = format!("contacts.relate:{contact}:{organization}");
    let request = serde_json::to_value(&input).expect("relationship serializes");
    let saved = reliability::replay::<Relationship>(
        &mut tx,
        actor.workspace,
        &access,
        &operation,
        key,
        &request,
    )
    .await?;
    let current = super::dao::get(&mut tx, actor.workspace, contact, &access, true).await?;
    super::dao::get(&mut tx, actor.workspace, contact, &read, false).await?;
    let org = dao::get(
        &mut tx,
        actor.workspace,
        organization,
        &organization_access,
        true,
    )
    .await?;
    if let Some(saved) = saved {
        return Ok(saved);
    }
    if current.version != input.expected_contact_version || current.merged_into_id.is_some() {
        return Err(Error::Conflict(
            "Contact changed; reload before changing relationships",
        ));
    }
    if !input.ended && (current.archived || org.archived) {
        return Err(Error::Conflict(
            "Restore archived records before creating relationships",
        ));
    }
    let result = dao::relate(&mut tx, actor.workspace, contact, organization, &input).await?;
    super::dao::touch(&mut tx, actor.workspace, contact).await?;
    reliability::audit(
        &mut tx,
        actor,
        &access,
        if input.ended {
            "contacts.relationship_ended"
        } else {
            "contacts.relationship_set"
        },
        contact,
        serde_json::json!({"organization_id":organization}),
    )
    .await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &access,
        &operation,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}

/// Contacts merge has already authorized and locked both people. Organization
/// rows are not mutated; only this module's relationship records are transferred.
pub(super) async fn merge_relationships(
    db: &mut sqlx::PgConnection,
    workspace: Uuid,
    source: Uuid,
    survivor: Uuid,
) -> Result<(), Error> {
    dao::merge(db, workspace, source, survivor).await
}
