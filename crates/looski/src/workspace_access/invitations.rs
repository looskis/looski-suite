// SPDX-License-Identifier: AGPL-3.0-only
//! Invitation identity proof is independent of workspace authorization and Contacts.
mod dao;
pub mod delivery;
use super::{AccessQuery, Member};
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    reliability,
};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone)]
pub struct TokenIssuer {
    signing: EncodingKey,
    verification: DecodingKey,
}
#[derive(Serialize, Deserialize, Clone)]
struct Claims {
    sub: Uuid,
    workspace: Uuid,
    nonce: Uuid,
    exp: u64,
    aud: String,
}
impl TokenIssuer {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            signing: EncodingKey::from_secret(secret),
            verification: DecodingKey::from_secret(secret),
        }
    }
    fn issue(&self, record: &Record) -> Result<String, Error> {
        encode(
            &Header::default(),
            &Claims {
                sub: record.invitation.id,
                workspace: record.invitation.workspace_id,
                nonce: record.nonce,
                exp: record.expires_epoch as u64,
                aud: "looski-workspace-invitation".into(),
            },
            &self.signing,
        )
        .map_err(|_| Error::Invalid("Invitation could not be issued"))
    }
    fn verify(&self, token: &str) -> Result<Claims, Error> {
        if token.len() > 2048 {
            return Err(Error::NotFound);
        }
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(&["looski-workspace-invitation"]);
        validation.leeway = 0;
        decode::<Claims>(token, &self.verification, &validation)
            .map(|v| v.claims)
            .map_err(|_| Error::NotFound)
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum InvitationState {
    Pending,
    Accepted,
    Revoked,
    Expired,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct InvitationRole {
    pub id: Uuid,
    pub name: String,
    pub version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Invitation {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub email: String,
    pub state: InvitationState,
    pub expires_at: String,
    pub created_at: String,
    pub version: i64,
    pub inviter_membership_id: Uuid,
    pub accepted_membership_id: Option<Uuid>,
    pub roles: Vec<InvitationRole>,
}
#[derive(Deserialize)]
struct Record {
    invitation: Invitation,
    nonce: Uuid,
    expires_epoch: i64,
    inviter_identity_id: Uuid,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct InvitationPage {
    pub items: Vec<Invitation>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct InvitationLink {
    pub invitation: Invitation,
    pub token: Option<String>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateInvitation {
    pub email: String,
    pub role_ids: Vec<Uuid>,
    pub expires_in_hours: Option<i64>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RevokeInvitation {
    pub expected_version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AcceptInvitation {
    pub token: String,
    pub display_name: String,
}
async fn admin(db: &mut PgConnection, actor: &ActorContext<'_>) -> Result<access::Access, Error> {
    super::authorize(db, actor, "workspace.access.read").await?;
    super::authorize(db, actor, "workspace.access.admin").await
}
fn link(issuer: &TokenIssuer, record: Record) -> Result<InvitationLink, Error> {
    let token = if record.invitation.state == InvitationState::Pending {
        Some(issuer.issue(&record)?)
    } else {
        None
    };
    Ok(InvitationLink {
        invitation: record.invitation,
        token,
    })
}
pub async fn list(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: AccessQuery,
) -> Result<InvitationPage, Error> {
    let limit = super::limit(&query)?;
    let mut tx = pool.begin().await?;
    super::authorize(&mut tx, actor, "workspace.access.read").await?;
    let mut items = dao::list(&mut tx, actor.workspace, &query, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    Ok(InvitationPage { items, next_cursor })
}
pub async fn get(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    issuer: &TokenIssuer,
    id: Uuid,
) -> Result<InvitationLink, Error> {
    let mut tx = pool.begin().await?;
    admin(&mut tx, actor).await?;
    link(issuer, dao::record(&mut tx, actor.workspace, id).await?)
}
pub async fn create(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    issuer: &TokenIssuer,
    key: &str,
    input: CreateInvitation,
) -> Result<InvitationLink, Error> {
    let email = input.email.trim();
    if email.len() > 254 || !email_address::EmailAddress::is_valid(email) {
        return Err(Error::Invalid("Enter a valid invitation email"));
    }
    let hours = input.expires_in_hours.unwrap_or(72);
    if !(1..=168).contains(&hours)
        || input.role_ids.is_empty()
        || input.role_ids.len() > 50
        || input
            .role_ids
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
            != input.role_ids.len()
    {
        return Err(Error::Invalid(
            "Choose 1–50 distinct roles and an expiry of 1–168 hours",
        ));
    }
    let mut tx = pool.begin().await?;
    admin(&mut tx, actor).await?;
    super::dao::lock(&mut tx, actor.workspace).await?;
    let access = admin(&mut tx, actor).await?;
    let request = serde_json::to_value(&input).expect("invitation serializes");
    if let Some(id) = reliability::replay::<Uuid>(
        &mut tx,
        actor.workspace,
        &access,
        "access.invitation.create",
        key,
        &request,
    )
    .await?
    {
        return link(issuer, dao::record(&mut tx, actor.workspace, id).await?);
    }
    for id in &input.role_ids {
        if super::dao::role(&mut tx, actor.workspace, *id)
            .await?
            .archived
        {
            return Err(Error::Conflict("Archived roles cannot be invited"));
        }
    }
    let id = dao::create(
        &mut tx,
        actor,
        &access,
        &email.to_lowercase(),
        hours,
        &input.role_ids,
    )
    .await?;
    reliability::audit(
        &mut tx,
        actor,
        &access,
        "workspace.invitation.created",
        id,
        serde_json::json!({"role_ids":input.role_ids,"expires_in_hours":hours}),
    )
    .await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &access,
        "access.invitation.create",
        key,
        &request,
        &id,
    )
    .await?;
    let result = link(issuer, dao::record(&mut tx, actor.workspace, id).await?)?;
    tx.commit().await?;
    Ok(result)
}
pub async fn revoke(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: RevokeInvitation,
) -> Result<Invitation, Error> {
    let mut tx = pool.begin().await?;
    admin(&mut tx, actor).await?;
    super::dao::lock(&mut tx, actor.workspace).await?;
    let access = admin(&mut tx, actor).await?;
    let operation = format!("access.invitation.revoke:{id}");
    let request = serde_json::to_value(&input).expect("revoke serializes");
    if let Some(saved) =
        reliability::replay(&mut tx, actor.workspace, &access, &operation, key, &request).await?
    {
        return Ok(saved);
    }
    let record = dao::record(&mut tx, actor.workspace, id).await?;
    if record.invitation.version != input.expected_version {
        return Err(Error::Conflict(
            "Invitation changed; reload before revoking",
        ));
    }
    if !matches!(
        record.invitation.state,
        InvitationState::Pending | InvitationState::Expired
    ) {
        return Err(Error::Conflict("Only pending invitations can be revoked"));
    }
    dao::revoke(&mut tx, actor.workspace, id).await?;
    reliability::audit(
        &mut tx,
        actor,
        &access,
        "workspace.invitation.revoked",
        id,
        serde_json::json!({}),
    )
    .await?;
    let result = dao::record(&mut tx, actor.workspace, id).await?.invitation;
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
pub async fn accept(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    issuer: &TokenIssuer,
    id: Uuid,
    key: &str,
    input: AcceptInvitation,
) -> Result<Member, Error> {
    let claims = issuer.verify(&input.token)?;
    if claims.sub != id || claims.workspace != actor.workspace {
        return Err(Error::NotFound);
    }
    let email = actor.identity.confirmed_email().ok_or(Error::Forbidden)?;
    let name = input.display_name.trim();
    if name.is_empty() || name.chars().count() > 120 || name.chars().any(char::is_control) {
        return Err(Error::Invalid(
            "Display name must be 1–120 printable characters",
        ));
    }
    let mut tx = pool.begin().await?;
    super::dao::lock(&mut tx, actor.workspace).await?;
    let record = dao::record(&mut tx, actor.workspace, id).await?;
    if record.nonce != claims.nonce || record.invitation.email != email.to_lowercase() {
        return Err(Error::Forbidden);
    }
    if !matches!(
        record.invitation.state,
        InvitationState::Pending | InvitationState::Accepted
    ) {
        return Err(Error::Conflict("Invitation is no longer pending"));
    }
    dao::require_inviter(&mut tx, actor.workspace, &record).await?;
    for role in &record.invitation.roles {
        let current = super::dao::role(&mut tx, actor.workspace, role.id).await?;
        if current.archived || current.version != role.version {
            return Err(Error::Conflict(
                "Invited role changed; ask for a new invitation",
            ));
        }
    }
    let access = dao::recipient(
        &mut tx,
        actor,
        name,
        record.invitation.accepted_membership_id,
    )
    .await?;
    let operation = format!("access.invitation.accept:{id}");
    // Bind retry semantics without persisting the bearer token.
    let request = serde_json::json!({"invitation_id":id,"nonce":claims.nonce,"display_name":input.display_name});
    if let Some(saved) =
        reliability::replay::<Uuid>(&mut tx, actor.workspace, &access, &operation, key, &request)
            .await?
    {
        return super::dao::member(&mut tx, actor.workspace, saved).await;
    }
    if record.invitation.state == InvitationState::Pending {
        dao::accept(&mut tx, actor.workspace, id, access.membership_id).await?;
        reliability::audit(
            &mut tx,
            actor,
            &access,
            "workspace.invitation.accepted",
            id,
            serde_json::json!({"membership_id":access.membership_id}),
        )
        .await?;
    }
    let result = super::dao::member(&mut tx, actor.workspace, access.membership_id).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &access,
        &operation,
        key,
        &request,
        &access.membership_id,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}

#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PreviewInvitation {
    pub token: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct InvitationPreview {
    pub invitation: Invitation,
    pub workspace_name: String,
}
/// Read-only recipient preview uses a protected POST body to keep the bearer
/// token out of URL/query logs. It does not create an identity or membership.
pub async fn preview(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    issuer: &TokenIssuer,
    id: Uuid,
    input: PreviewInvitation,
) -> Result<InvitationPreview, Error> {
    let claims = issuer.verify(&input.token)?;
    if claims.sub != id || claims.workspace != actor.workspace {
        return Err(Error::NotFound);
    }
    let email = actor.identity.confirmed_email().ok_or(Error::Forbidden)?;
    let mut tx = pool.begin().await?;
    let record = dao::record(&mut tx, actor.workspace, id).await?;
    if record.nonce != claims.nonce || record.invitation.email != email.to_lowercase() {
        return Err(Error::Forbidden);
    }
    if !matches!(
        record.invitation.state,
        InvitationState::Pending | InvitationState::Accepted
    ) {
        return Err(Error::Conflict("Invitation is no longer pending"));
    }
    dao::require_inviter(&mut tx, actor.workspace, &record).await?;
    if let Some(member) = record.invitation.accepted_membership_id {
        dao::require_accepted_recipient(&mut tx, actor, member).await?;
    }
    let workspace_name = dao::workspace_name(&mut tx, actor.workspace).await?;
    Ok(InvitationPreview {
        invitation: record.invitation,
        workspace_name,
    })
}
