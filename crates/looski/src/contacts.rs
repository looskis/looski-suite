// SPDX-License-Identifier: AGPL-3.0-only
//! Contacts business operations own matching, authorization and transactions.
mod dao;
pub use crate::application::OperationError as Error;
use crate::{access, application::ActorContext, reliability};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Clone, Deserialize, Serialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct IdentifierInput {
    pub kind: String,
    pub value: String,
    pub country: Option<String>,
    pub extension: Option<String>,
}
#[derive(Clone, Deserialize, Serialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ContactInput {
    pub display_name: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub identifiers: Vec<IdentifierInput>,
    pub owner_membership_id: Option<Uuid>,
    /// Explicitly reviewed creation of a different person sharing identifiers.
    #[serde(default)]
    pub allow_shared: bool,
}
#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EditContact {
    pub expected_version: i64,
    #[serde(flatten)]
    pub contact: ContactInput,
}
#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ArchiveContact {
    pub expected_version: i64,
    pub archived: bool,
}
#[derive(Clone, Deserialize, Serialize, JsonSchema, ToSchema)]
pub struct ContactIdentifier {
    pub kind: String,
    pub value: String,
    pub normalized_value: String,
    pub country: Option<String>,
    pub extension: Option<String>,
}
#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
pub struct Contact {
    pub id: Uuid,
    pub display_name: String,
    pub notes: String,
    pub owner_membership_id: Uuid,
    pub archived: bool,
    pub merged_into_id: Option<Uuid>,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
    pub identifiers: Vec<ContactIdentifier>,
}
#[derive(Default, Deserialize, Serialize, JsonSchema, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListContacts {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
    pub archived: Option<bool>,
    pub search: Option<String>,
}
#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
pub struct ContactPage {
    pub items: Vec<Contact>,
    pub next_cursor: Option<Uuid>,
}

struct Identifier {
    kind: String,
    original: String,
    normalized: String,
    country: Option<String>,
    extension: Option<String>,
}
fn validate(input: &ContactInput) -> Result<Vec<Identifier>, Error> {
    if input.display_name.trim().is_empty()
        || input.display_name.chars().count() > 200
        || input.notes.chars().count() > 10000
        || input.identifiers.len() > 20
    {
        return Err(Error::Invalid(
            "Provide a name of 1–200 characters, notes up to 10000 characters, and at most 20 identifiers",
        ));
    }
    let mut identifiers = Vec::new();
    for item in &input.identifiers {
        let original = item.value.trim().to_owned();
        if original.len() > 320 {
            return Err(Error::Invalid("Identifier is too long"));
        }
        let mut extension = item.extension.clone();
        let country = item.country.as_ref().map(|c| c.to_ascii_uppercase());
        let normalized = match item.kind.as_str() {
            "email" => {
                if country.is_some() || extension.is_some() {
                    return Err(Error::Invalid(
                        "Country and extension apply only to phone identifiers",
                    ));
                }
                let email: email_address::EmailAddress = original
                    .parse()
                    .map_err(|_| Error::Invalid("Invalid email address"))?;
                if email.email() != original {
                    return Err(Error::Invalid(
                        "Use the email address without a display name",
                    ));
                }
                let domain = idna::domain_to_ascii(email.domain())
                    .map_err(|_| Error::Invalid("Invalid email domain"))?;
                format!("{}@{}", email.local_part(), domain.to_ascii_lowercase())
            }
            "phone" => {
                let region = country
                    .as_ref()
                    .map(|c| {
                        c.parse::<phonenumber::country::Id>().map_err(|_| {
                            Error::Invalid("Country must be a supported two-letter code")
                        })
                    })
                    .transpose()?;
                let phone = phonenumber::parse(region, &original).map_err(|_| {
                    Error::Invalid(
                        "Provide a valid phone number with country code or explicit country",
                    )
                })?;
                if !phone.is_valid() {
                    return Err(Error::Invalid("Invalid phone number"));
                }
                if let Some(parsed) = phone.extension() {
                    let parsed = parsed.to_string();
                    if extension.as_ref().is_some_and(|e| *e != parsed) {
                        return Err(Error::Invalid("Conflicting phone extensions"));
                    }
                    extension = Some(parsed);
                }
                if extension.as_ref().is_some_and(|e| {
                    e.is_empty() || e.len() > 20 || !e.bytes().all(|b| b.is_ascii_digit())
                }) {
                    return Err(Error::Invalid("Phone extension must contain 1–20 digits"));
                }
                phone.format().mode(phonenumber::Mode::E164).to_string()
            }
            _ => return Err(Error::Invalid("Identifier kind must be email or phone")),
        };
        if !identifiers
            .iter()
            .any(|i: &Identifier| i.kind == item.kind && i.normalized == normalized)
        {
            identifiers.push(Identifier {
                kind: item.kind.clone(),
                original,
                normalized,
                country,
                extension,
            });
        }
    }
    identifiers.sort_by(|a, b| (&a.kind, &a.normalized).cmp(&(&b.kind, &b.normalized)));
    Ok(identifiers)
}

