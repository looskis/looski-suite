// SPDX-License-Identifier: AGPL-3.0-only
//! Web support. Customer capability authority is confined to one public projection;
//! it never becomes an ActorContext or a staff/contact identity.
mod dao;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    contacts::references,
    knowledge, modules, reliability,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct HelpdeskSettings {
    pub public_key: Uuid,
    pub accepting: bool,
    pub version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfigureHelpdesk {
    pub accepting: bool,
    pub expected_version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct HelpdeskIntake {
    pub accepting: bool,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct OpenTicket {
    pub subject: String,
    pub requester_name: String,
    pub body: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TicketReceipt {
    pub id: Uuid,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Ticket {
    pub id: Uuid,
    pub subject: String,
    pub requester_name: String,
    pub status: String,
    pub assignee_id: Option<Uuid>,
    pub contact_id: Option<Uuid>,
    pub archived: bool,
    pub customer_revoked: bool,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub articles: Vec<knowledge::LinkedKnowledgeDocument>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct CustomerTicket {
    pub id: Uuid,
    pub subject: String,
    pub status: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TicketMessage {
    pub id: Uuid,
    pub kind: String,
    pub body: String,
    pub created_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TicketPage {
    pub items: Vec<Ticket>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TicketMessages {
    pub items: Vec<TicketMessage>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct TicketQuery {
    pub after: Option<Uuid>,
    pub limit: Option<i64>,
    pub archived: Option<bool>,
    pub status: Option<String>,
    pub contact_id: Option<Uuid>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct MessageQuery {
    pub after: Option<Uuid>,
    pub limit: Option<i64>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateTicket {
    pub expected_version: i64,
    pub status: String,
    pub assignee_id: Option<Uuid>,
    pub archived: bool,
    #[serde(default)]
    pub revoke_customer_access: bool,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LinkTicket {
    pub expected_version: i64,
    pub contact_id: Option<Uuid>,
    #[serde(default)]
    pub article_ids: Vec<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplyTicket {
    pub body: String,
    #[serde(default)]
    pub internal: bool,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CustomerReply {
    pub body: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct HelpdeskMember {
    pub id: Uuid,
    pub display_name: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TicketEvent {
    pub ticket_id: Uuid,
    pub subject: String,
    pub action: String,
}

fn digest(s: &str) -> String {
    Sha256::digest(s.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn request_hash<T: Serialize>(v: &T) -> serde_json::Value {
    serde_json::json!({"sha256":digest(&serde_json::to_string(v).expect("request serializes"))})
}
fn code_hash(code: &str) -> Result<String, Error> {
    if code.len() != 64
        || !code
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        return Err(Error::NotFound);
    }
    Ok(digest(code))
}
fn text(value: &str, max: usize) -> Result<(), Error> {
    if value.trim().is_empty()
        || value.chars().count() > max
        || value
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t' && c != '\r')
    {
        return Err(Error::Invalid(
            "Text is empty, too long or contains control characters",
        ));
    }
    Ok(())
}
fn limit(n: Option<i64>) -> Result<i64, Error> {
    let n = n.unwrap_or(25);
    if !(1..=100).contains(&n) {
        return Err(Error::Invalid("Limit must be 1–100"));
    }
    Ok(n)
}
fn status(s: &str) -> Result<(), Error> {
    if !["open", "waiting", "resolved"].contains(&s) {
        return Err(Error::Invalid("Status must be open, waiting or resolved"));
    }
    Ok(())
}
async fn authority(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    action: &str,
) -> Result<access::Access, Error> {
    modules::require(db, a.workspace, "helpdesk").await?;
    access::require(db, a.identity, a.workspace, action).await
}
async fn permitted(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    id: Uuid,
    action: &str,
) -> Result<(access::Access, Ticket), Error> {
    let read = authority(db, a, "helpdesk.read").await?;
    let ticket = dao::ticket(db, a.workspace, id, &read).await?;
    let access = authority(db, a, action).await?;
    if !access.workspace_wide && ticket.assignee_id != Some(access.membership_id) {
        return Err(Error::Forbidden);
    }
    Ok((access, ticket))
}
async fn present(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    mut t: Ticket,
) -> Result<Ticket, Error> {
    let ids = dao::articles(db, a.workspace, t.id).await?;
    t.articles = knowledge::previews(db, a, &ids).await?;
    if let Some(id) = t.contact_id {
        t.contact_id = references::visible(db, a, references::Kind::Contact, &[id])
            .await?
            .first()
            .map(|r| r.canonical_id);
    }
    Ok(t)
}
pub async fn settings(pool: &PgPool, a: &ActorContext<'_>) -> Result<HelpdeskSettings, Error> {
    let mut tx = pool.begin().await?;
    let p = authority(&mut tx, a, "helpdesk.configure").await?;
    if !p.workspace_wide {
        return Err(Error::Forbidden);
    }
    // Missing configuration reads as closed; its stable public key is materialized only on explicit configuration.
    Ok(dao::settings(&mut tx, a.workspace)
        .await?
        .unwrap_or(HelpdeskSettings {
            public_key: Uuid::nil(),
            accepting: false,
            version: 0,
        }))
}
pub async fn configure(
    pool: &PgPool,
    a: &ActorContext<'_>,
    key: &str,
    input: ConfigureHelpdesk,
) -> Result<HelpdeskSettings, Error> {
    let mut tx = pool.begin().await?;
    modules::lock_workspace(&mut tx, a.workspace).await?;
    let p = authority(&mut tx, a, "helpdesk.configure").await?;
    if !p.workspace_wide {
        return Err(Error::Forbidden);
    }
    let req = request_hash(&input);
    if reliability::replay::<bool>(&mut tx, a.workspace, &p, "helpdesk.configure", key, &req)
        .await?
        .is_none()
    {
        let current = dao::settings(&mut tx, a.workspace).await?;
        if current.as_ref().map_or(0, |s| s.version) != input.expected_version {
            return Err(Error::Conflict("Settings changed; reload"));
        }
        dao::configure(&mut tx, a.workspace, input.accepting).await?;
        reliability::audit(
            &mut tx,
            a,
            &p,
            "helpdesk.configured",
            a.workspace,
            serde_json::json!({"accepting":input.accepting}),
        )
        .await?;
        reliability::save_replay(
            &mut tx,
            a.workspace,
            &p,
            "helpdesk.configure",
            key,
            &req,
            &true,
        )
        .await?;
    }
    let result = dao::settings(&mut tx, a.workspace)
        .await?
        .ok_or(Error::NotFound)?;
    tx.commit().await?;
    Ok(result)
}
pub async fn directory(pool: &PgPool, a: &ActorContext<'_>) -> Result<Vec<HelpdeskMember>, Error> {
    let mut tx = pool.begin().await?;
    authority(&mut tx, a, "helpdesk.manage").await?;
    authority(&mut tx, a, "helpdesk.read").await?;
    dao::directory(&mut tx, a.workspace).await
}
pub async fn list(
    pool: &PgPool,
    a: &ActorContext<'_>,
    q: TicketQuery,
) -> Result<TicketPage, Error> {
    let n = limit(q.limit)?;
    if let Some(s) = &q.status {
        status(s)?;
    }
    let mut tx = pool.begin().await?;
    let p = authority(&mut tx, a, "helpdesk.read").await?;
    let family = if let Some(id) = q.contact_id {
        references::contact_family(&mut tx, a, id).await?
    } else {
        Vec::new()
    };
    let mut rows = dao::list(&mut tx, a.workspace, &p, &q, &family, n + 1).await?;
    let more = rows.len() > n as usize;
    rows.truncate(n as usize);
    let next_cursor = if more {
        rows.last().map(|t| t.id)
    } else {
        None
    };
    let mut items = Vec::new();
    for t in rows {
        items.push(present(&mut tx, a, t).await?);
    }
    Ok(TicketPage { items, next_cursor })
}
pub async fn get(pool: &PgPool, a: &ActorContext<'_>, id: Uuid) -> Result<Ticket, Error> {
    let mut tx = pool.begin().await?;
    let (_, t) = permitted(&mut tx, a, id, "helpdesk.read").await?;
    present(&mut tx, a, t).await
}
pub async fn messages(
    pool: &PgPool,
    a: &ActorContext<'_>,
    id: Uuid,
    q: MessageQuery,
) -> Result<TicketMessages, Error> {
    let mut tx = pool.begin().await?;
    permitted(&mut tx, a, id, "helpdesk.read").await?;
    message_page(&mut tx, a.workspace, id, false, q).await
}
async fn message_page(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    public: bool,
    q: MessageQuery,
) -> Result<TicketMessages, Error> {
    let n = limit(q.limit)?;
    let mut items = dao::messages(db, ws, id, public, q.after, n + 1).await?;
    let more = items.len() > n as usize;
    items.truncate(n as usize);
    let next_cursor = if more {
        items.last().map(|m| m.id)
    } else {
        None
    };
    Ok(TicketMessages { items, next_cursor })
}
pub async fn update(
    pool: &PgPool,
    a: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: UpdateTicket,
) -> Result<Ticket, Error> {
    let mut tx = pool.begin().await?;
    modules::lock_workspace(&mut tx, a.workspace).await?;
    let (p, t) = permitted(&mut tx, a, id, "helpdesk.manage").await?;
    let operation = format!("helpdesk.update:{id}");
    let req = request_hash(&input);
    if reliability::replay::<bool>(&mut tx, a.workspace, &p, &operation, key, &req)
        .await?
        .is_none()
    {
        status(&input.status)?;
        if t.version != input.expected_version {
            return Err(Error::Conflict("Ticket changed; reload before editing"));
        }
        if let Some(member) = input.assignee_id {
            access::require_active_member(&mut tx, a.workspace, member).await?;
        }
        // Assigned-only managers may neither give themselves new scope nor lose read scope in a response.
        if !p.workspace_wide && input.assignee_id != t.assignee_id {
            return Err(Error::Forbidden);
        }
        dao::update(&mut tx, a.workspace, id, &input).await?;
        dao::event(&mut tx, a.workspace, id, "updated", Some(p.user_id)).await?;
        reliability::audit(
            &mut tx,
            a,
            &p,
            "helpdesk.ticket.updated",
            id,
            serde_json::json!({"version":t.version+1}),
        )
        .await?;
        reliability::save_replay(&mut tx, a.workspace, &p, &operation, key, &req, &true).await?;
    }
    // A reassignment that would remove the caller's read authority rolls back.
    let read = authority(&mut tx, a, "helpdesk.read").await?;
    let t = dao::ticket(&mut tx, a.workspace, id, &read).await?;
    let result = present(&mut tx, a, t).await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn link(
    pool: &PgPool,
    a: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: LinkTicket,
) -> Result<Ticket, Error> {
    let mut tx = pool.begin().await?;
    modules::lock_workspace(&mut tx, a.workspace).await?;
    let (p, t) = permitted(&mut tx, a, id, "helpdesk.manage").await?;
    let operation = format!("helpdesk.link:{id}");
    let req = request_hash(&input);
    if reliability::replay::<bool>(&mut tx, a.workspace, &p, &operation, key, &req)
        .await?
        .is_none()
    {
        if t.archived {
            return Err(Error::Conflict("Restore the ticket before linking"));
        }
        if t.version != input.expected_version {
            return Err(Error::Conflict("Ticket changed; reload before editing"));
        }
        if input.article_ids.len() > 20 {
            return Err(Error::Invalid("At most 20 articles"));
        }
        let mut ids = input.article_ids.clone();
        ids.sort_unstable();
        ids.dedup();
        let previews = knowledge::previews(&mut tx, a, &ids).await?;
        if previews.iter().any(|p| p.preview.is_none()) {
            return Err(Error::NotFound);
        }
        let contact = if let Some(id) = input.contact_id {
            Some(
                references::for_mutation(&mut tx, a, references::Kind::Contact, id)
                    .await?
                    .canonical_id,
            )
        } else {
            None
        };
        // Removing a hidden association also requires permission over the old contact.
        if let Some(old) = t.contact_id {
            references::require(&mut tx, a, references::Kind::Contact, old).await?;
        }
        dao::link(&mut tx, a.workspace, id, contact, &ids).await?;
        dao::event(&mut tx, a.workspace, id, "linked", Some(p.user_id)).await?;
        reliability::audit(
            &mut tx,
            a,
            &p,
            "helpdesk.ticket.linked",
            id,
            serde_json::json!({}),
        )
        .await?;
        reliability::save_replay(&mut tx, a.workspace, &p, &operation, key, &req, &true).await?;
    }
    let read = authority(&mut tx, a, "helpdesk.read").await?;
    let t = dao::ticket(&mut tx, a.workspace, id, &read).await?;
    let result = present(&mut tx, a, t).await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn reply(
    pool: &PgPool,
    a: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: ReplyTicket,
) -> Result<TicketMessage, Error> {
    let mut tx = pool.begin().await?;
    modules::lock_workspace(&mut tx, a.workspace).await?;
    let (p, t) = permitted(&mut tx, a, id, "helpdesk.reply").await?;
    if t.archived {
        return Err(Error::Conflict("Restore the ticket before replying"));
    }
    text(&input.body, 20000)?;
    let operation = format!("helpdesk.reply:{id}");
    let req = request_hash(&input);
    let message = if let Some(saved) =
        reliability::replay::<Uuid>(&mut tx, a.workspace, &p, &operation, key, &req).await?
    {
        saved
    } else {
        let kind = if input.internal { "note" } else { "reply" };
        let message = dao::message(
            &mut tx,
            a.workspace,
            id,
            kind,
            &input.body,
            Some(p.membership_id),
        )
        .await?;
        dao::touch(&mut tx, a.workspace, id, false).await?;
        dao::event(&mut tx, a.workspace, id, kind, Some(p.user_id)).await?;
        reliability::audit(
            &mut tx,
            a,
            &p,
            "helpdesk.message.created",
            message,
            serde_json::json!({"ticket_id":id,"kind":kind}),
        )
        .await?;
        reliability::save_replay(&mut tx, a.workspace, &p, &operation, key, &req, &message).await?;
        message
    };
    let result = dao::get_message(&mut tx, a.workspace, id, message, false).await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn intake(pool: &PgPool, public: Uuid) -> Result<HelpdeskIntake, Error> {
    let mut tx = pool.begin().await?;
    dao::public_workspace(&mut tx, public).await?;
    Ok(HelpdeskIntake { accepting: true })
}
pub async fn open(
    pool: &PgPool,
    public: Uuid,
    code: &str,
    input: OpenTicket,
) -> Result<TicketReceipt, Error> {
    let hash = code_hash(code)?;
    text(&input.subject, 200)?;
    text(&input.requester_name, 120)?;
    text(&input.body, 20000)?;
    let mut tx = pool.begin().await?;
    let ws = dao::public_workspace(&mut tx, public).await?;
    modules::lock_workspace(&mut tx, ws).await?;
    dao::public_workspace(&mut tx, public).await?;
    let req = digest(&serde_json::to_string(&input).expect("serializes"));
    let id = if let Some((id, old)) = dao::existing(&mut tx, ws, &hash).await? {
        // A revoked or archived ticket cannot be recovered by replaying its creation.
        dao::customer(&mut tx, id, &hash).await?;
        if old != req {
            return Err(Error::Conflict(
                "Access code already used for different input",
            ));
        }
        id
    } else {
        let id = dao::open(&mut tx, ws, &hash, &req, &input).await?;
        dao::message(&mut tx, ws, id, "customer", &input.body, None).await?;
        dao::event(&mut tx, ws, id, "opened", None).await?;
        id
    };
    tx.commit().await?;
    Ok(TicketReceipt { id })
}
pub async fn customer_get(pool: &PgPool, id: Uuid, code: &str) -> Result<CustomerTicket, Error> {
    let mut tx = pool.begin().await?;
    Ok(dao::customer(&mut tx, id, &code_hash(code)?).await?.1)
}
pub async fn customer_messages(
    pool: &PgPool,
    id: Uuid,
    code: &str,
    q: MessageQuery,
) -> Result<TicketMessages, Error> {
    let mut tx = pool.begin().await?;
    let (ws, _) = dao::customer(&mut tx, id, &code_hash(code)?).await?;
    message_page(&mut tx, ws, id, true, q).await
}
pub async fn customer_reply(
    pool: &PgPool,
    id: Uuid,
    code: &str,
    key: &str,
    input: CustomerReply,
) -> Result<TicketMessage, Error> {
    let hash = code_hash(code)?;
    let mut tx = pool.begin().await?;
    let (ws, _) = dao::customer(&mut tx, id, &hash).await?;
    modules::lock_workspace(&mut tx, ws).await?;
    dao::customer(&mut tx, id, &hash).await?;
    if key.is_empty() || key.len() > 128 || !key.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(Error::Invalid(
            "Idempotency-Key must be 1–128 visible ASCII characters",
        ));
    }
    text(&input.body, 20000)?;
    let req = digest(&input.body);
    let message = if let Some((old, message)) = dao::retry(&mut tx, ws, id, key).await? {
        if old != req {
            return Err(Error::Conflict(
                "Retry key already used for different input",
            ));
        }
        message
    } else {
        let message = dao::message(&mut tx, ws, id, "customer", &input.body, None).await?;
        dao::touch(&mut tx, ws, id, true).await?;
        dao::event(&mut tx, ws, id, "customer_reply", None).await?;
        dao::save_retry(&mut tx, ws, id, key, &req, message).await?;
        message
    };
    let result = dao::get_message(&mut tx, ws, id, message, true).await?;
    tx.commit().await?;
    Ok(result)
}
/// Helpdesk-owned contact history, independently permission-checked by the caller's actor.
pub(crate) async fn history(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    family: &[Uuid],
    after: Option<&crate::history::Position>,
    only: Option<Uuid>,
    n: i64,
) -> Result<Vec<(crate::history::Position, TicketEvent)>, Error> {
    let p = match authority(db, a, "helpdesk.read").await {
        Ok(p) => p,
        Err(Error::Forbidden) => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    dao::history(db, a.workspace, &p, family, after, only, n).await
}
