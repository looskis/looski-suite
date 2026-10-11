// SPDX-License-Identifier: AGPL-3.0-only
//! Outbound notification mail through the installation's SMTP configuration.
//! Delivery is at least once: a crash after the SMTP handoff and before commit
//! sends again. Each attempt re-checks the recipient's current access.
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    forms::{FormVersion, Recipient},
    jobs,
};
use jsonwebtoken::{EncodingKey, Header, encode};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, header::ContentType},
    transport::smtp::authentication::Credentials,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize)]
struct SmtpConfig {
    host: String,
    port: u16,
    user: String,
    password: String,
    from: String,
    sender_name: String,
}
pub struct Outbox {
    transport: Option<(AsyncSmtpTransport<Tokio1Executor>, Mailbox)>,
    http: reqwest::Client,
    auth_url: String,
    issuer: String,
    public_url: String,
    admin_key: EncodingKey,
}
impl Outbox {
    /// `smtp` is the private mail configuration shared with Auth; `None`
    /// leaves notifications visibly blocked rather than silently dropped.
    pub fn new(
        smtp: Option<&str>,
        auth_url: String,
        public_url: String,
        secret: &[u8],
    ) -> anyhow::Result<Self> {
        let transport = match smtp {
            None => None,
            Some(text) => {
                let config: SmtpConfig = serde_json::from_str(text)?;
                let from = Mailbox::new(Some(config.sender_name), config.from.parse()?);
                let transport = if config.port == 465 {
                    AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)?
                        .credentials(Credentials::new(config.user, config.password))
                        .timeout(Some(std::time::Duration::from_secs(10)))
                        .build()
                } else {
                    // Only the private local capture inbox listens without TLS.
                    anyhow::ensure!(
                        config.host == "mailpit",
                        "SMTP without TLS is limited to local mail capture"
                    );
                    AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.host)
                        .port(config.port)
                        .timeout(Some(std::time::Duration::from_secs(10)))
                        .build()
                };
                Some((transport, from))
            }
        };
        Ok(Self {
            transport,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            issuer: format!("{public_url}/auth/v1"),
            auth_url,
            public_url,
            admin_key: EncodingKey::from_secret(secret),
        })
    }
    /// The confirmed address Auth holds for a sign-in identity.
    async fn address(&self, subject: &str) -> Result<String, Error> {
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
        let response = self
            .http
            .get(format!("{}/admin/users/{subject}", self.auth_url))
            .bearer_auth(admin)
            .send()
            .await
            .map_err(|_| Error::Unavailable)?;
        if response.status().as_u16() == 404 {
            return Err(Error::NotFound);
        }
        if !response.status().is_success() {
            return Err(Error::Unavailable);
        }
        #[derive(Deserialize)]
        struct User {
            email: Option<String>,
            email_confirmed_at: Option<String>,
        }
        let user: User = response.json().await.map_err(|_| Error::Unavailable)?;
        match (user.email, user.email_confirmed_at) {
            (Some(email), Some(_)) => Ok(email),
            _ => Err(Error::Conflict("Recipient has no confirmed email address")),
        }
    }
    pub(crate) async fn send(&self, to: &str, subject: &str, body: String) -> Result<(), Error> {
        let Some((transport, from)) = &self.transport else {
            return Err(Error::Conflict("Notification email is not configured"));
        };
        let message = Message::builder()
            .from(from.clone())
            .to(to
                .parse()
                .map_err(|_| Error::Conflict("Recipient address is invalid"))?)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body)
            .map_err(|_| Error::Invalid("Notification could not be composed"))?;
        transport
            .send(message)
            .await
            .map_err(|_| Error::Unavailable)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct NotificationContext {
    pub purpose: String,
    pub reference_id: Uuid,
    pub recipient_membership_id: Uuid,
    pub sent: bool,
}

/// Permission the recipient must still hold for each notification purpose.
pub(crate) fn authority(purpose: &str) -> &'static str {
    match purpose {
        "forms.submission" => "forms.submissions.read",
        // Unknown purposes match no grant, so they cannot be delivered.
        _ => "mail.unknown_purpose",
    }
}
pub(crate) async fn notify_submission(
    db: &mut PgConnection,
    workspace: Uuid,
    recipient: &Recipient,
    version: &FormVersion,
    submission: Uuid,
) -> Result<(), Error> {
    let job = jobs::enqueue_recorded(
        db,
        workspace,
        recipient.user,
        recipient.membership,
        &recipient.issuer,
        &recipient.subject,
        &format!("forms.submission:{submission}:{}", recipient.membership),
        "mail.notification",
    )
    .await?;
    let name: String = version.name.chars().take(150).collect();
    sqlx::query("INSERT INTO suite.outbound_messages(job_id,workspace_id,recipient_membership_id,purpose,subject,body,reference_id,link_path) VALUES($1,$2,$3,'forms.submission',$4,$5,$6,$7)")
        .bind(job).bind(workspace).bind(recipient.membership)
        .bind(format!("New response: {name}"))
        // Answers stay in the suite; the message only points to them.
        .bind(format!("A new response was submitted to the form \"{name}\". Sign in to review it."))
        .bind(submission)
        .bind(format!("/workspaces/{workspace}/forms/{}/submissions/{submission}", version.form_id))
        .execute(db).await?;
    Ok(())
}
pub(crate) async fn execute(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    job: Uuid,
    outbox: &Outbox,
) -> Result<(), Error> {
    let (subject, body, link, sent) = sqlx::query_as::<_, (String, String, String, bool)>("SELECT subject,body,link_path,sent_at IS NOT NULL FROM suite.outbound_messages WHERE workspace_id=$1 AND job_id=$2 FOR UPDATE")
        .bind(actor.workspace).bind(job).fetch_optional(&mut *db).await?.ok_or(Error::NotFound)?;
    if sent {
        return Ok(());
    }
    let to = outbox.address(actor.identity.subject()).await?;
    outbox
        .send(
            &to,
            &subject,
            format!("{body}\n\n{}{link}\n", outbox.public_url),
        )
        .await?;
    sqlx::query(
        "UPDATE suite.outbound_messages SET sent_at=now() WHERE workspace_id=$1 AND job_id=$2",
    )
    .bind(actor.workspace)
    .bind(job)
    .execute(db)
    .await?;
    Ok(())
}
pub(crate) async fn context(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    job: Uuid,
) -> Result<Option<NotificationContext>, Error> {
    access::require(db, actor.identity, actor.workspace, "workspace.jobs.read").await?;
    Ok(sqlx::query_as::<_, (String, Uuid, Uuid, bool)>("SELECT purpose,reference_id,recipient_membership_id,sent_at IS NOT NULL FROM suite.outbound_messages WHERE workspace_id=$1 AND job_id=$2")
        .bind(actor.workspace).bind(job).fetch_optional(db).await?
        .map(|(purpose, reference_id, recipient_membership_id, sent)| NotificationContext { purpose, reference_id, recipient_membership_id, sent }))
}
