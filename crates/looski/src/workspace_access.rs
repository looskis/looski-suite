// SPDX-License-Identifier: AGPL-3.0-only
//! Staff grant administration. External memberships remain a separate audience.
mod dao;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    reliability,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Workspace,
    Assigned,
}
impl Scope {
    fn key(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::Assigned => "assigned",
        }
    }
}
#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    pub action: String,
    pub scope: Scope,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Permission {
    pub action: String,
    pub scopes: Vec<Scope>,
}
/// Only implemented actions are assignable. Adding a permission never grants it implicitly.
fn supported() -> Vec<Permission> {
    let workspace = [
        "tasks.configuration.read",
        "tasks.configuration.manage",
        "workspace.read",
        "workspace.jobs.read",
        "workspace.jobs.manage",
        "workspace.modules.read",
        "workspace.modules.manage",
        "crm.configuration.read",
        "crm.configuration.manage",
        "workspace.access.read",
        "workspace.access.admin",
        "workspace.tags.read",
        "workspace.tags.manage",
    ];
    let scoped = [
        "crm.opportunities.tasks.manage",
        "crm.deals.coordination.apply",
        "crm.deals.tasks.manage",
        "crm.activities.read",
        "crm.activities.create",
        "crm.activities.update",
        "crm.activities.archive",
        "tasks.read",
        "tasks.create",
        "tasks.update",
        "tasks.archive",
        "contacts.read",
        "crm.opportunities.read",
        "crm.opportunities.create",
        "crm.opportunities.update",
        "crm.opportunities.archive",
        "crm.deals.read",
        "crm.deals.create",
        "crm.deals.update",
        "crm.deals.archive",
        "crm.deals.tags.manage",
        "crm.deals.participants.manage",
        "crm.opportunities.participants.manage",
        "crm.opportunities.tags.manage",
        "contacts.create",
        "contacts.update",
        "contacts.archive",
        "contacts.merge",
        "contacts.identifiers.share",
        "contacts.organizations.read",
        "contacts.organizations.create",
        "contacts.organizations.update",
        "contacts.organizations.archive",
        "contacts.relationships.manage",
        "contacts.tags.manage",
        "contacts.organizations.tags.manage",
        "contacts.intake.create",
        "contacts.intake.read",
        "contacts.intake.resolve",
    ];
    workspace
        .into_iter()
        .map(|action| Permission {
            action: action.into(),
            scopes: vec![Scope::Workspace],
        })
        .chain(scoped.into_iter().map(|action| Permission {
            action: action.into(),
            scopes: vec![Scope::Workspace, Scope::Assigned],
        }))
        .collect()
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub archived: bool,
    pub grants: Vec<Grant>,
    pub version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveRole {
    pub name: String,
    pub grants: Vec<Grant>,
    pub archived: Option<bool>,
    pub expected_version: Option<i64>,
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MemberState {
    Active,
    Suspended,
    Removed,
}
impl MemberState {
    fn key(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Removed => "removed",
        }
    }
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RoleRef {
    pub id: Uuid,
    pub name: String,
    pub archived: bool,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Member {
    pub id: Uuid,
    pub user_id: Uuid,
    pub display_name: String,
    pub state: MemberState,
    pub roles: Vec<RoleRef>,
    pub version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EditMember {
    pub state: MemberState,
    pub role_ids: Vec<Uuid>,
    pub expected_version: i64,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct AccessQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RolePage {
    pub items: Vec<Role>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct MemberPage {
    pub items: Vec<Member>,
    pub next_cursor: Option<Uuid>,
}
async fn authorize(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    action: &str,
) -> Result<access::Access, Error> {
    let access = access::require(db, actor.identity, actor.workspace, action).await?;
    if !access.workspace_wide {
        return Err(Error::Forbidden);
    }
    Ok(access)
}
fn limit(query: &AccessQuery) -> Result<i64, Error> {
    let limit = query.limit.unwrap_or(50);
    if !(1..=100).contains(&limit) {
        return Err(Error::Invalid("Limit must be 1–100"));
    }
    Ok(limit)
}
pub async fn permissions(
    pool: &PgPool,
    actor: &ActorContext<'_>,
) -> Result<Vec<Permission>, Error> {
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "workspace.access.read").await?;
    Ok(supported())
}
pub async fn roles(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: AccessQuery,
) -> Result<RolePage, Error> {
    let limit = limit(&query)?;
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "workspace.access.read").await?;
    let mut items = dao::roles(&mut tx, actor.workspace, &query, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    Ok(RolePage { items, next_cursor })
}
pub async fn role(pool: &PgPool, actor: &ActorContext<'_>, id: Uuid) -> Result<Role, Error> {
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "workspace.access.read").await?;
    dao::role(&mut tx, actor.workspace, id).await
}
pub async fn members(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: AccessQuery,
) -> Result<MemberPage, Error> {
    let limit = limit(&query)?;
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "workspace.access.read").await?;
    let mut items = dao::members(&mut tx, actor.workspace, &query, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    Ok(MemberPage { items, next_cursor })
}
pub async fn member(pool: &PgPool, actor: &ActorContext<'_>, id: Uuid) -> Result<Member, Error> {
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "workspace.access.read").await?;
    dao::member(&mut tx, actor.workspace, id).await
}
pub async fn save_role(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Option<Uuid>,
    key: &str,
    input: SaveRole,
) -> Result<Role, Error> {
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 120 || name.chars().any(char::is_control) {
        return Err(Error::Invalid(
            "Role name must be 1–120 printable characters",
        ));
    }
    if id.is_some() != input.expected_version.is_some() {
        return Err(Error::Invalid(
            "Updates require expected_version; creation must omit it",
        ));
    }
    if input.grants.len() > 100 {
        return Err(Error::Invalid("At most 100 grants per role"));
    }
    let catalog = supported();
    let mut seen = std::collections::HashSet::new();
    for grant in &input.grants {
        if !catalog
            .iter()
            .any(|p| p.action == grant.action && p.scopes.contains(&grant.scope))
            || !seen.insert((&grant.action, grant.scope.key()))
        {
            return Err(Error::Invalid(
                "Grants must be distinct, implemented action/scope pairs",
            ));
        }
    }
    let mut tx = pool.begin().await?;
    // Serialize role and membership mutations before checking current grant authority.
    authorize(&mut tx, actor, "workspace.access.admin").await?;
    dao::lock(&mut tx, actor.workspace).await?;
    let access = authorize(&mut tx, actor, "workspace.access.admin").await?;
    authorize(&mut tx, actor, "workspace.access.read").await?;
    let operation = id
        .map(|id| format!("access.role.update:{id}"))
        .unwrap_or_else(|| "access.role.create".into());
    let request = serde_json::to_value(&input).expect("role serializes");
    if let Some(saved) =
        reliability::replay(&mut tx, actor.workspace, &access, &operation, key, &request).await?
    {
        return Ok(saved);
    }
    if let Some(id) = id {
        let current = dao::role(&mut tx, actor.workspace, id).await?;
        if Some(current.version) != input.expected_version {
            return Err(Error::Conflict("Role changed; reload before editing"));
        }
    }
    let id = dao::save_role(&mut tx, actor.workspace, id, &input).await?;
    dao::require_administrator(&mut tx, actor.workspace).await?;
    reliability::audit(
        &mut tx,
        actor,
        &access,
        "workspace.role.saved",
        id,
        serde_json::json!({"grants":input.grants,"archived":input.archived}),
    )
    .await?;
    let result = dao::role(&mut tx, actor.workspace, id).await?;
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
pub async fn edit_member(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: EditMember,
) -> Result<Member, Error> {
    if input.role_ids.len() > 50
        || input
            .role_ids
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
            != input.role_ids.len()
    {
        return Err(Error::Invalid("Assign at most 50 distinct roles"));
    }
    if input.state == MemberState::Removed && !input.role_ids.is_empty() {
        return Err(Error::Invalid("Removed memberships must have no roles"));
    }
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, "workspace.access.admin").await?;
    dao::lock(&mut tx, actor.workspace).await?;
    let access = authorize(&mut tx, actor, "workspace.access.admin").await?;
    authorize(&mut tx, actor, "workspace.access.read").await?;
    let operation = format!("access.member.update:{id}");
    let request = serde_json::to_value(&input).expect("member serializes");
    if let Some(saved) =
        reliability::replay(&mut tx, actor.workspace, &access, &operation, key, &request).await?
    {
        return Ok(saved);
    }
    let current = dao::member(&mut tx, actor.workspace, id).await?;
    if current.version != input.expected_version {
        return Err(Error::Conflict("Member changed; reload before editing"));
    }
    for role_id in &input.role_ids {
        if dao::role(&mut tx, actor.workspace, *role_id)
            .await?
            .archived
        {
            return Err(Error::Conflict("Archived roles cannot be assigned"));
        }
    }
    dao::edit_member(&mut tx, actor.workspace, id, &input).await?;
    dao::require_administrator(&mut tx, actor.workspace).await?;
    reliability::audit(
        &mut tx,
        actor,
        &access,
        "workspace.member.saved",
        id,
        serde_json::json!({"state":input.state,"role_ids":input.role_ids}),
    )
    .await?;
    let result = dao::member(&mut tx, actor.workspace, id).await?;
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
