// SPDX-License-Identifier: AGPL-3.0-only
//! Consent-gated first-party attribution. No public history/identity reads exist.
mod matching;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    modules, reliability,
};
use matching::Signals;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool, types::Json};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
fn digest(value: &str) -> String {
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn public_key() -> Result<String, Error> {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).map_err(|_| Error::Unavailable)?;
    Ok(URL_SAFE_NO_PAD.encode(b))
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Site {
    pub id: Uuid,
    pub name: String,
    pub public_key: String,
    pub origins: Vec<String>,
    pub fingerprint_enabled: bool,
    pub similarity_enabled: bool,
    pub threshold: f64,
    pub retention_days: i32,
    pub enabled: bool,
    pub version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveSite {
    pub id: Option<Uuid>,
    pub name: String,
    pub origins: Vec<String>,
    #[serde(default)]
    pub fingerprint_enabled: bool,
    #[serde(default)]
    pub similarity_enabled: bool,
    pub threshold: f64,
    pub retention_days: i32,
    pub enabled: bool,
    pub expected_version: Option<i64>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Collect {
    pub event_id: Uuid,
    pub token: String,
    pub consent: bool,
    pub page: String,
    pub referrer_host: Option<String>,
    #[serde(default)]
    pub components: Signals,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Withdraw {
    pub token: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct PublicSite {
    pub fingerprint_enabled: bool,
    pub retention_days: i32,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Receipt {
    pub accepted: bool,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct Query {
    pub after: Option<Uuid>,
    pub site_id: Option<Uuid>,
    pub contact_id: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Visitor {
    pub id: Uuid,
    pub site_id: Uuid,
    pub contact_id: Option<Uuid>,
    pub evidence: Option<String>,
    pub first_seen: String,
    pub last_seen: String,
    pub expires_at: String,
    pub version: i64,
    pub views: i64,
    pub suggestions: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Page {
    pub items: Vec<Visitor>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct VisitEvent {
    pub id: Uuid,
    pub page: String,
    pub referrer_host: Option<String>,
    pub created_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Candidate {
    pub candidate_id: Uuid,
    pub score: f64,
    pub reason: String,
    pub state: String,
    pub version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Detail {
    pub visitor: Visitor,
    pub components: Signals,
    pub events: Vec<VisitEvent>,
    pub candidates: Vec<Candidate>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Resolve {
    pub site_key: String,
    pub token: String,
    pub contact_id: Uuid,
    pub evidence: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Review {
    pub candidate_id: Uuid,
    pub expected_version: i64,
    pub state: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Link {
    pub contact_id: Option<Uuid>,
    pub expected_version: i64,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct EventsPage {
    pub items: Vec<VisitEvent>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct EventsQuery {
    pub after: Option<Uuid>,
}
const SITE: &str = "SELECT jsonb_build_object('id',id,'name',name,'public_key',public_key,'origins',origins,'fingerprint_enabled',fingerprint_enabled,'similarity_enabled',similarity_enabled,'threshold',threshold,'retention_days',retention_days,'enabled',enabled,'version',version) FROM visitors.sites";
const VISITOR: &str = "jsonb_build_object('id',v.id,'site_id',v.site_id,'contact_id',v.contact_id,'evidence',v.evidence,'first_seen',v.first_seen::text,'last_seen',v.last_seen::text,'expires_at',v.expires_at::text,'version',v.version,'views',(SELECT count(*) FROM visitors.events e WHERE e.visitor_id=v.id),'suggestions',(SELECT count(*) FROM visitors.matches m WHERE m.visitor_id=v.id AND m.state='suggested'))";
async fn authority(
    db: &mut PgConnection,
    a: &ActorContext<'_>,
    p: &str,
) -> Result<access::Access, Error> {
    let grant = access::require(db, a.identity, a.workspace, p).await?;
    if !grant.workspace_wide {
        return Err(Error::Forbidden);
    }
    modules::require(db, a.workspace, "visitors").await?;
    Ok(grant)
}
async fn contacts_authority(db: &mut PgConnection, a: &ActorContext<'_>) -> Result<(), Error> {
    if !access::require(db, a.identity, a.workspace, "contacts.read")
        .await?
        .workspace_wide
    {
        return Err(Error::Forbidden);
    }
    Ok(())
}
pub async fn sites(pool: &PgPool, a: &ActorContext<'_>) -> Result<Vec<Site>, Error> {
    let mut tx = pool.begin().await?;
    authority(&mut tx, a, "visitors.read").await?;
    let sql = format!("{SITE} WHERE workspace_id=$1 ORDER BY name");
    Ok(
        sqlx::query_scalar::<_, Json<Site>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(a.workspace)
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .map(|x| x.0)
            .collect(),
    )
}
pub async fn save_site(
    pool: &PgPool,
    a: &ActorContext<'_>,
    key: &str,
    input: SaveSite,
) -> Result<Site, Error> {
    if input.name.trim().is_empty()
        || input.name.len() > 120
        || input.origins.is_empty()
        || input.origins.len() > 20
        || !(7..=180).contains(&input.retention_days)
        || !(0.8..=1.).contains(&input.threshold)
        || input.similarity_enabled && !input.fingerprint_enabled
    {
        return Err(Error::Invalid(
            "Check site name, origins, retention and matching settings",
        ));
    }
    for origin in &input.origins {
        let u = reqwest::Url::parse(origin).map_err(|_| Error::Invalid("Invalid site origin"))?;
        if u.origin().ascii_serialization() != *origin
            || !(u.scheme() == "https"
                || u.scheme() == "http" && matches!(u.host_str(), Some("localhost" | "127.0.0.1")))
        {
            return Err(Error::Invalid(
                "Origins must be exact HTTPS origins (HTTP only on localhost)",
            ));
        }
    }
    let mut tx = pool.begin().await?;
    modules::lock_workspace(&mut tx, a.workspace).await?;
    let grant = authority(&mut tx, a, "visitors.configure").await?;
    let request = serde_json::to_value(&input).expect("serializes");
    let op = "visitors.site.save";
    if let Some(saved) =
        reliability::replay(&mut tx, a.workspace, &grant, op, key, &request).await?
    {
        return Ok(saved);
    }
    let id = if let Some(id) = input.id {
        let changed=sqlx::query("UPDATE visitors.sites SET name=$3,origins=$4,fingerprint_enabled=$5,similarity_enabled=$6,threshold=$7,retention_days=$8,enabled=$9,version=version+1 WHERE workspace_id=$1 AND id=$2 AND version=$10").bind(a.workspace).bind(id).bind(input.name.trim()).bind(Json(&input.origins)).bind(input.fingerprint_enabled).bind(input.similarity_enabled).bind(input.threshold).bind(input.retention_days).bind(input.enabled).bind(input.expected_version).execute(&mut *tx).await?;
        if changed.rows_affected() != 1 {
            return Err(Error::Conflict("Site changed or is unavailable; reload"));
        }
        // Turning off fingerprinting removes captured signals and all suggestions.
        if !input.fingerprint_enabled {
            sqlx::query("UPDATE visitors.visitors SET components='{}',fingerprint=NULL WHERE workspace_id=$1 AND site_id=$2").bind(a.workspace).bind(id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM visitors.matches WHERE workspace_id=$1 AND visitor_id IN(SELECT id FROM visitors.visitors WHERE site_id=$2)").bind(a.workspace).bind(id).execute(&mut *tx).await?;
        }
        sqlx::query("UPDATE visitors.visitors SET expires_at=LEAST(expires_at,last_seen+$3*interval '1 day') WHERE workspace_id=$1 AND site_id=$2").bind(a.workspace).bind(id).bind(input.retention_days).execute(&mut *tx).await?;
        id
    } else {
        if input.expected_version.is_some() {
            return Err(Error::Invalid("New sites do not have a version"));
        }
        sqlx::query_scalar::<_,Uuid>("INSERT INTO visitors.sites(workspace_id,name,public_key,origins,fingerprint_enabled,similarity_enabled,threshold,retention_days,enabled) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING id").bind(a.workspace).bind(input.name.trim()).bind(public_key()?).bind(Json(&input.origins)).bind(input.fingerprint_enabled).bind(input.similarity_enabled).bind(input.threshold).bind(input.retention_days).bind(input.enabled).fetch_one(&mut *tx).await?
    };
    let sql = format!("{SITE} WHERE workspace_id=$1 AND id=$2");
    let result = sqlx::query_scalar::<_, Json<Site>>(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(a.workspace)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?
        .0;
    reliability::audit(&mut tx,a,&grant,"visitors.site_saved",id,json!({"fingerprint_enabled":input.fingerprint_enabled,"similarity_enabled":input.similarity_enabled,"retention_days":input.retention_days})).await?;
    reliability::save_replay(&mut tx, a.workspace, &grant, op, key, &request, &result).await?;
    tx.commit().await?;
    Ok(result)
}
async fn public_site(
    db: &mut PgConnection,
    key: &str,
    origin: &str,
    lock: bool,
) -> Result<(Uuid, Site), Error> {
    if key.len() != 22 {
        return Err(Error::NotFound);
    }
    let sql = format!(
        "{SITE} WHERE public_key=$1 {}",
        if lock { "FOR UPDATE" } else { "" }
    );
    let site = sqlx::query_scalar::<_, Json<Site>>(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(key)
        .fetch_optional(&mut *db)
        .await?
        .ok_or(Error::NotFound)?
        .0;
    let ws=sqlx::query_scalar::<_,Uuid>("SELECT s.workspace_id FROM visitors.sites s JOIN suite.workspaces w ON w.id=s.workspace_id WHERE s.id=$1 AND w.state='active'").bind(site.id).fetch_optional(&mut *db).await?.ok_or(Error::NotFound)?;
    if !site.enabled
        || !site.origins.iter().any(|o| o == origin)
        || !modules::enabled(db, ws, "visitors").await?
    {
        return Err(Error::NotFound);
    }
    Ok((ws, site))
}
fn token_hash(token: &str) -> Result<String, Error> {
    if token.len() != 64
        || !token
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(Error::Invalid("Invalid visitor token"));
    }
    Ok(digest(token))
}
pub async fn configuration(pool: &PgPool, key: &str, origin: &str) -> Result<PublicSite, Error> {
    let mut tx = pool.begin().await?;
    let (_, site) = public_site(&mut tx, key, origin, false).await?;
    Ok(PublicSite {
        fingerprint_enabled: site.fingerprint_enabled,
        retention_days: site.retention_days,
    })
}
pub async fn collect(
    pool: &PgPool,
    key: &str,
    origin: &str,
    input: Collect,
) -> Result<Receipt, Error> {
    if !input.consent {
        return Err(Error::Invalid("Recognition consent is required"));
    }
    let hash = token_hash(&input.token)?;
    // Query strings and fragments never enter the event store.
    let page = input.page.split(['?', '#']).next().unwrap_or("/");
    if !page.starts_with('/')
        || page.starts_with("//")
        || page.len() > 1000
        || page.chars().any(char::is_control)
    {
        return Err(Error::Invalid("Invalid page path"));
    }
    if input.referrer_host.as_ref().is_some_and(|h| {
        h.len() > 253
            || h.bytes()
                .any(|b| !b.is_ascii_alphanumeric() && !b".-".contains(&b))
    }) {
        return Err(Error::Invalid("Referrer must be a hostname only"));
    }
    if input.components.len() > matching::WEIGHTS.len()
        || input.components.iter().any(|(k, v)| {
            !matching::WEIGHTS.iter().any(|(key, _)| key == k)
                || v.is_empty()
                || v.len() > 500
                || v.chars().any(char::is_control)
        })
    {
        return Err(Error::Invalid("Invalid browser signals"));
    }
    let mut tx = pool.begin().await?;
    let (ws, site) = public_site(&mut tx, key, origin, true).await?;
    if sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM visitors.withdrawals WHERE site_id=$1 AND token_hash=$2 AND expires_at>now())").bind(site.id).bind(&hash).fetch_one(&mut *tx).await? { return Err(Error::Invalid("Recognition consent was withdrawn")); }
    if sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM visitors.events WHERE site_id=$1 AND id=$2)",
    )
    .bind(site.id)
    .bind(input.event_id)
    .fetch_one(&mut *tx)
    .await?
    {
        return Ok(Receipt { accepted: true });
    }
    sqlx::query("DELETE FROM visitors.visitors WHERE site_id=$1 AND expires_at<=now()")
        .bind(site.id)
        .execute(&mut *tx)
        .await?;
    let signals = if site.fingerprint_enabled {
        input.components
    } else {
        Signals::new()
    };
    let fingerprint = (!signals.is_empty()).then(|| {
        digest(&format!(
            "{}:{}",
            site.id,
            serde_json::to_string(&signals).expect("serializes")
        ))
    });
    let id=sqlx::query_scalar::<_,Uuid>("INSERT INTO visitors.visitors(workspace_id,site_id,token_hash,components,fingerprint,expires_at) VALUES($1,$2,$3,$4,$5,now()+$6*interval '1 day') ON CONFLICT(site_id,token_hash) DO UPDATE SET last_seen=now(),expires_at=now()+$6*interval '1 day',components=CASE WHEN $5 IS NULL THEN visitors.visitors.components ELSE $4 END,fingerprint=COALESCE($5,visitors.visitors.fingerprint) RETURNING id").bind(ws).bind(site.id).bind(hash).bind(Json(&signals)).bind(&fingerprint).bind(site.retention_days).fetch_one(&mut *tx).await?;
    sqlx::query("INSERT INTO visitors.events(id,workspace_id,site_id,visitor_id,page,referrer_host) VALUES($1,$2,$3,$4,$5,$6)").bind(input.event_id).bind(ws).bind(site.id).bind(id).bind(page).bind(input.referrer_host).execute(&mut *tx).await?;
    if let Some(fingerprint) = fingerprint {
        let candidates=sqlx::query_as::<_,(Uuid,Json<Signals>,Option<String>)>("SELECT id,components,fingerprint FROM visitors.visitors WHERE site_id=$1 AND id<>$2 AND last_seen>now()-interval '60 days' AND expires_at>now() AND fingerprint IS NOT NULL ORDER BY last_seen DESC LIMIT 200").bind(site.id).bind(id).fetch_all(&mut *tx).await?;
        let mut count = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM visitors.matches WHERE workspace_id=$1 AND visitor_id=$2",
        )
        .bind(ws)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        for (other, old, old_hash) in candidates {
            if count >= 200 {
                break;
            }
            if let Some(score) = matching::score(&signals, &old.0) {
                let exact = old_hash.as_deref() == Some(&fingerprint);
                if exact || site.similarity_enabled && score >= site.threshold {
                    let changed=sqlx::query("INSERT INTO visitors.matches(workspace_id,visitor_id,candidate_id,score,reason) VALUES($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING").bind(ws).bind(id).bind(other).bind(score).bind(if exact{"Exact browser signals; possible collision"}else{"Similar browser signals; uncertain attribution"}).execute(&mut *tx).await?;
                    count += changed.rows_affected() as i64;
                }
            }
        }
    }
    tx.commit().await?;
    Ok(Receipt { accepted: true })
}
pub async fn withdraw(
    pool: &PgPool,
    key: &str,
    origin: &str,
    input: Withdraw,
) -> Result<Receipt, Error> {
    let hash = token_hash(&input.token)?;
    let mut tx = pool.begin().await?;
    // Withdrawal remains possible after collection is disabled or its module is off.
    let row = sqlx::query_as::<_, (Uuid, Json<Vec<String>>)>(
        "SELECT id,origins FROM visitors.sites WHERE public_key=$1 FOR UPDATE",
    )
    .bind(key)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(Error::NotFound)?;
    if !row.1.0.iter().any(|o| o == origin) {
        return Err(Error::NotFound);
    }
    sqlx::query("INSERT INTO visitors.withdrawals(site_id,token_hash) VALUES($1,$2) ON CONFLICT(site_id,token_hash) DO UPDATE SET expires_at=now()+interval '180 days'").bind(row.0).bind(&hash).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM visitors.visitors WHERE site_id=$1 AND token_hash=$2")
        .bind(row.0)
        .bind(hash)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Receipt { accepted: true })
}
pub async fn list(pool: &PgPool, a: &ActorContext<'_>, q: Query) -> Result<Page, Error> {
    let mut tx = pool.begin().await?;
    authority(&mut tx, a, "visitors.read").await?;
    contacts_authority(&mut tx, a).await?;
    let sql = format!(
        "SELECT {VISITOR} FROM visitors.visitors v WHERE workspace_id=$1 AND expires_at>now() AND ($2::uuid IS NULL OR site_id=$2) AND ($3::uuid IS NULL OR contact_id=$3) AND ($4::uuid IS NULL OR (first_seen,id)<(SELECT first_seen,id FROM visitors.visitors WHERE workspace_id=$1 AND id=$4)) ORDER BY first_seen DESC,id DESC LIMIT 51"
    );
    let mut items: Vec<Visitor> =
        sqlx::query_scalar::<_, Json<Visitor>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(a.workspace)
            .bind(q.site_id)
            .bind(q.contact_id)
            .bind(q.after)
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .map(|x| x.0)
            .collect();
    let next_cursor = if items.len() > 50 {
        items.pop();
        items.last().map(|v| v.id)
    } else {
        None
    };
    Ok(Page { items, next_cursor })
}
async fn visitor(db: &mut PgConnection, ws: Uuid, id: Uuid) -> Result<Visitor, Error> {
    let sql = format!(
        "SELECT {VISITOR} FROM visitors.visitors v WHERE workspace_id=$1 AND id=$2 AND expires_at>now()"
    );
    Ok(
        sqlx::query_scalar::<_, Json<Visitor>>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(ws)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?
            .0,
    )
}
pub async fn detail(pool: &PgPool, a: &ActorContext<'_>, id: Uuid) -> Result<Detail, Error> {
    let mut tx = pool.begin().await?;
    authority(&mut tx, a, "visitors.read").await?;
    contacts_authority(&mut tx, a).await?;
    let v = visitor(&mut tx, a.workspace, id).await?;
    let components = sqlx::query_scalar::<_, Json<Signals>>(
        "SELECT components FROM visitors.visitors WHERE workspace_id=$1 AND id=$2",
    )
    .bind(a.workspace)
    .bind(id)
    .fetch_one(&mut *tx)
    .await?
    .0;
    let events=sqlx::query_scalar::<_,Json<VisitEvent>>("SELECT jsonb_build_object('id',id,'page',page,'referrer_host',referrer_host,'created_at',created_at::text) FROM visitors.events WHERE workspace_id=$1 AND visitor_id=$2 ORDER BY created_at DESC LIMIT 1000").bind(a.workspace).bind(id).fetch_all(&mut *tx).await?.into_iter().map(|x|x.0).collect();
    let candidates=sqlx::query_scalar::<_,Json<Candidate>>("SELECT jsonb_build_object('candidate_id',m.candidate_id,'score',m.score,'reason',m.reason,'state',m.state,'version',m.version) FROM visitors.matches m JOIN visitors.visitors v ON v.id=m.candidate_id WHERE m.workspace_id=$1 AND m.visitor_id=$2 AND v.expires_at>now() ORDER BY score DESC LIMIT 200").bind(a.workspace).bind(id).fetch_all(&mut *tx).await?.into_iter().map(|x|x.0).collect();
    Ok(Detail {
        visitor: v,
        components,
        events,
        candidates,
    })
}
pub async fn resolve(
    pool: &PgPool,
    a: &ActorContext<'_>,
    key: &str,
    input: Resolve,
) -> Result<Visitor, Error> {
    if !["declared_contact", "staff_review"].contains(&input.evidence.as_str()) {
        return Err(Error::Invalid(
            "Use declared_contact or staff_review evidence",
        ));
    }
    let hash = token_hash(&input.token)?;
    let mut tx = pool.begin().await?;
    modules::lock_workspace(&mut tx, a.workspace).await?;
    let grant = authority(&mut tx, a, "visitors.manage").await?;
    contacts_authority(&mut tx, a).await?;
    let request = json!({"site":input.site_key,"token_hash":hash,"contact_id":input.contact_id,"evidence":input.evidence});
    let op = "visitors.resolve";
    if let Some(saved) =
        reliability::replay(&mut tx, a.workspace, &grant, op, key, &request).await?
    {
        return Ok(saved);
    }
    if !sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM contacts.contacts WHERE workspace_id=$1 AND id=$2)",
    )
    .bind(a.workspace)
    .bind(input.contact_id)
    .fetch_one(&mut *tx)
    .await?
    {
        return Err(Error::NotFound);
    }
    let row=sqlx::query_as::<_,(Uuid,Option<Uuid>)>("SELECT v.id,v.contact_id FROM visitors.visitors v JOIN visitors.sites s ON s.id=v.site_id WHERE v.workspace_id=$1 AND s.public_key=$2 AND v.token_hash=$3 AND v.expires_at>now() FOR UPDATE OF v").bind(a.workspace).bind(input.site_key).bind(hash).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
    if row.1.is_some_and(|id| id != input.contact_id) {
        return Err(Error::Conflict(
            "Visitor has different declared contact evidence; review it manually",
        ));
    }
    sqlx::query("UPDATE visitors.visitors SET contact_id=$3,evidence=$4,version=version+1 WHERE workspace_id=$1 AND id=$2").bind(a.workspace).bind(row.0).bind(input.contact_id).bind(&input.evidence).execute(&mut *tx).await?;
    reliability::audit(
        &mut tx,
        a,
        &grant,
        "visitors.resolved",
        row.0,
        json!({"contact_id":input.contact_id,"evidence":input.evidence}),
    )
    .await?;
    let result = visitor(&mut tx, a.workspace, row.0).await?;
    reliability::save_replay(&mut tx, a.workspace, &grant, op, key, &request, &result).await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn review(
    pool: &PgPool,
    a: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: Review,
) -> Result<Receipt, Error> {
    if !["accepted", "rejected", "suggested"].contains(&input.state.as_str()) {
        return Err(Error::Invalid("Invalid match decision"));
    }
    let mut tx = pool.begin().await?;
    modules::lock_workspace(&mut tx, a.workspace).await?;
    let grant = authority(&mut tx, a, "visitors.manage").await?;
    contacts_authority(&mut tx, a).await?;
    visitor(&mut tx, a.workspace, id).await?;
    visitor(&mut tx, a.workspace, input.candidate_id).await?;
    let request = serde_json::to_value(&input).expect("serializes");
    let op = format!("visitors.{id}.review");
    if let Some(saved) =
        reliability::replay(&mut tx, a.workspace, &grant, &op, key, &request).await?
    {
        return Ok(saved);
    }
    let result=sqlx::query("UPDATE visitors.matches SET state=$4,version=version+1 WHERE workspace_id=$1 AND visitor_id=$2 AND candidate_id=$3 AND version=$5").bind(a.workspace).bind(id).bind(input.candidate_id).bind(&input.state).bind(input.expected_version).execute(&mut *tx).await?;
    if result.rows_affected() != 1 {
        return Err(Error::Conflict("Match changed; reload"));
    }
    reliability::audit(
        &mut tx,
        a,
        &grant,
        "visitors.match_reviewed",
        id,
        json!({"candidate_id":input.candidate_id,"state":input.state}),
    )
    .await?;
    let result = Receipt { accepted: true };
    reliability::save_replay(&mut tx, a.workspace, &grant, &op, key, &request, &result).await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn delete(
    pool: &PgPool,
    a: &ActorContext<'_>,
    id: Uuid,
    key: &str,
) -> Result<Receipt, Error> {
    let mut tx = pool.begin().await?;
    modules::lock_workspace(&mut tx, a.workspace).await?;
    let grant = authority(&mut tx, a, "visitors.manage").await?;
    contacts_authority(&mut tx, a).await?;
    let op = format!("visitors.{id}.delete");
    let request = json!({"id":id});
    if let Some(saved) =
        reliability::replay(&mut tx, a.workspace, &grant, &op, key, &request).await?
    {
        return Ok(saved);
    }
    // Serialize staff deletion with collection and public withdrawal for this site.
    sqlx::query("SELECT s.id FROM visitors.sites s JOIN visitors.visitors v ON v.site_id=s.id WHERE v.workspace_id=$1 AND v.id=$2 FOR UPDATE OF s").bind(a.workspace).bind(id).fetch_optional(&mut *tx).await?;
    sqlx::query("INSERT INTO visitors.withdrawals(site_id,token_hash) SELECT site_id,token_hash FROM visitors.visitors WHERE workspace_id=$1 AND id=$2 ON CONFLICT(site_id,token_hash) DO UPDATE SET expires_at=now()+interval '180 days'").bind(a.workspace).bind(id).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM visitors.visitors WHERE workspace_id=$1 AND id=$2")
        .bind(a.workspace)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    reliability::audit(&mut tx, a, &grant, "visitors.deleted", id, json!({})).await?;
    let result = Receipt { accepted: true };
    reliability::save_replay(&mut tx, a.workspace, &grant, &op, key, &request, &result).await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn prune(pool: &PgPool) -> Result<(), Error> {
    sqlx::query("DELETE FROM visitors.withdrawals WHERE (site_id,token_hash) IN(SELECT site_id,token_hash FROM visitors.withdrawals WHERE expires_at<=now() LIMIT 1000)").execute(pool).await?;
    sqlx::query("DELETE FROM visitors.events WHERE (site_id,id) IN (SELECT e.site_id,e.id FROM visitors.events e JOIN visitors.sites s ON s.id=e.site_id WHERE e.created_at < now()-s.retention_days*interval '1 day' LIMIT 1000)").execute(pool).await?;
    sqlx::query("DELETE FROM visitors.visitors WHERE id IN(SELECT id FROM visitors.visitors WHERE expires_at<=now() ORDER BY expires_at LIMIT 1000)").execute(pool).await?;
    Ok(())
}

pub async fn events(
    pool: &PgPool,
    a: &ActorContext<'_>,
    id: Uuid,
    q: EventsQuery,
) -> Result<EventsPage, Error> {
    let mut tx = pool.begin().await?;
    authority(&mut tx, a, "visitors.read").await?;
    contacts_authority(&mut tx, a).await?;
    visitor(&mut tx, a.workspace, id).await?;
    let mut items:Vec<VisitEvent>=sqlx::query_scalar::<_,Json<VisitEvent>>("SELECT jsonb_build_object('id',id,'page',page,'referrer_host',referrer_host,'created_at',created_at::text) FROM visitors.events WHERE workspace_id=$1 AND visitor_id=$2 AND ($3::uuid IS NULL OR (created_at,id)<(SELECT created_at,id FROM visitors.events WHERE workspace_id=$1 AND visitor_id=$2 AND id=$3)) ORDER BY created_at DESC,id DESC LIMIT 201").bind(a.workspace).bind(id).bind(q.after).fetch_all(&mut *tx).await?.into_iter().map(|x|x.0).collect();
    let next_cursor = if items.len() > 200 {
        items.pop();
        items.last().map(|e| e.id)
    } else {
        None
    };
    Ok(EventsPage { items, next_cursor })
}
pub async fn link(
    pool: &PgPool,
    a: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: Link,
) -> Result<Visitor, Error> {
    let mut tx = pool.begin().await?;
    modules::lock_workspace(&mut tx, a.workspace).await?;
    let grant = authority(&mut tx, a, "visitors.manage").await?;
    contacts_authority(&mut tx, a).await?;
    let op = format!("visitors.{id}.link");
    let request = serde_json::to_value(&input).expect("serializes");
    if let Some(saved) =
        reliability::replay(&mut tx, a.workspace, &grant, &op, key, &request).await?
    {
        return Ok(saved);
    }
    if let Some(contact) = input.contact_id
        && !sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM contacts.contacts WHERE workspace_id=$1 AND id=$2)",
        )
        .bind(a.workspace)
        .bind(contact)
        .fetch_one(&mut *tx)
        .await?
    {
        return Err(Error::NotFound);
    }
    let changed=sqlx::query("UPDATE visitors.visitors SET contact_id=$3,evidence=CASE WHEN $3::uuid IS NULL THEN NULL ELSE 'staff_review' END,version=version+1 WHERE workspace_id=$1 AND id=$2 AND version=$4 AND expires_at>now()").bind(a.workspace).bind(id).bind(input.contact_id).bind(input.expected_version).execute(&mut *tx).await?;
    if changed.rows_affected() != 1 {
        return Err(Error::Conflict("Visitor changed or is unavailable; reload"));
    }
    reliability::audit(
        &mut tx,
        a,
        &grant,
        "visitors.contact_reviewed",
        id,
        json!({"contact_id":input.contact_id}),
    )
    .await?;
    let result = visitor(&mut tx, a.workspace, id).await?;
    reliability::save_replay(&mut tx, a.workspace, &grant, &op, key, &request, &result).await?;
    tx.commit().await?;
    Ok(result)
}
