// SPDX-License-Identifier: AGPL-3.0-only
use super::*;
use sqlx::types::Json;
pub(super) async fn account(db: &mut PgConnection, ws: Uuid, id: Uuid) -> Result<Account, Error> {
    Ok(sqlx::query_scalar::<_,Json<Account>>("SELECT to_jsonb(a)-'workspace_id'-'sync_at' FROM communications.accounts a WHERE workspace_id=$1 AND id=$2").bind(ws).bind(id).fetch_optional(db).await?.ok_or(Error::NotFound)?.0)
}
pub(super) async fn accounts(db: &mut PgConnection, ws: Uuid) -> Result<Vec<Account>, Error> {
    Ok(sqlx::query_scalar::<_,Json<Account>>("SELECT to_jsonb(a)-'workspace_id'-'sync_at' FROM communications.accounts a WHERE workspace_id=$1 ORDER BY name,id LIMIT 100").bind(ws).fetch_all(db).await?.into_iter().map(|v|v.0).collect())
}
pub(super) async fn workspace(db: &mut PgConnection, id: Uuid) -> Result<Uuid, Error> {
    sqlx::query_scalar("SELECT workspace_id FROM communications.accounts WHERE id=$1")
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or(Error::NotFound)
}
pub(super) async fn connect(
    db: &mut PgConnection,
    ws: Uuid,
    p: &access::Access,
    i: &ConnectAccount,
    adapter: &HttpAdapter,
) -> Result<Uuid, Error> {
    if sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM communications.accounts WHERE connector_key=$1)",
    )
    .bind(&i.connector_key)
    .fetch_one(&mut *db)
    .await?
    {
        return Err(Error::Conflict("Connector is already connected"));
    }
    Ok(sqlx::query_scalar("INSERT INTO communications.accounts(workspace_id,connector_key,name,owner_membership_id,shared,provider,installation_id,capabilities) VALUES($1,$2,$3,$4,$5,$6,$7,$8) RETURNING id").bind(ws).bind(&i.connector_key).bind(&i.name).bind(p.membership_id).bind(i.shared).bind(&adapter.config.provider).bind(&adapter.config.installation_id).bind(Json(adapter.capabilities())).fetch_one(db).await?)
}
pub(super) async fn update_account(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    i: &UpdateAccount,
) -> Result<(), Error> {
    sqlx::query("UPDATE communications.accounts SET name=$3,enabled=$4,shared=$5,version=version+1 WHERE workspace_id=$1 AND id=$2").bind(ws).bind(id).bind(&i.name).bind(i.enabled).bind(i.shared).execute(db).await?;
    Ok(())
}
pub(super) async fn conversation(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
) -> Result<Conversation, Error> {
    Ok(sqlx::query_scalar::<_,Json<Conversation>>("SELECT to_jsonb(c)-'workspace_id' FROM communications.conversations c WHERE workspace_id=$1 AND id=$2").bind(ws).bind(id).fetch_optional(db).await?.ok_or(Error::NotFound)?.0)
}
pub(super) async fn list(
    db: &mut PgConnection,
    ws: Uuid,
    p: &access::Access,
    q: &ConversationQuery,
    family: &[Uuid],
    n: i64,
) -> Result<Vec<Conversation>, Error> {
    let cursor = if q.newest_first.unwrap_or(false) {
        "(c.updated_at,c.id)<(SELECT updated_at,id FROM communications.conversations WHERE workspace_id=$1 AND id=$5)"
    } else {
        "c.id>$5"
    };
    let order = if q.newest_first.unwrap_or(false) {
        "c.updated_at DESC,c.id DESC"
    } else {
        "c.id"
    };
    let sql = format!(
        "SELECT to_jsonb(c)-'workspace_id' FROM communications.conversations c JOIN communications.accounts a ON a.id=c.account_id WHERE c.workspace_id=$1 AND (a.owner_membership_id=$2 OR (a.shared AND $3)) AND c.archived=$4 AND ($5::uuid IS NULL OR {cursor}) AND ($6 OR c.contact_id=ANY($7)) AND ($9::text IS NULL OR strpos(lower(c.subject),lower($9))>0) AND ($10::uuid IS NULL OR c.account_id=$10) ORDER BY {order} LIMIT $8"
    );
    Ok(
        sqlx::query_scalar::<_, Json<Conversation>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(p.membership_id)
            .bind(p.workspace_wide)
            .bind(q.archived.unwrap_or(false))
            .bind(q.after)
            .bind(q.contact_id.is_none())
            .bind(family)
            .bind(n)
            .bind(&q.search)
            .bind(q.account_id)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|v| v.0)
            .collect(),
    )
}
pub(super) async fn message(db: &mut PgConnection, ws: Uuid, id: Uuid) -> Result<Message, Error> {
    Ok(sqlx::query_scalar::<_,Json<Message>>("SELECT (to_jsonb(m)-'workspace_id'-'account_id'-'status_cursor'-'provider_dispatch_id') || jsonb_build_object('job_state',j.state,'job_error',j.error_code) FROM communications.messages m LEFT JOIN suite.jobs j ON j.id=m.job_id WHERE m.workspace_id=$1 AND m.id=$2").bind(ws).bind(id).fetch_optional(db).await?.ok_or(Error::NotFound)?.0)
}
pub(super) async fn messages(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    after: Option<Uuid>,
    newest_first: bool,
    n: i64,
) -> Result<Vec<Message>, Error> {
    if let Some(after) = after {
        let m = message(db, ws, after).await?;
        if m.conversation_id != id {
            return Err(Error::NotFound);
        }
    }
    let comparison = if newest_first { "<" } else { ">" };
    let order = if newest_first {
        "m.created_at DESC,m.id DESC"
    } else {
        "m.created_at,m.id"
    };
    let sql = format!(
        "SELECT (to_jsonb(m)-'workspace_id'-'account_id'-'status_cursor'-'provider_dispatch_id') || jsonb_build_object('job_state',j.state,'job_error',j.error_code) FROM communications.messages m LEFT JOIN suite.jobs j ON j.id=m.job_id WHERE m.workspace_id=$1 AND m.conversation_id=$2 AND ($3::uuid IS NULL OR (m.created_at,m.id){comparison}(SELECT created_at,id FROM communications.messages WHERE workspace_id=$1 AND conversation_id=$2 AND id=$3)) ORDER BY {order} LIMIT $4"
    );
    Ok(
        sqlx::query_scalar::<_, Json<Message>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(id)
            .bind(after)
            .bind(n)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|v| v.0)
            .collect(),
    )
}
pub(super) async fn update(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    contact: Option<Uuid>,
    archived: bool,
) -> Result<(), Error> {
    sqlx::query("UPDATE communications.conversations SET contact_id=$3,archived=$4,version=version+1,updated_at=clock_timestamp() WHERE workspace_id=$1 AND id=$2").bind(ws).bind(id).bind(contact).bind(archived).execute(db).await?;
    Ok(())
}
pub(super) async fn outbound(
    db: &mut PgConnection,
    ws: Uuid,
    c: &Conversation,
    id: Uuid,
    job: Uuid,
    body: &str,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO communications.messages(id,workspace_id,account_id,conversation_id,direction,body,state,job_id) VALUES($1,$2,$3,$4,'outbound',$5,'queued',$6)").bind(id).bind(ws).bind(c.account_id).bind(c.id).bind(body).bind(job).execute(&mut *db).await?;
    touch(db, ws, c.id).await
}
async fn touch(db: &mut PgConnection, ws: Uuid, id: Uuid) -> Result<(), Error> {
    sqlx::query("UPDATE communications.conversations SET updated_at=clock_timestamp() WHERE workspace_id=$1 AND id=$2").bind(ws).bind(id).execute(db).await?;
    Ok(())
}
pub(super) async fn job_message(
    db: &mut PgConnection,
    ws: Uuid,
    job: Uuid,
) -> Result<Message, Error> {
    Ok(sqlx::query_scalar::<_,Json<Message>>("SELECT (to_jsonb(m)-'workspace_id'-'account_id'-'status_cursor'-'provider_dispatch_id') || jsonb_build_object('job_state',j.state,'job_error',j.error_code) FROM communications.messages m LEFT JOIN suite.jobs j ON j.id=m.job_id WHERE m.workspace_id=$1 AND m.job_id=$2").bind(ws).bind(job).fetch_optional(db).await?.ok_or(Error::NotFound)?.0)
}
pub(super) async fn accepted(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    dispatch: &str,
) -> Result<(), Error> {
    sqlx::query("UPDATE communications.messages SET state=CASE WHEN state='queued' THEN 'accepted' ELSE state END,provider_dispatch_id=$3 WHERE workspace_id=$1 AND id=$2").bind(ws).bind(id).bind(dispatch).execute(db).await?;
    Ok(())
}
pub(super) async fn ingest(
    db: &mut PgConnection,
    ws: Uuid,
    ac: &Account,
    config: &config::Connector,
    e: &adapter::Event,
) -> Result<(), Error> {
    // Allowlisting happens before persistence, so excluded personal traffic is not retained.
    if e.kind == "message.received"
        && !config.destinations.is_empty()
        && !e
            .destination
            .as_ref()
            .is_some_and(|d| config.destinations.contains(d))
    {
        return Ok(());
    }
    let fingerprint = hash(e)["sha256"].as_str().unwrap().to_string();
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT fingerprint FROM communications.events WHERE account_id=$1 AND cursor=$2",
    )
    .bind(ac.id)
    .bind(e.id)
    .fetch_optional(&mut *db)
    .await?;
    if let Some(existing) = existing {
        if existing != fingerprint {
            return Err(Error::Conflict(
                "Provider reused an event cursor with different content",
            ));
        }
        return Ok(());
    }
    sqlx::query("INSERT INTO communications.events(account_id,cursor,fingerprint,payload) VALUES($1,$2,$3,$4)").bind(ac.id).bind(e.id).bind(fingerprint).bind(Json(e)).execute(&mut *db).await?;
    if e.kind == "message.received" {
        let thread = e
            .thread_id
            .as_ref()
            .ok_or(Error::Invalid("Missing thread"))?;
        let channel = e
            .channel
            .as_ref()
            .ok_or(Error::Invalid("Missing channel"))?;
        let cid:Uuid=sqlx::query_scalar("INSERT INTO communications.conversations(workspace_id,account_id,provider_thread_id,channel,subject) VALUES($1,$2,$3,$4,$5) ON CONFLICT(account_id,provider_thread_id) DO UPDATE SET updated_at=clock_timestamp() RETURNING id").bind(ws).bind(ac.id).bind(thread).bind(channel).bind(e.subject.as_deref().unwrap_or("")).fetch_one(&mut *db).await?;
        sqlx::query("INSERT INTO communications.messages(workspace_id,account_id,conversation_id,direction,body,sender,provider_message_id,state,metadata) VALUES($1,$2,$3,'inbound',$4,$5,$6,'received',$7) ON CONFLICT(account_id,provider_message_id) DO NOTHING").bind(ws).bind(ac.id).bind(cid).bind(e.body.as_deref().unwrap_or("")).bind(&e.sender).bind(&e.message_id).bind(Json(&e.metadata)).execute(&mut *db).await?;
    } else if let Some(id) = e.client_ref.as_ref().and_then(|r| Uuid::parse_str(r).ok()) {
        // Only an already-created outbound message in this exact account can receive receipts.
        // Cursor fencing stops delayed queued/sent receipts regressing a delivered/read message.
        if let Some(state) = &e.state {
            sqlx::query("UPDATE communications.messages SET state=CASE WHEN state='read' THEN state WHEN state='delivered' AND $4!='read' THEN state ELSE $4 END,status_cursor=$5,provider_message_id=CASE WHEN $4 IN ('sent','delivered','read') THEN $6 ELSE provider_message_id END WHERE workspace_id=$1 AND account_id=$2 AND id=$3 AND direction='outbound' AND status_cursor<$5").bind(ws).bind(ac.id).bind(id).bind(state).bind(e.id).bind(&e.message_id).execute(&mut *db).await?;
        }
    }
    Ok(())
}
pub(super) async fn cursor(
    db: &mut PgConnection,
    ws: Uuid,
    id: Uuid,
    cursor: i64,
) -> Result<(), Error> {
    sqlx::query("UPDATE communications.accounts SET cursor=$3,sync_error=NULL,sync_at=clock_timestamp() WHERE workspace_id=$1 AND id=$2").bind(ws).bind(id).bind(cursor).execute(db).await?;
    Ok(())
}
pub(super) async fn next_sync(db: &mut PgConnection) -> Result<Option<(Uuid, Uuid)>, Error> {
    Ok(sqlx::query_as("UPDATE communications.accounts SET sync_at=clock_timestamp() WHERE id=(SELECT a.id FROM communications.accounts a JOIN suite.workspaces w ON w.id=a.workspace_id JOIN suite.workspace_modules m ON m.workspace_id=w.id AND m.module_key='communications' WHERE a.enabled AND w.state='active' AND m.enabled AND (a.sync_at IS NULL OR a.sync_at<now()-interval '10 seconds') ORDER BY a.sync_at NULLS FIRST,a.id LIMIT 1 FOR UPDATE OF a SKIP LOCKED) RETURNING workspace_id,id").fetch_optional(db).await?)
}
pub(super) async fn sync_error(db: &mut PgConnection, ws: Uuid, id: Uuid) -> Result<(), Error> {
    sqlx::query("UPDATE communications.accounts SET sync_error='connector_unavailable' WHERE workspace_id=$1 AND id=$2").bind(ws).bind(id).execute(db).await?;
    Ok(())
}
pub(super) async fn owner_active(db: &mut PgConnection, ws: Uuid, id: Uuid) -> Result<(), Error> {
    let allowed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM suite.memberships m JOIN suite.workspaces w ON w.id=m.workspace_id JOIN suite.users u ON u.id=m.user_id JOIN suite.membership_roles mr ON mr.workspace_id=m.workspace_id AND mr.membership_id=m.id JOIN suite.roles r ON r.workspace_id=mr.workspace_id AND r.id=mr.role_id JOIN suite.role_permissions p ON p.workspace_id=r.workspace_id AND p.role_id=r.id WHERE m.workspace_id=$1 AND m.id=$2 AND w.state='active' AND m.state='active' AND u.state='active' AND r.archived_at IS NULL AND p.permission_key='communications.read' AND p.record_scope IN ('workspace','assigned'))").bind(ws).bind(id).fetch_one(db).await?;
    if !allowed {
        return Err(Error::Forbidden);
    }
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
) -> Result<Vec<(crate::history::Position, TimelineMessage)>, Error> {
    Ok(sqlx::query_as::<_,(Json<crate::history::Position>,Json<TimelineMessage>)>("SELECT jsonb_build_object('id',m.id,'recorded_at',to_char(m.created_at AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"'),'source','communication'),jsonb_build_object('conversation_id',c.id,'message_id',m.id,'subject',c.subject,'channel',c.channel,'direction',m.direction,'body',m.body,'state',m.state) FROM communications.messages m JOIN communications.conversations c ON c.workspace_id=m.workspace_id AND c.id=m.conversation_id JOIN communications.accounts a ON a.workspace_id=c.workspace_id AND a.id=c.account_id WHERE c.workspace_id=$1 AND c.contact_id=ANY($2) AND (a.owner_membership_id=$4 OR (a.shared AND $3)) AND ($5::uuid IS NULL OR m.id=$5) AND ($6::text IS NULL OR (m.created_at,'communication'::text,m.id)<($6::timestamptz,$7::text,$8::uuid)) ORDER BY m.created_at DESC,m.id DESC LIMIT $9").bind(ws).bind(family).bind(p.workspace_wide).bind(p.membership_id).bind(only).bind(after.map(|p|p.recorded_at.as_str())).bind(after.map(|p|p.source.as_str())).bind(after.map(|p|p.id)).bind(n).fetch_all(db).await?.into_iter().map(|(p,e)|(p.0,e.0)).collect())
}
