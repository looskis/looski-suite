// SPDX-License-Identifier: AGPL-3.0-only
//! Durable, independently authorized conversations shared by suite applications.
pub mod adapter;
mod config;
mod dao;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    contacts::references,
    jobs, modules, reliability,
};
use adapter::{Capabilities, ChannelAdapter, HttpAdapter};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Account {
    pub id: Uuid,
    pub connector_key: String,
    pub provider: String,
    pub installation_id: String,
    pub name: String,
    pub owner_membership_id: Uuid,
    pub shared: bool,
    pub enabled: bool,
    pub cursor: i64,
    pub sync_error: Option<String>,
    pub version: i64,
    pub capabilities: Capabilities,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConnectAccount {
    pub connector_key: String,
    pub name: String,
    #[serde(default)]
    pub shared: bool,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateAccount {
    pub expected_version: i64,
    pub name: String,
    pub enabled: bool,
    pub shared: bool,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Conversation {
    pub id: Uuid,
    pub account_id: Uuid,
    pub provider_thread_id: String,
    pub channel: String,
    pub subject: String,
    pub contact_id: Option<Uuid>,
    pub archived: bool,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub direction: String,
    pub body: String,
    pub sender: Option<String>,
    pub state: String,
    pub provider_message_id: Option<String>,
    pub created_at: String,
    pub job_id: Option<Uuid>,
    pub job_state: Option<String>,
    pub job_error: Option<String>,
    pub metadata: serde_json::Value,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct ConversationQuery {
    pub newest_first: Option<bool>,
    pub search: Option<String>,
    pub account_id: Option<Uuid>,
    pub after: Option<Uuid>,
    pub limit: Option<i64>,
    pub contact_id: Option<Uuid>,
    pub archived: Option<bool>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct MessageQuery {
    pub newest_first: Option<bool>,
    pub after: Option<Uuid>,
    pub limit: Option<i64>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Conversations {
    pub items: Vec<Conversation>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Messages {
    pub items: Vec<Message>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateConversation {
    pub expected_version: i64,
    pub contact_id: Option<Uuid>,
    pub archived: bool,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SendMessage {
    pub body: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct SyncResult {
    pub processed: usize,
    pub cursor: i64,
}
fn hash<T: Serialize>(value: &T) -> serde_json::Value {
    let bytes = serde_json::to_vec(value).expect("serializable request");
    serde_json::json!({"sha256":Sha256::digest(bytes).iter().map(|b|format!("{b:02x}")).collect::<String>()})
}
fn text(s: &str, max: usize) -> Result<(), Error> {
    if s.trim().is_empty()
        || s.chars().count() > max
        || s.chars()
            .any(|c| c.is_control() && !['\r', '\n', '\t'].contains(&c))
    {
        return Err(Error::Invalid(
            "Text is empty, too long or contains control characters",
        ));
    }
    Ok(())
}
fn limit(n: Option<i64>) -> Result<i64, Error> {
    let n = n.unwrap_or(50);
    if !(1..=100).contains(&n) {
        return Err(Error::Invalid("Limit must be 1–100"));
    }
    Ok(n)
}
async fn authority(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    action: &str,
) -> Result<access::Access, Error> {
    modules::require(db, a.workspace, "communications").await?;
    access::require(db, a.identity, a.workspace, action).await
}
fn visible(account: &Account, p: &access::Access) -> bool {
    account.owner_membership_id == p.membership_id || (account.shared && p.workspace_wide)
}
async fn account(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    id: Uuid,
    action: &str,
) -> Result<(access::Access, Account), Error> {
    let read = authority(db, a, "communications.read").await?;
    let ac = dao::account(db, a.workspace, id).await?;
    if !visible(&ac, &read) {
        return Err(Error::NotFound);
    }
    let p = authority(db, a, action).await?;
    if !visible(&ac, &p) {
        return Err(Error::Forbidden);
    }
    Ok((p, ac))
}
async fn conversation(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    id: Uuid,
    action: &str,
) -> Result<(access::Access, Account, Conversation), Error> {
    let c = dao::conversation(db, a.workspace, id).await?;
    let (p, ac) = account(db, a, c.account_id, action).await?;
    Ok((p, ac, c))
}
async fn present(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    mut c: Conversation,
) -> Result<Conversation, Error> {
    if let Some(id) = c.contact_id {
        c.contact_id = references::visible(db, a, references::Kind::Contact, &[id])
            .await?
            .first()
            .map(|r| r.canonical_id);
    }
    Ok(c)
}
fn adapter(workspace: Uuid, ac: &Account) -> Result<HttpAdapter, Error> {
    let c = config::load(&ac.connector_key, workspace)?;
    if c.provider != ac.provider || c.installation_id != ac.installation_id {
        return Err(Error::Conflict(
            "Connector identity changed; create a new connection",
        ));
    }
    HttpAdapter::new(c)
}
pub async fn accounts(pool: &PgPool, a: &ActorContext<'_>) -> Result<Vec<Account>, Error> {
    let mut tx = pool.begin().await?;
    let p = authority(&mut tx, a, "communications.read").await?;
    Ok(dao::accounts(&mut tx, a.workspace)
        .await?
        .into_iter()
        .filter(|ac| visible(ac, &p))
        .collect())
}
pub async fn connect(
    pool: &PgPool,
    a: &ActorContext<'_>,
    key: &str,
    input: ConnectAccount,
) -> Result<Account, Error> {
    text(&input.name, 120)?;
    text(&input.connector_key, 120)?;
    let mut tx = pool.begin().await?;
    modules::lock_workspace(&mut tx, a.workspace).await?;
    let p = authority(&mut tx, a, "communications.configure").await?;
    authority(&mut tx, a, "communications.read").await?;
    if !p.workspace_wide {
        return Err(Error::Forbidden);
    }
    let request = hash(&input);
    let id = if let Some(id) = reliability::replay::<Uuid>(
        &mut tx,
        a.workspace,
        &p,
        "communications.connect",
        key,
        &request,
    )
    .await?
    {
        id
    } else {
        let adapter = HttpAdapter::new(config::load(&input.connector_key, a.workspace)?)?;
        let id = dao::connect(&mut tx, a.workspace, &p, &input, &adapter).await?;
        reliability::audit(
            &mut tx,
            a,
            &p,
            "communications.account.connected",
            id,
            serde_json::json!({"provider":adapter.config.provider}),
        )
        .await?;
        reliability::save_replay(
            &mut tx,
            a.workspace,
            &p,
            "communications.connect",
            key,
            &request,
            &id,
        )
        .await?;
        id
    };
    let (_, result) = account(&mut tx, a, id, "communications.read").await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn update_account(
    pool: &PgPool,
    a: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: UpdateAccount,
) -> Result<Account, Error> {
    text(&input.name, 120)?;
    let mut tx = pool.begin().await?;
    modules::lock_workspace(&mut tx, a.workspace).await?;
    let (p, ac) = account(&mut tx, a, id, "communications.configure").await?;
    // A shared-account administrator may pause it, but only its owner can change sharing.
    if !p.workspace_wide || (input.shared != ac.shared && p.membership_id != ac.owner_membership_id)
    {
        return Err(Error::Forbidden);
    }
    let op = format!("communications.account.update:{id}");
    let request = hash(&input);
    if reliability::replay::<bool>(&mut tx, a.workspace, &p, &op, key, &request)
        .await?
        .is_none()
    {
        if ac.version != input.expected_version {
            return Err(Error::Conflict("Connection changed; reload"));
        }
        dao::update_account(&mut tx, a.workspace, id, &input).await?;
        reliability::audit(
            &mut tx,
            a,
            &p,
            "communications.account.updated",
            id,
            serde_json::json!({"enabled":input.enabled,"shared":input.shared}),
        )
        .await?;
        reliability::save_replay(&mut tx, a.workspace, &p, &op, key, &request, &true).await?;
    }
    let (_, result) = account(&mut tx, a, id, "communications.read").await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn list(
    pool: &PgPool,
    a: &ActorContext<'_>,
    q: ConversationQuery,
) -> Result<Conversations, Error> {
    let n = limit(q.limit)?;
    let mut tx = pool.begin().await?;
    let p = authority(&mut tx, a, "communications.read").await?;
    if let Some(id) = q.after {
        conversation(&mut tx, a, id, "communications.read").await?;
    }
    if q.search.as_ref().is_some_and(|s| s.chars().count() > 200) {
        return Err(Error::Invalid("Search must be at most 200 characters"));
    }
    let family = if let Some(id) = q.contact_id {
        references::contact_family(&mut tx, a, id).await?
    } else {
        Vec::new()
    };
    let mut items = dao::list(&mut tx, a.workspace, &p, &q, &family, n + 1).await?;
    let more = items.len() > n as usize;
    items.truncate(n as usize);
    let next_cursor = if more {
        items.last().map(|c| c.id)
    } else {
        None
    };
    let mut presented = Vec::new();
    for c in items {
        presented.push(present(&mut tx, a, c).await?);
    }
    Ok(Conversations {
        items: presented,
        next_cursor,
    })
}
pub async fn get(pool: &PgPool, a: &ActorContext<'_>, id: Uuid) -> Result<Conversation, Error> {
    let mut tx = pool.begin().await?;
    let (_, _, c) = conversation(&mut tx, a, id, "communications.read").await?;
    present(&mut tx, a, c).await
}
pub async fn messages(
    pool: &PgPool,
    a: &ActorContext<'_>,
    id: Uuid,
    q: MessageQuery,
) -> Result<Messages, Error> {
    let n = limit(q.limit)?;
    let mut tx = pool.begin().await?;
    conversation(&mut tx, a, id, "communications.read").await?;
    let mut items = dao::messages(
        &mut tx,
        a.workspace,
        id,
        q.after,
        q.newest_first.unwrap_or(false),
        n + 1,
    )
    .await?;
    let more = items.len() > n as usize;
    items.truncate(n as usize);
    let next_cursor = if more {
        items.last().map(|m| m.id)
    } else {
        None
    };
    Ok(Messages { items, next_cursor })
}
pub async fn update(
    pool: &PgPool,
    a: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: UpdateConversation,
) -> Result<Conversation, Error> {
    let mut tx = pool.begin().await?;
    modules::lock_workspace(&mut tx, a.workspace).await?;
    let (p, _, c) = conversation(&mut tx, a, id, "communications.manage").await?;
    let op = format!("communications.update:{id}");
    let request = hash(&input);
    if reliability::replay::<bool>(&mut tx, a.workspace, &p, &op, key, &request)
        .await?
        .is_none()
    {
        if c.version != input.expected_version {
            return Err(Error::Conflict("Conversation changed; reload"));
        }
        if let Some(old) = c.contact_id {
            references::require(&mut tx, a, references::Kind::Contact, old).await?;
        }
        let contact = if let Some(contact) = input.contact_id {
            Some(
                references::for_mutation(&mut tx, a, references::Kind::Contact, contact)
                    .await?
                    .canonical_id,
            )
        } else {
            None
        };
        dao::update(&mut tx, a.workspace, id, contact, input.archived).await?;
        reliability::audit(
            &mut tx,
            a,
            &p,
            "communications.conversation.updated",
            id,
            serde_json::json!({"archived":input.archived}),
        )
        .await?;
        reliability::save_replay(&mut tx, a.workspace, &p, &op, key, &request, &true).await?;
    }
    let c = dao::conversation(&mut tx, a.workspace, id).await?;
    let result = present(&mut tx, a, c).await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn send(
    pool: &PgPool,
    a: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: SendMessage,
) -> Result<Message, Error> {
    text(&input.body, 20000)?;
    let mut tx = pool.begin().await?;
    modules::lock_workspace(&mut tx, a.workspace).await?;
    let (p, ac, c) = conversation(&mut tx, a, id, "communications.send").await?;
    if !ac.enabled || c.archived {
        return Err(Error::Conflict(
            "Connection paused or conversation archived",
        ));
    }
    let connector = adapter(a.workspace, &ac)?;
    if !connector.capabilities().send_text
        || !connector.capabilities().channels.contains(&c.channel)
    {
        return Err(Error::Conflict(
            "Connection cannot send text on this channel",
        ));
    }
    let request = hash(&input);
    let op = format!("communications.send:{id}");
    let message = if let Some(m) =
        reliability::replay::<Uuid>(&mut tx, a.workspace, &p, &op, key, &request).await?
    {
        m
    } else {
        let m = Uuid::new_v4();
        let job = jobs::enqueue_recorded(
            &mut tx,
            a.workspace,
            p.user_id,
            p.membership_id,
            a.identity.issuer(),
            a.identity.subject(),
            &format!("communications.send:{m}"),
            "communications.send",
        )
        .await?;
        dao::outbound(&mut tx, a.workspace, &c, m, job, &input.body).await?;
        reliability::audit(
            &mut tx,
            a,
            &p,
            "communications.message.queued",
            m,
            serde_json::json!({"conversation_id":id}),
        )
        .await?;
        reliability::save_replay(&mut tx, a.workspace, &p, &op, key, &request, &m).await?;
        m
    };
    let result = dao::message(&mut tx, a.workspace, message).await?;
    tx.commit().await?;
    Ok(result)
}
pub(crate) async fn execute(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    job: Uuid,
) -> Result<(), Error> {
    let m = dao::job_message(db, a.workspace, job).await?;
    let (_, ac, c) = conversation(db, a, m.conversation_id, "communications.send").await?;
    if !ac.enabled || c.archived {
        return Err(Error::Conflict(
            "Connection paused or conversation archived",
        ));
    }
    if m.state != "queued" {
        return Ok(());
    }
    let adapter = adapter(a.workspace, &ac)?;
    if !adapter.capabilities().send_text || !adapter.capabilities().channels.contains(&c.channel) {
        return Err(Error::Conflict("Channel cannot send text"));
    }
    let accepted = adapter
        .send(&adapter::Outbound {
            version: 1,
            client_ref: m.id.to_string(),
            thread_id: c.provider_thread_id,
            channel: c.channel,
            body: m.body,
            subject: c.subject,
        })
        .await?;
    dao::accepted(db, a.workspace, m.id, &accepted.message_id).await
}
/// Webhooks never advance the polling watermark: out-of-order delivery cannot skip gaps.
pub async fn webhook(pool: &PgPool, id: Uuid, signature: &str, body: &[u8]) -> Result<(), Error> {
    if body.len() > 256_000 {
        return Err(Error::Invalid("Event too large"));
    }
    let mut tx = pool.begin().await?;
    let ws = dao::workspace(&mut tx, id).await?;

    modules::require(&mut tx, ws, "communications").await?;
    let ac = dao::account(&mut tx, ws, id).await?;
    if !ac.enabled {
        return Err(Error::Forbidden);
    }
    dao::owner_active(&mut tx, ws, ac.owner_membership_id).await?;
    let adapter = adapter(ws, &ac)?;
    verify_signature(adapter.config.webhook_secret.as_bytes(), signature, body)?;
    let event = adapter.decode(body)?;
    modules::lock_workspace(&mut tx, ws).await?;
    modules::require(&mut tx, ws, "communications").await?;
    let ac = dao::account(&mut tx, ws, id).await?;
    if !ac.enabled {
        return Err(Error::Forbidden);
    }
    dao::owner_active(&mut tx, ws, ac.owner_membership_id).await?;
    dao::ingest(&mut tx, ws, &ac, &adapter.config, &event).await?;
    tx.commit().await?;
    Ok(())
}
fn verify_signature(secret: &[u8], signature: &str, body: &[u8]) -> Result<(), Error> {
    use hmac::{Hmac, KeyInit, Mac};
    let hex = signature.strip_prefix("sha256=").unwrap_or(signature);
    if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::Forbidden);
    }
    let bytes: Vec<u8> = (0..64)
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    let mut mac =
        <Hmac<Sha256> as KeyInit>::new_from_slice(secret).map_err(|_| Error::Forbidden)?;
    mac.update(body);
    mac.verify_slice(&bytes).map_err(|_| Error::Forbidden)
}
pub async fn sync(pool: &PgPool, a: &ActorContext<'_>, id: Uuid) -> Result<SyncResult, Error> {
    let mut tx = pool.begin().await?;
    let (p, _) = account(&mut tx, a, id, "communications.configure").await?;
    if !p.workspace_wide {
        return Err(Error::Forbidden);
    }
    drop(tx);
    sync_account(pool, a.workspace, id, Some(a)).await
}
async fn sync_account(
    pool: &PgPool,
    ws: Uuid,
    id: Uuid,
    actor: Option<&ActorContext<'_>>,
) -> Result<SyncResult, Error> {
    let mut tx = pool.begin().await?;
    modules::lock_workspace(&mut tx, ws).await?;
    modules::require(&mut tx, ws, "communications").await?;
    let ac = dao::account(&mut tx, ws, id).await?;
    if !ac.enabled {
        return Err(Error::Forbidden);
    }
    if let Some(a) = actor {
        let (p, _) = account(&mut tx, a, id, "communications.configure").await?;
        if !p.workspace_wide {
            return Err(Error::Forbidden);
        }
    }
    dao::owner_active(&mut tx, ws, ac.owner_membership_id).await?;
    let connector = adapter(ws, &ac)?;
    let events = connector.poll(ac.cursor).await?;
    let mut cursor = ac.cursor;
    for event in &events {
        dao::ingest(&mut tx, ws, &ac, &connector.config, event).await?;
        cursor = event.id;
    }
    dao::cursor(&mut tx, ws, id, cursor).await?;
    tx.commit().await?;
    Ok(SyncResult {
        processed: events.len(),
        cursor,
    })
}
/// One account per tick bounds provider outage impact. The worker calls this every 10 seconds.
pub async fn poll_once(pool: &PgPool) -> Result<(), Error> {
    let mut db = pool.acquire().await?;
    let candidate = dao::next_sync(&mut db).await?;
    drop(db);
    if let Some((ws, id)) = candidate
        && sync_account(pool, ws, id, None).await.is_err()
    {
        let mut db = pool.acquire().await?;
        dao::sync_error(&mut db, ws, id).await?;
    }
    Ok(())
}
pub(crate) async fn get_job_authority(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    id: Uuid,
) -> Result<(), Error> {
    let m = dao::job_message(db, a.workspace, id).await?;
    conversation(db, a, m.conversation_id, "communications.read").await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn webhook_hmac_checks_exact_bytes() {
        // RFC 4231 test case 1, HMAC-SHA256.
        let secret = [0x0b; 20];
        let signature = "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7";
        assert!(verify_signature(&secret, signature, b"Hi There").is_ok());
        assert!(verify_signature(&secret, signature, b"Hi there").is_err());
        assert!(verify_signature(&secret, &"é".repeat(32), b"Hi There").is_err());
        assert!(verify_signature(&secret, "bad", b"Hi There").is_err());
    }
}

#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TimelineMessage {
    pub conversation_id: Uuid,
    pub message_id: Uuid,
    pub subject: String,
    pub channel: String,
    pub direction: String,
    pub body: String,
    pub state: String,
}
pub(crate) async fn history(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    family: &[Uuid],
    after: Option<&crate::history::Position>,
    only: Option<Uuid>,
    n: i64,
) -> Result<Vec<(crate::history::Position, TimelineMessage)>, Error> {
    let p = match authority(db, a, "communications.read").await {
        Ok(p) => p,
        Err(Error::Forbidden) => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    dao::history(db, a.workspace, &p, family, after, only, n).await
}
