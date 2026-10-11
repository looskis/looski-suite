// SPDX-License-Identifier: AGPL-3.0-only
//! Helpdesk-owned persistence. Public selects never project staff-only fields.
use super::*;
use sqlx::types::Json;

pub(super) async fn settings(
    db: &mut PgConnection,
    ws: Uuid,
) -> Result<Option<HelpdeskSettings>, Error> {
    Ok(sqlx::query_scalar::<_,Json<HelpdeskSettings>>("SELECT jsonb_build_object('public_key',public_key,'accepting',accepting,'version',version) FROM helpdesk.settings WHERE workspace_id=$1").bind(ws).fetch_optional(db).await?.map(|v|v.0))
}
pub(super) async fn configure(
    db: &mut PgConnection,
    ws: Uuid,
    accepting: bool,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO helpdesk.settings(workspace_id,accepting) VALUES($1,$2) ON CONFLICT(workspace_id) DO UPDATE SET accepting=$2,version=helpdesk.settings.version+1").bind(ws).bind(accepting).execute(db).await?;
    Ok(())
}
pub(super) async fn public_workspace(db: &mut PgConnection, key: Uuid) -> Result<Uuid, Error> {
    sqlx::query_scalar("SELECT s.workspace_id FROM helpdesk.settings s JOIN suite.workspaces w ON w.id=s.workspace_id JOIN suite.workspace_modules m ON m.workspace_id=w.id AND m.module_key='helpdesk' WHERE s.public_key=$1 AND s.accepting AND w.state='active' AND m.enabled").bind(key).fetch_optional(db).await?.ok_or(Error::NotFound)
}
pub(super) async fn ticket(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    p: &access::Access,
) -> Result<Ticket, Error> {
    Ok(sqlx::query_scalar::<_,Json<Ticket>>("SELECT jsonb_build_object('id',t.id,'subject',t.subject,'requester_name',t.requester_name,'status',t.status,'assignee_id',t.assignee_id,'contact_id',t.contact_id,'archived',t.archived,'customer_revoked',t.customer_revoked,'version',t.version,'created_at',t.created_at::text,'updated_at',t.updated_at::text) FROM helpdesk.tickets t WHERE t.workspace_id=$1 AND t.id=$2 AND ($3 OR t.assignee_id=$4)").bind(ws).bind(id).bind(p.workspace_wide).bind(p.membership_id).fetch_optional(db).await?.ok_or(Error::NotFound)?.0)
}
pub(super) async fn list(
    db: &mut PgConnection,
    ws: Uuid,
    p: &access::Access,
    q: &TicketQuery,
    family: &[Uuid],
    n: i64,
) -> Result<Vec<Ticket>, Error> {
    // Cursors compare values rather than probing a potentially hidden row.
    Ok(sqlx::query_scalar::<_,Json<Ticket>>("SELECT jsonb_build_object('id',t.id,'subject',t.subject,'requester_name',t.requester_name,'status',t.status,'assignee_id',t.assignee_id,'contact_id',t.contact_id,'archived',t.archived,'customer_revoked',t.customer_revoked,'version',t.version,'created_at',t.created_at::text,'updated_at',t.updated_at::text) FROM helpdesk.tickets t WHERE t.workspace_id=$1 AND ($2 OR t.assignee_id=$3) AND t.archived=$4 AND ($5::uuid IS NULL OR t.id>$5) AND ($6::text IS NULL OR t.status=$6) AND ($7 OR t.contact_id=ANY($8)) ORDER BY t.id LIMIT $9").bind(ws).bind(p.workspace_wide).bind(p.membership_id).bind(q.archived.unwrap_or(false)).bind(q.after).bind(&q.status).bind(q.contact_id.is_none()).bind(family).bind(n).fetch_all(db).await?.into_iter().map(|v|v.0).collect())
}
pub(super) async fn directory(
    db: &mut PgConnection,
    ws: Uuid,
) -> Result<Vec<HelpdeskMember>, Error> {
    Ok(sqlx::query_scalar::<_,Json<HelpdeskMember>>("SELECT jsonb_build_object('id',m.id,'display_name',u.display_name) FROM suite.memberships m JOIN suite.users u ON u.id=m.user_id WHERE m.workspace_id=$1 AND m.state='active' AND u.state='active' ORDER BY u.display_name,m.id LIMIT 500").bind(ws).fetch_all(db).await?.into_iter().map(|v|v.0).collect())
}
pub(super) async fn articles(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
) -> Result<Vec<Uuid>, Error> {
    Ok(sqlx::query_scalar("SELECT document_id FROM helpdesk.article_links WHERE workspace_id=$1 AND ticket_id=$2 ORDER BY document_id").bind(ws).bind(id).fetch_all(db).await?)
}
pub(super) async fn update(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    i: &UpdateTicket,
) -> Result<(), Error> {
    sqlx::query("UPDATE helpdesk.tickets SET status=$3,assignee_id=$4,archived=$5,customer_revoked=customer_revoked OR $6,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(ws).bind(id).bind(&i.status).bind(i.assignee_id).bind(i.archived).bind(i.revoke_customer_access).execute(db).await?;
    Ok(())
}
pub(super) async fn link(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    contact: Option<Uuid>,
    articles: &[Uuid],
) -> Result<(), Error> {
    sqlx::query("UPDATE helpdesk.tickets SET contact_id=$3,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(ws).bind(id).bind(contact).execute(&mut *db).await?;
    sqlx::query("DELETE FROM helpdesk.article_links WHERE workspace_id=$1 AND ticket_id=$2")
        .bind(ws)
        .bind(id)
        .execute(&mut *db)
        .await?;
    for doc in articles {
        sqlx::query("INSERT INTO helpdesk.article_links(workspace_id,ticket_id,document_id) VALUES($1,$2,$3)").bind(ws).bind(id).bind(doc).execute(&mut *db).await?;
    }
    Ok(())
}
pub(super) async fn existing(
    db: &mut PgConnection,
    ws: Uuid,
    hash: &str,
) -> Result<Option<(Uuid, String)>, Error> {
    Ok(sqlx::query_as("SELECT id,creation_hash FROM helpdesk.tickets WHERE workspace_id=$1 AND customer_key_hash=$2").bind(ws).bind(hash).fetch_optional(db).await?)
}
pub(super) async fn open(
    db: &mut PgConnection,
    ws: Uuid,
    hash: &str,
    req: &str,
    i: &OpenTicket,
) -> Result<Uuid, Error> {
    Ok(sqlx::query_scalar("INSERT INTO helpdesk.tickets(workspace_id,subject,requester_name,customer_key_hash,creation_hash) VALUES($1,$2,$3,$4,$5) RETURNING id").bind(ws).bind(&i.subject).bind(&i.requester_name).bind(hash).bind(req).fetch_one(db).await?)
}
pub(super) async fn customer(
    db: &mut PgConnection,
    id: Uuid,
    hash: &str,
) -> Result<(Uuid, CustomerTicket), Error> {
    let(ws,t)=sqlx::query_as::<_,(Uuid,Json<CustomerTicket>)>("SELECT t.workspace_id,jsonb_build_object('id',t.id,'subject',t.subject,'status',t.status) FROM helpdesk.tickets t JOIN suite.workspaces w ON w.id=t.workspace_id JOIN suite.workspace_modules m ON m.workspace_id=w.id AND m.module_key='helpdesk' WHERE t.id=$1 AND t.customer_key_hash=$2 AND NOT t.customer_revoked AND NOT t.archived AND w.state='active' AND m.enabled").bind(id).bind(hash).fetch_optional(db).await?.ok_or(Error::NotFound)?;
    Ok((ws, t.0))
}
pub(super) async fn message(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    kind: &str,
    body: &str,
    author: Option<Uuid>,
) -> Result<Uuid, Error> {
    Ok(sqlx::query_scalar("INSERT INTO helpdesk.messages(workspace_id,ticket_id,kind,body,author_membership_id,created_at) VALUES($1,$2,$3,$4,$5,clock_timestamp()) RETURNING id").bind(ws).bind(id).bind(kind).bind(body).bind(author).fetch_one(db).await?)
}
pub(super) async fn get_message(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    message: Uuid,
    public: bool,
) -> Result<TicketMessage, Error> {
    Ok(sqlx::query_scalar::<_,Json<TicketMessage>>("SELECT jsonb_build_object('id',m.id,'kind',m.kind,'body',m.body,'created_at',m.created_at::text) FROM helpdesk.messages m WHERE workspace_id=$1 AND ticket_id=$2 AND m.id=$3 AND (NOT $4 OR kind!='note')").bind(ws).bind(id).bind(message).bind(public).fetch_optional(db).await?.ok_or(Error::NotFound)?.0)
}
pub(super) async fn messages(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    public: bool,
    after: Option<Uuid>,
    n: i64,
) -> Result<Vec<TicketMessage>, Error> {
    if let Some(cursor) = after {
        get_message(db, ws, id, cursor, public).await?;
    }
    Ok(sqlx::query_scalar::<_,Json<TicketMessage>>("SELECT jsonb_build_object('id',m.id,'kind',m.kind,'body',m.body,'created_at',m.created_at::text) FROM helpdesk.messages m WHERE workspace_id=$1 AND ticket_id=$2 AND (NOT $3 OR kind!='note') AND ($4::uuid IS NULL OR (m.created_at,m.id)>(SELECT created_at,id FROM helpdesk.messages WHERE workspace_id=$1 AND ticket_id=$2 AND id=$4 AND (NOT $3 OR kind!='note'))) ORDER BY m.created_at,m.id LIMIT $5").bind(ws).bind(id).bind(public).bind(after).bind(n).fetch_all(db).await?.into_iter().map(|v|v.0).collect())
}
pub(super) async fn touch(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    reopen: bool,
) -> Result<(), Error> {
    sqlx::query("UPDATE helpdesk.tickets SET version=version+1,updated_at=now(),status=CASE WHEN $3 THEN 'open' ELSE status END WHERE workspace_id=$1 AND id=$2").bind(ws).bind(id).bind(reopen).execute(db).await?;
    Ok(())
}
pub(super) async fn event(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    action: &str,
    actor: Option<Uuid>,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO helpdesk.events(workspace_id,ticket_id,action,actor_user_id,created_at) VALUES($1,$2,$3,$4,clock_timestamp())").bind(ws).bind(id).bind(action).bind(actor).execute(db).await?;
    Ok(())
}
pub(super) async fn retry(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    key: &str,
) -> Result<Option<(String, Uuid)>, Error> {
    Ok(sqlx::query_as("SELECT request_hash,message_id FROM helpdesk.customer_retries WHERE workspace_id=$1 AND ticket_id=$2 AND retry_key=$3").bind(ws).bind(id).bind(key).fetch_optional(db).await?)
}
pub(super) async fn save_retry(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    key: &str,
    hash: &str,
    message: Uuid,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO helpdesk.customer_retries(workspace_id,ticket_id,retry_key,request_hash,message_id) VALUES($1,$2,$3,$4,$5)").bind(ws).bind(id).bind(key).bind(hash).bind(message).execute(db).await?;
    Ok(())
}
pub(super) async fn history(
    db: &mut PgConnection,
    ws: Uuid,
    p: &access::Access,
    family: &[Uuid],
    after: Option<&crate::history::Position>,
    only: Option<Uuid>,
    n: i64,
) -> Result<Vec<(crate::history::Position, TicketEvent)>, Error> {
    Ok(sqlx::query_as::<_,(Json<crate::history::Position>,Json<TicketEvent>)>("SELECT jsonb_build_object('id',e.id,'recorded_at',to_char(e.created_at AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"'),'source','ticket'),jsonb_build_object('ticket_id',t.id,'subject',t.subject,'action',e.action) FROM helpdesk.events e JOIN helpdesk.tickets t ON (t.workspace_id,t.id)=(e.workspace_id,e.ticket_id) WHERE t.workspace_id=$1 AND t.contact_id=ANY($2) AND ($3 OR t.assignee_id=$4) AND ($5::uuid IS NULL OR e.id=$5) AND ($6::text IS NULL OR (e.created_at,'ticket'::text,e.id)<($6::timestamptz,$7::text,$8::uuid)) ORDER BY e.created_at DESC,e.id DESC LIMIT $9").bind(ws).bind(family).bind(p.workspace_wide).bind(p.membership_id).bind(only).bind(after.map(|p|p.recorded_at.as_str())).bind(after.map(|p|p.source.as_str())).bind(after.map(|p|p.id)).bind(n).fetch_all(db).await?.into_iter().map(|(p,e)|(p.0,e.0)).collect())
}
