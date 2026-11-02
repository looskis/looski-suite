// SPDX-License-Identifier: AGPL-3.0-only
//! Controlled identity setup through private Auth; delivery is at least once.
use super::{InvitationState, Record, TokenIssuer, admin, dao};
use crate::{
    application::{ActorContext, OperationError as Error},
    jobs, reliability,
};
use jsonwebtoken::{EncodingKey, Header, encode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use utoipa::ToSchema;
use uuid::Uuid;

pub struct Mailer {
    enabled: bool,
    http: reqwest::Client,
    auth_url: String,
    issuer: String,
    public_url: String,
    admin_key: EncodingKey,
    invitations: TokenIssuer,
}
impl Mailer {
    pub fn new(
        enabled: bool,
        auth_url: String,
        public_url: String,
        issuer: String,
        secret: &[u8],
        invitation_key: &[u8],
    ) -> Result<Self, reqwest::Error> {
        Ok(Self {
            enabled,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            auth_url,
            issuer,
            public_url,
            admin_key: EncodingKey::from_secret(secret),
            invitations: TokenIssuer::new(invitation_key),
        })
    }
    async fn deliver(&self, record: &Record) -> Result<(), Error> {
        if !self.enabled {
            return Err(Error::Conflict("Authentication email is not configured"));
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| Error::Unavailable)?
            .as_secs();
        let admin = encode(
            &Header::default(),
            &serde_json::json!({"role":"service_role","iss":self.issuer,"exp":now+60}),
            &self.admin_key,
        )
        .map_err(|_| Error::Unavailable)?;
        // An existing identity is never overwritten or assigned a password here.
        let response = self
            .http
            .post(format!("{}/admin/users", self.auth_url))
            .bearer_auth(admin)
            .json(&serde_json::json!({"email":record.invitation.email,"email_confirm":false}))
            .send()
            .await
            .map_err(|_| Error::Unavailable)?;
        if !response.status().is_success() {
            let status = response.status();
            let body: serde_json::Value = response.json().await.map_err(|_| Error::Unavailable)?;
            if status.as_u16() != 422
                || body.get("error_code").and_then(|s| s.as_str()) != Some("email_exists")
            {
                return Err(Error::Unavailable);
            }
        }
        let mut target = reqwest::Url::parse(&format!("{}/invitations/accept", self.public_url))
            .map_err(|_| Error::Unavailable)?;
        target
            .query_pairs_mut()
            .append_pair("workspace", &record.invitation.workspace_id.to_string())
            .append_pair("invitation", &record.invitation.id.to_string())
            .append_pair("token", &self.invitations.issue(record)?);
        // Recovery verifies new, unconfirmed identities as well as existing ones.
        // It leaves the password unchanged until its owner explicitly sets one.
        let response = self
            .http
            .post(format!("{}/recover", self.auth_url))
            .query(&[("redirect_to", target.as_str())])
            .json(&serde_json::json!({"email":record.invitation.email}))
            .send()
            .await
            .map_err(|_| Error::Unavailable)?;
        if !response.status().is_success() {
            return Err(Error::Unavailable);
        }
        Ok(())
    }
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SendInvitation {
    pub expected_version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct InvitationJobContext {
    pub invitation_id: Uuid,
    pub email: String,
}
async fn current(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<Record, Error> {
    let record = dao::record(db, actor.workspace, id).await?;
    if record.invitation.state != InvitationState::Pending {
        return Err(Error::Conflict("Only pending invitations can be sent"));
    }
    dao::require_inviter(db, actor.workspace, &record).await?;
    for role in &record.invitation.roles {
        let r = super::super::dao::role(db, actor.workspace, role.id).await?;
        if r.archived || r.version != role.version {
            return Err(Error::Conflict(
                "Invited role changed; ask for a new invitation",
            ));
        }
    }
    Ok(record)
}
pub async fn send(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: SendInvitation,
    enabled: bool,
) -> Result<jobs::Job, Error> {
    let mut tx = pool.begin().await?;
    admin(&mut tx, actor).await?;
    super::super::dao::lock(&mut tx, actor.workspace).await?;
    let access = admin(&mut tx, actor).await?;
    let request = serde_json::to_value(&input).expect("send serializes");
    let operation = format!("access.invitation.send:{id}");
    if let Some(job) =
        reliability::replay::<Uuid>(&mut tx, actor.workspace, &access, &operation, key, &request)
            .await?
    {
        return jobs::record(&mut tx, actor.workspace, job).await;
    }
    if !enabled {
        return Err(Error::Conflict("Authentication email is not configured"));
    }
    let record = current(&mut tx, actor, id).await?;
    if record.invitation.version != input.expected_version {
        return Err(Error::Conflict("Invitation changed; reload before sending"));
    }
    let deliveries = dao::delivery_jobs(&mut tx, actor.workspace, id).await?;
    if jobs::pending(&mut tx, actor.workspace, &deliveries).await? {
        return Err(Error::Conflict("An invitation email is already queued"));
    }
    let job = jobs::enqueue_invitation(
        &mut tx,
        actor,
        &access,
        &format!("invitation:{id}:{}", Uuid::new_v4()),
    )
    .await?;
    dao::add_delivery(&mut tx, actor.workspace, id, job).await?;
    reliability::audit(
        &mut tx,
        actor,
        &access,
        "workspace.invitation.email_queued",
        id,
        serde_json::json!({"job_id":job}),
    )
    .await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &access,
        &operation,
        key,
        &request,
        &job,
    )
    .await?;
    let result = jobs::record(&mut tx, actor.workspace, job).await?;
    tx.commit().await?;
    Ok(result)
}
pub(crate) async fn execute(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    job: Uuid,
    mailer: &Mailer,
) -> Result<(), Error> {
    admin(db, actor).await?;
    let id = dao::delivery_invitation(db, actor.workspace, job).await?;
    let record = current(db, actor, id).await?;
    mailer.deliver(&record).await
}
pub(crate) async fn context(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    job: Uuid,
) -> Result<Option<InvitationJobContext>, Error> {
    match super::super::authorize(db, actor, "workspace.access.read").await {
        Ok(_) => (),
        Err(Error::Forbidden) => return Ok(None),
        Err(e) => return Err(e),
    }
    let id = dao::delivery_invitation(db, actor.workspace, job).await?;
    let record = dao::record(db, actor.workspace, id).await?;
    Ok(Some(InvitationJobContext {
        invitation_id: id,
        email: record.invitation.email,
    }))
}
