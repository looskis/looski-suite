// SPDX-License-Identifier: AGPL-3.0-only
//! Shared typed record fields, query expressions and personal/team view definitions.
mod query;
use crate::{
    access,
    application::{ActorContext, OperationError as Error},
    reliability,
};
pub use query::{append_filter, append_sort, validate_filter};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::{PgConnection, PgPool, Row, types::Json};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, ToSchema, JsonSchema)]
pub struct RecordField {
    pub key: String,
    pub name: String,
    pub field_type: String,
    pub options: Vec<String>,
    pub editable: bool,
    pub custom: bool,
    pub archived: bool,
    pub version: i64,
}
#[derive(Clone, Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Filter {
    Condition {
        field: String,
        operator: String,
        #[serde(default)]
        value: Value,
    },
    Group {
        operator: String,
        #[schema(no_recursion)]
        rules: Vec<Filter>,
    },
}
#[derive(Clone, Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ViewConfig {
    pub columns: Vec<String>,
    pub filter: Option<Filter>,
    pub sort: String,
    pub direction: String,
    #[serde(default)]
    pub archived: bool,
}
#[derive(Serialize, Deserialize, ToSchema, JsonSchema)]
pub struct RecordView {
    pub id: Uuid,
    pub name: String,
    pub shared: bool,
    pub mine: bool,
    pub version: i64,
    pub config: ViewConfig,
}
#[derive(Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveView {
    pub name: String,
    #[serde(default)]
    pub shared: bool,
    pub config: ViewConfig,
    pub expected_version: Option<i64>,
}
#[derive(Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveField {
    pub key: String,
    pub name: String,
    pub field_type: String,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub archived: bool,
    pub expected_version: Option<i64>,
}