pub async fn list(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: ListContacts,
) -> Result<ContactPage, Error> {
    let (identity, workspace) = (actor.identity, actor.workspace);
    let limit = query.limit.unwrap_or(50);
    if !(1..=100).contains(&limit) || query.search.as_ref().is_some_and(|s| s.len() > 200) {
        return Err(Error::Invalid(
            "Limit must be 1–100 and search at most 200 bytes",
        ));
    }
    let mut tx = pool.begin().await?;
    let access = access::require(&mut tx, identity, workspace, "contacts.read").await?;
    let mut items = dao::list(&mut tx, workspace, &access, &query, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|c| c.id)
    } else {
        None
    };
    tx.commit().await?;
    Ok(ContactPage { items, next_cursor })
}
pub async fn get(pool: &PgPool, actor: &ActorContext<'_>, id: Uuid) -> Result<Contact, Error> {
    let (identity, workspace) = (actor.identity, actor.workspace);
    let mut tx = pool.begin().await?;
    let access = access::require(&mut tx, identity, workspace, "contacts.read").await?;
    let mut contact = dao::get(&mut tx, workspace, id, &access, false).await?;
    if let Some(survivor) = contact.merged_into_id {
        contact = dao::get(&mut tx, workspace, survivor, &access, false).await?;
    }
    tx.commit().await?;
    Ok(contact)
}
pub async fn create(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    key: &str,
    input: ContactInput,
) -> Result<Contact, Error> {
    let (identity, workspace) = (actor.identity, actor.workspace);
    let identifiers = validate(&input)?;
    let mut tx = pool.begin().await?;
    let access = access::require(&mut tx, identity, workspace, "contacts.create").await?;
    let read = access::require(&mut tx, identity, workspace, "contacts.read").await?;
    let owner = input.owner_membership_id.unwrap_or(access.membership_id);
    if !access.permits_owner(owner) || !read.permits_owner(owner) {
        return Err(Error::Forbidden);
    }
    if input.allow_shared {
        let sharing =
            access::require(&mut tx, identity, workspace, "contacts.identifiers.share").await?;
        if !sharing.permits_owner(owner) {
            return Err(Error::Forbidden);
        }
    }
    let request = serde_json::to_value(&input).expect("contact input serializes");
    if let Some(saved) = reliability::replay::<Contact>(
        &mut tx,
        workspace,
        &access,
        "contacts.create",
        key,
        &request,
    )
    .await?
    {
        dao::get(&mut tx, workspace, saved.id, &access, false).await?;
        dao::get(&mut tx, workspace, saved.id, &read, false).await?;
        return Ok(saved);
    }
    access::require_active_member(&mut tx, workspace, owner).await?;
    dao::lock_identifiers(&mut tx, workspace, &identifiers).await?;
    let matches = dao::matches(&mut tx, workspace, &identifiers).await?;
    if !matches.is_empty() && !input.allow_shared {
        // Creation is deliberately not implicit resolution: intake gets a separate operation.
        return Err(Error::Conflict(
            "Identifier match requires review; reuse a permitted contact or explicitly confirm a shared identifier",
        ));
    }
    let id = dao::insert(&mut tx, workspace, owner, &input).await?;
    dao::set_identifiers(&mut tx, workspace, id, &identifiers).await?;
    reliability::audit(
        &mut tx,
        actor,
        &access,
        "contacts.created",
        id,
        serde_json::json!({}),
    )
    .await?;
    let contact = dao::get(&mut tx, workspace, id, &read, false).await?;
    reliability::save_replay(
        &mut tx,
        workspace,
        &access,
        "contacts.create",
        key,
        &request,
        &contact,
    )
    .await?;
    tx.commit().await?;
    Ok(contact)
}
pub async fn edit(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: EditContact,
) -> Result<Contact, Error> {
    let (identity, workspace) = (actor.identity, actor.workspace);
    let identifiers = validate(&input.contact)?;
    let mut tx = pool.begin().await?;
    let access = access::require(&mut tx, identity, workspace, "contacts.update").await?;
    let read = access::require(&mut tx, identity, workspace, "contacts.read").await?;
    let request = serde_json::to_value(&input).expect("edit serializes");
    let operation = format!("contacts.update:{id}");
    if let Some(saved) =
        reliability::replay::<Contact>(&mut tx, workspace, &access, &operation, key, &request)
            .await?
    {
        dao::get(&mut tx, workspace, saved.id, &access, false).await?;
        dao::get(&mut tx, workspace, saved.id, &read, false).await?;
        return Ok(saved);
    }
    dao::lock_identifiers(&mut tx, workspace, &identifiers).await?;
    let existing = dao::get(&mut tx, workspace, id, &access, true).await?;
    if existing.version != input.expected_version || existing.merged_into_id.is_some() {
        return Err(Error::Conflict("Contact changed; reload before editing"));
    }
    let owner = input
        .contact
        .owner_membership_id
        .unwrap_or(existing.owner_membership_id);
    if !access.permits_owner(owner) || !read.permits_owner(owner) {
        return Err(Error::Forbidden);
    }
    access::require_active_member(&mut tx, workspace, owner).await?;
    let matches = dao::matches(&mut tx, workspace, &identifiers).await?;
    if matches.iter().any(|other| *other != id) {
        if !input.contact.allow_shared {
            return Err(Error::Conflict("Shared identifier requires review"));
        }
        let sharing =
            access::require(&mut tx, identity, workspace, "contacts.identifiers.share").await?;
        if !sharing.permits_owner(owner) {
            return Err(Error::Forbidden);
        }
    }
    dao::update(&mut tx, workspace, id, owner, &input.contact).await?;
    dao::set_identifiers(&mut tx, workspace, id, &identifiers).await?;
    reliability::audit(
        &mut tx,
        actor,
        &access,
        "contacts.updated",
        id,
        serde_json::json!({}),
    )
    .await?;
    let contact = dao::get(&mut tx, workspace, id, &read, false).await?;
    reliability::save_replay(
        &mut tx, workspace, &access, &operation, key, &request, &contact,
    )
    .await?;
    tx.commit().await?;
    Ok(contact)
}
pub async fn archive(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: ArchiveContact,
) -> Result<Contact, Error> {
    let (identity, workspace) = (actor.identity, actor.workspace);
    let mut tx = pool.begin().await?;
    let access = access::require(&mut tx, identity, workspace, "contacts.archive").await?;
    let read = access::require(&mut tx, identity, workspace, "contacts.read").await?;
    let request = serde_json::to_value(&input).expect("archive serializes");
    let operation = format!("contacts.archive:{id}");
    if let Some(saved) =
        reliability::replay::<Contact>(&mut tx, workspace, &access, &operation, key, &request)
            .await?
    {
        dao::get(&mut tx, workspace, saved.id, &access, false).await?;
        dao::get(&mut tx, workspace, saved.id, &read, false).await?;
        return Ok(saved);
    }
    let existing = dao::get(&mut tx, workspace, id, &access, true).await?;
    if existing.version != input.expected_version || existing.merged_into_id.is_some() {
        return Err(Error::Conflict(
            "Contact changed; reload before archiving or restoring",
        ));
    }
    dao::archive(&mut tx, workspace, id, input.archived).await?;
    reliability::audit(
        &mut tx,
        actor,
        &access,
        if input.archived {
            "contacts.archived"
        } else {
            "contacts.restored"
        },
        id,
        serde_json::json!({}),
    )
    .await?;
    let contact = dao::get(&mut tx, workspace, id, &read, false).await?;
    reliability::save_replay(
        &mut tx, workspace, &access, &operation, key, &request, &contact,
    )
    .await?;
    tx.commit().await?;
    Ok(contact)
}

