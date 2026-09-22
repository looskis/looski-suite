// SPDX-License-Identifier: AGPL-3.0-only
//! Sales pursuit and concrete transactions have distinct records and histories.
pub mod configuration;
mod dao;
pub mod participants;
pub mod tagging;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    modules,
};
use sqlx::PgConnection;
async fn authorize(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    action: &str,
) -> Result<access::Access, Error> {
    let a = access::require(db, actor.identity, actor.workspace, action).await?;
    modules::require_crm(db, actor).await?;
    Ok(a)
}
use crate::reliability;
use configuration::{Kind, Outcome};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct OpportunityInput {
    pub title: String,
    #[serde(default)]
    pub notes: String,
    pub owner_membership_id: Option<Uuid>,
    pub pipeline_id: Uuid,
    pub stage_id: Uuid,
    pub expected_value: Option<String>,
    pub currency_code: Option<String>,
    pub expected_close_on: Option<String>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EditOpportunity {
    pub expected_version: i64,
    #[serde(flatten)]
    pub input: OpportunityInput,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Opportunity {
    pub id: Uuid,
    pub title: String,
    pub notes: String,
    pub owner_membership_id: Uuid,
    pub pipeline_id: Uuid,
    pub pipeline_name: String,
    pub stage_id: Uuid,
    pub stage_name: String,
    pub outcome: Outcome,
    pub expected_value: Option<String>,
    pub currency_code: Option<String>,
    pub expected_close_on: Option<String>,
    pub closed_at: Option<String>,
    pub archived: bool,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DealInput {
    pub title: String,
    #[serde(default)]
    pub notes: String,
    pub owner_membership_id: Option<Uuid>,
    pub category_id: Uuid,
    pub status_id: Uuid,
    pub value: Option<String>,
    pub currency_code: Option<String>,
    pub starts_on: Option<String>,
    pub ends_on: Option<String>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NewDeal {
    pub originating_opportunity_id: Option<Uuid>,
    #[serde(flatten)]
    pub input: DealInput,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EditDeal {
    pub expected_version: i64,
    #[serde(flatten)]
    pub input: DealInput,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Deal {
    pub id: Uuid,
    pub title: String,
    pub notes: String,
    pub owner_membership_id: Uuid,
    pub category_id: Uuid,
    pub category_name: String,
    pub status_id: Uuid,
    pub status_name: String,
    pub originating_opportunity_id: Option<Uuid>,
    pub value: Option<String>,
    pub currency_code: Option<String>,
    pub starts_on: Option<String>,
    pub ends_on: Option<String>,
    pub archived: bool,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ArchiveRecord {
    pub archived: bool,
    pub expected_version: i64,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct OpportunityQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
    pub search: Option<String>,
    pub tag_id: Option<Uuid>,
    pub archived: Option<bool>,
    pub pipeline_id: Option<Uuid>,
    pub stage_id: Option<Uuid>,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct DealQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
    pub search: Option<String>,
    pub tag_id: Option<Uuid>,
    pub archived: Option<bool>,
    pub category_id: Option<Uuid>,
    pub status_id: Option<Uuid>,
    pub originating_opportunity_id: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct OpportunityPage {
    pub items: Vec<Opportunity>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DealPage {
    pub items: Vec<Deal>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Transition {
    pub id: Uuid,
    pub from_id: Option<Uuid>,
    pub to_id: Uuid,
    pub from_label: Option<String>,
    pub to_label: String,
    pub from_outcome: Option<Outcome>,
    pub to_outcome: Option<Outcome>,
    pub audit_event_id: Uuid,
    pub occurred_at: String,
}
#[derive(Default, Serialize, Deserialize, JsonSchema, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct HistoryQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
}
#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct HistoryPage {
    pub items: Vec<Transition>,
    pub next_cursor: Option<Uuid>,
}
fn validate(
    title: &str,
    notes: &str,
    value: &Option<String>,
    currency: &Option<String>,
    dates: &[&Option<String>],
) -> Result<(), Error> {
    if title.trim().is_empty()
        || title.chars().count() > 200
        || title.chars().any(char::is_control)
        || notes.chars().count() > 10000
    {
        return Err(Error::Invalid(
            "Title must be 1–200 printable characters; notes at most 10000 characters",
        ));
    }
    if value.is_some() != currency.is_some() {
        return Err(Error::Invalid(
            "Amount and currency must be supplied together",
        ));
    }
    if let Some(v) = value {
        let parts: Vec<_> = v.split('.').collect();
        if parts.is_empty()
            || parts.len() > 2
            || parts[0].is_empty()
            || parts[0].len() > 14
            || !parts[0].bytes().all(|b| b.is_ascii_digit())
            || parts.get(1).is_some_and(|p| {
                p.is_empty() || p.len() > 6 || !p.bytes().all(|b| b.is_ascii_digit())
            })
        {
            return Err(Error::Invalid(
                "Amount must be a nonnegative decimal string with at most 14 integer and 6 fractional digits",
            ));
        }
    }
    if currency.as_ref().is_some_and(|c| {
        !include_str!("crm/currencies.txt")
            .lines()
            .any(|code| code == c)
    }) {
        return Err(Error::Invalid(
            "Currency must be a supported current or historical ISO currency code",
        ));
    }
    for date in dates.iter().filter_map(|d| d.as_ref()) {
        let p: Vec<_> = date.split('-').collect();
        if p.len() != 3
            || p[0].len() != 4
            || p[1].len() != 2
            || p[2].len() != 2
            || !p.iter().all(|v| v.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(Error::Invalid(
                "Dates must be valid YYYY-MM-DD calendar dates",
            ));
        }
        let y = p[0].parse::<u32>().unwrap_or(0);
        let m = p[1].parse::<u32>().unwrap_or(0);
        let d = p[2].parse::<u32>().unwrap_or(0);
        let max = match m {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) => 29,
            2 => 28,
            _ => 0,
        };
        if y == 0 || d == 0 || d > max {
            return Err(Error::Invalid(
                "Dates must be valid YYYY-MM-DD calendar dates",
            ));
        }
    }
    Ok(())
}
fn page_limit(limit: Option<i64>, search: Option<&String>) -> Result<i64, Error> {
    let limit = limit.unwrap_or(50);
    if !(1..=100).contains(&limit) || search.is_some_and(|s| s.len() > 200) {
        return Err(Error::Invalid(
            "Limit must be 1–100; search at most 200 bytes",
        ));
    }
    Ok(limit)
}
fn record_scope(a: &access::Access, owner: Uuid) -> Result<(), Error> {
    if !a.permits_owner(owner) {
        return Err(Error::NotFound);
    }
    Ok(())
}
async fn mutation(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    action: &str,
    read: &str,
) -> Result<(access::Access, access::Access), Error> {
    authorize(db, actor, action).await?;
    modules::lock_workspace(db, actor.workspace).await?;
    Ok((
        authorize(db, actor, action).await?,
        authorize(db, actor, read).await?,
    ))
}
async fn owner(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    write: &access::Access,
    read: &access::Access,
    id: Uuid,
) -> Result<(), Error> {
    if !write.permits_owner(id) || !read.permits_owner(id) {
        return Err(Error::Forbidden);
    }
    access::require_active_member(db, actor.workspace, id).await
}
async fn read_opportunity(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<Opportunity, Error> {
    let read = authorize(db, actor, "crm.opportunities.read").await?;
    let record = dao::opportunity(db, actor.workspace, id).await?;
    record_scope(&read, record.owner_membership_id)?;
    Ok(record)
}
pub async fn opportunity(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<Opportunity, Error> {
    let mut tx = pool.begin().await?;
    read_opportunity(&mut tx, actor, id).await
}
pub async fn opportunities(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: OpportunityQuery,
) -> Result<OpportunityPage, Error> {
    let limit = page_limit(query.limit, query.search.as_ref())?;
    let mut tx = pool.begin().await?;
    let a = authorize(&mut tx, actor, "crm.opportunities.read").await?;
    if let Some(tag) = query.tag_id {
        crate::tags::definitions(&mut tx, actor, &[tag], false).await?;
    }
    if let Some(after) = query.after {
        read_opportunity(&mut tx, actor, after).await?;
    }
    let mut items = dao::opportunities(&mut tx, actor.workspace, &a, &query, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    Ok(OpportunityPage { items, next_cursor })
}
pub async fn save_opportunity(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Option<Uuid>,
    expected: Option<i64>,
    key: &str,
    input: OpportunityInput,
) -> Result<Opportunity, Error> {
    validate(
        &input.title,
        &input.notes,
        &input.expected_value,
        &input.currency_code,
        &[&input.expected_close_on],
    )?;
    let mut tx = pool.begin().await?;
    let (write, read) = mutation(
        &mut tx,
        actor,
        if id.is_some() {
            "crm.opportunities.update"
        } else {
            "crm.opportunities.create"
        },
        "crm.opportunities.read",
    )
    .await?;
    let request = serde_json::json!({"input":input,"expected_version":expected});
    let operation = format!(
        "crm.opportunities.save:{}",
        id.map(|i| i.to_string()).unwrap_or_else(|| "new".into())
    );
    if let Some(saved) = reliability::replay::<Opportunity>(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
    )
    .await?
    {
        let current = read_opportunity(&mut tx, actor, saved.id).await?;
        record_scope(&write, current.owner_membership_id)?;
        return Ok(saved);
    }
    let current = if let Some(id) = id {
        let c = read_opportunity(&mut tx, actor, id).await?;
        record_scope(&write, c.owner_membership_id)?;
        if Some(c.version) != expected || c.archived {
            return Err(Error::Conflict(
                "Opportunity changed or is archived; reload before editing",
            ));
        }
        Some(c)
    } else {
        None
    };
    let owner_id = input.owner_membership_id.unwrap_or_else(|| {
        current
            .as_ref()
            .map(|c| c.owner_membership_id)
            .unwrap_or(write.membership_id)
    });
    owner(&mut tx, actor, &write, &read, owner_id).await?;
    let unchanged = current
        .as_ref()
        .is_some_and(|c| c.stage_id == input.stage_id && c.pipeline_id == input.pipeline_id);
    let state = configuration::state(
        &mut tx,
        actor.workspace,
        Kind::Stage,
        input.pipeline_id,
        input.stage_id,
        unchanged,
    )
    .await?;
    let id = dao::save_opportunity(
        &mut tx,
        actor.workspace,
        id,
        owner_id,
        &input,
        state.outcome.expect("stage outcome"),
        unchanged,
    )
    .await?;
    let audit = reliability::audit(
        &mut tx,
        actor,
        &write,
        "crm.opportunity.saved",
        id,
        serde_json::json!({"stage_id":input.stage_id}),
    )
    .await?;
    if !unchanged {
        dao::opportunity_transition(
            &mut tx,
            actor.workspace,
            id,
            current.as_ref(),
            &state,
            audit,
        )
        .await?;
    }
    let result = dao::opportunity(&mut tx, actor.workspace, id).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
async fn redact_origins(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    deals: &mut [Deal],
) -> Result<(), Error> {
    let a = match authorize(db, actor, "crm.opportunities.read").await {
        Ok(a) => Some(a),
        Err(Error::Forbidden) => None,
        Err(e) => return Err(e),
    };
    let visible = if let Some(a) = a {
        dao::visible_opportunities(
            db,
            actor.workspace,
            &a,
            &deals
                .iter()
                .filter_map(|d| d.originating_opportunity_id)
                .collect::<Vec<_>>(),
        )
        .await?
    } else {
        Vec::new()
    };
    for deal in deals {
        if deal
            .originating_opportunity_id
            .is_some_and(|id| !visible.contains(&id))
        {
            deal.originating_opportunity_id = None
        }
    }
    Ok(())
}
async fn read_deal(
    db: &mut PgConnection,
    actor: &ActorContext<'_>,
    id: Uuid,
) -> Result<Deal, Error> {
    let a = authorize(db, actor, "crm.deals.read").await?;
    let d = dao::deal(db, actor.workspace, id).await?;
    record_scope(&a, d.owner_membership_id)?;
    Ok(d)
}
pub async fn deal(pool: &PgPool, actor: &ActorContext<'_>, id: Uuid) -> Result<Deal, Error> {
    let mut tx = pool.begin().await?;
    let mut d = read_deal(&mut tx, actor, id).await?;
    redact_origins(&mut tx, actor, std::slice::from_mut(&mut d)).await?;
    Ok(d)
}
pub async fn deals(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    query: DealQuery,
) -> Result<DealPage, Error> {
    let limit = page_limit(query.limit, query.search.as_ref())?;
    let mut tx = pool.begin().await?;
    let a = authorize(&mut tx, actor, "crm.deals.read").await?;
    if let Some(tag) = query.tag_id {
        crate::tags::definitions(&mut tx, actor, &[tag], false).await?;
    }
    if let Some(after) = query.after {
        read_deal(&mut tx, actor, after).await?;
    }
    if let Some(origin) = query.originating_opportunity_id {
        read_opportunity(&mut tx, actor, origin).await?;
    }
    let mut items = dao::deals(&mut tx, actor.workspace, &a, &query, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    redact_origins(&mut tx, actor, &mut items).await?;
    Ok(DealPage { items, next_cursor })
}
pub async fn save_deal(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Option<Uuid>,
    expected: Option<i64>,
    origin: Option<Uuid>,
    key: &str,
    input: DealInput,
) -> Result<Deal, Error> {
    validate(
        &input.title,
        &input.notes,
        &input.value,
        &input.currency_code,
        &[&input.starts_on, &input.ends_on],
    )?;
    if let (Some(start), Some(end)) = (&input.starts_on, &input.ends_on)
        && end < start
    {
        return Err(Error::Invalid("End date must not precede start date"));
    }
    let mut tx = pool.begin().await?;
    let (write, read) = mutation(
        &mut tx,
        actor,
        if id.is_some() {
            "crm.deals.update"
        } else {
            "crm.deals.create"
        },
        "crm.deals.read",
    )
    .await?;
    let request = serde_json::json!({"input":input,"expected_version":expected,"origin":origin});
    let operation = format!(
        "crm.deals.save:{}",
        id.map(|i| i.to_string()).unwrap_or_else(|| "new".into())
    );
    if let Some(saved) =
        reliability::replay::<Deal>(&mut tx, actor.workspace, &write, &operation, key, &request)
            .await?
    {
        let current = read_deal(&mut tx, actor, saved.id).await?;
        record_scope(&write, current.owner_membership_id)?;
        if let Some(origin) = saved.originating_opportunity_id {
            read_opportunity(&mut tx, actor, origin).await?;
        }
        return Ok(saved);
    }
    let current = if let Some(id) = id {
        let c = read_deal(&mut tx, actor, id).await?;
        record_scope(&write, c.owner_membership_id)?;
        if Some(c.version) != expected || c.archived {
            return Err(Error::Conflict(
                "Deal changed or is archived; reload before editing",
            ));
        }
        Some(c)
    } else {
        None
    };
    if id.is_none()
        && let Some(origin) = origin
    {
        let o = read_opportunity(&mut tx, actor, origin).await?;
        if o.archived {
            return Err(Error::Conflict(
                "Archived opportunities cannot originate new deals",
            ));
        }
    }
    let owner_id = input.owner_membership_id.unwrap_or_else(|| {
        current
            .as_ref()
            .map(|c| c.owner_membership_id)
            .unwrap_or(write.membership_id)
    });
    owner(&mut tx, actor, &write, &read, owner_id).await?;
    let unchanged = current
        .as_ref()
        .is_some_and(|c| c.status_id == input.status_id && c.category_id == input.category_id);
    let state = configuration::state(
        &mut tx,
        actor.workspace,
        Kind::Status,
        input.category_id,
        input.status_id,
        unchanged,
    )
    .await?;
    let id = dao::save_deal(&mut tx, actor.workspace, id, owner_id, &input, origin).await?;
    let audit = reliability::audit(
        &mut tx,
        actor,
        &write,
        "crm.deal.saved",
        id,
        serde_json::json!({"status_id":input.status_id}),
    )
    .await?;
    if !unchanged {
        dao::deal_transition(
            &mut tx,
            actor.workspace,
            id,
            current.as_ref(),
            &state,
            audit,
        )
        .await?;
    }
    let mut result = dao::deal(&mut tx, actor.workspace, id).await?;
    redact_origins(&mut tx, actor, std::slice::from_mut(&mut result)).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn history(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    is_deal: bool,
    query: HistoryQuery,
) -> Result<HistoryPage, Error> {
    let limit = page_limit(query.limit, None)?;
    let mut tx = pool.begin().await?;
    if is_deal {
        read_deal(&mut tx, actor, id).await?;
    } else {
        read_opportunity(&mut tx, actor, id).await?;
    }
    let mut items = dao::history(&mut tx, actor.workspace, id, is_deal, &query, limit + 1).await?;
    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|r| r.id)
    } else {
        None
    };
    Ok(HistoryPage { items, next_cursor })
}
pub async fn archive_opportunity(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: ArchiveRecord,
) -> Result<Opportunity, Error> {
    let mut tx = pool.begin().await?;
    let (write, _) = mutation(
        &mut tx,
        actor,
        "crm.opportunities.archive",
        "crm.opportunities.read",
    )
    .await?;
    let request = serde_json::to_value(&input).expect("archive serializes");
    let operation = format!("crm.opportunities.archive:{id}");
    let current = read_opportunity(&mut tx, actor, id).await?;
    record_scope(&write, current.owner_membership_id)?;
    if let Some(saved) =
        reliability::replay(&mut tx, actor.workspace, &write, &operation, key, &request).await?
    {
        return Ok(saved);
    }
    if current.version != input.expected_version {
        return Err(Error::Conflict(
            "Opportunity changed; reload before archiving",
        ));
    }
    dao::archive(&mut tx, actor.workspace, id, false, input.archived).await?;
    reliability::audit(
        &mut tx,
        actor,
        &write,
        "crm.opportunity.archived",
        id,
        serde_json::json!({"archived":input.archived}),
    )
    .await?;
    let result = dao::opportunity(&mut tx, actor.workspace, id).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn archive_deal(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    id: Uuid,
    key: &str,
    input: ArchiveRecord,
) -> Result<Deal, Error> {
    let mut tx = pool.begin().await?;
    let (write, _) = mutation(&mut tx, actor, "crm.deals.archive", "crm.deals.read").await?;
    let request = serde_json::to_value(&input).expect("archive serializes");
    let operation = format!("crm.deals.archive:{id}");
    let current = read_deal(&mut tx, actor, id).await?;
    record_scope(&write, current.owner_membership_id)?;
    if let Some(saved) =
        reliability::replay::<Deal>(&mut tx, actor.workspace, &write, &operation, key, &request)
            .await?
    {
        if let Some(origin) = saved.originating_opportunity_id {
            read_opportunity(&mut tx, actor, origin).await?;
        }
        return Ok(saved);
    }
    if current.version != input.expected_version {
        return Err(Error::Conflict("Deal changed; reload before archiving"));
    }
    dao::archive(&mut tx, actor.workspace, id, true, input.archived).await?;
    reliability::audit(
        &mut tx,
        actor,
        &write,
        "crm.deal.archived",
        id,
        serde_json::json!({"archived":input.archived}),
    )
    .await?;
    let mut result = dao::deal(&mut tx, actor.workspace, id).await?;
    redact_origins(&mut tx, actor, std::slice::from_mut(&mut result)).await?;
    reliability::save_replay(
        &mut tx,
        actor.workspace,
        &write,
        &operation,
        key,
        &request,
        &result,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