pub fn permission(kind: &str, action: &str) -> Result<String, Error> {
    match kind {
        "contacts" => Ok(format!("contacts.{action}")),
        "organizations" => Ok(format!("contacts.organizations.{action}")),
        _ => Err(Error::NotFound),
    }
}
fn field(key: &str, name: &str, kind: &str, editable: bool, options: &[&str]) -> RecordField {
    RecordField {
        key: key.into(),
        name: name.into(),
        field_type: kind.into(),
        editable,
        options: options.iter().map(|s| s.to_string()).collect(),
        custom: false,
        archived: false,
        version: 0,
    }
}
pub fn builtin(kind: &str) -> Result<Vec<RecordField>, Error> {
    permission(kind, "read")?;
    let mut fields = vec![
        field("name", "Name", "text", false, &[]),
        field(
            "status",
            "Status",
            "select",
            true,
            &["lead", "prospect", "customer", "partner", "inactive"],
        ),
        field("notes", "Notes", "text", false, &[]),
        field("created_at", "Created", "date", false, &[]),
        field("updated_at", "Updated", "date", false, &[]),
    ];
    if kind == "contacts" {
        fields.extend([
            field("email", "Email addresses", "text", false, &[]),
            field("primary_email", "Primary email", "email", false, &[]),
            field("secondary_email", "Secondary email", "email", false, &[]),
            field("phone", "Phone numbers", "text", false, &[]),
            field("job_title", "Job title", "text", true, &[]),
            field("department", "Department", "text", true, &[]),
            field("source", "Source", "text", true, &[]),
            field("website", "Website", "url", true, &[]),
            field("linkedin", "LinkedIn", "url", true, &[]),
        ]);
    } else {
        fields.extend([
            field("website_url", "Website", "url", false, &[]),
            field("industry", "Industry", "text", true, &[]),
            field(
                "organization_type",
                "Type",
                "select",
                true,
                &["company", "nonprofit", "government", "education", "other"],
            ),
            field("email", "Email", "email", true, &[]),
            field("phone", "Phone", "text", true, &[]),
            field("employee_count", "Employees", "number", true, &[]),
            field(
                "annual_revenue",
                "Annual revenue (USD)",
                "number",
                true,
                &[],
            ),
            field("linkedin", "LinkedIn", "url", true, &[]),
        ]);
    }
    for (key, label) in [
        ("address_line1", "Address line 1"),
        ("address_line2", "Address line 2"),
        ("locality", "City / locality"),
        ("administrative_area", "State / province"),
        ("postal_code", "Postal code"),
        ("country_code", "Country / region"),
    ] {
        fields.push(field(&format!("location.{key}"), label, "text", true, &[]));
    }
    Ok(fields)
}
pub async fn definitions(
    db: &mut PgConnection,
    workspace: Uuid,
    kind: &str,
) -> Result<Vec<RecordField>, Error> {
    let mut fields = builtin(kind)?;
    for row in sqlx::query("SELECT key,name,field_type,options,archived,version FROM suite.record_fields WHERE workspace_id=$1 AND kind=$2 ORDER BY created_at,key").bind(workspace).bind(kind).fetch_all(db).await?{
  fields.push(RecordField{key:row.try_get("key")?,name:row.try_get("name")?,field_type:row.try_get("field_type")?,options:row.try_get::<Json<Vec<String>>,_>("options")?.0,archived:row.try_get("archived")?,version:row.try_get("version")?,custom:true,editable:true});
 }
    Ok(fields)
}
pub async fn fields(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    kind: &str,
) -> Result<Vec<RecordField>, Error> {
    let mut tx = pool.begin().await?;
    access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        &permission(kind, "read")?,
    )
    .await?;
    definitions(&mut tx, actor.workspace, kind).await
}
const COUNTRIES: &str = "AD AE AF AG AI AL AM AO AQ AR AS AT AU AW AX AZ BA BB BD BE BF BG BH BI BJ BL BM BN BO BQ BR BS BT BV BW BY BZ CA CC CD CF CG CH CI CK CL CM CN CO CR CU CV CW CX CY CZ DE DJ DK DM DO DZ EC EE EG EH ER ES ET FI FJ FK FM FO FR GA GB GD GE GF GG GH GI GL GM GN GP GQ GR GS GT GU GW GY HK HM HN HR HT HU ID IE IL IM IN IO IQ IR IS IT JE JM JO JP KE KG KH KI KM KN KP KR KW KY KZ LA LB LC LI LK LR LS LT LU LV LY MA MC MD ME MF MG MH MK ML MM MN MO MP MQ MR MS MT MU MV MW MX MY MZ NA NC NE NF NG NI NL NO NP NR NU NZ OM PA PE PF PG PH PK PL PM PN PR PS PT PW PY QA RE RO RS RU RW SA SB SC SD SE SG SH SI SJ SK SL SM SN SO SR SS ST SV SX SY SZ TC TD TF TG TH TJ TK TL TM TN TO TR TT TV TW TZ UA UG UM US UY UZ VA VC VE VG VI VN VU WF WS YE YT ZA ZM ZW";
pub fn valid_value(field: &RecordField, value: &Value) -> bool {
    if value.is_null() {
        return true;
    }
    match field.field_type.as_str() {
        "boolean" => value.is_boolean(),
        "number" => value
            .as_f64()
            .is_some_and(|v| v.is_finite() && v.abs() <= 1e15),
        "date" => value.as_str().is_some_and(|s| {
            s.len() == 10 && chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok()
        }),
        "select" => value
            .as_str()
            .is_some_and(|v| field.options.iter().any(|o| o == v)),
        "email" => value
            .as_str()
            .is_some_and(|s| s.len() <= 254 && s.parse::<email_address::EmailAddress>().is_ok()),
        "url" => value.as_str().is_some_and(|s| {
            s.len() <= 2048
                && reqwest::Url::parse(s).is_ok_and(|u| {
                    matches!(u.scheme(), "http" | "https")
                        && u.host_str().is_some()
                        && u.username().is_empty()
                        && u.password().is_none()
                })
        }),
        _ => value
            .as_str()
            .is_some_and(|s| s.chars().count() <= 2000 && !s.contains('\0')),
    }
}
pub async fn validate_properties(
    db: &mut PgConnection,
    workspace: Uuid,
    kind: &str,
    properties: Option<&Map<String, Value>>,
) -> Result<(), Error> {
    let Some(values) = properties else {
        return Ok(());
    };
    if serde_json::to_vec(values)
        .map_err(|_| Error::Invalid("Invalid properties"))?
        .len()
        > 32768
    {
        return Err(Error::Invalid("Record properties exceed 32 KiB"));
    }
    let fields = definitions(db, workspace, kind).await?;
    for (key, value) in values {
        if key == "location" {
            let Some(address) = value.as_object() else {
                return Err(Error::Invalid(
                    "Location must be a structured postal address",
                ));
            };
            for (part, v) in address {
                if !fields.iter().any(|f| f.key == format!("location.{part}"))
                    || !v.as_str().is_some_and(|s| {
                        s.chars().count() <= 200 && !s.chars().any(char::is_control)
                    })
                {
                    return Err(Error::Invalid(
                        "Address fields must be plain text up to 200 characters",
                    ));
                }
            }
            if !address.is_empty() {
                let country = address
                    .get("country_code")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if !COUNTRIES.split_whitespace().any(|c| c == country) {
                    return Err(Error::Invalid(
                        "Choose a valid two-letter country code for the address",
                    ));
                }
                if address
                    .get("postal_code")
                    .and_then(Value::as_str)
                    .is_some_and(|s| s.chars().count() > 32)
                {
                    return Err(Error::Invalid("Postal code must be at most 32 characters"));
                }
            }
        } else {
            let f = fields
                .iter()
                .find(|f| f.key == *key && f.editable)
                .ok_or(Error::Invalid("Unknown or read-only record field"))?;
            if !valid_value(f, value) {
                return Err(Error::Invalid(
                    "A record field has an invalid value or type",
                ));
            }
            if key == "employee_count"
                && !value.is_null()
                && !value.as_u64().is_some_and(|v| v <= 1_000_000_000)
            {
                return Err(Error::Invalid(
                    "Employee count must be a nonnegative whole number",
                ));
            }
            if key == "annual_revenue" && value.as_f64().is_some_and(|v| v < 0.0) {
                return Err(Error::Invalid("Annual revenue cannot be negative"));
            }
        }
    }
    Ok(())
}
pub async fn save_field(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    kind: &str,
    key: &str,
    input: SaveField,
) -> Result<RecordField, Error> {
    let mut tx = pool.begin().await?;
    permission(kind, "read")?;
    let access = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        "records.fields.manage",
    )
    .await?;
    if !access.workspace_wide {
        return Err(Error::Forbidden);
    }
    let suffix = input.key.strip_prefix("custom_").unwrap_or("");
    if suffix.is_empty()
        || suffix.len() > 48
        || !suffix.as_bytes()[0].is_ascii_lowercase()
        || !suffix
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        || input.name.trim().is_empty()
        || input.name.chars().count() > 80
        || ![
            "text", "number", "date", "boolean", "select", "email", "url",
        ]
        .contains(&input.field_type.as_str())
        || input.options.len() > 100
        || input
            .options
            .iter()
            .any(|o| o.trim().is_empty() || o.chars().count() > 80)
        || (input.field_type == "select" && input.options.is_empty())
        || (input.field_type != "select" && !input.options.is_empty())
    {
        return Err(Error::Invalid(
            "Provide a unique custom_ field key, name, supported type and valid options",
        ));
    }
    let mut unique = input.options.clone();
    unique.sort();
    unique.dedup();
    if unique.len() != input.options.len() {
        return Err(Error::Invalid("RecordField options must be unique"));
    }
    let request = serde_json::to_value(&input).unwrap();
    let operation = format!("records.{kind}.fields.save");
    if let Some(result) =
        reliability::replay(&mut tx, actor.workspace, &access, &operation, key, &request).await?
    {
        return Ok(result);
    }
    crate::modules::lock_workspace(&mut tx, actor.workspace).await?;
    let fields = definitions(&mut tx, actor.workspace, kind).await?;
    if let Some(old) = fields.iter().find(|f| f.key == input.key) {
        if !old.custom || Some(old.version) != input.expected_version {
            return Err(Error::Conflict("RecordField changed; reload before saving"));
        }
        // Type and option removal would invalidate stored records and saved views.
        if old.field_type != input.field_type
            || old.options.iter().any(|o| !input.options.contains(o))
        {
            return Err(Error::Invalid(
                "RecordField types are immutable; existing options cannot be removed",
            ));
        }
    } else if input.expected_version.is_some() || fields.iter().filter(|f| f.custom).count() >= 200
    {
        return Err(Error::Invalid(
            "Unknown field or workspace field limit reached",
        ));
    }
    sqlx::query("INSERT INTO suite.record_fields(workspace_id,kind,key,name,field_type,options,archived) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(workspace_id,kind,key) DO UPDATE SET name=$4,options=$6,archived=$7,version=suite.record_fields.version+1")
 .bind(actor.workspace).bind(kind).bind(&input.key).bind(input.name.trim()).bind(&input.field_type).bind(Json(&input.options)).bind(input.archived).execute(&mut *tx).await?;
    let result = definitions(&mut tx, actor.workspace, kind)
        .await?
        .into_iter()
        .find(|f| f.key == input.key)
        .unwrap();
    reliability::audit(
        &mut tx,
        actor,
        &access,
        &operation,
        actor.workspace,
        serde_json::json!({"field":input.key}),
    )
    .await?;
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
fn decode_view(row: sqlx::postgres::PgRow, user: Uuid) -> Result<RecordView, Error> {
    Ok(RecordView {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        shared: row.try_get("shared")?,
        mine: row.try_get::<Uuid, _>("owner_user_id")? == user,
        version: row.try_get("version")?,
        config: row.try_get::<Json<ViewConfig>, _>("config")?.0,
    })
}
pub async fn views(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    kind: &str,
) -> Result<Vec<RecordView>, Error> {
    let mut tx = pool.begin().await?;
    let access = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        &permission(kind, "read")?,
    )
    .await?;
    sqlx::query("SELECT * FROM suite.record_views WHERE workspace_id=$1 AND kind=$2 AND (shared OR owner_user_id=$3) ORDER BY name,id LIMIT 200").bind(actor.workspace).bind(kind).bind(access.user_id).fetch_all(&mut *tx).await?.into_iter().map(|r|decode_view(r,access.user_id)).collect()
}
pub async fn save_view(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    kind: &str,
    id: Option<Uuid>,
    key: &str,
    input: SaveView,
) -> Result<RecordView, Error> {
    let mut tx = pool.begin().await?;
    let access = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        &permission(kind, "read")?,
    )
    .await?;
    let fields = definitions(&mut tx, actor.workspace, kind).await?;
    if input.name.trim().is_empty()
        || input.name.chars().count() > 80
        || input.config.columns.is_empty()
        || input.config.columns.len() > 40
        || input
            .config
            .columns
            .iter()
            .any(|c| !fields.iter().any(|f| f.key == *c))
        || !fields.iter().any(|f| f.key == input.config.sort)
        || !["asc", "desc"].contains(&input.config.direction.as_str())
    {
        return Err(Error::Invalid("Invalid view name, columns or ordering"));
    }
    if let Some(filter) = &input.config.filter {
        validate_filter(filter, &fields)?;
    }
    if input.shared && !access.workspace_wide {
        return Err(Error::Forbidden);
    }
    let operation = format!("records.{kind}.views.save");
    let request = serde_json::json!({"id":id,"input":input});
    if let Some(result) =
        reliability::replay(&mut tx, actor.workspace, &access, &operation, key, &request).await?
    {
        return Ok(result);
    }
    crate::modules::lock_workspace(&mut tx, actor.workspace).await?;
    let id = if let Some(id) = id {
        let row=sqlx::query("SELECT owner_user_id,version FROM suite.record_views WHERE workspace_id=$1 AND kind=$2 AND id=$3 FOR UPDATE").bind(actor.workspace).bind(kind).bind(id).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
        if row.try_get::<Uuid, _>("owner_user_id")? != access.user_id {
            return Err(Error::Forbidden);
        }
        if Some(row.try_get::<i64, _>("version")?) != input.expected_version {
            return Err(Error::Conflict("View changed; reload before saving"));
        }
        id
    } else {
        let count:i64=sqlx::query_scalar("SELECT count(*) FROM suite.record_views WHERE workspace_id=$1 AND kind=$2 AND owner_user_id=$3").bind(actor.workspace).bind(kind).bind(access.user_id).fetch_one(&mut *tx).await?;
        if count >= 100 {
            return Err(Error::Invalid("Limit of 100 saved views per record type"));
        }
        Uuid::new_v4()
    };
    let row=sqlx::query("INSERT INTO suite.record_views(id,workspace_id,kind,owner_user_id,name,shared,config) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(id) DO UPDATE SET name=$5,shared=$6,config=$7,version=suite.record_views.version+1,updated_at=now() RETURNING *")
 .bind(id).bind(actor.workspace).bind(kind).bind(access.user_id).bind(input.name.trim()).bind(input.shared).bind(Json(&input.config)).fetch_one(&mut *tx).await?;
    let result = decode_view(row, access.user_id)?;
    reliability::audit(
        &mut tx,
        actor,
        &access,
        &operation,
        id,
        serde_json::json!({"shared":input.shared}),
    )
    .await?;
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