#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MergeContacts {
    pub source_id: Uuid,
    pub source_version: i64,
    pub survivor_version: i64,
    /// Explicitly resolved fields; all identifiers on both records are retained.
    pub resolved: ContactInput,
}

pub async fn merge(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    survivor: Uuid,
    key: &str,
    input: MergeContacts,
) -> Result<Contact, Error> {
    let (identity, workspace) = (actor.identity, actor.workspace);
    if survivor == input.source_id {
        return Err(Error::Invalid("Choose two different contacts"));
    }
    let mut identifiers = validate(&input.resolved)?;
    let mut tx = pool.begin().await?;
    let access = access::require(&mut tx, identity, workspace, "contacts.merge").await?;
    let read = access::require(&mut tx, identity, workspace, "contacts.read").await?;
    let update = access::require(&mut tx, identity, workspace, "contacts.update").await?;
    let operation = format!("contacts.merge:{survivor}");
    let request = serde_json::to_value(&input).expect("merge serializes");
    if let Some(saved) =
        reliability::replay::<Contact>(&mut tx, workspace, &access, &operation, key, &request)
            .await?
    {
        for id in [survivor, input.source_id] {
            dao::get(&mut tx, workspace, id, &access, false).await?;
            dao::get(&mut tx, workspace, id, &read, false).await?;
        }
        dao::get(&mut tx, workspace, survivor, &update, false).await?;
        return Ok(saved);
    }
    dao::lock_merge(&mut tx, workspace).await?;
    let source = dao::get(&mut tx, workspace, input.source_id, &read, false).await?;
    let target = dao::get(&mut tx, workspace, survivor, &read, false).await?;
    for contact in [&source, &target] {
        let preserved = validate(&ContactInput {
            display_name: contact.display_name.clone(),
            notes: String::new(),
            identifiers: contact
                .identifiers
                .iter()
                .map(|i| IdentifierInput {
                    kind: i.kind.clone(),
                    value: i.value.clone(),
                    country: i.country.clone(),
                    extension: i.extension.clone(),
                })
                .collect(),
            owner_membership_id: None,
            allow_shared: false,
        })?;
        for item in preserved {
            if !identifiers
                .iter()
                .any(|i| i.kind == item.kind && i.normalized == item.normalized)
            {
                identifiers.push(item);
            }
        }
    }
    if identifiers.len() > 20 {
        return Err(Error::Invalid(
            "Merged contact would exceed the supported limit of 20 identifiers; no records were changed",
        ));
    }
    identifiers.sort_by(|a, b| (&a.kind, &a.normalized).cmp(&(&b.kind, &b.normalized)));
    dao::lock_identifiers(&mut tx, workspace, &identifiers).await?;
    let mut ids = [survivor, input.source_id];
    ids.sort();
    for id in ids {
        dao::get(&mut tx, workspace, id, &access, true).await?;
    }
    let source = dao::get(&mut tx, workspace, input.source_id, &read, false).await?;
    let target = dao::get(&mut tx, workspace, survivor, &update, false).await?;
    if source.version != input.source_version
        || target.version != input.survivor_version
        || source.merged_into_id.is_some()
        || target.merged_into_id.is_some()
    {
        return Err(Error::Conflict(
            "Contacts changed; reload the merge preview",
        ));
    }
    let owner = input
        .resolved
        .owner_membership_id
        .unwrap_or(target.owner_membership_id);
    if !access.permits_owner(owner) || !update.permits_owner(owner) || !read.permits_owner(owner) {
        return Err(Error::Forbidden);
    }
    access::require_active_member(&mut tx, workspace, owner).await?;
    if dao::matches(&mut tx, workspace, &identifiers)
        .await?
        .iter()
        .any(|id| !ids.contains(id))
    {
        if !input.resolved.allow_shared {
            return Err(Error::Conflict(
                "Other contacts share these identifiers; explicit review is required",
            ));
        }
        let sharing =
            access::require(&mut tx, identity, workspace, "contacts.identifiers.share").await?;
        if !sharing.permits_owner(owner) {
            return Err(Error::Forbidden);
        }
    }
    dao::update(&mut tx, workspace, survivor, owner, &input.resolved).await?;
    dao::set_identifiers(&mut tx, workspace, survivor, &identifiers).await?;
    dao::merge_into(&mut tx, workspace, input.source_id, survivor).await?;
    reliability::audit(
        &mut tx,
        actor,
        &access,
        "contacts.merged",
        input.source_id,
        serde_json::json!({"survivor_id":survivor}),
    )
    .await?;
    let contact = dao::get(&mut tx, workspace, survivor, &read, false).await?;
    reliability::save_replay(
        &mut tx, workspace, &access, &operation, key, &request, &contact,
    )
    .await?;
    tx.commit().await?;
    Ok(contact)
}