#[derive(Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteView {
    pub expected_version: i64,
}
pub async fn delete_view(
    pool: &PgPool,
    actor: &ActorContext<'_>,
    kind: &str,
    id: Uuid,
    key: &str,
    input: DeleteView,
) -> Result<RecordView, Error> {
    let mut tx = pool.begin().await?;
    let access = access::require(
        &mut tx,
        actor.identity,
        actor.workspace,
        &permission(kind, "read")?,
    )
    .await?;
    let operation = format!("records.{kind}.views.delete");
    let request = serde_json::json!({"id":id,"input":input});
    if let Some(result) =
        reliability::replay(&mut tx, actor.workspace, &access, &operation, key, &request).await?
    {
        return Ok(result);
    }
    let row = sqlx::query(
        "SELECT * FROM suite.record_views WHERE workspace_id=$1 AND kind=$2 AND id=$3 FOR UPDATE",
    )
    .bind(actor.workspace)
    .bind(kind)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(Error::NotFound)?;
    let result = decode_view(row, access.user_id)?;
    if !result.mine {
        return Err(Error::Forbidden);
    }
    if result.version != input.expected_version {
        return Err(Error::Conflict("View changed; reload before deleting"));
    }
    sqlx::query("DELETE FROM suite.record_views WHERE workspace_id=$1 AND id=$2")
        .bind(actor.workspace)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    reliability::audit(
        &mut tx,
        actor,
        &access,
        &operation,
        id,
        serde_json::json!({}),
    )
    .await?;
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
